use std::collections::BTreeSet;

use anyhow::{Context, Result};
use serde::Serialize;

pub const MANIFEST_NAME: &str = "ink-capabilities-v1.json";
const MANIFEST_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Audio,
    AudioDetached,
    AudioPlayback,
    Background,
    CameraPermission,
    CodeScanner,
    LightSdk,
    LightSdkPush,
    LightSdkRingtone,
    Location,
    MicrophonePermission,
    Network,
    Nfc,
    NotificationPermission,
    Notifications,
    PhotoCapture,
    TextInput,
}

impl Capability {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::AudioDetached => "audio-detached",
            Self::AudioPlayback => "audio-playback",
            Self::Background => "background",
            Self::CameraPermission => "camera-permission",
            Self::CodeScanner => "code-scanner",
            Self::LightSdk => "light-sdk",
            Self::LightSdkPush => "light-sdk-push",
            Self::LightSdkRingtone => "light-sdk-ringtone",
            Self::Location => "location",
            Self::MicrophonePermission => "microphone-permission",
            Self::Network => "network",
            Self::Nfc => "nfc",
            Self::NotificationPermission => "notification-permission",
            Self::Notifications => "notifications",
            Self::PhotoCapture => "photo-capture",
            Self::TextInput => "text-input",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities(BTreeSet<Capability>);

impl Capabilities {
    pub(crate) fn insert(&mut self, capability: Capability) {
        self.0.insert(capability);
    }

    pub fn iter(&self) -> impl Iterator<Item = Capability> + '_ {
        self.0.iter().copied()
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&Manifest {
            version: MANIFEST_VERSION,
            capabilities: self.iter().collect(),
        })
        .context("could not encode Ink capabilities")
    }
}

#[derive(Serialize)]
struct Manifest {
    version: u32,
    capabilities: Vec<Capability>,
}
