use std::{collections::{HashMap, HashSet}, sync::OnceLock};

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
        r#type: HostKind,
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

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
enum HostKind {
    #[default]
    #[serde(skip)]
    Root,
    #[serde(rename = "#text")]
    RawText,
    List,
    Navigator,
    Tab,
    Tabs,
    Confirmation,
    Screen,
    Stack,
    Text,
    TextInput,
    Barcode,
    CameraPreview,
    Image,
    Icon,
    Toggle,
    Button,
    Field,
    Row,
    Message,
    MessageQuote,
    ConversationComposer,
    PlayingLayout,
    PlayingTransport,
    PlayingPressable,
    PlayingProgress,
}

enum HostProps {
    RawText(Box<str>),
    Text {
        size: Option<f32>,
        align: TextAlign,
        max_lines: Option<u32>,
    },
    Other(Map<String, Json>),
}

impl Default for HostProps {
    fn default() -> Self {
        Self::Other(Map::new())
    }
}

impl HostProps {
    fn new(kind: HostKind, mut props: Map<String, Json>) -> Result<Self> {
        if kind == HostKind::RawText {
            let Some(Json::String(text)) = props.remove("text") else {
                bail!("invalid React text");
            };
            return Ok(Self::RawText(text.into_boxed_str()));
        }
        let props = Self::Other(props);
        if kind == HostKind::Text {
            let align = match string(&props, "align").unwrap_or("start") {
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
            return Ok(Self::Text {
                size: number(&props, "size")?,
                align,
                max_lines,
            });
        }
        Ok(props)
    }

    fn object(&self) -> Option<&Map<String, Json>> {
        match self {
            Self::Other(props) => Some(props),
            _ => None,
        }
    }

    fn get(&self, key: &str) -> Option<&Json> {
        self.object()?.get(key)
    }
}

#[derive(Default)]
struct HostNode {
    kind: HostKind,
    props: HostProps,
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
    list_windows: HashMap<usize, (usize, usize, u64)>,
}

#[derive(Default)]
struct IconVariants {
    outlined: Option<Mask>,
    filled: Option<Mask>,
    outlined_bounds: OnceLock<Option<crate::Rect>>,
    filled_bounds: OnceLock<Option<crate::Rect>>,
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
    #[cfg(feature = "perf")]
    pub fn memory_diagnostics(&self) -> Json {
        let mut raw_text_nodes = 0;
        let mut raw_text_bytes = 0;
        let mut text_nodes = 0;
        let mut other_property_nodes = 0;
        let mut child_capacity_bytes = 0;
        for (id, node) in &self.nodes {
            if *id == 0 {
                continue;
            }
            child_capacity_bytes += node.children.capacity() * size_of::<usize>();
            match &node.props {
                HostProps::RawText(text) => {
                    raw_text_nodes += 1;
                    raw_text_bytes += text.len();
                }
                HostProps::Text { .. } => text_nodes += 1,
                HostProps::Other(_) => other_property_nodes += 1,
            }
        }
        json!({
            "host_nodes": self.nodes.len().saturating_sub(1),
            "host_node_inline_bytes": self.nodes.len().saturating_sub(1) * size_of::<HostNode>(),
            "raw_text_nodes": raw_text_nodes,
            "raw_text_bytes": raw_text_bytes,
            "text_nodes": text_nodes,
            "other_property_nodes": other_property_nodes,
            "child_id_capacity_bytes": child_capacity_bytes,
        })
    }

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
        let mut scroll_to_end = HashSet::new();
        let mut dismiss_keyboard = HashSet::new();
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
                            props: HostProps::new(r#type, props)?,
                            ..HostNode::default()
                        },
                    );
                }
                Operation::Update { id, props } => {
                    let node = self.node_mut(id)?;
                    if node.kind == HostKind::Screen && props.get("dismissKeyboard").is_some()
                        && props.get("dismissKeyboard") != node.props.get("dismissKeyboard") {
                        dismiss_keyboard.insert(id);
                    }
                    if node.kind == HostKind::Screen && props.get("scrollToEnd").is_some()
                        && props.get("scrollToEnd") != node.props.get("scrollToEnd") {
                        scroll_to_end.insert(id);
                    }
                    node.props = HostProps::new(node.kind, props)?;
                    let mut target = id;
                    while matches!(self.node(target)?.kind, HostKind::RawText | HostKind::Text) {
                        let Some(parent) = self.node(target)?.parent else {
                            break;
                        };
                        if !matches!(
                            self.node(parent)?.kind,
                            HostKind::Text | HostKind::Button | HostKind::Field
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
        engine.list_metrics.retain(|id, _| self.nodes.contains_key(id));
        self.sync_inputs(engine)?;
        if !structural && scroll_to_end.is_empty() && dismiss_keyboard.is_empty()
            && targets.iter().all(|id| {
                self.nodes
                    .get(id)
                    .is_some_and(|node| !matches!(node.kind, HostKind::Tabs | HostKind::Navigator | HostKind::Tab))
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
                if self.node(*id)?.kind == HostKind::Navigator
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
        for id in &scroll_to_end {
            self.scroll_positions.insert(*id, f32::MAX);
        }
        if self.active_screen != Some(screen_id) {
            if let Some(previous) = self.active_screen {
                self.scroll_positions.insert(previous, engine.scroll_offset);
            }
            engine.scroll_offset = self
                .scroll_positions
                .get(&screen_id)
                .copied()
                .unwrap_or_else(|| if self.nodes.get(&screen_id).is_some_and(|node| node.props.get("initialEnd") == Some(&Json::Bool(true))) { f32::MAX } else { 0.0 });
            engine.react_list_positions.clear();
            engine.pointer = None;
            engine.focused_input = None;
            engine.auto_focus_node = None;
            self.active_screen = Some(screen_id);
        }
        if scroll_to_end.contains(&screen_id) {
            engine.scroll_offset = f32::MAX;
            engine.react_list_positions.clear();
        }
        if dismiss_keyboard.contains(&screen_id) {
            engine.focused_input = None;
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
            if node.kind != HostKind::TextInput {
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
        for (id, top) in &engine.react_list_positions {
            let Some(metrics) = engine.list_metrics.get(id) else { continue; };
            let (start, end) = metrics.window(clip.y + engine.scroll_offset - top, clip.height);
            let window = (start, end, metrics.revision);
            if self.list_windows.get(id) != Some(&window) {
                self.list_windows.insert(*id, window);
                events.push(json!({"type":"event", "id": id, "name":"onWindow", "args":[start, end, metrics.revision]}));
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
        if let HostProps::RawText(text) = &node.props {
            return Ok(text.to_string());
        }
        ensure!(
            node.kind == HostKind::Text,
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
        if let HostProps::Text {
            size,
            align,
            max_lines,
        } = &host.props
        {
            let mut node = Node::text(self.text(id, depth)?, *size, *align, *max_lines);
            node.identity = NodeIdentity(id);
            return Ok(Some(node));
        }
        let props = &host.props;
        let mut node = match host.kind {
            HostKind::List => {
                let keys: Vec<String> = serde_json::from_value(props.get("keys").cloned().context("List requires keys")?)?;
                let count = keys.len();
                let content_versions: Vec<u64> = serde_json::from_value(props.get("contentVersions").cloned().context("List requires contentVersions")?)?;
                ensure!(content_versions.len() == count, "List content versions must match keys");
                let start = props.get("start").and_then(Json::as_u64).context("List requires start")? as usize;
                let revision = props.get("revision").and_then(Json::as_u64).context("List requires revision")?;
                let gap = number(props, "gap")?.unwrap_or(0.0);
                ensure!(gap >= 0.0 && ((40.0 + gap) * count as f32).is_finite()
                    && count <= 1_000_000 && start <= count && host.children.len() <= count - start,
                    "invalid List dimensions");
                ensure!(keys.iter().collect::<HashSet<_>>().len() == count, "List keys must be unique");
                Node {
                    identity: NodeIdentity(id),
                    kind: NodeKind::ReactList {
                        children: self.children(host, depth)?, start, keys, content_versions, revision, gap,
                        follow_end: props.get("followEnd") == Some(&Json::Bool(true)),
                    },
                }
            }

            HostKind::Navigator | HostKind::Tab => {
                let mut children = self.children(host, depth)?;
                ensure!(children.len() <= 1, "{:?} requires one screen", host.kind);
                return Ok(children.pop());
            }
            HostKind::Tabs => {
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
                    ensure!(tab.kind == HostKind::Tab, "Tabs requires Tab children");
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
            HostKind::Confirmation => {
                let title = string(props, "title").context("Confirmation requires title")?;
                let label = string(props, "confirmLabel").context("Confirmation requires confirmLabel")?;
                ensure!(props.get("onConfirm") == Some(&Json::Bool(true)), "Confirmation requires onConfirm");
                let mut screen = Node::screen(
                    self.children(host, depth)?,
                    Some(title.to_owned()),
                    props.get("centered") == Some(&Json::Bool(true)),
                );
                if let NodeKind::Screen { footer, .. } = &mut screen.kind {
                    *footer = Some((label.to_owned(),
                        (props.get("pending") != Some(&Json::Bool(true)))
                            .then(|| event(id, "onConfirm", vec![]))));
                }
                screen
            }
            HostKind::ConversationComposer => {
                ensure!((2..=3).contains(&host.children.len()), "Composer requires two or three children");
                Node { identity: NodeIdentity(id), kind: NodeKind::ConversationComposer { children: self.children(host, depth)? } }
            },
            HostKind::Message => Node { identity: NodeIdentity(id), kind: NodeKind::Message { children: self.children(host, depth)?, outgoing: props.get("outgoing") == Some(&Json::Bool(true)) } },
            HostKind::MessageQuote => Node { identity: NodeIdentity(id), kind: NodeKind::MessageQuote { children: self.children(host, depth)? } },
            HostKind::PlayingTransport => {
                ensure!(host.children.len() == 3, "Playing transport requires three controls");
                Node { identity: NodeIdentity(id), kind: NodeKind::PlayingTransport { children: self.children(host, depth)? } }
            },
            HostKind::PlayingLayout => {
                ensure!(host.children.len() == 2, "Playing layout requires content and actions");
                Node { identity: NodeIdentity(id), kind: NodeKind::PlayingLayout { children: self.children(host, depth)?, centred: props.get("centered") == Some(&Json::Bool(true)) } }
            },
            HostKind::PlayingPressable => Node { identity: NodeIdentity(id), kind: NodeKind::PlayingPressable {
                selected: props.get("selected") == Some(&Json::Bool(true)),
                long_action: (props.get("onLongPress") == Some(&Json::Bool(true))).then(|| event(id, "onLongPress", vec![])),
                children: self.children(host, depth)?, action: (props.get("onPress") == Some(&Json::Bool(true))).then(|| event(id, "onPress", vec![])),
            } },
            HostKind::PlayingProgress => Node { identity: NodeIdentity(id), kind: NodeKind::PlayingProgress {
                position: number(props, "position")?.unwrap_or(0.0), duration: number(props, "duration")?.unwrap_or(0.0), seek: props.get("onSeek") == Some(&Json::Bool(true)),
            } },
            HostKind::Row => Node {
                identity: NodeIdentity(id),
                kind: NodeKind::Row {
                    children: self.children(host, depth)?,
                    has_image: props.get("hasImage") == Some(&Json::Bool(true)),
                    action: (props.get("onPress") == Some(&Json::Bool(true)))
                        .then(|| event(id, "onPress", vec![])),
                },
            },
            HostKind::Screen => {
                let right_action = string(props, "rightIcon").map(|icon| {
                    ensure!(props.get("onRightPress") == Some(&Json::Bool(true)), "Screen right action requires onPress");
                    Ok((self.icon(icon, false)?, event(id, "onRightPress", vec![])))
                }).transpose()?;
                let props = props.object().context("invalid Screen properties")?;
                let props: ScreenProps = serde_json::from_value(Json::Object(props.clone()))?;
                let mut screen = Node::screen(self.children(host, depth)?, props.title, props.centered);
                if let NodeKind::Screen { pinned_header, pinned_footer, right_action: action, .. } = &mut screen.kind {
                    *pinned_header = props.pinned_header;
                    *pinned_footer = props.pinned_footer;
                    *action = right_action;
                }
                screen
            }
            HostKind::Stack => {
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
            HostKind::TextInput => {
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
            HostKind::Barcode => {
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
            HostKind::CameraPreview => {
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
            HostKind::Image => {
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
            HostKind::Icon => {
                let tone = match string(props, "tone").unwrap_or("primary") {
                    "primary" => Tone::Primary,
                    "muted" => Tone::Muted,
                    value => bail!("unsupported icon tone {value}"),
                };
                let name = string(props, "name").context("Icon requires a name")?;
                let filled = props.get("filled") == Some(&Json::Bool(true));
                let mask = self.icon(name, filled)?;
                let bounds = if props.get("tight") == Some(&Json::Bool(true)) {
                    let variants = self.icons.get(name).context("missing icon variants")?;
                    let cache = if filled { &variants.filled_bounds } else { &variants.outlined_bounds };
                    *cache.get_or_init(|| mask.content_bounds())
                } else { None };
                Node { identity: NodeIdentity(id), kind: NodeKind::Icon {
                    mask, size: number(props, "size")?.unwrap_or(28.0), tone, bounds,
                } }
            }
            HostKind::Toggle => {
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
            HostKind::Button | HostKind::Field => {
                let mut label = String::new();
                for child in &host.children {
                    label.push_str(&self.text(*child, depth + 1)?);
                }
                let action = (props.get("onPress") == Some(&Json::Bool(true))
                    && props.get("disabled") != Some(&Json::Bool(true)))
                .then(|| event(id, "onPress", vec![]));
                if host.kind == HostKind::Field {
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
            kind => bail!("unsupported Ink component {kind:?}"),
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
    #[serde(default, rename = "pinnedHeader")]
    pinned_header: bool,
    #[serde(default, rename = "pinnedFooter")]
    pinned_footer: bool,
}

fn string<'a>(props: &'a HostProps, key: &str) -> Option<&'a str> {
    props.get(key).and_then(Json::as_str)
}

fn number(props: &HostProps, key: &str) -> Result<Option<f32>> {
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

pub(super) fn event(id: usize, name: &str, args: Vec<Json>) -> Action {
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
        NodeKind::ConversationComposer { children, .. }
        | NodeKind::Message { children, .. }
        | NodeKind::MessageQuote { children, .. }
        | NodeKind::PlayingTransport { children, .. }
        | NodeKind::PlayingLayout { children, .. }
        | NodeKind::PlayingPressable { children, .. }
        | NodeKind::Screen { children, .. }
        | NodeKind::Row { children, .. }
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
        NodeKind::ConversationComposer { children, .. }
        | NodeKind::Message { children, .. }
        | NodeKind::MessageQuote { children, .. }
        | NodeKind::PlayingTransport { children, .. }
        | NodeKind::PlayingLayout { children, .. }
        | NodeKind::PlayingPressable { children, .. }
        | NodeKind::Screen { children, .. }
        | NodeKind::Row { children, .. }
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
