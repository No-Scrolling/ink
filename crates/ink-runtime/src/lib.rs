#[cfg(target_os = "android")]
pub mod cpu_affinity;
mod encoding;
mod commit;
mod binary;
mod queue;
pub type EventReceiver = queue::Receiver<Event>;

use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{RecvTimeoutError, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, anyhow};
use rquickjs::{CatchResultExt, Context, Ctx, Exception, Function, Persistent, Runtime, Value};

const CHANNEL_CAPACITY: usize = 256;
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
type WebLoader = Box<dyn Fn() -> Result<String> + Send>;

pub enum Event {
    Ready,
    Message(String),
    Commit(ink_protocol::ReactCommit),
    Error(String),
    Stopped,
}

enum Command {
    #[cfg(debug_assertions)]
    EvaluateDevelopment(String),
    Message(String, Option<Vec<u8>>),
    LatestMessage(u64),
    Stop,
}

pub struct AppRuntime {
    commands: queue::Sender<Command>,
    stopped: Arc<AtomicBool>,
    delivery_failed: Arc<Mutex<Option<String>>>,
    latest_messages: Arc<Mutex<HashMap<u64, queue::Retained<String>>>>,
    thread: Option<JoinHandle<()>>,
    binary: binary::Buffers,
    budget: Arc<queue::Budget>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl AppRuntime {
    pub fn spawn(source: String) -> Result<(Self, EventReceiver)> {
        Self::spawn_with_waker(source, || {})
    }

    pub fn spawn_with_waker(
        source: String,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<(Self, EventReceiver)> {
        Self::spawn_with_options(source, wake, None)
    }

    pub fn spawn_with_web_loader(
        source: String,
        wake: impl Fn() + Send + Sync + 'static,
        load_web: impl Fn() -> Result<String> + Send + 'static,
    ) -> Result<(Self, EventReceiver)> {
        Self::spawn_with_options(source, wake, Some(Box::new(load_web)))
    }

    fn spawn_with_options(
        source: String,
        wake: impl Fn() + Send + Sync + 'static,
        load_web: Option<WebLoader>,
    ) -> Result<(Self, EventReceiver)> {
        let budget = Arc::new(queue::Budget::default());
        let (commands, incoming) = queue::channel(CHANNEL_CAPACITY, budget.clone());
        let (outgoing, events) = queue::channel(CHANNEL_CAPACITY, budget.clone());
        let outgoing = EventSink {
            sender: outgoing,
            wake: Arc::new(wake),
        };
        let wake = outgoing.wake.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let cancelled = stopped.clone();
        let delivery_failed = Arc::new(Mutex::new(None));
        let failure = delivery_failed.clone();
        let latest_messages = Arc::new(Mutex::new(HashMap::new()));
        let pending_messages = latest_messages.clone();
        let binary = binary::Buffers::default();
        let binary_runtime = binary.clone();
        let thread = thread::Builder::new()
            .name("ink-js".into())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                let result = run(
                    source,
                    incoming,
                    outgoing.clone(),
                    cancelled.clone(),
                    failure,
                    pending_messages.clone(),
                    load_web,
                    binary_runtime,
                );
                pending_messages.lock().unwrap().clear();
                if let Err(error) = result {
                    outgoing.send_terminal(Event::Error(format!("{error:#}")), &cancelled);
                }
                cancelled.store(true, Ordering::Release);
                let _ = outgoing.try_send(Event::Stopped);
            })
            .context("could not start the JavaScript thread")?;
        Ok((
            Self {
                commands,
                stopped,
                delivery_failed,
                latest_messages,
                thread: Some(thread),
                binary,
                budget,
                wake,
            },
            events,
        ))
    }

    pub fn send(&self, message: String) -> Result<()> {
        if message.len() > MAX_MESSAGE_BYTES {
            return Err(anyhow!("native message exceeds {MAX_MESSAGE_BYTES} bytes"));
        }
        if self.stopped.load(Ordering::Acquire) {
            return Err(anyhow!("JavaScript runtime is closed"));
        }
        self.enqueue(Command::Message(message, None))
    }

    pub fn send_bytes(&self, message: String, bytes: Vec<u8>) -> Result<()> {
        anyhow::ensure!(message.len() <= MAX_MESSAGE_BYTES && bytes.len() <= binary::MAX_BYTES, "native binary result is too large");
        anyhow::ensure!(!self.stopped.load(Ordering::Acquire), "JavaScript runtime is closed");
        self.enqueue(Command::Message(message, Some(bytes)))
    }

    pub fn take_bytes(&self, id: u64) -> Option<Vec<u8>> { self.binary.take(id) }

    /// Replace an undelivered message for this key, keeping a single queue entry.
    pub fn send_latest(&self, key: u64, message: String) -> Result<()> {
        if message.len() > MAX_MESSAGE_BYTES {
            return Err(anyhow!("native message exceeds {MAX_MESSAGE_BYTES} bytes"));
        }
        if self.stopped.load(Ordering::Acquire) {
            return Err(anyhow!("JavaScript runtime is closed"));
        }
        let mut pending = self.latest_messages.lock().map_err(|_| anyhow!("JavaScript message queue is unavailable"))?;
        if let Some(previous) = pending.get_mut(&key) {
            let bytes = message.len();
            return previous.replace(message, bytes).map_err(|_| anyhow!("JavaScript message byte budget is full"));
        }
        let bytes = message.len();
        let message = self.budget.retain(message, bytes).map_err(|_| anyhow!("JavaScript message byte budget is full"))?;
        self.enqueue(Command::LatestMessage(key))?;
        pending.insert(key, message);
        Ok(())
    }

    fn enqueue(&self, command: Command) -> Result<()> {
        let bytes = match &command {
            Command::Message(message, bytes) => message.len() + bytes.as_ref().map_or(0, Vec::len),
            #[cfg(debug_assertions)]
            Command::EvaluateDevelopment(source) => source.len(),
            _ => 16,
        };
        self.commands
            .try_send(command, bytes)
            .map_err(|error| match error {
                TrySendError::Full(_) => {
                    let message = "JavaScript message queue is full; reload the runtime";
                    if let Ok(mut failure) = self.delivery_failed.lock() {
                        *failure = Some(message.into());
                    }
                    anyhow!(message)
                }

                TrySendError::Disconnected(_) => anyhow!("JavaScript runtime is closed"),
            })
    }

    #[cfg(debug_assertions)]
    pub fn evaluate_development(&self, source: String) -> Result<()> {
        if source.len() > 16 * 1024 * 1024 {
            return Err(anyhow!("development update exceeds 16 MiB"));
        }
        self.enqueue(Command::EvaluateDevelopment(source))
    }

    pub fn request_drain(&self) { (self.wake)(); }
    pub fn queue_metrics(&self) -> (usize, usize) { self.budget.metrics() }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self.commands.try_send(Command::Stop, 0);
    }
}

impl Drop for AppRuntime {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Clone)]
struct EventSink {
    sender: queue::Sender<Event>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl EventSink {
    fn send_terminal(&self, mut event: Event, stopped: &AtomicBool) {
        if let Event::Error(message) = &mut event
            && message.len() > MAX_MESSAGE_BYTES
        {
            let suffix = "\n[error truncated]";
            let end = message.floor_char_boundary(MAX_MESSAGE_BYTES - suffix.len());
            *message = format!("{}{suffix}", &message[..end]);
        }
        loop {
            match self.try_send(event) {
                Ok(()) | Err(TrySendError::Disconnected(_)) => return,
                Err(TrySendError::Full(returned)) => {
                    if stopped.load(Ordering::Acquire) {
                        return;
                    }
                    event = returned;
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    fn try_send(&self, event: Event) -> std::result::Result<(), TrySendError<Event>> {
        let bytes = match &event { Event::Message(value) | Event::Error(value) => value.len(), _ => 16 };
        self.try_send_sized(event, bytes)
    }

    fn try_send_sized(&self, event: Event, bytes: usize) -> std::result::Result<(), TrySendError<Event>> {
        let result = self.sender.try_send(event, bytes);
        (self.wake)();
        result
    }
}

#[derive(Default)]
struct TimerQueue {
    deadlines: BTreeSet<(Instant, u32)>,
    by_id: HashMap<u32, Instant>,
}

impl TimerQueue {
    fn insert(&mut self, id: u32, deadline: Instant) {
        self.remove(id);
        self.by_id.insert(id, deadline);
        self.deadlines.insert((deadline, id));
    }

    fn remove(&mut self, id: u32) {
        if let Some(deadline) = self.by_id.remove(&id) { self.deadlines.remove(&(deadline, id)); }
    }

    fn next(&self) -> Option<Instant> { self.deadlines.first().map(|&(deadline, _)| deadline) }

    fn pop_due(&mut self, now: Instant) -> Option<u32> {
        let &(deadline, id) = self.deadlines.first()?;
        if deadline > now { return None; }
        self.remove(id);
        Some(id)
    }
}

type Timers = Rc<RefCell<TimerQueue>>;

fn run(
    source: String,
    commands: queue::Receiver<Command>,
    events: EventSink,
    stopped: Arc<AtomicBool>,
    delivery_failed: Arc<Mutex<Option<String>>>,
    latest_messages: Arc<Mutex<HashMap<u64, queue::Retained<String>>>>,
    load_web: Option<WebLoader>,
    binary: binary::Buffers,
) -> Result<()> {
    #[cfg(target_os = "android")]
    let mut work_affinity = None;
    let runtime = Runtime::new()?;
    runtime.set_memory_limit(64 * 1024 * 1024);
    runtime.set_max_stack_size(512 * 1024);
    let interrupted = stopped.clone();
    runtime.set_interrupt_handler(Some(Box::new(move || interrupted.load(Ordering::Acquire))));
    let context = Context::full(&runtime)?;
    let rejections = Rc::new(RefCell::new(
        HashMap::<Persistent<Value<'static>>, String>::new(),
    ));
    let pending = Rc::downgrade(&rejections);
    runtime.set_host_promise_rejection_tracker(Some(Box::new(
        move |ctx, promise, reason, handled| {
            if let Some(pending) = pending.upgrade() {
                let promise = Persistent::save(&ctx, promise);
                if handled {
                    pending.borrow_mut().remove(&promise);
                } else {
                    let error = reason
                        .as_exception()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| format!("Unhandled promise rejection: {reason:?}"));
                    pending.borrow_mut().insert(promise, error);
                }
            }
        },
    )));
    let transport_failed = Rc::new(RefCell::new(false));
    let timers = Timers::default();
    install(
        &context,
        events.clone(),
        timers.clone(),
        transport_failed.clone(),
        binary,
    )?;
    if let Some(load_web) = load_web {
        context.with(|ctx| -> rquickjs::Result<()> {
            ctx.globals().set(
                "__inkLoadWeb",
                Function::new(ctx.clone(), move |ctx: Ctx<'_>| {
                    let source = load_web()
                        .map_err(|error| Exception::throw_message(&ctx, &error.to_string()))?;
                    ctx.eval_with_options::<(), _>(source, {
                        let mut options = rquickjs::context::EvalOptions::default();
                        options.filename = Some("ink-web.js".into());
                        options
                    })
                })?,
            )
        })?;
    }
    context.with(|ctx| {
        ctx.eval_with_options::<(), _>(source, {
            let mut options = rquickjs::context::EvalOptions::default();
            options.filename = Some("app.js".into());
            options
        })
        .catch(&ctx)
        .map_err(|error| anyhow!("{error}"))
    })?;
    events
        .try_send(Event::Ready)
        .map_err(|_| anyhow!("native event queue is unavailable"))?;

    // Reclaim temporary startup allocations once the initial render has settled.
    let mut startup_collection = Some(Instant::now() + Duration::from_secs(1));
    while !stopped.load(Ordering::Acquire) {
        if let Ok(mut failure) = delivery_failed.lock()
            && let Some(message) = failure.take()
        {
            return Err(anyhow!(message));
        }
        if *transport_failed.borrow() {
            return Err(anyhow!("Native commit queue is full; reload the runtime"));
        }
        // Bound each drain so messages and shutdown cannot be starved by promise chains.
        for _ in 0..256 {
            if stopped.load(Ordering::Acquire) {
                return Ok(());
            }
            match runtime.execute_pending_job() {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) => {
                    let message = error.0.with(|ctx| {
                        rquickjs::CaughtError::from_error(&ctx, rquickjs::Error::Exception)
                            .to_string()
                    });
                    return Err(anyhow!(message));
                }
            }
        }
        #[cfg(target_os = "android")]
        if !runtime.is_job_pending() {
            work_affinity = None;
        }
        if !runtime.is_job_pending()
            && let Some(error) = rejections.borrow().values().next()
        {
            return Err(anyhow!(error.clone()));
        }
        if *transport_failed.borrow() {
            return Err(anyhow!("Native commit queue is full; reload the runtime"));
        }
        if !runtime.is_job_pending()
            && startup_collection.is_some_and(|deadline| Instant::now() >= deadline)
        {
            runtime.run_gc();
            #[cfg(target_os = "android")]
            unsafe {
                const M_PURGE_ALL: i32 = -104;
                unsafe extern "C" {
                    fn mallopt(option: i32, value: i32) -> i32;
                }
                mallopt(M_PURGE_ALL, 0);
            }
            startup_collection = None;
        }
        let wait = if runtime.is_job_pending() {
            Some(Duration::ZERO)
        } else {
            timers
                .borrow()
                .next()
                .map(|deadline| deadline.saturating_duration_since(Instant::now()))
        };
        let wait = match startup_collection {
            Some(deadline) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                Some(wait.map_or(remaining, |wait| wait.min(remaining)))
            }
            None => wait,
        };
        let command = match wait {
            Some(wait) => commands.recv_timeout(wait),
            None => commands.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let message = match command {
            #[cfg(debug_assertions)]
            Ok(Command::EvaluateDevelopment(source)) => {
                context.with(|ctx| {
                    ctx.eval_with_options::<(), _>(source, {
                        let mut options = rquickjs::context::EvalOptions::default();
                        options.filename = Some("app.js".into());
                        options
                    })
                    .catch(&ctx)
                    .map_err(|error| anyhow!("{error}"))
                })?;
                None
            }
            Ok(Command::Message(message, bytes)) => Some((message, bytes)),
            Ok(Command::LatestMessage(key)) => Some(latest_messages.lock()
                .map_err(|_| anyhow!("JavaScript message queue is unavailable"))?
                .remove(&key).context("missing latest JavaScript message")?.take()).map(|message| (message, None)),
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => None,
        };
        if let Some((message, bytes)) = message {
            #[cfg(target_os = "android")]
            if work_affinity.is_none() {
                work_affinity = cpu_affinity::prefer_performance();
            }
            context
                .with(|ctx| {
                    let receive: Function = ctx.globals().get("__inkReceive")?;
                    let bytes = match bytes {
                        Some(bytes) => rquickjs::TypedArray::new(ctx.clone(), bytes)?.into_value(),
                        None => Value::new_undefined(ctx.clone()),
                    };
                    receive.call::<_, ()>((message, bytes))
                })
                .map_err(|error| {
                    context
                        .with(|ctx| anyhow!("{}", rquickjs::CaughtError::from_error(&ctx, error)))
                })?;
        }
        if !stopped.load(Ordering::Acquire) {
            let now = Instant::now();
            let due = timers.borrow_mut().pop_due(now);
            if let Some(id) = due {
                #[cfg(target_os = "android")]
                if work_affinity.is_none() {
                    work_affinity = cpu_affinity::prefer_performance();
                }
                context
                    .with(|ctx| {
                        let fire: Function = ctx.globals().get("__inkFireTimer")?;
                        fire.call::<_, ()>((id,))
                    })
                    .map_err(|error| {
                        context.with(|ctx| {
                            anyhow!("{}", rquickjs::CaughtError::from_error(&ctx, error))
                        })
                    })?;
            }
        }
    }
    Ok(())
}

fn install(
    context: &Context,
    events: EventSink,
    timers: Timers,
    transport_failed: Rc<RefCell<bool>>,
    binary: binary::Buffers,
) -> Result<()> {
    let start = Instant::now();
    context
        .with(|ctx| {
            encoding::install(&ctx)?;
            binary::install(&ctx, binary, events.clone())?;
            ctx.eval::<(), _>(include_str!("encoding.js"))?;
            let global = ctx.globals();
            global.set("__inkIsProxy", Function::new(ctx.clone(), |value: Value<'_>| {
                unsafe { rquickjs::qjs::JS_IsProxy(value.as_raw()) }
            })?)?;
            global.set(
                "__inkNextId",
                Function::new(ctx.clone(), |ctx: Ctx<'_>| {
                    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
                    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
                    if id > 9_007_199_254_740_991 {
                        return Err(Exception::throw_range(
                            &ctx,
                            "native identifier space is exhausted",
                        ));
                    }
                    Ok(id as f64)
                })?,
            )?;

            commit::install(&ctx, events.clone(), transport_failed)?;
            global.set(
                "__inkPost",
                Function::new(ctx.clone(), move |ctx: Ctx<'_>, message: String| {
                    if message.len() > MAX_MESSAGE_BYTES {
                        return Err(Exception::throw_range(&ctx, "native message is too large"));
                    }
                    events.try_send(Event::Message(message)).map_err(|error| {
                        Exception::throw_message(
                            &ctx,
                            match error {
                                TrySendError::Full(_) => "busy: native event queue is full",
                                TrySendError::Disconnected(_) => {
                                    "unavailable: native event queue is closed"
                                }
                            },
                        )
                    })
                })?,
            )?;
            let scheduled = timers.clone();
            global.set(
                "__inkSchedule",
                Function::new(ctx.clone(), move |id: u32, delay: u32| {
                    scheduled
                        .borrow_mut()
                        .insert(id, Instant::now() + Duration::from_millis(delay.into()));
                })?,
            )?;
            global.set(
                "__inkCancelTimer",
                Function::new(ctx.clone(), move |id: u32| {
                    timers.borrow_mut().remove(id);
                })?,
            )?;
            global.set(
                "__inkNow",
                Function::new(ctx.clone(), move || start.elapsed().as_secs_f64() * 1000.0)?,
            )?;
            ctx.eval::<(), _>(include_str!("host.js"))?;
            ctx.eval::<(), _>(include_str!("events.js"))
        })
        .map_err(|error| {
            context.with(|ctx| anyhow!("{}", rquickjs::CaughtError::from_error(&ctx, error)))
        })
}
