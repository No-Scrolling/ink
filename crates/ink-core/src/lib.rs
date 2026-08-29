use std::fmt;

const DEFAULT_TEXT_SIZE: f32 = 30.0;
const DEFAULT_BUTTON_WIDTH: f32 = 360.0;
const DEFAULT_BUTTON_HEIGHT: f32 = 76.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateId(usize);

impl StateId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateValue {
    Int(i64),
}

impl fmt::Display for StateValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(value) => value.fmt(formatter),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Increment { state: StateId, by: i64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextPart {
    Literal(String),
    State(StateId),
}

impl TextPart {
    pub fn literal(value: impl Into<String>) -> Self {
        Self::Literal(value.into())
    }

    pub const fn state(state: StateId) -> Self {
        Self::State(state)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub gap: Option<f32>,
    pub font_size: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    kind: NodeKind,
    style: Style,
}

#[derive(Clone, Debug, PartialEq)]
enum NodeKind {
    Column(Vec<Node>),
    Text(Vec<TextPart>),
    Button { label: String, action: Action },
}

impl Node {
    pub fn column(children: Vec<Self>) -> Self {
        Self {
            kind: NodeKind::Column(children),
            style: Style::default(),
        }
    }

    pub fn text(parts: Vec<TextPart>) -> Self {
        Self {
            kind: NodeKind::Text(parts),
            style: Style::default(),
        }
    }

    pub fn button(label: impl Into<String>, action: Action) -> Self {
        Self {
            kind: NodeKind::Button {
                label: label.into(),
                action,
            },
            style: Style::default(),
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppDefinition {
    initial_state: Vec<StateValue>,
    root: Node,
}

impl AppDefinition {
    pub fn new(initial_state: Vec<StateValue>, root: Node) -> Self {
        Self {
            initial_state,
            root,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colour {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl Colour {
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);

    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Quad {
    pub rect: Rect,
    pub colour: Colour,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Centre,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub rect: Rect,
    pub font_size: f32,
    pub colour: Colour,
    pub align: TextAlign,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub width: u32,
    pub height: u32,
    pub quads: Vec<Quad>,
    pub text: Vec<TextRun>,
}

#[derive(Clone, Debug)]
struct HitRegion {
    rect: Rect,
    action: Action,
}

#[derive(Debug)]
pub struct Engine {
    definition: AppDefinition,
    state: Vec<StateValue>,
    viewport: Viewport,
    scene: Scene,
    hit_regions: Vec<HitRegion>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Viewport {
    width: u32,
    height: u32,
    density: f32,
}

impl Engine {
    pub fn new(definition: AppDefinition) -> Self {
        let state = definition.initial_state.clone();
        Self {
            definition,
            state,
            viewport: Viewport::default(),
            scene: Scene::default(),
            hit_regions: Vec::new(),
        }
    }

    pub fn set_viewport(&mut self, width: u32, height: u32, density: f32) -> bool {
        let viewport = Viewport {
            width,
            height,
            density: if density.is_finite() && density > 0.0 {
                density
            } else {
                1.0
            },
        };
        if self.viewport == viewport {
            return false;
        }

        self.viewport = viewport;
        self.rebuild_scene();
        true
    }

    pub fn tap(&mut self, x: f32, y: f32) -> bool {
        let Some(action) = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| region.rect.contains(x, y))
            .map(|region| region.action.clone())
        else {
            return false;
        };

        if !self.apply(action) {
            return false;
        }

        self.rebuild_scene();
        true
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    fn apply(&mut self, action: Action) -> bool {
        match action {
            Action::Increment { state, by } => {
                let Some(StateValue::Int(value)) = self.state.get_mut(state.0) else {
                    return false;
                };
                *value += by;
                true
            }
        }
    }

    fn rebuild_scene(&mut self) {
        self.scene = Scene {
            width: self.viewport.width,
            height: self.viewport.height,
            quads: Vec::new(),
            text: Vec::new(),
        };
        self.hit_regions.clear();

        if self.viewport.width == 0 || self.viewport.height == 0 {
            return;
        }

        let root = self.definition.root.clone();
        let size = self.measure(&root);
        let origin_x = (self.viewport.width as f32 - size.width).max(0.0) / 2.0;
        let origin_y = (self.viewport.height as f32 - size.height).max(0.0) / 2.0;
        self.layout(&root, origin_x, origin_y, size.width);
    }

    fn measure(&self, node: &Node) -> MeasuredSize {
        match &node.kind {
            NodeKind::Column(children) => {
                let gap = self.scaled(node.style.gap.unwrap_or_default());
                let measured: Vec<_> = children.iter().map(|child| self.measure(child)).collect();
                let width = measured.iter().map(|size| size.width).fold(0.0, f32::max);
                let height = measured.iter().map(|size| size.height).sum::<f32>()
                    + gap * children.len().saturating_sub(1) as f32;
                MeasuredSize { width, height }
            }
            NodeKind::Text(_) => {
                let font_size = self.scaled(node.style.font_size.unwrap_or(DEFAULT_TEXT_SIZE));
                MeasuredSize {
                    width: self.viewport.width as f32,
                    height: font_size * 1.25,
                }
            }
            NodeKind::Button { .. } => MeasuredSize {
                width: self
                    .scaled(node.style.width.unwrap_or(DEFAULT_BUTTON_WIDTH))
                    .min((self.viewport.width as f32 - self.scaled(48.0)).max(0.0)),
                height: self.scaled(node.style.height.unwrap_or(DEFAULT_BUTTON_HEIGHT)),
            },
        }
    }

    fn layout(&mut self, node: &Node, x: f32, y: f32, available_width: f32) -> f32 {
        match &node.kind {
            NodeKind::Column(children) => {
                let gap = self.scaled(node.style.gap.unwrap_or_default());
                let mut cursor_y = y;
                for child in children {
                    let child_size = self.measure(child);
                    let child_x = x + (available_width - child_size.width).max(0.0) / 2.0;
                    cursor_y += self.layout(child, child_x, cursor_y, child_size.width) + gap;
                }
                (cursor_y - y - gap).max(0.0)
            }
            NodeKind::Text(parts) => {
                let font_size = self.scaled(node.style.font_size.unwrap_or(DEFAULT_TEXT_SIZE));
                let height = font_size * 1.25;
                self.scene.text.push(TextRun {
                    text: self.resolve_text(parts),
                    rect: Rect {
                        x,
                        y,
                        width: available_width,
                        height,
                    },
                    font_size,
                    colour: Colour::WHITE,
                    align: TextAlign::Centre,
                });
                height
            }
            NodeKind::Button { label, action } => {
                let width = self
                    .scaled(node.style.width.unwrap_or(DEFAULT_BUTTON_WIDTH))
                    .min((self.viewport.width as f32 - self.scaled(48.0)).max(0.0));
                let height = self.scaled(node.style.height.unwrap_or(DEFAULT_BUTTON_HEIGHT));
                let rect = Rect {
                    x,
                    y,
                    width,
                    height,
                };
                self.scene.quads.push(Quad {
                    rect,
                    colour: Colour::WHITE,
                });
                self.scene.text.push(TextRun {
                    text: label.clone(),
                    rect,
                    font_size: self.scaled(DEFAULT_TEXT_SIZE),
                    colour: Colour::BLACK,
                    align: TextAlign::Centre,
                });
                self.hit_regions.push(HitRegion {
                    rect,
                    action: action.clone(),
                });
                height
            }
        }
    }

    fn resolve_text(&self, parts: &[TextPart]) -> String {
        let mut text = String::new();
        for part in parts {
            match part {
                TextPart::Literal(value) => text.push_str(value),
                TextPart::State(state) => {
                    if let Some(value) = self.state.get(state.0) {
                        text.push_str(&value.to_string());
                    }
                }
            }
        }
        text
    }

    fn scaled(&self, value: f32) -> f32 {
        value * self.viewport.density
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct MeasuredSize {
    width: f32,
    height: f32,
}
