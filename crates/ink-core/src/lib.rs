#[cfg(feature = "perf")]
use std::time::Instant;
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use unicode_linebreak::linebreaks;
use unicode_properties::emoji::{
    EmojiStatus, UnicodeEmoji, is_emoji_presentation_selector, is_regional_indicator,
    is_text_presentation_selector, is_zwj,
};
use unicode_segmentation::UnicodeSegmentation;

mod list;
mod masks;
mod react;

pub use react::{ReactIcon, ReactTree};

#[cfg(all(feature = "perf", target_os = "android"))]
#[link(name = "android")]
unsafe extern "C" {
    fn ATrace_beginSection(section_name: *const std::ffi::c_char);
    fn ATrace_endSection();
    fn ATrace_setCounter(counter_name: *const std::ffi::c_char, counter_value: i64);
}

#[cfg(feature = "perf")]
#[doc(hidden)]
pub struct PerfTraceSection;

#[cfg(feature = "perf")]
impl PerfTraceSection {
    pub fn new(name: &'static [u8]) -> Self {
        debug_assert_eq!(name.last(), Some(&0));
        #[cfg(target_os = "android")]
        unsafe {
            ATrace_beginSection(name.as_ptr().cast());
        }
        Self
    }
}

#[cfg(feature = "perf")]
impl Drop for PerfTraceSection {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        unsafe {
            ATrace_endSection();
        }
    }
}

#[cfg(feature = "perf")]
#[doc(hidden)]
pub fn perf_trace_counter(name: &'static [u8], _value: u64) {
    debug_assert_eq!(name.last(), Some(&0));
    #[cfg(target_os = "android")]
    unsafe {
        ATrace_setCounter(name.as_ptr().cast(), _value.min(i64::MAX as u64) as i64);
    }
}

pub const PUBLIC_SANS: &[u8] = include_bytes!("../../../assets/fonts/PublicSans-Regular.ttf");

const DEFAULT_TEXT_SIZE: f32 = 30.0;
const TEXT_INPUT_TEXT_SIZE: f32 = 24.0;
const TEXT_INPUT_HEIGHT: f32 = 38.0;
const TEXT_INPUT_BOTTOM_PADDING: f32 = 6.0;
const TEXT_INPUT_MAX_LINES: usize = 3;
const TEXT_INPUT_CLEAR_ICON_SIZE: f32 = 24.0;
const TEXT_INPUT_CLEAR_GAP: f32 = 20.0;
const TEXT_INPUT_CLEAR_PADDING: f32 = 5.0;
const CONTROL_LINE_HEIGHT: f32 = 1.0;
const DEFAULT_ICON_SIZE: f32 = 28.0;
const BUTTON_HEIGHT: f32 = 40.0;
const BUTTON_ICON_SIZE: f32 = 30.0;
const BUTTON_ICON_GAP: f32 = 12.0;
const FIELD_LABEL_SIZE: f32 = 20.0;
const FIELD_LABEL_HEIGHT: f32 = 25.0;
const CONTENT_INSET_START: f32 = 37.0;
const CONTENT_INSET_END: f32 = CONTENT_INSET_START;
const CONTENT_TOP: f32 = 14.0;
const CONTENT_BOTTOM: f32 = 20.0;
const CONTENT_GAP: f32 = 47.0;
const HEADER_HEIGHT: f32 = 50.0;
const HEADER_TEXT_SIZE: f32 = 20.0;
const HEADER_HORIZONTAL_INSET: f32 = 22.0;
const HEADER_BUTTON_SIZE: f32 = 32.0;
const HEADER_BACK_ICON_SIZE: f32 = 28.0;
const HEADER_BACK_OFFSET_X: f32 = -7.0;
const HEADER_BACK_OFFSET_Y: f32 = 11.0;
const HEADER_CONTENT_TOP: f32 = 6.0;
const NAV_HEIGHT: f32 = 70.0;
const NAV_ICON_SIZE: f32 = 52.0;
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
const SCROLL_CONTENT_INSET_END: f32 = SCROLL_TRACK_END * 2.0 - SCROLL_TRACK_WIDTH;
const SCROLL_THUMB_WIDTH: f32 = 5.0;
const SCROLL_THUMB_TOUCH_MULTIPLIER: f32 = 6.0;
const MIN_SCROLL_THUMB_FRACTION: f32 = 0.1;
const MAX_SCROLL_THUMB_FRACTION: f32 = 0.85;
const TAP_SLOP: f32 = 8.0;
const BACK_SWIPE_EDGE_WIDTH: f32 = 30.0;
const BACK_SWIPE_ACTIVATION_DISTANCE: f32 = 12.0;
const BACK_SWIPE_TRIGGER_DISTANCE: f32 = 80.0;
const BACK_SWIPE_VERTICAL_RATIO: f32 = 1.5;
const IMAGE_MAX_SCALE: f32 = 4.0;
const LP3_REFERENCE_WIDTH: f32 = 1080.0;
const LP3_REFERENCE_SCALE: f32 = 2.55;
const PUBLIC_SANS_RASTER_SCALE: f32 = 7.0 / 6.0;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct StateId(usize);

impl StateId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ControllerId(usize);

impl ControllerId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
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

impl fmt::Display for StateValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => Ok(()),
            Self::Number(value) if value.fract() == 0.0 => write!(formatter, "{value:.0}"),
            Self::Number(value) => value.fmt(formatter),
            Self::Bool(value) => value.fmt(formatter),
            Self::String(value) => value.fmt(formatter),
            Self::List(values) => write!(formatter, "{} items", values.len()),
            Self::Object(_) => formatter.write_str("object"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeOperation {
    module: String,
    operation: String,
    payload: String,
    timeout_ms: u64,
}

impl NativeOperation {
    pub fn new(
        module: impl Into<String>,
        operation: impl Into<String>,
        payload: impl Into<String>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            module: module.into(),
            operation: operation.into(),
            payload: payload.into(),
            timeout_ms,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceErrorKind {
    Busy,
    Unavailable,
    PermissionDenied,
    PermissionBlocked,
    LocationDisabled,
    NfcDisabled,
    Timeout,
    Protocol,
    Unexpected,
}

impl ResourceErrorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::Unavailable => "unavailable",
            Self::PermissionDenied => "permission-denied",
            Self::PermissionBlocked => "permission-blocked",
            Self::LocationDisabled => "location-disabled",
            Self::NfcDisabled => "nfc-disabled",
            Self::Timeout => "timeout",
            Self::Protocol => "protocol",
            Self::Unexpected => "unexpected",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceError {
    kind: ResourceErrorKind,
    message: String,
    retryable: bool,
}

impl ResourceError {
    pub fn new(kind: ResourceErrorKind, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeRequestKind {
    Action,
    Cancel,
    Image,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeRequest {
    id: u64,
    kind: NativeRequestKind,
    operation: Option<NativeOperation>,
    payload: String,
    controller: Option<ControllerId>,
}

impl NativeRequest {
    pub const fn id(&self) -> u64 {
        self.id
    }

    pub const fn kind(&self) -> NativeRequestKind {
        self.kind
    }

    pub fn module(&self) -> &str {
        self.operation
            .as_ref()
            .map_or("", |operation| &operation.module)
    }

    pub fn operation(&self) -> &str {
        self.operation
            .as_ref()
            .map_or("", |operation| &operation.operation)
    }

    pub fn payload(&self) -> &str {
        &self.payload
    }

    pub const fn controller(&self) -> Option<ControllerId> {
        self.controller
    }

    pub fn timeout_ms(&self) -> u64 {
        self.operation
            .as_ref()
            .map_or(0, |operation| operation.timeout_ms)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextEdit {
    Insert(String),
    Backspace,
    Submit,
    Dismiss,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextInputAction {
    Return,
    #[default]
    Search,
    Done,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Seek { id: usize, left: f32, width: f32, duration: f32 },
    ClearInput {
        state: StateId,
    },
    Native {
        operation: NativeOperation,
    },
    FocusTextInput {
        state: StateId,
        action: TextInputAction,
    },
    Back,
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
    Justify,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Primary,
    Muted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mask {
    pub id: u64,
    pub width: u16,
    pub height: u16,
    pub pixels: AssetBytes,
}

impl Mask {
    pub const fn new(id: u64, width: u16, height: u16, pixels: &'static [u8]) -> Self {
        Self {
            id,
            width,
            height,
            pixels: AssetBytes::Static(pixels),
        }
    }

    fn owned(id: u64, width: u16, height: u16, pixels: Vec<u8>) -> Self {
        Self {
            id,
            width,
            height,
            pixels: AssetBytes::Owned(pixels.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageAsset {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub encoding: ImageAssetEncoding,
    pub bytes: AssetBytes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageAssetEncoding {
    RgbaZlib,
    Jpeg,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetBytes {
    Static(&'static [u8]),
    Owned(Arc<[u8]>),
}

impl AsRef<[u8]> for AssetBytes {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Static(bytes) => bytes,
            Self::Owned(bytes) => bytes,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RemoteImage {
    pub id: u64,
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<[u8]>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImageData {
    Asset(ImageAsset),
    Remote(RemoteImage),
}

impl ImageData {
    pub const fn id(&self) -> u64 {
        match self {
            Self::Asset(asset) => asset.id,
            Self::Remote(image) => image.id,
        }
    }

    pub const fn generation(&self) -> u64 {
        match self {
            Self::Asset(_) => 0,
            Self::Remote(image) => image.generation,
        }
    }

    const fn width(&self) -> u32 {
        match self {
            Self::Asset(asset) => asset.width,
            Self::Remote(image) => image.width,
        }
    }

    const fn height(&self) -> u32 {
        match self {
            Self::Asset(asset) => asset.height,
            Self::Remote(image) => image.height,
        }
    }
}

fn image_content_rect(image: &ImageData, mut rect: Rect, fit: ImageFit) -> Rect {
    if rect.width <= 0.0 || rect.height <= 0.0 || image.height() == 0 {
        return rect;
    }
    let image_aspect = image.width() as f32 / image.height() as f32;
    let rect_aspect = rect.width / rect.height;
    match fit {
        ImageFit::Cover => rect,
        ImageFit::Contain if image_aspect > rect_aspect => {
            let height = rect.width / image_aspect;
            rect.y += (rect.height - height) / 2.0;
            rect.height = height;
            rect
        }
        ImageFit::Contain => {
            let width = rect.height * image_aspect;
            rect.x += (rect.width - width) / 2.0;
            rect.width = width;
            rect
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImageSource {
    Asset(ImageAsset),
    Native(String, String),
}

impl ImageAsset {
    pub const fn new(id: u64, width: u32, height: u32, compressed_pixels: &'static [u8]) -> Self {
        Self {
            id,
            width,
            height,
            encoding: ImageAssetEncoding::RgbaZlib,
            bytes: AssetBytes::Static(compressed_pixels),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    identity: NodeIdentity,
    kind: NodeKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct NodeIdentity(usize);

#[derive(Clone, Debug, PartialEq)]
enum NodeKind {
    MediaGridRow { children: Vec<Node> },
    MediaCell { source: ImageSource, selected: bool, video: bool, check: Mask, play: Mask, action: Option<Action> },
    Message { children: Vec<Node>, outgoing: bool },
    MessageQuote { children: Vec<Node> },
    ConversationComposer { children: Vec<Node> },
    PlayingLayout { children: Vec<Node>, centred: bool },
    PlayingPressable { children: Vec<Node>, action: Option<Action>, long_action: Option<Action>, selected: bool },
    PlayingTransport { children: Vec<Node> },
    PlayingProgress { position: f32, duration: f32, seek: bool },
    Row {
        children: Vec<Node>,
        has_image: bool,
        action: Option<Action>,
    },
    ReactList {
        children: Vec<Node>,
        start: usize,
        keys: Arc<[String]>,
        content_versions: Arc<[u64]>,
        revision: u64,
        gap: f32,
        follow_end: bool,
    },
    Screen {
        children: Vec<Node>,
        title: Option<String>,
        centred: bool,
        footer: Option<(String, Option<Action>)>,
        pinned_header: bool,
        pinned_footer: bool,
        right_action: Option<(Mask, Action)>,
        media_picker: bool,
    },
    Stack {
        children: Vec<Node>,
        axis: Axis,
        gap: Option<f32>,
        align: Alignment,
        justify: Justification,
    },
    Text {
        text: String,
        font_size: Option<f32>,
        align: TextAlign,
        max_lines: Option<u32>,
    },
    TextInput {
        placeholder: String,
        state: StateId,
        action: TextInputAction,
        auto_focus: bool,
        clear: Mask,
    },
    Button {
        label: String,
        icon: Option<Mask>,
        underline: bool,
        action: Option<Action>,
    },
    Field {
        label: String,
        value: String,
        action: Option<Action>,
    },
    Icon {
        mask: Mask,
        size: f32,
        tone: Tone,
        bounds: Option<Rect>,
    },
    Image {
        source: ImageSource,
        fallback: Option<ImageAsset>,
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
    MapView {
        controller: ControllerId,
    },
    Toggle {
        label: String,
        value: bool,
        action: Option<Action>,
        off: Mask,
        on: Mask,
    },
    Tabs {
        value: usize,
        tabs: Vec<Tab>,
    },
}

impl Node {
    pub fn screen(children: Vec<Self>, title: Option<String>, centred: bool) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Screen {
                children,
                title,
                centred,
                footer: None,
                pinned_header: false,
                pinned_footer: false,
                right_action: None,
                media_picker: false,
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
            identity: NodeIdentity(0),
            kind: NodeKind::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            },
        }
    }

    pub fn text(
        text: String,
        font_size: Option<f32>,
        align: TextAlign,
        max_lines: Option<u32>,
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Text {
                text,
                font_size,
                align,
                max_lines,
            },
        }
    }

    pub fn text_input(
        placeholder: impl Into<String>,
        state: StateId,
        action: TextInputAction,
        auto_focus: bool,
        clear: Mask,
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::TextInput {
                placeholder: placeholder.into(),
                state,
                action,
                auto_focus,
                clear,
            },
        }
    }

    pub fn button(
        label: String,
        icon: Option<Mask>,
        underline: bool,
        action: Option<Action>,
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Button {
                label,
                icon,
                underline,
                action,
            },
        }
    }

    pub fn field(label: impl Into<String>, value: String, action: Option<Action>) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Field {
                label: label.into(),
                value,
                action,
            },
        }
    }

    pub const fn icon(mask: Mask, size: f32, tone: Tone) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Icon { mask, size, tone, bounds: None },
        }
    }

    pub fn image(
        source: ImageSource,
        fallback: Option<ImageAsset>,
        bleed: bool,
        zoomable: bool,
        width: f32,
        height: f32,
        fit: ImageFit,
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Image {
                source,
                fallback,
                bleed,
                zoomable,
                width,
                height,
                fit,
            },
        }
    }

    pub const fn camera_preview(controller: ControllerId, kind: CameraPreviewKind) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::CameraPreview { controller, kind },
        }
    }

    pub const fn map_view(controller: ControllerId) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::MapView { controller },
        }
    }

    pub fn tabs_with_value(value: usize, tabs: Vec<Tab>) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Tabs { value, tabs },
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
    pub const MUTED: Self = Self::rgb(64.0 / 255.0, 64.0 / 255.0, 64.0 / 255.0);

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
    pub scrolling: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub rect: Rect,
    pub clip: Rect,
    pub font_size: f32,
    pub colour: Colour,
    pub align: TextAlign,
    pub scrolling: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaskRun {
    pub mask: Mask,
    pub rect: Rect,
    pub clip: Rect,
    pub colour: Colour,
    pub scrolling: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageRun {
    pub image: ImageData,
    pub zoom_id: Option<usize>,
    pub rect: Rect,
    pub clip: Rect,
    pub fit: ImageFit,
    pub scrolling: bool,
    pub transform: ImageTransform,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageTransform {
    pub scale: f32,
    pub translation_x: f32,
    pub translation_y: f32,
}

impl Default for ImageTransform {
    fn default() -> Self {
        Self {
            scale: 1.0,
            translation_x: 0.0,
            translation_y: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollBar {
    pub track: Rect,
    pub thumb_width: f32,
}

impl ScrollBar {
    pub fn thumb_rect(self, scroll_offset: f32, scroll_max: f32) -> Rect {
        let content_height = self.track.height + scroll_max;
        let visible_fraction = self.track.height / content_height;
        let height = self.track.height
            * visible_fraction.clamp(MIN_SCROLL_THUMB_FRACTION, MAX_SCROLL_THUMB_FRACTION);
        let travel = self.track.height - height;
        let fraction = (scroll_offset / scroll_max).clamp(0.0, 1.0);

        Rect {
            x: self.track.x - (self.thumb_width - self.track.width) / 2.0,
            y: self.track.y + fraction * travel,
            width: self.thumb_width,
            height,
        }
    }

    fn contains_touch(self, x: f32, y: f32) -> bool {
        let touch_width = self.thumb_width * SCROLL_THUMB_TOUCH_MULTIPLIER;
        Rect {
            x: self.track.x - (touch_width - self.track.width) / 2.0,
            width: touch_width,
            ..self.track
        }
        .contains(x, y)
    }

    fn scroll_offset_for_thumb_top(self, thumb_top: f32, scroll_max: f32) -> f32 {
        let thumb = self.thumb_rect(0.0, scroll_max);
        let travel = self.track.height - thumb.height;
        ((thumb_top - self.track.y) / travel).clamp(0.0, 1.0) * scroll_max
    }

    fn scroll_offset_for_track_tap(self, y: f32, scroll_max: f32) -> f32 {
        let thumb = self.thumb_rect(0.0, scroll_max);
        self.scroll_offset_for_thumb_top(y - thumb.height / 2.0, scroll_max)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPortal {
    pub controller: ControllerId,
    pub kind: CameraPreviewKind,
    pub rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapPortal {
    pub controller: ControllerId,
    pub rect: Rect,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub light: bool,
    pub revision: u64,
    pub image_revision: u64,
    pub width: u32,
    pub height: u32,
    pub quads: Vec<Quad>,
    pub text: Vec<TextRun>,
    pub masks: Vec<MaskRun>,
    pub images: Vec<ImageRun>,
    pub camera_portal: Option<CameraPortal>,
    pub map_portal: Option<MapPortal>,
    pub scroll_origin: f32,
    pub scroll_offset: f32,
    pub scroll_max: f32,
    pub scroll_clip: Option<Rect>,
    pub scroll_bar: Option<ScrollBar>,
    pub text_cursor: Option<Quad>,
}

impl Scene {
    pub fn colour(&self, colour: Colour) -> Colour {
        if self.light {
            invert_colour(colour)
        } else {
            colour
        }
    }
}

#[derive(Clone, Debug)]
struct HitRegion {
    rect: Rect,
    action: Action,
    long_action: Option<Action>,
    scrolling: bool,
    preserve_input: bool,
}

#[derive(Clone, Copy, Debug)]
struct TextInputLayout {
    state: StateId,
    action: TextInputAction,
    text_run: usize,
    hit_rect: Rect,
    rect: Rect,
    text_rect: Rect,
    scroll_offset: f32,
    scroll_max: f32,
    scrolling: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerOutcome {
    pub changed: bool,
    pub activated: bool,
    pub captured: bool,
}

impl PointerOutcome {
    const fn changed(changed: bool) -> Self {
        Self {
            changed,
            activated: false,
            captured: false,
        }
    }

    const fn activated(changed: bool) -> Self {
        Self {
            changed,
            activated: changed,
            captured: false,
        }
    }

    const fn captured(mut self) -> Self {
        self.captured = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EdgeBackState {
    Pending,
    Claimed,
}

#[derive(Clone, Copy, Debug)]
struct EdgeBackPointer {
    start_x: f32,
    start_y: f32,
    start_offset: f32,
    state: EdgeBackState,
}

#[derive(Clone, Copy, Debug)]
struct ContentPointer {
    start_x: f32,
    start_y: f32,
    start_offset: f32,
    dragging: bool,
    cancelled: bool,
}

#[derive(Clone, Copy, Debug)]
struct TextInputPointer {
    input: TextInputLayout,
    start_x: f32,
    start_y: f32,
    content_offset: f32,
    dragging: bool,
}

#[derive(Clone, Copy, Debug)]
struct ImagePanPointer {
    image: NodeIdentity,
    start_x: f32,
    start_y: f32,
    translation_x: f32,
    translation_y: f32,
}

#[derive(Clone, Copy, Debug)]
enum Pointer {
    Content(ContentPointer),
    TextInput(TextInputPointer),
    ImagePan(ImagePanPointer),
    ScrollThumb {
        grab_offset: f32,
    },
    ScrollTrack {
        start_x: f32,
        start_y: f32,
        cancelled: bool,
    },
    EdgeBack(EdgeBackPointer),
}

#[derive(Clone, Copy, Debug)]
struct ImageZoomState {
    viewport: Rect,
    content: Rect,
    scrolling: bool,
    transform: ImageTransform,
}

#[derive(Clone, Copy, Debug)]
struct ImagePinch {
    image: NodeIdentity,
    focus_x: f32,
    focus_y: f32,
}

#[derive(Clone)]
struct PendingRequest {
    request: NativeRequest,
    owner: RequestOwner,
}

#[derive(Clone)]
enum RequestOwner {
    Action,
    Image(RemoteImageKey),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct RemoteImageKey {
    module: String,
    url: String,
    width: u32,
    height: u32,
    fit: ImageFit,
}

#[derive(Clone)]
enum RemoteImageState {
    Loading { request_id: u64 },
    Ready(RemoteImage),
    Failed,
}

enum QueuedRequest {
    Start(PendingRequest),
    Cancel(NativeRequest),
}

#[cfg(feature = "perf")]
#[derive(Clone, Copy, Debug, Default)]
pub struct CorePerfMetrics {
    pub measure_ns: u64,
    pub relayout_ns: u64,
    pub nodes_measured: u32,
    pub full_rebuilds: u32,
    pub incremental_rebuilds: u32,
}

pub struct Engine {
    root: Node,
    state: Vec<StateValue>,
    viewport: Viewport,
    keyboard_inset: u32,
    scene: Scene,
    react_list_positions: HashMap<usize, f32>,
    list_metrics: HashMap<usize, list::ListMetrics>,
    hit_regions: Vec<HitRegion>,
    text_inputs: Vec<TextInputLayout>,
    text_input_scroll_offsets: HashMap<StateId, f32>,
    clip: Rect,
    scrolling: bool,
    scroll_origin: f32,
    scroll_offset: f32,
    scroll_max: f32,
    pointer: Option<Pointer>,
    focused_input: Option<StateId>,
    focused_input_action: TextInputAction,
    focused_input_cursor: usize,
    auto_focus_node: Option<NodeIdentity>,
    queued_requests: VecDeque<QueuedRequest>,
    in_flight_requests: HashMap<u64, PendingRequest>,
    remote_images: HashMap<RemoteImageKey, RemoteImageState>,
    visible_images: BTreeSet<RemoteImageKey>,
    image_zooms: HashMap<NodeIdentity, ImageZoomState>,
    visible_zoom_images: BTreeSet<NodeIdentity>,
    image_pinch: Option<ImagePinch>,
    last_native_request: Option<NativeRequest>,
    next_request_id: u64,
    next_image_generation: u64,
    back_icon: Option<Mask>,
    navigation_handler: Option<(Mask, NativeOperation)>,
    font: FontRef<'static>,
    #[cfg(feature = "perf")]
    perf: CorePerfMetrics,
    #[cfg(feature = "perf")]
    measure_depth: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Viewport {
    width: u32,
    height: u32,
    scale: f32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            root: Node::screen(vec![], None, false),
            state: Vec::new(),
            viewport: Viewport::default(),
            keyboard_inset: 0,
            scene: Scene::default(),
            react_list_positions: HashMap::new(),
            list_metrics: HashMap::new(),
            hit_regions: Vec::new(),
            text_inputs: Vec::new(),
            text_input_scroll_offsets: HashMap::new(),
            clip: Rect::default(),
            scrolling: false,
            scroll_origin: 0.0,
            scroll_offset: 0.0,
            scroll_max: 0.0,
            pointer: None,
            focused_input: None,
            focused_input_action: TextInputAction::default(),
            focused_input_cursor: 0,
            auto_focus_node: None,
            queued_requests: VecDeque::new(),
            in_flight_requests: HashMap::new(),
            remote_images: HashMap::new(),
            visible_images: BTreeSet::new(),
            image_zooms: HashMap::new(),
            visible_zoom_images: BTreeSet::new(),
            image_pinch: None,
            last_native_request: None,
            next_request_id: 1,
            next_image_generation: 1,
            back_icon: None,
            navigation_handler: None,
            font: FontRef::try_from_slice(PUBLIC_SANS).expect("bundled Public Sans is valid"),
            #[cfg(feature = "perf")]
            perf: CorePerfMetrics::default(),
            #[cfg(feature = "perf")]
            measure_depth: 0,
        }
    }

    pub fn take_native_request(&mut self) -> Option<NativeRequest> {
        let request = match self.queued_requests.pop_front()? {
            QueuedRequest::Start(pending) => {
                let request = pending.request.clone();
                self.in_flight_requests.insert(request.id, pending);
                request
            }
            QueuedRequest::Cancel(request) => request,
        };
        self.last_native_request = Some(request.clone());
        Some(request)
    }

    #[cfg(feature = "perf")]
    pub fn take_perf_metrics(&mut self) -> CorePerfMetrics {
        std::mem::take(&mut self.perf)
    }

    pub fn native_request(&self, id: u64) -> Option<&NativeRequest> {
        self.last_native_request
            .as_ref()
            .filter(|request| request.id == id)
    }

    pub fn image_request_target(&self, request_id: u64) -> Option<(u32, u32, ImageFit)> {
        let pending = self.in_flight_requests.get(&request_id)?;
        let RequestOwner::Image(key) = &pending.owner else {
            return None;
        };
        Some((key.width, key.height, key.fit))
    }

    pub fn complete_native_image(
        &mut self,
        request_id: u64,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> bool {
        let Some(pending) = self.in_flight_requests.remove(&request_id) else {
            return false;
        };
        let RequestOwner::Image(key) = pending.owner else {
            return false;
        };
        if !matches!(
            self.remote_images.get(&key),
            Some(RemoteImageState::Loading { request_id: active }) if *active == request_id
        ) || width == 0
            || height == 0
            || pixels.len() != width as usize * height as usize * 4
        {
            return false;
        }
        let id = image_id(&key);
        let generation = self.next_image_generation;
        self.next_image_generation = self.next_image_generation.wrapping_add(1).max(1);
        self.remote_images.insert(
            key,
            RemoteImageState::Ready(RemoteImage {
                id,
                generation,
                width,
                height,
                pixels: pixels.into(),
            }),
        );
        self.rebuild_scene();
        true
    }

    pub fn fail_native(&mut self, request_id: u64, _error: ResourceError) -> bool {
        let Some(owner) = self
            .in_flight_requests
            .get(&request_id)
            .map(|pending| pending.owner.clone())
        else {
            return false;
        };
        match owner {
            RequestOwner::Image(key) => {
                self.in_flight_requests.remove(&request_id);
                self.remote_images.insert(key, RemoteImageState::Failed);
                self.rebuild_scene();
                true
            }
            RequestOwner::Action => self.complete_native_action(request_id),
        }
    }

    pub fn complete_native_action(&mut self, request_id: u64) -> bool {
        let Some(pending) = self.in_flight_requests.remove(&request_id) else {
            return false;
        };
        matches!(pending.owner, RequestOwner::Action)
    }

    fn queue_native_action(&mut self, operation: NativeOperation) -> bool {
        let payload = operation.payload.clone();
        let request = NativeRequest {
            id: self.next_request_id(),
            kind: NativeRequestKind::Action,
            operation: Some(operation),
            payload,
            controller: None,
        };
        self.queued_requests
            .push_back(QueuedRequest::Start(PendingRequest {
                request,
                owner: RequestOwner::Action,
            }));
        true
    }

    fn next_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        id
    }

    fn cancel_request(&mut self, request_id: u64) {
        let queued = self.queued_requests.iter().position(|request| {
            matches!(request, QueuedRequest::Start(pending) if pending.request.id == request_id)
        });
        if let Some(index) = queued {
            self.queued_requests.remove(index);
            return;
        }
        if self.in_flight_requests.remove(&request_id).is_some() {
            self.queued_requests
                .push_back(QueuedRequest::Cancel(NativeRequest {
                    id: request_id,
                    kind: NativeRequestKind::Cancel,
                    operation: None,
                    payload: String::new(),
                    controller: None,
                }));
        }
    }

    fn queue_remote_image(&mut self, key: RemoteImageKey) {
        if self.remote_images.contains_key(&key) {
            return;
        }
        let request_id = self.next_request_id();
        let payload = if key.module == "network" {
            format!(
                "{{\"url\":{},\"headers\":{{}}}}",
                serde_json::to_string(&key.url).expect("a Rust string is valid JSON"),
            )
        } else if key.module == "barcode" {
            let source: serde_json::Value = serde_json::from_str(&key.url).unwrap_or_default();
            let size = source["size"].as_f64().unwrap_or_default();
            serde_json::json!({
                "source": key.url,
                "pixelSize": (size * f64::from(self.viewport.scale)).round() as u32,
            })
            .to_string()
        } else {
            format!(
                "{{\"source\":{}}}",
                serde_json::to_string(&key.url).expect("a Rust string is valid JSON"),
            )
        };
        let request = NativeRequest {
            id: request_id,
            kind: NativeRequestKind::Image,
            operation: Some(NativeOperation::new(
                key.module.clone(),
                "image",
                "",
                20_000,
            )),
            payload,
            controller: None,
        };
        self.remote_images
            .insert(key.clone(), RemoteImageState::Loading { request_id });
        self.queued_requests
            .push_back(QueuedRequest::Start(PendingRequest {
                request,
                owner: RequestOwner::Image(key),
            }));
    }

    fn sync_visible_images(&mut self) {
        let stale = self
            .remote_images
            .iter()
            .filter_map(|(key, state)| {
                (!self.visible_images.contains(key)).then(|| match state {
                    RemoteImageState::Loading { request_id } => Some((key.clone(), *request_id)),
                    RemoteImageState::Ready(_) | RemoteImageState::Failed => Some((key.clone(), 0)),
                })?
            })
            .collect::<Vec<_>>();
        for (key, request_id) in stale {
            self.remote_images.remove(&key);
            if request_id != 0 {
                self.cancel_request(request_id);
            }
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
        self.rebuild_viewport();
        true
    }

    pub fn set_keyboard_inset(&mut self, inset: u32) -> bool {
        if self.keyboard_inset == inset {
            return false;
        }
        self.keyboard_inset = inset;
        self.rebuild_viewport();
        true
    }

    fn rebuild_viewport(&mut self) {
        let focused_rect = self
            .text_inputs
            .iter()
            .find(|input| input.scrolling && Some(input.state) == self.focused_input)
            .map(|input| Rect {
                y: input.rect.y + self.scroll_origin,
                ..input.rect
            });
        self.rebuild_scene();
        let focused_rect = self.text_inputs.iter().find(|input| {
            input.scrolling && Some(input.state) == self.focused_input
        }).map(|input| Rect {
            y: input.rect.y + self.scroll_origin,
            ..input.rect
        }).or(focused_rect);
        if let (Some(input), Some(clip)) = (focused_rect, self.scene.scroll_clip) {
            let bottom = input.y + input.height - self.scroll_offset;
            let top = input.y - self.scroll_offset;
            let offset = if bottom > clip.y + clip.height {
                self.scroll_offset + bottom - clip.y - clip.height
            } else if top < clip.y {
                self.scroll_offset + top - clip.y
            } else {
                self.scroll_offset
            };
            self.set_scroll_offset(offset.clamp(0.0, self.scroll_max));
        }
    }

    pub fn tap(&mut self, x: f32, y: f32) -> bool {
        let action = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| {
                if region.scrolling
                    && !self
                        .scene
                        .scroll_clip
                        .is_some_and(|clip| clip.contains(x, y))
                {
                    return false;
                }
                let y = if region.scrolling {
                    y + self.scroll_offset - self.scroll_origin
                } else {
                    y
                };
                region.rect.contains(x, y)
            })
            .map(|region| (match region.action.clone() {
                Action::Seek { id, left, width, duration } => react::event(id, "onSeek", vec![serde_json::json!(((x - left) / width).clamp(0.0, 1.0) * duration)]),
                action => action,
            }, region.preserve_input));
        let preserve_input = action.as_ref().is_some_and(|(_, preserve)| *preserve);
        let action = action.map(|(action, _)| action);
        let blurred = self.focused_input.is_some()
            && !preserve_input
            && !matches!(
                action.as_ref(),
                Some(Action::FocusTextInput { .. } | Action::ClearInput { .. } | Action::Back)
            );
        if blurred {
            self.focused_input = None;
        }

        let changed = action.is_some_and(|action| self.apply(action));
        if !changed && !blurred {
            return false;
        }

        self.relayout_scene();
        true
    }

    pub fn pointer_down(&mut self, x: f32, y: f32) -> PointerOutcome {
        if let Some(scroll_bar) = self
            .scene
            .scroll_bar
            .filter(|scroll_bar| scroll_bar.contains_touch(x, y))
        {
            let thumb = scroll_bar.thumb_rect(self.scroll_offset, self.scroll_max);
            self.pointer = Some(if y >= thumb.y && y <= thumb.y + thumb.height {
                Pointer::ScrollThumb {
                    grab_offset: y - thumb.y,
                }
            } else {
                Pointer::ScrollTrack {
                    start_x: x,
                    start_y: y,
                    cancelled: false,
                }
            });
            return PointerOutcome::default().captured();
        }

        if self.navigation_handler.is_some() && x <= self.scaled(BACK_SWIPE_EDGE_WIDTH) {
            self.pointer = Some(Pointer::EdgeBack(EdgeBackPointer {
                start_x: x,
                start_y: y,
                start_offset: self.scroll_offset,
                state: EdgeBackState::Pending,
            }));
            return PointerOutcome::default();
        }

        if let Some(image) = self.zoomable_image_at(x, y)
            && let Some(zoom) = self.image_zooms.get(&image).copied()
            && zoom.transform.scale > 1.0
        {
            self.pointer = Some(Pointer::ImagePan(ImagePanPointer {
                image,
                start_x: x,
                start_y: y,
                translation_x: zoom.transform.translation_x,
                translation_y: zoom.transform.translation_y,
            }));
            return PointerOutcome::default().captured();
        }

        if let Some(input) = self.text_input_at(x, y) {
            self.pointer = Some(Pointer::TextInput(TextInputPointer {
                input,
                start_x: x,
                start_y: y,
                content_offset: self.scroll_offset,
                dragging: false,
            }));
            return PointerOutcome::default();
        }

        self.pointer = Some(Pointer::Content(ContentPointer {
            start_x: x,
            start_y: y,
            start_offset: self.scroll_offset,
            dragging: false,
            cancelled: false,
        }));
        PointerOutcome::default()
    }

    pub fn pointer_move(&mut self, x: f32, y: f32) -> PointerOutcome {
        let tap_slop = self.scaled(TAP_SLOP);
        let Some(pointer) = self.pointer else {
            return PointerOutcome::default();
        };

        match pointer {
            Pointer::Content(pointer) => self.move_content_pointer(pointer, x, y, tap_slop),
            Pointer::TextInput(pointer) => self.move_text_input_pointer(pointer, x, y, tap_slop),
            Pointer::ImagePan(pointer) => {
                let Some(zoom) = self.image_zooms.get(&pointer.image).copied() else {
                    return PointerOutcome::default();
                };
                let transform = ImageTransform {
                    translation_x: pointer.translation_x + x - pointer.start_x,
                    translation_y: pointer.translation_y + y - pointer.start_y,
                    ..zoom.transform
                };
                PointerOutcome::changed(self.set_image_transform(pointer.image, transform))
                    .captured()
            }
            Pointer::ScrollThumb { grab_offset } => {
                let Some(scroll_bar) = self.scene.scroll_bar else {
                    return PointerOutcome::default();
                };
                let next = scroll_bar.scroll_offset_for_thumb_top(y - grab_offset, self.scroll_max);
                PointerOutcome::changed(self.set_scroll_offset(next)).captured()
            }
            Pointer::ScrollTrack {
                start_x,
                start_y,
                cancelled,
            } => {
                self.pointer = Some(Pointer::ScrollTrack {
                    start_x,
                    start_y,
                    cancelled: cancelled
                        || (x - start_x).abs() > tap_slop
                        || (y - start_y).abs() > tap_slop,
                });
                PointerOutcome::default().captured()
            }
            Pointer::EdgeBack(pointer) => self.move_edge_back_pointer(pointer, x, y, tap_slop),
        }
    }

    pub fn pointer_up(&mut self, x: f32, y: f32) -> PointerOutcome {
        let Some(pointer) = self.pointer.take() else {
            return PointerOutcome::default();
        };

        match pointer {
            Pointer::Content(ContentPointer {
                dragging: false,
                cancelled: false,
                ..
            }) => PointerOutcome::activated(self.tap(x, y)),
            Pointer::ScrollTrack { cancelled, .. } if !cancelled => {
                let Some(scroll_bar) = self
                    .scene
                    .scroll_bar
                    .filter(|scroll_bar| scroll_bar.contains_touch(x, y))
                else {
                    return PointerOutcome::default();
                };
                let next = scroll_bar.scroll_offset_for_track_tap(y, self.scroll_max);
                PointerOutcome::activated(self.set_scroll_offset(next)).captured()
            }
            Pointer::EdgeBack(EdgeBackPointer {
                state: EdgeBackState::Pending,
                ..
            }) => PointerOutcome::activated(self.tap(x, y)),
            Pointer::TextInput(pointer) if !pointer.dragging => {
                PointerOutcome::activated(self.focus_text_input(pointer.input, x, y))
            }
            Pointer::ScrollThumb { .. }
            | Pointer::ScrollTrack { .. }
            | Pointer::TextInput(_)
            | Pointer::ImagePan(_)
            | Pointer::EdgeBack(EdgeBackPointer {
                state: EdgeBackState::Claimed,
                ..
            }) => PointerOutcome::default().captured(),
            Pointer::Content(_) => PointerOutcome::default(),
        }
    }

    pub fn pointer_long_press(&mut self, x: f32, y: f32) -> PointerOutcome {
        if !matches!(self.pointer, Some(Pointer::Content(ContentPointer { dragging: false, cancelled: false, .. }))) {
            return PointerOutcome::default();
        }
        let action = self.hit_regions.iter().rev().find(|region| {
            let local_y = if region.scrolling { y + self.scroll_offset - self.scroll_origin } else { y };
            region.rect.contains(x, local_y)
        }).and_then(|region| region.long_action.clone());
        let Some(action) = action else { return PointerOutcome::default(); };
        self.pointer_cancel();
        let changed = self.apply(action);
        self.relayout_scene();
        PointerOutcome::activated(changed).captured()
    }

    pub fn pointer_cancel(&mut self) {
        self.pointer = None;
    }

    pub fn image_pinch_begin(&mut self, x: f32, y: f32) -> bool {
        let Some(image) = self.zoomable_image_at(x, y) else {
            return false;
        };
        self.pointer = None;
        self.image_pinch = Some(ImagePinch {
            image,
            focus_x: x,
            focus_y: y,
        });
        true
    }

    pub fn image_zoom_target(&self, x: f32, y: f32) -> Option<u64> {
        self.zoomable_image_at(x, y)
            .map(|identity| identity.0 as u64 + 1)
    }

    pub fn image_pinch_update(&mut self, scale_factor: f32, x: f32, y: f32) -> bool {
        if !scale_factor.is_finite() || scale_factor <= 0.0 {
            return false;
        }
        let Some(mut pinch) = self.image_pinch else {
            return false;
        };
        let Some(zoom) = self.image_zooms.get(&pinch.image).copied() else {
            self.image_pinch = None;
            return false;
        };
        let scroll = if zoom.scrolling {
            self.scroll_offset - self.scroll_origin
        } else {
            0.0
        };
        let focus_x = x;
        let focus_y = y + scroll;
        let previous_x = pinch.focus_x;
        let previous_y = pinch.focus_y + scroll;
        let mut transform = zoom.transform;
        transform.translation_x += focus_x - previous_x;
        transform.translation_y += focus_y - previous_y;
        let scale = (transform.scale * scale_factor).clamp(1.0, IMAGE_MAX_SCALE);
        let factor = scale / transform.scale;
        transform.translation_x = focus_x - (focus_x - transform.translation_x) * factor;
        transform.translation_y = focus_y - (focus_y - transform.translation_y) * factor;
        transform.scale = scale;
        pinch.focus_x = x;
        pinch.focus_y = y;
        self.image_pinch = Some(pinch);
        self.set_image_transform(pinch.image, transform)
    }

    pub fn image_pinch_end(&mut self) {
        self.image_pinch = None;
    }

    pub fn image_double_tap(&mut self, x: f32, y: f32) -> PointerOutcome {
        let Some(image) = self.zoomable_image_at(x, y) else {
            return PointerOutcome::default();
        };
        let Some(zoom) = self.image_zooms.get(&image).copied() else {
            return PointerOutcome::default();
        };
        self.pointer = None;
        self.image_pinch = None;
        let next_scale = if zoom.transform.scale >= IMAGE_MAX_SCALE {
            1.0
        } else {
            (zoom.transform.scale.floor() + 1.0).clamp(2.0, IMAGE_MAX_SCALE)
        };
        let target = if next_scale == 1.0 {
            ImageTransform::default()
        } else {
            let scroll = if zoom.scrolling {
                self.scroll_offset - self.scroll_origin
            } else {
                0.0
            };
            constrain_image_transform(
                zoom,
                ImageTransform {
                    scale: next_scale,
                    translation_x: x
                        - (x - zoom.transform.translation_x) * next_scale / zoom.transform.scale,
                    translation_y: y + scroll
                        - (y + scroll - zoom.transform.translation_y) * next_scale
                            / zoom.transform.scale,
                },
            )
        };
        PointerOutcome::changed(self.set_image_transform(image, target)).captured()
    }

    fn zoomable_image_at(&self, x: f32, y: f32) -> Option<NodeIdentity> {
        self.scene.images.iter().rev().find_map(|run| {
            let identity = NodeIdentity(run.zoom_id?);
            if run.scrolling
                && !self
                    .scene
                    .scroll_clip
                    .is_some_and(|clip| clip.contains(x, y))
            {
                return None;
            }
            let scroll = if run.scrolling {
                self.scroll_offset - self.scroll_origin
            } else {
                0.0
            };
            Rect {
                y: run.rect.y - scroll,
                ..run.rect
            }
            .contains(x, y)
            .then_some(identity)
        })
    }

    fn set_image_transform(&mut self, image: NodeIdentity, transform: ImageTransform) -> bool {
        let Some(zoom) = self.image_zooms.get(&image).copied() else {
            return false;
        };
        let transform = constrain_image_transform(zoom, transform);
        if transform == zoom.transform {
            return false;
        }
        self.image_zooms
            .get_mut(&image)
            .expect("visible zoom image exists")
            .transform = transform;
        for run in &mut self.scene.images {
            if run.zoom_id == Some(image.0) {
                run.transform = transform;
            }
        }
        self.scene.image_revision = self.scene.image_revision.wrapping_add(1);
        true
    }

    fn text_input_at(&self, x: f32, y: f32) -> Option<TextInputLayout> {
        self.text_inputs.iter().rev().copied().find(|input| {
            if input.scrolling
                && !self
                    .scene
                    .scroll_clip
                    .is_some_and(|clip| clip.contains(x, y))
            {
                return false;
            }
            let y = if input.scrolling {
                y + self.scroll_offset - self.scroll_origin
            } else {
                y
            };
            input.hit_rect.contains(x, y)
        })
    }

    fn focus_text_input(&mut self, input: TextInputLayout, x: f32, y: f32) -> bool {
        let Some(StateValue::String(value)) = self.state.get(input.state.0) else {
            return false;
        };
        let font_size = self.scaled_font(TEXT_INPUT_TEXT_SIZE);
        let cursor = if input.action == TextInputAction::Return {
            let lines = self.input_lines(value, input.text_rect.width);
            let y = if input.scrolling { y + self.scroll_offset - self.scroll_origin } else { y };
            let line = ((y - input.text_rect.y + input.scroll_offset) / self.scaled(TEXT_INPUT_HEIGHT - TEXT_INPUT_BOTTOM_PADDING)).max(0.0) as usize;
            let (start, end) = lines[line.min(lines.len() - 1)];
            start + self.text_cursor_for_offset(&value[start..end], font_size, (x - input.text_rect.x).max(0.0))
        } else {
            let target = (x - input.text_rect.x + input.scroll_offset)
                .clamp(0.0, self.text_width(value, font_size));
            self.text_cursor_for_offset(value, font_size, target)
        };
        let changed = self.focused_input != Some(input.state)
            || self.focused_input_action != input.action
            || self.focused_input_cursor != cursor;
        if !changed {
            return false;
        }
        self.focused_input = Some(input.state);
        self.focused_input_action = input.action;
        self.focused_input_cursor = cursor;
        self.text_input_scroll_offsets
            .insert(input.state, input.scroll_offset);
        self.relayout_scene();
        true
    }

    fn focus_text_input_at_end(&mut self, state: StateId, action: TextInputAction) {
        self.focused_input = Some(state);
        self.focused_input_action = action;
        self.focused_input_cursor = match self.state.get(state.0) {
            Some(StateValue::String(value)) => value.len(),
            _ => 0,
        };
        self.text_input_scroll_offsets.remove(&state);
    }

    fn move_text_input_pointer(
        &mut self,
        mut pointer: TextInputPointer,
        x: f32,
        y: f32,
        tap_slop: f32,
    ) -> PointerOutcome {
        let horizontal = pointer.start_x - x;
        let vertical = pointer.start_y - y;
        if pointer.input.action == TextInputAction::Return {
            if pointer.input.scroll_max == 0.0 { return PointerOutcome::default(); }
            if !pointer.dragging {
                if vertical.abs() <= tap_slop { return PointerOutcome::default(); }
                pointer.start_y -= vertical.signum() * tap_slop;
                pointer.dragging = true;
            }
            self.pointer = Some(Pointer::TextInput(pointer));
            let next = (pointer.input.scroll_offset + pointer.start_y - y)
                .clamp(0.0, pointer.input.scroll_max);
            return PointerOutcome::changed(self.set_text_input_scroll(pointer.input.state, next)).captured();
        }
        if !pointer.dragging {
            if horizontal.abs() <= tap_slop && vertical.abs() <= tap_slop {
                self.pointer = Some(Pointer::TextInput(pointer));
                return PointerOutcome::default();
            }
            if vertical.abs() > horizontal.abs() {
                return self.move_content_pointer(
                    ContentPointer {
                        start_x: pointer.start_x,
                        start_y: pointer.start_y,
                        start_offset: pointer.content_offset,
                        dragging: false,
                        cancelled: false,
                    },
                    x,
                    y,
                    tap_slop,
                );
            }
            pointer.start_x -= horizontal.signum() * tap_slop;
            pointer.dragging = true;
        }
        self.pointer = Some(Pointer::TextInput(pointer));
        let next = (pointer.input.scroll_offset + pointer.start_x - x)
            .clamp(0.0, pointer.input.scroll_max);
        PointerOutcome::changed(self.set_text_input_scroll(pointer.input.state, next)).captured()
    }

    fn set_text_input_scroll(&mut self, state: StateId, offset: f32) -> bool {
        let current = self
            .text_input_scroll_offsets
            .get(&state)
            .copied()
            .or_else(|| {
                self.text_inputs
                    .iter()
                    .find(|input| input.state == state)
                    .map(|input| input.scroll_offset)
            })
            .expect("visible text input has a scroll offset");
        if (current - offset).abs() < f32::EPSILON {
            return false;
        }
        self.text_input_scroll_offsets.insert(state, offset);
        if self.text_inputs.iter().any(|input| input.state == state && input.action == TextInputAction::Return) {
            self.relayout_scene();
            return true;
        }
        let delta = offset - current;
        for input in self
            .text_inputs
            .iter_mut()
            .filter(|input| input.state == state)
        {
            input.scroll_offset = offset;
            self.scene.text[input.text_run].rect.x -= delta;
        }
        if self.focused_input == Some(state)
            && let Some(cursor) = &mut self.scene.text_cursor
        {
            cursor.rect.x -= delta;
        }
        self.scene.revision = self.scene.revision.wrapping_add(1);
        true
    }

    fn reveal_text_cursor(&mut self, state: StateId) {
        let Some(input) = self
            .text_inputs
            .iter()
            .rev()
            .find(|input| input.state == state)
            .copied()
        else {
            return;
        };
        let Some(StateValue::String(value)) = self.state.get(state.0) else {
            return;
        };
        let cursor = self.focused_input_cursor;
        if input.action == TextInputAction::Return {
            let lines = self.input_lines(value, input.text_rect.width);
            let line_height = self.scaled(TEXT_INPUT_HEIGHT - TEXT_INPUT_BOTTOM_PADDING);
            let line = lines.iter().rposition(|(start, _)| *start <= cursor).unwrap_or(0);
            let height = lines.len().min(TEXT_INPUT_MAX_LINES) as f32 * line_height;
            let current = self.text_input_scroll_offsets.get(&state).copied().unwrap_or(input.scroll_offset);
            let top = line as f32 * line_height;
            let next = if top < current { top }
                else if top + line_height > current + height { top + line_height - height }
                else { current };
            self.text_input_scroll_offsets.insert(state, next.max(0.0));
            return;
        }
        let font_size = self.scaled_font(TEXT_INPUT_TEXT_SIZE);
        let cursor_offset = self.text_width(&value[..cursor], font_size);
        let scroll_max = (self.text_width(value, font_size) - input.text_rect.width).max(0.0);
        let current = self
            .text_input_scroll_offsets
            .get(&state)
            .copied()
            .unwrap_or(input.scroll_offset)
            .clamp(0.0, scroll_max);
        let next = if cursor_offset < current {
            cursor_offset
        } else if cursor_offset > current + input.text_rect.width {
            cursor_offset - input.text_rect.width
        } else {
            current
        };
        self.text_input_scroll_offsets
            .insert(state, next.clamp(0.0, scroll_max));
    }

    fn move_content_pointer(
        &mut self,
        mut pointer: ContentPointer,
        x: f32,
        y: f32,
        tap_slop: f32,
    ) -> PointerOutcome {
        let delta = pointer.start_y - y;
        if !pointer.dragging {
            if delta.abs() <= tap_slop {
                pointer.cancelled |= (x - pointer.start_x).abs() > tap_slop;
                self.pointer = Some(Pointer::Content(pointer));
                return PointerOutcome::default();
            }
            pointer.start_y -= delta.signum() * tap_slop;
            pointer.dragging = true;
            pointer.cancelled = true;
        }
        self.pointer = Some(Pointer::Content(pointer));
        let next = (pointer.start_offset + pointer.start_y - y).clamp(0.0, self.scroll_max);
        PointerOutcome::changed(self.set_scroll_offset(next))
    }

    fn move_edge_back_pointer(
        &mut self,
        pointer: EdgeBackPointer,
        x: f32,
        y: f32,
        tap_slop: f32,
    ) -> PointerOutcome {
        let horizontal = x - pointer.start_x;
        let horizontal_distance = horizontal.abs();
        let vertical_distance = (y - pointer.start_y).abs();

        if pointer.state == EdgeBackState::Pending
            && vertical_distance > tap_slop
            && vertical_distance > horizontal_distance * BACK_SWIPE_VERTICAL_RATIO
        {
            let content_pointer = ContentPointer {
                start_x: pointer.start_x,
                start_y: pointer.start_y,
                start_offset: pointer.start_offset,
                dragging: false,
                cancelled: false,
            };
            return self.move_content_pointer(content_pointer, x, y, tap_slop);
        }

        let claimed = pointer.state == EdgeBackState::Claimed
            || (horizontal_distance > self.scaled(BACK_SWIPE_ACTIVATION_DISTANCE)
                && vertical_distance <= horizontal_distance * BACK_SWIPE_VERTICAL_RATIO);
        if !claimed {
            return PointerOutcome::default();
        }

        if horizontal > self.scaled(BACK_SWIPE_TRIGGER_DISTANCE)
            && vertical_distance <= horizontal * BACK_SWIPE_VERTICAL_RATIO
        {
            self.pointer = None;
            return PointerOutcome::activated(self.back()).captured();
        }

        self.pointer = Some(Pointer::EdgeBack(EdgeBackPointer {
            start_x: pointer.start_x,
            start_y: pointer.start_y,
            start_offset: pointer.start_offset,
            state: EdgeBackState::Claimed,
        }));
        PointerOutcome::default().captured()
    }

    pub fn scroll_by(&mut self, delta: f32) -> bool {
        self.set_scroll_offset((self.scroll_offset + delta).clamp(0.0, self.scroll_max))
    }

    pub fn back(&mut self) -> bool {
        if self.focused_input.is_some() {
            return self.edit_text(TextEdit::Dismiss);
        }
        if !self.pop_route() {
            return false;
        }
        self.rebuild_scene();
        true
    }

    pub const fn text_input_active(&self) -> bool {
        self.focused_input.is_some()
    }

    pub const fn text_input_action(&self) -> TextInputAction {
        self.focused_input_action
    }

    pub fn edit_text(&mut self, edit: TextEdit) -> bool {
        let Some(state) = self.focused_input else {
            return false;
        };
        let mut mutated = false;
        let changed = match edit {
            TextEdit::Insert(text) if !text.chars().any(|c| c.is_control() && !(c == '\n' && self.focused_input_action == TextInputAction::Return)) => {
                let cursor = self.focused_input_cursor;
                let Some(StateValue::String(value)) = self.state.get_mut(state.0) else {
                    return false;
                };
                let cursor = text_cursor_boundary(value, cursor);
                value.insert_str(cursor, &text);
                self.focused_input_cursor = next_text_cursor_boundary(value, cursor + text.len());
                mutated = true;
                true
            }
            TextEdit::Backspace => {
                let cursor = self.focused_input_cursor;
                let Some(StateValue::String(value)) = self.state.get_mut(state.0) else {
                    return false;
                };
                let cursor = text_cursor_boundary(value, cursor);
                let Some(previous) = value[..cursor]
                    .grapheme_indices(true)
                    .next_back()
                    .map(|(index, _)| index)
                else {
                    return false;
                };
                value.replace_range(previous..cursor, "");
                self.focused_input_cursor = previous;
                mutated = true;
                true
            }
            TextEdit::Submit | TextEdit::Dismiss => {
                self.focused_input = None;
                true
            }
            TextEdit::Insert(_) => false,
        };
        if changed {
            if mutated {
                self.reveal_text_cursor(state);
            }
            self.relayout_scene();
        }
        changed
    }

    pub fn set_colour_scheme(&mut self, light: bool) -> bool {
        if self.scene.light == light {
            return false;
        }
        self.scene.light = light;
        self.relayout_scene();
        true
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn list_viewports_ready(&self) -> bool {
        let Some(clip) = self.scene.scroll_clip else { return true; };
        self.react_list_positions.iter().all(|(id, top)| {
            let Some(metrics) = self.list_metrics.get(id) else { return true; };
            let start = (self.scroll_offset + clip.y - top).max(0.0);
            let end = (self.scroll_offset + clip.y + clip.height - top).min(metrics.total());
            end <= start || (metrics.mounted.contains(&metrics.index_at(start))
                && metrics.mounted.contains(&metrics.index_at((end - 0.5).max(start))))
        })
    }

    pub const fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub const fn scroll_max(&self) -> f32 {
        self.scroll_max
    }

    fn apply(&mut self, action: Action) -> bool {
        match action {
            Action::Seek { .. } => false,
            Action::ClearInput { state } => {
                self.state[state.0] = StateValue::String(String::new());
                if self.focused_input == Some(state) {
                    self.focused_input_cursor = 0;
                }
                self.text_input_scroll_offsets.remove(&state);
                true
            }
            Action::FocusTextInput { state, action } => {
                self.focus_text_input_at_end(state, action);
                true
            }
            Action::Native { operation } => self.queue_native_action(operation),
            Action::Back => self.back(),
        }
    }

    fn pop_route(&mut self) -> bool {
        if let Some((_, operation)) = &self.navigation_handler {
            return self.queue_native_action(operation.clone());
        }
        false
    }

    fn set_scroll_offset(&mut self, offset: f32) -> bool {
        if (offset - self.scroll_offset).abs() < f32::EPSILON {
            return false;
        }
        self.scroll_offset = offset;
        self.scene.scroll_offset = offset;
        let window = self
            .scene
            .scroll_clip
            .map_or(0.0, |clip| clip.height * 0.75);
        if (self.scroll_offset - self.scroll_origin).abs() > window {
            self.relayout_scene();
        }
        true
    }

    fn rebuild_scene(&mut self) {
        #[cfg(feature = "perf")]
        {
            self.perf.full_rebuilds += 1;
        }
        self.back_icon = self
            .navigation_handler
            .as_ref()
            .map(|(icon, _)| icon.clone());
        self.relayout_scene();
    }

    fn relayout_scene(&mut self) {
        #[cfg(feature = "perf")]
        let (started, trace) = (Instant::now(), PerfTraceSection::new(b"Ink relayout\0"));
        let previous_offset = self.scroll_offset;
        let mut anchors: Vec<_> = self.react_list_positions.iter().filter_map(|(id, top)| {
            let metrics = self.list_metrics.get(id)?;
            let clip = self.scene.scroll_clip?;
            if *top + metrics.total() < self.scroll_offset + clip.y || *top > self.scroll_offset + clip.y + clip.height {
                return None;
            }
            let index = metrics.index_at(self.scroll_offset + clip.y - top);
            Some((*id, metrics.keys.get(index)?.clone(), index,
                top + metrics.offset(index) - self.scroll_offset,
                metrics.follow_end && self.pointer.is_none()
                    && self.scroll_max - self.scroll_offset <= self.scaled(64.0)))
        }).collect();
        let clip_top = self.scene.scroll_clip.map_or(0.0, |clip| clip.y);
        anchors.sort_by(|left, right| (left.3 - clip_top).abs().total_cmp(&(right.3 - clip_top).abs()).then_with(|| left.0.cmp(&right.0)));
        self.relayout_scene_inner();
        for (id, key, old_index, position, follow) in anchors {
            let Some(top) = self.react_list_positions.get(&id) else { continue; };
            let Some(metrics) = self.list_metrics.get(&id) else { continue; };
            if metrics.keys.is_empty() { continue; }
            let index = if metrics.keys.get(old_index) == Some(&key) { old_index } else {
                metrics.keys.iter().position(|candidate| candidate == &key)
                    .unwrap_or(old_index.min(metrics.keys.len() - 1))
            };
            let next = if follow { self.scroll_max } else { top + metrics.offset(index) - position }
                .clamp(0.0, self.scroll_max);
            if (next - self.scroll_offset).abs() > 0.5 {
                self.scroll_offset = next;
                self.relayout_scene_inner();
            }
            break;
        }
        // Layout corrections must move the drag origin too, or the next motion undoes them.
        let adjustment = self.scroll_offset - previous_offset;
        match &mut self.pointer {
            Some(Pointer::Content(pointer)) => pointer.start_offset += adjustment,
            Some(Pointer::EdgeBack(pointer)) => pointer.start_offset += adjustment,
            Some(Pointer::TextInput(pointer)) => pointer.content_offset += adjustment,
            _ => {}
        }

        #[cfg(feature = "perf")]
        {
            drop(trace);
            self.perf.relayout_ns += elapsed_ns(started);
        }
    }

    fn relayout_scene_inner(&mut self) {
        self.scene.revision = self.scene.revision.wrapping_add(1);
        self.scene.width = self.viewport.width;
        self.scene.height = self.viewport.height;
        self.react_list_positions.clear();
        self.scene.quads.clear();
        self.scene.text.clear();
        self.scene.masks.clear();
        self.scene.images.clear();
        self.scene.camera_portal = None;
        self.scene.map_portal = None;
        self.scroll_origin = self.scroll_offset;
        self.scene.scroll_origin = self.scroll_origin;
        self.scene.scroll_offset = self.scroll_offset;
        self.scene.scroll_clip = None;
        self.scene.scroll_bar = None;
        self.scene.text_cursor = None;
        self.hit_regions.clear();
        self.text_inputs.clear();
        self.visible_images.clear();
        self.visible_zoom_images.clear();
        self.scroll_max = 0.0;
        self.scene.scroll_max = 0.0;
        self.scrolling = false;

        if self.viewport.width == 0 || self.viewport.height == 0 {
            return;
        }

        let root = std::mem::replace(&mut self.root, Node::screen(vec![], None, false));
        let auto_focus = self.auto_focus(&root);
        let auto_focus_node = auto_focus.map(|(node, _, _)| node);
        if auto_focus_node != self.auto_focus_node {
            self.auto_focus_node = auto_focus_node;
            self.focused_input = auto_focus.map(|(_, state, _)| state);
            if let Some((_, state, action)) = auto_focus {
                self.focus_text_input_at_end(state, action);
            }
        }
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
                height: self.viewport.height.saturating_sub(self.keyboard_inset) as f32,
            },
        );
        self.root = root;
        self.sync_visible_images();
        self.image_zooms
            .retain(|identity, _| self.visible_zoom_images.contains(identity));
        if self
            .image_pinch
            .is_some_and(|pinch| !self.visible_zoom_images.contains(&pinch.image))
        {
            self.image_pinch = None;
        }
    }

    fn auto_focus(&self, node: &Node) -> Option<(NodeIdentity, StateId, TextInputAction)> {
        match &node.kind {
            NodeKind::TextInput {
                state,
                action,
                auto_focus: true,
                ..
            } => Some((node.identity, *state, *action)),
            NodeKind::ConversationComposer { children, .. }
            | NodeKind::Message { children, .. }
            | NodeKind::MessageQuote { children, .. }
            | NodeKind::PlayingTransport { children, .. }
            | NodeKind::PlayingLayout { children, .. }
            | NodeKind::PlayingPressable { children, .. }
            | NodeKind::Screen { children, .. }
            | NodeKind::MediaGridRow { children }
            | NodeKind::Row { children, .. }
            | NodeKind::Stack { children, .. }
            | NodeKind::ReactList { children, .. } => {
                children.iter().find_map(|child| self.auto_focus(child))
            }
            NodeKind::Tabs { value, tabs } => self
                .active_tab_index(value, tabs.len())
                .and_then(|active| self.auto_focus(&tabs[active].screen)),
            NodeKind::Text { .. }
            | NodeKind::TextInput { .. }
            | NodeKind::Button { .. }
            | NodeKind::Field { .. }
            | NodeKind::Icon { .. }
            | NodeKind::Image { .. }
            | NodeKind::CameraPreview { .. }
            | NodeKind::MapView { .. }
            | NodeKind::MediaCell { .. }
            | NodeKind::PlayingProgress { .. }
            | NodeKind::Toggle { .. } => None,
        }
    }

    fn measure(&mut self, node: &Node, available: Rect) -> MeasuredSize {
        #[cfg(feature = "perf")]
        let started = (self.measure_depth == 0).then(Instant::now);
        #[cfg(feature = "perf")]
        let trace = (self.measure_depth == 0).then(|| PerfTraceSection::new(b"Ink measure\0"));
        #[cfg(feature = "perf")]
        {
            self.measure_depth += 1;
            self.perf.nodes_measured += 1;
        }
        let measured = self.measure_inner(node, available);
        #[cfg(feature = "perf")]
        {
            self.measure_depth -= 1;
            drop(trace);
            if let Some(started) = started {
                self.perf.measure_ns += elapsed_ns(started);
            }
        }
        measured
    }

    fn measure_inner(&mut self, node: &Node, available: Rect) -> MeasuredSize {
        match &node.kind {
            NodeKind::MediaGridRow { .. } => MeasuredSize { width: available.width, height: available.width / 3.0 },
            NodeKind::MediaCell { .. } => MeasuredSize { width: available.width, height: available.width },
            NodeKind::ReactList { children, start, keys, content_versions, revision, gap, follow_end } => {
                let mut metrics = self.list_metrics.remove(&node.identity.0).unwrap_or_default();
                metrics.prepare(keys, content_versions, *revision, available.width, self.scaled(*gap), self.scaled(40.0));
                metrics.mounted = *start..*start + children.len();
                metrics.follow_end = *follow_end;
                for (index, child) in children.iter().enumerate() {
                    if start + index < keys.len() {
                        let size = self.measure(child, Rect { height: f32::INFINITY, ..available });
                        metrics.measure(start + index, size.height.max(1.0));
                    }
                }
                let height = metrics.total();
                self.list_metrics.insert(node.identity.0, metrics);
                MeasuredSize { width: available.width, height }
            }
            NodeKind::Message { children, .. } => {
                let width = available.width * 0.85;
                let size = children.first().map(|child| self.measure(child, Rect { width, ..available })).unwrap_or_default();
                MeasuredSize { width: available.width, height: size.height }
            }
            NodeKind::MessageQuote { children } => {
                let inset = self.scaled(10.0);
                let size = children.first().map(|child| self.measure(child, Rect { width: (available.width - inset).max(0.0), ..available })).unwrap_or_default();
                MeasuredSize { width: (size.width + inset).min(available.width), height: size.height }
            }
            NodeKind::ConversationComposer { children } => {
                let width = (available.width - self.scaled(40.0 * (children.len() - 1) as f32)).max(0.0);
                let height = children.iter().map(|child| self.measure(child, Rect { width, ..available }).height).fold(0.0, f32::max);
                MeasuredSize { width: available.width, height }
            }
            NodeKind::PlayingPressable { children, .. } => children.first().map(|child| self.measure(child, available)).unwrap_or_default(),
            NodeKind::PlayingTransport { children } => MeasuredSize {
                width: available.width,
                height: children.iter().map(|child| self.measure(child, available).height).fold(0.0, f32::max),
            },
            NodeKind::PlayingProgress { .. } => MeasuredSize { width: available.width, height: self.scaled(6.0) },
            NodeKind::Row { children, has_image, .. } => {
                let image_width = if *has_image { self.scaled(65.0).min(available.width) } else { 0.0 };
                let text = children.last().map(|child| self.measure(child, Rect { width: (available.width - image_width).max(0.0), ..available })).unwrap_or_default();
                MeasuredSize { width: available.width, height: text.height.max(self.scaled(50.0)).min(available.height) }
            }
            NodeKind::PlayingLayout { .. } | NodeKind::Screen { .. } | NodeKind::Tabs { .. } => MeasuredSize {
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
                match axis {
                    Axis::Vertical => {
                        let measured = self.measure_vertical_children(children, available);
                        let entries = measured.len();
                        MeasuredSize {
                            width: measured
                                .iter()
                                .map(|measure| measure.width)
                                .fold(0.0, f32::max)
                                .min(available.width),
                            height: (measured.iter().map(|measure| measure.height).sum::<f32>()
                                + gap * entries.saturating_sub(1) as f32)
                                .min(available.height),
                        }
                    }
                    Axis::Horizontal => {
                        let measured: Vec<_> = children
                            .iter()
                            .map(|child| self.measure(child, available))
                            .collect();
                        MeasuredSize {
                            width: (measured.iter().map(|size| size.width).sum::<f32>()
                                + gap * children.len().saturating_sub(1) as f32)
                                .min(available.width),
                            height: measured
                                .iter()
                                .map(|size| size.height)
                                .fold(0.0, f32::max)
                                .min(available.height),
                        }
                    }
                }
            }
            NodeKind::Text {
                text,
                font_size,
                align,
                max_lines,
            } => {
                let size = font_size.unwrap_or(DEFAULT_TEXT_SIZE);
                let font_size = self.scaled_font(size);
                let lines = self.wrap_text(text, font_size, available.width, *max_lines);
                let line_height = self.text_line_height(size, lines.len());
                MeasuredSize {
                    width: if *align == TextAlign::Justify && lines.iter().any(|line| line.wrapped)
                    {
                        available.width
                    } else {
                        lines.iter().map(|line| line.width).fold(0.0, f32::max)
                    }
                    .min(available.width),
                    height: (line_height * lines.len() as f32).min(available.height),
                }
            }
            NodeKind::TextInput { state, action, .. } => MeasuredSize {
                width: available.width,
                height: if *action == TextInputAction::Return {
                    let value = match self.state.get(state.0) { Some(StateValue::String(value)) => value.as_str(), _ => "" };
                    let lines = self.input_lines(value, (available.width - self.scaled(1.0)).max(0.0)).len().min(TEXT_INPUT_MAX_LINES);
                    self.scaled((TEXT_INPUT_HEIGHT - TEXT_INPUT_BOTTOM_PADDING) * lines as f32 + TEXT_INPUT_BOTTOM_PADDING).min(available.height)
                } else { self.scaled(TEXT_INPUT_HEIGHT).min(available.height) },
            },
            NodeKind::Button { label, icon, .. } => {
                let font_size = self.scaled_font(DEFAULT_TEXT_SIZE);
                let icon_width = icon
                    .as_ref()
                    .map(|_| self.scaled(if label.is_empty() { BUTTON_HEIGHT } else { BUTTON_ICON_SIZE + BUTTON_ICON_GAP }))
                    .unwrap_or_default();
                MeasuredSize {
                    width: (self.text_width(label, font_size).ceil() + 1.0 + icon_width)
                        .min(available.width),
                    height: self.scaled(BUTTON_HEIGHT).min(available.height),
                }
            }
            NodeKind::Field { label, value, .. } => {
                let label_width = self.text_width(label, self.scaled_font(FIELD_LABEL_SIZE));
                let lines = self.wrap_text(value, self.scaled_font(DEFAULT_TEXT_SIZE), available.width, None);
                let value_width = lines.iter().map(|line| line.width).fold(0.0, f32::max);
                let value_height = self.text_line_height(DEFAULT_TEXT_SIZE, lines.len()) * lines.len() as f32;
                MeasuredSize {
                    width: label_width.max(value_width).ceil().min(available.width),
                    height: (self.scaled(FIELD_LABEL_HEIGHT) + value_height).min(available.height),
                }
            }
            NodeKind::Icon { size, bounds, .. } => {
                let size = self.scaled(if *size > 0.0 {
                    *size
                } else {
                    DEFAULT_ICON_SIZE
                });
                MeasuredSize {
                    width: (size * bounds.map_or(1.0, |bounds| bounds.width)).min(available.width),
                    height: (size * bounds.map_or(1.0, |bounds| bounds.height)).min(available.height),
                }
            }
            NodeKind::Image {
                source,
                bleed,
                width,
                height,
                fit,
                ..
            } => {
                let measured_width = if *bleed {
                    self.viewport.width as f32
                } else {
                    self.scaled(*width).min(available.width)
                };
                let code_height = match source {
                    ImageSource::Native(module, url) if module == "barcode" => {
                        let pixels = measured_width.ceil().max(1.0) as u32;
                        let key = RemoteImageKey { module: module.clone(), url: url.clone(), width: pixels, height: pixels, fit: *fit };
                        match self.remote_images.get(&key) {
                            Some(RemoteImageState::Ready(image)) => Some(measured_width * image.height as f32 / image.width as f32),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                MeasuredSize {
                    width: measured_width,
                    height: if let Some(height) = code_height {
                        height.min(available.height)
                    } else if *bleed {
                        (measured_width * height / width).min(available.height)
                    } else {
                        self.scaled(*height).min(available.height)
                    },
                }
            }
            NodeKind::CameraPreview { .. } | NodeKind::MapView { .. } => MeasuredSize {
                width: available.width,
                height: if available.height.is_finite() {
                    available.height
                } else {
                    (self.viewport.height as f32 - available.y).max(0.0)
                },
            },
            NodeKind::Toggle { .. } => MeasuredSize {
                width: available.width,
                height: self.scaled(TOGGLE_HEIGHT).min(available.height),
            },
        }
    }

    fn measure_vertical_children(
        &mut self,
        children: &[Node],
        available: Rect,
    ) -> Vec<MeasuredSize> {
        children
            .iter()
            .map(|child| self.measure(child, available))
            .collect()
    }

    fn layout(&mut self, node: &Node, rect: Rect) {
        self.layout_node(node, rect, true);
    }

    fn layout_node(&mut self, node: &Node, rect: Rect, screen_bottom_inset: bool) {
        let visible = rect.intersection(self.clip);
        if self.scrolling
            && !matches!(
                &node.kind,
                NodeKind::Screen { .. }
                    | NodeKind::Stack { .. }
                    | NodeKind::MediaGridRow { .. }
                    | NodeKind::MediaCell { .. }
                    | NodeKind::Tabs { .. }
                    | NodeKind::ReactList { .. }
            )
            && (visible.width == 0.0 || visible.height == 0.0)
        {
            return;
        }
        match &node.kind {
            NodeKind::MediaGridRow { children } => {
                let size = rect.width / 3.0;
                for (column, child) in children.iter().enumerate() {
                    self.layout(child, Rect { x: rect.x + column as f32 * size, y: rect.y, width: size, height: size });
                }
            }
            NodeKind::MediaCell { source, selected, video, check, play, action } => {
                self.layout_image(node.identity, source, None, ImageFit::Cover, false, rect);
                if let Some(action) = action { self.push_hit_region(rect, action.clone()); }
                if *selected {
                    self.scene.masks.push(MaskRun {
                        mask: Mask::solid(), rect, clip: self.clip,
                        colour: Colour { alpha: 0.5, ..Colour::BLACK }, scrolling: self.scrolling,
                    });
                    let size = self.scaled(36.0).min(rect.width / 2.0);
                    self.scene.masks.push(MaskRun {
                        mask: check.clone(), rect: Rect {
                            x: rect.x + (rect.width - size) / 2.0, y: rect.y + (rect.height - size) / 2.0,
                            width: size, height: size,
                        }, clip: self.clip, colour: Colour::WHITE, scrolling: self.scrolling,
                    });
                }
                if *video {
                    let size = self.scaled(24.0).min(rect.width / 3.0);
                    let inset = self.scaled(6.0);
                    let x = rect.x + inset;
                    let y = rect.y + rect.height - size - inset;
                    self.scene.masks.push(MaskRun { mask: play.clone(), rect: Rect {
                        x, y, width: size, height: size },
                        clip: self.clip, colour: Colour::WHITE, scrolling: self.scrolling });
                }
            }
            NodeKind::ReactList { children, start, gap, .. } => {
                self.react_list_positions.insert(node.identity.0, rect.y + self.scroll_origin);
                let Some(metrics) = self.list_metrics.get(&node.identity.0) else { return; };
                let offsets: Vec<_> = (*start..=(*start + children.len()).min(metrics.keys.len()))
                    .map(|row| metrics.offset(row)).collect();
                let count = metrics.keys.len();
                for (index, child) in children.iter().enumerate() {
                    let row = start + index;
                    if index + 1 >= offsets.len() { break; }
                    let gap = if row + 1 < count { self.scaled(*gap) } else { 0.0 };
                    self.layout(child, Rect {
                        y: rect.y + offsets[index],
                        height: (offsets[index + 1] - offsets[index] - gap).max(0.0),
                        ..rect
                    });
                }
            }

            NodeKind::ConversationComposer { children } => {
                let side = self.scaled(28.0);
                let gap = self.scaled(12.0);
                for (index, child) in children.iter().enumerate() {
                    let (x, width) = match (children.len(), index) {
                        (2, 0) => (rect.x, (rect.width - side - gap).max(0.0)),
                        (3, 0) => (rect.x, side),
                        (3, 1) => (rect.x + side + gap, (rect.width - (side + gap) * 2.0).max(0.0)),
                        _ => (rect.x + rect.width - side, side),
                    };
                    let size = self.measure(child, Rect { width, ..rect });
                    let child_rect = Rect { x, y: rect.y + (rect.height - size.height) / 2.0, width, height: size.height };
                    if let NodeKind::TextInput { placeholder, state, action, clear, .. } = &child.kind {
                        self.layout_text_input(placeholder, *state, *action, clear,
                            Rect { width: width + side + gap, ..child_rect }, side + gap);
                    } else {
                        let first_hit = self.hit_regions.len();
                        self.layout(child, child_rect);
                        // Send dismisses the keyboard with the React message update.
                        if children.len() == 3 && index == 2 {
                            for hit in &mut self.hit_regions[first_hit..] { hit.preserve_input = true; }
                        }
                    }
                }
            }
            NodeKind::Message { children, outgoing } => {
                if let Some(child) = children.first() {
                    let max_width = rect.width * 0.85;
                    let size = self.measure(child, Rect { width: max_width, ..rect });
                    let width = size.width.ceil().min(max_width);
                    self.layout(child, Rect { x: if *outgoing { rect.x + rect.width - width } else { rect.x }, width, ..rect });
                }
            }
            NodeKind::MessageQuote { children } => {
                let inset = self.scaled(10.0);
                self.scene.quads.push(Quad { rect: Rect { width: self.scaled(2.0), ..rect }, clip: self.clip, colour: self.scene.colour(Colour::WHITE), scrolling: self.scrolling });
                if let Some(child) = children.first() { self.layout(child, Rect { x: rect.x + inset, width: (rect.width - inset).max(0.0), ..rect }); }
            }
            NodeKind::PlayingLayout { children, centred } => {
                let inset = self.scaled(CONTENT_INSET_START);
                let body = Rect { x: rect.x + inset, width: (rect.width - inset * 2.0).max(0.0), ..rect };
                let footer = self.measure(&children[1], body);
                let bottom = rect.y + rect.height - self.scaled(CONTENT_BOTTOM);
                self.layout(&children[1], Rect { y: bottom - footer.height, height: footer.height, ..body });
                let available = Rect { height: (bottom - footer.height - self.scaled(12.0) - body.y).max(0.0), ..body };
                let size = self.measure(&children[0], available);
                let y = available.y + if *centred { (available.height - size.height).max(0.0) / 2.0 } else { 0.0 };
                self.layout(&children[0], Rect { y, height: size.height, ..available });
            }
            NodeKind::PlayingTransport { children } => {
                for (index, child) in children.iter().enumerate() {
                    let size = self.measure(child, rect);
                    let x = match index {
                        0 => rect.x,
                        1 => rect.x + (rect.width - size.width) / 2.0,
                        _ => rect.x + rect.width - size.width,
                    };
                    self.layout(child, Rect { x, y: rect.y + (rect.height - size.height) / 2.0, width: size.width, height: size.height });
                }
            }
            NodeKind::PlayingPressable { children, action, long_action, selected } => {
                if *selected {
                    let height = self.control_line_height();
                    self.scene.quads.push(Quad {
                        rect: Rect { x: rect.x - self.scaled(3.0), y: rect.y + rect.height + self.scaled(5.0), width: rect.width + self.scaled(6.0), height },
                        clip: self.clip, colour: self.scene.colour(Colour::WHITE), scrolling: self.scrolling,
                    });
                }
                if let Some(action) = action {
                    let hit = if children.first().is_some_and(|child| matches!(child.kind, NodeKind::Icon { .. })) {
                        let width = rect.width.max(self.scaled(52.0));
                        let height = rect.height.max(self.scaled(52.0));
                        Rect { x: rect.x - (width - rect.width) / 2.0, y: rect.y - (height - rect.height) / 2.0, width, height }
                    } else { rect };
                    self.push_press_region(hit, action.clone(), long_action.clone());
                }
                if let Some(child) = children.first() { self.layout(child, rect); }
            }
            NodeKind::PlayingProgress { position, duration, seek } => {
                let ratio = if *duration > 0.0 { (position / duration).clamp(0.0, 1.0) } else { 0.0 };
                for (width, height) in [(rect.width, self.scaled(2.0)), (rect.width * ratio, self.scaled(6.0))] {
                    self.scene.quads.push(Quad { rect: Rect { y: rect.y + (rect.height - height) / 2.0, width, height, ..rect }, clip: self.clip, colour: self.scene.colour(Colour::WHITE), scrolling: self.scrolling });
                }
                if *seek && *duration > 0.0 {
                    self.push_hit_region(Rect { y: rect.y - self.scaled(15.0), height: self.scaled(36.0), ..rect }, Action::Seek { id: node.identity.0, left: rect.x, width: rect.width, duration: *duration });
                }
            }
            NodeKind::Row { children, has_image, action } => {
                let image_width = if *has_image { self.scaled(65.0).min(rect.width) } else { 0.0 };
                if *has_image {
                    if let Some(image) = children.first() {
                        let size = self.scaled(50.0).min(rect.width);
                        self.layout(image, Rect { width: size, height: size, y: rect.y + (rect.height - size) / 2.0, ..rect });
                    }
                }
                if let Some(text) = children.last() {
                    let available = Rect { x: rect.x + image_width, width: (rect.width - image_width).max(0.0), ..rect };
                    let size = self.measure(text, available);
                    self.layout(text, Rect { y: rect.y + (rect.height - size.height) / 2.0, height: size.height, ..available });
                }
                if let Some(action) = action { self.push_hit_region(rect, action.clone()); }
            }
            NodeKind::Screen {
                children,
                title,
                centred,
                footer,
                pinned_header,
                pinned_footer,
                right_action,
                media_picker,
            } => self.layout_screen(
                children,
                title.as_deref(),
                *centred,
                footer.as_ref(),
                *pinned_header,
                *pinned_footer,
                right_action.as_ref(),
                *media_picker,
                screen_bottom_inset,
                rect,
            ),
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
                text,
                font_size,
                align,
                max_lines,
            } => {
                self.layout_text(text, *font_size, *align, *max_lines, rect);
            }
            NodeKind::TextInput {
                placeholder,
                state,
                action,
                clear,
                ..
            } => self.layout_text_input(placeholder, *state, *action, clear, rect, 0.0),
            NodeKind::Button {
                label,
                icon,
                underline,
                action,
            } => self.layout_button(label, icon.clone(), *underline, action, rect),
            NodeKind::Field {
                label,
                value,
                action,
            } => self.layout_field(label, value, action, rect),
            NodeKind::Icon { mask, tone, bounds, .. } => self.scene.masks.push(MaskRun {
                mask: mask.clone(),
                rect: bounds.map_or(rect, |bounds| {
                    let width = rect.width / bounds.width;
                    let height = rect.height / bounds.height;
                    Rect { x: rect.x - bounds.x * width, y: rect.y - bounds.y * height, width, height }
                }),
                clip: self.clip,
                colour: self.scene.colour(tone_colour(*tone)),
                scrolling: self.scrolling,
            }),
            NodeKind::Image {
                source,
                fallback,
                fit,
                zoomable,
                ..
            } => self.layout_image(
                node.identity,
                source,
                fallback.as_ref(),
                *fit,
                *zoomable,
                rect,
            ),
            NodeKind::CameraPreview { controller, kind } => {
                self.layout_camera_preview(*controller, *kind, rect)
            }
            NodeKind::MapView { controller } => {
                self.scene.map_portal = Some(MapPortal { controller: *controller, rect });
            }
            NodeKind::Toggle {
                label,
                value,
                action,
                off,
                on,
            } => {
                let enabled = *value;
                self.layout_toggle(
                    label,
                    enabled,
                    action.as_ref(),
                    off.clone(),
                    on.clone(),
                    rect,
                );
            }
            NodeKind::Tabs { value, tabs } => self.layout_tabs(value, tabs, rect),
        }
    }

    fn layout_screen(
        &mut self,
        children: &[Node],
        title: Option<&str>,
        centred: bool,
        footer: Option<&(String, Option<Action>)>,
        pinned_header: bool,
        pinned_footer: bool,
        right_action: Option<&(Mask, Action)>,
        media_picker: bool,
        bottom_inset: bool,
        rect: Rect,
    ) {
        let has_header = title.is_some() || self.back_icon.is_some() || right_action.is_some();
        let header_height = if has_header {
            self.scaled(HEADER_HEIGHT)
        } else {
            0.0
        };
        let header_inset = self.scaled(HEADER_HORIZONTAL_INSET);
        let header_button_size = self.scaled(HEADER_BUTTON_SIZE);
        if self.back_icon.is_some() {
            self.push_hit_region(
                Rect {
                    x: rect.x + header_inset,
                    y: rect.y + (header_height - header_button_size) / 2.0,
                    width: header_button_size,
                    height: header_button_size,
                },
                Action::Back,
            );
        }
        if let Some((icon, action)) = right_action {
            let action_rect = Rect {
                x: rect.x + rect.width - header_inset - header_button_size,
                y: rect.y + (header_height - header_button_size) / 2.0,
                width: header_button_size,
                height: header_button_size,
            };
            self.push_hit_region(action_rect, action.clone());
            let size = self.scaled(HEADER_BACK_ICON_SIZE);
            self.scene.masks.push(MaskRun {
                mask: icon.clone(),
                rect: Rect { x: action_rect.x + (header_button_size - size) / 2.0, y: action_rect.y + (header_button_size - size) / 2.0, width: size, height: size },
                clip: self.clip,
                colour: self.scene.colour(Colour::WHITE),
                scrolling: false,
            });
        }
        if let Some(title) = title {
            let title_inset = header_inset
                + if self.back_icon.is_some() || right_action.is_some() {
                    header_button_size
                } else {
                    0.0
                };
            let title_height = self.scaled(32.0);
            let title_rect = Rect {
                x: rect.x + title_inset,
                y: rect.y + (header_height - title_height) / 2.0,
                width: (rect.width - title_inset * 2.0).max(0.0),
                height: title_height,
            };
            let font_size = self.scaled_font(HEADER_TEXT_SIZE);
            self.scene.text.push(TextRun {
                text: self.ellipsize(title, font_size, title_rect.width),
                rect: title_rect,
                clip: self.clip,
                font_size,
                colour: self.scene.colour(Colour::WHITE),
                align: TextAlign::Centre,
                scrolling: self.scrolling,
            });
        }

        let content_inset = if media_picker { 0.0 } else if pinned_footer { 16.0 } else { CONTENT_INSET_START };
        let scroll_track_end = if media_picker { SCROLL_TRACK_END } else { SCROLL_TRACK_END + content_inset - CONTENT_INSET_START };
        let scroll_content_inset_end = scroll_track_end * 2.0 - SCROLL_TRACK_WIDTH;

        let (children, rect) = if pinned_footer {
            let (composer, messages) = children.split_last().expect("screen footer requires a child");
            let inset = self.scaled(content_inset);
            let available = Rect { x: rect.x + inset, width: (rect.width - inset * 2.0).max(0.0), height: (rect.height - header_height).max(0.0), ..rect };
            let size = self.measure(composer, available);
            let bottom = if self.keyboard_inset > 0 { 0.0 } else { self.scaled(CONTENT_BOTTOM) };
            let height = size.height + bottom + self.scaled(10.0);
            self.layout(composer, Rect { y: rect.y + rect.height - bottom - size.height, height: size.height, ..available });
            (messages, Rect { height: (rect.height - height).max(header_height), ..rect })
        } else { (children, rect) };

        let rect = if let Some((label, action)) = footer {
            let font_size = self.scaled_font(40.0);
            let inset = self.scaled(CONTENT_INSET_START);
            let available_width = (rect.width - inset * 2.0).max(0.0);
            let lines = self.wrap_text(label, font_size, available_width, None);
            let line_height = self.text_line_height(40.0, lines.len());
            let height = (line_height * lines.len() as f32 + self.scaled(CONTENT_BOTTOM))
                .min((rect.height - header_height).max(0.0));
            let action_rect = Rect {
                x: rect.x + inset,
                y: rect.y + rect.height - height,
                width: available_width,
                height,
            };
            let scaled = self.font.as_scaled(PxScale::from(font_size));
            let baseline = (line_height - scaled.height()) / 2.0 + scaled.ascent();
            let ink_bottom = lines.last().into_iter().flat_map(|line| line.text.chars())
                .filter_map(|character| self.font.outline_glyph(self.font.glyph_id(character).with_scale(font_size)))
                .map(|glyph| glyph.px_bounds().max.y)
                .reduce(f32::max)
                .unwrap_or(0.0);
            let baseline_shift = line_height - baseline - ink_bottom;
            for (index, line) in lines.iter().enumerate() {
                self.scene.text.push(TextRun {
                    text: line.text.clone(),
                    rect: Rect {
                        y: action_rect.y + index as f32 * line_height + baseline_shift,
                        height: line_height,
                        ..action_rect
                    },
                    clip: action_rect.intersection(self.clip),
                    font_size,
                    colour: self.scene.colour(if action.is_some() { Colour::WHITE } else { Colour::MUTED }),
                    align: TextAlign::Centre,
                    scrolling: false,
                });
            }
            if let Some(action) = action {
                self.push_hit_region(action_rect, action.clone());
            }
            Rect { height: rect.height - height, ..rect }
        } else {
            rect
        };

        let (children, header_height) = if pinned_header && !children.is_empty() {
            let top = self.scaled(if has_header { HEADER_CONTENT_TOP } else { CONTENT_TOP });
            let available = Rect {
                x: rect.x + self.scaled(CONTENT_INSET_START),
                y: rect.y + header_height + top,
                width: (rect.width - self.scaled(CONTENT_INSET_START + CONTENT_INSET_END)).max(0.0),
                height: (rect.height - header_height - top - self.scaled(CONTENT_BOTTOM)).max(0.0),
            };
            let size = self.measure(&children[0], available);
            self.layout(&children[0], Rect { height: size.height, ..available });
            (&children[1..], header_height + size.height + self.scaled(CONTENT_GAP))
        } else {
            (children, header_height)
        };

        let fills_remaining = children.len() == 1 && fills_remaining_screen(&children[0]);
        let inset_start = if fills_remaining {
            0.0
        } else {
            self.scaled(content_inset)
        };
        let inset_end = if fills_remaining {
            0.0
        } else {
            self.scaled(content_inset)
        };
        let first_child_is_full_bleed = children.first().is_some_and(full_bleed_image);
        let inset_top = if media_picker || fills_remaining || first_child_is_full_bleed {
            0.0
        } else if has_header {
            self.scaled(HEADER_CONTENT_TOP)
        } else {
            self.scaled(CONTENT_TOP)
        };
        let requested_bottom_inset = if bottom_inset && !media_picker && !fills_remaining && !pinned_footer {
            self.scaled(CONTENT_BOTTOM)
        } else {
            0.0
        };
        let mut unbounded_content = Rect {
            x: rect.x + inset_start,
            y: rect.y + header_height + inset_top,
            width: (rect.width - inset_start - inset_end).max(0.0),
            height: if fills_remaining {
                (rect.height - header_height).max(0.0)
            } else {
                f32::INFINITY
            },
        };
        let gap = if media_picker || fills_remaining {
            0.0
        } else {
            self.scaled(CONTENT_GAP)
        };
        let (mut sizes, mut content_height) = if fills_remaining {
            (
                vec![MeasuredSize {
                    width: unbounded_content.width,
                    height: unbounded_content.height,
                }],
                unbounded_content.height,
            )
        } else {
            self.measure_screen_content(children, unbounded_content, gap)
        };
        let mut inset_bottom = if first_child_is_full_bleed {
            requested_bottom_inset
                .min((rect.height - header_height - inset_top - content_height).max(0.0))
        } else {
            requested_bottom_inset
        };
        let mut content = Rect {
            height: (rect.height - header_height - inset_top - inset_bottom).max(0.0),
            ..unbounded_content
        };
        if content_height > content.height && !fills_remaining && !media_picker {
            unbounded_content.width =
                (rect.width - inset_start - self.scaled(scroll_content_inset_end)).max(0.0);
            (sizes, content_height) = self.measure_screen_content(children, unbounded_content, gap);
            inset_bottom = if first_child_is_full_bleed {
                requested_bottom_inset
                    .min((rect.height - header_height - inset_top - content_height).max(0.0))
            } else {
                requested_bottom_inset
            };
            content = Rect {
                height: (rect.height - header_height - inset_top - inset_bottom).max(0.0),
                ..unbounded_content
            };
        }
        self.scroll_max = (content_height - content.height).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, self.scroll_max);
        self.scroll_origin = self.scroll_offset;
        self.scene.scroll_origin = self.scroll_origin;
        self.scene.scroll_offset = self.scroll_offset;
        self.scene.scroll_max = self.scroll_max;

        let scroll_clip = Rect {
            x: rect.x,
            y: content.y,
            width: rect.width,
            height: content.height,
        };
        self.scene.scroll_clip = Some(scroll_clip);
        let previous_clip = self.clip;
        self.clip = Rect {
            y: scroll_clip.y - scroll_clip.height,
            height: scroll_clip.height * 3.0,
            ..scroll_clip
        };
        self.scrolling = true;
        self.layout_vertical_children_with_sizes(
            children,
            sizes,
            gap,
            if centred { Alignment::Centre } else { Alignment::Stretch },
            if centred && self.scroll_max == 0.0 {
                Justification::Centre
            } else {
                Justification::Start
            },
            Rect {
                y: content.y - self.scroll_origin,
                height: content.height.max(content_height),
                ..content
            },
        );
        self.scrolling = false;
        self.clip = previous_clip;

        if self.scroll_max > 0.0 {
            let track_width = self.scaled(SCROLL_TRACK_WIDTH);
            let thumb_width = self.scaled(SCROLL_THUMB_WIDTH);
            let track_x = rect.x + rect.width - self.scaled(scroll_track_end);
            let track_inset = if media_picker { self.scaled(16.0).min(content.height / 2.0) } else { 0.0 };
            self.scene.scroll_bar = Some(ScrollBar {
                track: Rect {
                    x: track_x,
                    y: content.y + track_inset,
                    width: track_width,
                    height: content.height - track_inset * 2.0,
                },
                thumb_width,
            });
        }

        if let Some(back) = &self.back_icon {
            let icon_size = self.scaled(HEADER_BACK_ICON_SIZE);
            self.scene.masks.push(MaskRun {
                mask: back.clone(),
                rect: Rect {
                    x: rect.x + header_inset + self.scaled(HEADER_BACK_OFFSET_X),
                    y: rect.y + self.scaled(HEADER_BACK_OFFSET_Y),
                    width: icon_size,
                    height: icon_size,
                },
                clip: self.clip,
                colour: self.scene.colour(Colour::WHITE),
                scrolling: self.scrolling,
            });
        }
    }

    fn measure_screen_content(
        &mut self,
        children: &[Node],
        available: Rect,
        gap: f32,
    ) -> (Vec<MeasuredSize>, f32) {
        let sizes = self.measure_vertical_children(children, available);
        let entries = sizes.len();
        let height = sizes.iter().map(|measure| measure.height).sum::<f32>()
            + gap * entries.saturating_sub(1) as f32;
        (sizes, height)
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
        let sizes = self.measure_vertical_children(children, rect);
        self.layout_vertical_children_with_sizes(children, sizes, gap, align, justify, rect);
    }

    fn layout_vertical_children_with_sizes(
        &mut self,
        children: &[Node],
        sizes: Vec<MeasuredSize>,
        gap: f32,
        align: Alignment,
        justify: Justification,
        rect: Rect,
    ) {
        let entries = sizes.len();
        let content_height = sizes.iter().map(|measure| measure.height).sum::<f32>()
            + gap * entries.saturating_sub(1) as f32;
        let (mut cursor, actual_gap) =
            distribution(rect.y, rect.height, content_height, gap, entries, justify);
        for (child, size) in children.iter().zip(sizes) {
            let bleed = full_bleed_image(child);
            let width = if bleed {
                self.viewport.width as f32
            } else if align == Alignment::Stretch && stretchable(child) {
                rect.width
            } else {
                size.width.min(rect.width)
            };
            let page_centred_image = matches!(child.kind, NodeKind::Image { .. })
                && matches!(align, Alignment::Centre | Alignment::Stretch)
                && self.scroll_max > 0.0
                && (rect.x - self.scaled(CONTENT_INSET_START)).abs() < 1.0
                && (rect.width
                    - (self.viewport.width as f32
                        - self.scaled(CONTENT_INSET_START)
                        - self.scaled(SCROLL_CONTENT_INSET_END)))
                .abs()
                    < 1.0;
            let x = if bleed {
                0.0
            } else if page_centred_image {
                (self.viewport.width as f32 - width) / 2.0
            } else {
                cross_position(rect.x, rect.width, width, align)
            };
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
        let colour = self.scene.colour(if action.is_some() { Colour::WHITE } else { Colour::MUTED });
        let mut text_x = rect.x;
        if let Some(mask) = icon {
            let size = self.scaled(if label.is_empty() { BUTTON_HEIGHT } else { BUTTON_ICON_SIZE });
            self.scene.masks.push(MaskRun {
                mask,
                rect: Rect {
                    x: rect.x,
                    y: rect.y + (rect.height - size).max(0.0) / 2.0,
                    width: size,
                    height: size,
                },
                clip: self.clip,
                colour,
                scrolling: self.scrolling,
            });
            text_x += size + if label.is_empty() { 0.0 } else { self.scaled(BUTTON_ICON_GAP) };
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
            colour,
            align: TextAlign::Start,
            scrolling: self.scrolling,
        });
        if underline {
            let underline_height = self.control_line_height();
            self.scene.quads.push(Quad {
                rect: Rect {
                    x: text_x,
                    y: (rect.y + rect.height).round() - underline_height,
                    width: text_width.min(text_rect.width),
                    height: underline_height,
                },
                clip: self.clip,
                colour,
                scrolling: self.scrolling,
            });
        }
        if let Some(action) = action {
            self.push_hit_region(rect, action.clone());
        }
    }

    fn layout_text(&mut self, text: &str, font_size: Option<f32>, align: TextAlign, max_lines: Option<u32>, rect: Rect) {
        let size = font_size.unwrap_or(DEFAULT_TEXT_SIZE);
        let font_size = self.scaled_font(size);
        let lines = self.wrap_text(text, font_size, rect.width, max_lines);
        let line_height = self.text_line_height(size, lines.len());
        for (index, line) in lines.into_iter().enumerate() {
            let mut line_rect = Rect {
                y: rect.y + line_height * index as f32,
                height: line_height
                    .min((rect.height - line_height * index as f32).max(0.0)),
                ..rect
            };
            if size == DEFAULT_TEXT_SIZE {
                line_rect.y += self.scaled(1.0);
                line_rect.height = (line_rect.height - self.scaled(1.0)).max(0.0);
            }
            self.scene.text.push(TextRun {
                text: line.text,
                rect: line_rect,
                clip: self.clip,
                font_size,
                colour: self.scene.colour(Colour::WHITE),
                align: if align == TextAlign::Justify && !line.wrapped {
                    TextAlign::Start
                } else {
                    align
                },
                scrolling: self.scrolling,
            });
        }
    }

    fn layout_field(&mut self, label: &str, value: &str, action: &Option<Action>, rect: Rect) {
        let label_height = self.scaled(FIELD_LABEL_HEIGHT).min(rect.height);
        let label_font_size = self.scaled_font(FIELD_LABEL_SIZE);
        self.scene.text.push(TextRun {
            text: self.ellipsize(label, label_font_size, rect.width),
            rect: Rect {
                height: label_height,
                ..rect
            },
            clip: self.clip,
            font_size: label_font_size,
            colour: self.scene.colour(Colour::WHITE),
            align: TextAlign::Start,
            scrolling: self.scrolling,
        });
        self.layout_text(
            value,
            None,
            TextAlign::Start,
            None,
            Rect {
                y: rect.y + label_height,
                height: (rect.height - label_height).max(0.0),
                ..rect
            },
        );
        if let Some(action) = action {
            self.push_hit_region(rect, action.clone());
        }
    }

    fn input_lines(&self, value: &str, width: f32) -> Vec<(usize, usize)> {
        let font_size = self.scaled_font(TEXT_INPUT_TEXT_SIZE);
        let mut lines = Vec::new();
        let mut offset = 0;
        for paragraph in value.split('\n') {
            let mut start = 0;
            if paragraph.is_empty() { lines.push((offset, offset)); }
            while start < paragraph.len() {
                let end = self.forced_text_break(paragraph, start, font_size, width);
                lines.push((offset + start, offset + end));
                start = end;
            }
            offset += paragraph.len() + 1;
        }
        lines
    }

    fn layout_multiline_input(&mut self, placeholder: &str, state: StateId, rect: Rect, trailing_width: f32) {
        let value = match self.state.get(state.0) { Some(StateValue::String(value)) => value.clone(), _ => String::new() };
        let focused = self.focused_input == Some(state);
        let font_size = self.scaled_font(TEXT_INPUT_TEXT_SIZE);
        let line_height = self.scaled(TEXT_INPUT_HEIGHT - TEXT_INPUT_BOTTOM_PADDING);
        let cursor_width = self.scaled(1.0);
        let viewport = Rect { width: (rect.width - trailing_width - cursor_width).max(0.0), height: (rect.height - self.scaled(TEXT_INPUT_BOTTOM_PADDING)).max(0.0), ..rect };
        let lines = self.input_lines(&value, viewport.width);
        let cursor = if focused { self.focused_input_cursor } else { value.len() };
        let cursor_line = lines.iter().rposition(|(start, _)| *start <= cursor).unwrap_or(0);
        let scroll_max = (lines.len() as f32 * line_height - viewport.height).max(0.0);
        let scroll_offset = self.text_input_scroll_offsets.get(&state).copied()
            .unwrap_or((cursor_line + 1) as f32 * line_height - viewport.height)
            .clamp(0.0, scroll_max);
        let clip = viewport.intersection(self.clip);
        let text_run = self.scene.text.len();
        let showing_placeholder = value.is_empty() && !focused;
        for (index, (start, end)) in lines.iter().copied().enumerate() {
            self.scene.text.push(TextRun {
                text: if showing_placeholder { placeholder.to_owned() } else { value[start..end].to_owned() },
                rect: Rect { y: viewport.y + index as f32 * line_height - scroll_offset, height: line_height, ..viewport },
                clip, font_size, colour: self.scene.colour(if showing_placeholder { Colour::MUTED } else { Colour::WHITE }),
                align: TextAlign::Start, scrolling: self.scrolling,
            });
        }
        if focused {
            let (start, end) = lines[cursor_line];
            self.scene.text_cursor = Some(Quad {
                rect: Rect {
                    x: viewport.x + self.text_width(&value[start..cursor.min(end)], font_size),
                    y: viewport.y + cursor_line as f32 * line_height - scroll_offset + self.scaled(2.0),
                    width: cursor_width, height: (line_height - self.scaled(4.0)).max(0.0),
                },
                clip: Rect { width: viewport.width + cursor_width, ..viewport }.intersection(self.clip),
                colour: self.scene.colour(Colour::WHITE), scrolling: self.scrolling,
            });
        }
        let underline_height = self.control_line_height();
        self.scene.quads.push(Quad {
            rect: Rect { y: (rect.y + rect.height).round() - underline_height, height: underline_height, ..rect },
            clip: self.clip, colour: self.scene.colour(Colour::WHITE), scrolling: self.scrolling,
        });
        self.text_inputs.push(TextInputLayout {
            state, action: TextInputAction::Return, text_run, rect, text_rect: viewport,
            hit_rect: Rect { width: (rect.width - trailing_width).max(0.0), ..rect }.intersection(self.clip),
            scroll_offset, scroll_max, scrolling: self.scrolling,
        });
        self.push_hit_region(rect, Action::FocusTextInput { state, action: TextInputAction::Return });
    }

    fn layout_text_input(
        &mut self,
        placeholder: &str,
        state: StateId,
        action: TextInputAction,
        clear: &Mask,
        rect: Rect,
        trailing_width: f32,
    ) {
        if action == TextInputAction::Return {
            self.layout_multiline_input(placeholder, state, rect, trailing_width);
            return;
        }
        let value = match self.state.get(state.0) {
            Some(StateValue::String(value)) => value.clone(),
            _ => String::new(),
        };
        let focused = self.focused_input == Some(state);
        let showing_placeholder = value.is_empty() && !focused;
        let text = if showing_placeholder {
            placeholder
        } else {
            &value
        };
        let text_height = (rect.height - self.scaled(TEXT_INPUT_BOTTOM_PADDING)).max(0.0);
        let font_size = self.scaled_font(TEXT_INPUT_TEXT_SIZE);
        let clear_button_width = if value.is_empty() || trailing_width > 0.0 {
            0.0
        } else {
            self.scaled(TEXT_INPUT_CLEAR_ICON_SIZE + TEXT_INPUT_CLEAR_PADDING * 2.0)
        };
        let clear_gap = if value.is_empty() || trailing_width > 0.0 {
            0.0
        } else {
            self.scaled(TEXT_INPUT_CLEAR_GAP)
        };
        let cursor_width = self.scaled(1.0);
        let text_viewport = Rect {
            width: (rect.width - trailing_width - clear_button_width - clear_gap - cursor_width).max(0.0),
            height: text_height,
            ..rect
        };
        let text_width = self.text_width(text, font_size);
        let scroll_max = if value.is_empty() {
            0.0
        } else {
            (text_width - text_viewport.width).max(0.0)
        };
        let scroll_offset = if value.is_empty() {
            0.0
        } else {
            self.text_input_scroll_offsets
                .get(&state)
                .copied()
                .unwrap_or(scroll_max)
                .clamp(0.0, scroll_max)
        };
        let text_run = self.scene.text.len();
        self.scene.text.push(TextRun {
            text: text.to_owned(),
            rect: Rect {
                x: text_viewport.x - scroll_offset,
                width: text_width.max(text_viewport.width),
                ..text_viewport
            },
            clip: text_viewport.intersection(self.clip),
            font_size,
            colour: self.scene.colour(if showing_placeholder { Colour::MUTED } else { Colour::WHITE }),
            align: TextAlign::Start,
            scrolling: self.scrolling,
        });
        if focused {
            let cursor = self.focused_input_cursor;
            let cursor_offset = self.text_width(&value[..cursor], font_size);
            let cursor_clip = Rect {
                width: text_viewport.width + cursor_width,
                ..text_viewport
            };
            self.scene.text_cursor = Some(Quad {
                rect: Rect {
                    x: text_viewport.x + cursor_offset - scroll_offset,
                    y: rect.y + self.scaled(2.0),
                    width: cursor_width,
                    height: (text_height - self.scaled(4.0)).max(0.0),
                },
                clip: cursor_clip.intersection(self.clip),
                colour: self.scene.colour(Colour::WHITE),
                scrolling: self.scrolling,
            });
        }
        let underline_height = self.control_line_height();
        self.scene.quads.push(Quad {
            rect: Rect {
                x: rect.x,
                y: (rect.y + rect.height).round() - underline_height,
                width: rect.width,
                height: underline_height,
            },
            clip: self.clip,
            colour: self.scene.colour(Colour::WHITE),
            scrolling: self.scrolling,
        });
        self.text_inputs.push(TextInputLayout {
            state,
            action,
            text_run,
            hit_rect: Rect {
                width: (rect.width - trailing_width - clear_button_width).max(0.0),
                ..rect
            }
            .intersection(self.clip),
            text_rect: text_viewport,
            rect,
            scroll_offset,
            scroll_max,
            scrolling: self.scrolling,
        });
        self.push_hit_region(rect, Action::FocusTextInput { state, action });
        if !value.is_empty() && trailing_width == 0.0 {
            let icon_size = self.scaled(TEXT_INPUT_CLEAR_ICON_SIZE);
            let clear_rect = Rect {
                x: rect.x + rect.width - clear_button_width,
                y: rect.y,
                width: clear_button_width,
                height: rect.height,
            };
            self.scene.masks.push(MaskRun {
                mask: clear.clone(),
                rect: Rect {
                    x: clear_rect.x + self.scaled(TEXT_INPUT_CLEAR_PADDING),
                    y: rect.y + (text_height - icon_size) / 2.0,
                    width: icon_size,
                    height: icon_size,
                },
                clip: rect.intersection(self.clip),
                colour: self.scene.colour(Colour::WHITE),
                scrolling: self.scrolling,
            });
            self.push_hit_region(clear_rect, Action::ClearInput { state });
        }
    }

    fn layout_toggle(
        &mut self,
        label: &str,
        enabled: bool,
        action: Option<&Action>,
        off: Mask,
        on: Mask,
        rect: Rect,
    ) {
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
                colour: self.scene.colour(Colour::WHITE),
                scrolling: self.scrolling,
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
                colour: self.scene.colour(Colour::WHITE),
                scrolling: self.scrolling,
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
                colour: self.scene.colour(Colour::WHITE),
                scrolling: self.scrolling,
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
                colour: self.scene.colour(Colour::WHITE),
                scrolling: self.scrolling,
            });
        }

        let label_x = rect.x
            + self.scaled(TOGGLE_START + TOGGLE_ICON_SIZE + TOGGLE_LINE_WIDTH + TOGGLE_LABEL_GAP);
        let label_rect = Rect {
            x: label_x,
            y: rect.y,
            width: (rect.x + rect.width - label_x).max(0.0),
            height: rect.height,
        };
        let label_font_size = self.scaled_font(DEFAULT_TEXT_SIZE);
        self.scene.text.push(TextRun {
            text: self.ellipsize(label, label_font_size, label_rect.width),
            rect: label_rect,
            clip: self.clip,
            font_size: label_font_size,
            colour: self.scene.colour(Colour::WHITE),
            align: TextAlign::Start,
            scrolling: self.scrolling,
        });
        if let Some(action) = action {
            self.push_hit_region(rect, action.clone());
        }
    }

    fn layout_tabs(&mut self, value: &usize, tabs: &[Tab], rect: Rect) {
        let keyboard_visible = self.text_input_active();
        let nav_height = if keyboard_visible {
            0.0
        } else {
            self.scaled(NAV_HEIGHT).min(rect.height)
        };
        let content = Rect {
            height: (rect.height - nav_height).max(0.0),
            ..rect
        };
        let active = self.active_tab_index(value, tabs.len()).unwrap_or(0);
        if let Some(tab) = tabs.get(active) {
            self.layout_node(&tab.screen, content, false);
        }

        if keyboard_visible || tabs.is_empty() {
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
                mask: tab.icon.clone(),
                rect: Rect {
                    x: centres[index] - icon_size / 2.0,
                    y: icon_y,
                    width: icon_size,
                    height: icon_size,
                },
                clip: self.clip,
                colour: if index == active {
                    self.scene.colour(Colour::WHITE)
                } else {
                    self.scene.colour(Colour::MUTED)
                },
                scrolling: self.scrolling,
            });
            self.push_hit_region(slot, tab.action.clone());
        }
    }

    fn active_tab_index(&self, value: &usize, tab_count: usize) -> Option<usize> {
        (*value < tab_count).then_some(*value)
    }

    fn layout_image(
        &mut self,
        identity: NodeIdentity,
        source: &ImageSource,
        fallback: Option<&ImageAsset>,
        fit: ImageFit,
        zoomable: bool,
        rect: Rect,
    ) {
        let visible = rect.intersection(self.clip);
        if visible.width <= 0.0 || visible.height <= 0.0 {
            let nearby = rect.intersection(Rect {
                y: self.clip.y - self.clip.height,
                height: self.clip.height * 3.0,
                ..self.clip
            });
            if !matches!(source, ImageSource::Native(module, url) if module == "files" && url.starts_with("ink-media://"))
                || nearby.width <= 0.0 || nearby.height <= 0.0 {
                return;
            }
        }
        let image = match source {
            ImageSource::Asset(asset) => Some(ImageData::Asset(asset.clone())),
            ImageSource::Native(module, source) => {
                if source.is_empty() {
                    fallback.cloned().map(ImageData::Asset)
                } else {
                    let key = RemoteImageKey {
                        module: module.clone(),
                        url: source.clone(),
                        width: rect.width.ceil().max(1.0) as u32,
                        height: if module == "barcode" { rect.width.ceil().max(1.0) as u32 } else { rect.height.ceil().max(1.0) as u32 },
                        fit,
                    };
                    self.visible_images.insert(key.clone());
                    let loaded = match self.remote_images.get(&key) {
                        Some(RemoteImageState::Ready(image)) => {
                            Some(ImageData::Remote(image.clone()))
                        }
                        _ => None,
                    };
                    if loaded.is_none() {
                        self.queue_remote_image(key);
                    }
                    loaded.or_else(|| fallback.cloned().map(ImageData::Asset))
                }
            }
        };
        if visible.width <= 0.0 || visible.height <= 0.0 { return; }
        if let Some(image) = image {
            let transform = if zoomable {
                let content = image_content_rect(&image, rect, fit);
                let zoom = self.image_zooms.entry(identity).or_insert(ImageZoomState {
                    viewport: rect,
                    content,
                    scrolling: self.scrolling,
                    transform: ImageTransform::default(),
                });
                zoom.viewport = rect;
                zoom.content = content;
                zoom.scrolling = self.scrolling;
                zoom.transform = constrain_image_transform(*zoom, zoom.transform);
                self.visible_zoom_images.insert(identity);
                zoom.transform
            } else {
                ImageTransform::default()
            };
            self.scene.images.push(ImageRun {
                image,
                zoom_id: zoomable.then_some(identity.0),
                rect,
                clip: self.clip,
                fit,
                scrolling: self.scrolling,
                transform,
            });
        }
    }

    fn layout_camera_preview(
        &mut self,
        controller: ControllerId,
        kind: CameraPreviewKind,
        rect: Rect,
    ) {
        self.scene.quads.push(Quad {
            rect,
            clip: self.clip,
            colour: Colour::BLACK,
            scrolling: self.scrolling,
        });
        self.scene.camera_portal = Some(CameraPortal {
            controller,
            kind,
            rect,
        });
    }

    fn wrap_text(
        &self,
        text: &str,
        font_size: f32,
        available_width: f32,
        max_lines: Option<u32>,
    ) -> Vec<WrappedLine> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            if paragraph.is_empty() {
                lines.push(WrappedLine {
                    text: String::new(),
                    width: 0.0,
                    wrapped: false,
                });
                continue;
            }

            let breakpoints = linebreaks(paragraph)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let mut start = 0;
            while start < paragraph.len() {
                let mut best = None;
                for end in breakpoints.iter().copied().filter(|end| *end > start) {
                    let display_end = trim_whitespace_end(paragraph, start, end);
                    let candidate = &paragraph[start..display_end];
                    let width = self.text_width(candidate, font_size);
                    if width <= available_width {
                        best = Some((end, display_end, width));
                    } else {
                        break;
                    }
                }

                let (end, display_end, width) = best.unwrap_or_else(|| {
                    let end = self.forced_text_break(paragraph, start, font_size, available_width);
                    (end, end, self.text_width(&paragraph[start..end], font_size))
                });
                lines.push(WrappedLine {
                    text: paragraph[start..display_end].to_owned(),
                    width,
                    wrapped: end < paragraph.len(),
                });
                start = skip_whitespace_start(paragraph, end);
            }
        }
        if let Some(max_lines) = max_lines.map(|value| value as usize)
            && lines.len() > max_lines
        {
            lines.truncate(max_lines);
            if let Some(line) = lines.last_mut() {
                line.text = self.ellipsize_forced(&line.text, font_size, available_width);
                line.width = self.text_width(&line.text, font_size);
                line.wrapped = false;
            }
        }
        lines
    }

    fn forced_text_break(
        &self,
        text: &str,
        start: usize,
        font_size: f32,
        available_width: f32,
    ) -> usize {
        let mut best = start;
        for (offset, grapheme) in text[start..].grapheme_indices(true) {
            let end = start + offset + grapheme.len();
            if best > start && self.text_width(&text[start..end], font_size) > available_width {
                break;
            }
            best = end;
        }
        best
    }

    fn text_line_height(&self, size: f32, lines: usize) -> f32 {
        let single_line = if size == DEFAULT_TEXT_SIZE {
            BUTTON_HEIGHT
        } else {
            size * 1.25
        };
        self.scaled(if lines > 1 {
            single_line.max(size * 1.4)
        } else {
            single_line
        })
    }

    fn text_width(&self, text: &str, font_size: f32) -> f32 {
        let scaled = self.font.as_scaled(PxScale::from(font_size));
        let mut previous = None;
        text.graphemes(true).fold(0.0, |mut width, grapheme| {
            if is_emoji_grapheme(grapheme) {
                previous = None;
                return width + font_size;
            }
            for character in grapheme.chars() {
                let glyph = scaled.glyph_id(character);
                let kerning = previous
                    .map(|previous| scaled.kern(previous, glyph))
                    .unwrap_or_default();
                width += kerning + scaled.h_advance(glyph);
                previous = Some(glyph);
            }
            width
        })
    }

    fn text_cursor_for_offset(&self, text: &str, font_size: f32, offset: f32) -> usize {
        let scaled = self.font.as_scaled(PxScale::from(font_size));
        let mut previous = None;
        let mut width = 0.0;
        for (index, grapheme) in text.grapheme_indices(true) {
            let start = width;
            if is_emoji_grapheme(grapheme) {
                width += font_size;
                previous = None;
            } else {
                for character in grapheme.chars() {
                    let glyph = scaled.glyph_id(character);
                    width += previous
                        .map(|previous| scaled.kern(previous, glyph))
                        .unwrap_or_default()
                        + scaled.h_advance(glyph);
                    previous = Some(glyph);
                }
            }
            if offset < (start + width) / 2.0 {
                return index;
            }
        }
        text.len()
    }

    fn ellipsize(&self, text: &str, font_size: f32, available_width: f32) -> String {
        if self.text_width(text, font_size) <= available_width {
            return text.to_owned();
        }
        self.ellipsize_forced(text, font_size, available_width)
    }

    fn ellipsize_forced(&self, text: &str, font_size: f32, available_width: f32) -> String {
        let ellipsis = '…';
        let ellipsis_width = self.text_width("…", font_size);
        let mut visible = text.to_owned();
        while !visible.is_empty()
            && self.text_width(&visible, font_size) + ellipsis_width > available_width
        {
            let Some((index, _)) = visible.grapheme_indices(true).next_back() else {
                break;
            };
            visible.truncate(index);
        }
        visible.push(ellipsis);
        visible
    }

    fn scaled(&self, value: f32) -> f32 {
        value * self.viewport.scale
    }

    fn scaled_font(&self, value: f32) -> f32 {
        (self.scaled(value) * PUBLIC_SANS_RASTER_SCALE)
            .round()
            .clamp(1.0, u16::MAX as f32)
    }

    fn control_line_height(&self) -> f32 {
        self.scaled(CONTROL_LINE_HEIGHT).ceil().max(1.0)
    }

    fn push_hit_region(&mut self, rect: Rect, action: Action) {
        self.push_press_region(rect, action, None);
    }

    fn push_press_region(&mut self, rect: Rect, action: Action, long_action: Option<Action>) {
        let rect = rect.intersection(self.clip);
        if rect.width > 0.0 && rect.height > 0.0 {
            self.hit_regions.push(HitRegion {
                rect,
                action,
                long_action,
                scrolling: self.scrolling,
                preserve_input: false,
            });
        }
    }
}

fn text_cursor_boundary(text: &str, cursor: usize) -> usize {
    let cursor = cursor.min(text.len());
    if cursor == text.len() {
        return cursor;
    }
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .take_while(|index| *index <= cursor)
        .last()
        .unwrap_or_default()
}

fn next_text_cursor_boundary(text: &str, cursor: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .find(|index| *index >= cursor)
        .unwrap_or(text.len())
}

pub fn is_emoji_grapheme(grapheme: &str) -> bool {
    if grapheme.is_empty() || grapheme.chars().any(is_text_presentation_selector) {
        return false;
    }

    let mut has_emoji = false;
    let mut has_emoji_presentation = false;
    let mut has_sequence_marker = false;
    let mut regional_indicators = 0;
    for character in grapheme.chars() {
        has_emoji |= character.is_emoji_char();
        has_emoji_presentation |= matches!(
            character.emoji_status(),
            EmojiStatus::EmojiPresentation
                | EmojiStatus::EmojiPresentationAndModifierBase
                | EmojiStatus::EmojiPresentationAndEmojiComponent
                | EmojiStatus::EmojiPresentationAndModifierAndEmojiComponent
        );
        has_sequence_marker |= is_emoji_presentation_selector(character) || is_zwj(character);
        regional_indicators += usize::from(is_regional_indicator(character));
    }

    has_emoji_presentation || (has_emoji && (has_sequence_marker || regional_indicators >= 2))
}

#[derive(Clone, Copy, Debug, Default)]
struct MeasuredSize {
    width: f32,
    height: f32,
}

struct WrappedLine {
    text: String,
    width: f32,
    wrapped: bool,
}

fn trim_whitespace_end(text: &str, start: usize, end: usize) -> usize {
    text[start..end]
        .char_indices()
        .rev()
        .find(|(_, character)| !character.is_whitespace())
        .map_or(start, |(index, character)| {
            start + index + character.len_utf8()
        })
}

fn skip_whitespace_start(text: &str, start: usize) -> usize {
    text[start..]
        .char_indices()
        .find(|(_, character)| !character.is_whitespace())
        .map_or(text.len(), |(index, _)| start + index)
}

fn invert_colour(colour: Colour) -> Colour {
    Colour {
        red: 1.0 - colour.red,
        green: 1.0 - colour.green,
        blue: 1.0 - colour.blue,
        ..colour
    }
}

fn tone_colour(tone: Tone) -> Colour {
    match tone {
        Tone::Primary => Colour::WHITE,
        Tone::Muted => Colour::MUTED,
    }
}

fn image_id(key: &RemoteImageKey) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

fn stretchable(node: &Node) -> bool {
    matches!(
        &node.kind,
        NodeKind::Stack { .. }
            | NodeKind::ConversationComposer { .. }
            | NodeKind::Message { .. }
            | NodeKind::MessageQuote { .. }
            | NodeKind::PlayingTransport { .. }
            | NodeKind::Row { .. }
            | NodeKind::Text { .. }
            | NodeKind::TextInput { .. }
            | NodeKind::Button { .. }
            | NodeKind::Field { .. }
            | NodeKind::CameraPreview { .. }
            | NodeKind::MapView { .. }
            | NodeKind::MediaGridRow { .. }
            | NodeKind::MediaCell { .. }
            | NodeKind::Toggle { .. }
    )
}

fn full_bleed_image(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Image { bleed: true, .. } => true,
        _ => false,
    }
}

fn constrain_image_transform(
    zoom: ImageZoomState,
    mut transform: ImageTransform,
) -> ImageTransform {
    transform.scale = transform.scale.clamp(1.0, IMAGE_MAX_SCALE);
    let content = Rect {
        x: zoom.content.x * transform.scale + transform.translation_x,
        y: zoom.content.y * transform.scale + transform.translation_y,
        width: zoom.content.width * transform.scale,
        height: zoom.content.height * transform.scale,
    };
    transform.translation_x += constrained_axis(
        content.x,
        content.width,
        zoom.viewport.x,
        zoom.viewport.width,
    );
    transform.translation_y += constrained_axis(
        content.y,
        content.height,
        zoom.viewport.y,
        zoom.viewport.height,
    );
    transform
}

fn constrained_axis(content_start: f32, content_size: f32, view_start: f32, view_size: f32) -> f32 {
    if content_size <= view_size {
        view_start + (view_size - content_size) / 2.0 - content_start
    } else if content_start > view_start {
        view_start - content_start
    } else {
        let content_end = content_start + content_size;
        let view_end = view_start + view_size;
        (view_end - content_end).max(0.0)
    }
}

fn fills_remaining_screen(node: &Node) -> bool {
    matches!(
        &node.kind,
        NodeKind::PlayingLayout { .. } | NodeKind::CameraPreview { .. } | NodeKind::MapView { .. }
            | NodeKind::Image {
                bleed: true,
                zoomable: true,
                ..
            }
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

#[cfg(feature = "perf")]
fn elapsed_ns(started: Instant) -> u64 {
    started.elapsed().as_nanos() as u64
}
