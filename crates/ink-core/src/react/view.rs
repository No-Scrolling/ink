use super::*;

pub(super) struct BoundView {
    pub root: usize,
    targets: FxHashMap<usize, Vec<Target>>,
}

struct Target {
    node: usize,
    property: Property,
    path: Vec<Json>,
}

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
    fn mount(&mut self, tree: &ReactTree, description: Description, parent: usize,
        values: &FxHashMap<usize, Json>, seen: &mut FxHashSet<usize>, operations: &mut Vec<Operation>, depth: usize,
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
            let current = select(values.get(&source).context("binding value is missing")?, &path)?.clone();
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
        for child in children { self.mount(tree, child, id, values, seen, operations, depth + 1)?; }
        Ok(())
    }
}

impl ReactTree {
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
                    let mut view = BoundView { root: id, targets: FxHashMap::default() };
                    let values: FxHashMap<_, _> = definition.values.into_iter().collect();
                    let mut children = Vec::new();
                    view.mount(self, definition.root, id, &values, &mut FxHashSet::default(), &mut children, 0)?;
                    expanded.push(Operation::Create { id, r#type: HostKind::NativeView, props: Map::new() });
                    expanded.extend(children);
                    self.views.insert(definition.view, view);
                }
                Operation::Values { view, values } => {
                    let view = self.views.get(&view).context("native view is not mounted")?;
                    let mut patches: FxHashMap<usize, Patch> = FxHashMap::default();
                    for (source, value) in values {
                        let Some(targets) = view.targets.get(&source) else { continue; };
                        for target in targets {
                            let selected = select(&value, &target.path)?;
                            let patch = patches.entry(target.node).or_default();
                            match &target.property {
                                Property::Text => patch.text = Some(selected.as_str().context("bound text must be a string")?.into()),
                                Property::Hidden => patch.hidden = Some(selected.as_bool().context("hidden must be a boolean")?),
                                Property::Other(name) => { patch.props.insert(name.clone(), selected.clone()); }
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
                            if let Some(text) = patch.text { ids.push(id); texts.push(text); }
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
