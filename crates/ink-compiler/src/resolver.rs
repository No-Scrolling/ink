use std::path::{Path, PathBuf};

use oxc::span::Span;
use oxc_resolver::{ResolveOptions, Resolver};
use serde::Deserialize;

use crate::{diagnostic::CompileError, ir::Extension};

const LIGHT_SDK_PACKAGE: &str = "@ink/light-sdk";
const LIGHT_SDK_VERSION: &str = "0.1.1";
const NETWORK_PACKAGE: &str = "@ink/network";
const AUDIO_PACKAGE: &str = "@ink/audio";
const LOCATION_PACKAGE: &str = "@ink/location";

pub struct ModuleResolver {
    project_root: PathBuf,
    resolver: Resolver,
}

impl ModuleResolver {
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_owned(),
            resolver: Resolver::new(ResolveOptions {
                builtin_modules: true,
                condition_names: vec!["ink".into()],
                extensions: vec![".tsx".into(), ".ts".into()],
                node_path: false,
                symlinks: true,
                ..ResolveOptions::default()
            }),
        }
    }

    pub fn screen(
        &self,
        importer: &Path,
        specifier: &str,
        span: Span,
    ) -> Result<PathBuf, CompileError> {
        if is_unsupported_specifier(specifier) {
            return Err(CompileError::new(
                "screen imports must be relative paths or package exports",
                span,
            ));
        }

        let resolution = self
            .resolver
            .resolve_file(importer, specifier)
            .map_err(|error| {
                CompileError::new(
                    format!("could not resolve screen module {specifier:?}: {error}"),
                    span,
                )
            })?;
        let package_root = package_root(resolution.path());
        let path = resolution.into_path_buf();
        if path.extension().and_then(|extension| extension.to_str()) != Some("tsx") {
            return Err(CompileError::new("screen modules must be .tsx files", span));
        }

        if is_relative(specifier) {
            let owner = self
                .owner(importer)
                .ok_or_else(|| CompileError::new("could not find the importing package", span))?;
            if !path.starts_with(&owner) {
                return Err(CompileError::new(
                    "relative screen imports must stay inside their package",
                    span,
                ));
            }
        } else {
            let Some(package_root) = package_root else {
                return Err(CompileError::new(
                    "bare screen imports must resolve through a package export",
                    span,
                ));
            };
            if !path.starts_with(package_root) {
                return Err(CompileError::new(
                    "package exports must stay inside their package",
                    span,
                ));
            }
        }

        Ok(path)
    }

    pub fn extension(
        &self,
        importer: &Path,
        specifier: &str,
        span: Span,
    ) -> Result<Extension, CompileError> {
        if is_relative(specifier) || is_unsupported_specifier(specifier) {
            return Err(CompileError::new(
                "side-effect imports must name an installed Ink extension",
                span,
            ));
        }

        let resolution = self
            .resolver
            .resolve_file(importer, specifier)
            .map_err(|error| {
                CompileError::new(
                    format!("could not resolve Ink extension {specifier:?}: {error}"),
                    span,
                )
            })?;
        let package_root = package_root(resolution.path())
            .ok_or_else(|| CompileError::new("Ink extensions must be installed packages", span))?;
        if resolution
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("ts")
        {
            return Err(CompileError::new(
                "Ink extension entries must be .ts files",
                span,
            ));
        }
        if !resolution.path().starts_with(&package_root) {
            return Err(CompileError::new(
                "an Ink extension entry must stay inside its package",
                span,
            ));
        }

        let text = std::fs::read_to_string(package_root.join("package.json")).map_err(|error| {
            CompileError::new(
                format!("could not read extension package manifest: {error}"),
                span,
            )
        })?;
        let manifest: PackageManifest = serde_json::from_str(&text).map_err(|error| {
            CompileError::new(
                format!("could not parse extension package manifest: {error}"),
                span,
            )
        })?;
        let ink = manifest
            .ink
            .ok_or_else(|| CompileError::new("package is missing Ink extension metadata", span))?;

        match (manifest.name.as_str(), ink.extension.as_str()) {
            (LIGHT_SDK_PACKAGE, "light-sdk") if ink.sdk_version == LIGHT_SDK_VERSION => {
                Ok(Extension::LightSdk)
            }
            (LIGHT_SDK_PACKAGE, "light-sdk") => Err(CompileError::new(
                format!(
                    "@ink/light-sdk targets Light SDK {}, but this Ink version supports {LIGHT_SDK_VERSION}",
                    ink.sdk_version
                ),
                span,
            )),
            (NETWORK_PACKAGE, "network") if ink.sdk_version == "1" => Ok(Extension::Network),
            (NETWORK_PACKAGE, "network") => Err(CompileError::new(
                format!(
                    "@ink/network targets Ink network API {}, but this Ink version supports 1",
                    ink.sdk_version
                ),
                span,
            )),
            (AUDIO_PACKAGE, "audio") if ink.sdk_version == "1" => Ok(Extension::Audio),
            (AUDIO_PACKAGE, "audio") => Err(CompileError::new(
                format!(
                    "@ink/audio targets Ink audio API {}, but this Ink version supports 1",
                    ink.sdk_version
                ),
                span,
            )),
            (LOCATION_PACKAGE, "location") if ink.sdk_version == "1" => Ok(Extension::Location),
            (LOCATION_PACKAGE, "location") => Err(CompileError::new(
                format!(
                    "@ink/location targets Ink location API {}, but this Ink version supports 1",
                    ink.sdk_version
                ),
                span,
            )),
            _ => Err(CompileError::new(
                format!("{specifier:?} is not a supported Ink extension"),
                span,
            )),
        }
    }

    fn owner(&self, importer: &Path) -> Option<PathBuf> {
        package_root(importer).or_else(|| {
            importer
                .starts_with(&self.project_root)
                .then(|| self.project_root.clone())
        })
    }
}

#[derive(Deserialize)]
struct PackageManifest {
    name: String,
    ink: Option<ExtensionManifest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtensionManifest {
    extension: String,
    sdk_version: String,
}

fn is_relative(specifier: &str) -> bool {
    specifier.starts_with("./") || specifier.starts_with("../")
}

fn is_unsupported_specifier(specifier: &str) -> bool {
    specifier.is_empty()
        || Path::new(specifier).is_absolute()
        || specifier.starts_with("file:")
        || specifier.starts_with("node:")
        || specifier.contains("://")
        || specifier.contains('?')
        || specifier.contains('#')
}

fn package_root(importer: &Path) -> Option<PathBuf> {
    importer
        .parent()?
        .ancestors()
        .find(|directory| directory.join("package.json").is_file())
        .map(Path::to_owned)
}
