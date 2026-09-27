use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub fn run_quiet(command: &mut Command, message: &str, verbose: bool) -> Result<()> {
    if verbose {
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
        let result = command
            .output()
            .with_context(|| format!("could not start {}", program_name(command)))?;
        if !result.status.success() {
            print_bytes(&result.stdout);
            print_bytes(&result.stderr);
            bail!("{message} failed with {}", result.status);
        }
    }
    Ok(())
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
