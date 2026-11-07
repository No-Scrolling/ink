use std::{fs, path::Path};

use anyhow::Result;

const INSTRUCTIONS: &str = include_str!("app-agents.md");
const DESIGN_GUIDE: &str = "node_modules/ink/DESIGN.md";

/// Adds Ink's agent instructions to AGENTS.md, keeping any existing instructions.
pub fn write(root: &Path) -> Result<bool> {
    let path = root.join("AGENTS.md");
    let existing = if path.exists() { fs::read_to_string(&path)? } else { String::new() };
    if existing.contains(DESIGN_GUIDE) {
        return Ok(false);
    }
    let contents = if existing.trim().is_empty() {
        format!("# Agent instructions\n\n{INSTRUCTIONS}")
    } else {
        format!("{}\n\n{INSTRUCTIONS}", existing.trim_end())
    };
    fs::write(path, contents)?;
    Ok(true)
}

pub fn setup(root: &Path) -> Result<()> {
    let added = write(root)?;
    println!("Ink · setup agents");
    crate::output::tree_step(
        "AGENTS.md",
        if added { "design guide added" } else { "already up to date" },
        true,
        true,
    );
    Ok(())
}
