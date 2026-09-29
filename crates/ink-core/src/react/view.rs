use super::*;
mod collections;
mod expressions;
use collections::Collection;
use expressions::Expression;
use std::borrow::Cow;

#[derive(Clone)]
pub(super) struct BoundView {
    pub root: usize,
    targets: FxHashMap<usize, Vec<Target>>,
    values: FxHashMap<usize, Json>,
    inputs: FxHashSet<usize>,
    expressions: Vec<Derived>,
    dependants: FxHashMap<usize, Vec<usize>>,
    collections: FxHashMap<usize, Collection>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Derived { source: usize, expression: Expression }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectionDefinition { source: usize, key: String, items: Vec<Json> }

#[derive(Clone)]
struct Target {
    node: usize,
    property: Property,
    path: Vec<Json>,
}

#[derive(Clone)]
enum Property { Text, Hidden, Other(String) }

#[derive(Default)]
struct Patch {
    text: Option<super::super::SmolStr>,
    hidden: Option<bool>,
    props: Map<String, Json>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Description {
    id: usize,
    #[serde(rename = "type")]
    kind: HostKind,
    props: Map<String, Json>,
    children: Vec<Description>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    view: usize,
    values: Vec<(usize, Json)>,
    #[serde(default)]
    expressions: Vec<Derived>,
    #[serde(default)]
    collections: Vec<CollectionDefinition>,
    root: Description,
}

fn select<'a>(mut value: &'a Json, path: &[Json]) -> Result<&'a Json> {
    for part in path {
        value = match part {
            Json::String(key) => value.get(key),
            Json::Number(index) => index.as_u64().and_then(|index| usize::try_from(index).ok())
                .and_then(|index| value.get(index)),
            _ => None,
        }.context("binding path does not exist")?;
    }
    Ok(value)
}

impl BoundView {
    fn selected(&self, source: usize, path: &[Json]) -> Result<Cow<'_, Json>> {
        if let Some(collection) = self.collections.get(&source) { return collection.selected(path); }
        Ok(Cow::Borrowed(select(self.values.get(&source).context("binding value is missing")?, path)?))
    }

    fn update(&mut self, values: Vec<(usize, Json)>, collections: Vec<ink_protocol::CollectionPatch>)
        -> Result<(FxHashSet<usize>, FxHashMap<usize, collections::Delta>)> {
        let mut changed = FxHashSet::default();
        for (source, value) in values {
            ensure!(self.inputs.contains(&source), "invalid native value source");
            if self.values.get(&source) != Some(&value) { self.values.insert(source, value); changed.insert(source); }
        }
        let mut deltas = FxHashMap::default();
        for patch in collections {
            let source = patch.source;
            let collection = self.collections.get_mut(&source).context("native collection is missing")?;
            if let Some(delta) = collection.apply(patch)? {
                if delta.order || !delta.changes.is_empty() { changed.insert(source); deltas.insert(source, delta); }
            }
        }
        let mut pending = std::collections::BTreeSet::<usize>::new();
        for source in &changed {
            if let Some(indices) = self.dependants.get(source) { pending.extend(indices); }
        }
        while let Some(index) = pending.pop_first() {
            let derived = &self.expressions[index];
            let value = derived.expression.evaluate(self)?;
            if self.values.get(&derived.source) == Some(&value) { continue; }
            self.values.insert(derived.source, value);
            changed.insert(derived.source);
            if let Some(indices) = self.dependants.get(&derived.source) { pending.extend(indices); }
        }
        Ok((changed, deltas))
    }

    fn mount(&mut self, tree: &ReactTree, description: Description, parent: usize,
        seen: &mut FxHashSet<usize>, operations: &mut Vec<Operation>, depth: usize,
    ) -> Result<()> {
        let Description { id, kind, mut props, children } = description;
        ensure!(depth < 256 && seen.len() < 65_536, "native view is too large");
        ensure!(id != 0 && id as u64 <= 9_007_199_254_740_991
            && id != self.root && !tree.nodes.contains_key(&id) && seen.insert(id), "invalid native view node identifier");
        ensure!(!matches!(kind, HostKind::Root | HostKind::NativeView), "invalid native view node type");
        for (property, value) in &mut props {
            let Some(binding) = value.as_object().filter(|object| object.contains_key("$value")) else { continue; };
            ensure!(binding.len() == 2, "invalid native binding");
            let source = binding.get("$value").and_then(Json::as_u64)
                .and_then(|value| usize::try_from(value).ok()).context("invalid binding identifier")?;
            let path = binding.get("path").and_then(Json::as_array).context("invalid binding path")?.clone();
            let current = self.selected(source, &path)?.into_owned();
            let property = match property.as_str() {
                "text" if matches!(kind, HostKind::Text | HostKind::RawText) => Property::Text,
                "hidden" => Property::Hidden,
                _ => Property::Other(property.clone()),
            };
            self.targets.entry(source).or_default().push(Target { node: id, property, path });
            *value = current;
        }
        let hidden = props.remove("hidden").map(|value| value.as_bool().context("hidden must be a boolean")).transpose()?.unwrap_or(false);
        HostProps::new(kind, props.clone())?;
        operations.push(Operation::Create { id, r#type: kind, props });
        operations.push(Operation::Insert { id, parent, before: None });
        if hidden { operations.push(Operation::Hidden { id, value: true }); }
        for child in children { self.mount(tree, child, id, seen, operations, depth + 1)?; }
        Ok(())
    }
}

impl ReactTree {
    /// Deliver native database records without constructing an array in JavaScript.
    pub fn replace_view_collection(&mut self, view: usize, source: usize, revision: u64, items: Vec<Json>, engine: &mut Engine) -> Result<bool> {
        if !self.views.contains_key(&view) { return Ok(false); }
        let mut staged_tree = self.clone();
        let mut staged_engine = engine.stage_update();
        staged_tree.apply(ReactCommit(vec![Operation::Values { view, values: Vec::new(), collections: vec![ink_protocol::CollectionPatch {
            source, revision, edits: vec![ink_protocol::CollectionEdit::Reset { items }],
        }] }]), &mut staged_engine)?;
        staged_tree.refresh_native_lists(&mut staged_engine)?;
        engine.commit_update(staged_engine);
        *self = staged_tree;
        Ok(true)
    }

    pub(super) fn expand_views(&mut self, operations: Vec<Operation>) -> Result<Vec<Operation>> {
        if !cfg!(feature = "ui-views") { return Ok(operations); }
        if !operations.iter().any(|operation| matches!(operation,
            Operation::Values { .. } | Operation::Create { r#type: HostKind::NativeView, .. })) {
            return Ok(operations);
        }
        let mut expanded = Vec::with_capacity(operations.len());
        for operation in operations {
            match operation {
                Operation::Create { id, r#type: HostKind::NativeView, mut props } => {
                    let definition = props.remove("definition").context("NativeView requires a definition")?;
                    let definition: Definition = serde_json::from_str(definition.as_str().context("invalid native view definition")?)?;
                    ensure!(!self.views.contains_key(&definition.view), "native view is already mounted");
                    let mut view = BoundView { root: id, targets: FxHashMap::default(), values: definition.values.into_iter().collect(),
                        inputs: FxHashSet::default(), expressions: definition.expressions, dependants: FxHashMap::default(), collections: FxHashMap::default() };
                    view.inputs.extend(view.values.keys());
                    for collection in definition.collections {
                        ensure!(!view.values.contains_key(&collection.source) && !view.collections.contains_key(&collection.source), "duplicate binding source");
                        view.collections.insert(collection.source, Collection::new(collection.key, collection.items)?);
                    }
                    for (index, derived) in view.expressions.iter().enumerate() {
                        ensure!(!view.values.contains_key(&derived.source) && !view.collections.contains_key(&derived.source), "duplicate binding source");
                        let mut dependencies = FxHashSet::default();
                        derived.expression.dependencies(&mut dependencies, 0)?;
                        for source in dependencies {
                            ensure!(view.values.contains_key(&source) || view.collections.contains_key(&source), "native expressions must depend on earlier sources");
                            view.dependants.entry(source).or_default().push(index);
                        }
                        view.values.insert(derived.source, derived.expression.evaluate(&view)?);
                    }
                    let mut children = Vec::new();
                    view.mount(self, definition.root, id, &mut FxHashSet::default(), &mut children, 0)?;
                    expanded.push(Operation::Create { id, r#type: HostKind::NativeView, props: Map::new() });
                    expanded.extend(children);
                    self.views.insert(definition.view, view);
                }
                Operation::Values { view: view_id, values, collections } => {
                    let view = self.views.get_mut(&view_id).context("native view is not mounted")?;
                    let (changed, deltas) = view.update(values, collections)?;
                    let mut patches: FxHashMap<usize, Patch> = FxHashMap::default();
                    for source in changed {
                        let Some(targets) = view.targets.get(&source) else { continue; };
                        for target in targets {
                            let patch = patches.entry(target.node).or_default();
                            if let Some(delta) = deltas.get(&source).filter(|_| target.path.is_empty()
                                && matches!(&target.property, Property::Other(name) if name == "items")) {
                                if self.native_lists.contains_key(&target.node) {
                                    let collection = &view.collections[&source];
                                    patch.props.insert("keyField".into(), json!(collection.key));
                                    patch.props.insert("itemChanges".into(), json!(delta.changes));
                                    if delta.order { patch.props.insert("itemKeys".into(), json!(collection.keys)); }
                                    continue;
                                }
                            }
                            let selected = view.selected(source, &target.path)?;
                            match &target.property {
                                Property::Text => patch.text = Some(selected.as_str().context("bound text must be a string")?.into()),
                                Property::Hidden => patch.hidden = Some(selected.as_bool().context("hidden must be a boolean")?),
                                Property::Other(name) => { patch.props.insert(name.clone(), selected.into_owned()); }
                            }
                        }
                    }
                    let mut ids = Vec::new();
                    let mut texts = Vec::new();
                    for (id, patch) in patches {
                        if !self.nodes.contains_key(&id) {
                            let (kind, props) = expanded.iter_mut().find_map(|operation| match operation {
                                Operation::Create { id: created, r#type, props } if *created == id => Some((*r#type, props)),
                                _ => None,
                            }).context("bound native node is missing")?;
                            if let Some(text) = patch.text { props.insert("text".into(), Json::String(text.to_string())); }
                            for (name, value) in patch.props {
                                if value.is_null() { props.remove(&name); } else { props.insert(name, value); }
                            }
                            HostProps::new(kind, props.clone())?;
                            if let Some(value) = patch.hidden { expanded.push(Operation::Hidden { id, value }); }
                            continue;
                        }
                        let node = self.node(id)?;
                        if let Some(hidden) = patch.hidden {
                            if node.hidden != hidden { expanded.push(Operation::Hidden { id, value: hidden }); }
                        }
                        if patch.props.is_empty() {
                            if let Some(text) = patch.text {
                                if !matches!(&node.props, HostProps::RawText(current) | HostProps::Text { text: current, .. } if current == &text) {
                                    ids.push(id); texts.push(text);
                                }
                            }
                        } else {
                            let mut props = node.props.snapshot();
                            if let Some(text) = patch.text { props.insert("text".into(), Json::String(text.to_string())); }
                            for (name, value) in patch.props {
                                if value.is_null() { props.remove(&name); } else { props.insert(name, value); }
                            }
                            HostProps::new(node.kind, props.clone())?;
                            expanded.push(Operation::Update { id, props });
                        }
                    }
                    if !ids.is_empty() { expanded.push(Operation::Text { ids, values: texts }); }
                }
                operation => expanded.push(operation),
            }
        }
        Ok(expanded)
    }
}

impl HostProps {
    fn snapshot(&self) -> Map<String, Json> {
        match self {
            Self::Other(props) => props.clone(),
            Self::List(list) => list.props.clone(),
            Self::RawText(text) => Map::from_iter([("text".into(), Json::String(text.to_string()))]),
            Self::Text { text, on_press, width, size, align, max_lines, tabular_numbers } => {
                let align = match align {
                    TextAlign::Start => "start", TextAlign::Centre => "center",
                    TextAlign::End => "end", TextAlign::Justify => "justify",
                };
                let mut props = Map::from_iter([
                    ("text".into(), json!(text)), ("onPress".into(), json!(on_press)),
                    ("align".into(), json!(align)), ("tabularNumbers".into(), json!(tabular_numbers)),
                ]);
                if let Some(value) = width { props.insert("width".into(), json!(value)); }
                if let Some(value) = size { props.insert("size".into(), json!(value)); }
                if let Some(value) = max_lines { props.insert("maxLines".into(), json!(value)); }
                props
            }
        }
    }
}
