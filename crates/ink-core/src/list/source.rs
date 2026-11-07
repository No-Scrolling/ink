use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{self, Write};
use std::sync::{Arc, OnceLock};

use serde_json::Value as Json;

const MAX_ROWS: usize = 1_000_000;
const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;
const MAX_KEY_BYTES: usize = 1024;
const MAX_JSON_DEPTH: usize = 64;

#[derive(Clone, Debug)]
pub(crate) struct Record {
    pub key: String,
    pub value: Arc<Json>,
    pub revision: u64,
}

pub(crate) struct ListSnapshot {
    pub revision: u64,
    records: Arc<Records>,
    key_order: Arc<OnceLock<Arc<[String]>>>,
}

impl ListSnapshot {
    pub fn len(&self) -> usize {
        self.records.len
    }

    pub fn get(&self, key: &str) -> Option<&Record> {
        let index = *self.records.index.get(key)?;
        Some(&self.records.root.at(index, self.records.len).record)
    }

    pub fn keys(&self) -> Arc<[String]> {
        self.key_order
            .get_or_init(|| self.iter().map(|record| record.key.clone()).collect())
            .clone()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        self.stored().map(|record| &record.record)
    }

    fn stored(&self) -> StoredRecords<'_> {
        StoredRecords {
            stack: vec![&self.records.root],
        }
    }
}

impl fmt::Debug for ListSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListSnapshot")
            .field("revision", &self.revision)
            .field("count", &self.len())
            .finish()
    }
}

pub(crate) struct ListChange {
    pub previous_revision: u64,
    pub snapshot: Arc<ListSnapshot>,
    pub changed_keys: Vec<String>,
}

// One persistent store belongs to one large native list. Snapshots keep old
// records alive while sparse edits share the index and unchanged tree branches.
#[derive(Clone)]
pub(crate) struct ListSource {
    pub key: String,
    snapshot: Arc<ListSnapshot>,
}

impl ListSource {
    pub fn new(key: String, values: Vec<Arc<Json>>) -> Result<Self, String> {
        validate_key(&key)?;
        let (records, bytes) = validate_records(&key, values, 1, |_| None)?;
        Ok(Self {
            key,
            snapshot: Arc::new(ListSnapshot {
                revision: 1,
                records: Arc::new(Records::build(&records, bytes)),
                key_order: Arc::default(),
            }),
        })
    }

    pub fn snapshot(&self) -> Arc<ListSnapshot> {
        self.snapshot.clone()
    }

    pub fn set(
        &mut self,
        base_revision: u64,
        values: Vec<Arc<Json>>,
    ) -> Result<Option<ListChange>, String> {
        let revision = self.next_revision(base_revision)?;
        let previous = &self.snapshot.records;
        let (records, _) = validate_records(&self.key, values, revision, |key| {
            let index = *previous.index.get(key)?;
            Some(previous.root.at(index, previous.len).clone())
        })?;
        let mut replacements = Vec::with_capacity(records.len());
        let mut bytes = previous.bytes;
        for record in records {
            let index = *previous
                .index
                .get(&record.record.key)
                .ok_or_else(|| format!("unknown source key {:?}", record.record.key))?;
            let old = previous.root.at(index, previous.len);
            if !Arc::ptr_eq(old, &record) {
                bytes = bytes - old.bytes + record.bytes;
                replacements.push((index, record));
            }
        }
        if replacements.is_empty() {
            return Ok(None);
        }
        if bytes > MAX_TOTAL_BYTES {
            return Err(format!(
                "source data exceeds {MAX_TOTAL_BYTES} current encoded bytes"
            ));
        }
        let mut root = previous.root.clone();
        let mut changed_keys = Vec::with_capacity(replacements.len());
        for (index, record) in replacements {
            changed_keys.push(record.record.key.clone());
            root = root.set(index, previous.len, record);
        }
        let snapshot = Arc::new(ListSnapshot {
            revision,
            records: Arc::new(Records {
                index: previous.index.clone(),
                root,
                len: previous.len,
                bytes,
            }),
            key_order: self.snapshot.key_order.clone(),
        });
        self.snapshot = snapshot.clone();
        Ok(Some(ListChange {
            previous_revision: base_revision,
            snapshot,
            changed_keys,
        }))
    }

    pub fn replace(&mut self, base_revision: u64, values: Vec<Arc<Json>>) -> Result<(), String> {
        let revision = self.next_revision(base_revision)?;
        let previous = self.snapshot.stored().collect::<Vec<_>>();
        let (records, bytes) = validate_records(&self.key, values, revision, |key| {
            self.snapshot
                .records
                .index
                .get(key)
                .map(|index| previous[*index].clone())
        })?;
        let same_order = previous.len() == records.len()
            && previous
                .iter()
                .zip(&records)
                .all(|(old, record)| old.record.key == record.record.key);
        if same_order
            && previous
                .iter()
                .zip(&records)
                .all(|(old, record)| Arc::ptr_eq(old, record))
        {
            return Ok(());
        }
        self.snapshot = Arc::new(ListSnapshot {
            revision,
            records: Arc::new(Records::build(&records, bytes)),
            key_order: if same_order {
                self.snapshot.key_order.clone()
            } else {
                Arc::default()
            },
        });
        Ok(())
    }

    fn next_revision(&self, base_revision: u64) -> Result<u64, String> {
        if self.snapshot.revision != base_revision {
            return Err(format!(
                "stale source revision {base_revision}; current revision is {}",
                self.snapshot.revision
            ));
        }
        self.snapshot
            .revision
            .checked_add(1)
            .ok_or_else(|| "source revision is exhausted".into())
    }
}

struct StoredRecord {
    record: Record,
    bytes: usize,
}

struct Records {
    index: Arc<HashMap<String, usize>>,
    root: Arc<RecordTree>,
    len: usize,
    bytes: usize,
}

impl Records {
    fn build(records: &[Arc<StoredRecord>], bytes: usize) -> Self {
        Self {
            index: Arc::new(
                records
                    .iter()
                    .enumerate()
                    .map(|(index, record)| (record.record.key.clone(), index))
                    .collect(),
            ),
            root: RecordTree::build(records),
            len: records.len(),
            bytes,
        }
    }
}

// Stable-key edits copy only the path to each changed leaf. The key-to-position
// index is shared until a full replacement changes the source's order.
enum RecordTree {
    Empty,
    Leaf(Arc<StoredRecord>),
    Branch {
        left: Arc<RecordTree>,
        right: Arc<RecordTree>,
    },
}

impl RecordTree {
    fn build(records: &[Arc<StoredRecord>]) -> Arc<Self> {
        Arc::new(match records.len() {
            0 => Self::Empty,
            1 => Self::Leaf(records[0].clone()),
            len => Self::Branch {
                left: Self::build(&records[..len / 2]),
                right: Self::build(&records[len / 2..]),
            },
        })
    }

    fn at(&self, index: usize, len: usize) -> &Arc<StoredRecord> {
        match self {
            Self::Leaf(record) => record,
            Self::Branch { left, right } => {
                let middle = len / 2;
                if index < middle {
                    left.at(index, middle)
                } else {
                    right.at(index - middle, len - middle)
                }
            }
            Self::Empty => unreachable!("record position is validated before lookup"),
        }
    }

    fn set(&self, index: usize, len: usize, record: Arc<StoredRecord>) -> Arc<Self> {
        Arc::new(match self {
            Self::Leaf(_) => Self::Leaf(record),
            Self::Branch { left, right } => {
                let middle = len / 2;
                if index < middle {
                    Self::Branch {
                        left: left.set(index, middle, record),
                        right: right.clone(),
                    }
                } else {
                    Self::Branch {
                        left: left.clone(),
                        right: right.set(index - middle, len - middle, record),
                    }
                }
            }
            Self::Empty => unreachable!("record position is validated before replacement"),
        })
    }
}

struct StoredRecords<'a> {
    stack: Vec<&'a RecordTree>,
}

impl<'a> Iterator for StoredRecords<'a> {
    type Item = &'a Arc<StoredRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(records) = self.stack.pop() {
            match records {
                RecordTree::Empty => {}
                RecordTree::Leaf(record) => return Some(record),
                RecordTree::Branch { left, right } => {
                    self.stack.push(right);
                    self.stack.push(left);
                }
            }
        }
        None
    }
}

fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > MAX_KEY_BYTES {
        return Err(format!(
            "source keys must contain 1–{MAX_KEY_BYTES} UTF-8 bytes"
        ));
    }
    Ok(())
}

fn encoded_bytes(value: &Json) -> Result<usize, String> {
    let mut stack = vec![(value, 0)];
    while let Some((value, depth)) = stack.pop() {
        if depth > MAX_JSON_DEPTH {
            return Err(format!("source JSON exceeds {MAX_JSON_DEPTH} levels"));
        }
        match value {
            Json::Array(values) => stack.extend(values.iter().map(|value| (value, depth + 1))),
            Json::Object(values) => stack.extend(values.values().map(|value| (value, depth + 1))),
            _ => {}
        }
    }
    struct Counter(usize);
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > MAX_RECORD_BYTES - self.0 {
                return Err(io::Error::other("source JSON exceeds its byte limit"));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value)
        .map_err(|_| format!("source JSON exceeds {MAX_RECORD_BYTES} encoded bytes"))?;
    Ok(counter.0)
}

fn validate_records(
    key: &str,
    values: Vec<Arc<Json>>,
    revision: u64,
    previous: impl Fn(&str) -> Option<Arc<StoredRecord>>,
) -> Result<(Vec<Arc<StoredRecord>>, usize), String> {
    if values.len() > MAX_ROWS {
        return Err(format!("a source accepts at most {MAX_ROWS} records"));
    }
    let mut seen = HashSet::with_capacity(values.len());
    let mut records = Vec::with_capacity(values.len());
    let mut bytes = 0;
    for value in values {
        let object = value
            .as_object()
            .ok_or("source records must be JSON objects")?;
        let record_key = object
            .get(key)
            .and_then(Json::as_str)
            .ok_or_else(|| format!("source records require a string field {key:?}"))?;
        validate_key(record_key)?;
        if !seen.insert(record_key.to_owned()) {
            return Err(format!("duplicate source key {record_key:?}"));
        }
        let retained = previous(record_key)
            .filter(|old| Arc::ptr_eq(&old.record.value, &value) || old.record.value == value);
        let record_bytes = match &retained {
            Some(record) => record.bytes,
            None => encoded_bytes(&value)?,
        };
        bytes += record_bytes;
        if bytes > MAX_TOTAL_BYTES {
            return Err(format!(
                "source data exceeds {MAX_TOTAL_BYTES} encoded bytes"
            ));
        }
        let record_key = record_key.to_owned();
        records.push(retained.unwrap_or_else(|| {
            Arc::new(StoredRecord {
                record: Record {
                    key: record_key,
                    value,
                    revision,
                },
                bytes: record_bytes,
            })
        }));
    }
    Ok((records, bytes))
}
