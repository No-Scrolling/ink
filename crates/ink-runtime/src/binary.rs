use std::{collections::HashMap, sync::{Arc, Mutex}};
use rquickjs::{Ctx, Exception, Function, TypedArray};
use super::{Event, EventSink, MAX_MESSAGE_BYTES};

pub(super) const MAX_BYTES: usize = 512 * 1024;
const MAX_QUEUED_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Default)]
pub(super) struct Buffers(Arc<Mutex<Queued>>);

#[derive(Default)]
struct Queued {
    buffers: HashMap<u64, Vec<u8>>,
    bytes: usize,
}

impl Queued {
    fn remove(&mut self, id: u64) -> Option<Vec<u8>> {
        let bytes = self.buffers.remove(&id)?;
        self.bytes -= bytes.len();
        Some(bytes)
    }
}

impl Buffers {
    pub fn take(&self, id: u64) -> Option<Vec<u8>> { self.0.lock().ok()?.remove(id) }
}

pub(super) fn install(ctx: &Ctx<'_>, buffers: Buffers, events: EventSink) -> rquickjs::Result<()> {
    ctx.globals().set("__inkPostBytes", Function::new(ctx.clone(), move |ctx: Ctx<'_>, message: String, bytes: TypedArray<'_, u8>| {
        if message.len() > MAX_MESSAGE_BYTES { return Err(Exception::throw_range(&ctx, "native message is too large")); }
        let request: serde_json::Value = serde_json::from_str(&message).map_err(|_| Exception::throw_type(&ctx, "invalid binary request"))?;
        let id = request.get("id").and_then(serde_json::Value::as_u64).filter(|id| *id > 0)
            .ok_or_else(|| Exception::throw_type(&ctx, "invalid binary request ID"))?;
        if request.get("type").and_then(serde_json::Value::as_str) != Some("call") || request.get("binary") != Some(&serde_json::Value::Bool(true)) {
            return Err(Exception::throw_type(&ctx, "binary payload requires a native call"));
        }
        // No JavaScript executes while borrowing the typed array; the queue owns its copy.
        let bytes = unsafe { bytes.as_bytes() }.ok_or_else(|| Exception::throw_type(&ctx, "detached binary buffer"))?;
        if bytes.len() > MAX_BYTES { return Err(Exception::throw_range(&ctx, "native binary payload is too large")); }
        let mut queued = buffers.0.lock().map_err(|_| Exception::throw_message(&ctx, "binary queue unavailable"))?;
        if queued.buffers.len() >= 256 || queued.bytes + bytes.len() > MAX_QUEUED_BYTES {
            return Err(Exception::throw_message(&ctx, "busy: native binary queue is full"));
        }
        if queued.buffers.contains_key(&id) { return Err(Exception::throw_type(&ctx, "duplicate binary request ID")); }
        queued.buffers.insert(id, bytes.to_vec());
        queued.bytes += bytes.len();
        if events.try_send(Event::Message(message)).is_err() {
            queued.remove(id);
            return Err(Exception::throw_message(&ctx, "busy: native event queue is unavailable"));
        }
        Ok(())
    })?)
}

#[cfg(test)]
#[path = "../../../test/unit/runtime-binary.rs"]
mod tests;
