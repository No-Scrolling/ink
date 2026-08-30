use oxc::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateId(pub usize);

#[derive(Debug)]
pub struct App {
    pub states: Vec<State>,
    pub root: Node,
}

#[derive(Debug)]
pub struct State {
    pub initial: StateValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateValue {
    Int(i64),
    Bool(bool),
    String(String),
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
        label: String,
        icon: Option<String>,
        underline: bool,
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

#[derive(Debug)]
pub enum TextPart {
    Literal(String),
    State(StateId),
}

#[derive(Debug)]
pub enum Action {
    Increment { state: StateId, by: i64 },
    SetInt { state: StateId, value: i64 },
    SetBool { state: StateId, value: bool },
    Toggle { state: StateId },
    Navigate { path: String, span: Span },
}
