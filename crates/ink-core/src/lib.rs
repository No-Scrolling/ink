use std::time::{Duration, Instant};
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

use ab_glyph::{Font, PxScale, ScaleFont};
use unicode_linebreak::linebreaks;
use unicode_properties::emoji::{
    EmojiStatus, UnicodeEmoji, is_emoji_presentation_selector, is_regional_indicator,
    is_text_presentation_selector, is_zwj,
};
use unicode_segmentation::UnicodeSegmentation;

pub use ink_protocol::SmolStr;

mod fonts;
mod canvas;
mod capture;
pub use capture::CaptureKind;
pub use fonts::{FontRole, MAX_THEME_FONTS, font_bytes, font_for_character_in, primary_font, set_theme_fonts, tabular_digit_width_in, text_graphemes, text_width_in, visual_text, visual_text_width_in};

mod theme;
mod input;
pub use input::{InputAppearance, InputStyle, InputColour, InputBorder, InputFont, InputAlign, HitInsets};
use input::{InputOptions, input_alignment_offset};
/// The theme file description, for the compiler.
pub mod theme_source {
    pub use crate::theme::{SourceRow, SourceScrollBar, SourceSubtitle, SourceColours, SourceFonts, SourceHeader, SourceImage, SourceRoles, SourceSection, SourceSpacing, SourceText, SourceToggle};
}
pub use theme::{CutoutMode, FieldLayout, ScrollBarVisibility, HeaderLayout, Palette, Roles, SelectionStyle, Theme, ThemeBundle, ThemeSource, ToggleStyle, parse_colour};

mod icon_assets;
mod list;
mod masks;
mod native_editor;
mod react;

pub use icon_assets::ReactIcon;
pub use react::{ReactCommit, ReactTree};

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
const TEXT_INPUT_AFFIX_SIZE: f32 = 26.0;
const TEXT_INPUT_AFFIX_GAP: f32 = 8.0;
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
/// A field's image and the space it takes with its gap, the same as a row's.
const FIELD_IMAGE_SIZE: f32 = 50.0;
const FIELD_IMAGE_SLOT: f32 = 65.0;
const CONTENT_INSET_START: f32 = 37.0;
const CONTENT_TOP: f32 = 14.0;
const CONTENT_BOTTOM: f32 = 20.0;
const CONTENT_GAP: f32 = 47.0;
const SCROLL_OVERSCAN: f32 = 2.0;
const DECODED_IMAGE_CACHE_BYTES: usize = 8 * 1024 * 1024;
const DECODED_IMAGE_CACHE_ENTRIES: usize = 256;
const HEADER_HEIGHT: f32 = 50.0;
const HEADER_TEXT_SIZE: f32 = 20.0;
const LARGE_HEADER_TEXT_SIZE: f32 = 46.0;
const SECTION_TEXT_SIZE: f32 = 17.0;
const SCREEN_NOTE_SIZE: f32 = 14.0;
const LARGE_HEADER_SUBTITLE_SIZE: f32 = 19.0;
const LARGE_HEADER_TOP: f32 = 14.0;
const LARGE_HEADER_BOTTOM: f32 = 22.0;
const HEADER_HORIZONTAL_INSET: f32 = 22.0;
const HEADER_BUTTON_SIZE: f32 = 32.0;
const HEADER_BACK_ICON_SIZE: f32 = 28.0;
/// Space between neighbouring end actions in a header, as a share of the header icon size.
const HEADER_ACTION_GAP: f32 = 0.75;
const HEADER_BACK_OFFSET_X: f32 = -7.0;
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
const ROW_END_ICON_SIZE: f32 = 28.0;
const SWITCH_WIDTH: f32 = 76.0;
const SWITCH_HEIGHT: f32 = 42.0;
const SWITCH_KNOB_INSET: f32 = 4.0;
const SCROLL_TRACK_END: f32 = 34.0;
const SCROLL_TRACK_WIDTH: f32 = 1.0;
const SCROLL_CONTENT_INSET_END: f32 = SCROLL_TRACK_END * 2.0 - SCROLL_TRACK_WIDTH;
const SCROLL_THUMB_WIDTH: f32 = 5.0;
const SCROLL_BAR_HOLD: f32 = 0.8;
const SCROLL_BAR_FADE: f32 = 0.3;
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
    // Event arguments can change while the same control is held.
    event_target: Option<(usize, &'static str)>,
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
            event_target: None,
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

/// How a completed image changed the scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageUpdate {
    Unchanged,
    /// The image is on screen and should be presented promptly.
    Visible,
    /// The image only landed in the scroll overscan; any frame can upload it.
    Offscreen,
}

impl ImageUpdate {
    pub const fn changed(self) -> bool {
        !matches!(self, Self::Unchanged)
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
    Update(String),
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
    CanvasRange { id: usize },
    Seek { id: usize, left: f32, width: f32, duration: f32 },
    ClearInput {
        state: StateId,
    },
    Native {
        operation: Arc<NativeOperation>,
    },
    FocusTextInput {
        state: StateId,
        action: TextInputAction,
    },
    Back,
}

impl Action {
    fn changes_layout(&self) -> bool {
        match self {
            Self::ClearInput { .. } | Self::FocusTextInput { .. } => true,
            Self::Native { .. } | Self::Back | Self::Seek { .. } | Self::CanvasRange { .. } => false,
        }
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

pub trait ImageAnimation: Send {
    /// Returns the next frame once it is due. Frames are shared with the renderer without copying.
    fn advance(&mut self) -> Option<Arc<[u8]>>;
    fn finished(&self) -> bool;
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

fn visible_image(scene: &Scene, id: u64) -> bool {
    scene.images.iter().any(|run| {
        if run.image.id() != id { return false; }
        let mut rect = run.rect;
        if run.scrolling { rect.y += scene.scroll_origin - scene.scroll_offset; }
        let visible = rect.intersection(run.clip);
        visible.width > 0.0 && visible.height > 0.0
    })
}

// Overscanned images are laid out ahead of scrolling but are not yet on screen.
fn pending_image_on_screen(scene: &Scene, image: &PendingImage) -> bool {
    let mut rect = image.rect;
    if image.scrolling { rect.y += scene.scroll_origin - scene.scroll_offset; }
    let mut visible = rect.intersection(image.clip);
    if let (true, Some(clip)) = (image.scrolling, scene.scroll_clip) { visible = visible.intersection(clip); }
    visible.width > 0.0 && visible.height > 0.0
}

fn image_content_rect(image: &ImageData, mut rect: Rect, fit: ImageFit, rotation: u16) -> Rect {
    if rect.width <= 0.0 || rect.height <= 0.0 || image.height() == 0 {
        return rect;
    }
    let image_aspect = if rotation == 90 || rotation == 270 {
        image.height() as f32 / image.width() as f32
    } else { image.width() as f32 / image.height() as f32 };
    let rect_aspect = rect.width / rect.height;
    match fit {
        ImageFit::Cover if image_aspect > rect_aspect => {
            let width = rect.height * image_aspect;
            rect.x += (rect.width - width) / 2.0;
            rect.width = width;
            rect
        }
        ImageFit::Cover => {
            let height = rect.width / image_aspect;
            rect.y += (rect.height - height) / 2.0;
            rect.height = height;
            rect
        }
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
    Canvas { width: f32, height: f32, drawings: Vec<canvas::Drawing> },
    MediaGridRow { children: Vec<Node> },
    MediaCell { source: ImageSource, selected: bool, video: bool, check: Mask, play: Mask, action: Option<Action> },
    Message { children: Vec<Node>, outgoing: bool },
    MessageQuote { children: Vec<Node> },
    LinkPreview { children: Vec<Node> },
    ConversationComposer { children: Vec<Node> },
    PlayingLayout { children: Vec<Node>, centred: bool, hide_controls: bool, bleed: bool },
    Pressable { children: Vec<Node>, action: Option<Action>, long_action: Option<Action>, selected: bool, haptic: bool },
    PlayingTransport { children: Vec<Node>, loading: bool },
    PlayingLabel { text: String, size: f32 },
    /// `themed` titles belong to a `Row`, so the theme's row size replaces `size`.
    RowTitle { children: Vec<Node>, text: String, size: f32, max_lines: Option<u32>, themed: bool },
    PlayingProgress { position: f32, duration: f32, playing: bool, seek: bool, show_times: bool, clock: Option<u64>, speed: f32 },
    PitchIndicator { cents: Option<f32> },
    CaptureReadout { controller: u64, kind: CaptureKind },
    Row {
        children: Vec<Node>,
        has_image: bool,
        action: Option<Action>,
        long_action: Option<Action>,
        /// An icon at the end of the row, with its own action when it is a separate button.
        end: Option<(Mask, Option<Action>)>,
    },
    ReactList {
        children: Vec<Node>,
        start: usize,
        keys: Arc<[String]>,
        content_versions: list::ListVersions,
        revision: u64,
        gap: f32,
        follow_end: bool,
    },
    Screen(Box<ScreenNode>),
    Stack {
        children: Vec<Node>,
        axis: Axis,
        gap: Option<f32>,
        /// Use one of the theme's gaps instead of `gap`.
        theme_gap: Option<ThemeGap>,
        align: Alignment,
        justify: Justification,
    },
    SectionTitle {
        title: String,
        upper: String,
    },
    /// A row's subtitle, sized and toned by the theme's `subtitle` setting.
    RowSubtitle {
        text: SmolStr,
        wrap: bool,
    },
    Text {
        links: Vec<(std::ops::Range<usize>, Action)>,
        width: Option<f32>,
        text: SmolStr,
        font_size: Option<f32>,
        align: TextAlign,
        max_lines: Option<u32>,
        tabular_numbers: bool,
    },
    TextInput {
        placeholder: String,
        state: StateId,
        action: TextInputAction,
        auto_focus: bool,
        numeric: bool,
        prefix: String,
        suffix: String,
        clear: Mask,
        options: Box<InputOptions>,
    },
    Button {
        label: String,
        icon: Option<Mask>,
        underline: bool,
        /// One of a set of options, selected or not, like those in `SettingsChoices`.
        selectable: bool,
        action: Option<Action>,
        long_action: Option<Action>,
    },
    Field {
        label: String,
        value: String,
        action: Option<Action>,
        disabled: bool,
        /// A picture before the label, such as an app's icon.
        image: Option<Box<Node>>,
    },
    Icon {
        mask: Mask,
        size: f32,
        tone: Tone,
        bounds: Option<Rect>,
    },
    Image {
        avatar: Option<bool>,
        rotation: Option<u16>,
        background: Option<Colour>,
        crop: Option<ImageCrop>,
        crop_events: bool,
        horizontal_guides: Vec<f32>,
        overlay: Option<(ImageSource, Rect)>,
        looping: bool,
        source: ImageSource,
        preload: Vec<ImageSource>,
        retain_while_loading: bool,
        fallback: Option<ImageAsset>,
        bleed: bool,
        fill_width: bool,
        zoomable: bool,
        width: f32,
        height: f32,
        fit: ImageFit,
    },
    CameraPreview {
        controller: ControllerId,
        kind: CameraPreviewKind,
    },
    VideoView { controller: Option<ControllerId>, loading: bool },
    MapView {
        controller: ControllerId,
    },
    Toggle {
        label: String,
        subtitle: Option<String>,
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

#[derive(Clone, Debug, PartialEq)]
struct ScreenNode {
    children: Vec<Node>,
    title: Option<String>,
    subtitle: Option<String>,
    /// Small muted text pinned to the bottom of the screen.
    note: Option<String>,
    /// A named theme variant for this screen.
    theme: Option<String>,
    debug_hitboxes: bool,
    background: Option<Colour>,
    centred: bool,
    footer: Option<(String, Option<Action>)>,
    pinned_header: bool,
    pinned_footer: bool,
    wide: bool,
    bottom_inset: bool,
    left_action: Option<(Mask, Action)>,
    /// End actions in reading order; the last sits at the end edge.
    right_actions: Vec<(Mask, Action)>,
    media_picker: bool,
}

impl Node {
    pub fn screen(children: Vec<Self>, title: Option<String>, centred: bool) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Screen(Box::new(ScreenNode {
                children,
                title,
                subtitle: None,
                note: None,
                theme: None,
                debug_hitboxes: false,
                background: None,
                centred,
                footer: None,
                pinned_header: false,
                pinned_footer: false,
                wide: false,
                bottom_inset: true,
                left_action: None,
                right_actions: Vec::new(),
                media_picker: false,
            })),
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
                theme_gap: None,
                align,
                justify,
            },
        }
    }

    pub fn text(
        text: SmolStr,
        font_size: Option<f32>,
        align: TextAlign,
        max_lines: Option<u32>,
        tabular_numbers: bool,
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Text {
                links: Vec::new(),
                width: None,
                text,
                font_size,
                align,
                max_lines,
                tabular_numbers,
            },
        }
    }

    pub fn text_input(
        placeholder: impl Into<String>,
        state: StateId,
        action: TextInputAction,
        auto_focus: bool,
        numeric: bool,
        prefix: String,
        suffix: String,
        clear: Mask,
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::TextInput {
                placeholder: placeholder.into(),
                state,
                action,
                auto_focus,
                numeric,
                prefix,
                suffix,
                clear,
                options: Box::default(),
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
                selectable: underline,
                action,
                long_action: None,
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
                disabled: false,
                image: None,
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
                avatar: None,
                rotation: None,
                background: None,
                crop: None,
                crop_events: false,
                horizontal_guides: Vec::new(),
                overlay: None,
                looping: false,
                preload: Vec::new(),
                retain_while_loading: false,
                source,
                fallback,
                bleed,
                fill_width: false,
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

    pub fn to_linear(self) -> Self {
        let channel = |value: f32| if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        };
        Self { red: channel(self.red), green: channel(self.green), blue: channel(self.blue), ..self }
    }

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

/// A straight stroke with round ends; consecutive lines form a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeGap {
    Content,
    Section,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub width: f32,
    pub clip: Rect,
    pub colour: Colour,
    pub scrolling: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub font: FontRole,
    pub text: SmolStr,
    pub rect: Rect,
    pub clip: Rect,
    pub font_size: f32,
    pub colour: Colour,
    pub align: TextAlign,
    pub tabular_numbers: bool,
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
    pub avatar: Option<bool>,
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
    pub rotation: u16,
    pub scale: f32,
    pub translation_x: f32,
    pub translation_y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageCrop {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for ImageTransform {
    fn default() -> Self {
        Self {
            rotation: 0,
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
    /// Whether the thin full-height track line is drawn.
    pub track_visible: bool,
    pub opacity: f32,
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
        self.touch_rect().contains(x, y)
    }

    fn touch_rect(self) -> Rect {
        let touch_width = self.thumb_width * SCROLL_THUMB_TOUCH_MULTIPLIER;
        Rect {
            x: self.track.x - (touch_width - self.track.width) / 2.0,
            width: touch_width,
            ..self.track
        }
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
    pub hitboxes: Vec<Hitbox>,
    pub lines: Vec<Line>,
    pub text: Vec<TextRun>,
    pub masks: Vec<MaskRun>,
    pub background_masks: Vec<MaskRun>,
    pub images: Vec<ImageRun>,
    pub preloaded_images: Vec<ImageData>,
    pub camera_portal: Option<CameraPortal>,
    pub map_portal: Option<MapPortal>,
    pub video_portal: Option<MapPortal>,
    pub scroll_origin: f32,
    pub scroll_offset: f32,
    pub scroll_max: f32,
    pub scroll_clip: Option<Rect>,
    pub scroll_bar: Option<ScrollBar>,
    pub palette: Palette,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hitbox {
    pub rect: Rect,
    pub scrolling: bool,
}

impl Scene {
    /// Resolves Ink's role colours (white, black and muted) through the theme palette. Other
    /// colours are inverted in light mode.
    pub fn colour(&self, colour: Colour) -> Colour {
        let roles = if self.light { &self.palette.light } else { &self.palette.dark };
        let same = |role: Colour| colour.red == role.red && colour.green == role.green && colour.blue == role.blue;
        let themed = |role: Colour| Colour { alpha: role.alpha * colour.alpha, ..role };
        if same(Colour::WHITE) {
            themed(roles.foreground)
        } else if same(Colour::BLACK) {
            themed(roles.background)
        } else if same(Colour::MUTED) {
            themed(roles.muted)
        } else if self.light {
            invert_colour(colour)
        } else {
            colour
        }
    }

    /// A colour the app chose, which themes leave unchanged. Light mode still inverts it.
    pub fn explicit_colour(&self, colour: Colour) -> Colour {
        if self.light { invert_colour(colour) } else { colour }
    }

    /// The scroll bar's colour at `alpha`: the theme's scroll bar colour, or the foreground.
    pub fn scroll_bar_colour(&self, alpha: f32) -> Colour {
        let roles = if self.light { &self.palette.light } else { &self.palette.dark };
        match roles.scroll_bar {
            Some(colour) => Colour { alpha: colour.alpha * alpha, ..colour },
            None => self.colour(Colour { alpha, ..Colour::WHITE }),
        }
    }

    pub fn background(&self) -> Colour {
        if self.light { self.palette.light.background } else { self.palette.dark.background }
    }
}

#[derive(Clone, Debug)]
struct HitRegion {
    rect: Rect,
    action: Action,
    long_action: Option<Action>,
    drag_group: Option<NodeIdentity>,
    drag_action: Option<Action>,
    drag_start: Option<Action>,
    drag_end: Option<Action>,
    scrolling: bool,
    preserve_input: bool,
    haptic: bool,
    target: PressTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PressTarget {
    node: NodeIdentity,
    role: PressRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PressRole {
    CanvasRange,
    Event(&'static str),
    Native,
    Seek,
    Focus,
    Clear,
    Back,
}

#[derive(Clone, Copy, Debug)]
struct TextInputLayout {
    node: NodeIdentity,
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
    /// The native scene changed; dispatching a callback alone does not redraw it.
    pub changed: bool,
    pub activated: bool,
    pub haptic: bool,
    pub captured: bool,
}

impl PointerOutcome {
    const fn changed(changed: bool) -> Self {
        Self {
            changed,
            activated: false,
            haptic: false,
            captured: false,
        }
    }

    const fn activated(changed: bool) -> Self {
        Self {
            changed,
            activated: changed,
            haptic: changed,
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
    target: Option<PressTarget>,
    start_x: f32,
    start_y: f32,
    start_offset: f32,
    state: EdgeBackState,
}

#[derive(Clone, Copy, Debug)]
struct ContentPointer {
    target: Option<PressTarget>,
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
    CanvasRange(canvas::RangePointer),
    CanvasDrag { group: NodeIdentity, previous: (f32, f32), target: Option<PressTarget> },
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
    crop: Option<ImageCrop>,
    crop_events: bool,
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
    looping: bool,
    module: String,
    url: String,
    width: u32,
    height: u32,
    fit: ImageFit,
}

#[derive(Clone)]
enum RemoteImageState {
    Loading { request_id: u64 },
    Ready { image: RemoteImage, last_used: u64 },
    Failed,
}

#[derive(Clone)]
struct PendingImage {
    avatar: Option<bool>,
    key: RemoteImageKey,
    index: usize,
    rect: Rect,
    clip: Rect,
    fit: ImageFit,
    scrolling: bool,
    needs_layout: bool,
}

#[derive(Clone)]
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

#[derive(Clone)]
struct Marquee {
    text: String,
    started: Instant,
    run: usize,
    x: f32,
    distance: f32,
    speed: f32,
    visible: bool,
}

fn playback_time(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[derive(Clone)]
struct PlaybackClock {
    position: f32,
    duration: f32,
    playing: bool,
    speed: f32,
    started: Instant,
}

#[derive(Clone)]
struct PlaybackProgress {
    clock: Option<u64>,
    speed: f32,
    position: f32,
    duration: f32,
    playing: bool,
    looping: bool,
    started: Instant,
    quad: usize,
    width: f32,
    visible: bool,
    elapsed_label: Option<(usize, u64)>,
}

impl PlaybackProgress {
    fn current(&self) -> f32 {
        let elapsed = if self.playing { self.started.elapsed().as_secs_f32() * self.speed } else { 0.0 };
        let position = self.position + elapsed;
        if self.looping && self.playing && self.duration > 0.0 {
            position.rem_euclid(self.duration)
        } else {
            position.min(self.duration)
        }
    }

    fn animating(&self) -> bool {
        self.visible && self.playing && self.duration > 0.0
            && (self.looping || self.current() < self.duration)
    }

    /// Time until the bar's shaded edge, the elapsed label or the loop point next changes.
    fn next_change(&self, left: f32) -> Duration {
        let current = self.current();
        let mut media = self.duration - current;
        if self.elapsed_label.is_some() {
            media = media.min(current.max(0.0).floor() + 1.0 - current);
        }
        if self.width > 0.0 {
            let right = left + self.width * (current / self.duration).clamp(0.0, 1.0);
            let next = ((right * PROGRESS_EDGE_STEPS).floor() + 1.0) / PROGRESS_EDGE_STEPS;
            media = media.min((next - right) * self.duration / self.width);
        }
        // Land just past the boundary so the frame sees the change.
        Duration::from_secs_f32(media.max(0.0) / self.speed) + Duration::from_millis(1)
    }
}

/// Shading steps for the progress bar's partially covered pixel.
const PROGRESS_EDGE_STEPS: f32 = 4.0;
/// Clock corrections larger than this, in seconds, are seeks that should draw at once.
const PLAYBACK_CLOCK_JUMP: f32 = 0.25;

pub struct Engine {
    root: Node,
    marquees: HashMap<NodeIdentity, Marquee>,
    playback_progress: HashMap<NodeIdentity, PlaybackProgress>,
    playback_clocks: HashMap<u64, PlaybackClock>,
    capture_readings: capture::CaptureReadings,
    state: Vec<StateValue>,
    viewport: Viewport,
    keyboard_inset: u32,
    display_cutouts: Vec<Rect>,
    native_editor_state: Option<StateId>,
    native_editor_lines: Option<(StateId, String, usize)>,
    scene: Scene,
    fixed_text_runs: rustc_hash::FxHashMap<NodeIdentity, FixedTextRun>,
    react_list_positions: HashMap<usize, f32>,
    list_metrics: HashMap<usize, list::ListMetrics>,
    hit_regions: Vec<HitRegion>,
    hit_slops: HashMap<NodeIdentity, HitInsets>,
    action_hit_slops: HashMap<(NodeIdentity, &'static str), HitInsets>,
    debug_hitboxes: bool,
    screen_debug_hitboxes: bool,
    text_inputs: Vec<TextInputLayout>,
    text_input_scroll_offsets: HashMap<StateId, f32>,
    clip: Rect,
    scrolling: bool,
    scroll_origin: f32,
    scroll_offset: f32,
    scroll_max: f32,
    pointers: HashMap<i32, Pointer>,
    gesture_owner: Option<i32>,
    layout_owner: NodeIdentity,
    focused_input: Option<StateId>,
    focused_input_action: TextInputAction,
    focused_input_cursor: usize,
    auto_focus_node: Option<NodeIdentity>,
    queued_requests: VecDeque<QueuedRequest>,
    in_flight_requests: HashMap<u64, PendingRequest>,
    remote_images: HashMap<RemoteImageKey, RemoteImageState>,
    image_animations: HashMap<RemoteImageKey, Box<dyn ImageAnimation>>,
    pending_images: Vec<PendingImage>,
    preloaded_images: BTreeSet<RemoteImageKey>,
    retained_images: HashMap<NodeIdentity, ImageData>,
    visible_retained_images: BTreeSet<NodeIdentity>,
    visible_images: BTreeSet<RemoteImageKey>,
    blocking_images: BTreeSet<RemoteImageKey>,
    image_screens: HashMap<NodeIdentity, Option<Instant>>,
    visible_image_screens: BTreeSet<NodeIdentity>,
    image_zooms: HashMap<NodeIdentity, ImageZoomState>,
    canvas_ranges: HashMap<NodeIdentity, canvas::RangeState>,
    visible_zoom_images: BTreeSet<NodeIdentity>,
    image_pinch: Option<ImagePinch>,
    last_native_request: Option<NativeRequest>,
    next_request_id: u64,
    next_image_generation: u64,
    back_icon: Option<Mask>,
    navigation_handler: Option<(Mask, NativeOperation)>,
    /// The theme of the screen being laid out, or of the visible screen after layout.
    theme: Arc<Theme>,
    /// The app's theme, including its variants.
    base_theme: Arc<Theme>,
    /// When the screen last scrolled, while a scrolling-only scroll bar is showing.
    scroll_activity: Option<Instant>,
    /// Empty space to the right of each icon's drawn shape, as a fraction of its width.
    icon_trailing_space: HashMap<u64, f32>,
    switch_masks: Option<(u32, u32, Mask, Mask)>,
    input_masks: HashMap<(u32, u32, u32, u32), Mask>,
    wrapped_text: HashMap<TextWrapKey, (WrappedLines, bool), ahash::RandomState>,
    #[cfg(feature = "perf")]
    perf: CorePerfMetrics,
    #[cfg(feature = "perf")]
    measure_depth: u32,
}

#[derive(Clone)]
struct FixedTextRun {
    run: usize,
    width: f32,
    font_size: Option<f32>,
    align: TextAlign,
    tabular_numbers: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Viewport {
    width: u32,
    height: u32,
    scale: f32,
}

impl Engine {
    pub fn reset_session(&mut self) {
        let viewport = self.viewport;
        let display_cutouts = self.display_cutouts.clone();
        let light = self.scene.light;
        let revision = self.scene.revision.wrapping_add(1);
        let requests = self.next_request_id;
        let images = self.next_image_generation;
        let theme = self.base_theme.clone();
        *self = Self::new();
        self.apply_theme(theme);
        self.viewport = viewport;
        self.display_cutouts = display_cutouts;
        self.scene.light = light;
        self.scene.revision = revision;
        self.next_request_id = requests;
        self.next_image_generation = images;
        self.rebuild_viewport();
    }

    pub fn new() -> Self {
        let theme = Arc::<Theme>::default();
        Self {
            root: Node::screen(vec![], None, false),
            state: Vec::new(),
            viewport: Viewport::default(),
            keyboard_inset: 0,
            display_cutouts: Vec::new(),
            native_editor_state: None,
            native_editor_lines: None,
            scene: Scene::default(),
            fixed_text_runs: rustc_hash::FxHashMap::default(),
            react_list_positions: HashMap::new(),
            list_metrics: HashMap::new(),
            hit_regions: Vec::new(),
            hit_slops: HashMap::new(),
            action_hit_slops: HashMap::new(),
            debug_hitboxes: false,
            screen_debug_hitboxes: false,
            text_inputs: Vec::new(),
            text_input_scroll_offsets: HashMap::new(),
            clip: Rect::default(),
            scrolling: false,
            scroll_origin: 0.0,
            scroll_offset: 0.0,
            scroll_max: 0.0,
            pointers: HashMap::new(),
            gesture_owner: None,
            layout_owner: NodeIdentity(0),
            focused_input: None,
            focused_input_action: TextInputAction::default(),
            focused_input_cursor: 0,
            auto_focus_node: None,
            queued_requests: VecDeque::new(),
            in_flight_requests: HashMap::new(),
            remote_images: HashMap::new(),
            image_animations: HashMap::new(),
            pending_images: Vec::new(),
            preloaded_images: BTreeSet::new(),
            retained_images: HashMap::new(),
            visible_retained_images: BTreeSet::new(),
            visible_images: BTreeSet::new(),
            blocking_images: BTreeSet::new(),
            image_screens: HashMap::new(),
            visible_image_screens: BTreeSet::new(),
            marquees: HashMap::new(),
            playback_progress: HashMap::new(),
            playback_clocks: HashMap::new(),
            capture_readings: capture::CaptureReadings::default(),
            image_zooms: HashMap::new(),
            canvas_ranges: HashMap::new(),
            visible_zoom_images: BTreeSet::new(),
            image_pinch: None,
            last_native_request: None,
            next_request_id: 1,
            next_image_generation: 1,
            back_icon: None,
            navigation_handler: None,
            base_theme: theme.clone(),
            theme,
            scroll_activity: None,
            icon_trailing_space: HashMap::new(),
            switch_masks: None,
            input_masks: HashMap::new(),
            wrapped_text: HashMap::default(),
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

    pub fn screen_images_ready(&self) -> bool {
        self.image_wait_remaining_ms() == 0
    }

    pub fn image_wait_remaining_ms(&self) -> u64 {
        if !self.blocking_images.iter().any(|key| {
            matches!(self.remote_images.get(key), Some(RemoteImageState::Loading { .. }))
        }) { return 0; }
        self.image_screens.values().flatten()
            .map(|started| 1000_u64.saturating_sub(started.elapsed().as_millis() as u64))
            .max().unwrap_or(0)
    }

    pub fn camera_review_ready(&self) -> bool {
        !self.visible_images.iter().any(|key| {
            key.module == "camera"
                && key.url.starts_with("ink-camera-review://")
                && matches!(
                    self.remote_images.get(key),
                    Some(RemoteImageState::Loading { .. })
                )
        })
    }

    pub fn image_request_target(&self, request_id: u64) -> Option<(u32, u32, ImageFit, bool)> {
        let pending = self.in_flight_requests.get(&request_id)?;
        let RequestOwner::Image(key) = &pending.owner else {
            return None;
        };
        Some((key.width, key.height, key.fit, key.looping))
    }

    pub fn complete_native_image(
        &mut self,
        request_id: u64,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
        animation: Option<Box<dyn ImageAnimation>>,
    ) -> ImageUpdate {
        let Some(pending) = self.in_flight_requests.remove(&request_id) else {
            return ImageUpdate::Unchanged;
        };
        let RequestOwner::Image(key) = pending.owner else {
            return ImageUpdate::Unchanged;
        };
        if !matches!(
            self.remote_images.get(&key),
            Some(RemoteImageState::Loading { request_id: active }) if *active == request_id
        ) || width == 0
            || height == 0
            || pixels.len() != width as usize * height as usize * 4
        {
            return ImageUpdate::Unchanged;
        }
        if let Some(animation) = animation { self.image_animations.insert(key.clone(), animation); }
        let id = image_id(&key);
        let generation = self.next_image_generation;
        self.next_image_generation = self.next_image_generation.wrapping_add(1).max(1);
        let image = RemoteImage { id, generation, width, height, pixels: pixels.into() };
        self.remote_images.insert(
            key.clone(),
            RemoteImageState::Ready {
                image: image.clone(),
                last_used: self.scene.revision,
            },
        );
        if self.preloaded_images.contains(&key) {
            self.scene.preloaded_images.push(ImageData::Remote(image.clone()));
        }
        if key.module == "barcode" || self.pending_images.iter().any(|slot| slot.key == key && slot.needs_layout) {
            self.rebuild_scene();
        } else {
            let mut slots = 0;
            let mut on_screen = false;
            let mut index = 0;
            while index < self.pending_images.len() {
                if self.pending_images[index].key != key {
                    index += 1;
                    continue;
                }
                let slot = self.pending_images.remove(index);
                slots += 1;
                on_screen |= pending_image_on_screen(&self.scene, &slot);
                self.scene.images.insert(slot.index, ImageRun {
                    avatar: slot.avatar,
                    image: ImageData::Remote(image.clone()),
                    zoom_id: None,
                    rect: slot.rect,
                    clip: slot.clip,
                    fit: slot.fit,
                    scrolling: slot.scrolling,
                    transform: ImageTransform::default(),
                });
                // Later slots keep their paint order even when downloads finish out of order.
                for later in &mut self.pending_images[index..] { later.index += 1; }
            }
            self.scene.image_revision = self.scene.image_revision.wrapping_add(1);
            if slots > 0 && !on_screen && !self.preloaded_images.contains(&key) { return ImageUpdate::Offscreen; }
        }
        ImageUpdate::Visible
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
            let source: serde_json::Value =
                serde_json::from_slice(key.url.as_bytes()).unwrap_or_default();
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
        let mut cancelled = Vec::new();
        let mut bytes = 0;
        let mut ready_count = 0;
        let mut unused = Vec::new();
        self.remote_images.retain(|key, state| {
            let visible = self.visible_images.contains(key);
            match state {
                RemoteImageState::Ready { image, last_used } => {
                    let size = image.pixels.len() * if self.image_animations.contains_key(key) { 3 } else { 1 };
                    bytes += size;
                    ready_count += 1;
                    if visible {
                        *last_used = self.scene.revision;
                    } else {
                        unused.push((key.clone(), *last_used, size));
                    }
                    true
                }
                RemoteImageState::Loading { request_id } => {
                    if !visible { cancelled.push(*request_id); }
                    visible
                }
                RemoteImageState::Failed => visible,
            }
        });
        for request_id in cancelled {
            self.cancel_request(request_id);
        }
        if bytes <= DECODED_IMAGE_CACHE_BYTES && ready_count <= DECODED_IMAGE_CACHE_ENTRIES {
            return;
        }
        unused.sort_unstable_by_key(|(_, last_used, _)| *last_used);
        for (key, _, size) in unused {
            if bytes <= DECODED_IMAGE_CACHE_BYTES && ready_count <= DECODED_IMAGE_CACHE_ENTRIES {
                break;
            }
            self.remote_images.remove(&key);
            self.image_animations.remove(&key);
            bytes -= size;
            ready_count -= 1;
        }
    }

    pub fn set_viewport(&mut self, width: u32, height: u32) -> bool {
        let viewport = Viewport {
            width,
            height,
            scale: if width > 0 {
                width as f32 / LP3_REFERENCE_WIDTH * LP3_REFERENCE_SCALE * self.theme.scale
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

    pub fn set_viewport_scale(&mut self, scale: f32) -> bool {
        if !scale.is_finite() || scale <= 0.0 || self.viewport.scale == scale {
            return false;
        }
        self.viewport.scale = scale;
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

    /// Non-functional display areas, in surface pixels.
    pub fn set_display_cutouts(&mut self, cutouts: Vec<Rect>) -> bool {
        if self.display_cutouts == cutouts {
            return false;
        }
        self.display_cutouts = cutouts;
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
        self.snapshot_canvas_ranges();
        if let Some(Action::CanvasRange { id }) = self.hit_region_at(x, y).map(|region| &region.action) {
            let range = NodeIdentity(*id);
            self.begin_canvas_range(-1, range, x, y);
            let outcome = self.pointer_up(-1, x, y);
            return outcome.changed || outcome.activated;
        }
        // Snapshot gestures into the same FIFO as the following press event. The
        // confirmation handler therefore always sees the latest native crop.
        let crops: Vec<_> = self.image_zooms.keys().copied().collect();
        for image in crops { self.emit_image_crop(image); }
        let action = self.hit_region_at(x, y)
            .map(|region| (match region.action.clone() {
                Action::Seek { id, left, width, duration } => react::event(id, "onSeek", vec![serde_json::json!(((x - left) / width).clamp(0.0, 1.0) * duration)]),
                action => action,
            }, region.preserve_input));
        let preserve_input = action.as_ref().is_some_and(|(_, preserve)| *preserve);
        let action = action.map(|(action, _)| action);
        let blurred = self.focused_input.is_some_and(|state| !self.keeps_focus(state))
            && !preserve_input
            && !matches!(
                action.as_ref(),
                Some(Action::FocusTextInput { .. } | Action::ClearInput { .. } | Action::Back)
            );
        if blurred {
            self.focused_input = None;
        }

        let changes_layout = action.as_ref().is_some_and(Action::changes_layout);
        let changed = action.is_some_and(|action| self.apply(action));
        if !changed && !blurred {
            return false;
        }

        // Native callbacks update the scene when their result arrives.
        if changes_layout || blurred {
            self.relayout_scene();
        }
        true
    }

    pub fn pointer_down(&mut self, id: i32, x: f32, y: f32) -> PointerOutcome {
        if !self.screen_images_ready()
            && !matches!(self.hit_region_at(x, y).map(|region| &region.action), Some(Action::Back))
        {
            return PointerOutcome::default();
        }
        if self.gesture_owner.is_some() || self.image_pinch.is_some() {
            return PointerOutcome::default();
        }
        let target = self.hit_region_at(x, y).map(|region| region.target);
        if let Some(scroll_bar) = self
            .scene
            .scroll_bar
            .filter(|scroll_bar| scroll_bar.contains_touch(x, y))
        {
            let thumb = scroll_bar.thumb_rect(self.scroll_offset, self.scroll_max);
            self.pointers.insert(id, if y >= thumb.y && y <= thumb.y + thumb.height {
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
            self.claim_pointer(id);
            return PointerOutcome::default().captured();
        }

        if self.navigation_handler.is_some() && x <= self.scaled(BACK_SWIPE_EDGE_WIDTH) {
            self.pointers.insert(id, Pointer::EdgeBack(EdgeBackPointer {
                target,
                start_x: x,
                start_y: y,
                start_offset: self.scroll_offset,
                state: EdgeBackState::Pending,
            }));
            return PointerOutcome::default();
        }

        if let Some(Action::CanvasRange { id: range }) = self.hit_region_at(x, y).map(|region| &region.action) {
            return self.begin_canvas_range(id, NodeIdentity(*range), x, y);
        }

        if let Some(region) = self.hit_region_at(x, y).cloned()
            && let Some(group) = region.drag_group
            && let Some(action) = region.drag_start
        {
            self.pointers.insert(id, Pointer::CanvasDrag { group, previous: (x, y), target: Some(region.target) });
            self.claim_pointer(id);
            let activated = self.apply(action);
            return PointerOutcome { activated, haptic: activated && region.haptic, ..PointerOutcome::default().captured() };
        }

        if let Some(image) = self.zoomable_image_at(x, y)
            && let Some(zoom) = self.image_zooms.get(&image).copied()
            && (zoom.transform.scale > 1.0 || zoom.crop.is_some())
        {
            self.pointers.insert(id, Pointer::ImagePan(ImagePanPointer {
                image,
                start_x: x,
                start_y: y,
                translation_x: zoom.transform.translation_x,
                translation_y: zoom.transform.translation_y,
            }));
            return PointerOutcome::default().captured();
        }

        if let Some(input) = self.text_input_at(x, y) {
            self.pointers.insert(id, Pointer::TextInput(TextInputPointer {
                input,
                start_x: x,
                start_y: y,
                content_offset: self.scroll_offset,
                dragging: false,
            }));
            return PointerOutcome::default();
        }

        self.pointers.insert(id, Pointer::Content(ContentPointer {
            target,
            start_x: x,
            start_y: y,
            start_offset: self.scroll_offset,
            dragging: false,
            cancelled: false,
        }));
        PointerOutcome::default()
    }

    pub fn pointer_move(&mut self, id: i32, x: f32, y: f32) -> PointerOutcome {
        let tap_slop = self.scaled(TAP_SLOP);
        let Some(pointer) = self.pointers.get(&id).copied() else {
            return PointerOutcome::default();
        };

        match pointer {
            Pointer::CanvasRange(pointer) => self.move_canvas_range(id, pointer, x, y),
            Pointer::CanvasDrag { group, previous, target } =>
                self.move_canvas_pointer(id, group, previous, target, x, y),
            Pointer::Content(pointer) => self.move_content_pointer(id, pointer, x, y, tap_slop),
            Pointer::TextInput(pointer) => self.move_text_input_pointer(id, pointer, x, y, tap_slop),
            Pointer::ImagePan(pointer) => {
                self.claim_pointer(id);
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
                self.pointers.insert(id, Pointer::ScrollTrack {
                    start_x,
                    start_y,
                    cancelled: cancelled
                        || (x - start_x).abs() > tap_slop
                        || (y - start_y).abs() > tap_slop,
                });
                PointerOutcome::default().captured()
            }
            Pointer::EdgeBack(pointer) => self.move_edge_back_pointer(id, pointer, x, y, tap_slop),
        }
    }

    pub fn pointer_up(&mut self, id: i32, x: f32, y: f32) -> PointerOutcome {
        if self.gesture_owner == Some(id) { self.gesture_owner = None; }
        let Some(pointer) = self.pointers.remove(&id) else {
            return PointerOutcome::default().captured();
        };
        if let Pointer::ImagePan(pointer) = pointer { self.emit_image_crop(pointer.image); }

        match pointer {
            Pointer::CanvasRange(pointer) => self.end_canvas_range(id, pointer, x, y),
            Pointer::Content(ContentPointer {
                dragging: false,
                cancelled: false,
                target,
                ..
            })
            | Pointer::EdgeBack(EdgeBackPointer {
                state: EdgeBackState::Pending,
                target,
                ..
            }) => {
                let region = self.hit_region_at(x, y);
                if region.map(|region| region.target) != target {
                    return PointerOutcome::default();
                }
                let revision = self.scene.revision;
                let haptic = region.is_some_and(|region| region.haptic);
                let activated = self.tap(x, y);
                PointerOutcome {
                    changed: self.scene.revision != revision,
                    activated,
                    haptic: activated && haptic,
                    captured: false,
                }
            }
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
            Pointer::TextInput(pointer) if !pointer.dragging => {
                let input = self.text_inputs.iter().find(|input| input.node == pointer.input.node).copied();
                PointerOutcome::activated(input.is_some_and(|input| {
                    self.hit_region_at(x, y).is_some_and(|region| {
                        region.target == PressTarget { node: input.node, role: PressRole::Focus }
                    }) && self.focus_text_input(input, x, y)
                }))
            }
            Pointer::CanvasDrag { target, .. } => {
                let action = self.hit_regions.iter().find(|region| Some(region.target) == target)
                    .and_then(|region| region.drag_end.clone().map(|action| (action, region.haptic)));
                let haptic = action.as_ref().is_some_and(|(_, haptic)| *haptic);
                let activated = action.is_some_and(|(action, _)| self.apply(action));
                PointerOutcome { activated, haptic: activated && haptic, ..PointerOutcome::default().captured() }
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

    pub fn pointer_long_press(&mut self, id: i32, x: f32, y: f32) -> PointerOutcome {
        if let Some(Pointer::TextInput(pointer)) = self.pointers.get(&id) {
            if !pointer.dragging {
                let input = pointer.input;
                self.focus_text_input(input, x, y);
                self.pointers.clear();
                self.gesture_owner = Some(id);
                return PointerOutcome { changed: true, activated: true, haptic: true, captured: true };
            }
            return PointerOutcome::default();
        }
        let Some(Pointer::Content(ContentPointer { dragging: false, cancelled: false, target: Some(target), .. })) = self.pointers.get(&id) else {
            return PointerOutcome::default();
        };
        let region = self.hit_region_at(x, y).filter(|region| region.target == *target).cloned();
        let Some(region) = region else { return PointerOutcome::default(); };
        let Some(action) = region.long_action else { return PointerOutcome::default(); };
        self.pointers.clear();
        self.gesture_owner = Some(id);
        if let Some(group) = region.drag_group {
            self.pointers.insert(id, Pointer::CanvasDrag { group, previous: (x, y), target: Some(region.target) });
        }
        let changes_layout = action.changes_layout();
        let activated = self.apply(action);
        if changes_layout { self.relayout_scene(); }
        PointerOutcome { changed: changes_layout && activated, activated, haptic: activated, captured: true }
    }

    pub fn pointer_cancel(&mut self) -> PointerOutcome {
        let ranges: Vec<_> = self.pointers.values().filter_map(|pointer| match pointer {
            Pointer::CanvasRange(pointer) => Some(*pointer),
            _ => None,
        }).collect();
        let mut activated = false;
        for pointer in ranges { activated |= self.cancel_canvas_range(pointer); }
        let endings: Vec<_> = self.pointers.values().filter_map(|pointer| match pointer {
            Pointer::CanvasDrag { target, .. } => self.hit_regions.iter()
                .find(|region| Some(region.target) == *target)
                .and_then(|region| region.drag_end.clone()),
            _ => None,
        }).collect();
        self.pointers.clear();
        self.gesture_owner = None;
        for action in endings { activated |= self.apply(action); }
        PointerOutcome { activated, ..PointerOutcome::default().captured() }
    }

    fn claim_pointer(&mut self, id: i32) {
        self.pointers.retain(|pointer_id, _| *pointer_id == id);
        self.gesture_owner = Some(id);
    }

    fn pointer_screen_changed(&mut self) {
        self.pointers.retain(|_, pointer| match *pointer {
            Pointer::Content(ContentPointer { dragging: false, cancelled: false, target: Some(_), .. })
            | Pointer::TextInput(TextInputPointer { dragging: false, .. }) => true,
            Pointer::EdgeBack(EdgeBackPointer { state: EdgeBackState::Pending, target: Some(target), start_x, start_y, start_offset }) => {
                *pointer = Pointer::Content(ContentPointer {
                    target: Some(target), start_x, start_y, start_offset, dragging: false, cancelled: false,
                });
                true
            }
            _ => false,
        });
        self.gesture_owner = None;
    }

    pub fn image_pinch_begin(&mut self, x: f32, y: f32) -> bool {
        let Some(image) = self.zoomable_image_at(x, y) else {
            return false;
        };
        self.pointer_cancel();
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
        if let Some(pinch) = self.image_pinch { self.emit_image_crop(pinch.image); }
        self.image_pinch = None;
    }

    pub fn image_double_tap(&mut self, x: f32, y: f32) -> PointerOutcome {
        let Some(image) = self.zoomable_image_at(x, y) else {
            return PointerOutcome::default();
        };
        let Some(zoom) = self.image_zooms.get(&image).copied() else {
            return PointerOutcome::default();
        };
        self.pointer_cancel();
        self.image_pinch = None;
        let next_scale = if zoom.transform.scale >= IMAGE_MAX_SCALE {
            1.0
        } else {
            (zoom.transform.scale.floor() + 1.0).clamp(2.0, IMAGE_MAX_SCALE)
        };
        let target = if next_scale == 1.0 {
            ImageTransform { rotation: zoom.transform.rotation, ..ImageTransform::default() }
        } else {
            let scroll = if zoom.scrolling {
                self.scroll_offset - self.scroll_origin
            } else {
                0.0
            };
            constrain_image_transform(
                zoom,
                ImageTransform {
                    rotation: zoom.transform.rotation,
                    scale: next_scale,
                    translation_x: x
                        - (x - zoom.transform.translation_x) * next_scale / zoom.transform.scale,
                    translation_y: y + scroll
                        - (y + scroll - zoom.transform.translation_y) * next_scale
                            / zoom.transform.scale,
                },
            )
        };
        let changed = self.set_image_transform(image, target);
        self.emit_image_crop(image);
        PointerOutcome::changed(changed).captured()
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

    fn emit_image_crop(&mut self, image: NodeIdentity) {
        let Some(zoom) = self.image_zooms.get(&image).copied().filter(|zoom| zoom.crop_events) else { return; };
        let transform = zoom.transform;
        let x = (((zoom.viewport.x + zoom.viewport.width / 2.0 - transform.translation_x) / transform.scale - zoom.content.x) / zoom.content.width).clamp(0.0, 1.0);
        let y = (((zoom.viewport.y + zoom.viewport.height / 2.0 - transform.translation_y) / transform.scale - zoom.content.y) / zoom.content.height).clamp(0.0, 1.0);
        self.apply(react::event(image.0, "onCropChange", vec![serde_json::json!({ "zoom": transform.scale, "x": x, "y": y })]));
    }

    fn text_input_at(&self, x: f32, y: f32) -> Option<TextInputLayout> {
        let Some(Action::FocusTextInput { state, .. }) = self.hit_region_at(x, y).map(|region| &region.action) else { return None; };
        self.text_inputs.iter().rev().copied().find(|input| {
            if input.state != *state { return false; }
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
        let Some(Node { kind: NodeKind::TextInput { options, .. }, .. }) = self.text_input_node(input.state) else {
            return false;
        };
        let style = self.theme.text_input.resolve(self.focused_input == Some(input.state), options);
        let role = style.font(&self.theme);
        let font_size = self.scaled_font_in(style.size(), role);
        let cursor = if input.action == TextInputAction::Return {
            let lines = self.input_lines(value, input.text_rect.width, &style);
            let y = if input.scrolling { y + self.scroll_offset - self.scroll_origin } else { y };
            let line = ((y - input.text_rect.y + input.scroll_offset) / self.scaled(style.line_height())).max(0.0) as usize;
            let (start, end) = lines[line.min(lines.len() - 1)];
            start + self.text_cursor_for_offset(&value[start..end], font_size, (x - input.text_rect.x - input_alignment_offset(&style, input.text_rect.width, text_width_in(&value[start..end], font_size, false, role))).max(0.0), role)
        } else {
            let width = text_width_in(value, font_size, false, role);
            let target = (x - input.text_rect.x + input.scroll_offset - input_alignment_offset(&style, input.text_rect.width, width)).clamp(0.0, width);
            self.text_cursor_for_offset(value, font_size, target, role)
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
        id: i32,
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
            self.pointers.insert(id, Pointer::TextInput(pointer));
            self.claim_pointer(id);
            let next = (pointer.input.scroll_offset + pointer.start_y - y)
                .clamp(0.0, pointer.input.scroll_max);
            return PointerOutcome::changed(self.set_text_input_scroll(pointer.input.state, next)).captured();
        }
        if !pointer.dragging {
            if horizontal.abs() <= tap_slop && vertical.abs() <= tap_slop {
                self.pointers.insert(id, Pointer::TextInput(pointer));
                return PointerOutcome::default();
            }
            if vertical.abs() > horizontal.abs() {
                return self.move_content_pointer(
                    id,
                    ContentPointer {
                        target: Some(PressTarget { node: pointer.input.node, role: PressRole::Focus }),
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
        self.pointers.insert(id, Pointer::TextInput(pointer));
        self.claim_pointer(id);
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
        self.scene.revision = self.scene.revision.wrapping_add(1);
        true
    }

    fn move_content_pointer(
        &mut self,
        id: i32,
        mut pointer: ContentPointer,
        x: f32,
        y: f32,
        tap_slop: f32,
    ) -> PointerOutcome {
        let delta = pointer.start_y - y;
        if !pointer.dragging {
            if delta.abs() <= tap_slop {
                pointer.cancelled |= (x - pointer.start_x).abs() > tap_slop;
                self.pointers.insert(id, Pointer::Content(pointer));
                return PointerOutcome::default();
            }
            pointer.start_y -= delta.signum() * tap_slop;
            pointer.dragging = true;
            pointer.cancelled = true;
        }
        self.pointers.insert(id, Pointer::Content(pointer));
        self.claim_pointer(id);
        let next = (pointer.start_offset + pointer.start_y - y).clamp(0.0, self.scroll_max);
        PointerOutcome::changed(self.set_scroll_offset(next))
    }

    fn move_edge_back_pointer(
        &mut self,
        id: i32,
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
                target: pointer.target,
                start_x: pointer.start_x,
                start_y: pointer.start_y,
                start_offset: pointer.start_offset,
                dragging: false,
                cancelled: false,
            };
            return self.move_content_pointer(id, content_pointer, x, y, tap_slop);
        }

        let claimed = pointer.state == EdgeBackState::Claimed
            || (horizontal_distance > self.scaled(BACK_SWIPE_ACTIVATION_DISTANCE)
                && vertical_distance <= horizontal_distance * BACK_SWIPE_VERTICAL_RATIO);
        if !claimed {
            return PointerOutcome::default();
        }

        self.claim_pointer(id);
        if horizontal > self.scaled(BACK_SWIPE_TRIGGER_DISTANCE)
            && vertical_distance <= horizontal * BACK_SWIPE_VERTICAL_RATIO
        {
            self.pointers.remove(&id);
            return PointerOutcome::activated(self.pop_route()).captured();
        }

        self.pointers.insert(id, Pointer::EdgeBack(EdgeBackPointer {
            target: pointer.target,
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
        self.pop_route()
    }

    pub const fn text_input_active(&self) -> bool {
        self.focused_input.is_some()
    }

    pub const fn text_input_action(&self) -> TextInputAction {
        self.focused_input_action
    }

    pub fn text_input_numeric(&self) -> bool {
        self.focused_input
            .and_then(|state| self.text_input_numeric_for(state))
            .unwrap_or(false)
    }

    fn text_input_numeric_for(&self, state: StateId) -> Option<bool> {
        match &self.text_input_node(state)?.kind {
            NodeKind::TextInput { numeric, .. } => Some(*numeric),
            _ => None,
        }
    }

    fn keeps_focus(&self, state: StateId) -> bool {
        matches!(self.text_input_node(state), Some(Node { kind: NodeKind::TextInput { options, .. }, .. }) if options.keep_focus)
    }

    fn text_input_node(&self, state: StateId) -> Option<&Node> {
        fn find(node: &Node, target: StateId) -> Option<&Node> {
            match &node.kind {
                NodeKind::TextInput { state, .. } if *state == target => Some(node),
                NodeKind::ConversationComposer { children, .. }
                | NodeKind::Message { children, .. }
                | NodeKind::MessageQuote { children, .. }
                | NodeKind::LinkPreview { children, .. }
                | NodeKind::RowTitle { children, .. }
                | NodeKind::PlayingTransport { children, .. }
                | NodeKind::PlayingLayout { children, .. }
                | NodeKind::Pressable { children, .. }
                | NodeKind::MediaGridRow { children }
                | NodeKind::Row { children, .. }
                | NodeKind::Stack { children, .. }
                | NodeKind::ReactList { children, .. } => children.iter().find_map(|child| find(child, target)),
                NodeKind::Screen(screen) => screen.children.iter().find_map(|child| find(child, target)),
                NodeKind::Tabs { tabs, .. } => tabs.iter().find_map(|tab| find(&tab.screen, target)),
                _ => None,
            }
        }
        find(&self.root, state)
    }

    pub fn edit_text(&mut self, edit: TextEdit) -> bool {
        if self.focused_input.is_none() {
            return false;
        }
        match edit {
            TextEdit::Update(payload) => self.apply_editor_update(&payload),
            TextEdit::Submit | TextEdit::Dismiss => {
                self.focused_input = None;
                self.relayout_scene();
                true
            }
        }
    }

    /// Apply an app theme. Install its fonts with `set_theme_fonts` first; call before the first
    /// layout where possible, since fonts are process-wide.
    pub fn set_theme(&mut self, theme: Theme) -> bool {
        if *self.base_theme == theme {
            return false;
        }
        self.apply_theme(Arc::new(theme));
        self.wrapped_text.clear();
        let Viewport { width, height, .. } = self.viewport;
        self.viewport = Viewport::default();
        self.set_viewport(width, height);
        true
    }

    fn apply_theme(&mut self, theme: Arc<Theme>) {
        self.scene.palette = theme.palette;
        self.switch_masks = None;
        self.base_theme = theme.clone();
        self.theme = theme;
    }

    /// Use a screen's theme: its named variant, or the app theme. The visible screen's theme stays
    /// active after layout, so text input and scrolling use it too.
    fn use_screen_theme(&mut self, variant: Option<&str>) {
        let theme = variant
            .and_then(|name| self.base_theme.variants.get(name))
            .unwrap_or(&self.base_theme);
        if Arc::ptr_eq(theme, &self.theme) {
            return;
        }
        let theme = theme.clone();
        self.scene.palette = theme.palette;
        if self.viewport.width > 0 {
            self.viewport.scale = self.viewport.width as f32 / LP3_REFERENCE_WIDTH * LP3_REFERENCE_SCALE * theme.scale;
        }
        self.theme = theme;
    }

    pub fn set_colour_scheme(&mut self, light: bool) -> bool {
        if self.scene.light == light {
            return false;
        }
        self.scene.light = light;
        self.relayout_scene();
        true
    }

    /// Opacity of a scroll bar that shows while scrolling: visible, then fading out.
    fn scroll_bar_opacity(&self) -> f32 {
        if self.theme.scroll_bar_visibility == ScrollBarVisibility::Always {
            return 1.0;
        }
        let Some(started) = self.scroll_activity else { return 0.0 };
        let elapsed = started.elapsed().as_secs_f32();
        (1.0 - (elapsed - SCROLL_BAR_HOLD) / SCROLL_BAR_FADE).clamp(0.0, 1.0)
    }

    pub fn has_scene_animations(&self) -> bool {
        self.scroll_activity.is_some() ||
        self.marquees.values().any(|marquee| marquee.visible)
            || self.playback_progress.values().any(PlaybackProgress::animating)
            || self.image_animations.keys().any(|key| visible_image(&self.scene, image_id(key)))
    }

    /// How long until the scene changes on its own: zero for continuous animations, or the next
    /// visible step of slow playback progress.
    pub fn next_scene_frame(&self) -> Option<Duration> {
        if let Some(started) = self.scroll_activity {
            let hold = Duration::from_secs_f32(SCROLL_BAR_HOLD);
            return Some(hold.saturating_sub(started.elapsed()));
        }
        if self.marquees.values().any(|marquee| marquee.visible)
            || self.image_animations.keys().any(|key| visible_image(&self.scene, image_id(key)))
        {
            return Some(Duration::ZERO);
        }
        self.playback_progress.values().filter(|progress| progress.animating())
            .map(|progress| progress.next_change(self.scene.quads[progress.quad].rect.x))
            .min()
    }

    pub fn update_playback_clock(&mut self, controller: u64, position: f32, duration: f32, playing: bool, speed: f32) -> bool {
        if controller == 0 || !position.is_finite() || !duration.is_finite() || !speed.is_finite()
            || position < 0.0 || duration < 0.0 || speed <= 0.0 { return false; }
        let started = Instant::now();
        self.playback_clocks.insert(controller, PlaybackClock { position, duration, playing, speed, started });
        let mut visible = false;
        let mut duration_changed = false;
        let mut changed = false;
        for progress in self.playback_progress.values_mut().filter(|progress| progress.clock == Some(controller)) {
            visible |= progress.visible;
            duration_changed |= progress.duration != duration;
            // Routine updates while playing only correct drift, which the next scheduled frame shows.
            changed |= progress.visible && (!playing || progress.playing != playing
                || progress.speed != speed || progress.duration != duration
                || (progress.current() - position).abs() > PLAYBACK_CLOCK_JUMP);
            progress.position = position; progress.duration = duration; progress.playing = playing;
            progress.speed = speed; progress.started = started;
        }
        if visible && duration_changed { self.rebuild_scene(); }
        changed
    }

    pub fn remove_playback_clock(&mut self, controller: u64) {
        self.playback_clocks.remove(&controller);
        for progress in self.playback_progress.values_mut().filter(|progress| progress.clock == Some(controller)) {
            progress.position = progress.current();
            progress.playing = false;
        }
    }

    pub fn animate_scene(&mut self) -> bool {
        let mut changed = false;
        if self.scroll_activity.is_some() {
            let opacity = self.scroll_bar_opacity();
            if let Some(bar) = &mut self.scene.scroll_bar
                && bar.opacity != opacity
            {
                bar.opacity = opacity;
                changed = true;
            }
            if opacity == 0.0 { self.scroll_activity = None; }
        }
        // New image frames only bump `image_revision`, so the renderer refreshes images alone.
        let mut images_changed = false;
        self.image_animations.retain(|key, animation| {
            let id = image_id(key);
            if !visible_image(&self.scene, id) { return true; }
            if let Some(pixels) = animation.advance()
                && let Some(RemoteImageState::Ready { image, .. }) = self.remote_images.get_mut(key)
            {
                image.pixels = pixels;
                image.generation = self.next_image_generation;
                self.next_image_generation = self.next_image_generation.wrapping_add(1).max(1);
                for run in &mut self.scene.images {
                    if run.image.id() == id { run.image = ImageData::Remote(image.clone()); }
                }
                for preloaded in &mut self.scene.preloaded_images {
                    if preloaded.id() == id { *preloaded = ImageData::Remote(image.clone()); }
                }
                for retained in self.retained_images.values_mut() {
                    if retained.id() == id { *retained = ImageData::Remote(image.clone()); }
                }
                self.scene.image_revision = self.scene.image_revision.wrapping_add(1);
                images_changed = true;
            }
            !animation.finished()
        });
        for marquee in self.marquees.values().filter(|marquee| marquee.visible) {
            let cycle = 1.25 + marquee.distance / marquee.speed;
            let elapsed = marquee.started.elapsed().as_secs_f32() % cycle;
            let offset = (elapsed - 1.25).max(0.0) * marquee.speed;
            for copy in 0..2 {
                let run = &mut self.scene.text[marquee.run + copy];
                let x = marquee.x - offset + copy as f32 * marquee.distance;
                changed |= run.rect.x != x;
                run.rect.x = x;
            }
        }
        for progress in self.playback_progress.values_mut().filter(|progress| progress.visible) {
            let current = progress.current();
            if let Some((run, seconds)) = &mut progress.elapsed_label {
                let next = current.max(0.0).floor() as u64;
                if next != *seconds {
                    self.scene.text[*run].text = playback_time(next).into();
                    *seconds = next;
                    changed = true;
                }
            }
            let width = if progress.duration > 0.0 {
                progress.width * (current / progress.duration).clamp(0.0, 1.0)
            } else { 0.0 };
            let quad = &mut self.scene.quads[progress.quad];
            let right = quad.rect.x + width;
            let solid_width = (right.floor() - quad.rect.x).max(0.0);
            changed |= quad.rect.width != solid_width;
            quad.rect.width = solid_width;
            // Shade the partially covered pixel so slow progress doesn't jump pixel by pixel.
            // Quarter steps look the same while letting idle frames wait for the next step.
            let edge = &mut self.scene.quads[progress.quad + 1];
            let alpha = ((right.fract() * PROGRESS_EDGE_STEPS).floor() / PROGRESS_EDGE_STEPS).min(width);
            changed |= edge.rect.x != right.floor() || edge.colour.alpha != alpha;
            edge.rect.x = right.floor();
            edge.colour.alpha = alpha;
        }
        if changed { self.scene.revision = self.scene.revision.wrapping_add(1); }
        changed || images_changed
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn can_present_pointer_move(&self, id: i32, y: f32) -> bool {
        if matches!(self.pointers.get(&id), Some(Pointer::CanvasRange(_))) { return true; }
        let Some(Pointer::Content(pointer)) = self.pointers.get(&id) else { return false; };
        // Before dragging starts this includes touch slop, so checking both
        // endpoints conservatively covers the viewport the move can produce.
        let offset = (pointer.start_offset + pointer.start_y - y).clamp(0.0, self.scroll_max);
        self.list_viewports_ready() && self.list_viewports_ready_at(offset)
    }

    pub fn list_viewports_ready(&self) -> bool {
        self.list_viewports_ready_at(self.scroll_offset)
    }

    fn list_viewports_ready_at(&self, offset: f32) -> bool {
        let Some(clip) = self.scene.scroll_clip else { return true; };
        self.react_list_positions.iter().all(|(id, top)| {
            let Some(metrics) = self.list_metrics.get(id) else { return true; };
            let start = (offset + clip.y - top).max(0.0);
            let end = (offset + clip.y + clip.height - top).min(metrics.total());
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
            Action::CanvasRange { .. } => false,
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
            Action::Native { operation } => self.queue_native_action(Arc::unwrap_or_clone(operation)),
            Action::Back => self.pop_route(),
        }
    }

    fn pop_route(&mut self) -> bool {
        self.snapshot_canvas_ranges();
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
        if self.theme.scroll_bar_visibility == ScrollBarVisibility::Scrolling {
            self.scroll_activity = Some(Instant::now());
            if let Some(bar) = &mut self.scene.scroll_bar { bar.opacity = 1.0; }
        }
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
                metrics.follow_end && self.pointers.is_empty()
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
        for pointer in self.pointers.values_mut() {
            match pointer {
                Pointer::Content(pointer) => pointer.start_offset += adjustment,
                Pointer::EdgeBack(pointer) => pointer.start_offset += adjustment,
                Pointer::TextInput(pointer) => pointer.content_offset += adjustment,
                _ => {}
            }
        }

        // Validate against the final layout, after virtualised-list anchor corrections.
        self.pointers.retain(|_, pointer| match pointer {
            Pointer::Content(ContentPointer { dragging: true, .. }) => true,
            Pointer::Content(ContentPointer { target: Some(target), .. })
            | Pointer::EdgeBack(EdgeBackPointer { target: Some(target), .. }) =>
                self.hit_regions.iter().any(|region| region.target == *target),
            Pointer::TextInput(pointer) => {
                let input = self.text_inputs.iter().find(|input| input.node == pointer.input.node);
                if let Some(input) = input && !pointer.dragging { pointer.input = *input; }
                input.is_some()
            },
            Pointer::CanvasDrag { group, .. } => self.hit_regions.iter().any(|region| region.drag_group == Some(*group)),
            Pointer::CanvasRange(pointer) => self.canvas_ranges.contains_key(&pointer.range),
            Pointer::ImagePan(pointer) => self.visible_zoom_images.contains(&pointer.image),
            Pointer::ScrollThumb { .. } | Pointer::ScrollTrack { .. } => self.scene.scroll_bar.is_some(),
            _ => true,
        });

        // Keep only text used by this layout, ready for the next update.
        self.wrapped_text.retain(|_, (_, used)| std::mem::take(used));

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
        self.scene.hitboxes.clear();
        self.screen_debug_hitboxes = false;
        self.scene.lines.clear();
        self.scene.text.clear();
        self.fixed_text_runs.clear();
        self.clear_capture_runs();
        for marquee in self.marquees.values_mut() { marquee.visible = false; }
        for progress in self.playback_progress.values_mut() { progress.visible = false; }
        self.scene.masks.clear();
        self.scene.background_masks.clear();
        self.scene.images.clear();
        self.scene.preloaded_images.clear();
        self.preloaded_images.clear();
        if self.screen_images_ready() {
            for started in self.image_screens.values_mut() { *started = None; }
        }
        self.blocking_images.clear();
        self.visible_image_screens.clear();
        self.visible_retained_images.clear();
        self.pending_images.clear();
        self.scene.camera_portal = None;
        self.scene.map_portal = None;
        self.scene.video_portal = None;
        self.scroll_origin = self.scroll_offset;
        self.scene.scroll_origin = self.scroll_origin;
        self.scene.scroll_offset = self.scroll_offset;
        self.scene.scroll_clip = None;
        self.scene.scroll_bar = None;
        self.hit_regions.clear();
        self.text_inputs.clear();
        self.visible_images.clear();
        self.visible_zoom_images.clear();
        self.reset_canvas_range_layout();
        self.scroll_max = 0.0;
        self.scene.scroll_max = 0.0;
        self.scrolling = false;

        if self.viewport.width == 0 || self.viewport.height == 0 {
            self.pointer_cancel();
            return;
        }

        if self.focused_input.or(self.native_editor_state).is_some_and(|state| {
            self.text_input_node(state).is_none_or(|node| matches!(&node.kind, NodeKind::TextInput { options, .. } if options.disabled || options.read_only))
        }) {
            self.focused_input = None;
            self.native_editor_state = None;
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
        // Keep the native text on blur; switching inputs or screens releases it.
        if self.focused_input.is_some() && self.native_editor_state != self.focused_input {
            self.native_editor_state = None;
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
        if self.debug_hitboxes || self.screen_debug_hitboxes {
            self.scene.hitboxes.extend(self.hit_regions.iter().map(|region| Hitbox { rect: region.rect, scrolling: region.scrolling }));
            if let Some(bar) = self.scene.scroll_bar {
                self.scene.hitboxes.push(Hitbox { rect: bar.touch_rect(), scrolling: false });
            }
            self.scene.hitboxes.extend(self.scene.images.iter().filter(|image| image.zoom_id.is_some())
                .map(|image| Hitbox { rect: image.rect.intersection(image.clip), scrolling: image.scrolling }));
        }
        self.image_screens.retain(|id, _| self.visible_image_screens.contains(id));
        self.marquees.retain(|_, marquee| marquee.visible);
        self.playback_progress.retain(|_, progress| progress.visible);
        self.animate_scene();
        self.retained_images.retain(|identity, _| self.visible_retained_images.contains(identity));
        self.sync_visible_images();
        self.image_zooms
            .retain(|identity, _| self.visible_zoom_images.contains(identity));
        self.retain_canvas_ranges();
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
                options,
                ..
            } if !options.disabled && !options.read_only => Some((node.identity, *state, *action)),
            NodeKind::ConversationComposer { children, .. }
            | NodeKind::Message { children, .. }
            | NodeKind::MessageQuote { children, .. }
            | NodeKind::LinkPreview { children, .. }
            | NodeKind::RowTitle { children, .. }
            | NodeKind::PlayingTransport { children, .. }
            | NodeKind::PlayingLayout { children, .. }
            | NodeKind::Pressable { children, .. }
            | NodeKind::MediaGridRow { children }
            | NodeKind::Row { children, .. }
            | NodeKind::Stack { children, .. }
            | NodeKind::ReactList { children, .. } => {
                children.iter().find_map(|child| self.auto_focus(child))
            }
            NodeKind::Screen(screen) => screen.children.iter().find_map(|child| self.auto_focus(child)),
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
            | NodeKind::VideoView { .. } | NodeKind::MapView { .. }
            | NodeKind::MediaCell { .. }
            | NodeKind::PlayingLabel { .. }
            | NodeKind::PlayingProgress { .. }
            | NodeKind::PitchIndicator { .. }
            | NodeKind::CaptureReadout { .. }
            | NodeKind::Canvas { .. }
            | NodeKind::SectionTitle { .. }
            | NodeKind::RowSubtitle { .. }
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
            NodeKind::Canvas { width, height, .. } => {
                let scale = self.viewport.scale.min(available.width / width).min(available.height / height);
                MeasuredSize { width: width * scale, height: height * scale }
            }
            NodeKind::MediaGridRow { .. } => MeasuredSize { width: available.width, height: available.width / 3.0 },
            NodeKind::MediaCell { .. } => MeasuredSize { width: available.width, height: available.width },
            NodeKind::ReactList { children, start, keys, content_versions, revision, gap, follow_end, .. } => {
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
            NodeKind::LinkPreview { children } => {
                let width = available.width.min(self.scaled(220.0));
                let mut height = self.scaled(16.0);
                for (index, child) in children.iter().enumerate() {
                    let inset = if index + 1 == children.len() { self.scaled(20.0) } else { 0.0 };
                    height += self.measure(child, Rect { width: (width - inset).max(0.0), ..available }).height;
                }
                MeasuredSize { width, height: height.min(available.height) }
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
            NodeKind::Pressable { children, .. } => children.first().map(|child| self.measure(child, available)).unwrap_or_default(),
            NodeKind::PlayingTransport { children, .. } => MeasuredSize {
                width: available.width,
                height: children.iter().map(|child| self.measure(child, available).height).fold(0.0, f32::max),
            },
            NodeKind::PlayingProgress { show_times, .. } => MeasuredSize {
                width: available.width,
                height: self.scaled(6.0) + if *show_times { self.scaled(1.0) + self.text_line_height(12.0) } else { 0.0 },
            },
            NodeKind::PitchIndicator { .. } => MeasuredSize { width: available.width, height: self.scaled(40.0) },
            NodeKind::CaptureReadout { kind, .. } => MeasuredSize { width: available.width, height: self.text_line_height(kind.size()) },
            NodeKind::Row { children, has_image, end, .. } => {
                let image_width = if *has_image { self.scaled(65.0).min(available.width) } else { 0.0 };
                let end_width = if end.is_some() { self.scaled(ROW_END_ICON_SIZE + BUTTON_ICON_GAP) } else { 0.0 };
                let text = children.last().map(|child| self.measure(child, Rect { width: (available.width - image_width - end_width).max(0.0), ..available })).unwrap_or_default();
                MeasuredSize { width: available.width, height: text.height.max(self.scaled(50.0)).min(available.height) }
            }
            NodeKind::PlayingLayout { .. } | NodeKind::Screen(_) | NodeKind::Tabs { .. } => MeasuredSize {
                width: available.width,
                height: available.height,
            },
            NodeKind::Stack {
                children,
                axis,
                gap,
                theme_gap,
                ..
            } => {
                let gap = self.scaled(self.stack_gap(*gap, *theme_gap));
                match axis {
                    Axis::Vertical => {
                        let measured = self.measure_vertical_children(children, available);
                        MeasuredSize {
                            width: measured
                                .iter()
                                .map(|measure| measure.width)
                                .fold(0.0, f32::max)
                                .min(available.width),
                            height: (measured.iter().map(|measure| measure.height).sum::<f32>()
                                + self.vertical_gaps(children, gap).sum::<f32>())
                                .min(available.height),
                        }
                    }
                    Axis::Horizontal => {
                        let measured = self.measure_horizontal_children(children, gap, available);
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
            NodeKind::RowTitle { children, text, size, max_lines, themed } => {
                let size = &self.row_title_size(*size, *themed);
                let inset = if children.is_empty() { 0.0 } else { self.scaled(*size + 6.0) };
                let lines = self.wrap_inset_text(text, self.scaled_font(*size), available.width, *max_lines, false, &[], inset);
                MeasuredSize { width: available.width, height: (self.text_line_height(*size) * lines.len() as f32).min(available.height) }
            }
            NodeKind::PlayingLabel { text, size } => MeasuredSize {
                width: self.text_width(text, self.scaled_font(*size)).min(available.width),
                height: self.text_line_height(*size).min(available.height),
            },
            NodeKind::Text {
                width,
                text,
                font_size,
                align,
                max_lines,
                tabular_numbers,
                links,
            } => {
                let size = font_size.unwrap_or(DEFAULT_TEXT_SIZE);
                let font_size = self.scaled_font(size);
                let fixed_width = width.map(|width| self.scaled(width.max(0.0)).min(available.width));
                if let Some(width) = fixed_width
                    && *max_lines == Some(1)
                {
                    return MeasuredSize {
                        width,
                        height: self.text_line_height(size).min(available.height),
                    };
                }
                let lines = self.wrap_linked_text(text, font_size, fixed_width.unwrap_or(available.width), *max_lines, *tabular_numbers, links);
                let line_height = self.text_line_height(size);
                MeasuredSize {
                    width: fixed_width.unwrap_or_else(|| {
                        if *align == TextAlign::Justify && lines.iter().any(|line| line.wrapped) {
                            available.width
                        } else {
                            lines.iter().map(|line| line.width).fold(0.0, f32::max)
                        }
                    }).min(available.width),
                    height: (line_height * lines.len() as f32).min(available.height),
                }
            }
            NodeKind::TextInput { state, action, prefix, suffix, options, .. } =>
                self.measure_input(*state, *action, prefix, suffix, options, available),
            // With ticks, options fill the row so the whole width is pressable; underlined options
            // stay as wide as their text.
            NodeKind::Button { selectable: true, .. } if self.theme.selection == SelectionStyle::Check => MeasuredSize {
                width: available.width,
                height: self.scaled(BUTTON_HEIGHT).min(available.height),
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
            NodeKind::RowSubtitle { text, wrap } => {
                let text = Node::text(text.clone(), Some(self.toggle_subtitle_size()), TextAlign::Start, (!wrap).then_some(1), true);
                self.measure(&text, available)
            }
            // The rest of the content gap sits above the title, so titles group with their rows.
            NodeKind::SectionTitle { .. } => MeasuredSize {
                width: available.width,
                height: (self.scaled((self.content_gap() - self.section_gap()).max(0.0)) + self.text_line_height(self.section_size()))
                    .min(available.height),
            },
            NodeKind::Field { label, value, action, image, .. } => {
                let Some(_) = image else { return self.measure_field_text(label, value, action.is_some(), available) };
                let slot = self.scaled(FIELD_IMAGE_SLOT).min(available.width);
                let text = self.measure_field_text(label, value, action.is_some(), Rect { width: (available.width - slot).max(0.0), ..available });
                MeasuredSize { width: available.width, height: text.height.max(self.scaled(FIELD_IMAGE_SIZE)).min(available.height) }
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
                looping,
                source,
                bleed,
                fill_width,
                width,
                height,
                fit,
                ..
            } => {
                let measured_width = if *bleed {
                    self.viewport.width as f32
                } else if *fill_width {
                    available.width
                } else {
                    self.scaled(*width).min(available.width)
                };
                let code_height = match source {
                    ImageSource::Native(module, url) if module == "barcode" => {
                        let pixels = measured_width.ceil().max(1.0) as u32;
                        let key = RemoteImageKey { module: module.clone(), url: url.clone(), width: pixels, height: pixels, fit: *fit, looping: *looping };
                        match self.remote_images.get(&key) {
                            Some(RemoteImageState::Ready { image, .. }) => Some(measured_width * image.height as f32 / image.width as f32),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                MeasuredSize {
                    width: measured_width,
                    height: if let Some(height) = code_height {
                        height.min(available.height)
                    } else if *bleed || *fill_width {
                        (measured_width * height / width).min(available.height)
                    } else {
                        self.scaled(*height).min(available.height)
                    },
                }
            }
            NodeKind::CameraPreview { .. } | NodeKind::VideoView { .. } | NodeKind::MapView { .. } => MeasuredSize {
                width: available.width,
                height: if available.height.is_finite() {
                    available.height
                } else {
                    (self.viewport.height as f32 - available.y).max(0.0)
                },
            },
            NodeKind::Toggle { label, subtitle, .. } => {
                let width = (available.width - self.scaled(self.toggle_control_width() + TOGGLE_LABEL_GAP)).max(0.0);
                let lines = self.wrap_text(label, self.scaled_font(DEFAULT_TEXT_SIZE), width, None, false);
                MeasuredSize {
                    width: available.width,
                    height: (self.text_line_height(DEFAULT_TEXT_SIZE) * lines.len() as f32
                        + if subtitle.is_some() { self.text_line_height(self.toggle_subtitle_size()) } else { 0.0 })
                        .max(self.scaled(TOGGLE_HEIGHT))
                        .min(available.height),
                }
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
        let previous = std::mem::replace(&mut self.layout_owner, node.identity);
        self.layout_node_inner(node, rect, screen_bottom_inset);
        self.layout_owner = previous;
    }

    fn layout_node_inner(&mut self, node: &Node, rect: Rect, screen_bottom_inset: bool) {
        let visible = rect.intersection(self.clip);
        if self.scrolling
            && !matches!(
                &node.kind,
                NodeKind::Screen(_)
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
            NodeKind::Canvas { width, height, drawings } => self.layout_canvas(node.identity, *width, *height, drawings, rect),
            NodeKind::MediaGridRow { children } => {
                let size = rect.width / 3.0;
                for (column, child) in children.iter().enumerate() {
                    self.layout(child, Rect { x: rect.x + column as f32 * size, y: rect.y, width: size, height: size });
                }
            }
            NodeKind::MediaCell { source, selected, video, check, play, action } => {
                self.layout_image(node.identity, source, None, ImageFit::Cover, false, false, false, None, None, None, false, rect);
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
                    if let NodeKind::TextInput { placeholder, state, action, prefix, suffix, clear, options, .. } = &child.kind {
                        let previous = std::mem::replace(&mut self.layout_owner, child.identity);
                        self.layout_text_input(placeholder, *state, *action, prefix, suffix, clear, options,
                            Rect { width: width + side + gap, ..child_rect }, side + gap);
                        self.layout_owner = previous;
                    } else {
                        let first_hit = self.hit_regions.len();
                        self.layout(child, child_rect);
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
            NodeKind::LinkPreview { children } => {
                let mut y = rect.y;
                for (index, child) in children.iter().enumerate() {
                    let info = index + 1 == children.len();
                    let inset = if info { self.scaled(10.0) } else { 0.0 };
                    if info { y += self.scaled(8.0); }
                    let area = Rect { x: rect.x + inset, y, width: (rect.width - inset * 2.0).max(0.0), height: (rect.y + rect.height - y).max(0.0) };
                    let size = self.measure(child, area);
                    self.layout(child, Rect { height: size.height, ..area });
                    y += size.height;
                }
            }
            NodeKind::MessageQuote { children } => {
                let inset = self.scaled(10.0);
                self.scene.quads.push(Quad { rect: Rect { width: self.scaled(2.0), ..rect }, clip: self.clip, colour: self.scene.colour(Colour::WHITE), scrolling: self.scrolling });
                if let Some(child) = children.first() { self.layout(child, Rect { x: rect.x + inset, width: (rect.width - inset).max(0.0), ..rect }); }
            }
            NodeKind::PlayingLayout { children, centred, hide_controls, bleed } => {
                let inset = self.scaled(self.content_inset());
                let body = Rect { x: rect.x + inset, width: (rect.width - inset * 2.0).max(0.0), ..rect };
                let footer = self.measure(&children[1], body);
                let bottom = rect.y + rect.height - self.scaled(CONTENT_BOTTOM);
                if !hide_controls {
                    self.layout(&children[1], Rect { y: bottom - footer.height, height: footer.height, ..body });
                }
                let content = if *bleed { rect } else { body };
                let available = Rect { height: (bottom - footer.height - self.scaled(12.0) - content.y).max(0.0), ..content };
                let size = self.measure(&children[0], available);
                let y = available.y + if *centred { (available.height - size.height).max(0.0) / 2.0 } else { 0.0 };
                self.layout(&children[0], Rect { y, height: size.height, ..available });
            }
            NodeKind::PlayingTransport { children, loading } => {
                for (index, child) in children.iter().enumerate() {
                    if *loading && index != 1 {
                        continue;
                    }
                    let slot = rect.width / children.len() as f32;
                    let size = self.measure(child, Rect { width: slot, ..rect });
                    let x = if children.len() == 1 {
                        rect.x + (rect.width - size.width) / 2.0
                    } else {
                        let fraction = index as f32 / (children.len() - 1) as f32;
                        rect.x + fraction * (rect.width - size.width)
                    };
                    if index == 1 && *loading {
                        let font_size = self.scaled_font(14.0);
                        self.scene.text.push(TextRun { font: self.theme.body_font,
                            text: "Loading...".into(),
                            rect: Rect { y: rect.y + (rect.height - font_size) / 2.0, height: font_size, ..rect },
                            clip: rect.intersection(self.clip), font_size,
                            colour: self.scene.colour(Colour::WHITE),
                            align: TextAlign::Centre, tabular_numbers: false, scrolling: self.scrolling,
                        });
                    } else {
                        self.layout(child, Rect { x, y: rect.y + (rect.height - size.height) / 2.0, width: size.width, height: size.height });
                    }
                }
            }
            NodeKind::RowTitle { children, text, size, max_lines, themed } => {
                let size = &self.row_title_size(*size, *themed);
                let inset = if children.is_empty() { 0.0 } else { self.scaled(*size + 6.0) };
                let line_height = self.text_line_height(*size);
                let lines = self.wrap_inset_text(text, self.scaled_font(*size), rect.width, *max_lines, false, &[], inset);
                if let Some(icon) = children.first() {
                    let size = self.scaled(*size);
                    self.layout(icon, Rect { y: rect.y + (line_height - size) / 2.0, width: size, height: size, ..rect });
                }
                for (index, line) in lines.iter().enumerate() {
                    let inset = if index == 0 { inset } else { 0.0 };
                    self.layout_text_lines(std::slice::from_ref(line), *size, TextAlign::Start, false, Rect {
                        x: rect.x + inset, y: rect.y + index as f32 * line_height,
                        width: (rect.width - inset).max(0.0), height: line_height.min((rect.height - index as f32 * line_height).max(0.0)),
                    });
                }
            }
            NodeKind::PlayingLabel { text, size } => {
                let font_size = self.scaled_font(*size);
                let width = self.text_width(text, font_size);
                let overflow = width > rect.width;
                let run = self.scene.text.len();
                let distance = width + self.scaled(25.0);
                let speed = self.scaled(24.0);
                self.scene.text.push(TextRun { font: self.theme.body_font,
                    text: text.as_str().into(), rect: Rect { width: width.max(rect.width), ..rect },
                    clip: rect.intersection(self.clip), font_size,
                    colour: self.scene.colour(Colour::WHITE),
                    align: if overflow { TextAlign::Start } else { TextAlign::Centre },
                    tabular_numbers: false, scrolling: self.scrolling,
                });
                if overflow {
                    let mut copy = self.scene.text[run].clone();
                    copy.rect.x += distance;
                    self.scene.text.push(copy);
                    let marquee = self.marquees.entry(node.identity).or_insert_with(|| Marquee {
                        text: text.clone(), started: Instant::now(), run, x: rect.x,
                        distance, speed, visible: true,
                    });
                    if marquee.text != *text || marquee.distance != distance {
                        marquee.text = text.clone();
                        marquee.started = Instant::now();
                    }
                    marquee.run = run;
                    marquee.x = rect.x;
                    marquee.distance = distance;
                    marquee.speed = speed;
                    marquee.visible = true;
                }
            }
            NodeKind::Pressable { children, action, long_action, selected, haptic } => {
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
                    let start = self.hit_regions.len();
                    self.push_press_region(hit, action.clone(), long_action.clone());
                    for region in &mut self.hit_regions[start..] { region.haptic = *haptic; }
                }
                if let Some(child) = children.first() { self.layout(child, rect); }
            }
            NodeKind::CaptureReadout { controller, kind } => self.layout_capture(*controller, *kind, rect),
            NodeKind::PitchIndicator { cents } => {
                let active = cents.map(|value| {
                    if value.abs() <= 5.0 { 12.0 } else { (value + 50.0) / 100.0 * 24.0 }
                });
                let width = self.scaled(2.0).min(rect.width);
                for tick in 0..25 {
                    let height = self.scaled(if tick == 12 { 40.0 } else { 16.0 });
                    self.scene.quads.push(Quad {
                        rect: Rect {
                            x: rect.x + (rect.width - width) * tick as f32 / 24.0,
                            y: rect.y + (rect.height - height) / 2.0,
                            width, height,
                        },
                        clip: self.clip,
                        colour: self.scene.colour(if active.is_some_and(|value| tick as f32 <= value) { Colour::WHITE } else { Colour::MUTED }),
                        scrolling: self.scrolling,
                    });
                }
            }
            NodeKind::PlayingProgress { position, duration, playing, seek, show_times, clock, speed } => {
                let anchor = clock.and_then(|id| self.playback_clocks.get(&id)).cloned();
                let position = anchor.as_ref().map(|value| &value.position).unwrap_or(position);
                let duration = anchor.as_ref().map(|value| &value.duration).unwrap_or(duration);
                let playing = anchor.as_ref().map(|value| &value.playing).unwrap_or(playing);
                let speed = anchor.as_ref().map(|value| value.speed).unwrap_or(*speed);
                let bar = Rect { height: self.scaled(6.0), ..rect };
                let ratio = if *duration > 0.0 { (position / duration).clamp(0.0, 1.0) } else { 0.0 };
                for (width, height) in [(rect.width, self.scaled(2.0)), (rect.width * ratio, self.scaled(6.0))] {
                    self.scene.quads.push(Quad { rect: Rect { y: bar.y + (bar.height - height) / 2.0, width, height, ..bar }, clip: self.clip, colour: self.scene.colour(Colour::WHITE), scrolling: self.scrolling });
                }
                let quad = self.scene.quads.len() - 1;
                let mut edge = self.scene.quads[quad].clone();
                edge.rect.width = 1.0;
                edge.colour.alpha = 0.0;
                self.scene.quads.push(edge);
                let progress = self.playback_progress.entry(node.identity).or_insert_with(|| PlaybackProgress {
                    position: *position, duration: *duration, playing: *playing, looping: false, clock: *clock, speed,
                    started: Instant::now(), quad, width: rect.width, visible: true,
                    elapsed_label: None,
                });
                if (progress.position, progress.duration, progress.playing, progress.speed) != (*position, *duration, *playing, speed) {
                    progress.position = *position;
                    progress.duration = *duration;
                    progress.playing = *playing;
                    progress.started = Instant::now();
                }
                progress.clock = *clock;
                progress.speed = speed;
                if let Some(anchor) = &anchor { progress.started = anchor.started; }
                progress.quad = quad;
                progress.width = rect.width;
                progress.visible = true;
                progress.elapsed_label = None;
                let current = progress.current().max(0.0).floor() as u64;
                if *show_times {
                    let run = self.scene.text.len();
                    let label = Rect { y: rect.y + self.scaled(7.0), height: self.text_line_height(12.0), ..rect };
                    self.layout_text(&playback_time(current), Some(12.0), TextAlign::Start, Some(1), true, label);
                    let elapsed_run = (self.scene.text.len() > run).then_some((run, current));
                    self.layout_text(&playback_time(duration.max(0.0).floor() as u64), Some(12.0), TextAlign::End, Some(1), true, label);
                    self.playback_progress.get_mut(&node.identity).unwrap().elapsed_label = elapsed_run;
                }
                if *seek && *duration > 0.0 {
                    self.push_hit_region(Rect { y: bar.y - self.scaled(15.0), height: self.scaled(36.0), ..bar }, Action::Seek { id: node.identity.0, left: rect.x, width: rect.width, duration: *duration });
                }
            }
            NodeKind::Row { children, has_image, action, long_action, end } => {
                let image_width = if *has_image { self.scaled(65.0).min(rect.width) } else { 0.0 };
                let end_width = if end.is_some() { self.scaled(ROW_END_ICON_SIZE + BUTTON_ICON_GAP) } else { 0.0 };
                if *has_image {
                    if let Some(image) = children.first() {
                        let size = self.scaled(50.0).min(rect.width);
                        self.layout(image, Rect { width: size, height: size, y: rect.y + (rect.height - size) / 2.0, ..rect });
                    }
                }
                if let Some(text) = children.last() {
                    let available = Rect { x: rect.x + image_width, width: (rect.width - image_width - end_width).max(0.0), ..rect };
                    let size = self.measure(text, available);
                    self.layout(text, Rect { y: rect.y + (rect.height - size.height) / 2.0, height: size.height, ..available });
                }
                if let Some(action) = action { self.push_press_region(rect, action.clone(), long_action.clone()); }
                if let Some((icon, end_action)) = end {
                    let size = self.scaled(ROW_END_ICON_SIZE);
                    let icon_rect = Rect { x: rect.x + rect.width - size, y: rect.y + (rect.height - size) / 2.0, width: size, height: size };
                    self.scene.masks.push(MaskRun {
                        mask: icon.clone(),
                        rect: icon_rect,
                        clip: self.clip,
                        colour: self.scene.colour(Colour::MUTED),
                        scrolling: self.scrolling,
                    });
                    // A separate end button covers the row's height so it is easy to press.
                    if let Some(end_action) = end_action {
                        let width = rect.height.max(size + self.scaled(BUTTON_ICON_GAP) * 2.0);
                        self.push_press_region(Rect { x: rect.x + rect.width - width, width, ..rect }, end_action.clone(), None);
                    }
                }
            }
            NodeKind::Screen(screen) => {
                let ScreenNode {
                    children,
                    title,
                    subtitle,
                    note,
                    theme,
                    centred,
                    footer,
                    pinned_header,
                    pinned_footer,
                    wide,
                    left_action,
                    right_actions,
                    media_picker,
                    bottom_inset,
                    background,
                    debug_hitboxes,
                } = screen.as_ref();
                self.use_screen_theme(theme.as_deref());
                self.screen_debug_hitboxes = *debug_hitboxes;
                if let Some(colour) = background {
                    self.scene.quads.push(Quad {
                        rect, clip: self.clip, colour: self.scene.explicit_colour(*colour), scrolling: false,
                    });
                }
                self.visible_image_screens.insert(node.identity);
                let waiting = self.image_screens.get(&node.identity) != Some(&None);
                let pending_start = self.pending_images.len();
                let image_start = self.scene.images.len();
                self.layout_screen(
                    children,
                    title.as_deref(),
                    subtitle.as_deref(),
                    note.as_deref(),
                    *centred,
                    footer.as_ref(),
                    *pinned_header,
                    *pinned_footer,
                    *wide,
                    left_action.as_ref(),
                    right_actions,
                    *media_picker,
                    screen_bottom_inset && *bottom_inset,
                    rect,
                );
                if waiting && (self.scene.images.len() > image_start || self.pending_images.len() > pending_start) {
                    let blocking: Vec<_> = self.pending_images[pending_start..].iter()
                        .filter(|image| pending_image_on_screen(&self.scene, image))
                        .map(|image| image.key.clone()).collect();
                    let ready = !blocking.iter().any(|key| {
                        matches!(self.remote_images.get(key), Some(RemoteImageState::Loading { .. }))
                    });
                    let started = self.image_screens.entry(node.identity).or_insert_with(|| Some(Instant::now()));
                    if ready { *started = None; }
                    self.blocking_images.extend(blocking);
                }
            },
            NodeKind::Stack {
                children,
                axis,
                gap,
                theme_gap,
                align,
                justify,
            } => self.layout_stack(
                children,
                *axis,
                self.scaled(self.stack_gap(*gap, *theme_gap)),
                *align,
                *justify,
                rect,
            ),
            NodeKind::Text {
                text,
                font_size,
                align,
                max_lines,
                tabular_numbers,
                links, width,
            } => {
                let size = font_size.unwrap_or(DEFAULT_TEXT_SIZE);
                let font = self.scaled_font(size);
                let lines = self.wrap_linked_text(text, font, rect.width, *max_lines, *tabular_numbers, links);
                if !links.is_empty() {
                    let line_height = self.text_line_height(size);
                    let mut offset = 0;
                    for (index, line) in lines.iter().enumerate() {
                        let Some(relative) = text[offset..].find(line.text.as_str()) else { break; };
                        let start = offset + relative;
                        let end = start + line.text.len();
                        let inset = match align { TextAlign::Centre => (rect.width - line.width) / 2.0, TextAlign::End => rect.width - line.width, _ => 0.0 };
                        for (range, action) in links {
                            let from = range.start.max(start);
                            let to = range.end.min(end);
                            if from < to {
                                let x = text_width_in(&text[start..from], font, *tabular_numbers, self.theme.body_font);
                                let width = text_width_in(&text[start..to], font, *tabular_numbers, self.theme.body_font) - x;
                                let hit = Rect { x: rect.x + inset + x, y: rect.y + index as f32 * line_height, width, height: line_height };
                                let long_action = self.hit_regions.iter().rev()
                                    .find(|region| region.rect.contains(hit.x, hit.y))
                                    .and_then(|region| region.long_action.clone());
                                self.push_press_region(hit, action.clone(), long_action);
                                self.scene.quads.push(Quad {
                                    rect: Rect { x: rect.x + inset + x, y: rect.y + (index + 1) as f32 * line_height - self.control_line_height(), width, height: self.control_line_height() },
                                    clip: self.clip, colour: self.scene.colour(Colour::WHITE), scrolling: self.scrolling,
                                });
                            }
                        }
                        offset = end;
                    }
                }
                let run = self.scene.text.len();
                self.layout_text_lines(&lines, size, *align, *tabular_numbers, rect);
                if node.identity.0 != 0 && *max_lines == Some(1) && links.is_empty() && self.scene.text.len() == run + 1 {
                    if let Some(width) = width { self.fixed_text_runs.insert(node.identity, FixedTextRun {
                        run, width: *width, font_size: *font_size, align: *align, tabular_numbers: *tabular_numbers,
                    }); }
                }
            }
            NodeKind::TextInput {
                placeholder,
                state,
                action,
                prefix,
                suffix,
                clear,
                options,
                ..
            } => self.layout_text_input(placeholder, *state, *action, prefix, suffix, clear, options, rect, 0.0),
            NodeKind::Button {
                label,
                icon,
                underline,
                action,
                long_action,
                ..
            } => self.layout_button(label, icon.clone(), *underline, action, long_action, rect),
            NodeKind::Field {
                label,
                value,
                action,
                disabled,
                image,
            } => self.layout_field(label, value, action, *disabled, image.as_deref(), rect),
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
            NodeKind::Image { source, fallback, fit, zoomable, looping, preload, retain_while_loading, avatar, rotation, background, crop, crop_events, horizontal_guides, overlay, .. } => {
                if let Some(colour) = background {
                    self.scene.quads.push(Quad { rect, clip: self.clip, colour: self.scene.explicit_colour(*colour), scrolling: self.scrolling });
                }
                self.layout_image(node.identity, source, fallback.as_ref(), *fit, *zoomable, *retain_while_loading, *looping, *avatar, *rotation, *crop, *crop_events, rect);
                self.layout_image_guides(horizontal_guides, rect);
                if let Some((source, bounds)) = overlay {
                    let overlay_rect = Rect { x: rect.x + bounds.x * rect.width, y: rect.y + bounds.y * rect.height,
                        width: bounds.width * rect.width, height: bounds.height * rect.height };
                    let clip = self.clip;
                    self.clip = self.clip.intersection(rect);
                    self.layout_image(node.identity, source, None, ImageFit::Contain, false, false, false, None, None, None, false, overlay_rect);
                    self.clip = clip;
                }
                let visible = rect.intersection(self.clip);
                if visible.width > 0.0 && visible.height > 0.0 {
                    for source in preload {
                        if let ImageSource::Native(module, url) = source {
                            let key = RemoteImageKey {
                                module: module.clone(), url: url.clone(), fit: *fit, looping: *looping,
                                width: rect.width.ceil().max(1.0) as u32,
                                height: rect.height.ceil().max(1.0) as u32,
                            };
                            self.visible_images.insert(key.clone());
                            if self.preloaded_images.insert(key.clone()) {
                                if let Some(RemoteImageState::Ready { image, .. }) = self.remote_images.get(&key) {
                                    self.scene.preloaded_images.push(ImageData::Remote(image.clone()));
                                } else {
                                    self.queue_remote_image(key);
                                }
                            }
                        }
                    }
                }
            }
            NodeKind::CameraPreview { controller, kind } => {
                self.layout_camera_preview(*controller, *kind, rect)
            }
            NodeKind::VideoView { controller, loading } => {
                if !cfg!(feature = "video") { return; }
                self.scene.video_portal = controller.map(|controller| MapPortal { controller, rect });
                if *loading {
                    let height = self.text_line_height(18.0);
                    self.layout_text("Loading…", Some(18.0), TextAlign::Centre, Some(1), false,
                        Rect { y: rect.y + (rect.height - height) / 2.0, height, ..rect });
                }
            }
            NodeKind::MapView { controller } => {
                if !cfg!(feature = "maps") { return; }
                self.scene.map_portal = Some(MapPortal { controller: *controller, rect });
            }
            NodeKind::Toggle {
                label,
                subtitle,
                value,
                action,
                off,
                on,
            } => {
                let enabled = *value;
                self.layout_toggle(
                    label,
                    subtitle.as_deref(),
                    enabled,
                    action.as_ref(),
                    off.clone(),
                    on.clone(),
                    rect,
                );
            }
            NodeKind::Tabs { value, tabs } => self.layout_tabs(value, tabs, rect),
            NodeKind::RowSubtitle { text, wrap } => {
                let start = self.scene.text.len();
                let text = Node::text(text.clone(), Some(self.toggle_subtitle_size()), TextAlign::Start, (!wrap).then_some(1), true);
                self.layout(&text, rect);
                if self.theme.subtitle_muted == Some(true) {
                    let muted = self.scene.colour(Colour::MUTED);
                    for run in &mut self.scene.text[start..] {
                        run.colour = muted;
                    }
                }
            }
            NodeKind::SectionTitle { title, upper } => {
                let text = if self.theme.section_uppercase { upper } else { title };
                let size = self.section_size();
                let top = self.scaled((self.content_gap() - self.section_gap()).max(0.0));
                let start = self.scene.text.len();
                self.layout_text(text, Some(size), TextAlign::Start, Some(1), false, Rect { y: rect.y + top, height: (rect.height - top).max(0.0), ..rect });
                let colour = self.scene.colour(Colour::MUTED);
                for run in &mut self.scene.text[start..] {
                    run.colour = colour;
                }
            }
        }
    }

    fn unobstructed_header(&self, rect: Rect) -> Rect {
        let mut cutouts: Vec<_> = self.display_cutouts.iter()
            .map(|cutout| cutout.intersection(rect))
            .filter(|cutout| cutout.width > 0.0 && cutout.height > 0.0)
            .collect();
        cutouts.sort_by(|a, b| a.x.total_cmp(&b.x));
        let mut start = rect.x;
        let mut best = Rect { width: 0.0, ..rect };
        for cutout in cutouts {
            let width = cutout.x - start;
            if width > best.width {
                best = Rect { x: start, width, ..rect };
            }
            start = start.max(cutout.x + cutout.width);
        }
        let width = rect.x + rect.width - start;
        if width > best.width {
            best = Rect { x: start, width, ..rect };
        }
        best
    }

    fn layout_screen(
        &mut self,
        children: &[Node],
        title: Option<&str>,
        subtitle: Option<&str>,
        note: Option<&str>,
        centred: bool,
        footer: Option<&(String, Option<Action>)>,
        pinned_header: bool,
        pinned_footer: bool,
        wide: bool,
        left_action: Option<&(Mask, Action)>,
        right_actions: &[(Mask, Action)],
        media_picker: bool,
        bottom_inset: bool,
        rect: Rect,
    ) {
        // Themes can start the screen below top cutouts instead of fitting the header beside them.
        let rect = if self.theme.header_cutout == CutoutMode::Below {
            let inset = self.display_cutouts.iter()
                .filter(|cutout| cutout.width > 0.0 && cutout.height > 0.0 && cutout.y < rect.y + rect.height / 4.0)
                .map(|cutout| cutout.y + cutout.height - rect.y)
                .fold(0.0, f32::max);
            Rect { y: rect.y + inset, height: (rect.height - inset).max(0.0), ..rect }
        } else {
            rect
        };
        let automatic_back = left_action.is_none();
        let has_header = title.is_some() || self.back_icon.is_some() || left_action.is_some() || !right_actions.is_empty();
        let large = self.theme.header == HeaderLayout::Large && title.is_some();
        let title_size = self.theme.header_size.unwrap_or(if large { LARGE_HEADER_TEXT_SIZE } else { HEADER_TEXT_SIZE });
        let subtitle = subtitle.filter(|_| large);
        // Every header is padding, a title row and padding. Centred headers default to padding
        // that fills Ink's 50-unit bar.
        let title_row = title_size * 1.25;
        let centred_padding = ((HEADER_HEIGHT - title_row) / 2.0).max(0.0);
        let header_top = self.theme.header_top.unwrap_or(if large { LARGE_HEADER_TOP } else { centred_padding });
        let header_bottom = self.theme.header_bottom.unwrap_or(if large { LARGE_HEADER_BOTTOM } else { centred_padding });
        let mut header_height = if has_header {
            self.scaled(header_top + title_row + header_bottom
                + if subtitle.is_some() { self.header_subtitle_size() * 1.4 } else { 0.0 })
        } else {
            0.0
        };
        let header_centre = rect.y + self.scaled(header_top + title_row / 2.0);
        let header_inset = self.scaled(HEADER_HORIZONTAL_INSET);
        let header_icon = self.theme.header_icon_size.unwrap_or(HEADER_BACK_ICON_SIZE);
        let header_button_size = self.scaled(HEADER_BUTTON_SIZE.max(header_icon));
        let action_stride = header_button_size + self.scaled(header_icon * HEADER_ACTION_GAP);
        let back_visible = self.back_icon.is_some() && automatic_back;
        let back_at_end = back_visible && self.theme.back_at_end;
        let subtitle_line_height = self.scaled(self.header_subtitle_size() * 1.4);
        let subtitle_font_size = self.scaled_font(self.header_subtitle_size());
        // Large headers align the end action's icon with the content edge.
        let end_inset = if large {
            self.scaled(self.content_inset()) - (header_button_size - self.scaled(header_icon)) / 2.0
        } else {
            header_inset
        };
        let has_left = (back_visible && !back_at_end) || left_action.is_some();
        let (header, back_x, end_x, x, width, subtitle_lines) = loop {
            let header = self.unobstructed_header(Rect { height: header_height, ..rect });
            let back_x = if back_at_end { header.x + header.width - end_inset - header_button_size } else { header.x + header_inset };
            // An end back button takes the end slot; the screen's end action moves in beside it.
            let end_x = header.x + header.width - end_inset - header_button_size * if back_at_end { 2.0 } else { 1.0 };
            let x = (header.x + self.scaled(self.content_inset()))
                .max(if has_left { header.x + header_inset + header_button_size } else { 0.0 });
            let end = if !right_actions.is_empty() { end_x - action_stride * (right_actions.len() - 1) as f32 }
                else if back_at_end { back_x }
                else { header.x + header.width - self.scaled(self.content_inset()) };
            let width = (end - x).max(0.0);
            let lines = subtitle.map(|text| self.wrap_text(text, subtitle_font_size, width, None, false));
            if let Some(lines) = &lines {
                let height = self.scaled(header_top + title_row + header_bottom) + subtitle_line_height * lines.len() as f32;
                // Growing the header can bring another cutout into its bounds and narrow the text.
                if height > header_height {
                    header_height = height;
                    continue;
                }
            }
            break (header, back_x, end_x, x, width, lines);
        };
        if back_visible {
            self.push_hit_region(
                Rect {
                    x: back_x,
                    y: header_centre - header_button_size / 2.0,
                    width: header_button_size,
                    height: header_button_size,
                },
                Action::Back,
            );
        }
        let ends = right_actions.iter().enumerate().map(|(index, action)| (Some(right_actions.len() - 1 - index), action));
        for (from_end, (icon, action)) in left_action.map(|action| (None, action)).into_iter().chain(ends) {
            let action_rect = Rect {
                x: from_end.map_or(header.x + header_inset, |from_end| end_x - action_stride * from_end as f32),
                y: header_centre - header_button_size / 2.0,
                width: header_button_size,
                height: header_button_size,
            };
            self.push_hit_region(action_rect, action.clone());
            let size = self.scaled(header_icon);
            // In large headers, the outermost end icon's drawn edge meets the content edge.
            let optical = if large && from_end == Some(0) && !back_at_end { self.icon_trailing_space(icon) * size } else { 0.0 };
            self.scene.masks.push(MaskRun {
                mask: icon.clone(),
                rect: Rect { x: action_rect.x + (header_button_size - size) / 2.0 + optical, y: action_rect.y + (header_button_size - size) / 2.0, width: size, height: size },
                clip: self.clip,
                colour: self.scene.colour(Colour::WHITE),
                scrolling: false,
            });
        }
        if let Some(title) = title.filter(|_| large) {
            let line = self.scaled(title_size * 1.25);
            let font_size = self.scaled_font_in(title_size, self.theme.display_font);
            self.scene.text.push(TextRun { font: self.theme.display_font,
                tabular_numbers: false,
                text: Self::ellipsize_in(title, font_size, width, self.theme.display_font).into(),
                rect: Rect { x, y: header_centre - line / 2.0, width, height: line },
                clip: self.clip,
                font_size,
                colour: self.scene.colour(Colour::WHITE),
                align: TextAlign::Start,
                scrolling: self.scrolling,
            });
            if let Some(lines) = subtitle_lines {
                let y = rect.y + self.scaled(header_top + title_size * 1.25);
                let colour = self.scene.colour(if self.theme.subtitle_muted == Some(false) { Colour::WHITE } else { Colour::MUTED });
                for (index, line) in lines.iter().enumerate() {
                    self.scene.text.push(TextRun { font: self.theme.body_font,
                        tabular_numbers: false,
                        text: line.text.clone(),
                        rect: Rect { x, y: y + index as f32 * subtitle_line_height, width, height: subtitle_line_height },
                        clip: self.clip,
                        font_size: subtitle_font_size,
                        colour,
                        align: TextAlign::Start,
                        scrolling: self.scrolling,
                    });
                }
            }
        } else if let Some(title) = title {
            let title_inset = header_inset
                + if self.back_icon.is_some() || left_action.is_some() || !right_actions.is_empty() {
                    header_button_size
                } else {
                    0.0
                };
            let title_height = self.scaled(32.0);
            let title_rect = Rect {
                x: header.x + title_inset,
                y: header_centre - title_height / 2.0,
                width: (header.width - title_inset * 2.0).max(0.0),
                height: title_height,
            };
            let font_size = self.scaled_font_in(title_size, self.theme.display_font);
            self.scene.text.push(TextRun { font: self.theme.display_font,
                tabular_numbers: false,
                text: Self::ellipsize_in(title, font_size, title_rect.width, self.theme.display_font).into(),
                rect: title_rect,
                clip: self.clip,
                font_size,
                colour: self.scene.colour(Colour::WHITE),
                align: TextAlign::Centre,
                scrolling: self.scrolling,
            });
        }

        let content_inset = if media_picker { 0.0 } else if pinned_footer { 16.0 } else if wide { 20.0 } else { self.content_inset() };
        let scroll_track_end = if media_picker { SCROLL_TRACK_END } else { SCROLL_TRACK_END + content_inset - self.content_inset() };
        // A theme that places the bar by the screen edge keeps the content's own end inset.
        let scroll_content_inset_end = if self.theme.scroll_bar_inset.is_some() && !media_picker {
            content_inset
        } else {
            scroll_track_end * 2.0 - SCROLL_TRACK_WIDTH
        };

        let (children, rect) = if pinned_footer {
            let (composer, messages) = children.split_last().expect("screen footer requires a child");
            let inset = self.scaled(content_inset);
            let available = Rect { x: rect.x + inset, width: (rect.width - inset * 2.0).max(0.0), height: (rect.height - header_height).max(0.0), ..rect };
            let size = self.measure(composer, available);
            let bottom = self.scaled(if self.keyboard_inset > 0 { 4.0 } else { CONTENT_BOTTOM });
            let height = size.height + bottom + self.scaled(10.0);
            self.layout(composer, Rect { y: rect.y + rect.height - bottom - size.height, height: size.height, ..available });
            (messages, Rect { height: (rect.height - height).max(header_height), ..rect })
        } else { (children, rect) };

        let rect = if let Some((label, action)) = footer {
            let font_size = self.scaled_font(40.0);
            let inset = self.scaled(self.content_inset());
            let available_width = (rect.width - inset * 2.0).max(0.0);
            let lines = self.wrap_text(label, font_size, available_width, None, false);
            let line_height = self.text_line_height(40.0);
            let height = (line_height * lines.len() as f32 + self.scaled(CONTENT_BOTTOM))
                .min((rect.height - header_height).max(0.0));
            let action_rect = Rect {
                x: rect.x + inset,
                y: rect.y + rect.height - height,
                width: available_width,
                height,
            };
            let scaled = primary_font(self.theme.body_font).as_scaled(PxScale::from(font_size));
            let baseline = (line_height - scaled.height()) / 2.0 + scaled.ascent();
            let ink_bottom = lines.last().into_iter().flat_map(|line| line.text.chars())
                .filter_map(|character| {
                    let (_, font) = font_for_character_in(character, self.theme.body_font);
                    font.outline_glyph(font.glyph_id(character).with_scale(font_size))
                })
                .map(|glyph| glyph.px_bounds().max.y)
                .reduce(f32::max)
                .unwrap_or(0.0);
            let baseline_shift = line_height - baseline - ink_bottom;
            for (index, line) in lines.iter().enumerate() {
                self.scene.text.push(TextRun { font: self.theme.body_font,
                    tabular_numbers: false,
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

        let rect = if let Some(note) = note {
            let inset = self.scaled(self.content_inset());
            let width = (rect.width - inset * 2.0).max(0.0);
            let font_size = self.scaled_font(SCREEN_NOTE_SIZE);
            let lines = self.wrap_text(note, font_size, width, None, false);
            let height = self.text_line_height(SCREEN_NOTE_SIZE) * lines.len() as f32;
            let bottom = self.scaled(CONTENT_BOTTOM);
            let note_rect = Rect { x: rect.x + inset, y: rect.y + rect.height - bottom - height, width, height };
            let start = self.scene.text.len();
            self.layout_text_lines(&lines, SCREEN_NOTE_SIZE, TextAlign::Centre, false, note_rect);
            let colour = self.scene.colour(Colour::MUTED);
            for run in &mut self.scene.text[start..] {
                run.colour = colour;
                run.scrolling = false;
            }
            Rect { height: (rect.height - height - bottom).max(0.0), ..rect }
        } else {
            rect
        };

        let (children, header_height) = if pinned_header && !children.is_empty() {
            let top = self.scaled(if has_header { HEADER_CONTENT_TOP } else { CONTENT_TOP });
            let available = Rect {
                x: rect.x + self.scaled(content_inset),
                y: rect.y + header_height + top,
                width: (rect.width - self.scaled(content_inset * 2.0)).max(0.0),
                height: (rect.height - header_height - top - self.scaled(CONTENT_BOTTOM)).max(0.0),
            };
            let size = self.measure(&children[0], available);
            self.layout(&children[0], Rect { height: size.height, ..available });
            (&children[1..], header_height + size.height + self.scaled(self.content_gap()))
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
            self.scaled(self.content_gap())
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
        if children.len() == 1
            && matches!(children[0].kind, NodeKind::Stack {
                axis: Axis::Vertical, justify: Justification::SpaceBetween, ..
            })
        {
            sizes[0].height = sizes[0].height.max(content.height);
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
            y: scroll_clip.y - scroll_clip.height * SCROLL_OVERSCAN,
            height: scroll_clip.height * (1.0 + 2.0 * SCROLL_OVERSCAN),
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
            let thumb_width = self.scaled(self.theme.scroll_bar_width.unwrap_or(SCROLL_THUMB_WIDTH));
            let track_x = match self.theme.scroll_bar_inset.filter(|_| !media_picker) {
                Some(inset) => rect.x + rect.width - self.scaled(inset) - track_width / 2.0,
                None => rect.x + rect.width - self.scaled(scroll_track_end),
            };
            let track_inset = if media_picker { self.scaled(16.0).min(content.height / 2.0) } else { 0.0 };
            self.scene.scroll_bar = Some(ScrollBar {
                track: Rect {
                    x: track_x,
                    y: content.y + track_inset,
                    width: track_width,
                    height: content.height - track_inset * 2.0,
                },
                thumb_width,
                track_visible: self.theme.scroll_bar_track,
                opacity: self.scroll_bar_opacity(),
            });
        }

        if let Some(back) = self.back_icon.clone().filter(|_| automatic_back) {
            let icon_size = self.scaled(header_icon);
            let optical = if large && back_at_end { self.icon_trailing_space(&back) * icon_size } else { 0.0 };
            self.scene.masks.push(MaskRun {
                mask: back,
                rect: Rect {
                    // The chevron sits optically left within its button; other icons centre.
                    x: if back_at_end || self.theme.back_arrow {
                        back_x + (header_button_size - icon_size) / 2.0
                            + optical
                    } else {
                        header.x + header_inset + self.scaled(HEADER_BACK_OFFSET_X)
                    },
                    y: header_centre - icon_size / 2.0,
                    width: icon_size,
                    height: icon_size,
                },
                clip: self.clip,
                colour: self.scene.colour(Colour::WHITE),
                scrolling: self.scrolling,
            });
        }
    }

    fn layout_image_guides(&mut self, guides: &[f32], rect: Rect) {
        if guides.is_empty() { return; }
        let dash = self.scaled(6.0).round().max(1.0);
        let step = dash + self.scaled(4.0).round().max(1.0);
        let thickness = self.scaled(0.75).round().max(1.0);
        let clip = rect.intersection(self.clip);
        for position in guides {
            let y = (rect.y + rect.height * position).round();
            for index in 0..(rect.width / step).ceil() as usize {
                let x = rect.x.round() + index as f32 * step;
                let width = dash.min(rect.x + rect.width - x);
                // Masks render above the photo and do not participate in crop gestures.
                for (height, colour) in [(thickness + 2.0, Colour::BLACK), (thickness, Colour::WHITE)] {
                    self.scene.masks.push(MaskRun {
                        mask: Mask::solid(),
                        rect: Rect { x, y: y - (height / 2.0).floor(), width, height },
                        clip, colour: self.scene.colour(colour), scrolling: self.scrolling,
                    });
                }
            }
        }
    }

    fn measure_screen_content(
        &mut self,
        children: &[Node],
        available: Rect,
        gap: f32,
    ) -> (Vec<MeasuredSize>, f32) {
        let sizes = self.measure_vertical_children(children, available);
        let height = sizes.iter().map(|measure| measure.height).sum::<f32>()
            + self.vertical_gaps(children, gap).sum::<f32>();
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
            + self.vertical_gaps(children, gap).sum::<f32>();
        let (mut cursor, extra_gap) =
            distribution(rect.y, rect.height, content_height, 0.0, entries, justify);
        let gaps = self.vertical_gaps(children, gap).chain(std::iter::once(0.0));
        for ((child, size), gap_after) in children.iter().zip(sizes).zip(gaps) {
            let bleed = full_bleed_image(child);
            let width = if bleed {
                self.viewport.width as f32
            } else if align == Alignment::Stretch && stretchable(child) {
                rect.width
            } else {
                size.width.min(rect.width)
            };
            let page_centred_child = (matches!(child.kind, NodeKind::Image { .. })
                && matches!(align, Alignment::Centre | Alignment::Stretch)
                || matches!(child.kind, NodeKind::Stack { axis: Axis::Vertical, align: Alignment::Centre, .. }))
                && self.scroll_max > 0.0
                && (rect.x - self.scaled(self.content_inset())).abs() < 1.0
                && (rect.width
                    - (self.viewport.width as f32
                        - self.scaled(self.content_inset())
                        - self.scaled(SCROLL_CONTENT_INSET_END)))
                .abs()
                    < 1.0;
            let x = if bleed {
                0.0
            } else if page_centred_child {
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
            cursor += size.height + gap_after + extra_gap;
        }
    }

    fn vertical_gaps<'a>(&self, children: &'a [Node], gap: f32) -> impl Iterator<Item = f32> + use<'a> {
        let row_gap = self.theme.row_gap.map_or(gap, |row_gap| self.scaled(row_gap).min(gap));
        children.windows(2).map(move |pair| {
            if matches!(pair[0].kind, NodeKind::Row { .. }) && matches!(pair[1].kind, NodeKind::Row { .. }) {
                row_gap
            } else {
                gap
            }
        })
    }

    fn measure_horizontal_children(&mut self, children: &[Node], gap: f32, rect: Rect) -> Vec<MeasuredSize> {
        let mut sizes: Vec<_> = children.iter().map(|child| self.measure(child, rect)).collect();
        let shrinkable = |child: &Node| matches!(child.kind,
            NodeKind::Text { width: None, .. } | NodeKind::Stack { axis: Axis::Vertical, .. }
            | NodeKind::Row { .. } | NodeKind::RowSubtitle { wrap: true, .. });
        let label_width: f32 = children.iter().zip(&sizes)
            .filter(|(child, _)| shrinkable(child)).map(|(_, size)| size.width).sum();
        let total = sizes.iter().map(|size| size.width).sum::<f32>()
            + gap * children.len().saturating_sub(1) as f32;
        if total > rect.width && label_width > 0.0 {
            let remaining = (label_width - (total - rect.width)).max(0.0);
            for (child, size) in children.iter().zip(&mut sizes) {
                if shrinkable(child) {
                    let width = size.width * remaining / label_width;
                    *size = self.measure(child, Rect { width, ..rect });
                }
            }
        }
        sizes
    }

    fn layout_horizontal_children(
        &mut self,
        children: &[Node],
        gap: f32,
        align: Alignment,
        justify: Justification,
        rect: Rect,
    ) {
        let sizes = self.measure_horizontal_children(children, gap, rect);
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
        long_action: &Option<Action>,
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
        // A selected row's tick sits at the end, so the label stops before it.
        let tick_space = if underline && self.theme.selection == SelectionStyle::Check {
            self.scaled(BUTTON_ICON_SIZE * 0.8 + BUTTON_ICON_GAP)
        } else {
            0.0
        };
        let text_rect = Rect {
            x: text_x,
            y: rect.y + self.scaled(1.0),
            width: (rect.x + rect.width - text_x - tick_space).max(0.0),
            height: (rect.height - self.scaled(1.0)).max(0.0),
        };
        let visible_label = self.ellipsize(label, font_size, text_rect.width);
        let text_width = self.text_width(&visible_label, font_size);
        self.scene.text.push(TextRun { font: self.theme.body_font,
            tabular_numbers: false,
            text: visible_label.into(),
            rect: text_rect,
            clip: self.clip,
            font_size,
            colour,
            align: TextAlign::Start,
            scrolling: self.scrolling,
        });
        if underline && self.theme.selection == SelectionStyle::Check {
            let size = self.scaled(BUTTON_ICON_SIZE * 0.8);
            let stroke = self.scaled(2.4);
            let x = rect.x + rect.width - size;
            let y = rect.y + (rect.height - size) / 2.0;
            let points = [[x + size * 0.1, y + size * 0.52], [x + size * 0.38, y + size * 0.8], [x + size * 0.92, y + size * 0.24]];
            for pair in points.windows(2) {
                self.scene.lines.push(Line { from: pair[0], to: pair[1], width: stroke, clip: self.clip, colour, scrolling: self.scrolling });
            }
        } else if underline {
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
            self.push_press_region(rect, action.clone(), long_action.clone());
        }
    }

    fn layout_text(&mut self, text: &str, font_size: Option<f32>, align: TextAlign, max_lines: Option<u32>, tabular_numbers: bool, rect: Rect) {
        let size = font_size.unwrap_or(DEFAULT_TEXT_SIZE);
        let font_size = self.scaled_font(size);
        let lines = self.wrap_text(text, font_size, rect.width, max_lines, tabular_numbers);
        self.layout_text_lines(&lines, size, align, tabular_numbers, rect);
    }

    fn layout_text_lines(&mut self, lines: &[WrappedLine], size: f32, align: TextAlign, tabular_numbers: bool, rect: Rect) {
        let font_size = self.scaled_font(size);
        let line_height = self.text_line_height(size);
        for (index, line) in lines.iter().enumerate() {
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
            self.scene.text.push(TextRun { font: self.theme.body_font,
                tabular_numbers,
                text: line.text.clone(),
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

    /// The label and value, without the image.
    fn measure_field_text(&mut self, label: &str, value: &str, pressable: bool, available: Rect) -> MeasuredSize {
        if self.theme.field == FieldLayout::Inline {
            let lines = self.inline_field_wrap(label, value, pressable, available.width).map_or(0, |lines| lines.len());
            return MeasuredSize {
                width: available.width,
                height: (self.text_line_height(DEFAULT_TEXT_SIZE) * (1 + lines) as f32).min(available.height),
            };
        }
        let label_width = self.text_width(label, self.scaled_font(FIELD_LABEL_SIZE));
        let lines = self.wrap_text(value, self.scaled_font(DEFAULT_TEXT_SIZE), available.width, None, false);
        let value_width = lines.iter().map(|line| line.width).fold(0.0, f32::max);
        let value_height = self.text_line_height(DEFAULT_TEXT_SIZE) * lines.len() as f32;
        MeasuredSize {
            width: label_width.max(value_width).ceil().min(available.width),
            height: (self.scaled(FIELD_LABEL_HEIGHT) + value_height).min(available.height),
        }
    }

    fn layout_field(&mut self, label: &str, value: &str, action: &Option<Action>, disabled: bool, image: Option<&Node>, hit: Rect) {
        let mut rect = hit;
        if let Some(image) = image {
            let size = self.scaled(FIELD_IMAGE_SIZE).min(hit.width);
            self.layout(image, Rect { width: size, height: size, y: hit.y + (hit.height - size) / 2.0, ..hit });
            let slot = self.scaled(FIELD_IMAGE_SLOT).min(hit.width);
            let available = Rect { x: hit.x + slot, width: (hit.width - slot).max(0.0), ..hit };
            let height = self.measure_field_text(label, value, action.is_some(), available).height.min(hit.height);
            rect = Rect { y: hit.y + (hit.height - height) / 2.0, height, ..available };
        }
        if self.theme.field == FieldLayout::Inline {
            return self.layout_inline_field(label, value, action, disabled, rect, hit);
        }
        let start = self.scene.text.len();
        let label_height = self.scaled(FIELD_LABEL_HEIGHT).min(rect.height);
        let label_font_size = self.scaled_font(FIELD_LABEL_SIZE);
        self.scene.text.push(TextRun { font: self.theme.body_font,
            tabular_numbers: false,
            text: self.ellipsize(label, label_font_size, rect.width).into(),
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
            false,
            Rect {
                y: rect.y + label_height,
                height: (rect.height - label_height).max(0.0),
                ..rect
            },
        );
        if disabled {
            let colour = self.scene.colour(Colour::MUTED);
            for run in &mut self.scene.text[start..] {
                run.colour = colour;
            }
        } else if let Some(action) = action {
            self.push_hit_region(hit, action.clone());
        }
    }

    /// Width reserved for an inline field's chevron and its gap, or zero without an action.
    fn inline_field_chevron(&self, pressable: bool) -> f32 {
        if pressable { self.scaled(BUTTON_ICON_SIZE * 0.6 + BUTTON_ICON_GAP) } else { 0.0 }
    }

    /// The value's lines when an inline field's label and value don't fit on one line.
    fn inline_field_wrap(&mut self, label: &str, value: &str, pressable: bool, width: f32) -> Option<WrappedLines> {
        let font_size = self.scaled_font(DEFAULT_TEXT_SIZE);
        let available = (width - self.inline_field_chevron(pressable)).max(0.0);
        let one_line = self.text_width(label, font_size) + self.scaled(BUTTON_ICON_GAP) + self.text_width(value, font_size);
        (!value.is_empty() && one_line > available).then(|| self.wrap_text(value, font_size, available, None, false))
    }

    /// Label at the start with the value and a chevron at the end. Values that don't fit wrap
    /// under the label.
    fn layout_inline_field(&mut self, label: &str, value: &str, action: &Option<Action>, disabled: bool, rect: Rect, hit: Rect) {
        let font_size = self.scaled_font(DEFAULT_TEXT_SIZE);
        let chevron_space = self.inline_field_chevron(action.is_some());
        let available = (rect.width - chevron_space).max(0.0);
        let line_height = self.text_line_height(DEFAULT_TEXT_SIZE);
        let first = Rect { y: rect.y + self.scaled(1.0), height: (line_height - self.scaled(1.0)).max(0.0), width: available, ..rect };
        let primary = self.scene.colour(if disabled { Colour::MUTED } else { Colour::WHITE });
        let muted = self.scene.colour(Colour::MUTED);
        let mut runs = Vec::new();
        if let Some(lines) = self.inline_field_wrap(label, value, action.is_some(), rect.width) {
            runs.push((self.ellipsize(label, font_size, available), first, primary, TextAlign::Start));
            for (index, line) in lines.iter().enumerate() {
                runs.push((line.text.to_string(), Rect { y: first.y + line_height * (index + 1) as f32, ..first }, muted, TextAlign::Start));
            }
        } else {
            let label_width = self.text_width(label, font_size).ceil().min(available);
            let value_width = (available - label_width - self.scaled(BUTTON_ICON_GAP)).max(0.0);
            runs.push((label.to_owned(), Rect { width: label_width, ..first }, primary, TextAlign::Start));
            runs.push((value.to_owned(), Rect { x: rect.x + available - value_width, width: value_width, ..first }, muted, TextAlign::End));
        }
        for (text, run_rect, colour, align) in runs {
            self.scene.text.push(TextRun { font: self.theme.body_font, text: text.into(), rect: run_rect, clip: self.clip, font_size, colour, align, tabular_numbers: false, scrolling: self.scrolling });
        }
        if chevron_space > 0.0 {
            let size = self.scaled(BUTTON_ICON_SIZE * 0.6);
            let x = rect.x + rect.width - size * 0.7;
            let centre = rect.y + line_height / 2.0;
            let half = size / 2.0;
            let points = [[x, centre - half], [x + half * 0.9, centre], [x, centre + half]];
            for pair in points.windows(2) {
                self.scene.lines.push(Line { from: pair[0], to: pair[1], width: self.scaled(2.0), clip: self.clip, colour: muted, scrolling: self.scrolling });
            }
        }
        if let Some(action) = action.as_ref().filter(|_| !disabled) {
            self.push_hit_region(hit, action.clone());
        }
    }

    fn layout_toggle(
        &mut self,
        label: &str,
        subtitle: Option<&str>,
        enabled: bool,
        action: Option<&Action>,
        off: Mask,
        on: Mask,
        rect: Rect,
    ) {
        let colour = self.scene.colour(if action.is_some() { Colour::WHITE } else { Colour::MUTED });
        if self.theme.toggle != ToggleStyle::Line {
            let control = self.scaled(self.toggle_control_width());
            self.layout_toggle_control(enabled, action.is_some(), Rect { x: rect.x + rect.width - control, width: control, ..rect });
            let label_rect = Rect { width: (rect.width - control - self.scaled(TOGGLE_LABEL_GAP)).max(0.0), ..rect };
            return self.layout_toggle_label(label, subtitle, colour, action, label_rect, rect);
        }
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
                colour,
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
                colour,
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
                colour,
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
                colour,
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
        self.layout_toggle_label(label, subtitle, colour, action, label_rect, rect);
    }

    /// Reference width of the toggle control, excluding the gap before the label.
    fn icon_trailing_space(&mut self, icon: &Mask) -> f32 {
        *self.icon_trailing_space.entry(icon.id).or_insert_with(|| {
            icon.content_bounds().map_or(0.0, |bounds| 1.0 - bounds.x - bounds.width)
        })
    }

    fn toggle_control_width(&self) -> f32 {
        match &self.theme.toggle {
            ToggleStyle::Line => TOGGLE_START + TOGGLE_ICON_SIZE + TOGGLE_LINE_WIDTH,
            ToggleStyle::Switch => SWITCH_WIDTH,
            ToggleStyle::Custom { width, .. } => *width,
        }
    }

    fn layout_toggle_control(&mut self, on: bool, enabled: bool, rect: Rect) {
        let alpha = if enabled { 1.0 } else { 0.5 };
        match &self.theme.toggle {
            ToggleStyle::Line => {}
            ToggleStyle::Custom { height, images, .. } => {
                let height = self.scaled(*height);
                self.scene.images.push(ImageRun {
                    avatar: None,
                    image: images[self.scene.light as usize][on as usize + 2 * !enabled as usize].clone(),
                    zoom_id: None,
                    rect: Rect { y: rect.y + (rect.height - height) / 2.0, height, ..rect },
                    clip: self.clip,
                    fit: ImageFit::Contain,
                    scrolling: self.scrolling,
                    transform: ImageTransform::default(),
                });
            }
            ToggleStyle::Switch => {
                let width = rect.width.round();
                let height = self.scaled(SWITCH_HEIGHT).round();
                let inset = self.scaled(SWITCH_KNOB_INSET).round();
                let knob = height - inset * 2.0;
                if self.switch_masks.as_ref().is_none_or(|(w, h, ..)| (*w, *h) != (width as u32, height as u32)) {
                    self.switch_masks = Some((
                        width as u32,
                        height as u32,
                        masks::rounded_rect(width as u32, height as u32, height / 2.0),
                        masks::rounded_rect(knob as u32, knob as u32, knob / 2.0),
                    ));
                }
                let (_, _, track, circle) = self.switch_masks.clone().expect("switch masks are cached above");
                let y = (rect.y + (rect.height - height) / 2.0).round();
                let x = rect.x.round();
                let (track_colour, knob_colour) = if on { (Colour::WHITE, Colour::BLACK) } else { (Colour::MUTED, Colour::WHITE) };
                let track_colour = self.scene.colour(Colour { alpha, ..track_colour });
                let knob_colour = self.scene.colour(Colour { alpha: if on { 1.0 } else { alpha }, ..knob_colour });
                self.scene.masks.push(MaskRun {
                    mask: track,
                    rect: Rect { x, y, width, height },
                    clip: self.clip,
                    colour: track_colour,
                    scrolling: self.scrolling,
                });
                self.scene.masks.push(MaskRun {
                    mask: circle,
                    rect: Rect { x: if on { x + width - inset - knob } else { x + inset }, y: y + inset, width: knob, height: knob },
                    clip: self.clip,
                    colour: knob_colour,
                    scrolling: self.scrolling,
                });
            }
        }
    }

    fn layout_toggle_label(&mut self, label: &str, subtitle: Option<&str>, colour: Colour, action: Option<&Action>, label_rect: Rect, rect: Rect) {
        let label_font_size = self.scaled_font(DEFAULT_TEXT_SIZE);
        let lines = self.wrap_text(label, label_font_size, label_rect.width, None, false);
        let text_height = self.text_line_height(DEFAULT_TEXT_SIZE) * lines.len() as f32;
        let subtitle_size = self.toggle_subtitle_size();
        let subtitle_height = if subtitle.is_some() { self.text_line_height(subtitle_size) } else { 0.0 };
        let text_y = rect.y + (rect.height - text_height - subtitle_height).max(0.0) / 2.0;
        let start = self.scene.text.len();
        self.layout_text(
            label, None, TextAlign::Start, None, false,
            Rect {
                y: text_y,
                height: text_height.min(rect.height),
                ..label_rect
            },
        );
        let subtitle_start = self.scene.text.len();
        if let Some(subtitle) = subtitle {
            self.layout_text(subtitle, Some(subtitle_size), TextAlign::Start, Some(1), true,
                Rect { y: text_y + text_height, height: subtitle_height, ..label_rect });
        }
        for run in &mut self.scene.text[start..] {
            run.colour = colour;
        }
        if self.theme.subtitle_muted == Some(true) {
            let muted = self.scene.colour(Colour::MUTED);
            for run in &mut self.scene.text[subtitle_start..] {
                run.colour = muted;
            }
        }
        if let Some(action) = action {
            self.push_hit_region(rect, action.clone());
        }
    }

    fn layout_tabs(&mut self, value: &usize, tabs: &[Tab], rect: Rect) {
        let keyboard_visible = self.keyboard_inset > 0;
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
        retain_while_loading: bool,
        looping: bool,
        avatar: Option<bool>,
        rotation: Option<u16>,
        crop: Option<ImageCrop>,
        crop_events: bool,
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
        let mut failed = false;
        let image = match source {
            ImageSource::Asset(asset) => Some(ImageData::Asset(asset.clone())),
            ImageSource::Native(module, source) => {
                if source.is_empty() {
                    fallback.cloned().map(ImageData::Asset)
                } else {
                    // Editing uses one bounded, orientation-independent decode. Fit and
                    // quarter turns only change the draw, never the source or texture ID.
                    let editing_size = (rect.width.max(rect.height).ceil().max(1.0) as u32 * 2).min(2048);
                    let key = RemoteImageKey {
                        looping,
                        module: module.clone(),
                        url: source.clone(),
                        width: if rotation.is_some() { editing_size } else { rect.width.ceil().max(1.0) as u32 },
                        height: if rotation.is_some() { editing_size } else if module == "barcode" { rect.width.ceil().max(1.0) as u32 } else { rect.height.ceil().max(1.0) as u32 },
                        fit: if rotation.is_some() { ImageFit::Contain } else { fit },
                    };
                    self.visible_images.insert(key.clone());
                    let loaded = match self.remote_images.get(&key) {
                        Some(RemoteImageState::Ready { image, .. }) => {
                            Some(ImageData::Remote(image.clone()))
                        }
                        Some(RemoteImageState::Failed) => { failed = true; None },
                        _ => None,
                    };
                    if loaded.is_none() {
                        if visible.width > 0.0 && visible.height > 0.0 {
                            self.pending_images.push(PendingImage {
                                avatar,
                                key: key.clone(),
                                index: self.scene.images.len(),
                                rect,
                                clip: self.clip,
                                fit,
                                scrolling: self.scrolling,
                                needs_layout: zoomable || fallback.is_some() || retain_while_loading || rotation.is_some(),
                            });
                        }
                        self.queue_remote_image(key);
                    }
                    loaded.or_else(|| fallback.cloned().map(ImageData::Asset))
                }
            }
        };
        if visible.width <= 0.0 || visible.height <= 0.0 { return; }
        let image = if retain_while_loading {
            self.visible_retained_images.insert(identity);
            if let Some(image) = &image {
                self.retained_images.insert(identity, image.clone());
            } else if failed {
                self.retained_images.remove(&identity);
            }
            image.or_else(|| self.retained_images.get(&identity).cloned())
        } else { image };
        if let Some(image) = image {
            let rotation = rotation.unwrap_or(0);
            let transform = if zoomable {
                let content = image_content_rect(&image, rect, fit, rotation);
                let zoom = self.image_zooms.entry(identity).or_insert(ImageZoomState {
                    viewport: rect,
                    content,
                    scrolling: self.scrolling,
                    transform: ImageTransform::default(),
                    crop: None,
                    crop_events,
                });
                zoom.viewport = rect;
                zoom.content = content;
                zoom.scrolling = self.scrolling;
                zoom.crop_events = crop_events;
                if crop != zoom.crop {
                    if let Some(crop) = crop {
                        zoom.transform = ImageTransform {
                            scale: crop.zoom,
                            translation_x: rect.x + rect.width / 2.0 - (content.x + crop.x * content.width) * crop.zoom,
                            translation_y: rect.y + rect.height / 2.0 - (content.y + crop.y * content.height) * crop.zoom,
                            ..zoom.transform
                        };
                    }
                    zoom.crop = crop;
                }
                zoom.transform.rotation = rotation;
                zoom.transform = constrain_image_transform(*zoom, zoom.transform);
                self.visible_zoom_images.insert(identity);
                zoom.transform
            } else {
                ImageTransform { rotation, ..ImageTransform::default() }
            };
            self.scene.images.push(ImageRun {
                avatar,
                image,
                zoom_id: zoomable.then_some(identity.0),
                rect,
                clip: self.clip,
                fit,
                scrolling: self.scrolling,
                transform,
            });
        } else if zoomable && crop.is_none() {
            let height = self.text_line_height(18.0);
            self.layout_text(
                if failed { "Could not load photo" } else { "Loading…" },
                Some(18.0), TextAlign::Centre, Some(1), false,
                Rect { y: rect.y + (rect.height - height) / 2.0, height, ..rect },
            );
        }
    }

    fn layout_camera_preview(
        &mut self,
        controller: ControllerId,
        kind: CameraPreviewKind,
        rect: Rect,
    ) {
        if !cfg!(feature = "camera") { return; }
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
        &mut self,
        text: &str,
        font_size: f32,
        available_width: f32,
        max_lines: Option<u32>,
        tabular_numbers: bool,
    ) -> WrappedLines {
        self.wrap_linked_text(text, font_size, available_width, max_lines, tabular_numbers, &[])
    }

    fn wrap_linked_text(
        &mut self,
        text: &str,
        font_size: f32,
        available_width: f32,
        max_lines: Option<u32>,
        tabular_numbers: bool,
        links: &[(std::ops::Range<usize>, Action)],
    ) -> WrappedLines {
        self.wrap_inset_text(text, font_size, available_width, max_lines, tabular_numbers, links, 0.0)
    }

    fn wrap_inset_text(
        &mut self,
        text: &str,
        font_size: f32,
        available_width: f32,
        max_lines: Option<u32>,
        tabular_numbers: bool,
        links: &[(std::ops::Range<usize>, Action)],
        first_line_inset: f32,
    ) -> WrappedLines {
        let key = TextWrapKey {
            font: self.theme.body_font,
            first_line_inset: first_line_inset.to_bits(),
            text: text.into(),
            font_size: font_size.to_bits(),
            width: available_width.to_bits(),
            max_lines,
            tabular_numbers,
            links: links.iter().map(|(range, _)| range.clone()).collect(),
        };
        let entry = match self.wrapped_text.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let (lines, used) = entry.get_mut();
                *used = true;
                return lines.clone();
            }
            std::collections::hash_map::Entry::Vacant(entry) => entry,
        };
        let lines = Self::wrap_lines(text, font_size, available_width, max_lines, tabular_numbers, links, first_line_inset, self.theme.body_font);
        entry.insert((lines.clone(), true));
        lines
    }

    fn wrap_lines(
        text: &str,
        font_size: f32,
        available_width: f32,
        max_lines: Option<u32>,
        tabular_numbers: bool,
        links: &[(std::ops::Range<usize>, Action)],
        first_line_inset: f32,
        role: FontRole,
    ) -> WrappedLines {
        // Short labels that fit need no line-break candidates or truncation.
        if text.len() <= 64 && links.is_empty() && !text.contains('\n') && max_lines != Some(0) {
            let display = text.trim_end();
            let width = text_width_in(display, font_size, tabular_numbers, role);
            if width <= (available_width - first_line_inset).max(0.0) {
                let lines = WrappedLines::Single(WrappedLine {
                    text: display.into(), width, wrapped: false,
                });
                return lines;
            }
        }
        let unbroken_links = links.iter().filter_map(|(range, _)| {
            (text_width_in(&text[range.clone()], font_size, tabular_numbers, role) <= available_width)
                .then_some(range)
        }).collect::<Vec<_>>();
        let mut lines = Vec::new();
        let mut paragraph_offset = 0;
        for paragraph in text.split('\n') {
            let offset = paragraph_offset;
            paragraph_offset += paragraph.len() + 1;
            if paragraph.is_empty() {
                lines.push(WrappedLine {
                    text: "".into(),
                    width: 0.0,
                    wrapped: false,
                });
                continue;
            }

            let rtl = fonts::has_rtl(paragraph);
            let protected = unbroken_links.iter().copied().filter(|range| {
                range.start >= offset && range.end <= offset + paragraph.len()
            }).collect::<Vec<_>>();
            let mut breakpoints = linebreaks(paragraph)
                .map(|(index, _)| index)
                .filter(|index| !protected.iter().any(|range| range.start < offset + index && offset + index < range.end))
                .collect::<Vec<_>>();
            if !protected.is_empty() {
                for range in protected {
                    breakpoints.extend([range.start - offset, range.end - offset]);
                }
                breakpoints.sort_unstable();
                breakpoints.dedup();
            }
            let mut start = 0;
            while start < paragraph.len() {
                let available_width = (available_width - if lines.is_empty() { first_line_inset } else { 0.0 }).max(0.0);
                let mut best = None;
                let mut measured_end = start;
                let mut measure = fonts::TextWidth::new_in(font_size, tabular_numbers, role);
                for end in breakpoints.iter().copied().filter(|end| *end > start) {
                    let display_end = trim_whitespace_end(paragraph, start, end);
                    let width = if rtl {
                        text_width_in(&paragraph[start..display_end], font_size, tabular_numbers, role)
                    } else { measure.push(&paragraph[measured_end..display_end]) };
                    measured_end = display_end;
                    if width <= available_width {
                        best = Some((end, display_end, width));
                    } else {
                        break;
                    }
                }

                let (end, display_end, width) = best.unwrap_or_else(|| {
                    let end = Self::forced_text_break(paragraph, start, font_size, available_width, tabular_numbers, role);
                    (end, end, text_width_in(&paragraph[start..end], font_size, tabular_numbers, role))
                });
                lines.push(WrappedLine {
                    text: paragraph[start..display_end].into(),
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
                line.text = Self::ellipsize_forced(&line.text, font_size, (available_width - if max_lines == 1 { first_line_inset } else { 0.0 }).max(0.0), tabular_numbers, role).into();
                line.width = text_width_in(&line.text, font_size, tabular_numbers, role);
                line.wrapped = false;
            }
        }
        lines.into()
    }

    fn forced_text_break(
        text: &str,
        start: usize,
        font_size: f32,
        available_width: f32,
        tabular_numbers: bool,
        role: FontRole,
    ) -> usize {
        let mut best = start;
        let rtl = fonts::has_rtl(text);
        let mut measure = fonts::TextWidth::new_in(font_size, tabular_numbers, role);
        for (offset, grapheme) in text[start..].grapheme_indices(true) {
            let end = start + offset + grapheme.len();
            let width = if rtl {
                text_width_in(&text[start..end], font_size, tabular_numbers, role)
            } else { measure.push(grapheme) };
            if best > start && width > available_width {
                break;
            }
            best = end;
        }
        best
    }

    fn text_line_height(&self, size: f32) -> f32 {
        self.scaled(if size == DEFAULT_TEXT_SIZE {
            BUTTON_HEIGHT
        } else {
            size * 1.25
        })
    }

    fn text_width(&self, text: &str, font_size: f32) -> f32 {
        text_width_in(text, font_size, false, self.theme.body_font)
    }

    fn text_cursor_for_offset(&self, text: &str, font_size: f32, offset: f32, role: FontRole) -> usize {
        let mut previous = None;
        let mut width = 0.0;
        for (index, grapheme) in text.grapheme_indices(true) {
            let start = width;
            if is_emoji_grapheme(grapheme) {
                width += font_size;
                previous = None;
            } else {
                for character in grapheme.chars() {
                    let (font_index, font) = font_for_character_in(character, role);
                    let scaled = font.as_scaled(PxScale::from(font_size));
                    let glyph = scaled.glyph_id(character);
                    if let Some((previous_index, previous_glyph)) = previous {
                        if previous_index == font_index {
                            width += scaled.kern(previous_glyph, glyph);
                        }
                    }
                    width += scaled.h_advance(glyph);
                    previous = Some((font_index, glyph));
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
        Self::ellipsize_forced(text, font_size, available_width, false, self.theme.body_font)
    }

    fn ellipsize_in(text: &str, font_size: f32, available_width: f32, role: FontRole) -> String {
        let width = |text: &str| visual_text_width_in(&visual_text(text), font_size, false, role);
        if width(text) <= available_width {
            return text.to_owned();
        }
        let ellipsis_width = width("…");
        let mut visible = text.to_owned();
        while !visible.is_empty() && width(&visible) + ellipsis_width > available_width {
            let Some((index, _)) = visible.grapheme_indices(true).next_back() else { break };
            visible.truncate(index);
        }
        visible.push('…');
        visible
    }

    fn ellipsize_forced(text: &str, font_size: f32, available_width: f32, tabular_numbers: bool, role: FontRole) -> String {
        let ellipsis = '…';
        let ellipsis_width = text_width_in("…", font_size, false, role);
        let mut visible = text.to_owned();
        while !visible.is_empty()
            && text_width_in(&visible, font_size, tabular_numbers, role) + ellipsis_width > available_width
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
        self.scaled_font_in(value, self.theme.body_font)
    }

    fn scaled_font_in(&self, value: f32, role: FontRole) -> f32 {
        (self.scaled(value) * fonts::raster_scale(role).unwrap_or(PUBLIC_SANS_RASTER_SCALE))
            .round()
            .clamp(1.0, u16::MAX as f32)
    }

    fn content_inset(&self) -> f32 {
        self.theme.inset.unwrap_or(CONTENT_INSET_START)
    }

    /// Toggle and row subtitles share a size.
    fn toggle_subtitle_size(&self) -> f32 {
        self.theme.subtitle_size.unwrap_or(16.0)
    }

    fn header_subtitle_size(&self) -> f32 {
        self.theme.subtitle_size.unwrap_or(LARGE_HEADER_SUBTITLE_SIZE)
    }

    fn row_title_size(&self, size: f32, themed: bool) -> f32 {
        if themed { self.theme.row_size.unwrap_or(size) } else { size }
    }

    fn section_size(&self) -> f32 {
        self.theme.section_size.unwrap_or(SECTION_TEXT_SIZE)
    }

    /// Space between a section title and its rows.
    fn section_gap(&self) -> f32 {
        self.theme.section_gap.unwrap_or(self.content_gap() * 0.5)
    }

    fn stack_gap(&self, gap: Option<f32>, theme_gap: Option<ThemeGap>) -> f32 {
        match theme_gap {
            Some(ThemeGap::Content) => self.content_gap(),
            Some(ThemeGap::Section) => self.section_gap(),
            None => gap.unwrap_or_default(),
        }
    }

    fn content_gap(&self) -> f32 {
        self.theme.gap.unwrap_or(CONTENT_GAP)
    }

    fn control_line_height(&self) -> f32 {
        self.scaled(CONTROL_LINE_HEIGHT).ceil().max(1.0)
    }

    fn press_target(&self, action: &Action) -> PressTarget {
        let (node, role) = match action {
            Action::CanvasRange { id } => (NodeIdentity(*id), PressRole::CanvasRange),
            Action::Native { operation } => match operation.event_target {
                Some((id, name)) => (NodeIdentity(id), PressRole::Event(name)),
                None => (self.layout_owner, PressRole::Native),
            },
            Action::Seek { id, .. } => (NodeIdentity(*id), PressRole::Seek),
            Action::FocusTextInput { .. } => (self.layout_owner, PressRole::Focus),
            Action::ClearInput { .. } => (self.layout_owner, PressRole::Clear),
            Action::Back => (self.layout_owner, PressRole::Back),
        };
        PressTarget { node, role }
    }

    fn hit_region_at(&self, x: f32, y: f32) -> Option<&HitRegion> {
        self.hit_regions.iter().rev().find(|region| {
            if region.scrolling && !self.scene.scroll_clip.is_some_and(|clip| clip.contains(x, y)) {
                return false;
            }
            let local_y = if region.scrolling { y + self.scroll_offset - self.scroll_origin } else { y };
            region.rect.contains(x, local_y)
        })
    }

    fn push_hit_region(&mut self, rect: Rect, action: Action) {
        self.push_press_region(rect, action, None);
    }

    fn push_press_region(&mut self, rect: Rect, action: Action, long_action: Option<Action>) {
        let rect = self.hit_rect(rect, &action);
        if rect.width > 0.0 && rect.height > 0.0 {
            let target = self.press_target(&action);
            self.hit_regions.push(HitRegion {
                rect,
                action,
                long_action,
                drag_group: None,
                drag_action: None,
                drag_start: None,
                drag_end: None,
                scrolling: self.scrolling,
                preserve_input: false,
                haptic: true,
                target,
            });
        }
    }

    fn hit_rect(&self, rect: Rect, action: &Action) -> Rect {
        let action_slop = match action {
            Action::Native { operation } => operation.event_target
                .and_then(|(id, event)| self.action_hit_slops.get(&(NodeIdentity(id), event)).copied()),
            _ => None,
        };
        action_slop.or_else(|| self.hit_slops.get(&self.layout_owner).copied()).unwrap_or(self.theme.hit_slop)
            .expand(rect, self.viewport.scale).intersection(self.clip)
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

pub fn is_emoji_grapheme(grapheme: &str) -> bool {
    if grapheme.is_ascii() || grapheme.chars().any(is_text_presentation_selector) {
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

#[derive(Clone, Hash, PartialEq, Eq)]
struct TextWrapKey {
    font: FontRole,
    first_line_inset: u32,
    text: SmolStr,
    font_size: u32,
    width: u32,
    max_lines: Option<u32>,
    tabular_numbers: bool,
    links: Vec<std::ops::Range<usize>>,
}

#[derive(Clone)]
struct WrappedLine {
    text: SmolStr,
    width: f32,
    wrapped: bool,
}

#[derive(Clone)]
enum WrappedLines {
    Single(WrappedLine),
    Multiple(Arc<[WrappedLine]>),
}

impl std::ops::Deref for WrappedLines {
    type Target = [WrappedLine];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Single(line) => std::slice::from_ref(line),
            Self::Multiple(lines) => lines,
        }
    }
}

impl From<Vec<WrappedLine>> for WrappedLines {
    fn from(mut lines: Vec<WrappedLine>) -> Self {
        if lines.len() == 1 {
            Self::Single(lines.pop().unwrap())
        } else {
            Self::Multiple(lines.into())
        }
    }
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
            | NodeKind::RowTitle { .. }
            | NodeKind::PlayingTransport { .. }
            | NodeKind::Row { .. }
            | NodeKind::Text { .. }
            | NodeKind::TextInput { .. }
            | NodeKind::Button { .. }
            | NodeKind::Field { .. }
            | NodeKind::CameraPreview { .. }
            | NodeKind::VideoView { .. } | NodeKind::MapView { .. }
            | NodeKind::MediaGridRow { .. }
            | NodeKind::MediaCell { .. }
            | NodeKind::SectionTitle { .. }
            | NodeKind::RowSubtitle { .. }
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
        NodeKind::PlayingLayout { .. } | NodeKind::CameraPreview { .. } | NodeKind::VideoView { .. } | NodeKind::MapView { .. }
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
