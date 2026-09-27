use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};

static FRAMEWORK: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub fn include_framework(root: PathBuf) {
    let _ = FRAMEWORK.set(root);
}

const POLL_INTERVAL: Duration = Duration::from_millis(75);
const DEBOUNCE: Duration = Duration::from_millis(225);

#[derive(Clone, PartialEq, Eq)]
pub struct Snapshot(BTreeMap<PathBuf, FileState>);

#[derive(Clone, PartialEq, Eq)]
struct FileState {
    modified: SystemTime,
    length: u64,
}

pub fn capture(root: &Path) -> Result<Snapshot> {
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    if let Some(framework) = FRAMEWORK.get() {
        for name in [
            "Cargo.toml",
            "Cargo.lock",
            "sdk.json",
            "package.json",
            "bun.lock",
        ] {
            let path = framework.join(name);
            if let Ok(metadata) = fs::metadata(&path) {
                files.insert(
                    path,
                    FileState {
                        modified: metadata.modified()?,
                        length: metadata.len(),
                    },
                );
            }
        }

        for directory in [framework.join("platform/android"), framework.join("crates")] {
            visit(&directory, &directory, &mut files)?;
        }
    }
    let manifest = root.join(".ink/android/assets/ink-bundle-v1.json");
    if let Ok(bytes) = fs::read(manifest)
        && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && let Some(inputs) = value["inputs"].as_array()
    {
        for input in inputs.iter().filter_map(serde_json::Value::as_str) {
            let path = PathBuf::from(input);
            if path.components().any(|part| part.as_os_str() == ".ink") {
                continue;
            }
            if let Some(parent) = path.parent()
                && let Ok(metadata) = fs::metadata(parent)
            {
                files.insert(
                    parent.to_owned(),
                    FileState {
                        modified: metadata.modified()?,
                        length: metadata.len(),
                    },
                );
            }
            if let Ok(metadata) = fs::metadata(&path) {
                files.insert(
                    path,
                    FileState {
                        modified: metadata.modified()?,
                        length: metadata.len(),
                    },
                );
            }
        }
    }
    Ok(Snapshot(files))
}

pub fn inputs_changed(before: &Snapshot, after: &Snapshot) -> bool {
    before.0.iter().any(|(path, state)| {
        !path.is_dir()
            && match after.0.get(path) {
                Some(current) => current != state,
                None => fs::metadata(path).map_or(true, |metadata| {
                    metadata.len() != state.length
                        || metadata.modified().ok() != Some(state.modified)
                }),
            }
    })
}

pub fn wait(
    root: &Path,
    baseline: &Snapshot,
    mut on_tick: impl FnMut() -> Result<bool>,
) -> Result<()> {
    loop {
        thread::sleep(POLL_INTERVAL);
        if on_tick()? {
            return Ok(());
        }
        let current = capture(root)?;
        if &current != baseline {
            settle(root, current)?;
            return Ok(());
        }
    }
}

pub fn settle(root: &Path, mut current: Snapshot) -> Result<Snapshot> {
    let mut quiet = Duration::ZERO;
    while quiet < DEBOUNCE {
        thread::sleep(POLL_INTERVAL);
        let next = capture(root)?;
        if next == current {
            quiet += POLL_INTERVAL;
        } else {
            current = next;
            quiet = Duration::ZERO;
        }
    }
    Ok(current)
}

fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, FileState>) -> Result<()> {
    let entries = fs::read_dir(directory)
        .with_context(|| format!("could not read {}", directory.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("could not read {}", directory.display()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .expect("walked paths stay under the project root");
        if ignored(relative) {
            continue;
        }
        let metadata = entry
            .metadata()
            .with_context(|| format!("could not inspect {}", path.display()))?;
        if metadata.is_dir() {
            visit(root, &path, files)?;
        } else if metadata.is_file() {
            files.insert(
                path.to_owned(),
                FileState {
                    modified: metadata
                        .modified()
                        .with_context(|| format!("could not inspect {}", path.display()))?,
                    length: metadata.len(),
                },
            );
        }
    }
    Ok(())
}

fn ignored(relative: &Path) -> bool {
    relative.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".ink" | ".git" | "target" | "build" | ".gradle" | "dist" | "node_modules")
        )
    })
}
