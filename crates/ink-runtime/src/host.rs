//! Native modules implemented in Rust answer calls without crossing to the platform's UI thread.
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Command, MAX_MESSAGE_BYTES, queue};

/// A `callNative` request addressed to a Rust module.
pub struct NativeCall {
    pub id: u64,
    pub module: String,
    pub operation: String,
    pub payload: Value,
    pub timeout: Duration,
}

// Field order and escaping match the existing JSON result envelope. Count its
// encoded bytes without retaining a second copy of the result string.
#[derive(Serialize)]
pub(crate) struct NativeResult {
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retryable: Option<bool>,
    r#type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

impl NativeResult {
    fn value(id: u64, value: &str) -> Self {
        Self { id, kind: None, message: None, retryable: None, r#type: "result", value: Some(value.into()) }
    }

    fn error(id: u64, kind: &str, message: &str, retryable: bool) -> Self {
        Self { id, kind: Some(kind.into()), message: Some(message.into()), retryable: Some(retryable), r#type: "result", value: None }
    }

    fn encoded_size(&self) -> usize {
        struct Size(usize);
        impl std::io::Write for Size {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 += bytes.len();
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
        }
        let mut size = Size(0);
        serde_json::to_writer(&mut size, self).expect("native results serialise");
        size.0
    }

    pub(crate) fn deliver(self, ctx: &rquickjs::Ctx<'_>) -> rquickjs::Result<()> {
        let resolver: rquickjs::Value = ctx.globals().get("__inkResolveNativeResult")?;
        if let Some(resolver) = resolver.as_function() {
            resolver.call((self.id, self.value, self.kind, self.message, self.retryable))
        } else {
            // Embedders can supply __inkReceive without loading the Ink SDK.
            let receive: rquickjs::Function = ctx.globals().get("__inkReceive")?;
            receive.call((serde_json::to_string(&self).expect("native results serialise"),))
        }
    }
}

/// Delivers results to one runtime. Safe to use from worker threads after the runtime stops.
#[derive(Clone)]
pub struct Responder {
    commands: queue::Sender<Command>,
    stopped: Arc<AtomicBool>,
    pending: Arc<Mutex<HashMap<u64, Instant>>>,
    delivery_failed: Arc<Mutex<Option<String>>>,
}

impl Responder {
    pub(crate) fn new(commands: queue::Sender<Command>, stopped: Arc<AtomicBool>, delivery_failed: Arc<Mutex<Option<String>>>) -> Self {
        Self { commands, stopped, pending: Arc::default(), delivery_failed }
    }

    pub fn is_open(&self) -> bool {
        !self.stopped.load(Ordering::Acquire)
    }

    /// Queue a message for `__inkReceive`. Returns false when the runtime can no longer receive it.
    pub fn send(&self, message: String) -> bool {
        let bytes = message.len();
        self.send_command(Command::Message(message, None), bytes)
    }

    fn send_result(&self, result: NativeResult) -> bool {
        let bytes = result.encoded_size();
        self.send_command(Command::NativeResult(result), bytes)
    }

    fn send_command(&self, command: Command, bytes: usize) -> bool {
        if !self.is_open() || bytes > MAX_MESSAGE_BYTES { return false; }
        match self.commands.try_send(command, bytes) {
            Ok(()) => true,
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                *self.delivery_failed.lock().unwrap() = Some("JavaScript message queue is full; reload the runtime".into());
                false
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => false,
        }
    }

    /// Resolve request `id` with a string value, or reject it when the envelope exceeds the message limit.
    pub fn resolve(&self, id: u64, value: &str) -> bool {
        let message = NativeResult::value(id, value);
        let bytes = message.encoded_size();
        if bytes > MAX_MESSAGE_BYTES {
            return self.reject(id, "protocol", "Native result exceeds the message limit", false);
        }
        self.complete(message, bytes)
    }

    pub fn reject(&self, id: u64, kind: &str, message: &str, retryable: bool) -> bool {
        let result = NativeResult::error(id, kind, message, retryable);
        let bytes = result.encoded_size();
        self.complete(result, bytes)
    }

    fn complete(&self, message: NativeResult, bytes: usize) -> bool {
        let id = message.id;
        let Some(deadline) = self.pending.lock().unwrap().remove(&id) else { return false };
        if Instant::now() >= deadline {
            self.send_result(timeout(id));
            return false;
        }
        self.send_command(Command::NativeResult(message), bytes)
    }
}

fn timeout(id: u64) -> NativeResult {
    NativeResult::error(id, "timeout", "Native request timed out", true)
}

/// Keeps Rust requests subject to the same deadlines and cancellation as platform requests.
pub(crate) struct Requests {
    session: RefCell<Box<dyn NativeSession>>,
    responder: Responder,
}

impl Requests {
    pub(crate) fn new(host: Arc<dyn NativeHost>, responder: Responder) -> Self {
        Self { session: RefCell::new(host.connect(responder.clone())), responder }
    }

    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.responder.pending.lock().unwrap().values().copied().min()
    }

    pub(crate) fn expire(&self) {
        let now = Instant::now();
        let expired = {
            let mut pending = self.responder.pending.lock().unwrap();
            let expired: Vec<_> = pending.iter().filter_map(|(&id, &deadline)| (deadline <= now).then_some(id)).collect();
            for id in &expired { pending.remove(id); }
            expired
        };
        for id in expired {
            self.session.borrow_mut().cancel(id);
            self.responder.send_result(timeout(id));
        }
    }
}

impl Drop for Requests {
    fn drop(&mut self) {
        let pending = std::mem::take(&mut *self.responder.pending.lock().unwrap());
        for id in pending.keys() { self.session.get_mut().cancel(*id); }
    }
}

/// Process-wide provider of Rust native modules.
pub trait NativeHost: Send + Sync {
    fn connect(&self, responder: Responder) -> Box<dyn NativeSession>;
}

/// Native modules for one runtime, used on its JavaScript thread.
pub trait NativeSession {
    /// Handle `call` and return true, or return false to forward it to the platform.
    fn call(&mut self, call: NativeCall) -> bool;
    /// Cancel a request this session accepted. Returns false for requests it does not own.
    fn cancel(&mut self, id: u64) -> bool;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Posted {
    r#type: String,
    id: u64,
    module: Option<String>,
    operation: Option<String>,
    #[serde(default)]
    payload: Value,
    timeout_ms: Option<u64>,
    controller: Option<Value>,
    #[serde(default)]
    binary: bool,
}

/// Returns true if the message was handled without forwarding to the platform.
pub(crate) fn intercept(requests: &Requests, message: &str) -> bool {
    if !message.starts_with(r#"{"type":"call""#) && !message.starts_with(r#"{"type":"cancel""#) {
        return false;
    }
    let Ok(posted) = serde_json::from_str::<Posted>(message) else {
        return false;
    };
    match posted.r#type.as_str() {
        "cancel" => {
            let owned = requests.responder.pending.lock().unwrap().remove(&posted.id).is_some();
            if owned { requests.session.borrow_mut().cancel(posted.id); }
            owned
        }
        "call" if posted.controller.is_none() && !posted.binary => {
            let (Some(module), Some(operation)) = (posted.module, posted.operation) else {
                return false;
            };
            let timeout = Duration::from_millis(posted.timeout_ms.unwrap_or(30_000).clamp(1, 2_147_483_647));
            let failure = {
                let mut pending = requests.responder.pending.lock().unwrap();
                if posted.id == 0 || posted.id > 9_007_199_254_740_991 || pending.contains_key(&posted.id) {
                    Some(("protocol", "Invalid native request ID", false))
                } else if pending.len() >= 256 {
                    Some(("busy", "Too many pending native requests", true))
                } else {
                    pending.insert(posted.id, Instant::now() + timeout);
                    None
                }
            };
            if let Some((kind, message, retryable)) = failure {
                requests.responder.send_result(NativeResult::error(posted.id, kind, message, retryable));
                return true;
            }
            let handled = requests.session.borrow_mut().call(NativeCall {
                id: posted.id,
                module,
                operation,
                payload: posted.payload,
                timeout,
            });
            if !handled { requests.responder.pending.lock().unwrap().remove(&posted.id); }
            handled
        }
        _ => false,
    }
}
