use std::{collections::BTreeMap, fs, process::Command};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{Capability, Project, write_if_changed};

pub(super) struct Bundle {
    pub source: Vec<u8>,
    pub worker: Option<Vec<u8>>,
    pub icons: Vec<u8>,
    pub capabilities: crate::Capabilities,
    pub assets: Vec<Asset>,
    pub manifest: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
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
struct Metadata {
    #[serde(rename = "devRuntimeHash")]
    dev_runtime_hash: Option<String>,
    #[serde(rename = "refreshCompatibilityHash")]
    refresh_compatibility_hash: Option<String>,
    inputs: Vec<std::path::PathBuf>,
    assets: Vec<Asset>,
    capabilities: Vec<Capability>,
    icons: Vec<IconUse>,
}

pub(super) fn bundle(project: &Project) -> Result<Bundle> {
    bundle_profile(project, false)
}

pub(super) fn bundle_profile(project: &Project, development: bool) -> Result<Bundle> {
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
        .arg(&output)
        .arg(if development {
            "development"
        } else {
            "release"
        }))?;
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
            .arg(&output)
            .arg(if development {
                "development"
            } else {
                "release"
            }))?;
        let worker: Metadata =
            serde_json::from_slice(&fs::read(output.with_extension("js.metadata.json"))?)?;
        uses.inputs.extend(worker.inputs);
        uses.assets.extend(worker.assets);
        uses.capabilities.extend(worker.capabilities);
        uses.icons.extend(worker.icons);
        Some(fs::read(output)?)
    } else {
        None
    };
    let mut capabilities = crate::Capabilities::default();
    for capability in &project.config.capabilities {
        capabilities.insert(*capability);
    }
    for capability in &uses.capabilities {
        capabilities.insert(*capability);
    }
    if worker.is_some() {
        capabilities.insert(Capability::Background);
    }
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

    capabilities.resolve_dependencies()?;
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
    add("arrow_back_ios".into(), 52.0, false)?;
    add("close".into(), 52.0, false)?;
    for icon in uses.icons {
        add(icon.name, icon.size, icon.filled)?;
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
    uses.inputs.push(project.config_path().to_path_buf());
    for name in ["tsconfig.json", "package.json", "bun.lock", "bun.lockb"] {
        let path = project.root().join(name);
        if path.is_file() {
            uses.inputs.push(path);
        }
    }
    uses.inputs.sort();
    uses.inputs.dedup();
    uses.assets.sort_by(|a, b| a.name.cmp(&b.name));
    uses.assets.dedup_by(|a, b| a.name == b.name);
    let manifest = serde_json::to_vec_pretty(&serde_json::json!({
        "version": 1, "frameworkVersion": env!("CARGO_PKG_VERSION"), "protocolVersion": 1,
        "inputs": uses.inputs, "assets": uses.assets,
        "icons": icons.iter().map(|icon| serde_json::json!({"name": icon.name, "filled": icon.filled, "width": icon.width, "height": icon.height})).collect::<Vec<_>>(),
        "capabilities": capabilities.iter().collect::<Vec<_>>(), "worker": worker.is_some(),
        "devRuntimeHash": uses.dev_runtime_hash,
        "refreshCompatibilityHash": uses.refresh_compatibility_hash,
        "profile": if development { "development" } else { "release" },
    }))?;
    write_if_changed(
        &project.root().join(".ink/bundle/ink-bundle-v1.json"),
        &manifest,
    )?;
    Ok(Bundle {
        source,
        manifest,
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
