use std::{collections::{HashMap, HashSet}, sync::{Arc, OnceLock}};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use crate::ReactIcon;
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
    ScreenState,
    Stack,
    Canvas,
    CanvasRectangle,
    CanvasText,
    CanvasIcon,
    Text,
    TextInput,
    Barcode,
    CameraPreview,
    MapView,
    VideoView,
    MediaPickerScreen,
    MediaGridRow,
    MediaCell,
    Image,
    Icon,
    Toggle,
    Button,
    Field,
    Row,
    RowTitle,
    Message,
    MessageQuote,
    LinkPreview,
    ConversationComposer,
    PlayingLayout,
    PlayingTransport,
    Pressable,
    PlayingProgress,
    PlayingLabel,
    PitchIndicator,
}

struct ListProps {
    props: Map<String, Json>,
    keys: Arc<[String]>,
    content_versions: Arc<[u64]>,
}

impl ListProps {
    fn new(mut props: Map<String, Json>, previous: Option<&Self>) -> Result<Self> {
        let (keys, content_versions) = if !props.contains_key("keys") && !props.contains_key("contentVersions") {
            let previous = previous.context("List requires keys and contentVersions")?;
            ensure!(props.get("revision") == previous.props.get("revision"), "List metadata revision changed without keys");
            (previous.keys.clone(), previous.content_versions.clone())
        } else {
            let keys: Vec<String> = serde_json::from_value(props.remove("keys").context("List requires keys")?)?;
            let content_versions: Vec<u64> = serde_json::from_value(props.remove("contentVersions").context("List requires contentVersions")?)?;
            ensure!(keys.len() <= 1_000_000 && keys.len() == content_versions.len(), "invalid List metadata length");
            ensure!(keys.iter().collect::<HashSet<_>>().len() == keys.len(), "List keys must be unique");
            (keys.into(), content_versions.into())
        };
        Ok(Self { props, keys, content_versions })
    }
}

enum HostProps {
    RawText(Box<str>),
    Text {
        on_press: bool,
        width: Option<f32>,
        size: Option<f32>,
        align: TextAlign,
        max_lines: Option<u32>,
        tabular_numbers: bool,
    },
    List(Box<ListProps>),
    Other(Map<String, Json>),
}

impl Default for HostProps {
    fn default() -> Self {
        Self::Other(Map::new())
    }
}

impl HostProps {
    fn new(kind: HostKind, mut props: Map<String, Json>) -> Result<Self> {
        if kind == HostKind::List {
            return Ok(Self::List(Box::new(ListProps::new(props, None)?)));
        }
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
                on_press: props.get("onPress") == Some(&Json::Bool(true)),
                width: number(&props, "width")?,
                size: number(&props, "size")?,
                align,
                max_lines,
                tabular_numbers: props.get("tabularNumbers") == Some(&Json::Bool(true)),
            });
        }
        Ok(props)
    }

    fn object(&self) -> Option<&Map<String, Json>> {
        match self {
            Self::Other(props) => Some(props),
            Self::List(list) => Some(&list.props),
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
    list_windows: HashMap<usize, (usize, usize, u64, bool)>,
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
                HostProps::Other(_) | HostProps::List(_) => other_property_nodes += 1,
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
        let icons = ReactIcon::decode(bytes)?;
        let mut tree = Self::default();
        for icon in icons {
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

    fn icon(&self, reference: &str) -> Result<Mask> {
        let (name, filled) = icon_reference(reference);
        self.icons.get(name).and_then(|variants| {
            if filled { variants.filled.as_ref() } else { variants.outlined.as_ref() }
        }).cloned()
            .with_context(|| format!("icon {name:?} was not bundled; import its reference from ink/icons"))
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
                    node.props = if let HostProps::List(previous) = &node.props {
                        HostProps::List(Box::new(ListProps::new(props, Some(previous))?))
                    } else {
                        HostProps::new(node.kind, props)?
                    };
                    let mut target = id;
                    if matches!(self.node(target)?.kind, HostKind::CanvasRectangle | HostKind::CanvasText | HostKind::CanvasIcon) {
                        target = self.node(target)?.parent.context("Canvas item requires a Canvas parent")?;
                    }
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
                    .is_some_and(|node| !matches!(node.kind, HostKind::Tabs | HostKind::Navigator | HostKind::Tab | HostKind::ScreenState))
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
                    self.icon("arrow_back_ios")?,
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
            engine.pointer_screen_changed();
            engine.focused_input = None;
            engine.native_editor_state = None;
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
            if engine.native_editor_state == Some(input.state) {
                engine.native_editor_state = None;
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
            let offset = clip.y + engine.scroll_offset - top;
            let (start, end) = metrics.window(offset, clip.height);
            let near_end = offset + clip.height * 2.0 >= metrics.total();
            let window = (start, end, metrics.revision, near_end);
            if self.list_windows.get(id) != Some(&window) {
                self.list_windows.insert(*id, window);
                events.push(json!({"type":"event", "id": id, "name":"onWindow", "args":[start, end, metrics.revision, near_end]}));
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

    fn text_links(&self, id: usize, depth: usize, offset: &mut usize, links: &mut Vec<(std::ops::Range<usize>, Action)>) -> Result<()> {
        ensure!(depth < 256, "React text is too deep");
        let node = self.node(id)?;
        if node.hidden { return Ok(()); }
        let start = *offset;
        if let HostProps::RawText(text) = &node.props {
            *offset += text.len();
        } else {
            for child in &node.children { self.text_links(*child, depth + 1, offset, links)?; }
            if matches!(&node.props, HostProps::Text { on_press: true, .. }) {
                links.push((start..*offset, event(id, "onPress", vec![])));
            }
        }
        Ok(())
    }

    fn render_node(&self, id: usize, depth: usize) -> Result<Option<Node>> {
        ensure!(depth < 256, "React tree is too deep");
        let host = self.node(id)?;
        if host.hidden {
            return Ok(None);
        }
        if let HostProps::Text {
            width,
            size,
            align,
            max_lines,
            tabular_numbers,
            ..
        } = &host.props
        {
            let mut node = Node::text(self.text(id, depth)?, *size, *align, *max_lines, *tabular_numbers);
            if let NodeKind::Text { width: node_width, links, .. } = &mut node.kind {
                *node_width = *width;
                self.text_links(id, depth, &mut 0, links)?;
            }
            node.identity = NodeIdentity(id);
            return Ok(Some(node));
        }
        let props = &host.props;
        let mut node = match host.kind {
            HostKind::Canvas => {
                let width = number(props, "width")?.context("Canvas requires width")?;
                let height = number(props, "height")?.context("Canvas requires height")?;
                ensure!(width > 0.0 && height > 0.0, "Canvas dimensions must be positive");
                let mut drawings = Vec::with_capacity(host.children.len());
                for child_id in &host.children {
                    let child = self.node(*child_id)?;
                    if child.hidden { continue; }
                    let props = &child.props;
                    let bounds = super::Rect {
                        x: number(props, "x")?.context("Canvas item requires x")?,
                        y: number(props, "y")?.context("Canvas item requires y")?,
                        width: number(props, "width")?.context("Canvas item requires width")?,
                        height: number(props, "height")?.context("Canvas item requires height")?,
                    };
                    let drawing = match child.kind {
                        HostKind::CanvasIcon => super::canvas::Drawing::Icon {
                            bounds,
                            mask: self.icon(string(props, "name").context("CanvasIcon requires a name")?)?,
                            colour: canvas_colour(props, "colour")?.unwrap_or(super::Colour::WHITE),
                        },
                        HostKind::CanvasRectangle => super::canvas::Drawing::Rectangle {
                            bounds,
                            fill: canvas_colour(props, "fill")?,
                            stroke: canvas_colour(props, "stroke")?,
                            stroke_width: number(props, "strokeWidth")?.unwrap_or(1.0),
                            action: (props.get("onPress") == Some(&Json::Bool(true)))
                                .then(|| event(*child_id, "onPress", vec![])),
                            long_action: (props.get("onLongPress") == Some(&Json::Bool(true)))
                                .then(|| event(*child_id, "onLongPress", vec![])),
                            drag_action: (props.get("onDragEnter") == Some(&Json::Bool(true)))
                                .then(|| event(*child_id, "onDragEnter", vec![])),
                        },
                        HostKind::CanvasText => super::canvas::Drawing::Text {
                            bounds,
                            text: string(props, "text").unwrap_or_default().to_owned(),
                            size: number(props, "size")?.unwrap_or(26.0),
                            tabular_numbers: props.get("tabularNumbers") == Some(&Json::Bool(true)),
                            colour: canvas_colour(props, "colour")?.unwrap_or(super::Colour::WHITE),
                            align: match string(props, "align").unwrap_or("center") {
                                "start" => TextAlign::Start,
                                "center" => TextAlign::Centre,
                                "end" => TextAlign::End,
                                _ => bail!("unsupported CanvasText alignment"),
                            },
                        },
                        _ => bail!("Canvas accepts only Rectangle, CanvasText and CanvasIcon children"),
                    };
                    drawings.push(drawing);
                }
                Node { identity: NodeIdentity(id), kind: NodeKind::Canvas { width, height, drawings } }
            }
            HostKind::List => {
                let HostProps::List(list) = props else { bail!("List requires metadata"); };
                let keys = list.keys.clone();
                let count = keys.len();
                let content_versions = list.content_versions.clone();
                let start = props.get("start").and_then(Json::as_u64).context("List requires start")? as usize;
                let revision = props.get("revision").and_then(Json::as_u64).context("List requires revision")?;
                let gap = number(props, "gap")?.unwrap_or(0.0);
                ensure!(gap >= 0.0 && ((40.0 + gap) * count as f32).is_finite()
                    && count <= 1_000_000 && start <= count && host.children.len() <= count - start,
                    "invalid List dimensions");
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
                        self.icon(string(&tab.props, "icon").context("Tab requires an icon")?)?,
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
                    false,
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
            HostKind::LinkPreview => Node { identity: NodeIdentity(id), kind: NodeKind::LinkPreview { children: self.children(host, depth)? } },
            HostKind::PlayingTransport => {
                let loading = props.get("loading") == Some(&Json::Bool(true));
                ensure!(!loading || host.children.len() == 3, "Loading transport requires three controls");
                Node { identity: NodeIdentity(id), kind: NodeKind::PlayingTransport { children: self.children(host, depth)?, loading } }
            },
            HostKind::PlayingLayout => {
                ensure!(host.children.len() == 2, "Playing layout requires content and actions");
                Node { identity: NodeIdentity(id), kind: NodeKind::PlayingLayout {
                    children: self.children(host, depth)?,
                    centred: props.get("centered") == Some(&Json::Bool(true)),
                    hide_controls: props.get("hideControls") == Some(&Json::Bool(true)),
                    bleed: props.get("bleed") == Some(&Json::Bool(true)),
                } }
            },
            HostKind::PlayingLabel => Node { identity: NodeIdentity(id), kind: NodeKind::PlayingLabel {
                text: string(&host.props, "text").unwrap_or_default().to_owned(),
                size: number(&host.props, "size")?.unwrap_or(22.0),
            } },
            HostKind::Pressable => Node { identity: NodeIdentity(id), kind: NodeKind::Pressable {
                haptic: props.get("haptic") != Some(&Json::Bool(false)),
                selected: props.get("selected") == Some(&Json::Bool(true)),
                long_action: (props.get("onLongPress") == Some(&Json::Bool(true))).then(|| event(id, "onLongPress", vec![])),
                children: self.children(host, depth)?, action: (props.get("onPress") == Some(&Json::Bool(true))).then(|| event(id, "onPress", vec![])),
            } },
            HostKind::PitchIndicator => Node { identity: NodeIdentity(id), kind: NodeKind::PitchIndicator {
                cents: props.get("cents").filter(|value| !value.is_null()).map(|value| {
                    let cents = value.as_f64().context("PitchIndicator requires numeric cents")?;
                    ensure!(cents.is_finite(), "PitchIndicator requires finite cents");
                    Ok(cents.clamp(-50.0, 50.0) as f32)
                }).transpose()?,
            } },
            HostKind::PlayingProgress => Node { identity: NodeIdentity(id), kind: NodeKind::PlayingProgress {
                position: number(props, "position")?.unwrap_or(0.0), duration: number(props, "duration")?.unwrap_or(0.0), seek: props.get("onSeek") == Some(&Json::Bool(true)),
                playing: props.get("playing") == Some(&Json::Bool(true)),
            } },
            HostKind::RowTitle => Node { identity: NodeIdentity(id), kind: NodeKind::RowTitle {
                size: number(props, "size")?.unwrap_or(26.0),
                text: string(props, "text").unwrap_or_default().to_owned(),
                max_lines: number(props, "maxLines")?.map(|value| value.max(1.0) as u32),
                children: self.children(host, depth)?,
            } },
            HostKind::Row => Node {
                identity: NodeIdentity(id),
                kind: NodeKind::Row {
                    children: self.children(host, depth)?,
                    has_image: props.get("hasImage") == Some(&Json::Bool(true)),
                    long_action: (props.get("onLongPress") == Some(&Json::Bool(true)))
                        .then(|| event(id, "onLongPress", vec![])),
                    action: (props.get("onPress") == Some(&Json::Bool(true)))
                        .then(|| event(id, "onPress", vec![])),
                },
            },
            HostKind::MediaGridRow => {
                ensure!(host.children.len() <= 3, "Media grid rows have at most three cells");
                Node { identity: NodeIdentity(id), kind: NodeKind::MediaGridRow { children: self.children(host, depth)? } }
            }
            HostKind::MediaCell => {
                let src = string(props, "src").context("Media cell requires a source")?;
                ensure!(src.starts_with("ink-media://") || src.starts_with("ink-file://"), "Media cells require a media or managed file source");
                Node { identity: NodeIdentity(id), kind: NodeKind::MediaCell {
                    source: ImageSource::Native("files".into(), src.to_owned()),
                    selected: props.get("selected") == Some(&Json::Bool(true)),
                    video: props.get("video") == Some(&Json::Bool(true)),
                    check: self.icon(string(props, "checkIcon").context("Media cell requires a check icon")?)?,
                    play: self.icon(string(props, "videoIcon").context("Media cell requires a video icon")?)?,
                    action: (props.get("onPress") == Some(&Json::Bool(true))).then(|| event(id, "onPress", vec![])),
                } }
            }
            HostKind::Screen | HostKind::MediaPickerScreen => {
                let left_action = string(props, "leftIcon").map(|icon| {
                    ensure!(props.get("onLeftPress") == Some(&Json::Bool(true)), "Screen left action requires onPress");
                    Ok((self.icon(icon)?, event(id, "onLeftPress", vec![])))
                }).transpose()?;
                let right_action = string(props, "rightIcon").map(|icon| {
                    ensure!(props.get("onRightPress") == Some(&Json::Bool(true)), "Screen right action requires onPress");
                    Ok((self.icon(icon)?, event(id, "onRightPress", vec![])))
                }).transpose()?;
                let props = props.object().context("invalid Screen properties")?;
                let props: ScreenProps = serde_json::from_value(Json::Object(props.clone()))?;
                let mut state = None;
                for child_id in &host.children {
                    let child = self.node(*child_id)?;
                    if !child.hidden && child.kind == HostKind::ScreenState {
                        state = Some((*child_id, child));
                        break;
                    }
                }
                let mut screen = if let Some((state_id, state)) = state {
                    let message = string(&state.props, "message").context("Screen state requires a message")?;
                    let mut screen = Node::screen(
                        vec![Node::text(message.to_owned(), Some(18.0), TextAlign::Centre, None, false)],
                        props.title,
                        true,
                    );
                    if let Some(label) = string(&state.props, "retryLabel") {
                        ensure!(state.props.get("onRetry") == Some(&Json::Bool(true)), "Error state requires onRetry");
                        if let NodeKind::Screen { footer, .. } = &mut screen.kind {
                            *footer = Some((label.to_owned(), (state.props.get("disabled") != Some(&Json::Bool(true)))
                                .then(|| event(state_id, "onRetry", vec![]))));
                        }
                    }
                    screen
                } else {
                    Node::screen(self.children(host, depth)?, props.title, props.centered)
                };
                if let NodeKind::Screen { pinned_header, pinned_footer, wide, bottom_inset, wait_for_images, left_action: left, right_action: action, media_picker, .. } = &mut screen.kind {
                    *wide = props.wide;
                    *bottom_inset = props.bottom_inset.unwrap_or(true);
                    *wait_for_images = props.wait_for_images.unwrap_or(true);
                    *pinned_header = state.is_none() && props.pinned_header;
                    *pinned_footer = state.is_none() && props.pinned_footer;
                    *left = left_action;
                    *action = right_action;
                    *media_picker = host.kind == HostKind::MediaPickerScreen;
                }
                screen
            }
            HostKind::ScreenState => bail!("LoadingState, EmptyState and ErrorState must be direct children of Screen"),
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
                let numeric = match string(props, "inputMode").unwrap_or("text") {
                    "text" => false,
                    "numeric" => true,
                    value => bail!("unsupported input mode {value}"),
                };
                ensure!(!numeric || action != TextInputAction::Return, "numeric inputs require a search or done action");
                Node::text_input(
                    string(props, "placeholder").unwrap_or(""),
                    self.inputs
                        .get(&id)
                        .context("TextInput has no native state")?
                        .state,
                    action,
                    props.get("autoFocus") == Some(&Json::Bool(true)),
                    numeric,
                    string(props, "prefix").unwrap_or("").to_owned(),
                    string(props, "suffix").unwrap_or("").to_owned(),
                    self.icon("close")?,
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
            HostKind::VideoView => {
                let controller = match props.get("controller") {
                    None | Some(Json::Null) => None,
                    Some(value) => {
                        let id = value.as_i64().context("Invalid video controller")?;
                        ensure!((1..=9_007_199_254_740_991).contains(&id), "Invalid video controller");
                        Some(ControllerId::new((-id) as usize))
                    }
                };
                Node { identity: NodeIdentity(0), kind: NodeKind::VideoView {
                    controller,
                    loading: props.get("loading").and_then(Json::as_bool).unwrap_or(false),
                } }
            }
            HostKind::MapView => {
                let id = props.get("controller").and_then(Json::as_i64)
                    .context("Map requires a controller")?;
                ensure!((1..=9_007_199_254_740_991).contains(&id), "Invalid map controller");
                Node::map_view(ControllerId::new((-id) as usize))
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
                let source = image_source(src)?;
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
                let mut node = Node::image(
                    source,
                    None,
                    props.get("bleed") == Some(&Json::Bool(true)),
                    props.get("zoomable") == Some(&Json::Bool(true)),
                    width,
                    height,
                    fit,
                );
                if let NodeKind::Image { preload, retain_while_loading, .. } = &mut node.kind {
                    *retain_while_loading = props.get("retainWhileLoading") == Some(&Json::Bool(true));
                    if let Some(sources) = props.get("preload").and_then(Json::as_array) {
                        *preload = sources.iter().take(2).map(|source| {
                            image_source(source.as_str().context("Preloaded images require a source")?)
                        }).collect::<Result<_>>()?;
                    }
                }
                node
            }
            HostKind::Icon => {
                let tone = match string(props, "tone").unwrap_or("primary") {
                    "primary" => Tone::Primary,
                    "muted" => Tone::Muted,
                    value => bail!("unsupported icon tone {value}"),
                };
                let name = string(props, "name").context("Icon requires a name")?;
                let mask = self.icon(name)?;
                let (name, filled) = icon_reference(name);
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
                        subtitle: string(props, "subtitle").map(str::to_owned),
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
                            .map(|name| self.icon(name))
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
    #[serde(rename = "bottomInset")]
    bottom_inset: Option<bool>,
    #[serde(rename = "waitForImages")]
    wait_for_images: Option<bool>,
    title: Option<String>,
    #[serde(default)]
    wide: bool,
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

fn canvas_colour(props: &HostProps, key: &str) -> Result<Option<super::Colour>> {
    let Some(value) = props.get(key) else { return Ok(None); };
    let hex = value.as_str().and_then(|value| value.strip_prefix('#'))
        .filter(|value| value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .with_context(|| format!("{key} must be a #RRGGBB colour"))?;
    let rgb = u32::from_str_radix(hex, 16)?;
    Ok(Some(super::Colour::rgb(
        ((rgb >> 16) & 255) as f32 / 255.0,
        ((rgb >> 8) & 255) as f32 / 255.0,
        (rgb & 255) as f32 / 255.0,
    )))
}

pub(super) fn event(id: usize, name: &'static str, args: Vec<Json>) -> Action {
    Action::Native {
        operation: event_operation(id, name, args),
    }
}

fn event_operation(id: usize, name: &'static str, args: Vec<Json>) -> NativeOperation {
    let mut operation = NativeOperation::new(
        "ink",
        "event",
        json!({ "type": "event", "id": id, "name": name, "args": args }).to_string(),
        0,
    );
    operation.event_target = Some((id, name));
    operation
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
        | NodeKind::Pressable { children, .. }
        | NodeKind::Screen { children, .. }
        | NodeKind::MediaGridRow { children }
        | NodeKind::RowTitle { children, .. }
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
        | NodeKind::Pressable { children, .. }
        | NodeKind::Screen { children, .. }
        | NodeKind::MediaGridRow { children }
        | NodeKind::RowTitle { children, .. }
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

fn icon_reference(reference: &str) -> (&str, bool) {
    if let Some(name) = reference.strip_prefix("filled:") {
        (name, true)
    } else {
        (reference.strip_prefix("outlined:").unwrap_or(reference), false)
    }
}

fn image_source(src: &str) -> Result<ImageSource> {
    Ok(if let Some(path) = src.strip_prefix("asset://") {
        ImageSource::Native("assets".into(), path.to_owned())
    } else if src.starts_with("https://") || src.starts_with("http://") {
        ImageSource::Native("network".into(), src.to_owned())
    } else if src.starts_with("ink-file://") || src.starts_with("ink-media://") {
        ImageSource::Native("files".into(), src.to_owned())
    } else if src.starts_with("ink-camera://") || src.starts_with("ink-camera-review://") {
        ImageSource::Native("camera".into(), src.to_owned())
    } else {
        bail!("Image sources must be bundled assets, managed files or HTTPS URLs");
    })
}
