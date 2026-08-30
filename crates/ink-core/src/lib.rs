use std::fmt;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

pub const PUBLIC_SANS: &[u8] = include_bytes!("../../../assets/fonts/PublicSans-Regular.ttf");

const DEFAULT_TEXT_SIZE: f32 = 30.0;
const TEXT_INPUT_TEXT_SIZE: f32 = 24.0;
const TEXT_INPUT_HEIGHT: f32 = 38.0;
const TEXT_INPUT_BOTTOM_PADDING: f32 = 6.0;
const DEFAULT_ICON_SIZE: f32 = 28.0;
const BUTTON_HEIGHT: f32 = 40.0;
const BUTTON_ICON_SIZE: f32 = 30.0;
const BUTTON_ICON_GAP: f32 = 12.0;
const CONTENT_INSET_START: f32 = 37.0;
const CONTENT_INSET_END: f32 = 46.0;
const CONTENT_TOP: f32 = 14.0;
const CONTENT_GAP: f32 = 47.0;
const HEADER_HEIGHT: f32 = 42.0;
const HEADER_TEXT_SIZE: f32 = 20.0;
const NAV_HEIGHT: f32 = 70.0;
const NAV_ICON_SIZE: f32 = 48.0;
const NAV_VERTICAL_INSET: f32 = 10.0;
const TOGGLE_HEIGHT: f32 = 46.0;
const TOGGLE_ICON_SIZE: f32 = 9.8;
const TOGGLE_MASK_PADDING: f32 = 1.0 / LP3_REFERENCE_SCALE;
const TOGGLE_LINE_WIDTH: f32 = 14.5;
const TOGGLE_LINE_HEIGHT: f32 = 2.22;
const TOGGLE_START: f32 = 8.5;
const TOGGLE_LABEL_GAP: f32 = 20.0;
const SCROLL_TRACK_END: f32 = 34.0;
const SCROLL_TRACK_WIDTH: f32 = 1.0;
const SCROLL_THUMB_WIDTH: f32 = 5.0;
const TAP_SLOP: f32 = 8.0;
const LP3_REFERENCE_WIDTH: f32 = 1080.0;
const LP3_REFERENCE_SCALE: f32 = 2.55;
const PUBLIC_SANS_RASTER_SCALE: f32 = 7.0 / 6.0;

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
    Bool(bool),
}

impl fmt::Display for StateValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(value) => value.fmt(formatter),
            Self::Bool(value) => value.fmt(formatter),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Increment { state: StateId, by: i64 },
    SetInt { state: StateId, value: i64 },
    SetBool { state: StateId, value: bool },
    Toggle { state: StateId },
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Centre,
    End,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justification {
    #[default]
    Start,
    Centre,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Start,
    Centre,
    End,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Primary,
    Muted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mask {
    pub id: u64,
    pub width: u16,
    pub height: u16,
    pub pixels: &'static [u8],
}

impl Mask {
    pub const fn new(id: u64, width: u16, height: u16, pixels: &'static [u8]) -> Self {
        Self {
            id,
            width,
            height,
            pixels,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageAsset {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub compressed_pixels: &'static [u8],
}

impl ImageAsset {
    pub const fn new(id: u64, width: u32, height: u32, compressed_pixels: &'static [u8]) -> Self {
        Self {
            id,
            width,
            height,
            compressed_pixels,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageFit {
    #[default]
    Cover,
    Contain,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    kind: NodeKind,
}

#[derive(Clone, Debug, PartialEq)]
enum NodeKind {
    Screen {
        children: Vec<Node>,
        title: Option<String>,
        centred: bool,
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
    },
    TextInput {
        placeholder: String,
    },
    Button {
        label: String,
        icon: Option<Mask>,
        underline: bool,
        action: Option<Action>,
    },
    Icon {
        mask: Mask,
        size: f32,
        tone: Tone,
    },
    Image {
        asset: ImageAsset,
        width: f32,
        height: f32,
        fit: ImageFit,
    },
    Toggle {
        label: String,
        state: StateId,
        action: Action,
        off: Mask,
        on: Mask,
    },
    Tabs {
        state: StateId,
        tabs: Vec<Tab>,
    },
}

impl Node {
    pub fn screen(children: Vec<Self>, title: Option<String>, centred: bool) -> Self {
        Self {
            kind: NodeKind::Screen {
                children,
                title,
                centred,
            },
        }
    }

    pub fn stack(
        children: Vec<Self>,
        axis: Axis,
        gap: Option<f32>,
        align: Alignment,
        justify: Justification,
    ) -> Self {
        Self {
            kind: NodeKind::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            },
        }
    }

    pub fn text(parts: Vec<TextPart>, font_size: Option<f32>, align: TextAlign) -> Self {
        Self {
            kind: NodeKind::Text {
                parts,
                font_size,
                align,
            },
        }
    }

    pub fn text_input(placeholder: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::TextInput {
                placeholder: placeholder.into(),
            },
        }
    }

    pub fn button(
        label: impl Into<String>,
        icon: Option<Mask>,
        underline: bool,
        action: Option<Action>,
    ) -> Self {
        Self {
            kind: NodeKind::Button {
                label: label.into(),
                icon,
                underline,
                action,
            },
        }
    }

    pub const fn icon(mask: Mask, size: f32, tone: Tone) -> Self {
        Self {
            kind: NodeKind::Icon { mask, size, tone },
        }
    }

    pub const fn image(asset: ImageAsset, width: f32, height: f32, fit: ImageFit) -> Self {
        Self {
            kind: NodeKind::Image {
                asset,
                width,
                height,
                fit,
            },
        }
    }

    pub fn toggle(
        label: impl Into<String>,
        state: StateId,
        action: Action,
        off: Mask,
        on: Mask,
    ) -> Self {
        Self {
            kind: NodeKind::Toggle {
                label: label.into(),
                state,
                action,
                off,
                on,
            },
        }
    }

    pub fn tabs(state: StateId, tabs: Vec<Tab>) -> Self {
        Self {
            kind: NodeKind::Tabs { state, tabs },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tab {
    icon: Mask,
    action: Action,
    screen: Node,
}

impl Tab {
    pub const fn new(icon: Mask, action: Action, screen: Node) -> Self {
        Self {
            icon,
            action,
            screen,
        }
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

    fn intersection(self, other: Self) -> Self {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        Self {
            x: left,
            y: top,
            width: (right - left).max(0.0),
            height: (bottom - top).max(0.0),
        }
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
    pub const MUTED: Self = Self::rgb(110.0 / 255.0, 110.0 / 255.0, 110.0 / 255.0);

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
    pub clip: Rect,
    pub colour: Colour,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub rect: Rect,
    pub clip: Rect,
    pub font_size: f32,
    pub colour: Colour,
    pub align: TextAlign,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaskRun {
    pub mask: Mask,
    pub rect: Rect,
    pub clip: Rect,
    pub colour: Colour,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageRun {
    pub asset: ImageAsset,
    pub rect: Rect,
    pub clip: Rect,
    pub fit: ImageFit,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub width: u32,
    pub height: u32,
    pub quads: Vec<Quad>,
    pub text: Vec<TextRun>,
    pub masks: Vec<MaskRun>,
    pub images: Vec<ImageRun>,
}

#[derive(Clone, Debug)]
struct HitRegion {
    rect: Rect,
    action: Action,
}

#[derive(Clone, Copy, Debug)]
struct Pointer {
    start_y: f32,
    start_offset: f32,
}

pub struct Engine {
    definition: AppDefinition,
    state: Vec<StateValue>,
    viewport: Viewport,
    scene: Scene,
    hit_regions: Vec<HitRegion>,
    clip: Rect,
    scroll_offset: f32,
    scroll_max: f32,
    pointer: Option<Pointer>,
    font: FontRef<'static>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Viewport {
    width: u32,
    height: u32,
    scale: f32,
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
            clip: Rect::default(),
            scroll_offset: 0.0,
            scroll_max: 0.0,
            pointer: None,
            font: FontRef::try_from_slice(PUBLIC_SANS).expect("bundled Public Sans is valid"),
        }
    }

    pub fn set_viewport(&mut self, width: u32, height: u32) -> bool {
        let viewport = Viewport {
            width,
            height,
            scale: if width > 0 {
                width as f32 / LP3_REFERENCE_WIDTH * LP3_REFERENCE_SCALE
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

    pub fn pointer_down(&mut self, y: f32) {
        self.pointer = Some(Pointer {
            start_y: y,
            start_offset: self.scroll_offset,
        });
    }

    pub fn pointer_move(&mut self, y: f32) -> bool {
        let tap_slop = self.scaled(TAP_SLOP);
        let Some(pointer) = &mut self.pointer else {
            return false;
        };
        if (pointer.start_y - y).abs() <= tap_slop {
            return false;
        }
        let next = (pointer.start_offset + pointer.start_y - y).clamp(0.0, self.scroll_max);
        self.set_scroll_offset(next)
    }

    pub fn pointer_up(&mut self, x: f32, y: f32) -> bool {
        let Some(pointer) = self.pointer.take() else {
            return false;
        };
        if (pointer.start_y - y).abs() <= self.scaled(TAP_SLOP) {
            return self.tap(x, y);
        }
        false
    }

    pub fn pointer_cancel(&mut self) {
        self.pointer = None;
    }

    pub fn scroll_by(&mut self, delta: f32) -> bool {
        self.set_scroll_offset((self.scroll_offset + delta).clamp(0.0, self.scroll_max))
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
            }
            Action::SetInt { state, value } => {
                let Some(StateValue::Int(current)) = self.state.get_mut(state.0) else {
                    return false;
                };
                *current = value;
                self.scroll_offset = 0.0;
            }
            Action::SetBool { state, value } => {
                let Some(StateValue::Bool(current)) = self.state.get_mut(state.0) else {
                    return false;
                };
                *current = value;
            }
            Action::Toggle { state } => {
                let Some(StateValue::Bool(value)) = self.state.get_mut(state.0) else {
                    return false;
                };
                *value = !*value;
            }
        }
        true
    }

    fn set_scroll_offset(&mut self, offset: f32) -> bool {
        if (offset - self.scroll_offset).abs() < f32::EPSILON {
            return false;
        }
        self.scroll_offset = offset;
        self.rebuild_scene();
        true
    }

    fn rebuild_scene(&mut self) {
        self.scene = Scene {
            width: self.viewport.width,
            height: self.viewport.height,
            quads: Vec::new(),
            text: Vec::new(),
            masks: Vec::new(),
            images: Vec::new(),
        };
        self.hit_regions.clear();
        self.scroll_max = 0.0;

        if self.viewport.width == 0 || self.viewport.height == 0 {
            return;
        }

        let root = self.definition.root.clone();
        self.clip = Rect {
            x: 0.0,
            y: 0.0,
            width: self.viewport.width as f32,
            height: self.viewport.height as f32,
        };
        self.layout(
            &root,
            Rect {
                x: 0.0,
                y: 0.0,
                width: self.viewport.width as f32,
                height: self.viewport.height as f32,
            },
        );
    }

    fn measure(&self, node: &Node, available: Rect) -> MeasuredSize {
        match &node.kind {
            NodeKind::Screen { .. } | NodeKind::Tabs { .. } => MeasuredSize {
                width: available.width,
                height: available.height,
            },
            NodeKind::Stack {
                children,
                axis,
                gap,
                ..
            } => {
                let gap = self.scaled(gap.unwrap_or_default());
                let measured: Vec<_> = children
                    .iter()
                    .map(|child| self.measure(child, available))
                    .collect();
                match axis {
                    Axis::Vertical => MeasuredSize {
                        width: measured
                            .iter()
                            .map(|size| size.width)
                            .fold(0.0, f32::max)
                            .min(available.width),
                        height: (measured.iter().map(|size| size.height).sum::<f32>()
                            + gap * children.len().saturating_sub(1) as f32)
                            .min(available.height),
                    },
                    Axis::Horizontal => MeasuredSize {
                        width: (measured.iter().map(|size| size.width).sum::<f32>()
                            + gap * children.len().saturating_sub(1) as f32)
                            .min(available.width),
                        height: measured
                            .iter()
                            .map(|size| size.height)
                            .fold(0.0, f32::max)
                            .min(available.height),
                    },
                }
            }
            NodeKind::Text {
                parts, font_size, ..
            } => {
                let logical_size = self.scaled(font_size.unwrap_or(DEFAULT_TEXT_SIZE));
                let font_size = logical_size * PUBLIC_SANS_RASTER_SCALE;
                MeasuredSize {
                    width: self
                        .text_width(&self.resolve_text(parts), font_size)
                        .min(available.width),
                    height: (logical_size * 1.25).min(available.height),
                }
            }
            NodeKind::TextInput { .. } => MeasuredSize {
                width: available.width,
                height: self.scaled(TEXT_INPUT_HEIGHT).min(available.height),
            },
            NodeKind::Button { label, icon, .. } => {
                let font_size = self.scaled_font(DEFAULT_TEXT_SIZE);
                let icon_width = icon
                    .map(|_| self.scaled(BUTTON_ICON_SIZE + BUTTON_ICON_GAP))
                    .unwrap_or_default();
                MeasuredSize {
                    width: (self.text_width(label, font_size).ceil() + 1.0 + icon_width)
                        .min(available.width),
                    height: self.scaled(BUTTON_HEIGHT).min(available.height),
                }
            }
            NodeKind::Icon { size, .. } => {
                let size = self.scaled(if *size > 0.0 {
                    *size
                } else {
                    DEFAULT_ICON_SIZE
                });
                MeasuredSize {
                    width: size.min(available.width),
                    height: size.min(available.height),
                }
            }
            NodeKind::Image { width, height, .. } => MeasuredSize {
                width: self.scaled(*width).min(available.width),
                height: self.scaled(*height).min(available.height),
            },
            NodeKind::Toggle { .. } => MeasuredSize {
                width: available.width,
                height: self.scaled(TOGGLE_HEIGHT).min(available.height),
            },
        }
    }

    fn layout(&mut self, node: &Node, rect: Rect) {
        match &node.kind {
            NodeKind::Screen {
                children,
                title,
                centred,
            } => self.layout_screen(children, title.as_deref(), *centred, rect),
            NodeKind::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            } => self.layout_stack(
                children,
                *axis,
                self.scaled(gap.unwrap_or_default()),
                *align,
                *justify,
                rect,
            ),
            NodeKind::Text {
                parts,
                font_size,
                align,
            } => {
                let font_size = self.scaled_font(font_size.unwrap_or(DEFAULT_TEXT_SIZE));
                self.scene.text.push(TextRun {
                    text: self.resolve_text(parts),
                    rect,
                    clip: self.clip,
                    font_size,
                    colour: Colour::WHITE,
                    align: *align,
                });
            }
            NodeKind::TextInput { placeholder } => self.layout_text_input(placeholder, rect),
            NodeKind::Button {
                label,
                icon,
                underline,
                action,
            } => self.layout_button(label, *icon, *underline, action, rect),
            NodeKind::Icon { mask, tone, .. } => self.scene.masks.push(MaskRun {
                mask: *mask,
                rect,
                clip: self.clip,
                colour: tone_colour(*tone),
            }),
            NodeKind::Image { asset, fit, .. } => self.scene.images.push(ImageRun {
                asset: *asset,
                rect,
                clip: self.clip,
                fit: *fit,
            }),
            NodeKind::Toggle {
                label,
                state,
                action,
                off,
                on,
            } => self.layout_toggle(label, *state, action, *off, *on, rect),
            NodeKind::Tabs { state, tabs } => self.layout_tabs(*state, tabs, rect),
        }
    }

    fn layout_screen(&mut self, children: &[Node], title: Option<&str>, centred: bool, rect: Rect) {
        let header_height = title.map_or(0.0, |_| self.scaled(HEADER_HEIGHT));
        if let Some(title) = title {
            let inset = self.scaled(22.0);
            self.scene.text.push(TextRun {
                text: title.to_owned(),
                rect: Rect {
                    x: rect.x + inset,
                    y: rect.y + self.scaled(7.0),
                    width: (rect.width - inset * 2.0).max(0.0),
                    height: self.scaled(32.0),
                },
                clip: self.clip,
                font_size: self.scaled_font(HEADER_TEXT_SIZE),
                colour: Colour::WHITE,
                align: TextAlign::Centre,
            });
        }

        let inset_start = self.scaled(CONTENT_INSET_START);
        let inset_end = self.scaled(CONTENT_INSET_END);
        let content = Rect {
            x: rect.x + inset_start,
            y: rect.y + header_height + self.scaled(CONTENT_TOP),
            width: (rect.width - inset_start - inset_end).max(0.0),
            height: (rect.height - header_height - self.scaled(CONTENT_TOP)).max(0.0),
        };
        let gap = self.scaled(CONTENT_GAP);
        let content_height = children
            .iter()
            .map(|child| self.measure(child, content).height)
            .sum::<f32>()
            + gap * children.len().saturating_sub(1) as f32;
        self.scroll_max = (content_height - content.height).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, self.scroll_max);

        let scroll_clip = Rect {
            x: rect.x,
            y: content.y,
            width: rect.width,
            height: content.height,
        };
        let previous_clip = self.clip;
        self.clip = self.clip.intersection(scroll_clip);
        self.layout_vertical_children(
            children,
            gap,
            Alignment::Stretch,
            if centred && self.scroll_max == 0.0 {
                Justification::Centre
            } else {
                Justification::Start
            },
            Rect {
                y: content.y - self.scroll_offset,
                height: content.height.max(content_height),
                ..content
            },
        );
        self.clip = previous_clip;

        if self.scroll_max > 0.0 {
            let track_width = self.scaled(SCROLL_TRACK_WIDTH);
            let thumb_width = self.scaled(SCROLL_THUMB_WIDTH);
            let track_x = rect.x + rect.width - self.scaled(SCROLL_TRACK_END);
            let thumb_height = (content.height * content.height / content_height)
                .clamp(thumb_width, content.height);
            let thumb_y =
                content.y + self.scroll_offset / self.scroll_max * (content.height - thumb_height);
            let clip = previous_clip.intersection(scroll_clip);
            self.scene.quads.push(Quad {
                rect: Rect {
                    x: track_x,
                    y: content.y,
                    width: track_width,
                    height: content.height,
                },
                clip,
                colour: Colour::WHITE,
            });
            self.scene.quads.push(Quad {
                rect: Rect {
                    x: track_x - (thumb_width - track_width) / 2.0,
                    y: thumb_y,
                    width: thumb_width,
                    height: thumb_height,
                },
                clip,
                colour: Colour::WHITE,
            });
        }
    }

    fn layout_stack(
        &mut self,
        children: &[Node],
        axis: Axis,
        gap: f32,
        align: Alignment,
        justify: Justification,
        rect: Rect,
    ) {
        match axis {
            Axis::Vertical => self.layout_vertical_children(children, gap, align, justify, rect),
            Axis::Horizontal => {
                self.layout_horizontal_children(children, gap, align, justify, rect)
            }
        }
    }

    fn layout_vertical_children(
        &mut self,
        children: &[Node],
        gap: f32,
        align: Alignment,
        justify: Justification,
        rect: Rect,
    ) {
        let sizes: Vec<_> = children
            .iter()
            .map(|child| self.measure(child, rect))
            .collect();
        let content_height = sizes.iter().map(|size| size.height).sum::<f32>()
            + gap * children.len().saturating_sub(1) as f32;
        let (mut cursor, actual_gap) = distribution(
            rect.y,
            rect.height,
            content_height,
            gap,
            children.len(),
            justify,
        );
        for (child, size) in children.iter().zip(sizes) {
            let width = if align == Alignment::Stretch && stretchable(child) {
                rect.width
            } else {
                size.width.min(rect.width)
            };
            let x = cross_position(rect.x, rect.width, width, align);
            self.layout(
                child,
                Rect {
                    x,
                    y: cursor,
                    width,
                    height: size.height,
                },
            );
            cursor += size.height + actual_gap;
        }
    }

    fn layout_horizontal_children(
        &mut self,
        children: &[Node],
        gap: f32,
        align: Alignment,
        justify: Justification,
        rect: Rect,
    ) {
        let sizes: Vec<_> = children
            .iter()
            .map(|child| self.measure(child, rect))
            .collect();
        let content_width = sizes.iter().map(|size| size.width).sum::<f32>()
            + gap * children.len().saturating_sub(1) as f32;
        let (mut cursor, actual_gap) = distribution(
            rect.x,
            rect.width,
            content_width,
            gap,
            children.len(),
            justify,
        );
        for (child, size) in children.iter().zip(sizes) {
            let height = if align == Alignment::Stretch && stretchable(child) {
                rect.height
            } else {
                size.height.min(rect.height)
            };
            let y = cross_position(rect.y, rect.height, height, align);
            self.layout(
                child,
                Rect {
                    x: cursor,
                    y,
                    width: size.width,
                    height,
                },
            );
            cursor += size.width + actual_gap;
        }
    }

    fn layout_button(
        &mut self,
        label: &str,
        icon: Option<Mask>,
        underline: bool,
        action: &Option<Action>,
        rect: Rect,
    ) {
        let mut text_x = rect.x;
        if let Some(mask) = icon {
            let size = self.scaled(BUTTON_ICON_SIZE);
            self.scene.masks.push(MaskRun {
                mask,
                rect: Rect {
                    x: rect.x,
                    y: rect.y + (rect.height - size).max(0.0) / 2.0,
                    width: size,
                    height: size,
                },
                clip: self.clip,
                colour: Colour::WHITE,
            });
            text_x += size + self.scaled(BUTTON_ICON_GAP);
        }

        let font_size = self.scaled_font(DEFAULT_TEXT_SIZE);
        let text_rect = Rect {
            x: text_x,
            y: rect.y + self.scaled(1.0),
            width: (rect.x + rect.width - text_x).max(0.0),
            height: (rect.height - self.scaled(1.0)).max(0.0),
        };
        let visible_label = self.ellipsize(label, font_size, text_rect.width);
        let text_width = self.text_width(&visible_label, font_size);
        self.scene.text.push(TextRun {
            text: visible_label,
            rect: text_rect,
            clip: self.clip,
            font_size,
            colour: Colour::WHITE,
            align: TextAlign::Start,
        });
        if underline {
            self.scene.quads.push(Quad {
                rect: Rect {
                    x: text_x,
                    y: rect.y + rect.height - self.scaled(2.0),
                    width: text_width.min(text_rect.width),
                    height: self.scaled(1.0),
                },
                clip: self.clip,
                colour: Colour::WHITE,
            });
        }
        if let Some(action) = action {
            self.push_hit_region(rect, action.clone());
        }
    }

    fn layout_text_input(&mut self, placeholder: &str, rect: Rect) {
        let text_height = (rect.height - self.scaled(TEXT_INPUT_BOTTOM_PADDING)).max(0.0);
        self.scene.text.push(TextRun {
            text: placeholder.to_owned(),
            rect: Rect {
                height: text_height,
                ..rect
            },
            clip: self.clip,
            font_size: self.scaled_font(TEXT_INPUT_TEXT_SIZE),
            colour: Colour::WHITE,
            align: TextAlign::Start,
        });
        self.scene.quads.push(Quad {
            rect: Rect {
                x: rect.x,
                y: rect.y + rect.height - self.scaled(1.0),
                width: rect.width,
                height: self.scaled(1.0),
            },
            clip: self.clip,
            colour: Colour::WHITE,
        });
    }

    fn layout_toggle(
        &mut self,
        label: &str,
        state: StateId,
        action: &Action,
        off: Mask,
        on: Mask,
        rect: Rect,
    ) {
        let enabled = matches!(self.state.get(state.0), Some(StateValue::Bool(true)));
        let icon_size = self.scaled(TOGGLE_ICON_SIZE);
        let mask_padding = self.scaled(TOGGLE_MASK_PADDING);
        let mask_size = icon_size + mask_padding * 2.0;
        let line_width = self.scaled(TOGGLE_LINE_WIDTH);
        let line_height = self.scaled(TOGGLE_LINE_HEIGHT);
        let mut x = rect.x + self.scaled(TOGGLE_START);
        let centre_y = rect.y + rect.height / 2.0;

        if enabled {
            self.scene.quads.push(Quad {
                rect: Rect {
                    x,
                    y: centre_y - line_height / 2.0,
                    width: line_width,
                    height: line_height,
                },
                clip: self.clip,
                colour: Colour::WHITE,
            });
            x += line_width;
            self.scene.masks.push(MaskRun {
                mask: on,
                rect: Rect {
                    x: x - mask_padding,
                    y: centre_y - mask_size / 2.0,
                    width: mask_size,
                    height: mask_size,
                },
                clip: self.clip,
                colour: Colour::WHITE,
            });
        } else {
            self.scene.masks.push(MaskRun {
                mask: off,
                rect: Rect {
                    x: x - mask_padding,
                    y: centre_y - mask_size / 2.0,
                    width: mask_size,
                    height: mask_size,
                },
                clip: self.clip,
                colour: Colour::WHITE,
            });
            x += icon_size;
            self.scene.quads.push(Quad {
                rect: Rect {
                    x,
                    y: centre_y - line_height / 2.0,
                    width: line_width,
                    height: line_height,
                },
                clip: self.clip,
                colour: Colour::WHITE,
            });
        }

        let label_x = rect.x
            + self.scaled(TOGGLE_START + TOGGLE_ICON_SIZE + TOGGLE_LINE_WIDTH + TOGGLE_LABEL_GAP);
        self.scene.text.push(TextRun {
            text: label.to_owned(),
            rect: Rect {
                x: label_x,
                y: rect.y,
                width: (rect.x + rect.width - label_x).max(0.0),
                height: rect.height,
            },
            clip: self.clip,
            font_size: self.scaled_font(DEFAULT_TEXT_SIZE),
            colour: Colour::WHITE,
            align: TextAlign::Start,
        });
        self.push_hit_region(rect, action.clone());
    }

    fn layout_tabs(&mut self, state: StateId, tabs: &[Tab], rect: Rect) {
        let nav_height = self.scaled(NAV_HEIGHT).min(rect.height);
        let content = Rect {
            height: (rect.height - nav_height).max(0.0),
            ..rect
        };
        let active = match self.state.get(state.0) {
            Some(StateValue::Int(value)) if *value >= 0 => *value as usize,
            _ => 0,
        }
        .min(tabs.len().saturating_sub(1));
        if let Some(tab) = tabs.get(active) {
            self.layout(&tab.screen, content);
        }

        if tabs.is_empty() {
            return;
        }
        let nav_y = rect.y + rect.height - nav_height;
        let icon_size = self.scaled(NAV_ICON_SIZE).min(nav_height);
        let icon_y = nav_y + self.scaled(NAV_VERTICAL_INSET);
        let nav_inset = self.scaled(20.0);
        let first_centre = rect.x + nav_inset + icon_size / 2.0;
        let last_centre = rect.x + rect.width - nav_inset - icon_size / 2.0;
        let centre_gap = if tabs.len() > 1 {
            (last_centre - first_centre) / (tabs.len() - 1) as f32
        } else {
            0.0
        };
        let centres: Vec<_> = (0..tabs.len())
            .map(|index| {
                if tabs.len() == 1 {
                    rect.x + rect.width / 2.0
                } else {
                    first_centre + centre_gap * index as f32
                }
            })
            .collect();
        for (index, tab) in tabs.iter().enumerate() {
            let left = if index == 0 {
                rect.x
            } else {
                (centres[index - 1] + centres[index]) / 2.0
            };
            let right = if index + 1 == tabs.len() {
                rect.x + rect.width
            } else {
                (centres[index] + centres[index + 1]) / 2.0
            };
            let slot = Rect {
                x: left,
                y: nav_y,
                width: right - left,
                height: nav_height,
            };
            self.scene.masks.push(MaskRun {
                mask: tab.icon,
                rect: Rect {
                    x: centres[index] - icon_size / 2.0,
                    y: icon_y,
                    width: icon_size,
                    height: icon_size,
                },
                clip: self.clip,
                colour: if index == active {
                    Colour::WHITE
                } else {
                    Colour::MUTED
                },
            });
            self.push_hit_region(slot, tab.action.clone());
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

    fn text_width(&self, text: &str, font_size: f32) -> f32 {
        let scaled = self.font.as_scaled(PxScale::from(font_size));
        let mut previous = None;
        text.chars().fold(0.0, |width, character| {
            let glyph = scaled.glyph_id(character);
            let kerning = previous
                .map(|previous| scaled.kern(previous, glyph))
                .unwrap_or_default();
            previous = Some(glyph);
            width + kerning + scaled.h_advance(glyph)
        })
    }

    fn ellipsize(&self, text: &str, font_size: f32, available_width: f32) -> String {
        if self.text_width(text, font_size) <= available_width {
            return text.to_owned();
        }
        let ellipsis = '…';
        let ellipsis_width = self.text_width("…", font_size);
        let mut visible = text.to_owned();
        while !visible.is_empty()
            && self.text_width(&visible, font_size) + ellipsis_width > available_width
        {
            visible.pop();
        }
        visible.push(ellipsis);
        visible
    }

    fn scaled(&self, value: f32) -> f32 {
        value * self.viewport.scale
    }

    fn scaled_font(&self, value: f32) -> f32 {
        self.scaled(value) * PUBLIC_SANS_RASTER_SCALE
    }

    fn push_hit_region(&mut self, rect: Rect, action: Action) {
        let rect = rect.intersection(self.clip);
        if rect.width > 0.0 && rect.height > 0.0 {
            self.hit_regions.push(HitRegion { rect, action });
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct MeasuredSize {
    width: f32,
    height: f32,
}

fn tone_colour(tone: Tone) -> Colour {
    match tone {
        Tone::Primary => Colour::WHITE,
        Tone::Muted => Colour::MUTED,
    }
}

fn stretchable(node: &Node) -> bool {
    matches!(
        &node.kind,
        NodeKind::Stack { .. }
            | NodeKind::Text { .. }
            | NodeKind::TextInput { .. }
            | NodeKind::Button { .. }
            | NodeKind::Toggle { .. }
    )
}

fn cross_position(origin: f32, available: f32, size: f32, align: Alignment) -> f32 {
    match align {
        Alignment::Start | Alignment::Stretch => origin,
        Alignment::Centre => origin + (available - size).max(0.0) / 2.0,
        Alignment::End => origin + (available - size).max(0.0),
    }
}

fn distribution(
    origin: f32,
    available: f32,
    content: f32,
    gap: f32,
    count: usize,
    justify: Justification,
) -> (f32, f32) {
    let remaining = (available - content).max(0.0);
    match justify {
        Justification::Start => (origin, gap),
        Justification::Centre => (origin + remaining / 2.0, gap),
        Justification::End => (origin + remaining, gap),
        Justification::SpaceBetween if count > 1 => (origin, gap + remaining / (count - 1) as f32),
        Justification::SpaceBetween => (origin, gap),
    }
}
