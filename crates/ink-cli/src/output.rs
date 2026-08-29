use std::{env, io::IsTerminal, time::Duration};

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use owo_colors::{OwoColorize, Stream, set_override};

pub fn initialise() {
    let colour = std::io::stderr().is_terminal()
        && env::var_os("NO_COLOR").is_none()
        && env::var_os("CLICOLOR").is_none_or(|value| value != "0")
        && env::var_os("TERM").is_none_or(|value| value != "dumb");
    set_override(colour);
}

pub fn info(message: impl AsRef<str>) {
    println!(
        "{} {}",
        "→".if_supports_color(Stream::Stdout, |text| text.cyan()),
        message.as_ref()
    );
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

pub fn field(label: &str, value: impl AsRef<str>) {
    println!("{label:<12} {}", value.as_ref());
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

pub fn spinner(message: &str) -> ProgressBar {
    let progress = if std::io::stderr().is_terminal() && env::var_os("CI").is_none() {
        ProgressBar::new_spinner()
    } else {
        ProgressBar::with_draw_target(None, ProgressDrawTarget::hidden())
    };
    progress.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .expect("the spinner template is valid")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    progress.set_message(message.to_owned());
    progress.enable_steady_tick(Duration::from_millis(80));
    progress
}
