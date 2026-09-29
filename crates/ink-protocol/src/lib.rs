pub use smol_str::SmolStr;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct ReactCommit(pub Vec<Operation>);

#[derive(Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Operation {
    Create {
        id: usize,
        r#type: HostKind,
        props: Map<String, Json>,
    },
    Update {
        id: usize,
        props: Map<String, Json>,
    },
    Insert {
        id: usize,
        parent: usize,
        before: Option<usize>,
    },
    Remove {
        id: usize,
        parent: usize,
    },
    Text {
        ids: Vec<usize>,
        values: Vec<SmolStr>,
    },
    Hidden {
        id: usize,
        value: bool,
    },
    Values {
        view: usize,
        values: Vec<(usize, Json)>,
        #[serde(default)]
        collections: Vec<CollectionPatch>,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionPatch {
    pub source: usize,
    pub revision: u64,
    pub edits: Vec<CollectionEdit>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "lowercase", deny_unknown_fields)]
pub enum CollectionEdit {
    Reverse,
    Reset { items: Vec<Json> },
    Insert { item: Json, before: Option<String> },
    Update { key: String, value: Map<String, Json> },
    Remove { key: String },
    Move { key: String, before: Option<String> },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum HostKind {
    #[default]
    #[serde(skip)]
    Root,
    #[serde(rename = "#text")]
    RawText,
    NativeView,
    NativeList,
    PlayingScreen,
    List,
    Navigator,
    Tab,
    Tabs,
    Confirmation,
    Screen,
    ScreenState,
    Stack,
    Canvas,
    CanvasRectangle,
    CanvasText,
    CanvasIcon,
    Text,
    TextInput,
    Barcode,
    CameraPreview,
    MapView,
    VideoView,
    MediaPickerScreen,
    MediaGridRow,
    MediaCell,
    Image,
    Icon,
    Toggle,
    Button,
    Field,
    Row,
    RowContent,
    Avatar,
    RowTitle,
    Message,
    MessageContent,
    MessageQuote,
    LinkPreview,
    ConversationComposer,
    PlayingLayout,
    PlayingTransport,
    Pressable,
    PlayingProgress,
    PlayingLabel,
    PitchIndicator,
    CaptureReadout,
}
