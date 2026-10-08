use std::{collections::{BTreeMap, BTreeSet}, fs, path::Path};

use anyhow::{Context, Result, ensure};
use serde_json::json;

pub fn create(directory: &Path, name: Option<&str>, package: &str) -> Result<()> {
    ensure!(
        !directory.exists(),
        "destination already exists: {}",
        directory.display()
    );
    let sdk = super::android::framework_root()?;
    let installed = super::distribution::installed_root().is_some();
    let title = name.unwrap_or("My Ink App");
    ensure!(
        package.split('.').count() >= 2
            && package.split('.').all(|part| {
                !part.is_empty()
                    && part.chars().enumerate().all(|(index, c)| {
                        c.is_ascii_alphabetic() || c == '_' || (index > 0 && c.is_ascii_digit())
                    })
            }),
        "package must be an Android application identifier such as com.example.myapp"
    );
    fs::create_dir_all(directory)?;
    fs::write(
        directory.join("ink.toml"),
        format!(
            "name = {}\npackage = {}\nversion = \"0.1.0\"\nversion_code = 1\n",
            serde_json::to_string(title)?,
            serde_json::to_string(package)?
        ),
    )?;
    let dependency = if installed {
        format!("npm:ink-framework@{}", env!("CARGO_PKG_VERSION"))
    } else {
        format!("file:{}", sdk.join("packages/ink").display())
    };
    let metadata = json!({
        "name": package.replace('.', "-"),
        "private": true,
        "scripts": { "dev": "ink dev", "build": "ink build", "check": "ink check" },
        "dependencies": {
            "ink": dependency,
            "react": "19.2.8"
        },
        "devDependencies": { "typescript": "5.9.3", "@types/react": "19.2.18" }
    });
    fs::write(
        directory.join("package.json"),
        serde_json::to_string_pretty(&metadata)?,
    )?;
    fs::write(
        directory.join("tsconfig.json"),
        "{\n  \"extends\": \"ink/tsconfig\",\n  \"include\": [\"**/*.ts\", \"**/*.tsx\"]\n}\n",
    )?;
    fs::create_dir_all(directory.join("app"))?;
    fs::write(
        directory.join("app/index.tsx"),
        "import { Screen, Text } from \"ink\";\n\nexport default function App() {\n  return (\n    <Screen title=\"Home\">\n      <Text>Welcome to Ink!</Text>\n    </Screen>\n  );\n}\n",
    )?;
    fs::write(
        directory.join(".gitignore"),
        "node_modules/\n.ink/\ndist/\n*.jks\n*.keystore\n.env.local\n",
    )?;
    if installed {
        super::distribution::prepare(directory)?;
    }
    super::output::success(if installed {
        format!("Created {}. Run ink dev in {}", title, directory.display())
    } else {
        format!("Created {}. Run bun install in {}", title, directory.display())
    });
    Ok(())
}

pub fn add_modules(project: &ink_compiler::Project, modules: &[String]) -> Result<()> {
    if super::distribution::installed_root().is_some() {
        return super::distribution::run("add", Some(project.root()), modules);
    }
    let sdk = super::android::project_framework_root(project)?;
    let mut packages = BTreeMap::new();
    for entry in fs::read_dir(sdk.join("packages"))? {
        let directory = entry?.path();
        let manifest = directory.join("package.json");
        if !manifest.is_file() {
            continue;
        }
        let metadata: serde_json::Value = serde_json::from_slice(&fs::read(manifest)?)?;
        if let Some(name) = metadata["name"].as_str() {
            packages.insert(name.to_owned(), (directory, metadata));
        }
    }
    for name in modules {
        ensure!(
            name.starts_with("@ink/") && packages.contains_key(name),
            "unknown Ink module {name}; use a package name such as @ink/audio"
        );
    }
    let manifest = project.root().join("package.json");
    let mut metadata: serde_json::Value = serde_json::from_slice(&fs::read(&manifest)?)?;
    let mut pending = modules.to_vec();
    if let Some(dependencies) = metadata["dependencies"].as_object() {
        pending.extend(
            dependencies
                .keys()
                .filter(|name| name.starts_with("@ink/"))
                .cloned(),
        );
    }
    let mut resolved = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !resolved.insert(name.clone()) {
            continue;
        }
        let (_, package) = packages
            .get(&name)
            .with_context(|| format!("SDK has no package {name}"))?;
        for kind in ["dependencies", "peerDependencies"] {
            if let Some(dependencies) = package[kind].as_object() {
                pending.extend(
                    dependencies
                        .keys()
                        .filter(|name| name.as_str() == "ink" || name.starts_with("@ink/"))
                        .cloned(),
                );
            }
        }
    }
    let object = metadata
        .as_object_mut()
        .context("package.json must contain an object")?;
    let dependencies = object
        .entry("dependencies")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("dependencies must be an object")?;
    for name in modules {
        dependencies.insert(
            name.clone(),
            json!(format!("file:{}", packages[name].0.display())),
        );
    }
    let overrides = object
        .entry("overrides")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("overrides must be an object")?;
    for name in resolved {
        overrides.insert(
            name.clone(),
            json!(format!("file:{}", packages[&name].0.display())),
        );
    }
    fs::write(
        &manifest,
        format!("{}\n", serde_json::to_string_pretty(&metadata)?),
    )?;
    let status = std::process::Command::new("bun")
        .arg("install")
        .current_dir(project.root())
        .status()?;
    ensure!(
        status.success(),
        "dependencies were updated; bun install failed, fix the reported problem and retry bun install"
    );
    super::output::success(format!("Added {}", modules.join(", ")));
    Ok(())
}
