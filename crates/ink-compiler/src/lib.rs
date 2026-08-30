mod codegen;
mod config;
mod diagnostic;
mod icon;
mod icons;
mod ir;
mod lower;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
pub use config::ReleaseSigning;
use config::ResolvedConfig;
use oxc::{allocator::Allocator, parser::Parser, semantic::SemanticBuilder, span::SourceType};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppFeatures {
    pub text_input: bool,
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
    let source = std::fs::read_to_string(&project.config.source)
        .with_context(|| format!("could not read {}", project.config.source.display()))?;
    let source_type = SourceType::from_path(&project.config.source).with_context(|| {
        format!(
            "{} is not a TypeScript/TSX source path",
            project.config.source.display()
        )
    })?;
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, source_type).parse();

    if !parsed.diagnostics.is_empty() {
        let diagnostics = parsed
            .diagnostics
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::bail!("{}:\n{diagnostics}", project.config.source.display());
    }

    let semantic = SemanticBuilder::new_compiler().build(&parsed.program);
    if !semantic.diagnostics.is_empty() {
        let diagnostics = semantic
            .diagnostics
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::bail!("{}:\n{diagnostics}", project.config.source.display());
    }

    let app = lower::lower(&parsed.program)
        .map_err(|error| anyhow::anyhow!(error.render(&project.config.source, &source)))?;
    Ok(GeneratedApp {
        features: AppFeatures {
            text_input: uses_text_input(&app.root),
        },
        source: codegen::generate(&app, project.root())?,
    })
}

fn uses_text_input(node: &ir::Node) -> bool {
    match node {
        ir::Node::TextInput { .. } => true,
        ir::Node::Screen { children, .. } | ir::Node::Stack { children, .. } => {
            children.iter().any(uses_text_input)
        }
        ir::Node::Tabs { tabs, .. } => tabs.iter().any(|tab| uses_text_input(&tab.screen)),
        ir::Node::Navigator { routes } => routes.iter().any(|route| uses_text_input(&route.screen)),
        ir::Node::Text { .. }
        | ir::Node::Button { .. }
        | ir::Node::Icon { .. }
        | ir::Node::Image { .. }
        | ir::Node::Toggle { .. } => false,
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
