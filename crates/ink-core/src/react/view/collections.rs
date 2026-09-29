use super::*;
use ink_protocol::{CollectionEdit, CollectionPatch};

#[derive(Clone)]
pub(super) struct Collection {
    pub key: String,
    pub keys: Vec<String>,
    items: FxHashMap<String, Arc<Json>>,
    revision: u64,
}

pub(super) struct Delta {
    pub changes: Vec<Json>,
    pub order: bool,
}

impl Collection {
    pub fn new(key: String, items: Vec<Json>) -> Result<Self> {
        let mut collection = Self { key, keys: Vec::new(), items: FxHashMap::default(), revision: 0 };
        collection.reset(items)?;
        Ok(collection)
    }
    fn reset(&mut self, items: Vec<Json>) -> Result<()> {
        ensure!(items.len() <= 100_000, "native collection is too large");
        let mut data = FxHashMap::default();
        let mut keys = Vec::with_capacity(items.len());
        for item in items {
            let key = item.get(&self.key).and_then(Json::as_str).context("collection keys must be strings")?.to_owned();
            ensure!(data.insert(key.clone(), Arc::new(item)).is_none(), "collection keys must be unique");
            keys.push(key);
        }
        self.keys = keys;
        self.items = data;
        Ok(())
    }
    pub fn data(&self) -> Json { Json::Array(self.keys.iter().map(|key| self.items[key].as_ref().clone()).collect()) }
    pub fn selected(&self, path: &[Json]) -> Result<Cow<'_, Json>> {
        let Some((index, path)) = path.split_first() else { return Ok(Cow::Owned(self.data())); };
        let index = index.as_u64().and_then(|index| usize::try_from(index).ok()).context("binding path does not exist")?;
        let key = self.keys.get(index).context("binding path does not exist")?;
        Ok(Cow::Borrowed(select(&self.items[key], path)?))
    }
    pub fn apply(&mut self, patch: CollectionPatch) -> Result<Option<Delta>> {
        if patch.revision <= self.revision { return Ok(None); }
        ensure!(patch.edits.len() <= 100_000, "collection patch is too large");
        let mut changed = FxHashSet::default();
        let mut order = false;
        for edit in patch.edits {
            match edit {
                CollectionEdit::Reverse => { self.keys.reverse(); order = true; }
                CollectionEdit::Reset { items } => {
                    self.reset(items)?;
                    changed = self.keys.iter().cloned().collect();
                    order = true;
                }
                CollectionEdit::Insert { item, before } => {
                    let key = item.get(&self.key).and_then(Json::as_str).context("collection keys must be strings")?.to_owned();
                    ensure!(!self.items.contains_key(&key) && self.keys.len() < 100_000, "invalid collection insertion");
                    let index = self.before(before.as_deref())?;
                    self.keys.insert(index, key.clone());
                    self.items.insert(key.clone(), Arc::new(item));
                    changed.insert(key);
                    order = true;
                }
                CollectionEdit::Update { key, value } => {
                    ensure!(value.get(&self.key).is_none_or(|value| value.as_str() == Some(&key)), "collection keys cannot change");
                    let item = self.items.get_mut(&key).map(Arc::make_mut).and_then(Json::as_object_mut).context("collection record is missing")?;
                    let mut different = false;
                    for (name, value) in value {
                        if item.get(&name) != Some(&value) { item.insert(name, value); different = true; }
                    }
                    if different { changed.insert(key); }
                }
                CollectionEdit::Remove { key } => {
                    ensure!(self.items.remove(&key).is_some(), "collection record is missing");
                    self.keys.retain(|candidate| candidate != &key);
                    changed.remove(&key);
                    order = true;
                }
                CollectionEdit::Move { key, before } => {
                    ensure!(self.items.contains_key(&key), "collection record is missing");
                    if before.as_deref() == Some(&key) { continue; }
                    self.before(before.as_deref())?;
                    self.keys.retain(|candidate| candidate != &key);
                    let index = self.before(before.as_deref())?;
                    self.keys.insert(index, key);
                    order = true;
                }
            }
        }
        self.revision = patch.revision;
        Ok(Some(Delta { changes: changed.iter().filter_map(|key| self.items.get(key).map(|item| item.as_ref().clone())).collect(), order }))
    }
    fn before(&self, key: Option<&str>) -> Result<usize> {
        match key {
            Some(key) => self.keys.iter().position(|candidate| candidate == key).context("collection insertion anchor is missing"),
            None => Ok(self.keys.len()),
        }
    }
}
