use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use oxc::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControllerId(pub usize);

#[derive(Clone, Debug)]
pub struct SourceSpan {
    pub path: PathBuf,
    pub span: Span,
}

#[derive(Debug)]
pub struct App {
    pub extensions: BTreeSet<Extension>,
    pub android_permissions: BTreeSet<AndroidPermission>,
    pub states: Vec<State>,
    pub resources: Vec<Resource>,
    pub application_resources: Vec<ResourceId>,
    pub controllers: Vec<Controller>,
    pub application_controllers: Vec<ControllerId>,
    pub root: Node,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Extension {
    LightSdk,
    Network,
    Audio,
    Location,
    Nfc,
    Background,
    Notifications,
    Camera,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AndroidPermission {
    Camera,
    Microphone,
    Location,
    Nfc,
    Notifications,
}

#[derive(Debug)]
pub struct State {
    pub initial: StateValue,
    pub shape: StateShape,
    pub lifetime: StateLifetime,
    pub source: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct Resource {
    pub module: String,
    pub operation: String,
    pub payload: Vec<PayloadPart>,
    pub shape: StateShape,
    pub timeout_ms: u64,
    pub reload_on_resume: bool,
    pub protocol: ResourceProtocol,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceProtocol {
    Async,
    Cached,
    Mutation,
    Background,
}

#[derive(Clone, Debug)]
pub struct Controller {
    pub state: StateId,
    pub module: String,
    pub kind: String,
    pub config: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeOperation {
    pub module: String,
    pub operation: String,
    pub payload: Vec<PayloadPart>,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PayloadPart {
    Literal(String),
    State(StateId),
    Item(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceField {
    Status,
    Value(Vec<String>),
    ErrorKind,
    ErrorMessage,
    ErrorRetryable,
    UpdatedAtMs,
    ErrorAttemptedAtMs,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateLifetime {
    Local,
    Shared(String),
    Persisted(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StateShape {
    Null,
    Number,
    Bool,
    String,
    Literal(StateLiteral),
    Optional(Box<StateShape>),
    Union(Vec<StateShape>),
    List(Box<StateShape>),
    Object(BTreeMap<String, StateShape>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StateLiteral {
    Number(f64),
    Bool(bool),
    String(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StateValue {
    Null,
    Number(f64),
    Bool(bool),
    String(String),
    List(Vec<StateValue>),
    Object(Vec<(String, StateValue)>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justification {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlignment {
    #[default]
    Start,
    Center,
    End,
    Justify,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextInputAction {
    Return,
    #[default]
    Search,
    Done,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Primary,
    Muted,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageFit {
    #[default]
    Cover,
    Contain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraPreviewKind {
    Photo,
    Scanner,
}

#[derive(Debug)]
pub enum Node {
    Screen {
        children: Vec<Node>,
        title: Option<String>,
        centered: bool,
        params: BTreeMap<String, StateShape>,
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
        align: TextAlignment,
        max_lines: Option<u32>,
    },
    TextInput {
        placeholder: String,
        state: StateId,
        action: TextInputAction,
    },
    Button {
        label: Vec<TextPart>,
        icon: Option<String>,
        underline: bool,
        action: Option<Action>,
    },
    Field {
        label: String,
        value: Vec<TextPart>,
        action: Option<Action>,
    },
    Icon {
        name: String,
        size: Option<f32>,
        tone: Tone,
    },
    Image {
        source: ImageSource,
        fallback: Option<String>,
        bleed: bool,
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
    },
    Tabs {
        state: StateId,
        tabs: Vec<Tab>,
    },
    Navigator {
        routes: Vec<Route>,
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
    ScreenModule {
        path: PathBuf,
    },
}

#[derive(Debug)]
pub struct Tab {
    pub icon: String,
    pub action: Action,
    pub screen: Box<Node>,
}

#[derive(Debug)]
pub struct Route {
    pub path: String,
    pub screen: Box<Node>,
}

#[derive(Clone, Debug)]
pub enum TextPart {
    Literal(String),
    State(StateId),
    Resource(ResourceId, ResourceField),
    Controller(ControllerId, Vec<String>),
    ListLength(StateId),
    Item(Vec<String>),
    Value(Value),
}

#[derive(Clone, Debug)]
pub enum ImageSource {
    Local(String),
    Remote(Vec<TextPart>),
    Camera(Vec<TextPart>),
}

#[derive(Clone, Debug)]
pub enum Collection {
    State(StateId),
    Resource(ResourceId, Vec<String>),
    Controller(ControllerId, Vec<String>),
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug)]
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
    RemoveListItem {
        state: StateId,
    },
    ReplaceListItem {
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
        params: Vec<RouteArgument>,
        source: SourceSpan,
    },
    Back,
    Sequence(Vec<Action>),
}

#[derive(Clone, Debug)]
pub struct RouteArgument {
    pub name: String,
    pub value: Value,
    pub shape: StateShape,
}
