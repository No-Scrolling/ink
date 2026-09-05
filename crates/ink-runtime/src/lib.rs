mod encoding;

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, anyhow};
use rquickjs::{CatchResultExt, Context, Ctx, Exception, Function, Persistent, Runtime, Value};

const CHANNEL_CAPACITY: usize = 256;
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

pub enum Event {
    Ready,
    Message(String),
    Error(String),
    Stopped,
}

enum Command {
    Message(String),
    Stop,
}

pub struct AppRuntime {
    commands: SyncSender<Command>,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl AppRuntime {
    pub fn spawn(source: String) -> Result<(Self, Receiver<Event>)> {
        Self::spawn_with_waker(source, || {})
    }

    pub fn spawn_with_waker(
        source: String,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<(Self, Receiver<Event>)> {
        let (commands, incoming) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let (outgoing, events) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let outgoing = EventSink {
            sender: outgoing,
            wake: Arc::new(wake),
        };
        let stopped = Arc::new(AtomicBool::new(false));
        let cancelled = stopped.clone();
        let thread = thread::Builder::new()
            .name("ink-js".into())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                if let Err(error) = run(source, incoming, outgoing.clone(), cancelled.clone()) {
                    let _ = outgoing.try_send(Event::Error(format!("{error:#}")));
                }
                cancelled.store(true, Ordering::Release);
                let _ = outgoing.try_send(Event::Stopped);
            })
            .context("could not start the JavaScript thread")?;
        Ok((
            Self {
                commands,
                stopped,
                thread: Some(thread),
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
        self.commands
            .try_send(Command::Message(message))
            .map_err(|error| match error {
                TrySendError::Full(_) => anyhow!("JavaScript message queue is full"),
                TrySendError::Disconnected(_) => anyhow!("JavaScript runtime is closed"),
            })
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self.commands.try_send(Command::Stop);
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
    sender: SyncSender<Event>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl EventSink {
    fn try_send(&self, event: Event) -> std::result::Result<(), TrySendError<Event>> {
        let result = self.sender.try_send(event);
        (self.wake)();
        result
    }
}

type Timers = Rc<RefCell<BTreeMap<u32, Instant>>>;

fn run(
    source: String,
    commands: Receiver<Command>,
    events: EventSink,
    stopped: Arc<AtomicBool>,
) -> Result<()> {
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
    let timers = Timers::default();
    install(&context, events.clone(), timers.clone())?;
    context.with(|ctx| {
        ctx.eval::<(), _>(source)
            .catch(&ctx)
            .map_err(|error| anyhow!("{error}"))
    })?;
    events
        .try_send(Event::Ready)
        .map_err(|_| anyhow!("native event queue is unavailable"))?;

    while !stopped.load(Ordering::Acquire) {
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
        if !runtime.is_job_pending()
            && let Some(error) = rejections.borrow().values().next()
        {
            return Err(anyhow!(error.clone()));
        }
        let wait = if runtime.is_job_pending() {
            Some(Duration::ZERO)
        } else {
            timers
                .borrow()
                .values()
                .min()
                .map(|deadline| deadline.saturating_duration_since(Instant::now()))
        };
        let command = match wait {
            Some(wait) => commands.recv_timeout(wait),
            None => commands.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match command {
            Ok(Command::Message(message)) => context
                .with(|ctx| {
                    let receive: Function = ctx.globals().get("__inkReceive")?;
                    receive.call::<_, ()>((message,))
                })
                .map_err(|error| {
                    context
                        .with(|ctx| anyhow!("{}", rquickjs::CaughtError::from_error(&ctx, error)))
                })?,
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                let now = Instant::now();
                let due = timers
                    .borrow()
                    .iter()
                    .filter(|(_, deadline)| **deadline <= now)
                    .min_by_key(|(id, deadline)| (**deadline, **id))
                    .map(|(&id, _)| id);
                if let Some(id) = due {
                    timers.borrow_mut().remove(&id);
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
    }
    Ok(())
}

fn install(context: &Context, events: EventSink, timers: Timers) -> Result<()> {
    let start = Instant::now();
    context
        .with(|ctx| {
            encoding::install(&ctx)?;
            ctx.eval::<(), _>(include_str!("encoding.js"))?;
            let global = ctx.globals();
            global.set(
                "__inkPost",
                Function::new(ctx.clone(), move |ctx: Ctx<'_>, message: String| {
                    if message.len() > MAX_MESSAGE_BYTES {
                        return Err(Exception::throw_range(&ctx, "native message is too large"));
                    }
                    events.try_send(Event::Message(message)).map_err(|_| {
                        Exception::throw_message(&ctx, "native event queue is unavailable")
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
                    timers.borrow_mut().remove(&id);
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
