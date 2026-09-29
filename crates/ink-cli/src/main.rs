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
        if !error.is::<output::ReportedError>() {
            output::error(format!("{error:#}"));
        }
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
            quality::check(&project, cli.verbose)
        }
        InkCommand::Format => {
            let project = load_project(cli.directory.as_deref())?;
            quality::format(&project, cli.verbose)
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
            let output_path = match android::copy_build(&project, profile, &artifact) {
                Ok(path) => path,
                Err(error) => {
                    output::tree_root_field("APK", "failed", true);
                    return Err(error);
                }
            };
            let size = fs::metadata(&output_path)?.len();
            let relative = output_path
                .strip_prefix(project.root())
                .expect("build output is inside the project");
            output::tree_root_field(
                "APK",
                format!("{} · {}", relative.display(), file_size(size)),
                true,
            );
            Ok(())
        }
        InkCommand::Dev { device, once, logs } => {
            let project = load_project(cli.directory.as_deref())?;
            develop(project, device.as_deref(), once, logs, cli.verbose)
        }
        InkCommand::Logs { device, resources } => {
            let project = load_project(cli.directory.as_deref())?;
            let device = android::select_device(device.as_deref())?;
            println!("{} · logs · {}\n", project.name(), device.name());
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
    println!("Android devices");
    if devices.is_empty() {
        output::tree_section("None connected", true);
        return Ok(());
    }
    let count = devices.len();
    for (index, device) in devices.into_iter().enumerate() {
        let name = if device.serial.starts_with("emulator-") {
            "Emulator".to_owned()
        } else {
            device.name()
        };
        let is_remembered = remembered.as_deref() == Some(device.serial.as_str());
        let is_ready = device.ready();
        let mut detail = device.serial;
        if !is_ready {
            detail.push_str(&format!(" · {}", device.state));
        }
        if is_remembered {
            detail.push_str(" · remembered");
        }
        output::tree_root_field(&name, detail, index + 1 == count);
    }
    Ok(())
}

fn show_info(project: &Project) -> Result<()> {
    let app = ink_compiler::inspect(project)?;
    let builds = build_summaries(&project.root().join("dist"))?;
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

    println!(
        "{} · {} ({})",
        project.name(),
        project.version(),
        project.version_code()
    );
    output::tree_root_field("Package", project.package(), false);
    output::tree_root_field("Ink", env!("CARGO_PKG_VERSION"), false);
    output::tree_section("Build", false);
    output::tree_field(
        false,
        false,
        "JavaScript",
        file_size(app.javascript_bytes as u64),
    );
    if let Some(signing) = project.release_signing() {
        output::tree_field(false, false, "Signing", &signing.key_alias);
    }
    let debug_suffix = format!("-{}-arm64-debug.apk", project.version());
    let release_suffix = format!("-{}-arm64.apk", project.version());
    let current_builds: Vec<_> = builds
        .iter()
        .filter(|build| {
            build.name.ends_with(&debug_suffix) || build.name.ends_with(&release_suffix)
        })
        .collect();
    if current_builds.is_empty() {
        output::tree_field(false, true, "APKs", "None");
    } else {
        for (index, build) in current_builds.iter().enumerate() {
            let label = if build.name.ends_with(&debug_suffix) {
                "Debug APK"
            } else {
                "Release APK"
            };
            output::tree_field(
                false,
                index + 1 == current_builds.len(),
                label,
                file_size(build.size),
            );
        }
    }
    let capabilities = app.capabilities.join(", ");
    let capabilities = if capabilities.is_empty() {
        "None".to_owned()
    } else if capabilities.len() > 60 {
        format!("{} enabled", app.capabilities.len())
    } else {
        capabilities
    };
    output::tree_root_field("Capabilities", capabilities, false);
    let device = device
        .map(|device| {
            let remembered = if remembered.as_deref() == Some(&device.serial) {
                " · remembered"
            } else {
                ""
            };
            format!("{}{}", device.name(), remembered)
        })
        .unwrap_or_else(|| "None selected".to_owned());
    output::tree_root_field("Device", device, true);
    Ok(())
}

struct BuildSummary {
    name: String,
    size: u64,
}

fn build_summaries(directory: &Path) -> Result<Vec<BuildSummary>> {
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
        builds.push(BuildSummary {
            name: file_name.into_owned(),
            size,
        });
    }
    builds.sort_by(|left, right| left.name.cmp(&right.name));
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
