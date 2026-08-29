use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};

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
    Ok(Snapshot(files))
}

pub fn changed(root: &Path, baseline: &Snapshot) -> Result<bool> {
    Ok(&capture(root)? != baseline)
}

pub fn wait(root: &Path, baseline: &Snapshot) -> Result<Snapshot> {
    loop {
        thread::sleep(POLL_INTERVAL);
        let current = capture(root)?;
        if &current != baseline {
            return settle(root, current);
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
                relative.to_owned(),
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
    relative.components().next().is_some_and(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".ink" | "dist" | "node_modules")
        )
    })
}
