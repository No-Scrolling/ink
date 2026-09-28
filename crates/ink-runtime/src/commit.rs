use std::{cell::RefCell, rc::Rc, sync::mpsc::TrySendError};

use ink_protocol::{Operation, ReactCommit};
use rquickjs::{Array, Ctx, Exception, FromJs, Function, Object, Result, Value};

use crate::{Event, EventSink, MAX_MESSAGE_BYTES};

pub(crate) fn install(ctx: &Ctx<'_>, events: EventSink, failed: Rc<RefCell<bool>>) -> Result<()> {
    ctx.globals().set(
        "__inkCommit",
        Function::new(ctx.clone(), move |ctx: Ctx<'_>, batch: Array<'_>| {
            let commit = read(batch)?;
            events.try_send(Event::Commit(commit)).map_err(|error| {
                *failed.borrow_mut() = true;
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
    )
}

fn read(batch: Array<'_>) -> Result<ReactCommit> {
    let ctx = batch.ctx();
    let mut bytes = batch.len().saturating_mul(32);
    check_size(ctx, bytes)?;
    let mut operations = Vec::with_capacity(batch.len());
    for item in batch.iter::<Object>() {
        let item = item?;
        let op: String = item.get("op")?;
        operations.push(match op.as_str() {
            "create" => {
                let kind: String = item.get("type")?;
                Operation::Create {
                    id: item.get::<_, Id>("id")?.0,
                    r#type: serde_json::from_value(serde_json::Value::String(kind))
                        .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?,
                    props: props(ctx, &item, &mut bytes)?,
                }
            }
            "update" => Operation::Update {
                id: item.get::<_, Id>("id")?.0,
                props: props(ctx, &item, &mut bytes)?,
            },
            "insert" => Operation::Insert {
                id: item.get::<_, Id>("id")?.0,
                parent: item.get::<_, Id>("parent")?.0,
                before: item.get::<_, Option<Id>>("before")?.map(|id| id.0),
            },
            "remove" => Operation::Remove {
                id: item.get::<_, Id>("id")?.0,
                parent: item.get::<_, Id>("parent")?.0,
            },
            "hidden" => Operation::Hidden {
                id: item.get::<_, Id>("id")?.0,
                value: item.get("value")?,
            },
            "values" => {
                let value: Value = item.get("values")?;
                let encoded = ctx.json_stringify(value)?
                    .ok_or_else(|| Exception::throw_type(ctx, "invalid view values"))?
                    .to_string()?;
                bytes = bytes.saturating_add(encoded.len());
                check_size(ctx, bytes)?;
                let values: Vec<(usize, serde_json::Value)> = serde_json::from_str(&encoded)
                    .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
                if values.iter().any(|(id, _)| *id as u64 > 9_007_199_254_740_991) {
                    return Err(Exception::throw_type(ctx, "invalid binding identifier"));
                }
                Operation::Values { view: item.get::<_, Id>("view")?.0, values }
            }
            "text" => {
                let changes: Array = item.get("changes")?;
                let count = changes.len() / 2;
                if changes.len() % 2 != 0 {
                    return Err(Exception::throw_type(ctx, "invalid text batch length"));
                }
                bytes = bytes.saturating_add(count.saturating_mul(16));
                check_size(ctx, bytes)?;
                let mut ids = Vec::with_capacity(count);
                let mut texts = Vec::with_capacity(count);
                for index in (0..changes.len()).step_by(2) {
                    ids.push(changes.get::<Id>(index)?.0);
                    let encoded = changes.get::<rquickjs::String>(index + 1)?.to_cstring()?;
                    // CString owns this byte range until the compact copy is made.
                    let data = unsafe {
                        std::slice::from_raw_parts(encoded.as_ptr().cast::<u8>(), encoded.len())
                    };
                    let text = std::str::from_utf8(data)?;
                    bytes = bytes.saturating_add(text.len());
                    check_size(ctx, bytes)?;
                    texts.push(ink_protocol::SmolStr::new(text));
                }
                Operation::Text { ids, values: texts }
            }
            _ => return Err(Exception::throw_type(ctx, "unknown React operation")),
        });
    }
    Ok(ReactCommit(operations))
}

fn props<'js>(
    ctx: &Ctx<'js>,
    item: &Object<'js>,
    bytes: &mut usize,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let value: Value = item.get("props")?;
    let json = ctx
        .json_stringify(value)?
        .ok_or_else(|| Exception::throw_type(ctx, "invalid React props"))?
        .to_string()?;
    *bytes = bytes.saturating_add(json.len());
    check_size(ctx, *bytes)?;
    serde_json::from_slice(json.as_bytes()).map_err(|error| Exception::throw_type(ctx, &error.to_string()))
}

fn check_size(ctx: &Ctx<'_>, bytes: usize) -> Result<()> {
    if bytes > MAX_MESSAGE_BYTES {
        return Err(Exception::throw_range(ctx, "native message is too large"));
    }
    Ok(())
}

struct Id(usize);

impl<'js> FromJs<'js> for Id {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let number = value
            .as_number()
            .filter(|number| {
                number.fract() == 0.0 && (0.0..=9_007_199_254_740_991.0).contains(number)
            })
            .and_then(|number| usize::try_from(number as u64).ok())
            .ok_or_else(|| Exception::throw_type(ctx, "invalid React identifier"))?;
        Ok(Self(number))
    }
}
