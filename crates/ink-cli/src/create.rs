use std::{fs, path::Path};

use anyhow::{Result, ensure};
use serde_json::json;

pub fn create(directory: &Path, name: Option<&str>, package: &str) -> Result<()> {
    ensure!(
        !directory.exists(),
        "destination already exists: {}",
        directory.display()
    );
    let sdk = super::android::framework_root()?;
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
    let mut overrides = serde_json::Map::new();
    for entry in fs::read_dir(sdk.join("packages"))? {
        let path = entry?.path();
        let manifest = path.join("package.json");
        if !manifest.is_file() {
            continue;
        }
        let package: serde_json::Value = serde_json::from_slice(&fs::read(manifest)?)?;
        if let Some(name) = package["name"].as_str() {
            overrides.insert(name.to_owned(), json!(format!("file:{}", path.display())));
        }
    }
    let metadata = json!({
        "name": package.replace('.', "-"),
        "private": true,
        "overrides": overrides,
        "scripts": { "check": "tsc --noEmit" },
        "dependencies": {
            "ink": format!("file:{}", sdk.join("packages/ink").display()),
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
        "import { Screen, Text } from \"ink\";\n\nexport default function App() {\n  return <Screen title=\"Home\"><Text>Welcome to Ink!</Text></Screen>;\n}\n",
    )?;
    fs::write(
        directory.join(".gitignore"),
        "node_modules/\n.ink/\ndist/\n*.jks\n*.keystore\n",
    )?;
    super::output::success(format!(
        "Created {}. Run bun install in {}",
        title,
        directory.display()
    ));
    Ok(())
}
