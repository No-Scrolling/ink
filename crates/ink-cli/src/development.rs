use crate::{
    android::{self, Device},
    output, watch,
};
use anyhow::{Context, Result, bail};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal,
};
use ink_compiler::Project;
use std::{
    fs,
    io::{self, IsTerminal},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub fn run(
    mut project: Project,
    requested: Option<&str>,
    once: bool,
    logs: bool,
    verbose: bool,
) -> Result<()> {
    let device = android::select_device(requested)?;
    println!("{} · dev", project.name());
    output::tree_root_field("Device", device.name(), false);
    watch::include_framework(android::project_framework_root(&project)?);
    let config = project.config_path().to_owned();
    let mut installed = None;
    let mut log_stream = None;
    loop {
        let before = watch::capture(project.root())?;
        let result = (|| -> Result<()> {
            let build_started = Instant::now();
            if let Err(error) = ink_compiler::compile_development(&project) {
                if installed.is_none() {
                    output::tree_root_field("Build", "failed", once);
                }
                return Err(error);
            }
            let fingerprint = native_fingerprint(&project)?;
            if installed.as_ref() != Some(&fingerprint) {
                if installed.is_some() {
                    println!("\n{} · rebuild", project.name());
                }
                let artifact = stage("Build", false, "ready", build_started, || {
                    android::build_precompiled_for_device(&project, &device, verbose)
                })?;
                stage("Install", false, "complete", Instant::now(), || {
                    android::install(&device, &artifact.apk, verbose)
                })?;
                stage("Open", once, "complete", Instant::now(), || {
                    android::launch(&device, &project, verbose)
                })?;
                installed = Some(fingerprint);
            } else {
                activate(&project, &device)?;
                output::tree_root_field(
                    "Reload",
                    format!("complete · {}", output::duration(build_started.elapsed())),
                    false,
                );
            }
            if logs && log_stream.is_none() {
                log_stream = Some(android::LogStream::start(
                    &device,
                    project.package(),
                    false,
                )?);
            }
            Ok(())
        })();
        if once {
            result?;
            return match log_stream {
                Some(mut stream) => stream.wait(),
                None => Ok(()),
            };
        }
        if let Err(error) = result {
            output::error(format!("{error:#}"));
        }
        // Refresh the resolved graph after compilation; an edit during the build still retries.
        let baseline = watch::capture(project.root())?;
        if !watch::inputs_changed(&before, &baseline)
            && wait_for_change(&project, &device, &baseline)? == WaitAction::Quit
        {
            return Ok(());
        }
        loop {
            match Project::load(&config) {
                Ok(next) => {
                    project = next;
                    break;
                }
                Err(error) => {
                    output::error(format!("{error:#}"));
                    let baseline = watch::capture(project.root())?;
                    if wait_for_change(&project, &device, &baseline)? == WaitAction::Quit {
                        return Ok(());
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WaitAction {
    Changed,
    Reload,
    Open,
    Quit,
}

struct RawInput;

impl RawInput {
    fn enable() -> Result<Option<Self>> {
        if !io::stdin().is_terminal() {
            return Ok(None);
        }
        terminal::enable_raw_mode()?;
        Ok(Some(Self))
    }
}

impl Drop for RawInput {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

fn wait_for_change(
    project: &Project,
    device: &Device,
    baseline: &watch::Snapshot,
) -> Result<WaitAction> {
    loop {
        let raw_input = RawInput::enable()?;
        let mut action = WaitAction::Changed;
        let result = watch::wait(project.root(), baseline, || {
            if raw_input.is_none() {
                return Ok(false);
            }
            while event::poll(Duration::ZERO)? {
                let Event::Key(key) = event::read()? else {
                    continue;
                };
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char(ch)
                        if ch.eq_ignore_ascii_case(&'c')
                            && key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        action = WaitAction::Quit;
                    }
                    KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'a') => {
                        action = WaitAction::Open;
                    }
                    KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'r') => {
                        action = WaitAction::Reload;
                    }
                    _ => continue,
                }
                return Ok(true);
            }
            Ok(false)
        });
        drop(raw_input);
        result?;
        match action {
            WaitAction::Open => {
                let started = Instant::now();
                match android::open(device, project) {
                    Ok(()) => output::tree_root_field(
                        "Open",
                        format!("complete · {}", output::duration(started.elapsed())),
                        false,
                    ),
                    Err(error) => output::error(format!("{error:#}")),
                }
            }
            WaitAction::Reload => {
                let started = Instant::now();
                match activate(project, device) {
                    Ok(()) => output::tree_root_field(
                        "Reload",
                        format!("complete · {}", output::duration(started.elapsed())),
                        false,
                    ),
                    Err(error) => output::error(format!("{error:#}")),
                }
            }
            WaitAction::Changed | WaitAction::Quit => return Ok(action),
        }
    }
}

fn stage<T>(
    label: &str,
    last: bool,
    success: &str,
    started: Instant,
    run: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let progress = output::tree_spinner(label, last);
    let result = run();
    progress.finish_and_clear();
    match result {
        Ok(value) => {
            output::tree_root_field(
                label,
                format!("{success} · {}", output::duration(started.elapsed())),
                last,
            );
            Ok(value)
        }
        Err(error) => {
            output::tree_root_field(label, "failed", last);
            Err(error)
        }
    }
}

fn native_fingerprint(project: &Project) -> Result<Vec<u8>> {
    let mut result = fs::read(project.config_path())?;
    result.extend(fs::read(project.capability_manifest_path())?);
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(
        project.android_assets_path().join("ink-bundle-v1.json"),
    )?)?;
    if let Some(hash) = manifest["devRuntimeHash"].as_str() {
        result.extend(hash.as_bytes());
    }

    let framework = android::project_framework_root(&project)?;
    for name in ["Cargo.toml", "Cargo.lock", "sdk.json"] {
        fingerprint_files(&framework.join(name), &mut result)?;
    }

    for path in [
        framework.join("platform/android"),
        framework.join("crates"),
        project.android_resources_path().to_path_buf(),
    ] {
        fingerprint_files(&path, &mut result)?;
    }
    Ok(result)
}

fn fingerprint_files(path: &Path, output: &mut Vec<u8>) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        let mut entries = fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        entries.sort();
        for entry in entries {
            if matches!(
                entry.file_name().and_then(|name| name.to_str()),
                Some("build" | "target" | ".gradle" | ".git")
            ) {
                continue;
            }
            fingerprint_files(&entry, output)?;
        }
    } else {
        output.extend(path.to_string_lossy().as_bytes());
        output.extend(fs::read(path)?);
    }
    Ok(())
}

fn activate(project: &Project, device: &Device) -> Result<()> {
    let generation = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string();
    let directory = format!("files/ink-dev/{generation}");
    // The package name comes from validated ink.toml; all other remote words are constants/numbers.
    let remote = format!(
        "run-as {} sh -c 'mkdir -p {directory} && cd {directory} && tar xf - && touch complete'",
        project.package()
    );
    let mut archive = Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .arg("-cf")
        .arg("-")
        .arg("-C")
        .arg(project.android_assets_path())
        .arg(".")
        .stdout(Stdio::piped())
        .spawn()
        .context("could not archive bundle")?;
    let status = Command::new("adb")
        .args(["-s", &device.serial, "shell", "-T", &remote])
        .stdin(
            archive
                .stdout
                .take()
                .context("bundle archive has no stdout")?,
        )
        .status()?;
    let archived = archive.wait()?;
    if !status.success() || !archived.success() {
        bail!("bundle transfer failed; current generation remains active");
    }
    let result = Command::new("adb")
        .args([
            "-s",
            &device.serial,
            "shell",
            "am",
            "start",
            "-n",
            &format!("{}/com.vandam.ink.MainActivity", project.package()),
            "--es",
            "ink.dev.generation",
            &generation,
        ])
        .output()?;
    if !result.status.success() {
        bail!(
            "could not activate development bundle: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }
    Ok(())
}
