use std::{path::PathBuf, process::Command, time::Instant};

use anyhow::Result;
use ink_compiler::Project;

use crate::{android, output};

const SOURCES: &str = "**/*.{js,jsx,ts,tsx,mjs,cjs,mts,cts}";

fn tool(name: &str) -> Result<PathBuf> {
    let path = android::framework_root()?
        .join("node_modules/.bin")
        .join(name);
    if !path.is_file() {
        anyhow::bail!(
            "{name} is not installed in the Ink SDK; run `bun install` in the SDK checkout"
        );
    }
    Ok(path)
}

fn format_command(project: &Project, write: bool) -> Result<Command> {
    let mut command = Command::new("bun");
    command
        .arg(tool("oxfmt")?)
        .current_dir(project.root())
        .args([
            if write { "--write" } else { "--check" },
            "--no-error-on-unmatched-pattern",
            SOURCES,
        ])
        .args(["!**/.ink/**", "!**/dist/**", "!**/node_modules/**"]);
    Ok(command)
}

fn lint_command(project: &Project) -> Result<Command> {
    let config = android::framework_root()?.join("oxlint.config.mjs");
    anyhow::ensure!(
        config.is_file(),
        "Ink lint configuration is missing from the SDK"
    );
    let mut command = Command::new("bun");
    command
        .arg(tool("oxlint")?)
        .current_dir(project.root())
        .args([
            "--deny-warnings",
            "--report-unused-disable-directives",
            "--format=stylish",
            "--config",
        ])
        .arg(config)
        .args([
            "--ignore-pattern",
            ".ink/**",
            "--ignore-pattern",
            "**/.ink/**",
            "--ignore-pattern",
            "dist/**",
            "--ignore-pattern",
            "**/dist/**",
            "--ignore-pattern",
            "node_modules/**",
            "--ignore-pattern",
            "**/node_modules/**",
            ".",
        ]);
    Ok(command)
}

pub fn lint(project: &Project, verbose: bool) -> Result<()> {
    println!("{} · lint", project.name());
    run_step(lint_command(project), "Lint", "no issues", verbose, true)
}

pub fn format(project: &Project, verbose: bool) -> Result<()> {
    println!("{} · format", project.name());
    run_step(format_command(project, true), "Format", "done", verbose, true)
}

pub fn check(project: &Project, verbose: bool) -> Result<()> {
    println!("{} · check", project.name());
    let started = Instant::now();
    run_step(format_command(project, false), "Format", "passed", verbose, false)?;
    run_step(lint_command(project), "Lint", "no issues", verbose, false)?;

    let progress = output::tree_spinner("Compile", true);
    let compiled = ink_compiler::check(project);
    progress.finish_and_clear();
    match compiled {
        Ok(()) => {
            output::tree_step("Compile", "passed", true, true);
            output::tree_field(true, false, "Bundle", "passed");
            output::tree_field(true, true, "TypeScript", "no errors");
        }
        Err(error) => {
            output::tree_step("Compile", "failed", true, false);
            output::tree_diagnostic(&format!("{error:#}"));
            return Err(output::ReportedError.into());
        }
    }

    println!("\nChecked in {}", output::duration(started.elapsed()));
    Ok(())
}

fn run_step(
    command: Result<Command>,
    label: &str,
    success: &str,
    verbose: bool,
    last: bool,
) -> Result<()> {
    let mut command = match command {
        Ok(command) => command,
        Err(error) => {
            output::tree_step(label, "failed", true, false);
            output::tree_diagnostic(&format!("{error:#}"));
            return Err(output::ReportedError.into());
        }
    };
    let started = Instant::now();
    let progress = output::tree_spinner(label, last);
    let result = command.output();
    progress.finish_and_clear();
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            output::tree_step(label, "failed", true, false);
            output::tree_diagnostic(&format!(
                "Could not start {}: {error}",
                command.get_program().to_string_lossy()
            ));
            return Err(output::ReportedError.into());
        }
    };

    if !result.status.success() {
        output::tree_step(label, "failed", true, false);
        show_output(&result.stdout);
        show_output(&result.stderr);
        return Err(output::ReportedError.into());
    }

    if verbose {
        show_output(&result.stdout);
        show_output(&result.stderr);
    }
    let elapsed = started.elapsed();
    let detail = format!("{success} · {}", output::duration(elapsed));
    if last {
        output::tree_root_field(label, detail, true);
    } else {
        output::tree_step(label, &detail, false, true);
    }
    Ok(())
}

fn show_output(bytes: &[u8]) {
    let output = String::from_utf8_lossy(bytes);
    if !output.trim().is_empty() {
        output::tree_diagnostic(&output);
    }
}
