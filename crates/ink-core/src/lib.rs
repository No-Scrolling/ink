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

mod definition;
mod persistence;

pub use definition::AppDefinitionError;
pub use persistence::PersistenceTooLarge;
use persistence::{decode_persisted_state, encode_persisted_state};

pub const PUBLIC_SANS: &[u8] = include_bytes!("../../../assets/fonts/PublicSans-Regular.ttf");

const DEFAULT_TEXT_SIZE: f32 = 30.0;
const TEXT_INPUT_TEXT_SIZE: f32 = 24.0;
const TEXT_INPUT_HEIGHT: f32 = 38.0;
const TEXT_INPUT_BOTTOM_PADDING: f32 = 6.0;
const CONTROL_LINE_HEIGHT: f32 = 1.0;
const DEFAULT_ICON_SIZE: f32 = 28.0;
const BUTTON_HEIGHT: f32 = 40.0;
const BUTTON_ICON_SIZE: f32 = 30.0;
const BUTTON_ICON_GAP: f32 = 12.0;
const FIELD_LABEL_SIZE: f32 = 20.0;
const FIELD_LABEL_HEIGHT: f32 = 25.0;
const FIELD_HEIGHT: f32 = FIELD_LABEL_HEIGHT + BUTTON_HEIGHT;
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
const CAMERA_REVIEW_ACTION_HEIGHT: f32 = 64.0;
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
const SCROLL_CONTENT_INSET_END: f32 = 52.0;
const SCROLL_TRACK_END: f32 = 34.0;
const SCROLL_TRACK_WIDTH: f32 = 1.0;
const SCROLL_THUMB_WIDTH: f32 = 5.0;
const TAP_SLOP: f32 = 8.0;
const LP3_REFERENCE_WIDTH: f32 = 1080.0;
const LP3_REFERENCE_SCALE: f32 = 2.55;
const PUBLIC_SANS_RASTER_SCALE: f32 = 7.0 / 6.0;
const INCREMENTAL_TREE_THRESHOLD: usize = 32;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct StateId(usize);

impl StateId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceId(usize);

impl ResourceId {
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
    Object(Vec<(String, StateShape)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StateLiteral {
    Number(f64),
    Bool(bool),
    String(String),
}

impl StateShape {
    fn accepts(&self, value: &StateValue) -> bool {
        match (self, value) {
            (Self::Null, StateValue::Null) => true,
            (Self::Number, StateValue::Number(value)) if value.is_finite() => true,
            (Self::Bool, StateValue::Bool(_)) | (Self::String, StateValue::String(_)) => true,
            (Self::Literal(expected), value) => expected.accepts(value),
            (Self::Optional(_), StateValue::Null) => true,
            (Self::Optional(shape), value) => shape.accepts(value),
            (Self::Union(shapes), value) => shapes.iter().any(|shape| shape.accepts(value)),
            (Self::List(item), StateValue::List(values)) => {
                values.iter().all(|value| item.accepts(value))
            }
            (Self::Object(fields), StateValue::Object(values)) => {
                values
                    .iter()
                    .all(|(name, _)| fields.iter().any(|(field_name, _)| field_name == name))
                    && fields.iter().all(|(name, shape)| {
                        values
                            .iter()
                            .find(|(value_name, _)| value_name == name)
                            .map_or(matches!(shape, StateShape::Optional(_)), |(_, value)| {
                                shape.accepts(value)
                            })
                    })
            }
            _ => false,
        }
    }
}

impl StateLiteral {
    fn accepts(&self, value: &StateValue) -> bool {
        match (self, value) {
            (Self::Number(expected), StateValue::Number(value)) => expected == value,
            (Self::Bool(expected), StateValue::Bool(value)) => expected == value,
            (Self::String(expected), StateValue::String(value)) => expected == value,
            _ => false,
        }
    }
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
pub struct StateDefinition {
    initial: StateValue,
    shape: StateShape,
    persisted: Option<PersistedState>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeOperation {
    module: String,
    operation: String,
    payload: Vec<PayloadPart>,
    timeout_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PayloadPart {
    Literal(String),
    State(StateId),
    Item(Vec<String>),
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
            payload: vec![PayloadPart::Literal(payload.into())],
            timeout_ms,
        }
    }

    pub fn templated(
        module: impl Into<String>,
        operation: impl Into<String>,
        payload: Vec<PayloadPart>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            module: module.into(),
            operation: operation.into(),
            payload,
            timeout_ms,
        }
    }

    fn dependencies(&self) -> impl Iterator<Item = StateId> + '_ {
        self.payload.iter().filter_map(|part| match part {
            PayloadPart::State(state) => Some(*state),
            PayloadPart::Literal(_) | PayloadPart::Item(_) => None,
        })
    }

    fn materialise(&self, state: &[StateValue]) -> Option<String> {
        let mut output = String::new();
        for part in &self.payload {
            match part {
                PayloadPart::Literal(value) => output.push_str(value),
                PayloadPart::State(id) => {
                    output.push_str(&json_value(state.get(id.0)?)?);
                }
                PayloadPart::Item(_) => return None,
            }
        }
        Some(output)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceDefinition {
    shape: StateShape,
    read: NativeOperation,
    reload_on_resume: bool,
    protocol: ResourceProtocol,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceProtocol {
    Async,
    Cached,
    Mutation,
    Background,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerDefinition {
    state: StateId,
    module: String,
    kind: String,
    config: String,
}

impl ControllerDefinition {
    pub fn new(
        state: StateId,
        module: impl Into<String>,
        kind: impl Into<String>,
        config: impl Into<String>,
    ) -> Self {
        Self {
            state,
            module: module.into(),
            kind: kind.into(),
            config: config.into(),
        }
    }
}

impl ResourceDefinition {
    pub const fn new(shape: StateShape, read: NativeOperation, reload_on_resume: bool) -> Self {
        Self {
            shape,
            read,
            reload_on_resume,
            protocol: ResourceProtocol::Async,
        }
    }

    pub const fn with_protocol(
        shape: StateShape,
        read: NativeOperation,
        reload_on_resume: bool,
        protocol: ResourceProtocol,
    ) -> Self {
        Self {
            shape,
            read,
            reload_on_resume,
            protocol,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceErrorKind {
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

#[derive(Clone, Debug, PartialEq)]
enum ResourceState {
    Inactive,
    Loading {
        previous: Option<StateValue>,
    },
    Ready(StateValue),
    Failed {
        error: ResourceError,
        previous: Option<StateValue>,
    },
    BackgroundWaiting,
    BackgroundReady {
        value: StateValue,
        updated_at_ms: f64,
        error: Option<BackgroundError>,
    },
    BackgroundFailed(BackgroundError),
}

#[derive(Clone, Debug, PartialEq)]
struct BackgroundError {
    kind: String,
    message: String,
    retryable: bool,
    attempted_at_ms: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct NavigationEntry {
    route: usize,
    params: Vec<(String, StateValue)>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeRequestKind {
    ResourceRead,
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

impl StateDefinition {
    pub fn local(initial: StateValue, shape: StateShape) -> Self {
        Self {
            initial,
            shape,
            persisted: None,
        }
    }

    pub fn shared(initial: StateValue, shape: StateShape) -> Self {
        Self::local(initial, shape)
    }

    pub fn persisted(
        initial: StateValue,
        key: impl Into<String>,
        schema: u64,
        shape: StateShape,
    ) -> Self {
        Self {
            initial,
            shape: shape.clone(),
            persisted: Some(PersistedState {
                key: key.into(),
                schema,
                shape,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PersistedState {
    key: String,
    schema: u64,
    shape: StateShape,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hydration {
    #[default]
    Empty,
    Restored,
    Invalid,
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
    RemoveListItem {
        state: StateId,
        index: usize,
    },
    ReplaceCurrentListItem {
        state: StateId,
        value: Value,
    },
    ReplaceListItem {
        state: StateId,
        index: usize,
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
    FocusTextInput {
        state: StateId,
        action: TextInputAction,
    },
    Navigate {
        path: String,
        params: Vec<(String, Value)>,
    },
    Back,
    Sequence(Vec<Action>),
}

fn action_state(action: &Action) -> Option<StateId> {
    match action {
        Action::Increment { state, .. }
        | Action::SetValue { state, .. }
        | Action::Toggle { state }
        | Action::SetList { state, .. }
        | Action::AppendList { state, .. }
        | Action::RemoveCurrentListItem { state }
        | Action::RemoveListItem { state, .. }
        | Action::ReplaceCurrentListItem { state, .. }
        | Action::ReplaceListItem { state, .. }
        | Action::ClearList { state } => Some(*state),
        Action::FocusTextInput { .. }
        | Action::Navigate { .. }
        | Action::Back
        | Action::ReloadResource { .. }
        | Action::Controller { .. }
        | Action::Native { .. }
        | Action::Sequence(_) => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextPart {
    Literal(String),
    State(StateId),
    Resource(ResourceId, ResourceField),
    Controller(ControllerId, Vec<String>),
    ListLength(StateId),
    Item(Vec<String>),
    Value(Value),
}

impl TextPart {
    pub fn literal(value: impl Into<String>) -> Self {
        Self::Literal(value.into())
    }

    pub const fn state(state: StateId) -> Self {
        Self::State(state)
    }

    pub fn resource(resource: ResourceId, field: ResourceField) -> Self {
        Self::Resource(resource, field)
    }

    pub fn controller(controller: ControllerId, path: Vec<String>) -> Self {
        Self::Controller(controller, path)
    }

    pub const fn list_length(state: StateId) -> Self {
        Self::ListLength(state)
    }
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
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
    pub compressed_pixels: AssetBytes,
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImageSource {
    Asset(ImageAsset),
    Remote(Vec<TextPart>),
    Native(String, Vec<TextPart>),
}

impl ImageAsset {
    pub const fn new(id: u64, width: u32, height: u32, compressed_pixels: &'static [u8]) -> Self {
        Self {
            id,
            width,
            height,
            compressed_pixels: AssetBytes::Static(compressed_pixels),
        }
    }

    fn owned(id: u64, width: u32, height: u32, compressed_pixels: Vec<u8>) -> Self {
        Self {
            id,
            width,
            height,
            compressed_pixels: AssetBytes::Owned(compressed_pixels.into()),
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
struct NodeIdentity(usize);

#[derive(Clone, Debug, PartialEq)]
enum NodeKind {
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
    },
    Button {
        label: Vec<TextPart>,
        icon: Option<Mask>,
        underline: bool,
        action: Option<Action>,
    },
    Field {
        label: String,
        value: Vec<TextPart>,
        action: Option<Action>,
    },
    Icon {
        mask: Mask,
        size: f32,
        tone: Tone,
    },
    Image {
        source: ImageSource,
        fallback: Option<ImageAsset>,
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
        off: Mask,
        on: Mask,
    },
    Tabs {
        state: StateId,
        tabs: Vec<Tab>,
    },
    Navigator {
        routes: Vec<Route>,
        back: Mask,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Collection {
    State(StateId),
    Resource(ResourceId, Vec<String>),
    Controller(ControllerId, Vec<String>),
}

impl Node {
    pub fn screen(
        children: Vec<Self>,
        title: Option<String>,
        centred: bool,
        resources: Vec<ResourceId>,
        controllers: Vec<ControllerId>,
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Screen {
                children,
                title,
                centred,
                resources,
                controllers,
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
        parts: Vec<TextPart>,
        font_size: Option<f32>,
        align: TextAlign,
        max_lines: Option<u32>,
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Text {
                parts,
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
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::TextInput {
                placeholder: placeholder.into(),
                state,
                action,
            },
        }
    }

    pub fn button(
        label: Vec<TextPart>,
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

    pub fn field(label: impl Into<String>, value: Vec<TextPart>, action: Option<Action>) -> Self {
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
            kind: NodeKind::Icon { mask, size, tone },
        }
    }

    pub fn image(
        source: ImageSource,
        fallback: Option<ImageAsset>,
        bleed: bool,
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

    pub fn toggle(
        label: impl Into<String>,
        state: StateId,
        action: Action,
        off: Mask,
        on: Mask,
    ) -> Self {
        Self {
            identity: NodeIdentity(0),
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
            identity: NodeIdentity(0),
            kind: NodeKind::Tabs { state, tabs },
        }
    }

    pub fn navigator(routes: Vec<Route>, back: Mask) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Navigator { routes, back },
        }
    }

    pub fn conditional(condition: Condition, consequent: Self, alternate: Option<Self>) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::Conditional {
                condition,
                consequent: Box::new(consequent),
                alternate: alternate.map(Box::new),
            },
        }
    }

    pub fn for_each(collection: Collection, template: Self) -> Self {
        Self {
            identity: NodeIdentity(0),
            kind: NodeKind::ForEach {
                collection,
                template: Box::new(template),
            },
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
pub struct Route {
    path: String,
    screen: Node,
}

impl Route {
    pub fn new(path: impl Into<String>, screen: Node) -> Self {
        Self {
            path: path.into(),
            screen,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppDefinition {
    states: Vec<StateDefinition>,
    state_dependencies: Vec<Vec<NodeIdentity>>,
    node_count: usize,
    resources: Vec<ResourceDefinition>,
    application_resources: Vec<ResourceId>,
    controllers: Vec<ControllerDefinition>,
    application_controllers: Vec<ControllerId>,
    root: Node,
}

impl AppDefinition {
    pub(crate) fn new(
        states: Vec<StateDefinition>,
        state_dependencies: Vec<Vec<NodeIdentity>>,
        resources: Vec<ResourceDefinition>,
        application_resources: Vec<ResourceId>,
        controllers: Vec<ControllerDefinition>,
        application_controllers: Vec<ControllerId>,
        root: Node,
    ) -> Self {
        let node_count = subtree_node_count(&root);
        Self {
            states,
            state_dependencies,
            node_count,
            resources,
            application_resources,
            controllers,
            application_controllers,
            root,
        }
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, AppDefinitionError> {
        definition::decode(bytes)
    }

    pub fn uses_persistence(&self) -> bool {
        self.states.iter().any(|state| state.persisted.is_some())
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
    pub rect: Rect,
    pub clip: Rect,
    pub fit: ImageFit,
    pub scrolling: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollBar {
    pub track: Rect,
    pub track_width: f32,
    pub thumb_width: f32,
    pub content_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPortal {
    pub controller: ControllerId,
    pub kind: CameraPreviewKind,
    pub rect: Rect,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub revision: u64,
    pub width: u32,
    pub height: u32,
    pub quads: Vec<Quad>,
    pub text: Vec<TextRun>,
    pub masks: Vec<MaskRun>,
    pub images: Vec<ImageRun>,
    pub camera_portal: Option<CameraPortal>,
    pub scroll_origin: f32,
    pub scroll_offset: f32,
    pub scroll_max: f32,
    pub scroll_clip: Option<Rect>,
    pub scroll_bar: Option<ScrollBar>,
}

#[derive(Clone, Debug)]
struct HitRegion {
    rect: Rect,
    action: Action,
    scrolling: bool,
}

#[derive(Clone, Copy, Debug)]
struct Pointer {
    start_y: f32,
    start_offset: f32,
    dragging: bool,
}

#[derive(Clone, Copy)]
struct MaterialisedItem<'a> {
    value: &'a StateValue,
    index: usize,
}

#[derive(Clone, Copy)]
struct VerticalMeasure {
    size: MeasuredSize,
    entries: usize,
}

#[derive(Clone, Copy)]
struct VirtualListLayout {
    item_count: usize,
    available_width: u32,
    row_width: f32,
    row_height: f32,
}

#[derive(Clone)]
struct PendingRequest {
    request: NativeRequest,
    owner: RequestOwner,
}

#[derive(Clone)]
enum RequestOwner {
    Resource(ResourceId),
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

pub struct Engine {
    definition: AppDefinition,
    state: Vec<StateValue>,
    resources: Vec<ResourceState>,
    active_resources: BTreeSet<ResourceId>,
    active_controllers: BTreeSet<ControllerId>,
    persistence_revision: u64,
    persistence_dirty: bool,
    viewport: Viewport,
    scene: Scene,
    materialised_root: Option<Node>,
    virtual_lists: HashMap<NodeIdentity, VirtualListLayout>,
    hit_regions: Vec<HitRegion>,
    clip: Rect,
    scrolling: bool,
    scroll_origin: f32,
    scroll_offset: f32,
    scroll_max: f32,
    pointer: Option<Pointer>,
    focused_input: Option<StateId>,
    focused_input_action: TextInputAction,
    navigation: Vec<NavigationEntry>,
    queued_requests: VecDeque<QueuedRequest>,
    in_flight_requests: HashMap<u64, PendingRequest>,
    resource_requests: HashMap<ResourceId, u64>,
    remote_images: HashMap<RemoteImageKey, RemoteImageState>,
    camera_reviews: HashMap<ControllerId, String>,
    visible_images: BTreeSet<RemoteImageKey>,
    last_native_request: Option<NativeRequest>,
    next_request_id: u64,
    back_icon: Option<Mask>,
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
        Self::from_state(definition, None).0
    }

    pub fn hydrate(definition: AppDefinition, bytes: &[u8]) -> (Self, Hydration) {
        Self::from_state(definition, Some(bytes))
    }

    fn from_state(mut definition: AppDefinition, bytes: Option<&[u8]>) -> (Self, Hydration) {
        let mut next_node_identity = 1;
        assign_node_identities(&mut definition.root, &mut next_node_identity);
        let mut state = definition
            .states
            .iter()
            .map(|state| state.initial.clone())
            .collect::<Vec<_>>();
        let hydration = match bytes {
            Some(bytes) if !bytes.is_empty() => match decode_persisted_state(bytes) {
                Some(values) => {
                    for (index, definition) in definition.states.iter().enumerate() {
                        let Some(persisted) = &definition.persisted else {
                            continue;
                        };
                        if let Some((schema, value)) = values.get(&persisted.key)
                            && *schema == persisted.schema
                            && persisted.shape.accepts(value)
                        {
                            state[index] = value.clone();
                        }
                    }
                    Hydration::Restored
                }
                None => Hydration::Invalid,
            },
            _ => Hydration::Empty,
        };
        let navigation = match &definition.root.kind {
            NodeKind::Navigator { routes, .. } => {
                let root = routes
                    .iter()
                    .position(|route| route.path == "/")
                    .expect("navigator has a root route");
                vec![NavigationEntry {
                    route: root,
                    params: Vec::new(),
                }]
            }
            _ => Vec::new(),
        };
        let resources = vec![ResourceState::Inactive; definition.resources.len()];
        let mut engine = Self {
            definition,
            state,
            resources,
            active_resources: BTreeSet::new(),
            active_controllers: BTreeSet::new(),
            persistence_revision: 0,
            persistence_dirty: false,
            viewport: Viewport::default(),
            scene: Scene::default(),
            materialised_root: None,
            virtual_lists: HashMap::new(),
            hit_regions: Vec::new(),
            clip: Rect::default(),
            scrolling: false,
            scroll_origin: 0.0,
            scroll_offset: 0.0,
            scroll_max: 0.0,
            pointer: None,
            focused_input: None,
            focused_input_action: TextInputAction::default(),
            navigation,
            queued_requests: VecDeque::new(),
            in_flight_requests: HashMap::new(),
            resource_requests: HashMap::new(),
            remote_images: HashMap::new(),
            camera_reviews: HashMap::new(),
            visible_images: BTreeSet::new(),
            last_native_request: None,
            next_request_id: 1,
            back_icon: None,
            font: FontRef::try_from_slice(PUBLIC_SANS).expect("bundled Public Sans is valid"),
        };
        engine.sync_active_resources();
        (engine, hydration)
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

    pub fn native_request(&self, id: u64) -> Option<&NativeRequest> {
        self.last_native_request
            .as_ref()
            .filter(|request| request.id == id)
    }

    pub fn complete_native(
        &mut self,
        request_id: u64,
        result: Result<StateValue, ResourceError>,
    ) -> bool {
        let Some(pending) = self.in_flight_requests.remove(&request_id) else {
            return false;
        };
        let RequestOwner::Resource(resource) = pending.owner else {
            return false;
        };
        if self.resource_requests.remove(&resource) != Some(request_id) {
            return false;
        }
        let definition = &self.definition.resources[resource.0];
        let previous = match &self.resources[resource.0] {
            ResourceState::Loading { previous } => previous.clone(),
            _ => None,
        };
        self.resources[resource.0] = match result {
            Err(error) if definition.protocol == ResourceProtocol::Background => {
                ResourceState::BackgroundFailed(BackgroundError {
                    kind: "unexpected".to_owned(),
                    message: error.message,
                    retryable: error.retryable,
                    attempted_at_ms: now_ms_fallback(),
                })
            }
            Ok(_) if definition.protocol == ResourceProtocol::Background => {
                ResourceState::BackgroundFailed(BackgroundError {
                    kind: "unexpected".to_owned(),
                    message: "native background state used the wrong protocol".to_owned(),
                    retryable: false,
                    attempted_at_ms: now_ms_fallback(),
                })
            }
            Ok(value) if definition.shape.accepts(&value) => ResourceState::Ready(value),
            Ok(_) => ResourceState::Failed {
                error: ResourceError::new(
                    ResourceErrorKind::Protocol,
                    "native resource returned the wrong value type",
                    false,
                ),
                previous,
            },
            Err(error) => ResourceState::Failed { error, previous },
        };
        self.rebuild_scene();
        true
    }

    pub fn complete_native_json(&mut self, request_id: u64, bytes: &[u8]) -> bool {
        let Some(PendingRequest {
            owner: RequestOwner::Resource(resource),
            ..
        }) = self.in_flight_requests.get(&request_id)
        else {
            return false;
        };
        let shape = self.definition.resources[resource.0].shape.clone();
        if matches!(
            self.definition.resources[resource.0].protocol,
            ResourceProtocol::Background | ResourceProtocol::Cached
        ) {
            let result = parse_background_state(&shape, bytes);
            let Some(pending) = self.in_flight_requests.remove(&request_id) else {
                return false;
            };
            let RequestOwner::Resource(resource) = pending.owner else {
                return false;
            };
            if self.resource_requests.remove(&resource) != Some(request_id) {
                return false;
            }
            self.resources[resource.0] = result;
            self.rebuild_scene();
            return true;
        }
        let result = serde_json::from_slice(bytes)
            .map_err(|error| {
                ResourceError::new(
                    ResourceErrorKind::Protocol,
                    format!("response was not valid JSON: {error}"),
                    false,
                )
            })
            .and_then(|value| state_from_json(&shape, &value, "$"));
        self.complete_native(request_id, result)
    }

    pub fn update_controller_json(&mut self, controller: ControllerId, bytes: &[u8]) -> bool {
        if !self.active_controllers.contains(&controller) {
            return false;
        }
        let Some(definition) = self.definition.controllers.get(controller.0) else {
            return false;
        };
        let Some(state) = self.definition.states.get(definition.state.0) else {
            return false;
        };
        let shape = &state.shape;
        let Ok(json) = serde_json::from_slice(bytes) else {
            return false;
        };
        let Ok(value) = state_from_json(shape, &json, "$controller") else {
            return false;
        };
        self.update_controller(controller, value)
    }

    pub fn update_controller(&mut self, controller: ControllerId, value: StateValue) -> bool {
        if !self.active_controllers.contains(&controller) {
            return false;
        }
        let Some(definition) = self.definition.controllers.get(controller.0) else {
            return false;
        };
        let Some(state) = self.definition.states.get(definition.state.0) else {
            return false;
        };
        if !state.shape.accepts(&value) {
            return false;
        }
        if self.state[definition.state.0] == value {
            return false;
        }
        self.state[definition.state.0] = value;
        self.rebuild_scene();
        true
    }

    pub fn set_camera_review(&mut self, controller: ControllerId, source: Option<String>) -> bool {
        if !self.active_controllers.contains(&controller) {
            return false;
        }
        let changed = match source {
            Some(source) => {
                if self.camera_reviews.get(&controller) == Some(&source) {
                    false
                } else {
                    self.camera_reviews.insert(controller, source);
                    true
                }
            }
            None => self.camera_reviews.remove(&controller).is_some(),
        };
        if changed {
            self.rebuild_scene();
        }
        changed
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
        self.remote_images.insert(
            key,
            RemoteImageState::Ready(RemoteImage {
                id,
                width,
                height,
                pixels: pixels.into(),
            }),
        );
        self.rebuild_scene();
        true
    }

    pub fn fail_native(&mut self, request_id: u64, error: ResourceError) -> bool {
        let Some(owner) = self
            .in_flight_requests
            .get(&request_id)
            .map(|pending| pending.owner.clone())
        else {
            return false;
        };
        match owner {
            RequestOwner::Resource(_) => self.complete_native(request_id, Err(error)),
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
        let Some(payload) = operation.materialise(&self.state) else {
            return false;
        };
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

    fn queue_controller(
        &mut self,
        controller: ControllerId,
        operation: impl Into<String>,
        payload: Vec<PayloadPart>,
    ) -> bool {
        let Some(definition) = self.definition.controllers.get(controller.0) else {
            return false;
        };
        let operation =
            NativeOperation::templated(definition.module.clone(), operation, payload, 10_000);
        let Some(payload) = operation.materialise(&self.state) else {
            return false;
        };
        let request = NativeRequest {
            id: self.next_request_id(),
            kind: NativeRequestKind::Action,
            operation: Some(operation),
            payload,
            controller: Some(controller),
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

    fn cancel_resource(&mut self, resource: ResourceId) {
        if let Some(request) = self.resource_requests.remove(&resource) {
            self.cancel_request(request);
        }
        if let ResourceState::Loading { previous } = &self.resources[resource.0] {
            self.resources[resource.0] = previous
                .clone()
                .map_or(ResourceState::Inactive, ResourceState::Ready);
        }
    }

    fn queue_resource(&mut self, resource: ResourceId) -> bool {
        let Some(definition) = self.definition.resources.get(resource.0).cloned() else {
            return false;
        };
        if let Some(request) = self.resource_requests.remove(&resource) {
            self.cancel_request(request);
        }
        let previous = match &self.resources[resource.0] {
            ResourceState::Ready(value) => Some(value.clone()),
            ResourceState::Loading { previous } | ResourceState::Failed { previous, .. } => {
                previous.clone()
            }
            ResourceState::Inactive => None,
            ResourceState::BackgroundWaiting
            | ResourceState::BackgroundReady { .. }
            | ResourceState::BackgroundFailed(_) => None,
        };
        if matches!(
            definition.protocol,
            ResourceProtocol::Background | ResourceProtocol::Cached
        ) {
            if matches!(self.resources[resource.0], ResourceState::Inactive) {
                self.resources[resource.0] = ResourceState::BackgroundWaiting;
            }
        } else {
            self.resources[resource.0] = ResourceState::Loading { previous };
        }
        let request_id = self.next_request_id();
        let Some(payload) = definition.read.materialise(&self.state) else {
            return false;
        };
        self.resource_requests.insert(resource, request_id);
        self.queued_requests
            .push_back(QueuedRequest::Start(PendingRequest {
                request: NativeRequest {
                    id: request_id,
                    kind: NativeRequestKind::ResourceRead,
                    operation: Some(definition.read),
                    payload,
                    controller: None,
                },
                owner: RequestOwner::Resource(resource),
            }));
        true
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

    pub fn persisted_snapshot(&self) -> Result<Option<(u64, Vec<u8>)>, PersistenceTooLarge> {
        if !self.persistence_dirty {
            return Ok(None);
        }
        let revision = self.persistence_revision;
        let values =
            self.definition
                .states
                .iter()
                .zip(&self.state)
                .filter_map(|(definition, value)| {
                    definition
                        .persisted
                        .as_ref()
                        .map(|persisted| (persisted, value))
                });
        Ok(Some((revision, encode_persisted_state(values)?)))
    }

    pub fn persistence_saved(&mut self, revision: u64) {
        if self.persistence_revision == revision {
            self.persistence_dirty = false;
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
            .map(|region| region.action.clone());
        let blurred = self.focused_input.is_some()
            && !matches!(action.as_ref(), Some(Action::FocusTextInput { .. }));
        if blurred {
            self.focused_input = None;
        }

        let (changed, states) =
            action.map_or_else(|| (false, Vec::new()), |action| self.apply(action));
        if !changed && !blurred {
            return false;
        }

        if blurred || states.is_empty() {
            self.rebuild_scene();
        } else {
            self.rebuild_scene_for_states(states);
        }
        true
    }

    pub fn pointer_down(&mut self, y: f32) {
        self.pointer = Some(Pointer {
            start_y: y,
            start_offset: self.scroll_offset,
            dragging: false,
        });
    }

    pub fn pointer_move(&mut self, y: f32) -> bool {
        let tap_slop = self.scaled(TAP_SLOP);
        let Some(pointer) = &mut self.pointer else {
            return false;
        };
        let delta = pointer.start_y - y;
        if !pointer.dragging {
            if delta.abs() <= tap_slop {
                return false;
            }
            pointer.start_y -= delta.signum() * tap_slop;
            pointer.dragging = true;
        }
        let next = (pointer.start_offset + pointer.start_y - y).clamp(0.0, self.scroll_max);
        self.set_scroll_offset(next)
    }

    pub fn pointer_up(&mut self, x: f32, y: f32) -> bool {
        let Some(pointer) = self.pointer.take() else {
            return false;
        };
        if !pointer.dragging {
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

    pub fn back(&mut self) -> bool {
        if !self.pop_route() {
            return false;
        }
        self.rebuild_scene();
        true
    }

    pub fn navigate(&mut self, path: &str) -> bool {
        if !self
            .apply(Action::Navigate {
                path: path.to_owned(),
                params: Vec::new(),
            })
            .0
        {
            return false;
        }
        self.rebuild_scene();
        true
    }

    pub fn resume(&mut self) -> bool {
        let resources = self
            .active_resources
            .iter()
            .copied()
            .filter(|resource| {
                self.definition.resources[resource.0].reload_on_resume
                    || matches!(self.resources[resource.0], ResourceState::Loading { .. })
            })
            .collect::<Vec<_>>();
        if resources.is_empty() {
            return false;
        }
        for resource in resources {
            self.queue_resource(resource);
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
            TextEdit::Insert(text) if !text.chars().any(char::is_control) => {
                let Some(StateValue::String(value)) = self.state.get_mut(state.0) else {
                    return false;
                };
                value.push_str(&text);
                mutated = true;
                true
            }
            TextEdit::Backspace => {
                let Some(StateValue::String(value)) = self.state.get_mut(state.0) else {
                    return false;
                };
                let Some(last) = value.grapheme_indices(true).next_back() else {
                    return false;
                };
                value.truncate(last.0);
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
                self.mark_persisted(state);
                self.refresh_dependent_resources(state);
            }
            if mutated {
                self.rebuild_scene_for_states([state]);
            } else {
                self.rebuild_scene();
            }
        }
        changed
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub const fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub const fn scroll_max(&self) -> f32 {
        self.scroll_max
    }

    fn apply(&mut self, action: Action) -> (bool, Vec<StateId>) {
        let mut states = Vec::new();
        let changed = self.apply_inner(action, &mut states);
        (changed, states)
    }

    fn apply_inner(&mut self, action: Action, states: &mut Vec<StateId>) -> bool {
        let mutated_state = action_state(&action);
        match action {
            Action::Increment { state, by } => {
                let Some(StateValue::Number(value)) = self.state.get_mut(state.0) else {
                    return false;
                };
                let next = *value + by;
                if !next.is_finite() {
                    return false;
                }
                *value = next;
            }
            Action::SetValue { state, value } => {
                let Some(value) = self.evaluate_value(&value) else {
                    return false;
                };
                if !self
                    .definition
                    .states
                    .get(state.0)
                    .is_some_and(|definition| definition.shape.accepts(&value))
                {
                    return false;
                }
                let resets_scroll = matches!(value, StateValue::Number(_));
                let Some(current) = self.state.get_mut(state.0) else {
                    return false;
                };
                if *current == value {
                    return false;
                }
                *current = value;
                if resets_scroll {
                    self.scroll_offset = 0.0;
                }
            }
            Action::Toggle { state } => {
                let Some(StateValue::Bool(value)) = self.state.get_mut(state.0) else {
                    return false;
                };
                *value = !*value;
            }
            Action::SetList { state, value } => {
                let Some(StateValue::List(value)) = self.evaluate_value(&value) else {
                    return false;
                };
                let Some(StateValue::List(current)) = self.state.get_mut(state.0) else {
                    return false;
                };
                if *current == value {
                    return false;
                }
                *current = value;
                self.scroll_offset = 0.0;
            }
            Action::AppendList { state, value } => {
                let Some(value) = self.evaluate_value(&value) else {
                    return false;
                };
                let Some(StateValue::List(current)) = self.state.get_mut(state.0) else {
                    return false;
                };
                current.push(value);
            }
            Action::RemoveListItem { state, index } => {
                let Some(StateValue::List(current)) = self.state.get_mut(state.0) else {
                    return false;
                };
                if index >= current.len() {
                    return false;
                }
                current.remove(index);
            }
            Action::ReplaceListItem {
                state,
                index,
                value,
            } => {
                let Some(value) = self.evaluate_value(&value) else {
                    return false;
                };
                let Some(StateValue::List(current)) = self.state.get_mut(state.0) else {
                    return false;
                };
                let Some(item) = current.get_mut(index) else {
                    return false;
                };
                if *item == value {
                    return false;
                }
                *item = value;
            }
            Action::ClearList { state } => {
                let Some(StateValue::List(current)) = self.state.get_mut(state.0) else {
                    return false;
                };
                if current.is_empty() {
                    return false;
                }
                current.clear();
                self.scroll_offset = 0.0;
            }
            Action::RemoveCurrentListItem { .. } | Action::ReplaceCurrentListItem { .. } => {
                unreachable!("current-list actions are materialised before interaction")
            }
            Action::FocusTextInput { state, action } => {
                if self.focused_input == Some(state) && self.focused_input_action == action {
                    return false;
                }
                self.focused_input = Some(state);
                self.focused_input_action = action;
            }
            Action::ReloadResource { resource } => {
                return self.queue_resource(resource);
            }
            Action::Controller {
                controller,
                operation,
                payload,
            } => {
                return self.queue_controller(controller, operation, payload);
            }
            Action::Native { operation } => {
                return self.queue_native_action(operation);
            }
            Action::Navigate { path, params } => {
                let NodeKind::Navigator { routes, .. } = &self.definition.root.kind else {
                    return false;
                };
                let Some(route) = routes.iter().position(|route| route.path == path) else {
                    return false;
                };
                let Some(params) = params
                    .iter()
                    .map(|(name, value)| Some((name.clone(), self.evaluate_value(value)?)))
                    .collect::<Option<Vec<_>>>()
                else {
                    return false;
                };
                if self
                    .navigation
                    .last()
                    .is_some_and(|entry| entry.route == route && entry.params == params)
                {
                    return false;
                }
                self.navigation.push(NavigationEntry { route, params });
                self.scroll_offset = 0.0;
                self.pointer = None;
                self.focused_input = None;
            }
            Action::Back => return self.pop_route(),
            Action::Sequence(actions) => {
                let mut changed = false;
                for action in actions {
                    changed |= self.apply_inner(action, states);
                }
                return changed;
            }
        }
        if let Some(state) = mutated_state {
            if !states.contains(&state) {
                states.push(state);
            }
            self.mark_persisted(state);
            self.refresh_dependent_resources(state);
        }
        true
    }

    fn mark_persisted(&mut self, state: StateId) {
        if self
            .definition
            .states
            .get(state.0)
            .is_some_and(|state| state.persisted.is_some())
        {
            self.persistence_revision = self.persistence_revision.wrapping_add(1);
            self.persistence_dirty = true;
        }
    }

    fn refresh_dependent_resources(&mut self, state: StateId) {
        let resources = self
            .definition
            .resources
            .iter()
            .enumerate()
            .filter_map(|(index, resource)| {
                (self.active_resources.contains(&ResourceId(index))
                    && resource
                        .read
                        .dependencies()
                        .any(|dependency| dependency == state))
                .then_some(ResourceId(index))
            })
            .collect::<Vec<_>>();
        for resource in resources {
            self.queue_resource(resource);
        }
    }

    fn evaluate_value(&self, value: &Value) -> Option<StateValue> {
        match value {
            Value::Null => Some(StateValue::Null),
            Value::Number(value) => Some(StateValue::Number(*value)),
            Value::Bool(value) => Some(StateValue::Bool(*value)),
            Value::String(value) => Some(StateValue::String(value.clone())),
            Value::State(state) => self.state.get(state.0).cloned(),
            Value::Item(_) => None,
            Value::Resource(resource, field) => self.resource_field_value(*resource, field),
            Value::Controller(controller, path) => self.controller_field_value(*controller, path),
            Value::CombinedStatus(resources) => {
                let statuses = resources
                    .iter()
                    .map(|resource| self.resource_field_value(*resource, &ResourceField::Status))
                    .collect::<Option<Vec<_>>>()?;
                let status =
                    if statuses.iter().any(
                        |status| matches!(status, StateValue::String(value) if value == "error"),
                    ) {
                        "error"
                    } else if statuses.iter().all(
                        |status| matches!(status, StateValue::String(value) if value == "ready"),
                    ) {
                        "ready"
                    } else {
                        "loading"
                    };
                Some(StateValue::String(status.to_owned()))
            }
            Value::CombinedErrorResource(resources) => resources
                .iter()
                .find(|(_, resource)| self.resource_has_failed(*resource))
                .map(|(name, _)| StateValue::String(name.clone())),
            Value::CombinedErrorField(resources, field) => resources
                .iter()
                .copied()
                .find(|resource| self.resource_has_failed(*resource))
                .and_then(|resource| self.resource_field_value(resource, field)),
            Value::ListLength(state) => match self.state.get(state.0)? {
                StateValue::List(values) => Some(StateValue::Number(values.len() as f64)),
                _ => None,
            },
            Value::Binary {
                left,
                operator,
                right,
            } => evaluate_binary(
                self.evaluate_value(left)?,
                *operator,
                self.evaluate_value(right)?,
            ),
            Value::RouteParam(name) => self
                .navigation
                .last()?
                .params
                .iter()
                .find(|(param, _)| param == name)
                .map(|(_, value)| value.clone()),
            Value::List(values) => values
                .iter()
                .map(|value| self.evaluate_value(value))
                .collect::<Option<_>>()
                .map(StateValue::List),
            Value::Object(fields) => fields
                .iter()
                .map(|(name, value)| Some((name.clone(), self.evaluate_value(value)?)))
                .collect::<Option<_>>()
                .map(StateValue::Object),
        }
    }

    fn pop_route(&mut self) -> bool {
        if self.navigation.len() <= 1 {
            return false;
        }
        self.navigation.pop();
        self.scroll_offset = 0.0;
        self.pointer = None;
        self.focused_input = None;
        true
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

    fn sync_active_resources(&mut self) {
        let mut active = self
            .definition
            .application_resources
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let root = match &self.definition.root.kind {
            NodeKind::Navigator { routes, .. } => {
                let route = self
                    .navigation
                    .last()
                    .expect("navigator history is never empty")
                    .route;
                routes[route].screen.clone()
            }
            _ => self.definition.root.clone(),
        };
        self.collect_active_resources(&root, &mut active);

        let inactive = self
            .active_resources
            .difference(&active)
            .copied()
            .collect::<Vec<_>>();
        for resource in inactive {
            self.cancel_resource(resource);
        }
        let newly_active = active
            .difference(&self.active_resources)
            .copied()
            .collect::<Vec<_>>();
        self.active_resources = active;
        for resource in newly_active {
            if self.definition.resources[resource.0].protocol != ResourceProtocol::Mutation
                && matches!(self.resources[resource.0], ResourceState::Inactive)
            {
                self.queue_resource(resource);
            }
        }
        self.sync_active_controllers(&root);
    }

    fn sync_active_controllers(&mut self, root: &Node) {
        let mut active = self
            .definition
            .application_controllers
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        self.collect_active_controllers(root, &mut active);

        let inactive = self
            .active_controllers
            .difference(&active)
            .copied()
            .collect::<Vec<_>>();
        for controller in inactive {
            self.camera_reviews.remove(&controller);
            self.queue_controller(
                controller,
                "deactivate",
                vec![PayloadPart::Literal(String::new())],
            );
        }
        let newly_active = active
            .difference(&self.active_controllers)
            .copied()
            .collect::<Vec<_>>();
        self.active_controllers = active;
        for controller in newly_active {
            let Some(definition) = self.definition.controllers.get(controller.0) else {
                continue;
            };
            let payload = format!(
                "{{\"kind\":{},\"config\":{}}}",
                serde_json::to_string(&definition.kind).expect("a Rust string is valid JSON"),
                definition.config,
            );
            self.queue_controller(controller, "activate", vec![PayloadPart::Literal(payload)]);
        }
    }

    fn collect_active_resources(&self, node: &Node, active: &mut BTreeSet<ResourceId>) {
        match &node.kind {
            NodeKind::Screen {
                children,
                resources,
                ..
            } => {
                active.extend(resources.iter().copied());
                for child in children {
                    self.collect_active_resources(child, active);
                }
            }
            NodeKind::Stack { children, .. } => {
                for child in children {
                    self.collect_active_resources(child, active);
                }
            }
            NodeKind::Tabs { state, tabs } => {
                let active_tab = match self.state.get(state.0) {
                    Some(StateValue::Number(value)) if value.fract() == 0.0 && *value >= 0.0 => {
                        *value as usize
                    }
                    _ => 0,
                };
                if let Some(tab) = tabs.get(active_tab) {
                    self.collect_active_resources(&tab.screen, active);
                }
            }
            NodeKind::Navigator { routes, .. } => {
                let route = self.navigation.last().map_or(0, |entry| entry.route);
                if let Some(route) = routes.get(route) {
                    self.collect_active_resources(&route.screen, active);
                }
            }
            NodeKind::Conditional {
                condition,
                consequent,
                alternate,
            } => {
                let branch = if self.condition_enabled(condition) {
                    Some(consequent.as_ref())
                } else {
                    alternate.as_deref()
                };
                if let Some(branch) = branch {
                    self.collect_active_resources(branch, active);
                }
            }
            NodeKind::ForEach { template, .. } => {
                self.collect_active_resources(template, active);
            }
            NodeKind::Text { .. }
            | NodeKind::TextInput { .. }
            | NodeKind::Button { .. }
            | NodeKind::Field { .. }
            | NodeKind::Icon { .. }
            | NodeKind::Image { .. }
            | NodeKind::CameraPreview { .. }
            | NodeKind::Toggle { .. } => {}
        }
    }

    fn collect_active_controllers(&self, node: &Node, active: &mut BTreeSet<ControllerId>) {
        match &node.kind {
            NodeKind::Screen {
                children,
                controllers,
                ..
            } => {
                active.extend(controllers.iter().copied());
                for child in children {
                    self.collect_active_controllers(child, active);
                }
            }
            NodeKind::Stack { children, .. } => {
                for child in children {
                    self.collect_active_controllers(child, active);
                }
            }
            NodeKind::Tabs { state, tabs } => {
                let active_tab = match self.state.get(state.0) {
                    Some(StateValue::Number(value)) if value.fract() == 0.0 && *value >= 0.0 => {
                        *value as usize
                    }
                    _ => 0,
                };
                if let Some(tab) = tabs.get(active_tab) {
                    self.collect_active_controllers(&tab.screen, active);
                }
            }
            NodeKind::Navigator { routes, .. } => {
                let route = self.navigation.last().map_or(0, |entry| entry.route);
                if let Some(route) = routes.get(route) {
                    self.collect_active_controllers(&route.screen, active);
                }
            }
            NodeKind::Conditional {
                condition,
                consequent,
                alternate,
            } => {
                let branch = if self.condition_enabled(condition) {
                    Some(consequent.as_ref())
                } else {
                    alternate.as_deref()
                };
                if let Some(branch) = branch {
                    self.collect_active_controllers(branch, active);
                }
            }
            NodeKind::ForEach { template, .. } => {
                self.collect_active_controllers(template, active);
            }
            NodeKind::Text { .. }
            | NodeKind::TextInput { .. }
            | NodeKind::Button { .. }
            | NodeKind::Field { .. }
            | NodeKind::Icon { .. }
            | NodeKind::Image { .. }
            | NodeKind::CameraPreview { .. }
            | NodeKind::Toggle { .. } => {}
        }
    }

    fn condition_enabled(&self, condition: &Condition) -> bool {
        match condition {
            Condition::ValueEquals {
                value,
                expected,
                equals,
            } => (self.evaluate_value(value).as_ref() == Some(expected)) == *equals,
            Condition::Bool { state, expected } => {
                matches!(self.state.get(state.0), Some(StateValue::Bool(value)) if value == expected)
            }
            Condition::ListEmpty { state, expected } => {
                matches!(self.state.get(state.0), Some(StateValue::List(value)) if value.is_empty() == *expected)
            }
            Condition::Equals {
                state,
                value,
                expected,
            } => {
                self.state
                    .get(state.0)
                    .is_some_and(|current| current == value)
                    == *expected
            }
            Condition::ResourceEquals {
                resource,
                field,
                value,
                expected,
            } => (self.resource_field_value(*resource, field).as_ref() == Some(value)) == *expected,
            Condition::ControllerEquals {
                controller,
                path,
                value,
                expected,
            } => {
                (self.controller_field_value(*controller, path).as_ref() == Some(value))
                    == *expected
            }
        }
    }

    fn controller_field_value(
        &self,
        controller: ControllerId,
        path: &[String],
    ) -> Option<StateValue> {
        let definition = self.definition.controllers.get(controller.0)?;
        let value = self.state.get(definition.state.0)?;
        item_at_path(value, path).cloned()
    }

    fn resource_field_value(
        &self,
        resource: ResourceId,
        field: &ResourceField,
    ) -> Option<StateValue> {
        let state = self.resources.get(resource.0)?;
        match field {
            ResourceField::Status => Some(StateValue::String(
                match state {
                    ResourceState::Inactive
                        if self.definition.resources[resource.0].protocol
                            == ResourceProtocol::Mutation =>
                    {
                        "idle"
                    }
                    ResourceState::Loading { .. }
                        if self.definition.resources[resource.0].protocol
                            == ResourceProtocol::Mutation =>
                    {
                        "running"
                    }
                    ResourceState::Inactive | ResourceState::Loading { .. } => "loading",
                    ResourceState::Ready(_) => "ready",
                    ResourceState::Failed { .. } => "error",
                    ResourceState::BackgroundWaiting
                        if self.definition.resources[resource.0].protocol
                            == ResourceProtocol::Cached =>
                    {
                        "loading"
                    }
                    ResourceState::BackgroundWaiting => "waiting",
                    ResourceState::BackgroundReady { error: None, .. } => "ready",
                    ResourceState::BackgroundReady { error: Some(_), .. } => "stale",
                    ResourceState::BackgroundFailed(_) => "error",
                }
                .to_owned(),
            )),
            ResourceField::Value(path) => {
                let value = match state {
                    ResourceState::Ready(value) | ResourceState::BackgroundReady { value, .. } => {
                        value
                    }
                    _ => return None,
                };
                item_at_path(value, path).cloned()
            }
            ResourceField::ErrorKind => {
                let value = match state {
                    ResourceState::Failed { error, .. } => error.kind.as_str(),
                    ResourceState::BackgroundReady {
                        error: Some(error), ..
                    }
                    | ResourceState::BackgroundFailed(error) => &error.kind,
                    _ => return None,
                };
                Some(StateValue::String(value.to_owned()))
            }
            ResourceField::ErrorMessage => {
                let value = match state {
                    ResourceState::Failed { error, .. } => &error.message,
                    ResourceState::BackgroundReady {
                        error: Some(error), ..
                    }
                    | ResourceState::BackgroundFailed(error) => &error.message,
                    _ => return None,
                };
                Some(StateValue::String(value.clone()))
            }
            ResourceField::ErrorRetryable => {
                let value = match state {
                    ResourceState::Failed { error, .. } => error.retryable,
                    ResourceState::BackgroundReady {
                        error: Some(error), ..
                    }
                    | ResourceState::BackgroundFailed(error) => error.retryable,
                    _ => return None,
                };
                Some(StateValue::Bool(value))
            }
            ResourceField::UpdatedAtMs => match state {
                ResourceState::BackgroundReady { updated_at_ms, .. } => {
                    Some(StateValue::Number(*updated_at_ms))
                }
                _ => None,
            },
            ResourceField::ErrorAttemptedAtMs => match state {
                ResourceState::BackgroundReady {
                    error: Some(error), ..
                }
                | ResourceState::BackgroundFailed(error) => {
                    Some(StateValue::Number(error.attempted_at_ms))
                }
                _ => None,
            },
        }
    }

    fn resource_has_failed(&self, resource: ResourceId) -> bool {
        matches!(
            self.resources.get(resource.0),
            Some(ResourceState::Failed { .. } | ResourceState::BackgroundFailed(_))
        )
    }

    fn rebuild_scene(&mut self) {
        self.sync_active_resources();
        self.virtual_lists.clear();
        if self.viewport.width == 0 || self.viewport.height == 0 {
            self.materialised_root = None;
            self.relayout_scene();
            return;
        }

        let (root, back_icon) = match &self.definition.root.kind {
            NodeKind::Navigator { routes, back } => {
                let route = self
                    .navigation
                    .last()
                    .expect("navigator history is never empty")
                    .route;
                (&routes[route].screen, Some(back.clone()))
            }
            _ => (&self.definition.root, None),
        };
        let mut roots = self.materialise(root, None);
        assert_eq!(roots.len(), 1, "an app route has exactly one root");
        self.materialised_root = Some(roots.remove(0));
        self.back_icon = if self.navigation.len() > 1 {
            back_icon
        } else {
            None
        };
        self.relayout_scene();
    }

    fn rebuild_scene_for_states(&mut self, states: impl IntoIterator<Item = StateId>) {
        if self.definition.node_count < INCREMENTAL_TREE_THRESHOLD {
            self.rebuild_scene();
            return;
        }
        self.sync_active_resources();
        let states = states.into_iter().collect::<Vec<_>>();
        if self
            .definition
            .resources
            .iter()
            .enumerate()
            .any(|(index, resource)| {
                self.active_resources.contains(&ResourceId(index))
                    && resource
                        .read
                        .dependencies()
                        .any(|state| states.contains(&state))
            })
        {
            self.rebuild_scene();
            return;
        }
        let targets = states
            .iter()
            .filter_map(|state| self.definition.state_dependencies.get(state.0))
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        if targets.is_empty() {
            return;
        }
        if self.viewport.width == 0 || self.viewport.height == 0 {
            self.virtual_lists.clear();
            self.materialised_root = None;
            self.relayout_scene();
            return;
        }

        let root = match &self.definition.root.kind {
            NodeKind::Navigator { routes, .. } => {
                let route = self
                    .navigation
                    .last()
                    .expect("navigator history is never empty")
                    .route;
                &routes[route].screen
            }
            _ => &self.definition.root,
        };
        if !subtree_contains_target(root, &targets) {
            return;
        }
        self.virtual_lists.clear();
        let previous = self.materialised_root.take();
        let mut roots = previous.as_ref().map_or_else(
            || self.materialise(root, None),
            |previous| self.materialise_incremental(root, previous, &targets),
        );
        assert_eq!(roots.len(), 1, "an app route has exactly one root");
        self.materialised_root = Some(roots.remove(0));
        self.relayout_scene();
    }

    fn materialise_incremental(
        &self,
        source: &Node,
        previous: &Node,
        targets: &[NodeIdentity],
    ) -> Vec<Node> {
        if !subtree_contains_target(source, targets) {
            return vec![previous.clone()];
        }
        if source.identity != previous.identity || targets.contains(&source.identity) {
            return self.materialise(source, None);
        }

        match (&source.kind, &previous.kind) {
            (
                NodeKind::Screen {
                    children,
                    title,
                    centred,
                    resources,
                    controllers,
                },
                NodeKind::Screen {
                    children: previous_children,
                    ..
                },
            ) => vec![Node {
                identity: source.identity,
                kind: NodeKind::Screen {
                    children: self.materialise_incremental_children(
                        children,
                        previous_children,
                        targets,
                        true,
                    ),
                    title: title.clone(),
                    centred: *centred,
                    resources: resources.clone(),
                    controllers: controllers.clone(),
                },
            }],
            (
                NodeKind::Stack {
                    children,
                    axis,
                    gap,
                    align,
                    justify,
                },
                NodeKind::Stack {
                    children: previous_children,
                    ..
                },
            ) => vec![Node {
                identity: source.identity,
                kind: NodeKind::Stack {
                    children: self.materialise_incremental_children(
                        children,
                        previous_children,
                        targets,
                        *axis == Axis::Vertical,
                    ),
                    axis: *axis,
                    gap: *gap,
                    align: *align,
                    justify: *justify,
                },
            }],
            _ => self.materialise(source, None),
        }
    }

    fn materialise_incremental_children(
        &self,
        source: &[Node],
        previous: &[Node],
        targets: &[NodeIdentity],
        preserve_virtual_lists: bool,
    ) -> Vec<Node> {
        let mut output = Vec::new();
        for child in source {
            if preserve_virtual_lists
                && matches!(&child.kind, NodeKind::ForEach { template, .. } if virtualisable_template(template))
            {
                output.push(child.clone());
            } else if !subtree_contains_target(child, targets) {
                output.extend(
                    previous
                        .iter()
                        .filter(|node| subtree_contains_identity(child, node.identity))
                        .cloned(),
                );
            } else if let Some(previous) =
                previous.iter().find(|node| node.identity == child.identity)
            {
                output.extend(self.materialise_incremental(child, previous, targets));
            } else {
                output.extend(self.materialise(child, None));
            }
        }
        output
    }

    fn relayout_scene(&mut self) {
        self.scene.revision = self.scene.revision.wrapping_add(1);
        self.scene.width = self.viewport.width;
        self.scene.height = self.viewport.height;
        self.scene.quads.clear();
        self.scene.text.clear();
        self.scene.masks.clear();
        self.scene.images.clear();
        self.scene.camera_portal = None;
        self.scroll_origin = self.scroll_offset;
        self.scene.scroll_origin = self.scroll_origin;
        self.scene.scroll_offset = self.scroll_offset;
        self.scene.scroll_clip = None;
        self.scene.scroll_bar = None;
        self.hit_regions.clear();
        self.visible_images.clear();
        self.scroll_max = 0.0;
        self.scene.scroll_max = 0.0;
        self.scrolling = false;

        if self.viewport.width == 0 || self.viewport.height == 0 {
            return;
        }

        let Some(root) = self.materialised_root.take() else {
            return;
        };
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
        self.materialised_root = Some(root);
        self.sync_visible_images();
    }

    fn measure(&mut self, node: &Node, available: Rect) -> MeasuredSize {
        match &node.kind {
            NodeKind::Screen { .. } | NodeKind::Tabs { .. } | NodeKind::Navigator { .. } => {
                MeasuredSize {
                    width: available.width,
                    height: available.height,
                }
            }
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
                        let entries = measured
                            .iter()
                            .map(|measure| measure.entries)
                            .sum::<usize>();
                        MeasuredSize {
                            width: measured
                                .iter()
                                .map(|measure| measure.size.width)
                                .fold(0.0, f32::max)
                                .min(available.width),
                            height: (measured
                                .iter()
                                .map(|measure| measure.size.height)
                                .sum::<f32>()
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
                parts,
                font_size,
                align,
                max_lines,
            } => {
                let size = font_size.unwrap_or(DEFAULT_TEXT_SIZE);
                let font_size = self.scaled_font(size);
                let lines = self.wrap_text(
                    &self.resolve_text(parts),
                    font_size,
                    available.width,
                    *max_lines,
                );
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
            NodeKind::TextInput { .. } => MeasuredSize {
                width: available.width,
                height: self.scaled(TEXT_INPUT_HEIGHT).min(available.height),
            },
            NodeKind::Button { label, icon, .. } => {
                let font_size = self.scaled_font(DEFAULT_TEXT_SIZE);
                let label = self.resolve_text(label);
                let icon_width = icon
                    .as_ref()
                    .map(|_| self.scaled(BUTTON_ICON_SIZE + BUTTON_ICON_GAP))
                    .unwrap_or_default();
                MeasuredSize {
                    width: (self.text_width(&label, font_size).ceil() + 1.0 + icon_width)
                        .min(available.width),
                    height: self.scaled(BUTTON_HEIGHT).min(available.height),
                }
            }
            NodeKind::Field { label, value, .. } => {
                let label_width = self.text_width(label, self.scaled_font(FIELD_LABEL_SIZE));
                let value_width = self.text_width(
                    &self.resolve_text(value),
                    self.scaled_font(DEFAULT_TEXT_SIZE),
                );
                MeasuredSize {
                    width: label_width.max(value_width).ceil().min(available.width),
                    height: self.scaled(FIELD_HEIGHT).min(available.height),
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
            NodeKind::Image {
                bleed,
                width,
                height,
                ..
            } => {
                let measured_width = if *bleed {
                    self.viewport.width as f32
                } else {
                    self.scaled(*width).min(available.width)
                };
                MeasuredSize {
                    width: measured_width,
                    height: if *bleed {
                        (measured_width * height / width).min(available.height)
                    } else {
                        self.scaled(*height).min(available.height)
                    },
                }
            }
            NodeKind::CameraPreview { .. } => MeasuredSize {
                width: available.width,
                height: if available.height.is_finite() {
                    available.height
                } else {
                    0.0
                },
            },
            NodeKind::Toggle { .. } => MeasuredSize {
                width: available.width,
                height: self.scaled(TOGGLE_HEIGHT).min(available.height),
            },
            NodeKind::ForEach { .. } => self.measure_virtual_list(node, available).size,
            NodeKind::Conditional { .. } => {
                unreachable!("dynamic nodes are materialised before layout")
            }
        }
    }

    fn measure_vertical_children(
        &mut self,
        children: &[Node],
        available: Rect,
    ) -> Vec<VerticalMeasure> {
        children
            .iter()
            .map(|child| match &child.kind {
                NodeKind::ForEach { .. } => self.measure_virtual_list(child, available),
                _ => VerticalMeasure {
                    size: self.measure(child, available),
                    entries: 1,
                },
            })
            .collect()
    }

    fn measure_virtual_list(&mut self, node: &Node, available: Rect) -> VerticalMeasure {
        let NodeKind::ForEach {
            collection,
            template,
        } = &node.kind
        else {
            unreachable!("only list nodes have virtual list geometry")
        };
        let item_count = self
            .collection_items(collection)
            .map_or(0, <[StateValue]>::len);
        if item_count == 0 {
            return VerticalMeasure {
                size: MeasuredSize::default(),
                entries: 0,
            };
        }
        let available_width = available.width.to_bits();
        if let Some(layout) = self.virtual_lists.get(&node.identity).copied()
            && layout.item_count == item_count
            && layout.available_width == available_width
        {
            return VerticalMeasure {
                size: MeasuredSize {
                    width: layout.row_width,
                    height: layout.row_height * item_count as f32,
                },
                entries: item_count,
            };
        }

        let item = self
            .collection_items(collection)
            .and_then(|items| items.first())
            .cloned()
            .expect("a non-empty virtual list has a first item");
        let mut row = self.materialise(
            template,
            Some(MaterialisedItem {
                value: &item,
                index: 0,
            }),
        );
        assert_eq!(row.len(), 1, "a virtual list template has one root");
        let size = self.measure(&row.remove(0), available);
        self.virtual_lists.insert(
            node.identity,
            VirtualListLayout {
                item_count,
                available_width,
                row_width: size.width,
                row_height: size.height,
            },
        );
        VerticalMeasure {
            size: MeasuredSize {
                width: size.width,
                height: size.height * item_count as f32,
            },
            entries: item_count,
        }
    }

    fn collection_items(&self, collection: &Collection) -> Option<&[StateValue]> {
        let value = match collection {
            Collection::State(state) => self.state.get(state.0)?,
            Collection::Resource(resource, path) => {
                let value = match self.resources.get(resource.0)? {
                    ResourceState::Ready(value) | ResourceState::BackgroundReady { value, .. } => {
                        value
                    }
                    _ => return None,
                };
                item_at_path(value, path)?
            }
            Collection::Controller(controller, path) => {
                let definition = self.definition.controllers.get(controller.0)?;
                item_at_path(self.state.get(definition.state.0)?, path)?
            }
        };
        match value {
            StateValue::List(items) => Some(items),
            _ => None,
        }
    }

    fn layout(&mut self, node: &Node, rect: Rect) {
        self.layout_node(node, rect, true);
    }

    fn layout_node(&mut self, node: &Node, rect: Rect, screen_bottom_inset: bool) {
        let visible = rect.intersection(self.clip);
        if self.scrolling
            && !matches!(
                &node.kind,
                NodeKind::Screen { .. } | NodeKind::Stack { .. } | NodeKind::Tabs { .. }
            )
            && (visible.width == 0.0 || visible.height == 0.0)
        {
            return;
        }
        match &node.kind {
            NodeKind::Screen {
                children,
                title,
                centred,
                ..
            } => self.layout_screen(
                children,
                title.as_deref(),
                *centred,
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
                parts,
                font_size,
                align,
                max_lines,
            } => {
                let size = font_size.unwrap_or(DEFAULT_TEXT_SIZE);
                let font_size = self.scaled_font(size);
                let text = self.resolve_text(parts);
                let lines = self.wrap_text(&text, font_size, rect.width, *max_lines);
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
                        colour: Colour::WHITE,
                        align: if *align == TextAlign::Justify && !line.wrapped {
                            TextAlign::Start
                        } else {
                            *align
                        },
                        scrolling: self.scrolling,
                    });
                }
            }
            NodeKind::TextInput {
                placeholder,
                state,
                action,
            } => self.layout_text_input(placeholder, *state, *action, rect),
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
            NodeKind::Icon { mask, tone, .. } => self.scene.masks.push(MaskRun {
                mask: mask.clone(),
                rect,
                clip: self.clip,
                colour: tone_colour(*tone),
                scrolling: self.scrolling,
            }),
            NodeKind::Image {
                source,
                fallback,
                fit,
                ..
            } => self.layout_image(source, fallback.as_ref(), *fit, rect),
            NodeKind::CameraPreview { controller, kind } => {
                self.layout_camera_preview(*controller, *kind, rect)
            }
            NodeKind::Toggle {
                label,
                state,
                action,
                off,
                on,
            } => self.layout_toggle(label, *state, action, off.clone(), on.clone(), rect),
            NodeKind::Tabs { state, tabs } => self.layout_tabs(*state, tabs, rect),
            NodeKind::Navigator { .. } => unreachable!("navigator is resolved before layout"),
            NodeKind::Conditional { .. } | NodeKind::ForEach { .. } => {
                unreachable!("dynamic nodes are materialised before layout")
            }
        }
    }

    fn materialise(&self, node: &Node, item: Option<MaterialisedItem<'_>>) -> Vec<Node> {
        let identity = node.identity;
        let node = match &node.kind {
            NodeKind::Screen {
                children,
                title,
                centred,
                resources,
                controllers,
            } => Node::screen(
                self.materialise_vertical_children(children, item),
                title.clone(),
                *centred,
                resources.clone(),
                controllers.clone(),
            ),
            NodeKind::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            } => Node::stack(
                if *axis == Axis::Vertical {
                    self.materialise_vertical_children(children, item)
                } else {
                    self.materialise_children(children, item)
                },
                *axis,
                *gap,
                *align,
                *justify,
            ),
            NodeKind::Text {
                parts,
                font_size,
                align,
                max_lines,
            } => Node::text(
                self.materialise_text(parts, item),
                *font_size,
                *align,
                *max_lines,
            ),
            NodeKind::Button {
                label,
                icon,
                underline,
                action,
            } => Node::button(
                self.materialise_text(label, item),
                icon.clone(),
                *underline,
                action
                    .as_ref()
                    .map(|action| self.materialise_action(action, item)),
            ),
            NodeKind::Image {
                source,
                fallback,
                bleed,
                width,
                height,
                fit,
            } => Node::image(
                match source {
                    ImageSource::Asset(asset) => ImageSource::Asset(asset.clone()),
                    ImageSource::Remote(parts) => {
                        ImageSource::Remote(self.materialise_text(parts, item))
                    }
                    ImageSource::Native(module, parts) => {
                        ImageSource::Native(module.clone(), self.materialise_text(parts, item))
                    }
                },
                fallback.clone(),
                *bleed,
                *width,
                *height,
                *fit,
            ),
            NodeKind::CameraPreview { controller, kind } => {
                Node::camera_preview(*controller, *kind)
            }
            NodeKind::Field {
                label,
                value,
                action,
            } => Node::field(
                label.clone(),
                self.materialise_text(value, item),
                action
                    .as_ref()
                    .map(|action| self.materialise_action(action, item)),
            ),
            NodeKind::Tabs { state, tabs } => Node::tabs(
                *state,
                tabs.iter()
                    .map(|tab| {
                        let mut screens = self.materialise(&tab.screen, item);
                        assert_eq!(screens.len(), 1, "a tab has exactly one screen");
                        Tab::new(tab.icon.clone(), tab.action.clone(), screens.remove(0))
                    })
                    .collect(),
            ),
            NodeKind::Navigator { routes, back } => Node::navigator(
                routes
                    .iter()
                    .map(|route| {
                        let mut screens = self.materialise(&route.screen, item);
                        assert_eq!(screens.len(), 1, "a route has exactly one screen");
                        Route::new(route.path.clone(), screens.remove(0))
                    })
                    .collect(),
                back.clone(),
            ),
            NodeKind::Conditional {
                condition,
                consequent,
                alternate,
            } => {
                let enabled = self.condition_enabled(condition);
                return if enabled {
                    self.materialise(consequent, item)
                } else {
                    alternate
                        .as_deref()
                        .map_or_else(Vec::new, |alternate| self.materialise(alternate, item))
                };
            }
            NodeKind::ForEach {
                collection,
                template,
            } => {
                let items = match collection {
                    Collection::State(state) => match self.state.get(state.0) {
                        Some(StateValue::List(items)) => items.clone(),
                        _ => return Vec::new(),
                    },
                    Collection::Resource(resource, path) => match self
                        .resource_field_value(*resource, &ResourceField::Value(path.clone()))
                    {
                        Some(StateValue::List(items)) => items,
                        _ => return Vec::new(),
                    },
                    Collection::Controller(controller, path) => {
                        match self.controller_field_value(*controller, path) {
                            Some(StateValue::List(items)) => items.clone(),
                            _ => return Vec::new(),
                        }
                    }
                };
                if items.is_empty() {
                    return Vec::new();
                }
                return items
                    .iter()
                    .enumerate()
                    .flat_map(|(index, value)| {
                        self.materialise(template, Some(MaterialisedItem { value, index }))
                    })
                    .collect();
            }
            _ => node.clone(),
        };
        let mut node = node;
        node.identity = identity;
        vec![node]
    }

    fn materialise_vertical_children(
        &self,
        children: &[Node],
        item: Option<MaterialisedItem<'_>>,
    ) -> Vec<Node> {
        children
            .iter()
            .flat_map(|child| {
                if let NodeKind::ForEach { template, .. } = &child.kind
                    && virtualisable_template(template)
                {
                    vec![child.clone()]
                } else {
                    self.materialise(child, item)
                }
            })
            .collect()
    }

    fn materialise_children(
        &self,
        children: &[Node],
        item: Option<MaterialisedItem<'_>>,
    ) -> Vec<Node> {
        children
            .iter()
            .flat_map(|child| self.materialise(child, item))
            .collect()
    }

    fn materialise_text(
        &self,
        parts: &[TextPart],
        item: Option<MaterialisedItem<'_>>,
    ) -> Vec<TextPart> {
        parts
            .iter()
            .map(|part| match part {
                TextPart::Item(path) => {
                    let item = item.expect("list-item text has a mapped item");
                    let value =
                        item_at_path(item.value, path).expect("compiled list-item path is valid");
                    TextPart::literal(value.to_string())
                }
                TextPart::Value(value) => TextPart::Value(self.materialise_value(value, item)),
                part => part.clone(),
            })
            .collect()
    }

    fn materialise_action(&self, action: &Action, item: Option<MaterialisedItem<'_>>) -> Action {
        match action {
            Action::SetList { state, value } => Action::SetList {
                state: *state,
                value: self.materialise_value(value, item),
            },
            Action::AppendList { state, value } => Action::AppendList {
                state: *state,
                value: self.materialise_value(value, item),
            },
            Action::RemoveCurrentListItem { state } => Action::RemoveListItem {
                state: *state,
                index: item.expect("list action has a mapped item").index,
            },
            Action::ReplaceCurrentListItem { state, value } => Action::ReplaceListItem {
                state: *state,
                index: item.expect("list action has a mapped item").index,
                value: self.materialise_value(value, item),
            },
            Action::Controller {
                controller,
                operation,
                payload,
            } => Action::Controller {
                controller: *controller,
                operation: operation.clone(),
                payload: self.materialise_payload(payload, item),
            },
            Action::Native { operation } => Action::Native {
                operation: NativeOperation::templated(
                    operation.module.clone(),
                    operation.operation.clone(),
                    self.materialise_payload(&operation.payload, item),
                    operation.timeout_ms,
                ),
            },
            Action::Navigate { path, params } => Action::Navigate {
                path: path.clone(),
                params: params
                    .iter()
                    .map(|(name, value)| (name.clone(), self.materialise_value(value, item)))
                    .collect(),
            },
            Action::Sequence(actions) => Action::Sequence(
                actions
                    .iter()
                    .map(|action| self.materialise_action(action, item))
                    .collect(),
            ),
            action => action.clone(),
        }
    }

    fn materialise_payload(
        &self,
        payload: &[PayloadPart],
        item: Option<MaterialisedItem<'_>>,
    ) -> Vec<PayloadPart> {
        payload
            .iter()
            .map(|part| match part {
                PayloadPart::Item(path) => {
                    let value = item
                        .and_then(|item| item_at_path(item.value, path))
                        .expect("compiled list-item payload path is valid");
                    PayloadPart::Literal(
                        json_value(value).expect("list-item payload is valid JSON"),
                    )
                }
                part => part.clone(),
            })
            .collect()
    }

    fn materialise_value(&self, value: &Value, item: Option<MaterialisedItem<'_>>) -> Value {
        match value {
            Value::Item(path) => value_from_state(
                item.and_then(|item| item_at_path(item.value, path))
                    .expect("compiled list-item path is valid"),
            ),
            Value::List(values) => Value::List(
                values
                    .iter()
                    .map(|value| self.materialise_value(value, item))
                    .collect(),
            ),
            Value::Object(fields) => Value::Object(
                fields
                    .iter()
                    .map(|(name, value)| (name.clone(), self.materialise_value(value, item)))
                    .collect(),
            ),
            Value::Binary {
                left,
                operator,
                right,
            } => Value::Binary {
                left: Box::new(self.materialise_value(left, item)),
                operator: *operator,
                right: Box::new(self.materialise_value(right, item)),
            },
            value => value.clone(),
        }
    }

    fn layout_screen(
        &mut self,
        children: &[Node],
        title: Option<&str>,
        centred: bool,
        bottom_inset: bool,
        rect: Rect,
    ) {
        let has_header = title.is_some() || self.back_icon.is_some();
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
        if let Some(title) = title {
            let title_inset = header_inset
                + if self.back_icon.is_some() {
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
                colour: Colour::WHITE,
                align: TextAlign::Centre,
                scrolling: self.scrolling,
            });
        }

        let fills_remaining = children.len() == 1 && camera_preview(&children[0]);
        let inset_start = if fills_remaining {
            0.0
        } else {
            self.scaled(CONTENT_INSET_START)
        };
        let inset_end = if fills_remaining {
            0.0
        } else {
            self.scaled(CONTENT_INSET_END)
        };
        let first_child_is_full_bleed = children.first().is_some_and(full_bleed_image);
        let inset_top = if fills_remaining || first_child_is_full_bleed {
            0.0
        } else if has_header {
            self.scaled(HEADER_CONTENT_TOP)
        } else {
            self.scaled(CONTENT_TOP)
        };
        let requested_bottom_inset = if bottom_inset && !fills_remaining {
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
        let gap = if fills_remaining {
            0.0
        } else {
            self.scaled(CONTENT_GAP)
        };
        let (mut sizes, mut content_height) =
            self.measure_screen_content(children, unbounded_content, gap);
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
        if content_height > content.height && !fills_remaining {
            unbounded_content.width =
                (rect.width - inset_start - self.scaled(SCROLL_CONTENT_INSET_END)).max(0.0);
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
            Alignment::Stretch,
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
            let track_x = rect.x + rect.width - self.scaled(SCROLL_TRACK_END);
            self.scene.scroll_bar = Some(ScrollBar {
                track: Rect {
                    x: track_x,
                    y: content.y,
                    width: track_width,
                    height: content.height,
                },
                track_width,
                thumb_width,
                content_height,
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
                colour: Colour::WHITE,
                scrolling: self.scrolling,
            });
        }
    }

    fn measure_screen_content(
        &mut self,
        children: &[Node],
        available: Rect,
        gap: f32,
    ) -> (Vec<VerticalMeasure>, f32) {
        let sizes = self.measure_vertical_children(children, available);
        let entries = sizes.iter().map(|measure| measure.entries).sum::<usize>();
        let height = sizes.iter().map(|measure| measure.size.height).sum::<f32>()
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
        sizes: Vec<VerticalMeasure>,
        gap: f32,
        align: Alignment,
        justify: Justification,
        rect: Rect,
    ) {
        let entries = sizes.iter().map(|measure| measure.entries).sum::<usize>();
        let content_height = sizes.iter().map(|measure| measure.size.height).sum::<f32>()
            + gap * entries.saturating_sub(1) as f32;
        let (mut cursor, actual_gap) =
            distribution(rect.y, rect.height, content_height, gap, entries, justify);
        for (child, measure) in children.iter().zip(sizes) {
            if measure.entries == 0 {
                continue;
            }
            let size = measure.size;
            if matches!(&child.kind, NodeKind::ForEach { .. }) {
                self.layout_virtual_list(child, cursor, actual_gap, align, rect);
                cursor += size.height + actual_gap * measure.entries as f32;
                continue;
            }
            let bleed = full_bleed_image(child);
            let width = if bleed {
                self.viewport.width as f32
            } else if align == Alignment::Stretch && stretchable(child) {
                rect.width
            } else {
                size.width.min(rect.width)
            };
            let x = if bleed {
                0.0
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

    fn layout_virtual_list(
        &mut self,
        node: &Node,
        top: f32,
        gap: f32,
        align: Alignment,
        available: Rect,
    ) {
        let NodeKind::ForEach {
            collection,
            template,
        } = &node.kind
        else {
            unreachable!("only list nodes have virtual list layout")
        };
        let Some(layout) = self.virtual_lists.get(&node.identity).copied() else {
            return;
        };
        let stride = layout.row_height + gap;
        if layout.item_count == 0 || stride <= 0.0 {
            return;
        }
        let overscan = 1.0;
        let first = (((self.clip.y - top) / stride).floor() - overscan)
            .max(0.0)
            .min(layout.item_count as f32) as usize;
        let last = ((((self.clip.y + self.clip.height - top) / stride).ceil() + overscan)
            .max(0.0)
            .min(layout.item_count as f32)) as usize;

        for index in first..last {
            let Some(item) = self
                .collection_items(collection)
                .and_then(|items| items.get(index))
                .cloned()
            else {
                break;
            };
            let mut row = self.materialise(
                template,
                Some(MaterialisedItem {
                    value: &item,
                    index,
                }),
            );
            assert_eq!(row.len(), 1, "a virtual list template has one root");
            let row = row.remove(0);
            let row_y = top + stride * index as f32;
            let size = self.measure(
                &row,
                Rect {
                    y: row_y,
                    height: layout.row_height,
                    ..available
                },
            );
            let bleed = full_bleed_image(&row);
            let width = if bleed {
                self.viewport.width as f32
            } else if align == Alignment::Stretch && stretchable(&row) {
                available.width
            } else {
                size.width.min(available.width)
            };
            let x = if bleed {
                0.0
            } else {
                cross_position(available.x, available.width, width, align)
            };
            self.layout(
                &row,
                Rect {
                    x,
                    y: row_y,
                    width,
                    height: layout.row_height,
                },
            );
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
        label: &[TextPart],
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
                scrolling: self.scrolling,
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
        let label = self.resolve_text(label);
        let visible_label = self.ellipsize(&label, font_size, text_rect.width);
        let text_width = self.text_width(&visible_label, font_size);
        self.scene.text.push(TextRun {
            text: visible_label,
            rect: text_rect,
            clip: self.clip,
            font_size,
            colour: Colour::WHITE,
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
                colour: Colour::WHITE,
                scrolling: self.scrolling,
            });
        }
        if let Some(action) = action {
            self.push_hit_region(rect, action.clone());
        }
    }

    fn layout_field(
        &mut self,
        label: &str,
        value: &[TextPart],
        action: &Option<Action>,
        rect: Rect,
    ) {
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
            colour: Colour::WHITE,
            align: TextAlign::Start,
            scrolling: self.scrolling,
        });
        self.layout_button(
            value,
            None,
            false,
            &None,
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

    fn layout_text_input(
        &mut self,
        placeholder: &str,
        state: StateId,
        action: TextInputAction,
        rect: Rect,
    ) {
        let value = match self.state.get(state.0) {
            Some(StateValue::String(value)) => value.clone(),
            _ => String::new(),
        };
        let focused = self.focused_input == Some(state);
        let text = if value.is_empty() {
            placeholder
        } else {
            &value
        };
        let text_height = (rect.height - self.scaled(TEXT_INPUT_BOTTOM_PADDING)).max(0.0);
        let font_size = self.scaled_font(TEXT_INPUT_TEXT_SIZE);
        let visible_text = self.ellipsize(text, font_size, rect.width);
        self.scene.text.push(TextRun {
            text: visible_text.clone(),
            rect: Rect {
                height: text_height,
                ..rect
            },
            clip: self.clip,
            font_size,
            colour: Colour::WHITE,
            align: TextAlign::Start,
            scrolling: self.scrolling,
        });
        if focused && !value.is_empty() {
            self.scene.quads.push(Quad {
                rect: Rect {
                    x: rect.x + self.text_width(&visible_text, font_size),
                    y: rect.y + self.scaled(2.0),
                    width: self.scaled(1.0),
                    height: (text_height - self.scaled(4.0)).max(0.0),
                },
                clip: self.clip,
                colour: Colour::WHITE,
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
            colour: Colour::WHITE,
            scrolling: self.scrolling,
        });
        self.push_hit_region(rect, Action::FocusTextInput { state, action });
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
                colour: Colour::WHITE,
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
                colour: Colour::WHITE,
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
                colour: Colour::WHITE,
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
            colour: Colour::WHITE,
            align: TextAlign::Start,
            scrolling: self.scrolling,
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
            Some(StateValue::Number(value)) if value.fract() == 0.0 && *value >= 0.0 => {
                *value as usize
            }
            _ => 0,
        }
        .min(tabs.len().saturating_sub(1));
        if let Some(tab) = tabs.get(active) {
            self.layout_node(&tab.screen, content, false);
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
                mask: tab.icon.clone(),
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
                scrolling: self.scrolling,
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
                TextPart::Resource(resource, field) => {
                    if let Some(value) = self.resource_field_value(*resource, field) {
                        text.push_str(&value.to_string());
                    }
                }
                TextPart::Controller(controller, path) => {
                    if let Some(value) = self.controller_field_value(*controller, path) {
                        text.push_str(&value.to_string());
                    }
                }
                TextPart::ListLength(state) => {
                    if let Some(StateValue::List(value)) = self.state.get(state.0) {
                        text.push_str(&value.len().to_string());
                    }
                }
                TextPart::Item(_) => unreachable!("list items are materialised before layout"),
                TextPart::Value(value) => {
                    if let Some(value) = self.evaluate_value(value) {
                        text.push_str(&value.to_string());
                    }
                }
            }
        }
        text
    }

    fn layout_image(
        &mut self,
        source: &ImageSource,
        fallback: Option<&ImageAsset>,
        fit: ImageFit,
        rect: Rect,
    ) {
        let visible = rect.intersection(self.clip);
        if visible.width <= 0.0 || visible.height <= 0.0 {
            return;
        }
        let image = match source {
            ImageSource::Asset(asset) => Some(ImageData::Asset(asset.clone())),
            ImageSource::Remote(parts) => {
                let url = self.resolve_text(parts);
                if url.is_empty() {
                    fallback.cloned().map(ImageData::Asset)
                } else {
                    let key = RemoteImageKey {
                        module: "network".to_owned(),
                        url,
                        width: rect.width.ceil().max(1.0) as u32,
                        height: rect.height.ceil().max(1.0) as u32,
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
            ImageSource::Native(module, parts) => {
                let source = self.resolve_text(parts);
                if source.is_empty() {
                    fallback.cloned().map(ImageData::Asset)
                } else {
                    let key = RemoteImageKey {
                        module: module.clone(),
                        url: source,
                        width: rect.width.ceil().max(1.0) as u32,
                        height: rect.height.ceil().max(1.0) as u32,
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
        if let Some(image) = image {
            self.scene.images.push(ImageRun {
                image,
                rect,
                clip: self.clip,
                fit,
                scrolling: self.scrolling,
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
        if let Some(source) = self.camera_reviews.get(&controller).cloned() {
            let action_height = self.scaled(CAMERA_REVIEW_ACTION_HEIGHT).min(rect.height);
            self.layout_image(
                &ImageSource::Native("camera".to_owned(), vec![TextPart::Literal(source)]),
                None,
                ImageFit::Contain,
                Rect {
                    height: (rect.height - action_height).max(0.0),
                    ..rect
                },
            );
            let actions = Rect {
                y: rect.y + rect.height - action_height,
                height: action_height,
                ..rect
            };
            let half = actions.width / 2.0;
            for (label, operation, action_rect) in [
                (
                    "Retake",
                    "retake",
                    Rect {
                        width: half,
                        ..actions
                    },
                ),
                (
                    "Use photo",
                    "use-photo",
                    Rect {
                        x: actions.x + half,
                        width: actions.width - half,
                        ..actions
                    },
                ),
            ] {
                self.scene.text.push(TextRun {
                    text: label.to_owned(),
                    rect: action_rect,
                    clip: self.clip,
                    font_size: self.scaled_font(DEFAULT_TEXT_SIZE),
                    colour: Colour::WHITE,
                    align: TextAlign::Centre,
                    scrolling: self.scrolling,
                });
                self.push_hit_region(
                    action_rect,
                    Action::Controller {
                        controller,
                        operation: operation.to_owned(),
                        payload: Vec::new(),
                    },
                );
            }
            return;
        }

        let status = self
            .controller_field_value(controller, &["status".to_owned()])
            .and_then(|value| match value {
                StateValue::String(value) => Some(value),
                _ => None,
            })
            .unwrap_or_else(|| "idle".to_owned());
        if status == "ready" {
            match kind {
                CameraPreviewKind::Photo => {
                    if let Some(StateValue::String(source)) = self.controller_field_value(
                        controller,
                        &["value".to_owned(), "source".to_owned()],
                    ) {
                        self.layout_image(
                            &ImageSource::Native(
                                "camera".to_owned(),
                                vec![TextPart::Literal(source)],
                            ),
                            None,
                            ImageFit::Contain,
                            rect,
                        );
                    }
                }
                CameraPreviewKind::Scanner => {
                    let text = self
                        .controller_field_value(
                            controller,
                            &["value".to_owned(), "text".to_owned()],
                        )
                        .map_or_else(String::new, |value| value.to_string());
                    self.scene.text.push(TextRun {
                        text,
                        rect,
                        clip: self.clip,
                        font_size: self.scaled_font(DEFAULT_TEXT_SIZE),
                        colour: Colour::WHITE,
                        align: TextAlign::Centre,
                        scrolling: self.scrolling,
                    });
                }
            }
            self.push_hit_region(
                rect,
                Action::Controller {
                    controller,
                    operation: "open".to_owned(),
                    payload: Vec::new(),
                },
            );
        } else if status == "error" {
            let message = self
                .controller_field_value(controller, &["error".to_owned(), "message".to_owned()])
                .map_or_else(String::new, |value| value.to_string());
            self.scene.text.push(TextRun {
                text: message,
                rect,
                clip: self.clip,
                font_size: self.scaled_font(DEFAULT_TEXT_SIZE),
                colour: Colour::WHITE,
                align: TextAlign::Centre,
                scrolling: self.scrolling,
            });
            let retryable = self
                .controller_field_value(controller, &["error".to_owned(), "retryable".to_owned()])
                .is_some_and(|value| matches!(value, StateValue::Bool(true)));
            if retryable {
                self.push_hit_region(
                    rect,
                    Action::Controller {
                        controller,
                        operation: "open".to_owned(),
                        payload: Vec::new(),
                    },
                );
            }
        } else {
            self.scene.camera_portal = Some(CameraPortal {
                controller,
                kind,
                rect,
            });
        }
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
        self.scaled(value) * PUBLIC_SANS_RASTER_SCALE
    }

    fn control_line_height(&self) -> f32 {
        self.scaled(CONTROL_LINE_HEIGHT).ceil().max(1.0)
    }

    fn push_hit_region(&mut self, rect: Rect, action: Action) {
        let rect = rect.intersection(self.clip);
        if rect.width > 0.0 && rect.height > 0.0 {
            self.hit_regions.push(HitRegion {
                rect,
                action,
                scrolling: self.scrolling,
            });
        }
    }
}

fn evaluate_binary(
    left: StateValue,
    operator: ValueOperator,
    right: StateValue,
) -> Option<StateValue> {
    match (left, operator, right) {
        (StateValue::Number(left), ValueOperator::Add, StateValue::Number(right)) => {
            finite_number(left + right)
        }
        (StateValue::Number(left), ValueOperator::Subtract, StateValue::Number(right)) => {
            finite_number(left - right)
        }
        (StateValue::Number(left), ValueOperator::Multiply, StateValue::Number(right)) => {
            finite_number(left * right)
        }
        (StateValue::Number(left), ValueOperator::Divide, StateValue::Number(right))
            if right != 0.0 =>
        {
            finite_number(left / right)
        }
        (StateValue::String(mut left), ValueOperator::Add, StateValue::String(right)) => {
            left.push_str(&right);
            Some(StateValue::String(left))
        }
        _ => None,
    }
}

fn finite_number(value: f64) -> Option<StateValue> {
    value.is_finite().then_some(StateValue::Number(value))
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

fn assign_node_identities(node: &mut Node, next: &mut usize) {
    node.identity = NodeIdentity(*next);
    *next = next.checked_add(1).expect("an app has too many nodes");
    match &mut node.kind {
        NodeKind::Screen { children, .. } | NodeKind::Stack { children, .. } => {
            for child in children {
                assign_node_identities(child, next);
            }
        }
        NodeKind::Tabs { tabs, .. } => {
            for tab in tabs {
                assign_node_identities(&mut tab.screen, next);
            }
        }
        NodeKind::Navigator { routes, .. } => {
            for route in routes {
                assign_node_identities(&mut route.screen, next);
            }
        }
        NodeKind::Conditional {
            consequent,
            alternate,
            ..
        } => {
            assign_node_identities(consequent, next);
            if let Some(alternate) = alternate {
                assign_node_identities(alternate, next);
            }
        }
        NodeKind::ForEach { template, .. } => assign_node_identities(template, next),
        NodeKind::Text { .. }
        | NodeKind::TextInput { .. }
        | NodeKind::Button { .. }
        | NodeKind::Field { .. }
        | NodeKind::Icon { .. }
        | NodeKind::Image { .. }
        | NodeKind::CameraPreview { .. }
        | NodeKind::Toggle { .. } => {}
    }
}

fn subtree_contains_target(node: &Node, targets: &[NodeIdentity]) -> bool {
    subtree_contains(node, |identity| targets.contains(&identity))
}

fn subtree_node_count(node: &Node) -> usize {
    1 + match &node.kind {
        NodeKind::Screen { children, .. } | NodeKind::Stack { children, .. } => {
            children.iter().map(subtree_node_count).sum()
        }
        NodeKind::Tabs { tabs, .. } => tabs.iter().map(|tab| subtree_node_count(&tab.screen)).sum(),
        NodeKind::Navigator { routes, .. } => routes
            .iter()
            .map(|route| subtree_node_count(&route.screen))
            .sum(),
        NodeKind::Conditional {
            consequent,
            alternate,
            ..
        } => {
            subtree_node_count(consequent)
                + alternate.as_deref().map(subtree_node_count).unwrap_or(0)
        }
        NodeKind::ForEach { template, .. } => subtree_node_count(template),
        NodeKind::Text { .. }
        | NodeKind::TextInput { .. }
        | NodeKind::Button { .. }
        | NodeKind::Field { .. }
        | NodeKind::Icon { .. }
        | NodeKind::Image { .. }
        | NodeKind::CameraPreview { .. }
        | NodeKind::Toggle { .. } => 0,
    }
}

fn subtree_contains_identity(node: &Node, identity: NodeIdentity) -> bool {
    subtree_contains(node, |candidate| candidate == identity)
}

fn subtree_contains(node: &Node, predicate: impl Copy + Fn(NodeIdentity) -> bool) -> bool {
    if predicate(node.identity) {
        return true;
    }
    match &node.kind {
        NodeKind::Screen { children, .. } | NodeKind::Stack { children, .. } => children
            .iter()
            .any(|child| subtree_contains(child, predicate)),
        NodeKind::Tabs { tabs, .. } => tabs
            .iter()
            .any(|tab| subtree_contains(&tab.screen, predicate)),
        NodeKind::Navigator { routes, .. } => routes
            .iter()
            .any(|route| subtree_contains(&route.screen, predicate)),
        NodeKind::Conditional {
            consequent,
            alternate,
            ..
        } => {
            subtree_contains(consequent, predicate)
                || alternate
                    .as_deref()
                    .is_some_and(|node| subtree_contains(node, predicate))
        }
        NodeKind::ForEach { template, .. } => subtree_contains(template, predicate),
        NodeKind::Text { .. }
        | NodeKind::TextInput { .. }
        | NodeKind::Button { .. }
        | NodeKind::Field { .. }
        | NodeKind::Icon { .. }
        | NodeKind::Image { .. }
        | NodeKind::CameraPreview { .. }
        | NodeKind::Toggle { .. } => false,
    }
}

fn virtualisable_template(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Stack { children, .. } => children.iter().all(virtualisable_template),
        NodeKind::Text { .. }
        | NodeKind::TextInput { .. }
        | NodeKind::Button { .. }
        | NodeKind::Field { .. }
        | NodeKind::Icon { .. }
        | NodeKind::Image { .. }
        | NodeKind::Toggle { .. } => true,
        NodeKind::Screen { .. }
        | NodeKind::CameraPreview { .. }
        | NodeKind::Tabs { .. }
        | NodeKind::Navigator { .. }
        | NodeKind::Conditional { .. }
        | NodeKind::ForEach { .. } => false,
    }
}

fn tone_colour(tone: Tone) -> Colour {
    match tone {
        Tone::Primary => Colour::WHITE,
        Tone::Muted => Colour::MUTED,
    }
}

fn item_at_path<'a>(mut item: &'a StateValue, path: &[String]) -> Option<&'a StateValue> {
    for field in path {
        let StateValue::Object(fields) = item else {
            return None;
        };
        item = fields
            .iter()
            .find_map(|(name, value)| (name == field).then_some(value))?;
    }
    Some(item)
}

fn value_from_state(value: &StateValue) -> Value {
    match value {
        StateValue::Null => Value::Null,
        StateValue::Number(value) => Value::Number(*value),
        StateValue::Bool(value) => Value::Bool(*value),
        StateValue::String(value) => Value::String(value.clone()),
        StateValue::List(values) => Value::List(values.iter().map(value_from_state).collect()),
        StateValue::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), value_from_state(value)))
                .collect(),
        ),
    }
}

fn json_value(value: &StateValue) -> Option<String> {
    match value {
        StateValue::Null => Some("null".to_owned()),
        StateValue::Number(value) if value.is_finite() => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .map(|value| value.to_string()),
        StateValue::Bool(value) => Some(value.to_string()),
        StateValue::String(value) => serde_json::to_string(value).ok(),
        StateValue::Number(_) | StateValue::List(_) | StateValue::Object(_) => None,
    }
}

fn state_from_json(
    shape: &StateShape,
    value: &serde_json::Value,
    path: &str,
) -> Result<StateValue, ResourceError> {
    let mismatch = || {
        ResourceError::new(
            ResourceErrorKind::Protocol,
            format!("response field {path} had the wrong type"),
            false,
        )
    };
    match shape {
        StateShape::Null => value
            .is_null()
            .then_some(StateValue::Null)
            .ok_or_else(mismatch),
        StateShape::Number => value
            .as_f64()
            .filter(|value| value.is_finite())
            .map(StateValue::Number)
            .ok_or_else(mismatch),
        StateShape::Bool => value.as_bool().map(StateValue::Bool).ok_or_else(mismatch),
        StateShape::String => value
            .as_str()
            .map(|value| StateValue::String(value.to_owned()))
            .ok_or_else(mismatch),
        StateShape::Literal(expected) => {
            let value = match expected {
                StateLiteral::Number(_) => value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .map(StateValue::Number),
                StateLiteral::Bool(_) => value.as_bool().map(StateValue::Bool),
                StateLiteral::String(_) => value
                    .as_str()
                    .map(|value| StateValue::String(value.to_owned())),
            };
            value
                .filter(|value| expected.accepts(value))
                .ok_or_else(mismatch)
        }
        StateShape::Optional(shape) => {
            if value.is_null() {
                Ok(StateValue::Null)
            } else {
                state_from_json(shape, value, path)
            }
        }
        StateShape::Union(shapes) => shapes
            .iter()
            .find_map(|shape| state_from_json(shape, value, path).ok())
            .ok_or_else(mismatch),
        StateShape::List(item_shape) => value
            .as_array()
            .ok_or_else(mismatch)?
            .iter()
            .enumerate()
            .map(|(index, value)| state_from_json(item_shape, value, &format!("{path}[{index}]")))
            .collect::<Result<Vec<_>, _>>()
            .map(StateValue::List),
        StateShape::Object(fields) => {
            let object = value.as_object().ok_or_else(mismatch)?;
            fields
                .iter()
                .map(|(name, shape)| {
                    let field_path = format!("{path}.{name}");
                    let value = match object.get(name) {
                        None if matches!(shape, StateShape::Optional(_)) => StateValue::Null,
                        Some(value) => state_from_json(shape, value, &field_path)?,
                        None => {
                            return Err(ResourceError::new(
                                ResourceErrorKind::Protocol,
                                format!("response was missing field {field_path}"),
                                false,
                            ));
                        }
                    };
                    Ok((name.clone(), value))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(StateValue::Object)
        }
    }
}

fn parse_background_state(shape: &StateShape, bytes: &[u8]) -> ResourceState {
    let attempted_at_ms = now_ms_fallback();
    let failure = |message: String| {
        ResourceState::BackgroundFailed(BackgroundError {
            kind: "unexpected".to_owned(),
            message,
            retryable: false,
            attempted_at_ms,
        })
    };
    let value: serde_json::Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(error) => return failure(format!("background state was not valid JSON: {error}")),
    };
    let Some(object) = value.as_object() else {
        return failure("background state was not an object".to_owned());
    };
    let Some(status) = object.get("status").and_then(serde_json::Value::as_str) else {
        return failure("background state had no status".to_owned());
    };
    let parse_error = || -> Option<BackgroundError> {
        let error = object.get("error")?.as_object()?;
        let kind = error.get("kind")?.as_str()?;
        if ![
            "unavailable",
            "timeout",
            "protocol",
            "http",
            "invalid-data",
            "storage",
            "scheduler",
            "unexpected",
        ]
        .contains(&kind)
        {
            return None;
        }
        Some(BackgroundError {
            kind: kind.to_owned(),
            message: error.get("message")?.as_str()?.to_owned(),
            retryable: error.get("retryable")?.as_bool()?,
            attempted_at_ms: error.get("attemptedAtMs")?.as_f64()?,
        })
    };
    match status {
        "waiting" => ResourceState::BackgroundWaiting,
        "ready" | "stale" => {
            let Some(updated_at_ms) = object
                .get("updatedAtMs")
                .and_then(serde_json::Value::as_f64)
            else {
                return failure("background state had no update time".to_owned());
            };
            let Some(json) = object.get("value") else {
                return failure("background state had no value".to_owned());
            };
            let value = match state_from_json(shape, json, "$.value") {
                Ok(value) => value,
                Err(error) => return failure(error.message),
            };
            let error = if status == "stale" {
                let Some(error) = parse_error() else {
                    return failure("stale background state had no valid error".to_owned());
                };
                Some(error)
            } else {
                None
            };
            ResourceState::BackgroundReady {
                value,
                updated_at_ms,
                error,
            }
        }
        "error" => parse_error().map_or_else(
            || failure("failed background state had no valid error".to_owned()),
            ResourceState::BackgroundFailed,
        ),
        _ => failure(format!("unknown background status {status:?}")),
    }
}

fn now_ms_fallback() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |duration| duration.as_millis() as f64)
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
            | NodeKind::Text { .. }
            | NodeKind::TextInput { .. }
            | NodeKind::Button { .. }
            | NodeKind::Field { .. }
            | NodeKind::CameraPreview { .. }
            | NodeKind::Toggle { .. }
    )
}

fn full_bleed_image(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Image { bleed: true, .. } => true,
        NodeKind::ForEach { template, .. } => full_bleed_image(template),
        _ => false,
    }
}

fn camera_preview(node: &Node) -> bool {
    matches!(&node.kind, NodeKind::CameraPreview { .. })
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
