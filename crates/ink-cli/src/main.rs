mod android;
mod cli;
mod output;
mod process;
mod watch;

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};
use cli::{Cli, InkCommand};
use ink_compiler::Project;

fn main() {
    output::initialise();
    if let Err(error) = run() {
        output::error(format!("{error:#}"));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    };

    match command {
        InkCommand::Doctor => android::doctor(),
        InkCommand::Devices => list_devices(),
        InkCommand::Check => {
            let project = load_project(cli.directory.as_deref())?;
            ink_compiler::check(&project)?;
            output::success(format!("{} is valid", project.name()));
            Ok(())
        }
        InkCommand::Build { debug } => {
            let project = load_project(cli.directory.as_deref())?;
            let profile = if debug {
                android::Profile::Debug
            } else {
                android::Profile::Release
            };
            let artifact = android::build(&project, profile, cli.verbose)?;
            let output_path = android::copy_build(&project, profile, &artifact)?;
            output::success(format!(
                "Built {} {} in {} ({})",
                project.name(),
                project.version(),
                output::duration(artifact.duration),
                output_path.display()
            ));
            Ok(())
        }
        InkCommand::Dev { device, once, logs } => {
            let project = load_project(cli.directory.as_deref())?;
            develop(project, device.as_deref(), once, logs, cli.verbose)
        }
        InkCommand::Logs { device } => {
            let project = load_project(cli.directory.as_deref())?;
            let device = android::select_device(device.as_deref())?;
            output::info(format!("Using {}", device.description()));
            android::stream_logs(&device, project.package())
        }
        InkCommand::Info => {
            let project = load_project(cli.directory.as_deref())?;
            show_info(&project)
        }
    }
}

fn develop(
    project: Project,
    requested_device: Option<&str>,
    once: bool,
    logs: bool,
    verbose: bool,
) -> Result<()> {
    let device = android::select_device(requested_device)?;
    output::info(format!("Using {}", device.description()));
    let config_path = project.config_path().to_owned();
    let project_root = project.root().to_owned();
    let mut project = project;
    let mut baseline = watch::capture(&project_root)?;

    loop {
        let watched = (!once).then_some(&baseline);
        match develop_once(&project, &device, logs, verbose, watched) {
            Ok(DevOutcome::Changed) => {
                baseline = watch::settle(&project_root, watch::capture(&project_root)?)?;
                project = Project::load(&config_path)?;
                continue;
            }
            Ok(DevOutcome::Complete(log_stream)) if once => {
                if let Some(mut log_stream) = log_stream {
                    output::info(format!(
                        "Streaming logs for {}. Press Ctrl-C to stop.",
                        project.package()
                    ));
                    return log_stream.wait();
                }
                return Ok(());
            }
            Ok(DevOutcome::Complete(log_stream)) => {
                output::info("Watching for changes. Press Ctrl-C to stop.");
                baseline = watch::wait(&project_root, &baseline)?;
                drop(log_stream);
            }
            Err(error) if once => return Err(error),
            Err(error) => {
                output::error(format!("{error:#}"));
                baseline = watch::wait(&project_root, &baseline)?;
            }
        }
        output::info("Change detected, rebuilding");
        project = match Project::load(&config_path) {
            Ok(project) => project,
            Err(error) => {
                output::error(format!("{error:#}"));
                baseline = watch::wait(&project_root, &baseline)?;
                continue;
            }
        };
    }
}

enum DevOutcome {
    Complete(Option<android::LogStream>),
    Changed,
}

fn develop_once(
    project: &Project,
    device: &android::Device,
    logs: bool,
    verbose: bool,
    baseline: Option<&watch::Snapshot>,
) -> Result<DevOutcome> {
    let artifact = match baseline {
        Some(baseline) => {
            match android::build_watched(project, android::Profile::Debug, verbose, baseline)? {
                android::BuildOutcome::Complete(artifact) => artifact,
                android::BuildOutcome::Changed => return Ok(DevOutcome::Changed),
            }
        }
        None => android::build(project, android::Profile::Debug, verbose)?,
    };
    if let Some(baseline) = baseline
        && watch::changed(project.root(), baseline)?
    {
        output::info("Files changed before installation; rebuilding");
        return Ok(DevOutcome::Changed);
    }
    android::install(device, &artifact.apk, verbose)?;
    let log_stream = logs
        .then(|| android::LogStream::start(device, project.package()))
        .transpose()?;
    android::launch(device, project, verbose)?;
    output::success(format!("Launched {}", project.name()));
    Ok(DevOutcome::Complete(log_stream))
}

fn list_devices() -> Result<()> {
    let devices = android::connected_devices()?;
    let remembered = android::remembered_device();
    if devices.is_empty() {
        output::warning("No Android devices found");
        return Ok(());
    }
    for device in devices {
        let marker = if remembered.as_deref() == Some(&device.serial) {
            "*"
        } else {
            " "
        };
        println!("{marker} {}", device.description());
    }
    if remembered.is_some() {
        println!("\n* remembered device");
    }
    Ok(())
}

fn show_info(project: &Project) -> Result<()> {
    output::field("Application", project.name());
    output::field("Package", project.package());
    output::field(
        "Version",
        format!("{} ({})", project.version(), project.version_code()),
    );
    output::field("Source", project.source_path().display().to_string());
    output::field("Target", "Android arm64");
    output::field("Ink", env!("CARGO_PKG_VERSION"));
    output::field(
        "Signing",
        project
            .release_signing()
            .map(|signing| format!("{} ({})", signing.key_alias, signing.keystore.display()))
            .unwrap_or_else(|| "Not configured".to_owned()),
    );

    let devices = android::connected_devices()?;
    let remembered = android::remembered_device();
    let device = remembered
        .as_deref()
        .and_then(|serial| devices.iter().find(|device| device.serial == serial))
        .or_else(|| {
            let mut ready = devices.iter().filter(|device| device.ready());
            let first = ready.next()?;
            ready.next().is_none().then_some(first)
        });
    output::field(
        "Device",
        device
            .map(android::Device::description)
            .unwrap_or_else(|| "None selected".to_owned()),
    );

    let builds = build_summaries(&project.root().join("dist"))?;
    output::field(
        "Builds",
        if builds.is_empty() {
            "None".to_owned()
        } else {
            builds.join(", ")
        },
    );
    Ok(())
}

fn build_summaries(directory: &Path) -> Result<Vec<String>> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut builds = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("apk") {
            continue;
        }
        let size = entry.metadata()?.len();
        let file_name = path
            .file_name()
            .expect("directory entries have file names")
            .to_string_lossy();
        builds.push(format!("{file_name} ({})", file_size(size)));
    }
    builds.sort();
    Ok(builds)
}

fn file_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn load_project(directory: Option<&Path>) -> Result<Project> {
    let start = match directory {
        Some(directory) => fs::canonicalize(directory)
            .with_context(|| format!("could not find {}", directory.display()))?,
        None => std::env::current_dir().context("could not read the current directory")?,
    };

    for directory in start.ancestors() {
        let config = directory.join("ink.toml");
        if config.is_file() {
            return Project::load(config);
        }
    }

    bail!(
        "no ink.toml found from {}; run Ink inside an application or use -C <DIR>",
        start.display()
    )
}
