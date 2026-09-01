use std::fmt;

use serde::{Deserialize, Serialize};

const MAGIC: [u8; 4] = *b"INKA";
const HEADER_SIZE: usize = 6;
const MAX_DEFINITION_SIZE: usize = 64 * 1024 * 1024;
const MAX_TREE_DEPTH: usize = 256;
pub const FORMAT_VERSION: u16 = 3;
pub const ASSET_NAME: &str = "app.ink";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Application {
    pub states: Vec<StateDefinition>,
    pub state_dependencies: Vec<StateDependency>,
    pub resources: Vec<ResourceDefinition>,
    pub application_resources: Vec<ResourceId>,
    pub controllers: Vec<ControllerDefinition>,
    pub application_controllers: Vec<ControllerId>,
    pub masks: Vec<MaskAsset>,
    pub images: Vec<ImageAsset>,
    pub root: Node,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDependency {
    pub state: StateId,
    pub node: u32,
    pub kind: DependencyKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyKind {
    Layout,
    Structure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaskId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageId(pub u32);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateDefinition {
    pub initial: StateValue,
    pub shape: StateShape,
    pub persisted: Option<PersistedState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedState {
    pub key: String,
    pub schema: u64,
    pub shape: StateShape,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub shape: StateShape,
    pub read: NativeOperation,
    pub reload_on_resume: bool,
    pub protocol: ResourceProtocol,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceProtocol {
    Async,
    Cached,
    Mutation,
    Background,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerDefinition {
    pub state: StateId,
    pub module: String,
    pub kind: String,
    pub config: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NativeOperation {
    pub module: String,
    pub operation: String,
    pub payload: Vec<PayloadPart>,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PayloadPart {
    Literal(String),
    State(StateId),
    Item(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StateShape {
    Null,
    Number,
    Bool,
    String,
    Literal(StateLiteral),
    Optional(Box<StateShape>),
    Union(Vec<StateShape>),
    List(Box<StateShape>),
    Object(Vec<(String, StateShape)>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StateLiteral {
    Number(f64),
    Bool(bool),
    String(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StateValue {
    Null,
    Number(f64),
    Bool(bool),
    String(String),
    List(Vec<StateValue>),
    Object(Vec<(String, StateValue)>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    Start,
    Centre,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Justification {
    Start,
    Centre,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Start,
    Centre,
    End,
    Justify,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextInputAction {
    Return,
    Search,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tone {
    Primary,
    Muted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFit {
    Cover,
    Contain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraPreviewKind {
    Photo,
    Scanner,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Node {
    Screen {
        children: Vec<Node>,
        title: Option<String>,
        centred: bool,
        resources: Vec<ResourceId>,
        controllers: Vec<ControllerId>,
    },
    Stack {
        children: Vec<Node>,
        axis: Axis,
        gap: Option<f32>,
        align: Alignment,
        justify: Justification,
    },
    Text {
        parts: Vec<TextPart>,
        font_size: Option<f32>,
        align: TextAlign,
        max_lines: Option<u32>,
    },
    TextInput {
        placeholder: String,
        state: StateId,
        action: TextInputAction,
        auto_focus: bool,
    },
    Button {
        label: Vec<TextPart>,
        icon: Option<MaskId>,
        underline: bool,
        action: Option<Action>,
    },
    Field {
        label: String,
        value: Vec<TextPart>,
        action: Option<Action>,
    },
    Icon {
        mask: MaskId,
        size: f32,
        tone: Tone,
    },
    Image {
        source: ImageSource,
        fallback: Option<ImageId>,
        bleed: bool,
        zoomable: bool,
        width: f32,
        height: f32,
        fit: ImageFit,
    },
    CameraPreview {
        controller: ControllerId,
        kind: CameraPreviewKind,
    },
    Toggle {
        label: String,
        state: StateId,
        action: Action,
        off: MaskId,
        on: MaskId,
    },
    Tabs {
        state: StateId,
        tabs: Vec<Tab>,
    },
    Navigator {
        routes: Vec<Route>,
        back: MaskId,
    },
    Conditional {
        condition: Condition,
        consequent: Box<Node>,
        alternate: Option<Box<Node>>,
    },
    ForEach {
        collection: Collection,
        template: Box<Node>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    pub icon: MaskId,
    pub action: Action,
    pub screen: Node,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub screen: Node,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TextPart {
    Literal(String),
    State(StateId),
    Resource(ResourceId, ResourceField),
    Controller(ControllerId, Vec<String>),
    ListLength(StateId),
    Item(Vec<String>),
    Value(Value),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ImageSource {
    Asset(ImageId),
    Remote(Vec<TextPart>),
    Native(String, Vec<TextPart>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Collection {
    State(StateId),
    Resource(ResourceId, Vec<String>),
    Controller(ControllerId, Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResourceField {
    Status,
    Value(Vec<String>),
    ErrorKind,
    ErrorMessage,
    ErrorRetryable,
    UpdatedAtMs,
    ErrorAttemptedAtMs,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    ValueEquals {
        value: Value,
        expected: StateValue,
        equals: bool,
    },
    Bool {
        state: StateId,
        expected: bool,
    },
    ListEmpty {
        state: StateId,
        expected: bool,
    },
    Equals {
        state: StateId,
        value: StateValue,
        expected: bool,
    },
    ResourceEquals {
        resource: ResourceId,
        field: ResourceField,
        value: StateValue,
        expected: bool,
    },
    ControllerEquals {
        controller: ControllerId,
        path: Vec<String>,
        value: StateValue,
        expected: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Number(f64),
    Bool(bool),
    String(String),
    State(StateId),
    Item(Vec<String>),
    Resource(ResourceId, ResourceField),
    Controller(ControllerId, Vec<String>),
    CombinedStatus(Vec<ResourceId>),
    CombinedErrorResource(Vec<(String, ResourceId)>),
    CombinedErrorField(Vec<ResourceId>, ResourceField),
    ListLength(StateId),
    Binary {
        left: Box<Value>,
        operator: ValueOperator,
        right: Box<Value>,
    },
    RouteParam(String),
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Increment {
        state: StateId,
        by: f64,
    },
    SetValue {
        state: StateId,
        value: Value,
    },
    Toggle {
        state: StateId,
    },
    SetList {
        state: StateId,
        value: Value,
    },
    AppendList {
        state: StateId,
        value: Value,
    },
    RemoveCurrentListItem {
        state: StateId,
    },
    ReplaceCurrentListItem {
        state: StateId,
        value: Value,
    },
    ClearList {
        state: StateId,
    },
    ReloadResource {
        resource: ResourceId,
    },
    Controller {
        controller: ControllerId,
        operation: String,
        payload: Vec<PayloadPart>,
    },
    Native {
        operation: NativeOperation,
    },
    Navigate {
        path: String,
        params: Vec<(String, Value)>,
    },
    Back,
    Sequence(Vec<Action>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaskAsset {
    pub id: u64,
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageAsset {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub encoding: ImageAssetEncoding,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageAssetEncoding {
    RgbaZlib,
    Jpeg,
}

#[derive(Debug)]
pub enum FormatError {
    TooLarge(usize),
    InvalidMagic,
    UnsupportedVersion(u16),
    Codec(postcard::Error),
    Invalid(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge(size) => {
                write!(formatter, "Ink application is too large ({size} bytes)")
            }
            Self::InvalidMagic => formatter.write_str("not an Ink application definition"),
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "Ink application format {version} is unsupported; expected {FORMAT_VERSION}",
            ),
            Self::Codec(error) => write!(formatter, "invalid Ink application encoding: {error}"),
            Self::Invalid(message) => write!(formatter, "invalid Ink application: {message}"),
        }
    }
}

impl std::error::Error for FormatError {}

pub fn encode(application: &Application) -> Result<Vec<u8>, FormatError> {
    validate(application)?;
    let payload = postcard::to_allocvec(application).map_err(FormatError::Codec)?;
    let size = HEADER_SIZE + payload.len();
    if size > MAX_DEFINITION_SIZE {
        return Err(FormatError::TooLarge(size));
    }
    let mut bytes = Vec::with_capacity(size);
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> Result<Application, FormatError> {
    if bytes.len() > MAX_DEFINITION_SIZE {
        return Err(FormatError::TooLarge(bytes.len()));
    }
    if bytes.len() < HEADER_SIZE || bytes[..4] != MAGIC {
        return Err(FormatError::InvalidMagic);
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != FORMAT_VERSION {
        return Err(FormatError::UnsupportedVersion(version));
    }
    let application = postcard::from_bytes(&bytes[HEADER_SIZE..]).map_err(FormatError::Codec)?;
    validate(&application)?;
    Ok(application)
}

pub fn validate(application: &Application) -> Result<(), FormatError> {
    for (index, state) in application.states.iter().enumerate() {
        if !state.shape.accepts(&state.initial, 0) {
            return invalid(format!(
                "state {index} initial value does not match its shape"
            ));
        }
        if let Some(persisted) = &state.persisted {
            if persisted.key.is_empty() {
                return invalid(format!("state {index} has an empty persistence key"));
            }
            if persisted.shape != state.shape {
                return invalid(format!("state {index} persistence shape does not match"));
            }
        }
    }
    for id in &application.application_resources {
        resource(application, *id)?;
    }
    for id in &application.application_controllers {
        controller(application, *id)?;
    }
    for (index, resource_definition) in application.resources.iter().enumerate() {
        validate_operation(application, &resource_definition.read)
            .map_err(|error| context(error, format!("resource {index}")))?;
    }
    for (index, controller_definition) in application.controllers.iter().enumerate() {
        state(application, controller_definition.state)
            .map_err(|error| context(error, format!("controller {index}")))?;
    }
    for (index, mask) in application.masks.iter().enumerate() {
        let expected = usize::from(mask.width) * usize::from(mask.height);
        if expected == 0 || mask.pixels.len() != expected {
            return invalid(format!("mask {index} has invalid dimensions"));
        }
    }
    for (index, image) in application.images.iter().enumerate() {
        if image.width == 0 || image.height == 0 || image.bytes.is_empty() {
            return invalid(format!("image {index} is empty"));
        }
    }
    let node_count = count_nodes(&application.root, 0)?;
    let mut previous = None;
    for dependency in &application.state_dependencies {
        state(application, dependency.state)?;
        if dependency.node == 0 || dependency.node as usize > node_count {
            return invalid(format!("node {} does not exist", dependency.node));
        }
        let key = (dependency.state.0, dependency.node);
        if previous.is_some_and(|previous| previous >= key) {
            return invalid("state dependencies must be sorted and unique");
        }
        previous = Some(key);
    }
    validate_node(application, &application.root, 0)
}

fn count_nodes(node: &Node, depth: usize) -> Result<usize, FormatError> {
    if depth > MAX_TREE_DEPTH {
        return invalid("node tree is too deep");
    }
    let descendants = match node {
        Node::Screen { children, .. } | Node::Stack { children, .. } => children
            .iter()
            .map(|child| count_nodes(child, depth + 1))
            .sum::<Result<usize, _>>()?,
        Node::Tabs { tabs, .. } => tabs
            .iter()
            .map(|tab| count_nodes(&tab.screen, depth + 1))
            .sum::<Result<usize, _>>()?,
        Node::Navigator { routes, .. } => routes
            .iter()
            .map(|route| count_nodes(&route.screen, depth + 1))
            .sum::<Result<usize, _>>()?,
        Node::Conditional {
            consequent,
            alternate,
            ..
        } => {
            count_nodes(consequent, depth + 1)?
                + alternate
                    .as_deref()
                    .map(|node| count_nodes(node, depth + 1))
                    .transpose()?
                    .unwrap_or_default()
        }
        Node::ForEach { template, .. } => count_nodes(template, depth + 1)?,
        Node::Text { .. }
        | Node::TextInput { .. }
        | Node::Button { .. }
        | Node::Field { .. }
        | Node::Icon { .. }
        | Node::Image { .. }
        | Node::CameraPreview { .. }
        | Node::Toggle { .. } => 0,
    };
    Ok(1 + descendants)
}

impl StateShape {
    fn accepts(&self, value: &StateValue, depth: usize) -> bool {
        if depth > MAX_TREE_DEPTH {
            return false;
        }
        match (self, value) {
            (Self::Null, StateValue::Null)
            | (Self::Number, StateValue::Number(_))
            | (Self::Bool, StateValue::Bool(_))
            | (Self::String, StateValue::String(_)) => true,
            (Self::Literal(StateLiteral::Number(expected)), StateValue::Number(value)) => {
                expected == value
            }
            (Self::Literal(StateLiteral::Bool(expected)), StateValue::Bool(value)) => {
                expected == value
            }
            (Self::Literal(StateLiteral::String(expected)), StateValue::String(value)) => {
                expected == value
            }
            (Self::Optional(_), StateValue::Null) => true,
            (Self::Optional(shape), value) => shape.accepts(value, depth + 1),
            (Self::Union(shapes), value) => {
                shapes.iter().any(|shape| shape.accepts(value, depth + 1))
            }
            (Self::List(shape), StateValue::List(values)) => {
                values.iter().all(|value| shape.accepts(value, depth + 1))
            }
            (Self::Object(shapes), StateValue::Object(values)) => {
                shapes.len() == values.len()
                    && shapes.iter().all(|(name, shape)| {
                        values
                            .iter()
                            .find(|(field, _)| field == name)
                            .is_some_and(|(_, value)| shape.accepts(value, depth + 1))
                    })
            }
            _ => false,
        }
    }
}

fn validate_node(application: &Application, node: &Node, depth: usize) -> Result<(), FormatError> {
    if depth > MAX_TREE_DEPTH {
        return invalid("node tree is too deep");
    }
    match node {
        Node::Screen {
            children,
            resources,
            controllers,
            ..
        } => {
            for id in resources {
                resource(application, *id)?;
            }
            for id in controllers {
                controller(application, *id)?;
            }
            validate_children(application, children, depth)
        }
        Node::Stack { children, gap, .. } => {
            if gap.is_some_and(|gap| !gap.is_finite() || gap < 0.0) {
                return invalid("stack gap is invalid");
            }
            validate_children(application, children, depth)
        }
        Node::Text {
            parts,
            font_size,
            max_lines,
            ..
        } => {
            if font_size.is_some_and(|size| !size.is_finite() || size <= 0.0) {
                return invalid("text size is invalid");
            }
            if max_lines == &Some(0) {
                return invalid("text max lines is invalid");
            }
            validate_text_parts(application, parts, depth)
        }
        Node::TextInput { state: id, .. } => state(application, *id),
        Node::Button {
            label,
            icon,
            action,
            ..
        } => {
            validate_text_parts(application, label, depth)?;
            if let Some(id) = icon {
                mask(application, *id)?;
            }
            if let Some(action) = action {
                validate_action(application, action, depth)?;
            }
            Ok(())
        }
        Node::Field { value, action, .. } => {
            validate_text_parts(application, value, depth)?;
            if let Some(action) = action {
                validate_action(application, action, depth)?;
            }
            Ok(())
        }
        Node::Icon { mask: id, size, .. } => {
            mask(application, *id)?;
            if !size.is_finite() || *size <= 0.0 {
                return invalid("icon size is invalid");
            }
            Ok(())
        }
        Node::Image {
            source,
            fallback,
            width,
            height,
            ..
        } => {
            validate_image_source(application, source, depth)?;
            if let Some(id) = fallback {
                image(application, *id)?;
            }
            if !width.is_finite() || !height.is_finite() || *width <= 0.0 || *height <= 0.0 {
                return invalid("image dimensions are invalid");
            }
            Ok(())
        }
        Node::CameraPreview { controller: id, .. } => controller(application, *id),
        Node::Toggle {
            state: id,
            action,
            off,
            on,
            ..
        } => {
            state(application, *id)?;
            validate_action(application, action, depth)?;
            mask(application, *off)?;
            mask(application, *on)
        }
        Node::Tabs { state: id, tabs } => {
            state(application, *id)?;
            for tab in tabs {
                mask(application, tab.icon)?;
                validate_action(application, &tab.action, depth)?;
                validate_node(application, &tab.screen, depth + 1)?;
            }
            Ok(())
        }
        Node::Navigator { routes, back } => {
            mask(application, *back)?;
            if routes.iter().filter(|route| route.path == "/").count() != 1 {
                return invalid("navigator must have exactly one root route");
            }
            for (index, route) in routes.iter().enumerate() {
                if route.path.is_empty()
                    || routes[..index]
                        .iter()
                        .any(|previous| previous.path == route.path)
                {
                    return invalid("navigator routes must be non-empty and unique");
                }
                validate_node(application, &route.screen, depth + 1)?;
            }
            Ok(())
        }
        Node::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            validate_condition(application, condition, depth)?;
            validate_node(application, consequent, depth + 1)?;
            if let Some(alternate) = alternate {
                validate_node(application, alternate, depth + 1)?;
            }
            Ok(())
        }
        Node::ForEach {
            collection,
            template,
        } => {
            validate_collection(application, collection)?;
            validate_node(application, template, depth + 1)
        }
    }
}

fn validate_children(
    application: &Application,
    children: &[Node],
    depth: usize,
) -> Result<(), FormatError> {
    for child in children {
        validate_node(application, child, depth + 1)?;
    }
    Ok(())
}

fn validate_text_parts(
    application: &Application,
    parts: &[TextPart],
    depth: usize,
) -> Result<(), FormatError> {
    for part in parts {
        match part {
            TextPart::Literal(_) | TextPart::Item(_) => {}
            TextPart::State(id) | TextPart::ListLength(id) => state(application, *id)?,
            TextPart::Resource(id, _) => resource(application, *id)?,
            TextPart::Controller(id, _) => controller(application, *id)?,
            TextPart::Value(value) => validate_value(application, value, depth + 1)?,
        }
    }
    Ok(())
}

fn validate_image_source(
    application: &Application,
    source: &ImageSource,
    depth: usize,
) -> Result<(), FormatError> {
    match source {
        ImageSource::Asset(id) => image(application, *id),
        ImageSource::Remote(parts) | ImageSource::Native(_, parts) => {
            validate_text_parts(application, parts, depth)
        }
    }
}

fn validate_condition(
    application: &Application,
    condition: &Condition,
    depth: usize,
) -> Result<(), FormatError> {
    match condition {
        Condition::ValueEquals { value, .. } => validate_value(application, value, depth + 1),
        Condition::Bool { state: id, .. }
        | Condition::ListEmpty { state: id, .. }
        | Condition::Equals { state: id, .. } => state(application, *id),
        Condition::ResourceEquals { resource: id, .. } => resource(application, *id),
        Condition::ControllerEquals { controller: id, .. } => controller(application, *id),
    }
}

fn validate_collection(
    application: &Application,
    collection: &Collection,
) -> Result<(), FormatError> {
    match collection {
        Collection::State(id) => state(application, *id),
        Collection::Resource(id, _) => resource(application, *id),
        Collection::Controller(id, _) => controller(application, *id),
    }
}

fn validate_action(
    application: &Application,
    action: &Action,
    depth: usize,
) -> Result<(), FormatError> {
    if depth > MAX_TREE_DEPTH {
        return invalid("action tree is too deep");
    }
    match action {
        Action::Increment { state: id, by } => {
            state(application, *id)?;
            if !by.is_finite() {
                return invalid("increment is not finite");
            }
            Ok(())
        }
        Action::SetValue { state: id, value }
        | Action::SetList { state: id, value }
        | Action::AppendList { state: id, value }
        | Action::ReplaceCurrentListItem { state: id, value } => {
            state(application, *id)?;
            validate_value(application, value, depth + 1)
        }
        Action::Toggle { state: id }
        | Action::RemoveCurrentListItem { state: id }
        | Action::ClearList { state: id } => state(application, *id),
        Action::ReloadResource { resource: id } => resource(application, *id),
        Action::Controller {
            controller: id,
            payload,
            ..
        } => {
            controller(application, *id)?;
            validate_payload(application, payload)
        }
        Action::Native { operation } => validate_operation(application, operation),
        Action::Navigate { params, .. } => {
            for (_, value) in params {
                validate_value(application, value, depth + 1)?;
            }
            Ok(())
        }
        Action::Back => Ok(()),
        Action::Sequence(actions) => {
            for action in actions {
                validate_action(application, action, depth + 1)?;
            }
            Ok(())
        }
    }
}

fn validate_value(
    application: &Application,
    value: &Value,
    depth: usize,
) -> Result<(), FormatError> {
    if depth > MAX_TREE_DEPTH {
        return invalid("value tree is too deep");
    }
    match value {
        Value::Number(value) if !value.is_finite() => invalid("number is not finite"),
        Value::Null
        | Value::Number(_)
        | Value::Bool(_)
        | Value::String(_)
        | Value::Item(_)
        | Value::RouteParam(_) => Ok(()),
        Value::State(id) | Value::ListLength(id) => state(application, *id),
        Value::Resource(id, _) => resource(application, *id),
        Value::Controller(id, _) => controller(application, *id),
        Value::CombinedStatus(ids) => {
            for id in ids {
                resource(application, *id)?;
            }
            Ok(())
        }
        Value::CombinedErrorResource(resources) => {
            for (_, id) in resources {
                resource(application, *id)?;
            }
            Ok(())
        }
        Value::CombinedErrorField(ids, _) => {
            for id in ids {
                resource(application, *id)?;
            }
            Ok(())
        }
        Value::Binary { left, right, .. } => {
            validate_value(application, left, depth + 1)?;
            validate_value(application, right, depth + 1)
        }
        Value::List(values) => {
            for value in values {
                validate_value(application, value, depth + 1)?;
            }
            Ok(())
        }
        Value::Object(fields) => {
            for (_, value) in fields {
                validate_value(application, value, depth + 1)?;
            }
            Ok(())
        }
    }
}

fn validate_operation(
    application: &Application,
    operation: &NativeOperation,
) -> Result<(), FormatError> {
    if operation.module.is_empty() || operation.operation.is_empty() {
        return invalid("native operation has an empty name");
    }
    validate_payload(application, &operation.payload)
}

fn validate_payload(application: &Application, payload: &[PayloadPart]) -> Result<(), FormatError> {
    for part in payload {
        if let PayloadPart::State(id) = part {
            state(application, *id)?;
        }
    }
    Ok(())
}

fn state(application: &Application, id: StateId) -> Result<(), FormatError> {
    index("state", id.0, application.states.len())
}

fn resource(application: &Application, id: ResourceId) -> Result<(), FormatError> {
    index("resource", id.0, application.resources.len())
}

fn controller(application: &Application, id: ControllerId) -> Result<(), FormatError> {
    index("controller", id.0, application.controllers.len())
}

fn mask(application: &Application, id: MaskId) -> Result<(), FormatError> {
    index("mask", id.0, application.masks.len())
}

fn image(application: &Application, id: ImageId) -> Result<(), FormatError> {
    index("image", id.0, application.images.len())
}

fn index(kind: &str, value: u32, length: usize) -> Result<(), FormatError> {
    if usize::try_from(value).is_ok_and(|value| value < length) {
        Ok(())
    } else {
        invalid(format!("{kind} {value} does not exist"))
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, FormatError> {
    Err(FormatError::Invalid(message.into()))
}

fn context(error: FormatError, prefix: String) -> FormatError {
    match error {
        FormatError::Invalid(message) => FormatError::Invalid(format!("{prefix}: {message}")),
        error => error,
    }
}
