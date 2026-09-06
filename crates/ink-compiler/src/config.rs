use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AppConfig {
    name: String,
    package: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default = "default_version_code")]
    version_code: u32,
    signing: Option<SigningConfig>,
    lightos: Option<LightOsConfig>,
    background: Option<BackgroundConfig>,
    #[serde(default)]
    capabilities: Vec<crate::Capability>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SigningConfig {
    keystore: PathBuf,
    key_alias: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LightOsConfig {
    enabled: bool,
    #[serde(default = "default_light_server")]
    server: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackgroundConfig {
    entry: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ReleaseSigning {
    pub keystore: PathBuf,
    pub key_alias: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedConfig {
    pub(crate) name: String,
    pub(crate) package: String,
    pub(crate) version: String,
    pub(crate) version_code: u32,
    pub(crate) source: PathBuf,
    pub(crate) android_resources: PathBuf,
    pub(crate) signing: Option<ReleaseSigning>,
    pub(crate) light_server: String,
    pub(crate) capabilities: Vec<crate::Capability>,
    pub(crate) worker_entry: Option<PathBuf>,
}

impl ResolvedConfig {
    pub(crate) fn read(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("could not read {}", path.display()))?;
        let config: AppConfig =
            toml::from_str(&text).with_context(|| format!("could not parse {}", path.display()))?;
        let directory = path.parent().expect("a canonical config path has a parent");
        validate(&config)?;
        let mut capabilities = config.capabilities;
        if config
            .lightos
            .as_ref()
            .is_some_and(|lightos| lightos.enabled)
        {
            capabilities.push(crate::Capability::LightSdk);
        }

        Ok(Self {
            name: config.name,
            package: config.package,
            version: config.version,
            version_code: config.version_code,
            capabilities,
            worker_entry: config
                .background
                .map(|background| directory.join(background.entry)),
            source: directory.join("App.tsx"),
            android_resources: directory.join(".ink/android/res"),
            signing: config.signing.map(|signing| ReleaseSigning {
                keystore: directory.join(signing.keystore),
                key_alias: signing.key_alias,
            }),
            light_server: config
                .lightos
                .map_or_else(default_light_server, |lightos| lightos.server),
        })
    }
}

fn validate(config: &AppConfig) -> Result<()> {
    if let Some(lightos) = &config.lightos
        && (!lightos.server.split('.').all(valid_package_segment) || !lightos.server.contains('.'))
    {
        anyhow::bail!("lightos.server must be a dotted Android application ID");
    }
    if config.name.trim().is_empty() {
        anyhow::bail!("name must not be empty");
    }
    if config.version.trim().is_empty() {
        anyhow::bail!("version must not be empty");
    }
    if config.version_code == 0 {
        anyhow::bail!("version_code must be greater than zero");
    }
    if !config.package.split('.').all(valid_package_segment) || !config.package.contains('.') {
        anyhow::bail!(
            "package {:?} is invalid; use a dotted Android application ID such as com.example.app",
            config.package
        );
    }
    if let Some(signing) = &config.signing
        && signing.key_alias.trim().is_empty()
    {
        anyhow::bail!("signing.key_alias must not be empty");
    }
    Ok(())
}

fn valid_package_segment(segment: &str) -> bool {
    let mut characters = segment.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn default_version() -> String {
    "0.1.0".to_owned()
}

fn default_version_code() -> u32 {
    1
}

fn default_light_server() -> String {
    "com.lightos".to_owned()
}
