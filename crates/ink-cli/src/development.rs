use crate::{
    android::{self, Device},
    output, watch,
};
use anyhow::{Context, Result, bail};
use ink_compiler::Project;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn run(
    mut project: Project,
    requested: Option<&str>,
    once: bool,
    logs: bool,
    verbose: bool,
) -> Result<()> {
    let device = android::select_device(requested)?;
    output::info(format!("Using {}", device.description()));
    watch::include_framework(android::framework_root()?);
    let config = project.config_path().to_owned();
    let mut installed = None;
    let mut log_stream = None;
    loop {
        let before = watch::capture(project.root())?;
        let result = (|| -> Result<()> {
            ink_compiler::compile_development(&project)?;
            let fingerprint = native_fingerprint(&project)?;
            if installed.as_ref() != Some(&fingerprint) {
                let artifact =
                    android::build_for_device(&project, &device, android::Profile::Debug, verbose)?;
                android::install(&device, &artifact.apk, verbose)?;
                android::launch(&device, &project, verbose)?;
                installed = Some(fingerprint);
            } else {
                activate(&project, &device)?;
                output::success("Reloaded JavaScript and assets");
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
        if !watch::inputs_changed(&before, &baseline) {
            output::info("Watching resolved inputs. Press Ctrl-C to stop.");
            watch::wait(project.root(), &baseline)?;
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
                    watch::wait(project.root(), &baseline)?;
                }
            }
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

    let framework = android::framework_root()?;
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
    let status = Command::new("adb")
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
        .status()?;
    if !status.success() {
        bail!("could not activate development bundle");
    }
    Ok(())
}
