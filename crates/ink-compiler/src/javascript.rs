use std::{collections::BTreeMap, fs, process::Command};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{Capability, Project, write_if_changed};

pub(super) struct Bundle {
    pub source: Vec<u8>,
    pub worker: Option<Vec<u8>>,
    pub icons: Vec<u8>,
    pub capabilities: crate::Capabilities,
    pub assets: Vec<Asset>,
}

#[derive(Deserialize)]
pub(super) struct Asset {
    pub name: String,
    pub path: std::path::PathBuf,
}

#[derive(Deserialize)]
struct IconUse {
    name: String,
    size: f32,
    filled: bool,
}

#[derive(Deserialize)]
struct DynamicIcons {
    size: f32,
    filled: bool,
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<String>,
    assets: Vec<Asset>,
    #[serde(rename = "remoteImages")]
    remote_images: bool,
    #[serde(rename = "detachedAudio")]
    detached_audio: bool,
    #[serde(rename = "audioPlayback")]
    audio_playback: bool,
    #[serde(rename = "audioCapture")]
    audio_capture: bool,
    #[serde(rename = "photoCapture")]
    photo_capture: bool,
    #[serde(rename = "codeScanner")]
    code_scanner: bool,
    components: Vec<String>,
    icons: Vec<IconUse>,
    dynamic: Vec<DynamicIcons>,
    strings: Vec<String>,
}

pub(super) fn bundle(project: &Project) -> Result<Bundle> {
    let compiler = project
        .root()
        .ancestors()
        .map(|root| root.join("node_modules/typescript/bin/tsc"))
        .find(|path| path.is_file())
        .context("TypeScript is not installed; install the application's dependencies first")?;
    run(Command::new("bun")
        .current_dir(project.root())
        .arg(compiler)
        .args(["--noEmit", "--project", "tsconfig.json"]))?;

    let entry = project.root().join(".ink/entry.tsx");
    let output = project.root().join(".ink/bundle/app.js");
    let source = format!(
        "import {{ createElement }} from 'react';\nimport {{ render }} from 'ink/renderer';\nimport App from {};\nrender(createElement(App));\n",
        serde_json::to_string(project.source_path())?,
    );
    write_if_changed(&entry, source.as_bytes())?;
    let builder = project.root().join(".ink/build.js");
    write_if_changed(&builder, include_bytes!("bundle-javascript.js"))?;
    run(Command::new("bun")
        .current_dir(project.root())
        .arg(&builder)
        .arg(project.root())
        .arg(&entry)
        .arg(&output))?;
    let source =
        fs::read(&output).with_context(|| format!("could not read {}", output.display()))?;
    let mut uses: Metadata =
        serde_json::from_slice(&fs::read(output.with_extension("js.metadata.json"))?)?;
    let worker = if let Some(worker_source) = &project.config.worker_entry {
        let entry = project.root().join(".ink/worker-entry.ts");
        let output = project.root().join(".ink/bundle/worker.js");
        let source = format!(
            "import {};\nimport {{ startWorker }} from '@ink/background/worker';\nstartWorker();\n",
            serde_json::to_string(worker_source)?,
        );
        write_if_changed(&entry, source.as_bytes())?;
        run(Command::new("bun")
            .current_dir(project.root())
            .arg(&builder)
            .arg(project.root())
            .arg(&entry)
            .arg(&output))?;
        let worker: Metadata =
            serde_json::from_slice(&fs::read(output.with_extension("js.metadata.json"))?)?;
        anyhow::ensure!(
            worker.components.is_empty(),
            "Background workers cannot render UI components"
        );
        uses.packages.extend(worker.packages);
        uses.assets.extend(worker.assets);
        uses.remote_images |= worker.remote_images;
        uses.detached_audio |= worker.detached_audio;
        uses.audio_playback |= worker.audio_playback;
        uses.audio_capture |= worker.audio_capture;
        uses.photo_capture |= worker.photo_capture;
        uses.code_scanner |= worker.code_scanner;
        Some(fs::read(output)?)
    } else {
        None
    };
    let mut capabilities = crate::Capabilities::default();
    for capability in &project.config.capabilities {
        capabilities.insert(*capability);
    }
    for package in &uses.packages {
        let required: &[Capability] = match package.as_str() {
            "@ink/audio" => &[Capability::Audio],
            "@ink/location" => &[Capability::LightSdk, Capability::Location],
            "@ink/nfc" => &[Capability::Nfc],
            "@ink/background" | "@ink/background/worker" => &[Capability::Background],
            "@ink/notifications" => &[
                Capability::Notifications,
                Capability::NotificationPermission,
            ],
            "@ink/camera" => &[Capability::LightSdk, Capability::CameraPermission],
            "@ink/lightos" => &[Capability::LightSdk],
            "@ink/lightos/ringtone" => &[Capability::LightSdk, Capability::LightSdkRingtone],
            "@ink/lightos/push" => &[
                Capability::LightSdk,
                Capability::LightSdkPush,
                Capability::Notifications,
                Capability::Network,
            ],
            "@ink/barcode/generate" => &[Capability::BarcodeGenerate, Capability::Image],
            _ => &[],
        };
        for capability in required {
            capabilities.insert(*capability);
        }
    }
    for (enabled, capability) in [
        (worker.is_some(), Capability::Background),
        (uses.photo_capture, Capability::PhotoCapture),
        (
            uses.photo_capture || uses.components.iter().any(|name| name == "Image"),
            Capability::Image,
        ),
        (uses.code_scanner, Capability::CodeScanner),
        (uses.audio_capture, Capability::MicrophonePermission),
        (uses.audio_playback, Capability::AudioPlayback),
        (uses.detached_audio, Capability::AudioDetached),
        (uses.remote_images, Capability::Network),
        (
            uses.components.iter().any(|name| name == "TextInput"),
            Capability::TextInput,
        ),
    ] {
        if enabled {
            capabilities.insert(capability);
        }
    }
    let mut icons = BTreeMap::<(String, bool), f32>::new();
    let mut add = |name: String, size: f32, filled| -> Result<()> {
        anyhow::ensure!(
            size.is_finite() && size > 0.0,
            "invalid size for icon {name:?}"
        );
        icons
            .entry((name, filled))
            .and_modify(|existing| *existing = existing.max(size))
            .or_insert(size);
        Ok(())
    };
    if uses
        .components
        .iter()
        .any(|component| component == "Navigator")
    {
        add(
            "arrow_back_ios".into(),
            crate::design::HEADER_BACK_ICON_SIZE,
            false,
        )?;
    }
    if uses
        .components
        .iter()
        .any(|component| component == "TextInput")
    {
        add(
            "close".into(),
            crate::design::TEXT_INPUT_CLEAR_ICON_SIZE,
            false,
        )?;
    }
    for icon in uses.icons {
        add(icon.name, icon.size, icon.filled)?;
    }
    for dynamic in uses.dynamic {
        for name in &uses.strings {
            if crate::icons::exists(name) {
                add(name.clone(), dynamic.size, dynamic.filled)?;
            }
        }
    }
    let icons = icons
        .into_iter()
        .map(|((name, filled), size)| {
            let mask = if filled {
                crate::icons::raster_filled(&name, size)
            } else {
                crate::icons::raster(&name, size)
            }?;
            Ok(ink_core::ReactIcon {
                name,
                filled,
                id: mask.id,
                width: mask.width,
                height: mask.height,
                pixels: mask.pixels,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Bundle {
        source,
        worker,
        icons: serde_json::to_vec(&icons)?,
        capabilities,
        assets: uses.assets,
    })
}

fn run(command: &mut Command) -> Result<()> {
    let output = command
        .output()
        .context("could not run Bun; install Bun and the application's dependencies")?;
    if !output.status.success() {
        bail!(
            "JavaScript build failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
