//! Native modules implemented in Rust answer calls without crossing to the platform's UI thread.
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
    time::{Duration, Instant},
};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{Command, MAX_MESSAGE_BYTES, queue};

/// A `callNative` request addressed to a Rust module.
pub struct NativeCall {
    pub id: u64,
    pub module: String,
    pub operation: String,
    pub payload: Value,
    pub timeout: Duration,
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
        if !self.is_open() || message.len() > MAX_MESSAGE_BYTES {
            return false;
        }
        let bytes = message.len();
        match self.commands.try_send(Command::Message(message, None), bytes) {
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
        let message = json!({"type": "result", "id": id, "value": value}).to_string();
        if message.len() > MAX_MESSAGE_BYTES {
            return self.reject(id, "protocol", "Native result exceeds the message limit", false);
        }
        self.complete(id, message)
    }

    pub fn reject(&self, id: u64, kind: &str, message: &str, retryable: bool) -> bool {
        self.complete(id, json!({"type": "result", "id": id, "kind": kind, "message": message, "retryable": retryable}).to_string())
    }

    fn complete(&self, id: u64, message: String) -> bool {
        let Some(deadline) = self.pending.lock().unwrap().remove(&id) else { return false };
        if Instant::now() >= deadline {
            self.send(timeout(id));
            return false;
        }
        self.send(message)
    }
}

fn timeout(id: u64) -> String {
    json!({"type": "result", "id": id, "kind": "timeout", "message": "Native request timed out", "retryable": true}).to_string()
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
            self.responder.send(timeout(id));
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
                requests.responder.send(json!({"type": "result", "id": posted.id, "kind": kind, "message": message, "retryable": retryable}).to_string());
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
