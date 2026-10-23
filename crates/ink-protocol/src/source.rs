use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{self, Write};
use std::sync::{Arc, OnceLock};

use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::{Map, Value as Json};

pub const MAX_SOURCES: usize = 64;
pub const MAX_VIEWS: usize = 256;
pub const MAX_ROWS_PER_SOURCE: usize = 1_000_000;
pub const MAX_TOTAL_ROWS: usize = 1_000_000;
pub const MAX_RECORD_BYTES: usize = 1024 * 1024;
pub const MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_KEY_BYTES: usize = 1024;
pub const MAX_FILTER_FIELDS: usize = 128;
pub const MAX_JSON_DEPTH: usize = 64;

#[derive(Clone, Debug)]
pub struct Record {
    pub key: String,
    pub value: Arc<Json>,
    pub revision: u64,
}

pub struct ViewSnapshot {
    pub revision: u64,
    records: Arc<Records>,
    membership: Arc<Membership>,
    key_order: Arc<OnceLock<Arc<[String]>>>,
}

impl ViewSnapshot {
    pub fn len(&self) -> usize {
        self.membership.count(self.records.len)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, key: &str) -> Option<&Record> {
        let index = *self.records.index.get(key)?;
        self.membership
            .contains(index, self.records.len)
            .then(|| &self.records.root.at(index, self.records.len).record)
    }

    pub fn at(&self, index: usize) -> Option<&Record> {
        let position = self.membership.select(index, self.records.len)?;
        Some(&self.records.root.at(position, self.records.len).record)
    }

    pub fn keys(&self) -> Arc<[String]> {
        self.key_order
            .get_or_init(|| {
                self.selected()
                    .map(|record| record.record.key.clone())
                    .collect()
            })
            .clone()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        self.selected().map(|record| &record.record)
    }

    fn selected(&self) -> SelectedRecords<'_> {
        SelectedRecords {
            stack: vec![(&self.records.root, &self.membership)],
        }
    }
}

impl fmt::Debug for ViewSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewSnapshot")
            .field("revision", &self.revision)
            .field("count", &self.len())
            .finish()
    }
}

impl Serialize for ViewSnapshot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("ViewSnapshot", 2)?;
        state.serialize_field("revision", &self.revision)?;
        state.serialize_field("count", &self.len())?;
        state.end()
    }
}

#[derive(Clone, Debug)]
pub struct ViewChange {
    pub view: u64,
    pub previous_revision: u64,
    pub snapshot: Arc<ViewSnapshot>,
    pub changed_keys: Vec<String>,
    pub structural: bool,
}

#[derive(Clone)]
pub struct Graph {
    next_id: u64,
    sources: HashMap<u64, Source>,
    views: HashMap<u64, View>,
    total_rows: usize,
    total_bytes: usize,
}

impl Default for Graph {
    fn default() -> Self {
        Self {
            next_id: 1,
            sources: HashMap::new(),
            views: HashMap::new(),
            total_rows: 0,
            total_bytes: 0,
        }
    }
}

impl Graph {
    pub fn create(&mut self, key: String, records: Vec<Json>) -> Result<u64, String> {
        self.create_shared(key, records.into_iter().map(Arc::new).collect())
    }

    pub fn create_shared(&mut self, key: String, records: Vec<Arc<Json>>) -> Result<u64, String> {
        validate_key(&key)?;
        if self.sources.len() >= MAX_SOURCES {
            return Err(format!("a graph supports at most {MAX_SOURCES} sources"));
        }
        self.check_view_limit()?;
        let (records, bytes) = validate_records(&key, records, 1, |_| None)?;
        let total_rows = self.total_rows + records.len();
        let total_bytes = self.total_bytes + bytes;
        check_capacity(total_rows, total_bytes)?;
        let next_id = self
            .next_id
            .checked_add(1)
            .ok_or("source IDs are exhausted")?;
        let id = self.next_id;
        let records = Arc::new(Records::build(&records, bytes));
        let snapshot = Arc::new(ViewSnapshot {
            revision: 1,
            records: records.clone(),
            membership: Arc::new(Membership::All),
            key_order: Arc::default(),
        });
        self.sources.insert(
            id,
            Source {
                key,
                revision: 1,
                records,
                views: vec![id],
            },
        );
        self.views.insert(
            id,
            View {
                source: id,
                filter: Map::new(),
                snapshot,
            },
        );
        self.next_id = next_id;
        self.total_rows = total_rows;
        self.total_bytes = total_bytes;
        Ok(id)
    }

    pub fn view(&mut self, source_id: u64, filter: Map<String, Json>) -> Result<u64, String> {
        let source = self
            .sources
            .get(&source_id)
            .ok_or("unknown or released source")?;
        self.check_view_limit()?;
        if filter.len() > MAX_FILTER_FIELDS {
            return Err(format!(
                "a source filter supports at most {MAX_FILTER_FIELDS} fields"
            ));
        }
        if filter
            .values()
            .any(|value| value.is_array() || value.is_object())
        {
            return Err("source filters require literal JSON primitive values".into());
        }
        encoded_bytes(&Json::Object(filter.clone()))?;
        let next_id = self
            .next_id
            .checked_add(1)
            .ok_or("source IDs are exhausted")?;
        let id = self.next_id;
        let membership = Membership::from_tree(&source.records.root, source.records.len, &filter);
        let snapshot = Arc::new(ViewSnapshot {
            revision: source.revision,
            records: source.records.clone(),
            membership,
            key_order: Arc::default(),
        });
        self.views.insert(
            id,
            View {
                source: source_id,
                filter,
                snapshot,
            },
        );
        self.sources.get_mut(&source_id).unwrap().views.push(id);
        self.next_id = next_id;
        Ok(id)
    }

    pub fn snapshot(&self, view_id: u64) -> Result<Arc<ViewSnapshot>, String> {
        self.views
            .get(&view_id)
            .map(|view| view.snapshot.clone())
            .ok_or_else(|| "unknown or released source view".into())
    }

    pub fn set(
        &mut self,
        source_id: u64,
        base_revision: u64,
        records: Vec<Json>,
    ) -> Result<Vec<ViewChange>, String> {
        self.set_shared(source_id, base_revision, records.into_iter().map(Arc::new).collect())
    }

    pub fn set_shared(
        &mut self,
        source_id: u64,
        base_revision: u64,
        records: Vec<Arc<Json>>,
    ) -> Result<Vec<ViewChange>, String> {
        let source = self.source_at(source_id, base_revision)?;
        let revision = source
            .revision
            .checked_add(1)
            .ok_or("source revision is exhausted")?;
        let (records, _) = validate_records(&source.key, records, revision, |key| {
            let index = *source.records.index.get(key)?;
            Some(source.records.root.at(index, source.records.len).clone())
        })?;
        let mut replacements = Vec::with_capacity(records.len());
        let mut bytes = source.records.bytes;
        for record in records {
            let index = *source
                .records
                .index
                .get(&record.record.key)
                .ok_or_else(|| format!("unknown source key {:?}", record.record.key))?;
            let previous = source.records.root.at(index, source.records.len);
            if !Arc::ptr_eq(previous, &record) {
                bytes = bytes - previous.bytes + record.bytes;
                replacements.push((index, record));
            }
        }
        if replacements.is_empty() {
            return Ok(Vec::new());
        }
        let total_bytes = self.total_bytes - source.records.bytes + bytes;
        check_capacity(self.total_rows, total_bytes)?;
        let mut root = source.records.root.clone();
        for (index, record) in &replacements {
            root = root.set(*index, source.records.len, record.clone());
        }
        let records = Arc::new(Records {
            index: source.records.index.clone(),
            root,
            len: source.records.len,
            bytes,
        });
        let mut updates = Vec::with_capacity(source.views.len());
        let mut changes = Vec::new();
        for id in &source.views {
            let view = &self.views[id];
            let mut membership = view.snapshot.membership.clone();
            let mut changed_keys = Vec::new();
            let mut structural = false;
            for (index, record) in &replacements {
                let before = membership.contains(*index, records.len);
                let after = matches(&record.record.value, &view.filter);
                if before || after {
                    changed_keys.push(record.record.key.clone());
                }
                if before != after {
                    membership = membership.set(*index, records.len, after);
                    structural = true;
                }
            }
            let changed = !changed_keys.is_empty();
            let snapshot = Arc::new(ViewSnapshot {
                revision: if changed {
                    revision
                } else {
                    view.snapshot.revision
                },
                records: records.clone(),
                membership,
                key_order: if structural {
                    Arc::default()
                } else {
                    view.snapshot.key_order.clone()
                },
            });
            if changed {
                changes.push(ViewChange {
                    view: *id,
                    previous_revision: view.snapshot.revision,
                    snapshot: snapshot.clone(),
                    changed_keys,
                    structural,
                });
            }
            updates.push((*id, snapshot));
        }
        self.publish(source_id, revision, records, updates);
        self.total_bytes = total_bytes;
        Ok(changes)
    }

    pub fn replace(
        &mut self,
        source_id: u64,
        base_revision: u64,
        records: Vec<Json>,
    ) -> Result<Vec<ViewChange>, String> {
        self.replace_shared(source_id, base_revision, records.into_iter().map(Arc::new).collect())
    }

    pub fn replace_shared(
        &mut self,
        source_id: u64,
        base_revision: u64,
        records: Vec<Arc<Json>>,
    ) -> Result<Vec<ViewChange>, String> {
        let source = self.source_at(source_id, base_revision)?;
        let revision = source
            .revision
            .checked_add(1)
            .ok_or("source revision is exhausted")?;
        let previous = self.views[&source_id]
            .snapshot
            .selected()
            .collect::<Vec<_>>();
        let (records, bytes) = validate_records(&source.key, records, revision, |key| {
            source.records.index.get(key).map(|index| previous[*index].clone())
        })?;
        let total_rows = self.total_rows - source.records.len + records.len();
        let total_bytes = self.total_bytes - source.records.bytes + bytes;
        check_capacity(total_rows, total_bytes)?;
        let mut changed_keys = Vec::new();
        let mut same_order = previous.len() == records.len();
        for (index, record) in records.iter().enumerate() {
            same_order &=
                previous.get(index).map(|old| &old.record.key) == Some(&record.record.key);
            if let Some(old_index) = source.records.index.get(&record.record.key) {
                if Arc::ptr_eq(previous[*old_index], record) {
                    continue;
                }
            }
            changed_keys.push(record.record.key.clone());
        }
        let store = Arc::new(Records::build(&records, bytes));
        for record in previous.iter() {
            if !store.index.contains_key(&record.record.key) {
                changed_keys.push(record.record.key.clone());
            }
        }
        if same_order && changed_keys.is_empty() {
            return Ok(Vec::new());
        }
        let mut updates = Vec::with_capacity(source.views.len());
        let mut changes = Vec::new();
        for id in &source.views {
            let view = &self.views[id];
            let membership = Membership::from_tree(&store.root, store.len, &view.filter);
            let structural = view.snapshot.len() != membership.count(records.len())
                || !view
                    .snapshot
                    .selected()
                    .map(|record| &record.record.key)
                    .eq(SelectedRecords {
                        stack: vec![(&store.root, &membership)],
                    }
                    .map(|record| &record.record.key));
            let changed_keys =
                changed_keys
                    .iter()
                    .filter(|key| {
                        let before = source.records.index.get(key.as_str()).is_some_and(|index| {
                            matches(&previous[*index].record.value, &view.filter)
                        });
                        let after = store.index.get(key.as_str()).is_some_and(|index| {
                            matches(&records[*index].record.value, &view.filter)
                        });
                        before || after
                    })
                    .cloned()
                    .collect::<Vec<_>>();
            let changed = structural || !changed_keys.is_empty();
            let snapshot = Arc::new(ViewSnapshot {
                revision: if changed {
                    revision
                } else {
                    view.snapshot.revision
                },
                records: store.clone(),
                membership,
                key_order: if structural {
                    Arc::default()
                } else {
                    view.snapshot.key_order.clone()
                },
            });
            if changed {
                changes.push(ViewChange {
                    view: *id,
                    previous_revision: view.snapshot.revision,
                    snapshot: snapshot.clone(),
                    changed_keys,
                    structural,
                });
            }
            updates.push((*id, snapshot));
        }
        self.publish(source_id, revision, store, updates);
        self.total_rows = total_rows;
        self.total_bytes = total_bytes;
        Ok(changes)
    }

    pub fn release(&mut self, id: u64) -> Result<(), String> {
        if let Some(source) = self.sources.remove(&id) {
            for view in source.views {
                self.views.remove(&view);
            }
            self.total_rows -= source.records.len;
            self.total_bytes -= source.records.bytes;
            return Ok(());
        }
        let view = self
            .views
            .remove(&id)
            .ok_or("unknown or released source view")?;
        self.sources
            .get_mut(&view.source)
            .unwrap()
            .views
            .retain(|view_id| *view_id != id);
        Ok(())
    }

    fn check_view_limit(&self) -> Result<(), String> {
        if self.views.len() >= MAX_VIEWS {
            return Err(format!(
                "a graph supports at most {MAX_VIEWS} views, including source views"
            ));
        }
        Ok(())
    }

    fn source_at(&self, id: u64, revision: u64) -> Result<&Source, String> {
        let source = self.sources.get(&id).ok_or("unknown or released source")?;
        if source.revision != revision {
            return Err(format!(
                "stale source revision {revision}; current revision is {}",
                source.revision
            ));
        }
        Ok(source)
    }

    fn publish(
        &mut self,
        source_id: u64,
        revision: u64,
        records: Arc<Records>,
        updates: Vec<(u64, Arc<ViewSnapshot>)>,
    ) {
        let source = self.sources.get_mut(&source_id).unwrap();
        source.revision = revision;
        source.records = records;
        for (id, snapshot) in updates {
            self.views.get_mut(&id).unwrap().snapshot = snapshot;
        }
    }
}

fn check_capacity(rows: usize, bytes: usize) -> Result<(), String> {
    if rows > MAX_TOTAL_ROWS {
        return Err(format!(
            "a graph supports at most {MAX_TOTAL_ROWS} current records"
        ));
    }
    if bytes > MAX_TOTAL_BYTES {
        return Err(format!(
            "source data exceeds {MAX_TOTAL_BYTES} current encoded bytes"
        ));
    }
    Ok(())
}

#[derive(Clone)]
struct Source {
    key: String,
    revision: u64,
    records: Arc<Records>,
    views: Vec<u64>,
}

#[derive(Clone)]
struct View {
    source: u64,
    filter: Map<String, Json>,
    snapshot: Arc<ViewSnapshot>,
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

// All and Empty compact uniform subtrees. Split nodes retain subtree counts, so
// a filter crossing and rank selection touch only logarithmically many nodes.
enum Membership {
    Empty,
    All,
    Split {
        left: Arc<Membership>,
        right: Arc<Membership>,
        count: usize,
    },
}

impl Membership {
    fn count(&self, len: usize) -> usize {
        match self {
            Self::Empty => 0,
            Self::All => len,
            Self::Split { count, .. } => *count,
        }
    }

    fn from_tree(records: &RecordTree, len: usize, filter: &Map<String, Json>) -> Arc<Self> {
        if filter.is_empty() {
            return Arc::new(Self::All);
        }
        match records {
            RecordTree::Empty => Arc::new(Self::Empty),
            RecordTree::Leaf(record) => Arc::new(if matches(&record.record.value, filter) {
                Self::All
            } else {
                Self::Empty
            }),
            RecordTree::Branch { left, right } => Self::join(
                Self::from_tree(left, len / 2, filter),
                Self::from_tree(right, len - len / 2, filter),
                len,
            ),
        }
    }

    fn join(left: Arc<Self>, right: Arc<Self>, len: usize) -> Arc<Self> {
        let count = left.count(len / 2) + right.count(len - len / 2);
        Arc::new(if count == 0 {
            Self::Empty
        } else if count == len {
            Self::All
        } else {
            Self::Split { left, right, count }
        })
    }

    fn contains(&self, index: usize, len: usize) -> bool {
        match self {
            Self::Empty => false,
            Self::All => true,
            Self::Split { left, right, .. } => {
                let middle = len / 2;
                if index < middle {
                    left.contains(index, middle)
                } else {
                    right.contains(index - middle, len - middle)
                }
            }
        }
    }

    fn select(&self, index: usize, len: usize) -> Option<usize> {
        if index >= self.count(len) {
            return None;
        }
        match self {
            Self::Empty => None,
            Self::All => Some(index),
            Self::Split { left, right, .. } => {
                let middle = len / 2;
                let left_count = left.count(middle);
                if index < left_count {
                    left.select(index, middle)
                } else {
                    right
                        .select(index - left_count, len - middle)
                        .map(|position| position + middle)
                }
            }
        }
    }

    fn set(self: &Arc<Self>, index: usize, len: usize, included: bool) -> Arc<Self> {
        if matches!(
            (self.as_ref(), included),
            (Self::All, true) | (Self::Empty, false)
        ) {
            return self.clone();
        }
        if len == 1 {
            return Arc::new(if included { Self::All } else { Self::Empty });
        }
        let (left, right) = match self.as_ref() {
            Self::Split { left, right, .. } => (left.clone(), right.clone()),
            Self::All | Self::Empty => (self.clone(), self.clone()),
        };
        let middle = len / 2;
        if index < middle {
            Self::join(left.set(index, middle, included), right, len)
        } else {
            Self::join(left, right.set(index - middle, len - middle, included), len)
        }
    }
}

struct SelectedRecords<'a> {
    stack: Vec<(&'a RecordTree, &'a Membership)>,
}

impl<'a> Iterator for SelectedRecords<'a> {
    type Item = &'a Arc<StoredRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((records, membership)) = self.stack.pop() {
            match (records, membership) {
                (RecordTree::Empty, _) | (_, Membership::Empty) => {}
                (RecordTree::Leaf(record), _) => return Some(record),
                (RecordTree::Branch { left, right }, Membership::All) => {
                    self.stack.push((right, membership));
                    self.stack.push((left, membership));
                }
                (
                    RecordTree::Branch { left, right },
                    Membership::Split {
                        left: left_membership,
                        right: right_membership,
                        ..
                    },
                ) => {
                    self.stack.push((right, right_membership));
                    self.stack.push((left, left_membership));
                }
            }
        }
        None
    }
}

fn matches(value: &Json, filter: &Map<String, Json>) -> bool {
    filter
        .iter()
        .all(|(key, expected)| value.get(key) == Some(expected))
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
    if values.len() > MAX_ROWS_PER_SOURCE {
        return Err(format!(
            "a source accepts at most {MAX_ROWS_PER_SOURCE} records"
        ));
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
        let retained = previous(record_key).filter(|old| Arc::ptr_eq(&old.record.value, &value) || old.record.value == value);
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
        records.push(retained.unwrap_or_else(|| Arc::new(StoredRecord {
            record: Record {
                key: record_key,
                value,
                revision,
            },
            bytes: record_bytes,
        })));
    }
    Ok((records, bytes))
}
