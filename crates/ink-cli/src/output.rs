use std::{env, fmt, io::IsTerminal, time::Duration};

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use owo_colors::{OwoColorize, Stream, set_override};

pub fn initialise() {
    let colour = std::io::stderr().is_terminal()
        && env::var_os("NO_COLOR").is_none()
        && env::var_os("CLICOLOR").is_none_or(|value| value != "0")
        && env::var_os("TERM").is_none_or(|value| value != "dumb");
    set_override(colour);
}

pub fn success(message: impl AsRef<str>) {
    println!(
        "{} {}",
        "✓".if_supports_color(Stream::Stdout, |text| text.green()),
        message.as_ref()
    );
}

pub fn warning(message: impl AsRef<str>) {
    eprintln!(
        "{} {}",
        "warn:".if_supports_color(Stream::Stderr, |text| text.yellow()),
        message.as_ref()
    );
}

pub fn error(message: impl AsRef<str>) {
    eprintln!(
        "{} {}",
        "error:".if_supports_color(Stream::Stderr, |text| text.red()),
        message.as_ref()
    );
}

#[derive(Debug)]
pub struct ReportedError;

impl fmt::Display for ReportedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("command failed")
    }
}

impl std::error::Error for ReportedError {}

pub fn tree_section(label: &str, last: bool) {
    println!("{}── {label}", if last { "└" } else { "├" });
}

pub fn tree_root_field(label: &str, value: impl AsRef<str>, last: bool) {
    let branch = if last { "└──" } else { "├──" };
    println!("{branch} {label:<12} {}", value.as_ref());
}

pub fn tree_field(parent_last: bool, last: bool, label: &str, value: impl AsRef<str>) {
    let prefix = if parent_last { "    " } else { "│   " };
    let branch = if last { "└──" } else { "├──" };
    println!("{prefix}{branch} {label:<13} {}", value.as_ref());
}

pub fn tree_step(label: &str, detail: &str, last: bool, passed: bool) {
    let branch = if last { "└──" } else { "├──" };
    if passed {
        println!(
            "{branch} {} {label:<12} {detail}",
            "✓".if_supports_color(Stream::Stdout, |text| text.green())
        );
    } else {
        eprintln!(
            "{branch} {} {label:<12} {detail}",
            "✗".if_supports_color(Stream::Stderr, |text| text.red())
        );
    }
}

pub fn tree_diagnostic(message: &str) {
    for line in message.trim_matches('\n').lines() {
        eprintln!("    {line}");
    }
}

pub fn duration(duration: Duration) -> String {
    if duration.as_secs() >= 60 {
        format!(
            "{}m {:.1}s",
            duration.as_secs() / 60,
            duration.as_secs_f64() % 60.0
        )
    } else if duration.as_secs() > 0 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

pub fn tree_spinner(label: &str, last: bool) -> ProgressBar {
    let branch = if last { "└──" } else { "├──" };
    let progress = if std::io::stderr().is_terminal() && env::var_os("CI").is_none() {
        ProgressBar::new_spinner()
    } else {
        ProgressBar::with_draw_target(None, ProgressDrawTarget::hidden())
    };
    progress.set_style(
        ProgressStyle::with_template("{msg} {spinner:.cyan}")
            .expect("the spinner template is valid")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    progress.set_message(format!("{branch} {label}"));
    progress.enable_steady_tick(Duration::from_millis(80));
    progress
}
