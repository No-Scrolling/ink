mod capability;
mod config;
mod icon;
mod icons;
mod javascript;

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
pub use capability::{Capabilities, Capability};
pub use config::ReleaseSigning;
use config::ResolvedConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppInfo {
    pub javascript_bytes: usize,
    pub resolved_inputs: usize,
    pub asset_count: usize,
    pub icon_variants: usize,
    pub capabilities: Vec<String>,
    pub capability_details: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Project {
    config_path: PathBuf,
    config: ResolvedConfig,
    preview: bool,
}

impl Project {
    pub fn load(config_path: impl AsRef<Path>) -> Result<Self> {
        let config_path = config_path
            .as_ref()
            .canonicalize()
            .with_context(|| format!("could not find {}", config_path.as_ref().display()))?;
        let config = ResolvedConfig::read(&config_path)?;
        Ok(Self {
            config_path,
            config,
            preview: false,
        })
    }

    /// Compile design exports separately, without background workers.
    pub fn for_preview(&self) -> Self {
        let mut project = self.clone();
        project.preview = true;
        project.config.worker_entry = None;
        project
    }

    pub fn work_path(&self) -> PathBuf {
        self.root().join(if self.preview { ".ink/design" } else { ".ink" })
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn package(&self) -> &str {
        &self.config.package
    }

    pub fn version(&self) -> &str {
        &self.config.version
    }

    pub fn version_code(&self) -> u32 {
        self.config.version_code
    }

    pub fn release_signing(&self) -> Option<&ReleaseSigning> {
        self.config.signing.as_ref()
    }

    pub fn light_server(&self) -> &str {
        &self.config.light_server
    }

    pub fn auth_redirect_uri(&self) -> Option<&str> {
        self.config.auth_redirect_uri.as_deref()
    }

    pub fn root(&self) -> &Path {
        self.config_path
            .parent()
            .expect("a canonical config path has a parent")
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn source_path(&self) -> &Path {
        &self.config.source
    }

    pub fn android_resources_path(&self) -> &Path {
        &self.config.android_resources
    }

    pub fn android_assets_path(&self) -> PathBuf {
        self.work_path().join("android/assets")
    }

    pub fn bundle_manifest_path(&self) -> PathBuf {
        self.android_assets_path().join("ink-bundle-v1.json")
    }

    pub fn capability_manifest_path(&self) -> PathBuf {
        self.android_assets_path().join(capability::MANIFEST_NAME)
    }
}

pub fn check(project: &Project) -> Result<()> {
    javascript::bundle(project)?;
    Ok(())
}

/// Read the last compiled bundle without compiling or changing the project.
pub fn inspect(project: &Project) -> Result<Option<AppInfo>> {
    let directory = project.work_path().join("bundle");
    let manifest_path = directory.join("ink-bundle-v1.json");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(manifest_path)?)?;
    let capabilities: Vec<Capability> = serde_json::from_value(manifest["capabilities"].clone())?;
    Ok(Some(AppInfo {
        javascript_bytes: manifest["javascriptBytes"].as_u64().context("cached bundle has no size metadata; run ink check to refresh it")? as usize,
        resolved_inputs: manifest["inputs"].as_array().map_or(0, Vec::len),
        asset_count: manifest["assets"].as_array().map_or(0, Vec::len),
        icon_variants: manifest["icons"].as_array().map_or(0, Vec::len),
        capabilities: capabilities.iter().map(|capability| capability.name().to_owned()).collect(),
        capability_details: capabilities.iter().map(|capability| format!("{}: {}", capability.name(), capability.native_cost())).collect(),
    }))
}

pub fn compile(project: &Project) -> Result<Capabilities> {
    compile_profile(project, false)
}

pub fn compile_development(project: &Project) -> Result<Capabilities> {
    compile_profile(project, true)
}

fn compile_profile(project: &Project, development: bool) -> Result<Capabilities> {
    let bundle = javascript::bundle_profile(project, development)?;
    install_bundle(project, bundle, development)
}

/// Compile a design entry point, optionally generating file routes.
pub fn compile_preview(project: &Project, source: &str, routes: bool) -> Result<Capabilities> {
    let bundle = javascript::bundle_entry(project, false, Some(source), routes)?;
    install_bundle(project, bundle, false)
}

fn install_bundle(
    project: &Project,
    bundle: javascript::Bundle,
    development: bool,
) -> Result<Capabilities> {
    if development {
        write_if_changed(&project.bundle_manifest_path(), &bundle.manifest)?;
    } else {
        remove_obsolete_output(&project.bundle_manifest_path())?;
    }
    for name in ["app.js.map", "worker.js.map"] {
        let path = project.work_path().join("bundle").join(name);
        let destination = project.android_assets_path().join(name);
        if development && (name == "app.js.map" || bundle.worker.is_some()) && path.is_file() {
            write_if_changed(&destination, &std::fs::read(path)?)?;
        } else {
            remove_obsolete_output(&destination)?;
        }
    }
    let asset_directory = project.android_assets_path().join("ink-assets");
    let names: BTreeSet<_> = bundle
        .assets
        .iter()
        .map(|asset| asset.name.as_str())
        .collect();
    if asset_directory.is_dir() {
        for entry in std::fs::read_dir(&asset_directory)? {
            let entry = entry?;
            if !names.contains(entry.file_name().to_string_lossy().as_ref()) {
                std::fs::remove_file(entry.path())?;
            }
        }
    }
    for asset in &bundle.assets {
        write_if_changed(
            &asset_directory.join(&asset.name),
            &std::fs::read(&asset.path)?,
        )?;
    }
    let capabilities = bundle.capabilities;
    let worker_path = project.android_assets_path().join("worker.js");
    if let Some(worker) = &bundle.worker {
        write_if_changed(&worker_path, worker)?;
    } else {
        remove_obsolete_output(&worker_path)?;
    }
    write_if_changed(
        &project.android_assets_path().join("app.js"),
        &bundle.source,
    )?;
    write_if_changed(
        &project.android_assets_path().join("ink-icons-v1.bin"),
        &bundle.icons,
    )?;
    remove_obsolete_output(&project.android_assets_path().join("ink-icons-v1.json"))?;
    write_if_changed(&project.capability_manifest_path(), &capabilities.encode()?)?;
    if !project.preview {
        generate_icon(project)?;
    }
    Ok(capabilities)
}

pub fn generate_icon(project: &Project) -> Result<()> {
    icon::generate(&project.config.name, &project.config.android_resources)
}

fn write_if_changed(path: &Path, contents: &[u8]) -> Result<()> {
    if std::fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)
            .with_context(|| format!("could not create {}", directory.display()))?;
    }
    std::fs::write(path, contents).with_context(|| format!("could not write {}", path.display()))
}

fn remove_obsolete_output(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("could not remove {}", path.display())),
    }
}
