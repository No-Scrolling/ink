use std::fs;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use ink_compiler::Project;
use ink_core::{Colour, ThemeBundle};
use serde_json::{Value, json};

use crate::{export, output};

#[derive(Subcommand)]
pub enum ThemeCommand {
    /// Write theme.ts with every setting, commented out at its default
    Init,
    /// Build the theme, check its colour contrast and render every component
    Check {
        /// Screen width in pixels; the gallery frame is tall enough for every component
        #[arg(long, default_value = "1080x1240", value_name = "WIDTHxHEIGHT")]
        size: String,
        /// Simulate a display cutout, as X,Y,WIDTH,HEIGHT in pixels
        #[arg(long, value_name = "X,Y,WIDTH,HEIGHT")]
        cutout: Vec<String>,
        /// Print results as JSON
        #[arg(long)]
        json: bool,
    },
}

const TEMPLATE: &str = include_str!("theme-template.ts");
const GALLERY: &str = include_str!("theme-gallery.tsx");
// WCAG 2.2 minimum contrast for body text.
const MINIMUM_CONTRAST: f32 = 4.5;
const GALLERY_HEIGHT: u32 = 3200;

pub fn run(project: &Project, command: ThemeCommand) -> Result<()> {
    match command {
        ThemeCommand::Init => init(project),
        ThemeCommand::Check { size, cutout, json } => check(project, &size, &cutout, json),
    }
}

fn init(project: &Project) -> Result<()> {
    let path = project.root().join("theme.ts");
    if path.exists() {
        bail!("theme.ts already exists; edit it, or move it to start again");
    }
    fs::write(&path, TEMPLATE).with_context(|| format!("could not write {}", path.display()))?;
    println!("{} · theme", project.name());
    output::tree_root_field("Theme", path.display().to_string(), true);
    Ok(())
}

fn check(project: &Project, size: &str, cutouts: &[String], json: bool) -> Result<()> {
    let encoded = ink_compiler::compile_theme(project)?
        .context("this app has no theme; run `ink theme init` to create theme.ts")?;
    let theme = ThemeBundle::decode(&encoded)?.theme;

    // The app theme, then each variant, named as `<Screen theme>` would name it.
    let themes = std::iter::once((None, &theme))
        .chain(theme.variants.iter().map(|(name, variant)| (Some(name.as_str()), variant.as_ref())))
        .collect::<Vec<_>>();
    let mut problems = Vec::new();
    for (name, theme) in &themes {
        let label = name.map_or(String::new(), |name| format!("variant {name:?}: "));
        for (scheme, roles) in [("dark", theme.palette.dark), ("light", theme.palette.light)] {
            for (role, colour) in [("foreground", roles.foreground), ("muted", roles.muted)] {
                let ratio = contrast(colour, roles.background);
                if ratio < MINIMUM_CONTRAST {
                    problems.push(format!(
                        "{label}{scheme} {role} on background: contrast {ratio:.1}:1, below {MINIMUM_CONTRAST}:1"
                    ));
                }
            }
        }
    }

    // Render a screen of every themed component in both colour schemes, tall enough to show it all.
    let width = size.split_once(['x', '×']).map_or(size, |(width, _)| width);
    let size = format!("{width}x{GALLERY_HEIGHT}");
    let directory = project.root().join(".ink/theme");
    fs::create_dir_all(&directory)?;
    let mut frames = Vec::new();
    for (name, _) in &themes {
        let (gallery, folder) = match name {
            None => (GALLERY.to_owned(), String::new()),
            Some(name) => (GALLERY.replace("<Screen title=\"Theme\"", &format!("<Screen theme={name:?} title=\"Theme\"")), format!("{name}/")),
        };
        let file = directory.join(format!("gallery{}.tsx", name.map_or(String::new(), |name| format!("-{name}"))));
        fs::write(&file, gallery)?;
        for light in [false, true] {
            let scheme = if light { "light" } else { "dark" };
            let out = directory.join(format!("{folder}{scheme}"));
            let mut arguments = vec![
                "--entry".into(), file.display().to_string(),
                "--name".into(), format!("Theme · {}{scheme}", name.map_or(String::new(), |name| format!("{name} · "))),
                "--out".into(), out.display().to_string(),
                "--size".into(), size.clone(),
            ];
            if light {
                arguments.push("--light".into());
            }
            for cutout in cutouts {
                arguments.extend(["--cutout".into(), cutout.clone()]);
            }
            export::run(project, export::arguments(arguments)?)?;
            frames.push(out.join("frame.png"));
        }
    }

    if json {
        println!("{}", json!({
            "ok": problems.is_empty(),
            "problems": problems,
            "frames": frames.iter().map(|frame| Value::from(frame.display().to_string())).collect::<Vec<_>>(),
        }));
    } else {
        println!();
        println!("{} · theme check", project.name());
        for problem in &problems {
            output::tree_root_field("Problem", problem, false);
        }
        if problems.is_empty() {
            output::tree_root_field("Contrast", format!("at least {MINIMUM_CONTRAST}:1 in dark and light"), false);
        }
        output::tree_root_field("Frames", frames.iter().map(|frame| frame.display().to_string()).collect::<Vec<_>>().join("\n"), true);
    }
    if !problems.is_empty() {
        bail!("theme check found {} problem{}", problems.len(), if problems.len() == 1 { "" } else { "s" });
    }
    Ok(())
}

/// WCAG contrast ratio between two linear colours.
fn contrast(a: Colour, b: Colour) -> f32 {
    let luminance = |colour: Colour| 0.2126 * colour.red + 0.7152 * colour.green + 0.0722 * colour.blue;
    let (lighter, darker) = {
        let (a, b) = (luminance(a), luminance(b));
        if a > b { (a, b) } else { (b, a) }
    };
    (lighter + 0.05) / (darker + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_matches_the_wcag_reference_values() {
        assert!((contrast(Colour::WHITE, Colour::BLACK) - 21.0).abs() < 1e-4);
        assert!((contrast(Colour::BLACK, Colour::BLACK) - 1.0).abs() < 1e-4);
        // #767676 on white is WCAG's commonly cited 4.54:1.
        let grey = ink_core::parse_colour("#767676").unwrap();
        assert!((contrast(grey, Colour::WHITE) - 4.54).abs() < 0.01);
    }
}
