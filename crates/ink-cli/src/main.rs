mod android;
mod development;
mod cli;
mod create;
mod output;
mod process;
mod quality;
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
        InkCommand::Create { directory, name, package } => {
            let directory = cli.directory.as_deref().unwrap_or(Path::new(".")).join(directory);
            create::create(&directory, name.as_deref(), &package)
        }
        InkCommand::Doctor => android::doctor(),
        InkCommand::Devices => list_devices(),
        InkCommand::Check => {
            let project = load_project(cli.directory.as_deref())?;
            quality::format(&project, cli.verbose)?;
            quality::lint(&project, cli.verbose)?;
            ink_compiler::check(&project)?;
            output::success(format!("{} is valid", project.name()));
            Ok(())
        }
        InkCommand::Lint => {
            let project = load_project(cli.directory.as_deref())?;
            quality::lint(&project, cli.verbose)?;
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
        InkCommand::Logs { device, resources } => {
            let project = load_project(cli.directory.as_deref())?;
            let device = android::select_device(device.as_deref())?;
            output::info(format!("Using {}", device.description()));
            android::stream_logs(&device, project.package(), resources)
        }
        InkCommand::Info => {
            let project = load_project(cli.directory.as_deref())?;
            show_info(&project)
        }
    }
}

fn develop(project: Project, requested_device: Option<&str>, once: bool, logs: bool, verbose: bool) -> Result<()> {
    development::run(project, requested_device, once, logs, verbose)
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
    let app = ink_compiler::inspect(project)?;
    output::field("Application", project.name());
    output::field("Package", project.package());
    output::field(
        "Version",
        format!("{} ({})", project.version(), project.version_code()),
    );
    output::field("Source", project.source_path().display().to_string());
    output::field("Light server", project.light_server());
    output::field("Target", "Android arm64");
    output::field("Ink", env!("CARGO_PKG_VERSION"));
    output::field("Runtime", "React / QuickJS-ng");
    output::field(
        "JavaScript",
        format!("{} bytes (minified)", app.javascript_bytes),
    );
    output::field("Resolved inputs", app.resolved_inputs.to_string());
    output::field("Imported media", app.asset_count.to_string());
    output::field("Icon variants", app.icon_variants.to_string());
    output::field(
        "Capabilities",
        if app.capabilities.is_empty() {
            "None".to_owned()
        } else {
            app.capabilities.join(", ")
        },
    );
    for detail in &app.capability_details {
        output::field("", detail);
    }
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
