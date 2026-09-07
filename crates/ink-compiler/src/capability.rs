use std::collections::BTreeSet;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const MANIFEST_NAME: &str = "ink-capabilities-v1.json";
const MANIFEST_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Audio,
    AudioCapture,
    AudioDetached,
    AudioPlayback,
    Background,
    BarcodeGenerate,
    CameraPermission,
    CodeScanner,
    Connectivity,
    Downloads,
    Files,
    Image,
    LightSdk,
    LightSdkPush,
    LightSdkRingtone,
    Location,
    Maps,
    MediaLibrary,
    MicrophonePermission,
    Network,
    Nfc,
    NotificationPermission,
    Notifications,
    PhotoCapture,
    TextInput,
    TextInputFull,
}

impl Capability {
    pub fn name(self) -> &'static str {
        let value = serde_json::to_value(self).expect("capability name");
        catalogue()
            .as_object()
            .expect("capability catalogue")
            .get_key_value(value.as_str().expect("capability name"))
            .expect("capability exists in catalogue")
            .0
    }

    pub fn native_cost(self) -> &'static str {
        catalogue()[self.name()]["cost"]
            .as_str()
            .expect("catalogue cost")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities(BTreeSet<Capability>);

impl Capabilities {
    pub(crate) fn insert(&mut self, capability: Capability) {
        self.0.insert(capability);
    }

    pub(crate) fn resolve_dependencies(&mut self) -> Result<()> {
        loop {
            let before = self.0.len();
            for capability in self.iter().collect::<Vec<_>>() {
                for dependency in catalogue()[capability.name()]["dependencies"]
                    .as_array()
                    .context("missing capability dependencies")?
                {
                    self.insert(serde_json::from_value(dependency.clone())?);
                }
            }
            if self.0.len() == before {
                return Ok(());
            }
        }
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

fn catalogue() -> &'static serde_json::Value {
    static VALUE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    &VALUE.get_or_init(|| {
        serde_json::from_str(include_str!("../capabilities-v1.json"))
            .expect("valid capability catalogue")
    })["capabilities"]
}
