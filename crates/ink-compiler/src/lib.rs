mod codegen;
mod config;
mod diagnostic;
mod icon;
mod icons;
mod ir;
mod lower;
mod resolver;
mod source;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
pub use config::ReleaseSigning;
use config::ResolvedConfig;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppFeatures {
    pub light_sdk: bool,
    pub network: bool,
    pub text_input: bool,
    pub camera_permission: bool,
    pub audio: bool,
    pub audio_playback: bool,
    pub audio_detached: bool,
    pub microphone_permission: bool,
    pub location: bool,
    pub nfc: bool,
    pub background: bool,
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

    pub fn generated_source_path(&self) -> &Path {
        &self.config.generated
    }

    pub fn android_resources_path(&self) -> &Path {
        &self.config.android_resources
    }

    pub fn android_assets_path(&self) -> PathBuf {
        self.root().join(".ink/android/assets")
    }
}

pub fn check(project: &Project) -> Result<()> {
    generate(project)?;
    Ok(())
}

pub fn compile(project: &Project) -> Result<AppFeatures> {
    let generated = generate(project)?;
    write_if_changed(&project.config.generated, generated.source.as_bytes())?;
    icon::generate(&project.config.name, &project.config.android_resources)?;
    Ok(generated.features)
}

pub fn generate_icon(project: &Project) -> Result<()> {
    icon::generate(&project.config.name, &project.config.android_resources)
}

struct GeneratedApp {
    source: String,
    features: AppFeatures,
}

fn generate(project: &Project) -> Result<GeneratedApp> {
    let app = source::compile(project.root(), &project.config.source)?;
    write_background_registry(project, &app)?;
    let audio_playback = app
        .controllers
        .iter()
        .any(|controller| controller.kind == "player");
    let audio_detached = app.controllers.iter().any(|controller| {
        controller.kind == "player" && controller.config.contains("\"playback\":\"detached\"")
    });
    Ok(GeneratedApp {
        features: AppFeatures {
            light_sdk: app.extensions.contains(&ir::Extension::LightSdk)
                || app.extensions.contains(&ir::Extension::Location),
            network: app.extensions.contains(&ir::Extension::Network)
                || uses_remote_image(&app.root)
                || audio_playback,
            text_input: uses_text_input(&app.root),
            camera_permission: app
                .android_permissions
                .contains(&ir::AndroidPermission::Camera),
            audio: app.extensions.contains(&ir::Extension::Audio),
            audio_playback,
            audio_detached,
            microphone_permission: app
                .android_permissions
                .contains(&ir::AndroidPermission::Microphone),
            location: app.extensions.contains(&ir::Extension::Location),
            nfc: app.extensions.contains(&ir::Extension::Nfc),
            background: app.extensions.contains(&ir::Extension::Background),
        },
        source: codegen::generate(&app, project.root())?,
    })
}

fn write_background_registry(project: &Project, app: &ir::App) -> Result<()> {
    let path = project.android_assets_path().join("ink-background-v1.json");
    let mut jobs = std::collections::BTreeMap::new();
    for resource in app
        .resources
        .iter()
        .filter(|resource| resource.module == "background")
    {
        let [ir::PayloadPart::Literal(payload)] = resource.payload.as_slice() else {
            continue;
        };
        let mut value: serde_json::Value = serde_json::from_str(payload)?;
        let key = value["key"].as_str().unwrap_or_default().to_owned();
        value["bootstrapJobId"] = source::background_job_id(&key, "bootstrap").into();
        value["periodicJobId"] = source::background_job_id(&key, "periodic").into();
        jobs.insert(key, value);
    }
    if jobs.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("could not remove {}", path.display()))?;
        }
        return Ok(());
    }
    let registry = serde_json::json!({
        "version": 1,
        "jobs": jobs.into_values().collect::<Vec<_>>(),
    });
    write_if_changed(&path, registry.to_string().as_bytes())
}

fn uses_remote_image(node: &ir::Node) -> bool {
    match node {
        ir::Node::Image {
            source: ir::ImageSource::Remote(_),
            ..
        } => true,
        ir::Node::Screen { children, .. } | ir::Node::Stack { children, .. } => {
            children.iter().any(uses_remote_image)
        }
        ir::Node::Tabs { tabs, .. } => tabs.iter().any(|tab| uses_remote_image(&tab.screen)),
        ir::Node::Navigator { routes } => {
            routes.iter().any(|route| uses_remote_image(&route.screen))
        }
        ir::Node::Conditional {
            consequent,
            alternate,
            ..
        } => uses_remote_image(consequent) || alternate.as_deref().is_some_and(uses_remote_image),
        ir::Node::ForEach { template, .. } => uses_remote_image(template),
        ir::Node::Text { .. }
        | ir::Node::TextInput { .. }
        | ir::Node::Button { .. }
        | ir::Node::SelectorButton { .. }
        | ir::Node::Icon { .. }
        | ir::Node::Image { .. }
        | ir::Node::Toggle { .. } => false,
        ir::Node::ScreenModule { .. } => {
            unreachable!("screen modules are expanded before feature detection")
        }
    }
}

fn uses_text_input(node: &ir::Node) -> bool {
    match node {
        ir::Node::TextInput { .. } => true,
        ir::Node::Screen { children, .. } | ir::Node::Stack { children, .. } => {
            children.iter().any(uses_text_input)
        }
        ir::Node::Tabs { tabs, .. } => tabs.iter().any(|tab| uses_text_input(&tab.screen)),
        ir::Node::Navigator { routes } => routes.iter().any(|route| uses_text_input(&route.screen)),
        ir::Node::Conditional {
            consequent,
            alternate,
            ..
        } => uses_text_input(consequent) || alternate.as_deref().is_some_and(uses_text_input),
        ir::Node::ForEach { template, .. } => uses_text_input(template),
        ir::Node::Text { .. }
        | ir::Node::Button { .. }
        | ir::Node::SelectorButton { .. }
        | ir::Node::Icon { .. }
        | ir::Node::Image { .. }
        | ir::Node::Toggle { .. } => false,
        ir::Node::ScreenModule { .. } => {
            unreachable!("screen modules are expanded before feature detection")
        }
    }
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
