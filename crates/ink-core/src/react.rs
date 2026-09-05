use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json, json};

use super::{
    Action, Alignment, Axis, CameraPreviewKind, ControllerId, Engine, ImageFit, ImageSource,
    Justification, Mask, NativeOperation, Node, NodeIdentity, NodeKind, StateId, StateValue, Tab,
    TextAlign, TextInputAction, Tone,
};

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
enum Operation {
    Create {
        id: usize,
        r#type: String,
        props: Map<String, Json>,
    },
    Update {
        id: usize,
        props: Map<String, Json>,
    },
    Insert {
        id: usize,
        parent: usize,
        before: Option<usize>,
    },
    Remove {
        id: usize,
        parent: usize,
    },
    Hidden {
        id: usize,
        value: bool,
    },
}

#[derive(Default)]
struct HostNode {
    kind: String,
    props: Map<String, Json>,
    children: Vec<usize>,
    parent: Option<usize>,
    hidden: bool,
}

pub struct ReactTree {
    nodes: HashMap<usize, HostNode>,
    icons: HashMap<String, IconVariants>,
    inputs: HashMap<usize, InputBinding>,
    free_input_states: Vec<StateId>,
    active_screen: Option<usize>,
    scroll_positions: HashMap<usize, f32>,
    list_windows: HashMap<usize, (usize, usize)>,
}

#[derive(Default)]
struct IconVariants {
    outlined: Option<Mask>,
    filled: Option<Mask>,
}

struct InputBinding {
    state: StateId,
    value: String,
    event_count: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ReactIcon {
    pub name: String,
    pub filled: bool,
    pub id: u64,
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

impl Default for ReactTree {
    fn default() -> Self {
        Self {
            nodes: HashMap::from([(0, HostNode::default())]),
            icons: HashMap::new(),
            inputs: HashMap::new(),
            free_input_states: Vec::new(),
            active_screen: None,
            scroll_positions: HashMap::new(),
            list_windows: HashMap::new(),
        }
    }
}

impl ReactTree {
    pub fn with_icons(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 16 * 1024 * 1024, "icon assets are too large");
        let icons: Vec<ReactIcon> = serde_json::from_slice(bytes)?;
        let mut tree = Self::default();
        for icon in icons {
            ensure!(
                icon.width > 0
                    && icon.height > 0
                    && icon.width <= 512
                    && icon.height <= 512
                    && icon.pixels.len() == usize::from(icon.width) * usize::from(icon.height),
                "invalid icon asset"
            );
            let variants = tree.icons.entry(icon.name).or_default();
            let slot = if icon.filled {
                &mut variants.filled
            } else {
                &mut variants.outlined
            };
            ensure!(slot.is_none(), "duplicate icon asset");
            *slot = Some(Mask::owned(icon.id, icon.width, icon.height, icon.pixels));
        }
        Ok(tree)
    }

    fn icon(&self, name: &str, filled: bool) -> Result<Mask> {
        self.icons.get(name).and_then(|variants| {
            if filled { variants.filled.as_ref() } else { variants.outlined.as_ref() }
        }).cloned()
            .with_context(|| format!("icon {name:?} was not bundled; icon names must appear as string literals in the app"))
    }

    pub fn apply(&mut self, operations: Json, engine: &mut Engine) -> Result<()> {
        let operations: Vec<Operation> = serde_json::from_value(operations)?;
        let structural = operations
            .iter()
            .any(|operation| !matches!(operation, Operation::Update { .. }));
        let mut targets = HashSet::new();
        for operation in operations {
            match operation {
                Operation::Create { id, r#type, props } => {
                    ensure!(
                        id != 0 && !self.nodes.contains_key(&id),
                        "duplicate React node {id}"
                    );
                    ensure!(self.nodes.len() < 100_000, "React tree is too large");
                    self.nodes.insert(
                        id,
                        HostNode {
                            kind: r#type,
                            props,
                            ..HostNode::default()
                        },
                    );
                }
                Operation::Update { id, props } => {
                    self.node_mut(id)?.props = props;
                    let mut target = id;
                    while matches!(self.node(target)?.kind.as_str(), "#text" | "Text") {
                        let Some(parent) = self.node(target)?.parent else {
                            break;
                        };
                        if !matches!(
                            self.node(parent)?.kind.as_str(),
                            "Text" | "Button" | "Field"
                        ) {
                            break;
                        }
                        target = parent;
                    }
                    targets.insert(target);
                }
                Operation::Hidden { id, value } => self.node_mut(id)?.hidden = value,
                Operation::Insert { id, parent, before } => self.insert(id, parent, before)?,
                Operation::Remove { id, parent } => {
                    ensure!(
                        id != 0 && self.node(id)?.parent == Some(parent),
                        "invalid React removal"
                    );
                    self.node_mut(parent)?.children.retain(|child| *child != id);
                    self.remove(id);
                }
            }
        }
        self.sync_inputs(engine)?;
        if !structural
            && targets.iter().all(|id| {
                self.nodes
                    .get(id)
                    .is_some_and(|node| !matches!(node.kind.as_str(), "Tabs" | "Navigator" | "Tab"))
            })
        {
            let patches = targets
                .into_iter()
                .map(|id| self.render_node(id, 0).map(|node| (id, node)))
                .collect::<Result<Vec<_>>>()?;
            if patches
                .iter()
                .all(|(id, node)| node.is_some() && find_node(&engine.root, *id).is_some())
            {
                for (id, node) in patches {
                    let node = node.unwrap();
                    if let Some(previous) = find_node_mut(&mut engine.root, id) {
                        *previous = node;
                    }
                }
                #[cfg(feature = "perf")]
                {
                    engine.perf.incremental_rebuilds += 1;
                }
                engine.relayout_scene();
                return Ok(());
            }
        }
        let roots = &self.node(0)?.children;
        ensure!(roots.len() <= 1, "an Ink app must have one root screen");
        let root = match roots.first() {
            Some(id) => self.render_node(*id, 0)?.unwrap_or_else(empty_screen),
            None => empty_screen(),
        };
        engine.navigation_handler = match roots.first() {
            Some(id)
                if self.node(*id)?.kind == "Navigator"
                    && self.node(*id)?.props.get("onBack") == Some(&Json::Bool(true)) =>
            {
                Some((
                    self.icon("arrow_back_ios", false)?,
                    event_operation(*id, "onBack", vec![]),
                ))
            }
            _ => None,
        };
        let mut screen = &root;
        while let NodeKind::Tabs { value, tabs } = &screen.kind {
            let Some(index) = engine.active_tab_index(value, tabs.len()) else {
                break;
            };
            screen = &tabs[index].screen;
        }
        let screen_id = screen.identity.0;
        if self.active_screen != Some(screen_id) {
            if let Some(previous) = self.active_screen {
                self.scroll_positions.insert(previous, engine.scroll_offset);
            }
            engine.scroll_offset = self
                .scroll_positions
                .get(&screen_id)
                .copied()
                .unwrap_or(0.0);
            engine.pointer = None;
            engine.focused_input = None;
            engine.auto_focus_node = None;
            self.active_screen = Some(screen_id);
        }
        self.scroll_positions
            .retain(|id, _| self.nodes.contains_key(id));
        engine.root = root;
        engine.rebuild_scene();
        Ok(())
    }

    fn sync_inputs(&mut self, engine: &mut Engine) -> Result<()> {
        self.inputs.retain(|id, input| {
            if self.nodes.contains_key(id) {
                return true;
            }
            engine.state[input.state.0] = StateValue::String(String::new());
            engine.text_input_scroll_offsets.remove(&input.state);
            if engine.focused_input == Some(input.state) {
                engine.focused_input = None;
            }
            self.free_input_states.push(input.state);
            false
        });
        for (id, node) in &self.nodes {
            if node.kind != "TextInput" {
                continue;
            }
            let value =
                string(&node.props, "value").context("TextInput requires a string value")?;
            let count = node
                .props
                .get("eventCount")
                .and_then(Json::as_u64)
                .context("TextInput requires an event count")?;
            ensure!(
                node.props.get("onChange") == Some(&Json::Bool(true)),
                "TextInput requires onChange"
            );
            let input = self.inputs.entry(*id).or_insert_with(|| {
                let state = self.free_input_states.pop().unwrap_or_else(|| {
                    let state = StateId(engine.state.len());
                    engine.state.push(StateValue::String(String::new()));
                    state
                });
                InputBinding {
                    state,
                    value: String::new(),
                    event_count: 0,
                }
            });
            // A delayed React commit must not replace newer native keystrokes.
            if count >= input.event_count {
                if engine.focused_input == Some(input.state) {
                    engine.focused_input_cursor = super::text_cursor_boundary(
                        value,
                        engine.focused_input_cursor.min(value.len()),
                    );
                }
                engine.state[input.state.0] = StateValue::String(value.to_owned());
                input.value = value.to_owned();
            }
        }
        Ok(())
    }

    pub fn viewport_events(&mut self, engine: &Engine) -> Vec<Json> {
        self.list_windows
            .retain(|id, _| self.nodes.contains_key(id));
        let Some(clip) = engine.scene.scroll_clip else {
            return vec![];
        };
        let mut events = Vec::new();
        for (id, (top, height, count)) in &engine.react_list_positions {
            let first = ((clip.y + engine.scroll_offset - top) / height)
                .floor()
                .max(0.0) as usize;
            let visible = (clip.height / height).ceil() as usize + 1;
            let start = first.saturating_sub(visible).min(*count);
            let end = first.saturating_add(visible * 2).min(*count);
            let window = (start, end);
            if self.list_windows.get(id) != Some(&window) {
                self.list_windows.insert(*id, window);
                events.push(
                    json!({"type":"event", "id": id, "name":"onWindow", "args":[start, end]}),
                );
            }
        }
        events
    }

    pub fn input_events(&mut self, engine: &Engine) -> Vec<Json> {
        let mut events = Vec::new();
        for (id, input) in &mut self.inputs {
            let Some(StateValue::String(value)) = engine.state.get(input.state.0) else {
                continue;
            };
            if *value != input.value {
                input.value.clone_from(value);
                input.event_count += 1;
                events.push(json!({ "type": "event", "id": id, "name": "onChange",
                    "args": [value, input.event_count] }));
            }
        }
        events
    }

    pub fn submit_event(&self, engine: &Engine) -> Option<Json> {
        let state = engine.focused_input?;
        let (id, input) = self.inputs.iter().find(|(_, input)| input.state == state)?;
        (self.nodes.get(id)?.props.get("onSubmit") == Some(&Json::Bool(true))).then(
            || json!({ "type": "event", "id": id, "name": "onSubmit", "args": [input.value] }),
        )
    }

    fn node(&self, id: usize) -> Result<&HostNode> {
        self.nodes
            .get(&id)
            .with_context(|| format!("unknown React node {id}"))
    }

    fn node_mut(&mut self, id: usize) -> Result<&mut HostNode> {
        self.nodes
            .get_mut(&id)
            .with_context(|| format!("unknown React node {id}"))
    }

    fn insert(&mut self, id: usize, parent: usize, before: Option<usize>) -> Result<()> {
        ensure!(id != 0, "cannot insert the React root");
        if before == Some(id) {
            return Ok(());
        }
        let mut ancestor = Some(parent);
        let mut depth = 0;
        while let Some(current) = ancestor {
            ensure!(current != id, "React tree contains a cycle");
            ensure!(depth < 256, "React tree is too deep");
            ancestor = self.node(current)?.parent;
            depth += 1;
        }
        if let Some(before) = before {
            ensure!(
                self.node(parent)?.children.contains(&before),
                "invalid React insertion point"
            );
        }
        if let Some(previous) = self.node(id)?.parent {
            self.node_mut(previous)?
                .children
                .retain(|child| *child != id);
        }
        let children = &mut self.node_mut(parent)?.children;
        let index = before
            .and_then(|before| children.iter().position(|child| *child == before))
            .unwrap_or(children.len());
        children.insert(index, id);
        self.node_mut(id)?.parent = Some(parent);
        Ok(())
    }

    fn remove(&mut self, id: usize) {
        if let Some(node) = self.nodes.remove(&id) {
            for child in node.children {
                self.remove(child);
            }
        }
    }

    fn text(&self, id: usize, depth: usize) -> Result<String> {
        ensure!(depth < 256, "React text is too deep");
        let node = self.node(id)?;
        if node.hidden {
            return Ok(String::new());
        }
        if node.kind == "#text" {
            return Ok(node
                .props
                .get("text")
                .and_then(Json::as_str)
                .context("invalid React text")?
                .to_owned());
        }
        ensure!(
            node.kind == "Text",
            "text children must be strings or Text components"
        );
        let mut text = String::new();
        for child in &node.children {
            text.push_str(&self.text(*child, depth + 1)?);
        }
        Ok(text)
    }

    fn render_node(&self, id: usize, depth: usize) -> Result<Option<Node>> {
        ensure!(depth < 256, "React tree is too deep");
        let host = self.node(id)?;
        if host.hidden {
            return Ok(None);
        }
        let props = &host.props;
        let mut node = match host.kind.as_str() {
            "List" => {
                let count = props
                    .get("count")
                    .and_then(Json::as_u64)
                    .context("List requires count")? as usize;
                let start = props
                    .get("start")
                    .and_then(Json::as_u64)
                    .context("List requires start")? as usize;
                let row_height =
                    number(props, "itemHeight")?.context("List requires itemHeight")?;
                ensure!(
                    row_height > 0.0
                        && (row_height * count as f32).is_finite()
                        && count <= 1_000_000
                        && start <= count
                        && host.children.len() <= count - start,
                    "invalid List dimensions"
                );
                Node {
                    identity: NodeIdentity(id),
                    kind: NodeKind::ReactList {
                        children: self.children(host, depth)?,
                        start,
                        count,
                        row_height,
                    },
                }
            }

            "Navigator" | "Tab" => {
                let mut children = self.children(host, depth)?;
                ensure!(children.len() <= 1, "{} requires one screen", host.kind);
                return Ok(children.pop());
            }
            "Tabs" => {
                let active = props
                    .get("active")
                    .and_then(Json::as_u64)
                    .context("Tabs requires an active index")?;
                ensure!(
                    !host.children.is_empty() && active < host.children.len() as u64,
                    "invalid active tab"
                );
                let mut tabs = Vec::with_capacity(host.children.len());
                for child in &host.children {
                    let tab = self.node(*child)?;
                    ensure!(tab.kind == "Tab", "Tabs requires Tab children");
                    tabs.push(Tab::new(
                        self.icon(
                            string(&tab.props, "icon").context("Tab requires an icon")?,
                            true,
                        )?,
                        event(*child, "onPress", vec![]),
                        self.render_node(*child, depth + 1)?
                            .unwrap_or_else(empty_screen),
                    ));
                }
                Node::tabs_with_value(active as usize, tabs)
            }
            "Confirmation" => {
                let title = string(props, "title").context("Confirmation requires title")?;
                let label = string(props, "confirmLabel").context("Confirmation requires confirmLabel")?;
                ensure!(props.get("onConfirm") == Some(&Json::Bool(true)), "Confirmation requires onConfirm");
                let mut screen = Node::screen(self.children(host, depth)?, Some(title.to_owned()), false);
                if let NodeKind::Screen { footer, .. } = &mut screen.kind {
                    *footer = Some((label.to_owned(),
                        (props.get("pending") != Some(&Json::Bool(true)))
                            .then(|| event(id, "onConfirm", vec![]))));
                }
                screen
            }
            "Screen" => {
                let props: ScreenProps = serde_json::from_value(Json::Object(props.clone()))?;
                Node::screen(self.children(host, depth)?, props.title, props.centered)
            }
            "Stack" => {
                let axis = match string(props, "axis").unwrap_or("vertical") {
                    "vertical" => Axis::Vertical,
                    "horizontal" => Axis::Horizontal,
                    value => bail!("unsupported stack axis {value}"),
                };
                let align = match string(props, "align").unwrap_or("start") {
                    "start" => Alignment::Start,
                    "center" => Alignment::Centre,
                    "end" => Alignment::End,
                    "stretch" => Alignment::Stretch,
                    value => bail!("unsupported stack alignment {value}"),
                };
                let justify = match string(props, "justify").unwrap_or("start") {
                    "start" => Justification::Start,
                    "center" => Justification::Centre,
                    "end" => Justification::End,
                    "space-between" => Justification::SpaceBetween,
                    value => bail!("unsupported stack justification {value}"),
                };
                Node::stack(
                    self.children(host, depth)?,
                    axis,
                    number(props, "gap")?,
                    align,
                    justify,
                )
            }
            "Text" => {
                let align = match string(props, "align").unwrap_or("start") {
                    "start" => TextAlign::Start,
                    "center" => TextAlign::Centre,
                    "end" => TextAlign::End,
                    "justify" => TextAlign::Justify,
                    value => bail!("unsupported text alignment {value}"),
                };
                let max_lines = props
                    .get("maxLines")
                    .map(|value| {
                        value
                            .as_u64()
                            .and_then(|value| u32::try_from(value).ok())
                            .context("invalid maxLines")
                    })
                    .transpose()?;
                Node::text(
                    self.text(id, depth)?,
                    number(props, "size")?,
                    align,
                    max_lines,
                )
            }
            "TextInput" => {
                let action = match string(props, "action").unwrap_or("search") {
                    "return" => TextInputAction::Return,
                    "search" => TextInputAction::Search,
                    "done" => TextInputAction::Done,
                    value => bail!("unsupported input action {value}"),
                };
                Node::text_input(
                    string(props, "placeholder").unwrap_or(""),
                    self.inputs
                        .get(&id)
                        .context("TextInput has no native state")?
                        .state,
                    action,
                    props.get("autoFocus") == Some(&Json::Bool(true)),
                    self.icon("close", false)?,
                )
            }
            "Barcode" => {
                let value = string(props, "value").context("Barcode requires a value")?;
                let format = string(props, "format").context("Barcode requires a format")?;
                let size = number(props, "size")?.context("Barcode requires a size")?;
                ensure!(size > 0.0 && size <= 1080.0, "Invalid barcode size");
                let source = serde_json::json!({ "value": value, "format": format, "size": size })
                    .to_string();
                Node::image(
                    ImageSource::Native("barcode".into(), source),
                    None,
                    false,
                    false,
                    size,
                    size,
                    ImageFit::Contain,
                )
            }
            "CameraPreview" => {
                let id = props
                    .get("controller")
                    .and_then(Json::as_i64)
                    .context("Camera preview requires a controller")?;
                ensure!(
                    (1..=9_007_199_254_740_991).contains(&id),
                    "Invalid camera controller"
                );
                let kind = match string(props, "kind") {
                    Some("photo") => CameraPreviewKind::Photo,
                    Some("scanner") => CameraPreviewKind::Scanner,
                    _ => bail!("Invalid camera preview kind"),
                };
                Node::camera_preview(ControllerId::new((-id) as usize), kind)
            }
            "Image" => {
                let src = string(props, "src").context("Image requires a source")?;
                let source = if let Some(path) = src.strip_prefix("asset://") {
                    ImageSource::Native("assets".into(), path.to_owned())
                } else if src.starts_with("https://") {
                    ImageSource::Native("network".into(), src.to_owned())
                } else if src.starts_with("ink-camera://")
                    || src.starts_with("ink-camera-review://")
                {
                    ImageSource::Native("camera".into(), src.to_owned())
                } else {
                    bail!("Image sources must be bundled assets or HTTPS URLs");
                };
                let width = number(props, "width")?.context("Image requires a width")?;
                let height = number(props, "height")?.context("Image requires a height")?;
                ensure!(
                    width > 0.0 && height > 0.0,
                    "Image dimensions must be positive"
                );
                let fit = match string(props, "fit").unwrap_or("contain") {
                    "contain" => ImageFit::Contain,
                    "cover" => ImageFit::Cover,
                    value => bail!("unsupported image fit {value}"),
                };
                Node::image(
                    source,
                    None,
                    props.get("bleed") == Some(&Json::Bool(true)),
                    props.get("zoomable") == Some(&Json::Bool(true)),
                    width,
                    height,
                    fit,
                )
            }
            "Icon" => {
                let tone = match string(props, "tone").unwrap_or("primary") {
                    "primary" => Tone::Primary,
                    "muted" => Tone::Muted,
                    value => bail!("unsupported icon tone {value}"),
                };
                Node::icon(
                    self.icon(
                        string(props, "name").context("Icon requires a name")?,
                        false,
                    )?,
                    number(props, "size")?.unwrap_or(28.0),
                    tone,
                )
            }
            "Toggle" => {
                let value = props
                    .get("value")
                    .and_then(Json::as_bool)
                    .context("Toggle requires a boolean value")?;
                ensure!(
                    props.get("onChange") == Some(&Json::Bool(true)),
                    "Toggle requires onChange"
                );
                Node {
                    identity: NodeIdentity(id),
                    kind: NodeKind::Toggle {
                        label: string(props, "label")
                            .context("Toggle requires a label")?
                            .to_owned(),
                        value,
                        action: (props.get("disabled") != Some(&Json::Bool(true)))
                            .then(|| event(id, "onChange", vec![Json::Bool(!value)])),
                        off: Mask::toggle_circle(false),
                        on: Mask::toggle_circle(true),
                    },
                }
            }
            "Button" | "Field" => {
                let mut label = String::new();
                for child in &host.children {
                    label.push_str(&self.text(*child, depth + 1)?);
                }
                let action = (props.get("onPress") == Some(&Json::Bool(true))
                    && props.get("disabled") != Some(&Json::Bool(true)))
                .then(|| event(id, "onPress", vec![]));
                if host.kind == "Field" {
                    Node::field(
                        string(props, "label").context("Field requires a label")?,
                        label,
                        action,
                    )
                } else {
                    Node::button(
                        label,
                        string(props, "icon")
                            .map(|name| self.icon(name, false))
                            .transpose()?,
                        props.get("selected") == Some(&Json::Bool(true)),
                        action,
                    )
                }
            }
            kind => bail!("unsupported Ink component {kind}"),
        };
        node.identity = NodeIdentity(id);
        Ok(Some(node))
    }

    fn children(&self, host: &HostNode, depth: usize) -> Result<Vec<Node>> {
        host.children
            .iter()
            .filter_map(|id| self.render_node(*id, depth + 1).transpose())
            .collect()
    }
}

#[derive(Deserialize)]
struct ScreenProps {
    title: Option<String>,
    #[serde(default)]
    centered: bool,
}

fn string<'a>(props: &'a Map<String, Json>, key: &str) -> Option<&'a str> {
    props.get(key).and_then(Json::as_str)
}

fn number(props: &Map<String, Json>, key: &str) -> Result<Option<f32>> {
    props
        .get(key)
        .map(|value| {
            let value = value.as_f64().with_context(|| format!("invalid {key}"))?;
            ensure!(
                value.is_finite() && (0.0..=100_000.0).contains(&value),
                "invalid {key}"
            );
            Ok(value as f32)
        })
        .transpose()
}

fn empty_screen() -> Node {
    Node::screen(vec![], None, false)
}

fn event(id: usize, name: &str, args: Vec<Json>) -> Action {
    Action::Native {
        operation: event_operation(id, name, args),
    }
}

fn event_operation(id: usize, name: &str, args: Vec<Json>) -> NativeOperation {
    NativeOperation::new(
        "ink",
        "event",
        json!({ "type": "event", "id": id, "name": name, "args": args }).to_string(),
        0,
    )
}

fn find_node(node: &Node, id: usize) -> Option<&Node> {
    if node.identity.0 == id {
        return Some(node);
    }
    match &node.kind {
        NodeKind::Screen { children, .. }
        | NodeKind::Stack { children, .. }
        | NodeKind::ReactList { children, .. } => {
            children.iter().find_map(|node| find_node(node, id))
        }
        NodeKind::Tabs { tabs, .. } => tabs.iter().find_map(|tab| find_node(&tab.screen, id)),
        _ => None,
    }
}

fn find_node_mut(node: &mut Node, id: usize) -> Option<&mut Node> {
    if node.identity.0 == id {
        return Some(node);
    }
    match &mut node.kind {
        NodeKind::Screen { children, .. }
        | NodeKind::Stack { children, .. }
        | NodeKind::ReactList { children, .. } => {
            children.iter_mut().find_map(|node| find_node_mut(node, id))
        }
        NodeKind::Tabs { tabs, .. } => tabs
            .iter_mut()
            .find_map(|tab| find_node_mut(&mut tab.screen, id)),
        _ => None,
    }
}
