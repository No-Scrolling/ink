use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use semver::Version;
use sha2::{Digest, Sha256};
use toml::{Table, Value};

#[derive(Debug)]
pub struct Build {
    pub manifest: PathBuf,
    pub library: String,
}

/// App code stays in its own crate; only the final cdylib is generated.
pub fn prepare(app: &Path, sdk: &Path) -> Result<Build> {
    let manifest = app.join("android/rust/Cargo.toml");
    let app_crate = manifest
        .is_file()
        .then(|| read_app_crate(app, &manifest))
        .transpose()?;
    let app_root = app_crate
        .as_ref()
        .map(|(_, workspace)| read(&workspace.join("Cargo.toml")))
        .transpose()?;
    let sdk_root = read(&sdk.join("Cargo.toml"))?;
    let host = read(&sdk.join("platform/android/native/Cargo.toml"))?;
    let directory = app.join(".ink/android/rust");
    fs::create_dir_all(&directory)?;
    let mut root: Table = toml::from_str(
        r#"
[package]
name = "ink-app-runtime"
version = "0.0.0"
edition = "2024"
[lib]
crate-type = ["cdylib"]
path = "lib.rs"
[workspace]
resolver = "3"
"#,
    )?;
    let mut deps = Table::new();
    deps.insert(
        "ink_host".into(),
        dependency("ink-android", &sdk.join("platform/android/native")),
    );
    if let Some((package, _)) = &app_crate {
        deps.insert(
            "ink_app".into(),
            dependency(
                package["package"]["name"]
                    .as_str()
                    .context("app Rust package needs a name")?,
                manifest.parent().unwrap(),
            ),
        );
    }
    root.insert("dependencies".into(), Value::Table(deps));
    root.insert(
        "features".into(),
        Value::Table(
            host["features"]
                .as_table()
                .unwrap()
                .keys()
                .map(|name| {
                    (
                        name.clone(),
                        Value::Array(vec![Value::String(format!("ink_host/{name}"))]),
                    )
                })
                .collect(),
        ),
    );
    let mut profiles = sdk_root
        .get("profile")
        .cloned()
        .unwrap_or(Value::Table(Table::new()));
    if let Some(app_profiles) = app_root.as_ref().and_then(|root| root.get("profile")) {
        merge(&mut profiles, app_profiles);
    }
    root.insert("profile".into(), profiles);
    let mut patches = Table::new();
    let mut sources = vec![(&sdk_root, sdk)];
    if let (Some(app_root), Some((_, workspace))) = (&app_root, &app_crate) {
        sources.push((app_root, workspace.as_path()));
    }
    for (source, base) in sources {
        if let Some(groups) = source.get("patch").and_then(Value::as_table) {
            for (registry, entries) in groups {
                let group = patches
                    .entry(registry.clone())
                    .or_insert_with(|| Value::Table(Table::new()))
                    .as_table_mut()
                    .unwrap();
                for (name, value) in entries.as_table().context("invalid Cargo patch table")? {
                    let mut value = value.clone();
                    if let Some(path) = value.get("path").and_then(Value::as_str) {
                        let absolute = base
                            .join(path)
                            .canonicalize()
                            .with_context(|| format!("invalid patch path for {name}"))?;
                        value.as_table_mut().unwrap().insert(
                            "path".into(),
                            Value::String(absolute.to_string_lossy().into_owned()),
                        );
                    }
                    if group.get(name).is_some_and(|existing| existing != &value) {
                        bail!(
                            "Ink and app Rust workspaces provide conflicting patches for {name}; use the same patch before building a combined native library"
                        );
                    }
                    group.insert(name.clone(), value);
                }
            }
        }
    }
    root.insert("patch".into(), Value::Table(patches));
    let sdk_lock = fs::read_to_string(sdk.join("Cargo.lock"))?;
    let app_lock = app_crate
        .as_ref()
        .and_then(|(_, workspace)| fs::read_to_string(workspace.join("Cargo.lock")).ok())
        .unwrap_or_default();
    // Cargo does not hash cdylib filenames. Separate app/configuration outputs
    // prevent a fresh cached build from reusing another configuration's binary.
    // Ordinary rlib dependencies still share Cargo's hashed SDK cache.
    let mut identity = Sha256::new();
    identity.update(app.to_string_lossy().as_bytes());
    identity.update(toml::to_string(&root)?);
    identity.update(&sdk_lock);
    identity.update(&app_lock);
    identity.update(toml::to_string(&sdk_root)?);
    if let Some((package, _)) = &app_crate {
        identity.update(toml::to_string(package)?);
    }
    let capabilities = app.join(".ink/android/assets/ink-capabilities-v1.json");
    if capabilities.is_file() {
        identity.update(fs::read(capabilities)?);
    }
    for name in [
        "INK_BENCHMARK",
        "INK_BRIDGE_TIMING",
        "INK_PRESENTATION_TIMING",
        "INK_MEMORY_DIAGNOSTICS",
        "INK_BENCHMARK_REVISION",
        "CFLAGS_aarch64-linux-android",
    ] {
        identity.update(name);
        identity.update(std::env::var(name).unwrap_or_default());
    }
    let library = format!("ink_app_{}", &format!("{:x}", identity.finalize())[..16]);
    for table in ["package", "lib"] {
        root.get_mut(table)
            .unwrap()
            .as_table_mut()
            .unwrap()
            .insert("name".into(), Value::String(library.clone()));
    }
    let destination = directory.join("Cargo.toml");
    let contents = toml::to_string_pretty(&root)?;
    let changed = fs::read_to_string(&destination).ok().as_deref() != Some(&contents);
    if changed {
        fs::write(&destination, contents)?;
    }
    let source_path = directory.join("lib.rs");
    let source = if app_crate.is_some() {
        "pub extern crate ink_host;\npub extern crate ink_app;\n"
    } else {
        "pub extern crate ink_host;\n"
    };
    if fs::read_to_string(&source_path).ok().as_deref() != Some(source) {
        fs::write(source_path, source)?;
    }
    let lock = directory.join("Cargo.lock");
    let seeds = format!("{sdk_lock}\n{app_lock}");
    let seed_path = directory.join("lock-inputs");
    if changed || !lock.is_file() || fs::read_to_string(&seed_path).ok().as_deref() != Some(&seeds)
    {
        fs::write(&lock, merge_locks(&sdk_lock, &app_lock)?)?;
    }
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--filter-platform",
            "aarch64-linux-android",
            "--manifest-path",
        ])
        .arg(&destination)
        .output()
        .context("could not resolve combined Rust dependencies")?;
    if !output.status.success() {
        bail!(
            "could not unify app and Ink Rust dependencies: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::write(seed_path, seeds)?;
    Ok(Build {
        manifest: destination,
        library,
    })
}

fn read_app_crate(app: &Path, manifest: &Path) -> Result<(Table, PathBuf)> {
    let package = read(&manifest)?;
    let types = package
        .get("lib")
        .and_then(|v| v.get("crate-type"))
        .and_then(Value::as_array);
    if types.is_some_and(|types| {
        types.is_empty()
            || types
                .iter()
                .any(|v| !matches!(v.as_str(), Some("rlib" | "lib")))
    }) {
        bail!(
            "android/rust must provide an rlib: set [lib].crate-type = [\"rlib\"] in {}. Ink links app JNI exports into libink_android.so; use System.loadLibrary(\"ink_android\") and remove the app's separate cargo/jniLibs Gradle task (see docs/architecture.md).",
            manifest.display()
        );
    }
    let name = package["package"]["name"]
        .as_str()
        .context("app Rust package needs a name")?;
    let library = package
        .get("lib")
        .and_then(|v| v.get("name"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| name.replace('-', "_"));
    if library != "ink_android" {
        check_loaders(&app.join("android/kotlin"), &library)?;
    }
    let metadata = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .context("could not inspect app Rust workspace")?;
    if !metadata.status.success() {
        bail!(
            "could not inspect app Rust workspace: {}",
            String::from_utf8_lossy(&metadata.stderr)
        );
    }
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout)?;
    let app_workspace = Path::new(
        metadata["workspace_root"]
            .as_str()
            .context("missing app Rust workspace")?,
    );
    Ok((package, app_workspace.to_path_buf()))
}

type PackageGroup = (String, String, String);

fn package_group(package: &Value) -> Result<PackageGroup> {
    let name = package["name"]
        .as_str()
        .context("missing locked package name")?;
    let version: Version = package["version"]
        .as_str()
        .context("missing locked version")?
        .parse()?;
    let compatible = if !version.pre.is_empty() {
        version.to_string()
    } else if version.major > 0 {
        format!("{}.*", version.major)
    } else if version.minor > 0 {
        format!("0.{}.*", version.minor)
    } else {
        version.to_string()
    };
    Ok((
        name.into(),
        package
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("")
            .into(),
        compatible,
    ))
}

/// Retain app-only pins (notably build tools) while sharing the newer compatible
/// version of dependencies already locked by either workspace. Cargo validates
/// the result against both manifests before Gradle's locked build.
fn merge_locks(sdk: &str, app: &str) -> Result<String> {
    let mut locks = vec![toml::from_str::<Table>(sdk)?];
    if !app.is_empty() {
        locks.push(toml::from_str(app)?);
    }
    let mut selected: BTreeMap<PackageGroup, Value> = BTreeMap::new();
    for lock in &locks {
        for package in lock["package"]
            .as_array()
            .context("missing locked packages")?
        {
            let group = package_group(package)?;
            let newer = selected.get(&group).is_none_or(|old| {
                Version::parse(package["version"].as_str().unwrap()).unwrap()
                    > Version::parse(old["version"].as_str().unwrap()).unwrap()
            });
            if newer {
                selected.insert(group, package.clone());
            }
        }
    }
    let mut dependencies: BTreeMap<PackageGroup, BTreeSet<String>> = BTreeMap::new();
    for lock in &locks {
        let packages = lock["package"].as_array().unwrap();
        for package in packages {
            let group = package_group(package)?;
            for dependency in package
                .get("dependencies")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let parts: Vec<_> = dependency
                    .as_str()
                    .context("invalid locked dependency")?
                    .split_whitespace()
                    .collect();
                let original = packages
                    .iter()
                    .find(|candidate| {
                        candidate["name"].as_str() == Some(parts[0])
                            && parts.get(1).is_none_or(|version| {
                                candidate["version"].as_str() == Some(*version)
                            })
                            && parts.get(2).is_none_or(|source| {
                                candidate.get("source").and_then(Value::as_str)
                                    == Some(source.trim_matches(['(', ')']))
                            })
                    })
                    .context("locked dependency is missing from its workspace")?;
                let target = &selected[&package_group(original)?];
                let source = target
                    .get("source")
                    .and_then(Value::as_str)
                    .map(|source| format!(" ({source})"))
                    .unwrap_or_default();
                dependencies
                    .entry(group.clone())
                    .or_default()
                    .insert(format!(
                        "{} {}{source}",
                        target["name"].as_str().unwrap(),
                        target["version"].as_str().unwrap()
                    ));
            }
        }
    }
    for (group, package) in &mut selected {
        let table = package.as_table_mut().unwrap();
        if let Some(dependencies) = dependencies.remove(group) {
            table.insert(
                "dependencies".into(),
                Value::Array(dependencies.into_iter().map(Value::String).collect()),
            );
        } else {
            table.remove("dependencies");
        }
    }
    let output = Table::from_iter([
        ("version".into(), Value::Integer(4)),
        (
            "package".into(),
            Value::Array(selected.into_values().collect()),
        ),
    ]);
    Ok(toml::to_string(&output)?)
}

fn dependency(package: &str, path: &Path) -> Value {
    Value::Table(Table::from_iter([
        ("package".into(), Value::String(package.into())),
        (
            "path".into(),
            Value::String(path.to_string_lossy().into_owned()),
        ),
    ]))
}

fn read(path: &Path) -> Result<Table> {
    toml::from_str(&fs::read_to_string(path)?)
        .with_context(|| format!("invalid Cargo manifest: {}", path.display()))
}

fn check_loaders(directory: &Path, library: &str) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            check_loaders(&path, library)?;
        } else if matches!(
            path.extension().and_then(|v| v.to_str()),
            Some("kt" | "java")
        ) {
            let source: String = fs::read_to_string(&path)?
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if source.contains(&format!("loadLibrary(\"{library}\")")) {
                bail!(
                    "{} loads {library}, which is now linked into libink_android.so. Replace it with System.loadLibrary(\"ink_android\") and remove the separate app Rust cargo/jniLibs Gradle task (see docs/architecture.md).",
                    path.display()
                );
            }
        }
    }
    Ok(())
}

fn merge(target: &mut Value, source: &Value) {
    if let (Some(target), Some(source)) = (target.as_table_mut(), source.as_table()) {
        for (key, value) in source {
            if let Some(existing) = target.get_mut(key) {
                merge(existing, value);
            } else {
                target.insert(key.clone(), value.clone());
            }
        }
    } else {
        *target = source.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct App(PathBuf);
    impl App {
        fn new(case: &str, types: &str) -> Self {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let path = root.join(format!(
                ".test-output/rust-packaging-{case}-{}",
                std::process::id()
            ));
            fs::create_dir_all(path.join("android/rust/src")).unwrap();
            fs::create_dir_all(path.join("android/kotlin")).unwrap();
            fs::write(path.join("android/rust/Cargo.toml"), format!(
                "[package]\nname = \"legacy_audio\"\nversion = \"0.0.0\"\n[lib]\ncrate-type = {types}\n[workspace]\n"
            )).unwrap();
            fs::write(path.join("android/rust/src/lib.rs"), "").unwrap();
            Self(path)
        }
    }
    impl Drop for App {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn separate_cdylibs_fail_before_building_with_actionable_migration_guidance() {
        let app = App::new("cdylib", "[\"cdylib\"]");
        let error = prepare(&app.0, Path::new("unused-sdk"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("crate-type = [\"rlib\"]"));
        assert!(error.contains("System.loadLibrary(\"ink_android\")"));
        assert!(error.contains("remove the app's separate cargo/jniLibs Gradle task"));
    }

    #[test]
    fn legacy_loader_is_rejected_before_it_can_become_a_device_link_error() {
        let app = App::new("loader", "[\"rlib\"]");
        fs::write(
            app.0.join("android/kotlin/Legacy.kt"),
            "object Audio { init { System.loadLibrary ( \"legacy_audio\" ) } }",
        )
        .unwrap();
        let error = prepare(&app.0, Path::new("unused-sdk"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("Legacy.kt loads legacy_audio"));
        assert!(error.contains("Replace it with System.loadLibrary(\"ink_android\")"));
    }

    #[test]
    fn apps_and_capability_configurations_have_distinct_stable_cargo_outputs() {
        let first = App::new("first", "[\"rlib\"]");
        let second = App::new("second", "[\"rlib\"]");
        let sdk = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let first_build = prepare(&first.0, &sdk).unwrap();
        assert_eq!(
            first_build.library,
            prepare(&first.0, &sdk).unwrap().library
        );
        assert_ne!(
            first_build.library,
            prepare(&second.0, &sdk).unwrap().library
        );
        fs::create_dir_all(first.0.join(".ink/android/assets")).unwrap();
        fs::write(
            first.0.join(".ink/android/assets/ink-capabilities-v1.json"),
            "{\"version\":1,\"capabilities\":[\"network\"]}",
        )
        .unwrap();
        assert_ne!(
            first_build.library,
            prepare(&first.0, &sdk).unwrap().library
        );
    }
}
