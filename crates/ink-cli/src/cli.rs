use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ink",
    version,
    about = "Build small Android apps with TypeScript and Rust",
    disable_help_subcommand = true,
    after_help = "Examples:\n  ink dev\n  ink build\n  ink -C examples/counter check"
)]
pub struct Cli {
    /// Run as if Ink was started in this directory
    #[arg(short = 'C', value_name = "DIR", global = true)]
    pub directory: Option<PathBuf>,

    /// Show Cargo, Gradle and ADB output
    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<InkCommand>,
}

#[derive(Subcommand)]
pub enum InkCommand {
    /// Create an app using the selected local SDK
    Create {
        directory: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = "com.example.inkapp")]
        package: String,
    },
    /// Format, lint and type-check the application
    Check,

    /// Lint the application without changing files
    Lint,

    /// Build an optimised, release-signed APK
    Build {
        /// Build a development APK instead
        #[arg(long)]
        debug: bool,
    },

    /// Build, install and launch on a connected Android device
    Dev {
        /// Select a device by its serial or a unique name
        #[arg(long, value_name = "DEVICE")]
        device: Option<String>,

        /// Build and launch once instead of watching for changes
        #[arg(long)]
        once: bool,

        /// Stream application and crash logs
        #[arg(long)]
        logs: bool,
    },

    /// List connected Android devices
    Devices,

    /// Stream logs from the installed application
    Logs {
        /// Select a device by its serial or a unique name
        #[arg(long, value_name = "DEVICE")]
        device: Option<String>,

        /// Show only native resource and action transitions
        #[arg(long)]
        resources: bool,
    },

    /// Show resolved application and build information
    Info,

    /// Check the local Ink and Android development environment
    Doctor,
}
