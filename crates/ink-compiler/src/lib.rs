mod bundle;
mod capability;
mod config;
mod dependency;
mod design;
mod diagnostic;
mod icon;
mod icons;
mod ir;
mod lower;
mod module_schema;
mod resolver;
mod schema;
mod source;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
pub use capability::{Capabilities, Capability};
pub use config::ReleaseSigning;
use config::ResolvedConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppInfo {
    pub capabilities: Vec<String>,
    pub capability_details: Vec<String>,
    pub modules: Vec<String>,
    pub permissions: Vec<String>,
    pub resources: Vec<String>,
    pub controllers: Vec<String>,
    pub local_states: usize,
    pub shared_states: usize,
    pub persisted_states: usize,
    pub definition_bytes: usize,
    pub nodes: usize,
    pub state_dependencies: usize,
    pub layout_dependencies: usize,
    pub structural_dependencies: usize,
    pub virtual_lists: usize,
    pub general_lists: usize,
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

    pub fn app_definition_path(&self) -> PathBuf {
        self.android_assets_path().join(ink_app_format::ASSET_NAME)
    }

    pub fn capability_manifest_path(&self) -> PathBuf {
        self.android_assets_path().join(capability::MANIFEST_NAME)
    }
}

pub fn check(project: &Project) -> Result<()> {
    generate(project)?;
    Ok(())
}

pub fn inspect(project: &Project) -> Result<AppInfo> {
    let app = source::compile(project.root(), &project.config.source)?;
    let detected = detect_capabilities(&app);
    let capabilities = detected
        .iter()
        .map(Capability::name)
        .map(str::to_owned)
        .collect();
    let capability_details = detected
        .iter()
        .map(|capability| {
            format!(
                "{}: {} ({})",
                capability.name(),
                capability_reason(&app, capability),
                capability.native_cost(),
            )
        })
        .collect();
    let extension_names = app
        .extensions
        .iter()
        .map(|extension| module_schema::by_extension(*extension).name)
        .collect::<BTreeSet<_>>();
    let mut modules = app
        .extensions
        .iter()
        .map(|extension| module_schema::by_extension(*extension))
        .map(|module| format!("{} API {}", module.name, module.api_version))
        .collect::<BTreeSet<_>>();
    modules.extend(
        app.resources
            .iter()
            .filter(|resource| !extension_names.contains(resource.module.as_str()))
            .map(|resource| resource.module.clone()),
    );
    modules.extend(
        app.controllers
            .iter()
            .filter(|controller| !extension_names.contains(controller.module.as_str()))
            .map(|controller| controller.module.clone()),
    );
    let permissions = app
        .android_permissions
        .iter()
        .map(|permission| match permission {
            ir::AndroidPermission::Camera => "camera",
            ir::AndroidPermission::Microphone => "microphone",
            ir::AndroidPermission::Location => "location",
            ir::AndroidPermission::Nfc => "nfc",
            ir::AndroidPermission::Notifications => "notifications",
        })
        .map(str::to_owned)
        .collect();
    let resources = summarise(
        app.resources
            .iter()
            .map(|resource| format!("{}/{}", resource.module, resource.operation)),
    );
    let controllers = summarise(
        app.controllers
            .iter()
            .map(|controller| format!("{}/{}", controller.module, controller.kind)),
    );
    let local_states = app
        .states
        .iter()
        .filter(|state| matches!(state.lifetime, ir::StateLifetime::Local))
        .count();
    let shared_states = app
        .states
        .iter()
        .filter(|state| matches!(state.lifetime, ir::StateLifetime::Shared(_)))
        .count();
    let persisted_states = app
        .states
        .iter()
        .filter(|state| matches!(state.lifetime, ir::StateLifetime::Persisted(_)))
        .count();
    let dependencies = dependency::analyse(&app.root);
    let layout_dependencies = dependencies
        .iter()
        .filter(|dependency| dependency.kind == ink_app_format::DependencyKind::Layout)
        .count();
    let structural_dependencies = dependencies
        .iter()
        .filter(|dependency| dependency.kind == ink_app_format::DependencyKind::Structure)
        .count();
    let tree = tree_summary(&app.root);
    let definition_bytes = bundle::generate(&app, project.root())?.len();
    Ok(AppInfo {
        capabilities,
        capability_details,
        modules: modules.into_iter().collect(),
        permissions,
        resources,
        controllers,
        local_states,
        shared_states,
        persisted_states,
        definition_bytes,
        nodes: tree.nodes,
        state_dependencies: dependencies.len(),
        layout_dependencies,
        structural_dependencies,
        virtual_lists: tree.virtual_lists,
        general_lists: tree.general_lists,
    })
}

fn capability_reason(app: &ir::App, capability: Capability) -> String {
    let modules = app
        .extensions
        .iter()
        .map(|extension| module_schema::by_extension(*extension))
        .filter(|module| module.capabilities.contains(&capability))
        .map(|module| format!("{} import", module.package))
        .collect::<Vec<_>>();
    if !modules.is_empty() {
        return modules.join(" and ");
    }
    match capability {
        Capability::AudioDetached => "detached audio player".to_owned(),
        Capability::AudioPlayback => "audio player".to_owned(),
        Capability::CameraPermission => "camera permission request".to_owned(),
        Capability::CodeScanner => "code scanner controller".to_owned(),
        Capability::LightSdkPush => "Light SDK push controller".to_owned(),
        Capability::LightSdkRingtone => "ringtone installer controller".to_owned(),
        Capability::MicrophonePermission => "microphone permission request".to_owned(),
        Capability::Network if uses_remote_image(&app.root) => "remote image".to_owned(),
        Capability::Network => "audio playback transport".to_owned(),
        Capability::NotificationPermission => "notification permission request".to_owned(),
        Capability::PhotoCapture => "photo capture controller".to_owned(),
        Capability::TextInput => "TextInput component".to_owned(),
        capability => format!("{} operation", capability.name()),
    }
}

#[derive(Default)]
struct TreeSummary {
    nodes: usize,
    virtual_lists: usize,
    general_lists: usize,
}

fn tree_summary(node: &ir::Node) -> TreeSummary {
    let mut summary = TreeSummary {
        nodes: 1,
        ..TreeSummary::default()
    };
    match node {
        ir::Node::Screen { children, .. } | ir::Node::Stack { children, .. } => {
            for child in children {
                summary.add(tree_summary(child));
            }
        }
        ir::Node::Tabs { tabs, .. } => {
            for tab in tabs {
                summary.add(tree_summary(&tab.screen));
            }
        }
        ir::Node::Navigator { routes } => {
            for route in routes {
                summary.add(tree_summary(&route.screen));
            }
        }
        ir::Node::Conditional {
            consequent,
            alternate,
            ..
        } => {
            summary.add(tree_summary(consequent));
            if let Some(alternate) = alternate {
                summary.add(tree_summary(alternate));
            }
        }
        ir::Node::ForEach { template, .. } => {
            if virtualisable_template(template) {
                summary.virtual_lists += 1;
            } else {
                summary.general_lists += 1;
            }
            summary.add(tree_summary(template));
        }
        ir::Node::Text { .. }
        | ir::Node::TextInput { .. }
        | ir::Node::Button { .. }
        | ir::Node::Field { .. }
        | ir::Node::Icon { .. }
        | ir::Node::Image { .. }
        | ir::Node::CameraPreview { .. }
        | ir::Node::Toggle { .. } => {}
        ir::Node::ScreenModule { .. } => {
            unreachable!("screen modules are resolved before inspection")
        }
    }
    summary
}

impl TreeSummary {
    fn add(&mut self, other: Self) {
        self.nodes += other.nodes;
        self.virtual_lists += other.virtual_lists;
        self.general_lists += other.general_lists;
    }
}

fn virtualisable_template(node: &ir::Node) -> bool {
    match node {
        ir::Node::Stack { children, .. } => children.iter().all(virtualisable_template),
        ir::Node::Text { .. }
        | ir::Node::TextInput { .. }
        | ir::Node::Button { .. }
        | ir::Node::Field { .. }
        | ir::Node::Icon { .. }
        | ir::Node::Image { .. }
        | ir::Node::Toggle { .. } => true,
        ir::Node::Screen { .. }
        | ir::Node::CameraPreview { .. }
        | ir::Node::Tabs { .. }
        | ir::Node::Navigator { .. }
        | ir::Node::Conditional { .. }
        | ir::Node::ForEach { .. } => false,
        ir::Node::ScreenModule { .. } => {
            unreachable!("screen modules are resolved before inspection")
        }
    }
}

fn summarise(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_insert(0_usize) += 1;
    }
    counts
        .into_iter()
        .map(|(value, count)| {
            if count == 1 {
                value
            } else {
                format!("{value} ×{count}")
            }
        })
        .collect()
}

pub fn compile(project: &Project) -> Result<Capabilities> {
    let generated = generate(project)?;
    remove_obsolete_output(&project.root().join(".ink/generated/app.rs"))?;
    remove_obsolete_output(&project.android_assets_path().join("ink-app-v1.bin"))?;
    write_if_changed(&project.app_definition_path(), &generated.bundle)?;
    write_if_changed(
        &project.capability_manifest_path(),
        &generated.capabilities.encode()?,
    )?;
    icon::generate(&project.config.name, &project.config.android_resources)?;
    Ok(generated.capabilities)
}

pub fn generate_icon(project: &Project) -> Result<()> {
    icon::generate(&project.config.name, &project.config.android_resources)
}

struct GeneratedApp {
    bundle: Vec<u8>,
    capabilities: Capabilities,
}

fn generate(project: &Project) -> Result<GeneratedApp> {
    let app = source::compile(project.root(), &project.config.source)?;
    write_background_registry(project, &app)?;
    Ok(GeneratedApp {
        capabilities: detect_capabilities(&app),
        bundle: bundle::generate(&app, project.root())?,
    })
}

fn detect_capabilities(app: &ir::App) -> Capabilities {
    let audio_playback = app
        .controllers
        .iter()
        .any(|controller| controller.kind == "player");
    let audio_detached = app.controllers.iter().any(|controller| {
        controller.kind == "player" && controller.config.contains("\"playback\":\"detached\"")
    });
    let photo_capture = app
        .controllers
        .iter()
        .any(|controller| controller.module == "camera" && controller.kind == "photo");
    let code_scanner = app
        .controllers
        .iter()
        .any(|controller| controller.module == "camera" && controller.kind == "scanner");
    let light_sdk_ringtone = app
        .controllers
        .iter()
        .any(|controller| controller.kind == "ringtone-installer");
    let light_sdk_push = app
        .controllers
        .iter()
        .any(|controller| controller.kind == "light-push");
    let mut capabilities = Capabilities::default();
    for extension in &app.extensions {
        for capability in module_schema::by_extension(*extension).capabilities {
            capabilities.insert(*capability);
        }
    }
    let mut include = |condition, capability| {
        if condition {
            capabilities.insert(capability);
        }
    };
    include(light_sdk_ringtone, Capability::LightSdkRingtone);
    include(light_sdk_push, Capability::LightSdkPush);
    include(
        uses_remote_image(&app.root) || audio_playback,
        Capability::Network,
    );
    include(uses_text_input(&app.root), Capability::TextInput);
    include(
        app.android_permissions
            .contains(&ir::AndroidPermission::Camera),
        Capability::CameraPermission,
    );
    include(photo_capture, Capability::PhotoCapture);
    include(code_scanner, Capability::CodeScanner);
    include(audio_playback, Capability::AudioPlayback);
    include(audio_detached, Capability::AudioDetached);
    include(
        app.android_permissions
            .contains(&ir::AndroidPermission::Microphone),
        Capability::MicrophonePermission,
    );
    include(
        app.android_permissions
            .contains(&ir::AndroidPermission::Notifications),
        Capability::NotificationPermission,
    );
    capabilities
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
        | ir::Node::Field { .. }
        | ir::Node::Icon { .. }
        | ir::Node::Image { .. }
        | ir::Node::CameraPreview { .. }
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
        | ir::Node::Field { .. }
        | ir::Node::Icon { .. }
        | ir::Node::Image { .. }
        | ir::Node::CameraPreview { .. }
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

fn remove_obsolete_output(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("could not remove {}", path.display())),
    }
}
