#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateId(pub usize);

#[derive(Debug)]
pub struct App {
    pub states: Vec<State>,
    pub root: Node,
}

#[derive(Debug)]
pub struct State {
    pub initial: i64,
}

#[derive(Debug)]
pub enum Node {
    Column {
        children: Vec<Node>,
        gap: Option<f32>,
    },
    Text {
        parts: Vec<TextPart>,
        font_size: Option<f32>,
    },
    Button {
        label: String,
        action: Action,
    },
}

#[derive(Debug)]
pub enum TextPart {
    Literal(String),
    State(StateId),
}

#[derive(Debug)]
pub enum Action {
    Increment { state: StateId, by: i64 },
}
