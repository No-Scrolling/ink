use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};

use crate::output;

pub fn run(command: &mut Command, message: &str, verbose: bool) -> Result<Duration> {
    let started = Instant::now();
    if verbose {
        output::info(message);
        let status = command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("could not start {}", program_name(command)))?;
        if !status.success() {
            bail!("{message} failed with {status}");
        }
    } else {
        let progress = output::spinner(message);
        let result = command
            .output()
            .with_context(|| format!("could not start {}", program_name(command)));
        progress.finish_and_clear();
        let result = result?;
        if !result.status.success() {
            print_bytes(&result.stdout);
            print_bytes(&result.stderr);
            bail!("{message} failed with {}", result.status);
        }
    }
    let elapsed = started.elapsed();
    output::success(format!("{message} in {}", output::duration(elapsed)));
    Ok(elapsed)
}

fn print_bytes(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    if !text.trim().is_empty() {
        eprintln!("{}", text.trim_end());
    }
}

fn program_name(command: &Command) -> String {
    command.get_program().to_string_lossy().into_owned()
}
