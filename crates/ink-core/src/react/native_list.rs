use super::*;
use ink_protocol::source::{Graph, ViewSnapshot};

// Small lists keep their cheaper flat representation. Promotion and structural
// updates build the persistent tree; subsequent stable-key edits copy its paths.
const AUTOMATIC_SOURCE_MIN_ROWS: usize = 1024;

#[derive(Clone)]
struct AutomaticSource {
    graph: Graph,
    id: u64,
    key: String,
}

impl AutomaticSource {
    fn replace(previous: Option<Self>, key: &str, records: Vec<Arc<Json>>) -> Option<Self> {
        let mut source = previous.filter(|source| source.key == key);
        if let Some(source) = &mut source {
            let revision = source.graph.snapshot(source.id).ok()?.revision;
            source.graph.replace_shared(source.id, revision, records).ok()?;
        } else {
            let mut graph = Graph::default();
            let id = graph.create_shared(key.to_owned(), records).ok()?;
            source = Some(Self { graph, id, key: key.to_owned() });
        }
        source
    }
}

struct SourceUpdate {
    snapshot: Arc<ViewSnapshot>,
    previous_revision: u64,
    changed_keys: Vec<String>,
    structural: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Template {
    id: usize,
    #[serde(rename = "type")]
    kind: HostKind,
    props: Map<String, Json>,
    children: Vec<Template>,
}

#[derive(Clone)]
pub(super) struct RowEvent {
    target: usize,
    key: String,
    list: Option<usize>,
}

#[derive(Clone)]
struct Row {
    root: usize,
    nodes: Vec<usize>,
    item: Arc<Json>,
}

#[derive(Clone)]
pub(super) struct NativeList {
    template: Template,
    event_list: Option<usize>,
    items: FxHashMap<String, Arc<Json>>,
    source: Option<Arc<ViewSnapshot>>,
    source_view: Option<u64>,
    automatic: Option<AutomaticSource>,
    keys: Arc<[String]>,
    indices: FxHashMap<String, usize>,
    versions: Arc<[u64]>,
    revision: u64,
    start: usize,
    rows: FxHashMap<String, Row>,
    gap: f32,
    follow_end: bool,
    older: bool,
    more: bool,
    first_requested: Option<(usize, Option<String>)>,
    last_requested: Option<(usize, Option<String>)>,
}

impl NativeList {
    fn item(&self, key: &str) -> Result<&Arc<Json>> {
        match &self.source {
            Some(snapshot) => snapshot.get(key).map(|record| &record.value),
            None => self.items.get(key),
        }.context("native list is missing a row")
    }

    fn flatten_automatic(&mut self) {
        if self.automatic.take().is_none() { return; }
        let snapshot = self.source.take().expect("automatic source retains its snapshot");
        self.versions = snapshot.iter().map(|record| record.revision).collect();
        self.items = snapshot.iter().map(|record| (record.key.clone(), record.value.clone())).collect();
    }

    pub(super) fn boundary_events(&mut self, id: usize, start: usize, near_end: bool, events: &mut Vec<Json>) {
        let last = (self.keys.len(), self.keys.last().cloned());
        if self.more && near_end && (!self.keys.is_empty() || self.last_requested.is_none())
            && self.last_requested.as_ref() != Some(&last) {
            self.last_requested = Some(last);
            events.push(json!({"type":"event", "id":id, "name":"onEndReached", "args":[]}));
        }
        let first = (self.keys.len(), self.keys.first().cloned());
        if self.older && start <= 8 && (!self.keys.is_empty() || self.first_requested.is_none())
            && self.first_requested.as_ref() != Some(&first) {
            self.first_requested = Some(first);
            events.push(json!({"type":"event", "id":id, "name":"onStartReached", "args":[]}));
        }
    }
}

fn resolve(value: &Json, item: &Json) -> Result<Json> {
    let Some(binding) = value.as_object().filter(|value| value.contains_key("$value")) else { return Ok(value.clone()); };
    ensure!(binding.get("$value") == Some(&json!(0)), "list templates bind row fields; include derived values in the row data");
    let mut selected = item;
    for key in binding.get("path").and_then(Json::as_array).context("invalid row binding path")? {
        selected = match key {
            Json::String(key) => selected.get(key),
            Json::Number(index) => index.as_u64().and_then(|index| selected.get(index as usize)),
            _ => None,
        }.context("row binding path does not exist")?;
    }
    Ok(selected.clone())
}

impl ReactTree {
    #[cfg(feature = "perf")]
    pub(super) fn native_list_storage_diagnostics(&self) -> Json {
        let mut automatic = 0;
        let mut automatic_rows = 0;
        let mut external = 0;
        let mut flat = 0;
        let mut flat_records = 0;
        let mut flat_versions = 0;
        for list in self.native_lists.values() {
            if list.automatic.is_some() {
                automatic += 1;
                automatic_rows += list.keys.len();
            } else if list.source_view.is_some() {
                external += 1;
            } else {
                flat += 1;
            }
            flat_records += list.items.len();
            flat_versions += list.versions.len();
        }
        json!({"automatic": automatic, "automatic_rows": automatic_rows, "external": external,
            "flat": flat, "flat_records": flat_records, "flat_versions": flat_versions})
    }

    pub(super) fn native_id(&mut self) -> Result<usize> {
        ensure!(self.next_native_id as u64 > 9_007_199_254_740_991, "native identifiers exhausted");
        let id = self.next_native_id;
        self.next_native_id -= 1;
        Ok(id)
    }

    pub fn route_native_message(&self, message: String) -> Result<String> {
        if !cfg!(feature = "ui-lists") || self.native_events.is_empty() { return Ok(message); }
        let mut event: Json = serde_json::from_str(&message)?;
        if event.get("type").and_then(Json::as_str) != Some("event") { return Ok(message); }
        let Some(id) = event.get("id").and_then(Json::as_u64).map(|id| id as usize) else { return Ok(message); };
        let Some(row) = self.native_events.get(&id) else { return Ok(message); };
        if let Some(list) = row.list {
            let name = event["name"].clone();
            event["id"] = json!(list);
            event["name"] = json!("onRowEvent");
            let args = event["args"].as_array_mut().context("invalid event arguments")?;
            args.splice(0..0, [json!(row.key), json!(row.target), name]);
        } else {
            event["id"] = json!(row.target);
            event["args"].as_array_mut().context("invalid event arguments")?.insert(0, json!(row.key));
        }
        Ok(event.to_string())
    }

    fn row_operations(&mut self, template: &Template, item: &Json, key: &str, parent: usize,
        previous: Option<&Row>, cursor: &mut usize, nodes: &mut Vec<usize>, operations: &mut Vec<Operation>, depth: usize, event_list: Option<usize>,
    ) -> Result<usize> {
        ensure!(depth < 128 && *cursor < 4096, "native row template is too large");
        ensure!(!matches!(template.kind, HostKind::Root | HostKind::NativeList | HostKind::List | HostKind::PlayingScreen), "unsupported row template node");
        let id = match previous.and_then(|row| row.nodes.get(*cursor)) { Some(id) => *id, None => self.native_id()? };
        *cursor += 1;
        nodes.push(id);
        let mut props = template.props.iter().map(|(name, value)| Ok((name.clone(), resolve(value, item)?))).collect::<Result<Map<_, _>>>()?;
        if event_list.is_some() { props.retain(|_, value| !value.is_null()); }
        let hidden = props.remove("hidden").map(|value| value.as_bool().context("hidden must be a boolean")).transpose()?.unwrap_or(false);
        let prepared = HostProps::new(template.kind, props.clone())?;
        if previous.is_some() {
            if self.node(id)?.props != prepared { operations.push(Operation::Update { id, props }); }
        } else {
            self.native_events.insert(id, RowEvent { target: template.id, key: key.to_owned(), list: event_list });
            operations.push(Operation::Create { id, r#type: template.kind, props });
            operations.push(Operation::Insert { id, parent, before: None });
        }
        if previous.map_or(hidden, |_| self.nodes.get(&id).is_some_and(|node| node.hidden != hidden)) {
            operations.push(Operation::Hidden { id, value: hidden });
        }
        for child in &template.children { self.row_operations(child, item, key, id, previous, cursor, nodes, operations, depth + 1, event_list)?; }
        Ok(id)
    }

    fn mount_window(&mut self, id: usize, list: &mut NativeList, start: usize, end: usize, operations: &mut Vec<Operation>) -> Result<()> {
        let start = start.min(list.keys.len());
        let end = end.min(list.keys.len()).max(start);
        let wanted: FxHashSet<_> = list.keys[start..end].iter().cloned().collect();
        let removed: Vec<_> = list.rows.keys().filter(|key| !wanted.contains(*key)).cloned().collect();
        for key in removed {
            let row = list.rows.remove(&key).unwrap();
            operations.push(Operation::Remove { id: row.root, parent: id });
        }
        for key in &list.keys[start..end] {
            let item = list.item(key)?;
            let previous = list.rows.get(key);
            if previous.is_none_or(|row| !Arc::ptr_eq(&row.item, item) && row.item != *item) {
                let item = item.clone();
                let mut nodes = Vec::new();
                let root = self.row_operations(&list.template, &item, key, id, previous, &mut 0, &mut nodes, operations, 0, list.event_list)?;
                list.rows.insert(key.clone(), Row { root, nodes, item: item.clone() });
            }
            // Reinsert retained rows in their current keyed order.
            operations.push(Operation::Insert { id: list.rows[key].root, parent: id, before: None });
        }
        list.start = start;
        Ok(())
    }

    fn source_list(&mut self, id: usize, props: &Map<String, Json>, update: Option<SourceUpdate>,
        engine: &mut Engine, operations: &mut Vec<Operation>,
    ) -> Result<()> {
        let view = props.get("sourceView").and_then(Json::as_u64).context("NativeList requires a sourceView identifier")?;
        ensure!(!props.contains_key("items") && !props.contains_key("itemChanges") && !props.contains_key("itemKeys"),
            "source lists cannot contain copied row data");
        let template: Option<Template> = props.get("template").map(|value| -> Result<Template> {
            Ok(serde_json::from_str(value.as_str().context("invalid native list template")?)?)
        }).transpose()?;
        let gap = props.get("gap").and_then(Json::as_f64).unwrap_or(47.0) as f32;
        ensure!(gap.is_finite() && gap >= 0.0, "native list gap must be finite and non-negative");
        let mut list = if let Some(list) = self.native_lists.remove(&id) {
            ensure!(template.as_ref().is_none_or(|template| list.template == *template),
                "native list templates cannot change after mount");
            ensure!(list.source_view == Some(view) || update.is_some(), "source binding requires a snapshot");
            list
        } else {
            ensure!(update.is_some(), "source list requires an initial snapshot");
            NativeList {
                template: template.context("NativeList requires a template")?, event_list: None,
                items: FxHashMap::default(), source: None, source_view: None, automatic: None, keys: Arc::from([]),
                indices: FxHashMap::default(), versions: Arc::from([]), revision: 0, start: 0,
                rows: FxHashMap::default(), gap, follow_end: false, older: false, more: false,
                first_requested: None, last_requested: None,
            }
        };
        list.event_list = (props.get("onRowEvent") == Some(&Json::Bool(true))).then_some(id);
        list.gap = gap;
        list.follow_end = props.get("followEnd") == Some(&Json::Bool(true));
        list.more = props.get("onEndReached") == Some(&Json::Bool(true)) && props.get("hasMore") != Some(&Json::Bool(false));
        list.older = props.get("onStartReached") == Some(&Json::Bool(true)) && props.get("hasOlder") == Some(&Json::Bool(true));
        if list.source_view != Some(view) {
            if let Some(metrics) = engine.list_metrics.get_mut(&id) { metrics.reset_versions(); }
            list.source_view = Some(view);
            list.automatic = None;
            list.source = None;
            list.items = FxHashMap::default();
            list.versions = Arc::from([]);
        }
        if let Some(update) = update {
            if list.rows.is_empty() && props.get("initialEnd") == Some(&Json::Bool(true)) {
                list.start = update.snapshot.len().saturating_sub(32);
            }
            self.update_source_list(id, &mut list, update, engine, operations)?;
        }
        self.native_lists.insert(id, list);
        Ok(())
    }

    fn update_source_list(&mut self, id: usize, list: &mut NativeList, update: SourceUpdate,
        engine: &mut Engine, operations: &mut Vec<Operation>,
    ) -> Result<()> {
        let SourceUpdate { snapshot, previous_revision, changed_keys, structural } = update;
        let recover = list.source.is_none() || list.revision != previous_revision;
        if structural || recover {
            let keys = snapshot.keys();
            let indices: FxHashMap<_, _> = keys.iter().enumerate().map(|(index, key)| (key.clone(), index)).collect();
            let visible = engine.list_metrics.get(&id).zip(engine.react_list_positions.get(&id)).zip(engine.scene.scroll_clip)
                .map(|((metrics, top), clip)| metrics.index_at(engine.scroll_offset + clip.y - top)).unwrap_or(list.start);
            let anchor = list.keys.get(visible).and_then(|key| indices.get(key)).copied();
            list.start = anchor.map(|index| index.saturating_sub(visible.saturating_sub(list.start)))
                .unwrap_or(list.start.min(keys.len().saturating_sub(1)));
            list.keys = keys;
            list.indices = indices;
            list.revision = snapshot.revision;
            list.source = Some(snapshot.clone());
            if recover && let Some(metrics) = engine.list_metrics.get_mut(&id) { metrics.reset_versions(); }
            let start = list.start;
            let end = start + list.rows.len().max(32);
            self.mount_window(id, list, start, end, operations)?;
        } else {
            ensure!(snapshot.len() == list.keys.len(), "nonstructural source change altered row count");
            let rows = changed_keys.iter().map(|key| {
                ensure!(snapshot.get(key).is_some(), "source patch is missing a row");
                list.indices.get(key).copied().context("source patch has an unknown key")
            }).collect::<Result<Vec<_>>>()?;
            for key in &changed_keys {
                if let Some(previous) = list.rows.get(key) {
                    let item = snapshot.get(key).context("source patch is missing a row")?.value.clone();
                    let mut nodes = Vec::new();
                    let root = self.row_operations(&list.template, &item, key, id, Some(previous), &mut 0,
                        &mut nodes, operations, 0, list.event_list)?;
                    list.rows.insert(key.clone(), Row { root, nodes, item });
                }
            }
            if let Some(metrics) = engine.list_metrics.get_mut(&id) { metrics.patch_source(previous_revision, &snapshot, &rows); }
            list.revision = snapshot.revision;
            list.source = Some(snapshot.clone());
        }
        operations.push(Operation::Source { id, snapshot, previous_revision, changed_keys, structural });
        Ok(())
    }

    fn source_list_operations(&self, operations: &mut [Operation], sources: &mut FxHashMap<usize, SourceUpdate>)
        -> FxHashMap<usize, Option<usize>> {
        if operations.is_empty() { return FxHashMap::default(); }
        let mut created = FxHashSet::default();
        let mut source_hosts: FxHashSet<_> = sources.keys().copied().collect();
        for operation in operations.iter() {
            let (id, props) = match operation {
                Operation::Create { id, r#type: HostKind::NativeList, props } => {
                    created.insert(*id);
                    (*id, props)
                }
                Operation::Update { id, props } if created.contains(id) || self.native_lists.contains_key(id) => (*id, props),
                _ => continue,
            };
            if props.contains_key("sourceView") || self.native_lists.get(&id).is_some_and(|list| list.source_view.is_some()) {
                source_hosts.insert(id);
            }
        }
        let mut last = FxHashMap::default();
        if source_hosts.is_empty() { return last; }
        let mut templates = FxHashMap::default();
        for (index, operation) in operations.iter().enumerate() {
            if let Operation::Create { id, props, .. } | Operation::Update { id, props } = operation
                && source_hosts.contains(id) {
                last.insert(*id, Some(index));
                if let Some(template) = props.get("template") { templates.entry(*id).or_insert_with(|| template.clone()); }
            }
        }
        // Runtime retains only the final binding for surviving hosts, including
        // when React corrects a mount synchronously before flushing its batch.
        if operations.iter().any(|operation| matches!(operation, Operation::Remove { .. })) {
            let mut parents = FxHashMap::default();
            let mut children: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
            let mut created = FxHashSet::default();
            let mut removed = FxHashSet::default();
            for operation in operations.iter() {
                match operation {
                    Operation::Create { id, .. } => {
                        created.insert(*id);
                        removed.remove(id);
                        parents.insert(*id, None);
                        children.insert(*id, Vec::new());
                    }
                    Operation::Insert { id, parent, before } if *before != Some(*id) => {
                        parents.insert(*id, Some(*parent));
                        children.entry(*parent).or_default().push(*id);
                    }
                    Operation::Remove { id, .. } => {
                        let mut pending = vec![*id];
                        while let Some(id) = pending.pop() {
                            if !removed.insert(id) { continue; }
                            let existing = self.nodes.get(&id).filter(|_| !created.contains(&id));
                            let descendants = existing.into_iter().flat_map(|node| &node.children)
                                .chain(children.get(&id).into_iter().flatten());
                            for &child in descendants {
                                let parent = parents.get(&child).copied().unwrap_or_else(|| self.nodes.get(&child).and_then(|node| node.parent));
                                if parent == Some(id) { pending.push(child); }
                            }
                        }
                    }
                    _ => {}
                }
            }
            for id in removed {
                sources.remove(&id);
                if let Some(operation) = last.get_mut(&id) { *operation = None; }
            }
        }
        // A new list can supply its template on Create and its final binding on
        // Update. Keep that template available without expanding the first view.
        for (&id, &index) in &last {
            if let Some(index) = index
                && let Operation::Create { props, .. } | Operation::Update { props, .. } = &mut operations[index]
                && !props.contains_key("template")
                && let Some(template) = templates.remove(&id) {
                props.insert("template".to_owned(), template);
            }
        }
        last
    }

    pub(super) fn expand_native_lists(&mut self, operations: Vec<Operation>, engine: &mut Engine) -> Result<Vec<Operation>> {
        if !cfg!(feature = "ui-lists") { return Ok(operations); }
        let mut sources = FxHashMap::default();
        let mut operations = operations;
        for operation in operations.extract_if(.., |operation| matches!(operation, Operation::Source { .. })) {
            let Operation::Source { id, snapshot, previous_revision, changed_keys, structural } = operation else { unreachable!() };
            ensure!(sources.insert(id, SourceUpdate { snapshot, previous_revision, changed_keys, structural }).is_none(),
                "duplicate native list source operation");
        }
        let source_operations = self.source_list_operations(&mut operations, &mut sources);
        let mut expanded = Vec::with_capacity(operations.len());
        for (index, mut operation) in operations.into_iter().enumerate() {
            if let Operation::Create { id, props, .. } | Operation::Update { id, props } = &mut operation
                && let Some(last) = source_operations.get(id)
                && *last != Some(index) {
                for field in ["items", "itemChanges", "itemKeys", "template"] { props.remove(field); }
                expanded.push(operation);
                continue;
            }
            let (id, props) = match &operation {
                Operation::Create { id, r#type: HostKind::NativeList, props } => (*id, props),
                Operation::Update { id, props } if self.native_lists.contains_key(id) || source_operations.get(id) == Some(&Some(index)) => (*id, props),
                _ => { expanded.push(operation); continue; }
            };
            if props.contains_key("sourceView") {
                let mut rows = Vec::new();
                self.source_list(id, props, sources.remove(&id), engine, &mut rows)?;
                let mut operation = operation;
                if let Operation::Create { props, .. } | Operation::Update { props, .. } = &mut operation {
                    props.remove("template");
                }
                expanded.push(operation);
                expanded.extend(rows);
                continue;
            }
            ensure!(!sources.contains_key(&id), "source operation requires a sourceView binding");
            if self.native_lists.get(&id).is_some_and(|list| list.source_view.is_some()) {
                ensure!(props.contains_key("items"), "switching from a source requires a full items array");
                if let Some(metrics) = engine.list_metrics.get_mut(&id) { metrics.reset_versions(); }
            }
            if !props.contains_key("items") && !props.contains_key("itemKeys") {
                if let Some(mut list) = self.native_lists.remove(&id) {
                    let key_field = props.get("keyField").or_else(|| props.get("key")).and_then(Json::as_str).context("NativeList requires a key field")?;
                    if let Some(template) = props.get("template") {
                        let template: Template = serde_json::from_str(template.as_str().context("invalid native list template")?)?;
                        ensure!(template == list.template, "native list templates cannot change after mount");
                    }
                    let changes = match props.get("itemChanges") {
                        Some(value) => value.as_array().context("NativeList requires an itemChanges array")?.as_slice(),
                        None => &[],
                    };
                    ensure!(changes.len() <= 100_000, "native list patch is too large");
                    let mut seen = FxHashSet::default();
                    let rows = changes.iter().map(|item| {
                        let key = item.get(key_field).and_then(Json::as_str).context("native list keys must be strings")?;
                        let index = *list.indices.get(key).context("invalid native list patch key")?;
                        ensure!(seen.insert(index), "duplicate native list patch key");
                        Ok(index)
                    }).collect::<Result<Vec<_>>>()?;
                    let revision = list.revision + 1;
                    let mut backed = false;
                    if !rows.is_empty() && let Some(source) = &mut list.automatic {
                        let result = if source.key == key_field {
                            source.graph.set_shared(source.id, list.revision, changes.iter().cloned().map(Arc::new).collect())
                        } else {
                            Err("native list key field changed".to_owned())
                        };
                        if let Ok(changes) = result {
                            backed = true;
                            for change in changes {
                                self.update_source_list(id, &mut list, SourceUpdate {
                                    snapshot: change.snapshot, previous_revision: change.previous_revision,
                                    changed_keys: change.changed_keys, structural: change.structural,
                                }, engine, &mut expanded)?;
                            }
                        } else {
                            // Existing array lists accept data beyond Graph's limits.
                            // A rejected promotion/edit keeps that behaviour via flat storage.
                            list.flatten_automatic();
                        }
                    }
                    if !rows.is_empty() && !backed {
                        let versions = Arc::make_mut(&mut list.versions);
                        for (&index, item) in rows.iter().zip(changes) {
                            let key = &list.keys[index];
                            let item = Arc::new(item.clone());
                            list.items.insert(key.clone(), item.clone());
                            versions[index] = revision;
                            if let Some(previous) = list.rows.get(key) {
                                let mut nodes = Vec::new();
                                let root = self.row_operations(&list.template, &item, key, id, Some(previous), &mut 0,
                                    &mut nodes, &mut expanded, 0, list.event_list)?;
                                list.rows.insert(key.clone(), Row { root, nodes, item: item.clone() });
                            }
                        }
                        if let Some(metrics) = engine.list_metrics.get_mut(&id) { metrics.patch(list.revision, revision, &rows); }
                        list.revision = revision;
                    }
                    let gap = props.get("gap").and_then(Json::as_f64).unwrap_or(47.0) as f32;
                    ensure!(gap.is_finite() && gap >= 0.0, "native list gap must be finite and non-negative");
                    list.gap = gap;
                    list.follow_end = props.get("followEnd") == Some(&Json::Bool(true));
                    list.more = props.get("onEndReached") == Some(&Json::Bool(true)) && props.get("hasMore") != Some(&Json::Bool(false));
                    list.older = props.get("onStartReached") == Some(&Json::Bool(true)) && props.get("hasOlder") == Some(&Json::Bool(true));
                    let mut operation = operation;
                    if let Operation::Update { props, .. } = &mut operation {
                        props.remove("itemChanges"); props.remove("template");
                    }
                    expanded.push(operation);
                    self.native_lists.insert(id, list);
                    continue;
                }
            }
            let mut old = self.native_lists.remove(&id);
            let key_field = props.get("keyField").or_else(|| props.get("key")).and_then(Json::as_str).context("NativeList requires a key field")?;
            let template: Option<Template> = props.get("template").map(|value| -> Result<Template> {
                Ok(serde_json::from_str(value.as_str().context("invalid native list template")?)?)
            }).transpose()?;
            let mut changed = FxHashSet::default();
            let (mut items, keys) = if let Some(data) = props.get("items") {
                let data = data.as_array().context("NativeList requires an items array")?;
                ensure!(data.len() <= 100_000, "native list is too large");
                let mut items = FxHashMap::default();
                let mut keys = Vec::with_capacity(data.len());
                let shared: FxHashMap<_, _> = old.as_ref().and_then(|list| list.source.as_ref())
                    .into_iter().flat_map(|snapshot| snapshot.iter()).map(|record| (&record.key, &record.value)).collect();
                for item in data {
                    let key = item.get(key_field).and_then(Json::as_str).context("native list keys must be strings")?.to_owned();
                    let previous = shared.get(&key).copied().or_else(|| old.as_ref().and_then(|list| list.items.get(&key)));
                    let record = match previous.filter(|previous| previous.as_ref() == item) {
                        Some(previous) => previous.clone(),
                        None => { changed.insert(key.clone()); Arc::new(item.clone()) },
                    };
                    ensure!(items.insert(key.clone(), record).is_none(), "native list keys must be unique");
                    keys.push(key);
                }
                (items, keys)
            } else {
                let list = old.as_mut().context("native list patches require a mounted list")?;
                let changes = props.get("itemChanges").and_then(Json::as_array).context("NativeList requires items or itemChanges")?;
                ensure!(changes.len() <= 100_000, "native list patch is too large");
                let keys = match props.get("itemKeys") {
                    Some(value) => serde_json::from_value::<Vec<String>>(value.clone())?,
                    None => list.keys.to_vec(),
                };
                ensure!(keys.len() <= 100_000, "native list is too large");
                let wanted: FxHashSet<_> = keys.iter().collect();
                ensure!(wanted.len() == keys.len(), "native list keys must be unique");
                let mut items = match &list.source {
                    Some(snapshot) => snapshot.iter().filter(|record| wanted.contains(&record.key))
                        .map(|record| (record.key.clone(), record.value.clone())).collect(),
                    None => std::mem::take(&mut list.items),
                };
                for item in changes {
                    let key = item.get(key_field).and_then(Json::as_str).context("native list keys must be strings")?;
                    ensure!(wanted.contains(&key.to_owned()) && changed.insert(key.to_owned()), "invalid native list patch key");
                    items.insert(key.to_owned(), Arc::new(item.clone()));
                }
                items.retain(|key, _| wanted.contains(key));
                ensure!(keys.iter().all(|key| items.contains_key(key)), "native list patch is missing a row");
                (items, keys)
            };
            let previous_automatic = old.as_mut().and_then(|list| list.automatic.take());
            let retained_automatic = previous_automatic.as_ref().is_some_and(|source| source.key == key_field);
            let automatic = if keys.len() >= AUTOMATIC_SOURCE_MIN_ROWS {
                AutomaticSource::replace(previous_automatic, key_field, keys.iter().map(|key| items[key].clone()).collect())
            } else { None };
            let source = automatic.as_ref().map(|source| source.graph.snapshot(source.id).expect("automatic source exists"));
            let revision = source.as_ref().map_or_else(|| old.as_ref().map_or(1, |list| list.revision + 1), |snapshot| snapshot.revision);
            let versions: Arc<[u64]> = if source.is_some() {
                items = FxHashMap::default();
                if !retained_automatic && let Some(metrics) = engine.list_metrics.get_mut(&id) { metrics.reset_versions(); }
                Arc::from([])
            } else {
                let old_versions: FxHashMap<_, _> = match &old {
                    Some(list) => match &list.source {
                        Some(snapshot) => snapshot.iter().map(|record| (&record.key, record.revision)).collect(),
                        None => list.keys.iter().zip(list.versions.iter().copied()).collect(),
                    },
                    None => FxHashMap::default(),
                };
                keys.iter().map(|key| if changed.contains(key) { revision }
                    else { old_versions.get(key).copied().unwrap_or(revision) }).collect()
            };
            let gap = props.get("gap").and_then(Json::as_f64).unwrap_or(47.0) as f32;
            ensure!(gap.is_finite() && gap >= 0.0, "native list gap must be finite and non-negative");
            let mut list = if let Some(mut old) = old {
                ensure!(template.as_ref().is_none_or(|template| old.template == *template),
                    "native list templates cannot change after mount");
                let visible = engine.list_metrics.get(&id).zip(engine.react_list_positions.get(&id)).zip(engine.scene.scroll_clip)
                    .map(|((metrics, top), clip)| metrics.index_at(engine.scroll_offset + clip.y - top)).unwrap_or(old.start);
                let anchor = old.keys.get(visible).and_then(|key| keys.iter().position(|candidate| candidate == key));
                old.start = anchor.map(|index| index.saturating_sub(visible.saturating_sub(old.start)))
                    .unwrap_or(old.start.min(keys.len().saturating_sub(1)));
                old.indices = keys.iter().enumerate().map(|(index, key)| (key.clone(), index)).collect();
                old.items = items; old.source = source; old.source_view = None; old.automatic = automatic;
                old.keys = old.source.as_ref().map_or_else(|| keys.into(), |snapshot| snapshot.keys());
                old.versions = versions; old.revision = revision;
                old
            } else {
                let indices = keys.iter().enumerate().map(|(index, key)| (key.clone(), index)).collect();
                let keys = source.as_ref().map_or_else(|| keys.into(), |snapshot| snapshot.keys());
                NativeList { template: template.context("NativeList requires a template")?, event_list: (props.get("onRowEvent") == Some(&Json::Bool(true))).then_some(id), items, source, source_view: None, automatic, indices, keys, versions, revision, start: 0, rows: FxHashMap::default(), gap,
                    follow_end: false, older: false, more: false, first_requested: None, last_requested: None }
            };
            list.gap = gap;
            list.follow_end = props.get("followEnd") == Some(&Json::Bool(true));
            list.more = props.get("onEndReached") == Some(&Json::Bool(true)) && props.get("hasMore") != Some(&Json::Bool(false));
            list.older = props.get("onStartReached") == Some(&Json::Bool(true)) && props.get("hasOlder") == Some(&Json::Bool(true));
            if list.rows.is_empty() && props.get("initialEnd") == Some(&Json::Bool(true)) { list.start = list.keys.len().saturating_sub(32); }
            let start = list.start;
            let end = start + list.rows.len().max(32);
            let mut operation = operation;
            if let Operation::Create { props, .. } | Operation::Update { props, .. } = &mut operation {
                for field in ["items", "itemChanges", "itemKeys", "template"] { props.remove(field); }
            }
            expanded.push(operation);
            self.mount_window(id, &mut list, start, end, &mut expanded)?;
            self.native_lists.insert(id, list);
        }
        for (id, update) in sources {
            let mut list = self.native_lists.remove(&id).context("source operation requires a mounted native list")?;
            ensure!(list.source_view.is_some(), "source operation requires a sourceView binding");
            self.update_source_list(id, &mut list, update, engine, &mut expanded)?;
            self.native_lists.insert(id, list);
        }
        Ok(expanded)
    }

    pub(super) fn render_native_list(&self, id: usize, host: &HostNode, depth: usize) -> Result<Option<Node>> {
        let list = self.native_lists.get(&id).context("native list is not mounted")?;
        Ok(Some(Node { identity: NodeIdentity(id), kind: NodeKind::ReactList {
            children: self.children(host, depth)?, start: list.start, keys: list.keys.clone(),
            content_versions: match &list.source {
                Some(snapshot) => super::super::list::ListVersions::Source(snapshot.clone()),
                None => super::super::list::ListVersions::Flat(list.versions.clone()),
            },
            revision: list.revision, gap: list.gap, follow_end: list.follow_end,
        } }))
    }

    pub fn refresh_native_lists(&mut self, engine: &mut Engine) -> Result<()> {
        if !cfg!(feature = "ui-lists") { return Ok(()); }
        // Each pass measures the newly materialised rows and refines the estimated window.
        for _ in 0..8 {
            let Some(clip) = engine.scene.scroll_clip else { break; };
            let windows: Vec<_> = engine.react_list_positions.iter().filter_map(|(id, top)| {
                let list = self.native_lists.get(id)?;
                let metrics = engine.list_metrics.get(id)?;
                let (start, end) = metrics.window(clip.y + engine.scroll_offset - top, clip.height);
                (list.start != start || list.rows.len() != end - start).then_some((*id, start, end))
            }).collect();
            if windows.is_empty() { break; }
            let mut operations = Vec::new();
            for (id, start, end) in windows {
                let mut list = self.native_lists.remove(&id).unwrap();
                self.mount_window(id, &mut list, start, end, &mut operations)?;
                self.native_lists.insert(id, list);
            }
            self.apply(ReactCommit(operations), engine)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/core-native-list.rs"]
mod tests;
