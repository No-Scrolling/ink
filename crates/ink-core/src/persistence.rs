use std::{collections::HashMap, fmt};

use super::{PersistedState, StateValue};

const HEADER: &[u8] = b"INKS\x01";
const SIZE_LIMIT: usize = 1024 * 1024;
const DEPTH_LIMIT: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistenceTooLarge;

impl fmt::Display for PersistenceTooLarge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("persisted state exceeds Ink's 1 MiB limit")
    }
}

impl std::error::Error for PersistenceTooLarge {}

pub(super) fn encode_persisted_state<'a>(
    values: impl Iterator<Item = (&'a PersistedState, &'a StateValue)>,
) -> Result<Vec<u8>, PersistenceTooLarge> {
    let values = values.collect::<Vec<_>>();
    let mut output = Vec::new();
    output.extend_from_slice(HEADER);
    write_u32(&mut output, values.len())?;
    for (persisted, value) in values {
        write_string(&mut output, &persisted.key)?;
        output.extend_from_slice(&persisted.schema.to_le_bytes());
        encode_value(&mut output, value)?;
    }
    if output.len() > SIZE_LIMIT {
        return Err(PersistenceTooLarge);
    }
    Ok(output)
}

pub(super) fn decode_persisted_state(bytes: &[u8]) -> Option<HashMap<String, (u64, StateValue)>> {
    if bytes.len() > SIZE_LIMIT || !bytes.starts_with(HEADER) {
        return None;
    }
    let mut reader = Reader {
        bytes,
        position: HEADER.len(),
    };
    let count = reader.u32()? as usize;
    if count > 1024 {
        return None;
    }
    let mut values = HashMap::with_capacity(count);
    for _ in 0..count {
        let key = reader.string()?;
        if key.is_empty() || key.len() > 128 || values.contains_key(&key) {
            return None;
        }
        let schema = reader.u64()?;
        let value = reader.value(0)?;
        values.insert(key, (schema, value));
    }
    (reader.position == bytes.len()).then_some(values)
}

fn encode_value(output: &mut Vec<u8>, value: &StateValue) -> Result<(), PersistenceTooLarge> {
    match value {
        StateValue::Int(value) => {
            output.push(0);
            output.extend_from_slice(&value.to_le_bytes());
        }
        StateValue::Bool(value) => {
            output.push(1);
            output.push(u8::from(*value));
        }
        StateValue::String(value) => {
            output.push(2);
            write_string(output, value)?;
        }
        StateValue::List(values) => {
            output.push(3);
            write_u32(output, values.len())?;
            for value in values {
                encode_value(output, value)?;
            }
        }
        StateValue::Object(fields) => {
            output.push(4);
            write_u32(output, fields.len())?;
            for (name, value) in fields {
                write_string(output, name)?;
                encode_value(output, value)?;
            }
        }
    }
    if output.len() > SIZE_LIMIT {
        return Err(PersistenceTooLarge);
    }
    Ok(())
}

fn write_u32(output: &mut Vec<u8>, value: usize) -> Result<(), PersistenceTooLarge> {
    let value = u32::try_from(value).map_err(|_| PersistenceTooLarge)?;
    output.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_string(output: &mut Vec<u8>, value: &str) -> Result<(), PersistenceTooLarge> {
    write_u32(output, value.len())?;
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn take(&mut self, length: usize) -> Option<&[u8]> {
        let end = self.position.checked_add(length)?;
        let value = self.bytes.get(self.position..end)?;
        self.position = end;
        Some(value)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(*self.take(1)?.first()?)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn string(&mut self) -> Option<String> {
        let length = self.u32()? as usize;
        let value = std::str::from_utf8(self.take(length)?).ok()?;
        Some(value.to_owned())
    }

    fn value(&mut self, depth: usize) -> Option<StateValue> {
        if depth > DEPTH_LIMIT {
            return None;
        }
        match self.u8()? {
            0 => Some(StateValue::Int(i64::from_le_bytes(
                self.take(8)?.try_into().ok()?,
            ))),
            1 => match self.u8()? {
                0 => Some(StateValue::Bool(false)),
                1 => Some(StateValue::Bool(true)),
                _ => None,
            },
            2 => self.string().map(StateValue::String),
            3 => {
                let count = self.u32()? as usize;
                if count > self.bytes.len().saturating_sub(self.position) {
                    return None;
                }
                (0..count)
                    .map(|_| self.value(depth + 1))
                    .collect::<Option<_>>()
                    .map(StateValue::List)
            }
            4 => {
                let count = self.u32()? as usize;
                if count > self.bytes.len().saturating_sub(self.position) {
                    return None;
                }
                (0..count)
                    .map(|_| Some((self.string()?, self.value(depth + 1)?)))
                    .collect::<Option<_>>()
                    .map(StateValue::Object)
            }
            _ => None,
        }
    }
}
