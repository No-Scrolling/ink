use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use oxc::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateId(pub usize);

#[derive(Clone, Debug)]
pub struct SourceSpan {
    pub path: PathBuf,
    pub span: Span,
}

#[derive(Debug)]
pub struct App {
    pub extensions: BTreeSet<Extension>,
    pub states: Vec<State>,
    pub root: Node,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Extension {
    LightSdk,
}

#[derive(Debug)]
pub struct State {
    pub initial: StateValue,
    pub shape: StateShape,
    pub lifetime: StateLifetime,
    pub source: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateLifetime {
    Local,
    Shared(String),
    Persisted(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateShape {
    Int,
    Bool,
    String,
    List(Box<StateShape>),
    Object(BTreeMap<String, StateShape>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateValue {
    Int(i64),
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

#[derive(Debug)]
pub enum Node {
    Screen {
        children: Vec<Node>,
        title: Option<String>,
        centered: bool,
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
    SelectorButton {
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
        source: String,
        width: f32,
        height: f32,
        fit: ImageFit,
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
        state: StateId,
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
    ListLength(StateId),
    Item(Vec<String>),
}

#[derive(Clone, Debug)]
pub enum Condition {
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
}

#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
    Bool(bool),
    String(String),
    State(StateId),
    Item(Vec<String>),
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
}

#[derive(Clone, Debug)]
pub enum Action {
    Increment { state: StateId, by: i64 },
    SetInt { state: StateId, value: i64 },
    SetBool { state: StateId, value: bool },
    SetString { state: StateId, value: String },
    Toggle { state: StateId },
    SetList { state: StateId, value: Value },
    AppendList { state: StateId, value: Value },
    RemoveListItem { state: StateId },
    ReplaceListItem { state: StateId, value: Value },
    ClearList { state: StateId },
    Navigate { path: String, source: SourceSpan },
    Back,
    Sequence(Vec<Action>),
}
