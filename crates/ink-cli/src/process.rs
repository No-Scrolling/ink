use std::{
    fs::{self, File},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};

use crate::{output, watch};

pub enum PhaseOutcome {
    Complete(Duration),
    Changed,
}

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

pub fn run_cancellable(
    command: &mut Command,
    message: &str,
    verbose: bool,
    project_root: &Path,
    baseline: &watch::Snapshot,
) -> Result<PhaseOutcome> {
    let started = Instant::now();
    let state_directory = project_root.join(".ink");
    fs::create_dir_all(&state_directory)?;
    let log_path = state_directory.join("build.log");

    if verbose {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
    } else {
        let log = File::create(&log_path)
            .with_context(|| format!("could not create {}", log_path.display()))?;
        command
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log);
    }

    let progress = (!verbose).then(|| output::spinner(message));
    let mut child = command
        .spawn()
        .with_context(|| format!("could not start {}", program_name(command)))?;
    loop {
        if let Some(status) = child.try_wait()? {
            if let Some(progress) = progress {
                progress.finish_and_clear();
            }
            if !status.success() {
                if !verbose && let Ok(contents) = fs::read(&log_path) {
                    print_bytes(&contents);
                }
                bail!("{message} failed with {status}");
            }
            let elapsed = started.elapsed();
            output::success(format!("{message} in {}", output::duration(elapsed)));
            return Ok(PhaseOutcome::Complete(elapsed));
        }
        if watch::changed(project_root, baseline)? {
            child.kill().ok();
            child.wait().ok();
            if let Some(progress) = progress {
                progress.finish_and_clear();
            }
            output::info("Files changed during the build; restarting");
            return Ok(PhaseOutcome::Changed);
        }
        thread::sleep(Duration::from_millis(75));
    }
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
