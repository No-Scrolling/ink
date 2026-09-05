mod capability;
mod config;
mod design;
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
    pub capabilities: Vec<String>,
    pub capability_details: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Project {
    config_path: PathBuf,
    config: ResolvedConfig,
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
        })
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
        self.root().join(".ink/android/assets")
    }

    pub fn capability_manifest_path(&self) -> PathBuf {
        self.android_assets_path().join(capability::MANIFEST_NAME)
    }
}

pub fn check(project: &Project) -> Result<()> {
    javascript::bundle(project)?;
    Ok(())
}

pub fn inspect(project: &Project) -> Result<AppInfo> {
    let bundle = javascript::bundle(project)?;
    Ok(AppInfo {
        javascript_bytes: bundle.source.len(),
        capabilities: bundle
            .capabilities
            .iter()
            .map(|capability| capability.name().to_owned())
            .collect(),
        capability_details: bundle
            .capabilities
            .iter()
            .map(|capability| format!("{}: {}", capability.name(), capability.native_cost()))
            .collect(),
    })
}

pub fn compile(project: &Project) -> Result<Capabilities> {
    let bundle = javascript::bundle(project)?;
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
        &project.android_assets_path().join("ink-icons-v1.json"),
        &bundle.icons,
    )?;
    write_if_changed(&project.capability_manifest_path(), &capabilities.encode()?)?;
    generate_icon(project)?;
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
