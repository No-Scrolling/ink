use super::*;

#[derive(Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Template {
    id: usize,
    #[serde(rename = "type")]
    kind: HostKind,
    props: Map<String, Json>,
    children: Vec<Template>,
}

pub(super) struct RowEvent {
    target: usize,
    key: String,
    list: Option<usize>,
}

struct Row {
    root: usize,
    nodes: Vec<usize>,
    item: Json,
}

pub(super) struct NativeList {
    template: Template,
    event_list: Option<usize>,
    items: FxHashMap<String, Json>,
    keys: Arc<[String]>,
    indices: FxHashMap<String, usize>,
    versions: Arc<[u64]>,
    revision: u64,
    start: usize,
    rows: FxHashMap<String, Row>,
    gap: f32,
    follow_end: bool,
    initial_end: bool,
    older: bool,
    more: bool,
    first_requested: Option<(usize, String)>,
    last_requested: Option<(usize, String)>,
}

impl NativeList {
    pub(super) fn boundary_events(&mut self, id: usize, start: usize, near_end: bool, events: &mut Vec<Json>) {
        if let Some(last) = self.keys.last() {
            let boundary = (self.keys.len(), last.clone());
            if self.more && near_end && self.last_requested.as_ref() != Some(&boundary) {
                self.last_requested = Some(boundary);
                events.push(json!({"type":"event", "id":id, "name":"onEndReached", "args":[]}));
            }
        }
        if let Some(first) = self.keys.first() {
            let boundary = (self.keys.len(), first.clone());
            if self.older && start <= 8 && self.first_requested.as_ref() != Some(&boundary) {
                self.first_requested = Some(boundary);
                events.push(json!({"type":"event", "id":id, "name":"onStartReached", "args":[]}));
            }
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
        ensure!(!matches!(template.kind, HostKind::Root | HostKind::NativeView | HostKind::NativeList | HostKind::List | HostKind::PlayingScreen), "unsupported row template node");
        let id = match previous.and_then(|row| row.nodes.get(*cursor)) { Some(id) => *id, None => self.native_id()? };
        *cursor += 1;
        nodes.push(id);
        let mut props = template.props.iter().map(|(name, value)| Ok((name.clone(), resolve(value, item)?))).collect::<Result<Map<_, _>>>()?;
        if event_list.is_some() { props.retain(|_, value| !value.is_null()); }
        let hidden = props.remove("hidden").map(|value| value.as_bool().context("hidden must be a boolean")).transpose()?.unwrap_or(false);
        HostProps::new(template.kind, props.clone())?;
        if previous.is_some() {
            operations.push(Operation::Update { id, props });
        } else {
            self.native_events.insert(id, RowEvent { target: template.id, key: key.to_owned(), list: event_list });
            operations.push(Operation::Create { id, r#type: template.kind, props });
            operations.push(Operation::Insert { id, parent, before: None });
        }
        operations.push(Operation::Hidden { id, value: hidden });
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
            let item = &list.items[key];
            let previous = list.rows.get(key);
            if previous.is_none_or(|row| row.item != *item) {
                let mut nodes = Vec::new();
                let root = self.row_operations(&list.template, item, key, id, previous, &mut 0, &mut nodes, operations, 0, list.event_list)?;
                list.rows.insert(key.clone(), Row { root, nodes, item: item.clone() });
            }
            // Reinsert retained rows in their current keyed order.
            operations.push(Operation::Insert { id: list.rows[key].root, parent: id, before: None });
        }
        list.start = start;
        Ok(())
    }

    pub(super) fn expand_native_lists(&mut self, operations: Vec<Operation>, engine: &mut Engine) -> Result<Vec<Operation>> {
        if !cfg!(feature = "ui-lists") { return Ok(operations); }
        let mut expanded = Vec::with_capacity(operations.len());
        for operation in operations {
            let (id, props) = match &operation {
                Operation::Create { id, r#type: HostKind::NativeList, props } => (*id, props),
                Operation::Update { id, props } if self.native_lists.contains_key(id) => (*id, props),
                _ => { expanded.push(operation); continue; }
            };
            if !props.contains_key("items") && !props.contains_key("itemKeys") {
                if let Some(mut list) = self.native_lists.remove(&id) {
                    let key_field = props.get("keyField").or_else(|| props.get("key")).and_then(Json::as_str).context("NativeList requires a key field")?;
                    if let Some(template) = props.get("template") {
                        let template: Template = serde_json::from_str(template.as_str().context("invalid native list template")?)?;
                        ensure!(template == list.template, "native list templates cannot change after mount");
                    }
                    let changes = props.get("itemChanges").and_then(Json::as_array).context("NativeList requires itemChanges")?;
                    ensure!(changes.len() <= 100_000, "native list patch is too large");
                    let mut seen = FxHashSet::default();
                    let rows = changes.iter().map(|item| {
                        let key = item.get(key_field).and_then(Json::as_str).context("native list keys must be strings")?;
                        let index = *list.indices.get(key).context("invalid native list patch key")?;
                        ensure!(seen.insert(index), "duplicate native list patch key");
                        Ok(index)
                    }).collect::<Result<Vec<_>>>()?;
                    let revision = list.revision + 1;
                    if !rows.is_empty() {
                        let versions = Arc::make_mut(&mut list.versions);
                        for (&index, item) in rows.iter().zip(changes) {
                            let key = &list.keys[index];
                            list.items.insert(key.clone(), item.clone());
                            versions[index] = revision;
                            if let Some(previous) = list.rows.get(key) {
                                let mut nodes = Vec::new();
                                let root = self.row_operations(&list.template, item, key, id, Some(previous), &mut 0,
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
            let (items, keys) = if let Some(data) = props.get("items") {
                let data = data.as_array().context("NativeList requires an items array")?;
                ensure!(data.len() <= 100_000, "native list is too large");
                let mut items = FxHashMap::default();
                let mut keys = Vec::with_capacity(data.len());
                for item in data {
                    let key = item.get(key_field).and_then(Json::as_str).context("native list keys must be strings")?.to_owned();
                    if old.as_ref().and_then(|list| list.items.get(&key)) != Some(item) { changed.insert(key.clone()); }
                    ensure!(items.insert(key.clone(), item.clone()).is_none(), "native list keys must be unique");
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
                let mut items = std::mem::take(&mut list.items);
                for item in changes {
                    let key = item.get(key_field).and_then(Json::as_str).context("native list keys must be strings")?;
                    ensure!(wanted.contains(&key.to_owned()) && changed.insert(key.to_owned()), "invalid native list patch key");
                    items.insert(key.to_owned(), item.clone());
                }
                items.retain(|key, _| wanted.contains(key));
                ensure!(keys.iter().all(|key| items.contains_key(key)), "native list patch is missing a row");
                (items, keys)
            };
            let revision = old.as_ref().map_or(1, |list| list.revision + 1);
            let old_versions: FxHashMap<_, _> = old.as_ref().map(|list| list.keys.iter().zip(list.versions.iter()).collect()).unwrap_or_default();
            let versions = keys.iter().map(|key| if changed.contains(key) { revision }
                else { old_versions.get(key).map_or(revision, |value| **value) }).collect::<Vec<_>>();
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
                old.items = items; old.keys = keys.into(); old.versions = versions.into(); old.revision = revision;
                old
            } else {
                NativeList { template: template.context("NativeList requires a template")?, event_list: (props.get("onRowEvent") == Some(&Json::Bool(true))).then_some(id), items, indices: keys.iter().enumerate().map(|(index, key)| (key.clone(), index)).collect(), keys: keys.into(), versions: versions.into(), revision, start: 0, rows: FxHashMap::default(), gap,
                    follow_end: false, initial_end: props.get("initialEnd") == Some(&Json::Bool(true)), older: false, more: false, first_requested: None, last_requested: None }
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
        Ok(expanded)
    }

    pub(super) fn render_native_list(&self, id: usize, host: &HostNode, depth: usize) -> Result<Option<Node>> {
        let list = self.native_lists.get(&id).context("native list is not mounted")?;
        Ok(Some(Node { identity: NodeIdentity(id), kind: NodeKind::ReactList {
            children: self.children(host, depth)?, start: list.start, keys: list.keys.clone(), content_versions: list.versions.clone(),
            revision: list.revision, gap: list.gap, follow_end: list.follow_end,
        } }))
    }

    pub fn refresh_native_lists(&mut self, engine: &mut Engine) -> Result<()> {
        if !cfg!(feature = "ui-lists") { return Ok(()); }
        // Each pass measures the newly materialised rows and refines the estimated window.
        for _ in 0..8 {
            let Some(clip) = engine.scene.scroll_clip else { break; };
            let mut initial_end = false;
            for id in engine.react_list_positions.keys() {
                if let Some(list) = self.native_lists.get_mut(id) { initial_end |= std::mem::take(&mut list.initial_end); }
            }
            if initial_end { engine.scroll_by(f32::MAX); }
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
mod tests {
    use super::*;

    fn props(items: Vec<Json>) -> Map<String, Json> {
        json!({"key":"id", "items":items, "gap":12, "template":json!({
            "id":20,"type":"Stack","props":{"gap":0},"children":[
                {"id":21,"type":"Text","props":{"text":{"$value":0,"path":["label"]},"onPress":true},"children":[]}
            ]
        }).to_string()}).as_object().unwrap().clone()
    }

    fn fixture() -> (ReactTree, Engine, Vec<Json>) {
        let mut tree = ReactTree::default();
        let mut engine = Engine::new(); engine.set_viewport(1080, 1240);
        let items: Vec<_> = (0..1000).map(|i| json!({"id":format!("row-{i}"),"label":format!("Row {i}")})).collect();
        tree.apply(ReactCommit(vec![
            Operation::Create { id:1, r#type:HostKind::Screen, props:Map::new() },
            Operation::Create { id:2, r#type:HostKind::NativeList, props:props(items.clone()) },
            Operation::Insert { id:2,parent:1,before:None }, Operation::Insert { id:1,parent:0,before:None },
        ]), &mut engine).unwrap();
        tree.refresh_native_lists(&mut engine).unwrap();
        (tree, engine, items)
    }

    #[test]
    fn sparse_patch_preserves_key_storage_and_invalidates_only_changed_heights() {
        let (mut tree, mut engine, _) = fixture();
        let keys = tree.native_lists[&2].keys.clone();
        let untouched = tree.native_lists[&2].versions[1];
        tree.apply(ReactCommit(vec![Operation::Update { id: 2, props: json!({
            "key": "id", "gap": 12, "itemChanges": [{"id":"row-0", "label":"A much taller row ".repeat(80)}]
        }).as_object().unwrap().clone() }]), &mut engine).unwrap();
        tree.refresh_native_lists(&mut engine).unwrap();
        assert!(Arc::ptr_eq(&keys, &tree.native_lists[&2].keys));
        assert_eq!(tree.native_lists[&2].versions[1], untouched);
        assert!(tree.native_lists[&2].versions[0] > untouched);
        assert!(engine.list_metrics[&2].offset(1) > engine.list_metrics[&2].offset(2) - engine.list_metrics[&2].offset(1));
        engine.set_viewport(720, 1240);
        tree.refresh_native_lists(&mut engine).unwrap();
        engine.set_viewport(1080, 1240);
        tree.refresh_native_lists(&mut engine).unwrap();
        assert!(engine.list_viewports_ready());
    }

    #[test]
    fn native_window_is_bounded_and_scroll_needs_no_javascript() {
        let (mut tree, mut engine, _) = fixture();
        for _ in 0..100 {
            engine.scroll_by(1200.0);
            assert!(tree.viewport_events(&mut engine).unwrap().is_empty());
            assert!(engine.list_viewports_ready());
            assert!(tree.native_lists[&2].rows.len() < 80);
            assert!(tree.nodes.len() < 170);
        }
        assert!(tree.native_lists[&2].start > 100);
        assert!(tree.native_events.len() < 160);
    }

    #[test]
    fn keyed_updates_preserve_anchor_identity_and_route_row_actions() {
        let (mut tree, mut engine, mut items) = fixture();
        engine.scroll_by(20_000.0); tree.refresh_native_lists(&mut engine).unwrap();
        let metrics = &engine.list_metrics[&2];
        let top = engine.react_list_positions[&2];
        let index = metrics.index_at(engine.scroll_offset + engine.scene.scroll_clip.unwrap().y - top);
        let key = metrics.keys[index].clone();
        let old_y = top + metrics.offset(index) - engine.scroll_offset;
        let root = tree.native_lists[&2].rows[&key].root;
        let text = tree.native_lists[&2].rows[&key].nodes[1];
        let routed: Json = serde_json::from_str(&tree.route_native_message(json!({"type":"event","id":text,"name":"onPress","args":[]}).to_string()).unwrap()).unwrap();
        assert_eq!(routed, json!({"type":"event","id":21,"name":"onPress","args":[key]}));
        items.insert(0, json!({"id":"prepended","label":"Prepended"}));
        tree.apply(ReactCommit(vec![Operation::Update { id:2,props:props(items.clone()) }]), &mut engine).unwrap();
        tree.refresh_native_lists(&mut engine).unwrap();
        assert_eq!(tree.native_lists[&2].rows[&key].root, root);
        let metrics = &engine.list_metrics[&2];
        let index = metrics.keys.iter().position(|item| item == &key).unwrap();
        let y = engine.react_list_positions[&2] + metrics.offset(index) - engine.scroll_offset;
        assert!((y-old_y).abs()<1.0, "anchor moved: {old_y} to {y}");
        items.reverse();
        tree.apply(ReactCommit(vec![Operation::Update { id:2,props:props(items.clone()) }]), &mut engine).unwrap();
        tree.refresh_native_lists(&mut engine).unwrap();
        assert_eq!(tree.native_lists[&2].rows[&key].root, root);
        items.retain(|item| item["id"] != key);
        tree.apply(ReactCommit(vec![Operation::Update { id:2,props:props(items) }]), &mut engine).unwrap();
        tree.refresh_native_lists(&mut engine).unwrap();
        assert!(!tree.nodes.contains_key(&root));
        assert!(!tree.native_events.contains_key(&text));
        tree.apply(ReactCommit(vec![Operation::Remove { id:1,parent:0 }]), &mut engine).unwrap();
        assert!(tree.native_lists.is_empty()); assert!(tree.native_events.is_empty());
    }

    #[test]
    fn boundaries_request_once_and_empty_lists_release_rows() {
        let (mut tree, mut engine, items) = fixture();
        let mut properties = props(items.clone());
        properties.insert("hasOlder".into(), json!(true));
        properties.insert("onStartReached".into(), json!(true));
        properties.insert("onEndReached".into(), json!(true));
        properties.insert("followEnd".into(), json!(true));
        tree.apply(ReactCommit(vec![Operation::Update { id:2,props:properties.clone() }]), &mut engine).unwrap();
        let events = tree.viewport_events(&mut engine).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["name"], "onStartReached");
        assert!(tree.viewport_events(&mut engine).unwrap().is_empty());
        engine.scroll_by(f32::MAX);
        let events = tree.viewport_events(&mut engine).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["name"], "onEndReached");
        assert!(tree.viewport_events(&mut engine).unwrap().is_empty());
        properties["items"].as_array_mut().unwrap().push(json!({"id":"last","label":"Last"}));
        tree.apply(ReactCommit(vec![Operation::Update { id:2,props:properties.clone() }]), &mut engine).unwrap();
        let events = tree.viewport_events(&mut engine).unwrap();
        assert_eq!(events.len(), 1);
        assert!((engine.scroll_offset - engine.scroll_max).abs() < 1.0);
        properties.insert("items".into(), json!([]));
        tree.apply(ReactCommit(vec![Operation::Update { id:2,props:properties }]), &mut engine).unwrap();
        assert!(tree.viewport_events(&mut engine).unwrap().is_empty());
        assert!(tree.native_lists[&2].rows.is_empty());
        assert!(tree.native_events.is_empty());
    }
}
