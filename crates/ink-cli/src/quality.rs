use std::{path::PathBuf, process::Command};

use anyhow::Result;
use ink_compiler::Project;

use crate::{android, process};

const SOURCES: &str = "**/*.{js,jsx,ts,tsx,mjs,cjs,mts,cts}";

fn tool(name: &str) -> Result<PathBuf> {
    let path = android::framework_root()?.join("node_modules/.bin").join(name);
    if !path.is_file() {
        anyhow::bail!(
            "{name} is not installed in the Ink SDK; run `bun install` in the SDK checkout"
        );
    }
    Ok(path)
}

pub fn format(project: &Project, verbose: bool) -> Result<()> {
    let mut command = Command::new("bun");
    command
        .arg(tool("oxfmt")?)
        .current_dir(project.root())
        .args(["--write", "--no-error-on-unmatched-pattern", SOURCES])
        .args(["!**/.ink/**", "!**/dist/**", "!**/node_modules/**"]);
    process::run(&mut command, "Formatting source", verbose)?;
    Ok(())
}

pub fn lint(project: &Project, verbose: bool) -> Result<()> {
    let config = android::framework_root()?.join("oxlint.config.mjs");
    anyhow::ensure!(
        config.is_file(),
        "Ink lint configuration is missing from the SDK"
    );
    let mut command = Command::new("bun");
    command
        .arg(tool("oxlint")?)
        .current_dir(project.root())
        .args(["--deny-warnings", "--report-unused-disable-directives", "--config"])
        .arg(config)
        .args([
            "--ignore-pattern", ".ink/**",
            "--ignore-pattern", "**/.ink/**",
            "--ignore-pattern", "dist/**",
            "--ignore-pattern", "**/dist/**",
            "--ignore-pattern", "node_modules/**",
            "--ignore-pattern", "**/node_modules/**",
            ".",
        ]);
    process::run(&mut command, "Linting source", verbose)?;
    Ok(())
}
