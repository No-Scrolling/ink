use crate::{capability::Capability, ir};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExtensionFunction {
    LightSdkVersion,
    LightSdkPermission,
    OpenDialler,
    RingtoneInstaller,
    LightPush,
    Json,
    CachedJson,
    Mutation,
    MicrophonePermission,
    LevelMeter,
    PitchDetector,
    AudioPlayer,
    AudioRecorder,
    LocationPermission,
    CurrentLocation,
    NfcTag,
    PeriodicJson,
    NotificationPermission,
    LocalNotifications,
    NotificationTap,
    CameraPermission,
    PhotoCapture,
    CodeScanner,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExtensionElement {
    CameraPreview,
}

#[derive(Clone, Copy)]
pub enum ExportKind {
    Function(ExtensionFunction),
    Element(ExtensionElement),
}

#[derive(Clone, Copy)]
pub struct ExportSpec {
    pub name: &'static str,
    pub kind: ExportKind,
}

pub struct ModuleSpec {
    pub extension: ir::Extension,
    pub package: &'static str,
    pub name: &'static str,
    pub api_version: &'static str,
    pub capabilities: &'static [Capability],
    pub exports: &'static [ExportSpec],
}

const LIGHT_SDK_EXPORTS: &[ExportSpec] = &[
    function("lightSdkVersion", ExtensionFunction::LightSdkVersion),
    function("lightSdkPermission", ExtensionFunction::LightSdkPermission),
    function("openDialler", ExtensionFunction::OpenDialler),
    function("ringtoneInstaller", ExtensionFunction::RingtoneInstaller),
    function("lightPush", ExtensionFunction::LightPush),
];
const NETWORK_EXPORTS: &[ExportSpec] = &[
    function("json", ExtensionFunction::Json),
    function("cachedJson", ExtensionFunction::CachedJson),
    function("mutation", ExtensionFunction::Mutation),
];
const AUDIO_EXPORTS: &[ExportSpec] = &[
    function(
        "microphonePermission",
        ExtensionFunction::MicrophonePermission,
    ),
    function("levelMeter", ExtensionFunction::LevelMeter),
    function("pitchDetector", ExtensionFunction::PitchDetector),
    function("audioPlayer", ExtensionFunction::AudioPlayer),
    function("audioRecorder", ExtensionFunction::AudioRecorder),
];
const LOCATION_EXPORTS: &[ExportSpec] = &[
    function("locationPermission", ExtensionFunction::LocationPermission),
    function("currentLocation", ExtensionFunction::CurrentLocation),
];
const NFC_EXPORTS: &[ExportSpec] = &[function("nfcTag", ExtensionFunction::NfcTag)];
const BACKGROUND_EXPORTS: &[ExportSpec] =
    &[function("periodicJson", ExtensionFunction::PeriodicJson)];
const NOTIFICATION_EXPORTS: &[ExportSpec] = &[
    function(
        "notificationPermission",
        ExtensionFunction::NotificationPermission,
    ),
    function("localNotifications", ExtensionFunction::LocalNotifications),
    function("notificationTap", ExtensionFunction::NotificationTap),
];
const CAMERA_EXPORTS: &[ExportSpec] = &[
    function("cameraPermission", ExtensionFunction::CameraPermission),
    function("photoCapture", ExtensionFunction::PhotoCapture),
    function("codeScanner", ExtensionFunction::CodeScanner),
    ExportSpec {
        name: "CameraPreview",
        kind: ExportKind::Element(ExtensionElement::CameraPreview),
    },
];

const MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        extension: ir::Extension::LightSdk,
        package: "@ink/light-sdk",
        name: "light-sdk",
        api_version: "0.1.1",
        capabilities: &[Capability::LightSdk],
        exports: LIGHT_SDK_EXPORTS,
    },
    ModuleSpec {
        extension: ir::Extension::Network,
        package: "@ink/network",
        name: "network",
        api_version: "1",
        capabilities: &[Capability::Network],
        exports: NETWORK_EXPORTS,
    },
    ModuleSpec {
        extension: ir::Extension::Audio,
        package: "@ink/audio",
        name: "audio",
        api_version: "1",
        capabilities: &[Capability::Audio],
        exports: AUDIO_EXPORTS,
    },
    ModuleSpec {
        extension: ir::Extension::Location,
        package: "@ink/location",
        name: "location",
        api_version: "1",
        capabilities: &[Capability::LightSdk, Capability::Location],
        exports: LOCATION_EXPORTS,
    },
    ModuleSpec {
        extension: ir::Extension::Nfc,
        package: "@ink/nfc",
        name: "nfc",
        api_version: "1",
        capabilities: &[Capability::Nfc],
        exports: NFC_EXPORTS,
    },
    ModuleSpec {
        extension: ir::Extension::Background,
        package: "@ink/background",
        name: "background",
        api_version: "1",
        capabilities: &[Capability::Background],
        exports: BACKGROUND_EXPORTS,
    },
    ModuleSpec {
        extension: ir::Extension::Notifications,
        package: "@ink/notifications",
        name: "notifications",
        api_version: "1",
        capabilities: &[Capability::Notifications],
        exports: NOTIFICATION_EXPORTS,
    },
    ModuleSpec {
        extension: ir::Extension::Camera,
        package: "@ink/camera",
        name: "camera",
        api_version: "1",
        capabilities: &[Capability::LightSdk],
        exports: CAMERA_EXPORTS,
    },
];

const fn function(name: &'static str, function: ExtensionFunction) -> ExportSpec {
    ExportSpec {
        name,
        kind: ExportKind::Function(function),
    }
}

pub fn by_extension(extension: ir::Extension) -> &'static ModuleSpec {
    MODULES
        .iter()
        .find(|module| module.extension == extension)
        .expect("every compiler extension has a module schema")
}

pub fn by_package(package: &str, name: &str) -> Option<&'static ModuleSpec> {
    MODULES
        .iter()
        .find(|module| module.package == package && module.name == name)
}

pub fn export(extension: ir::Extension, name: &str) -> Option<ExportKind> {
    by_extension(extension)
        .exports
        .iter()
        .find(|export| export.name == name)
        .map(|export| export.kind)
}
