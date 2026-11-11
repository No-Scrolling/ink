use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result, bail};
use ink_core::{
    ThemeSource,
    theme_source::{SourceColours, SourceFonts, SourceHeader, SourceImage, SourceRoles, SourceRow, SourceScrollBar, SourceSubtitle, SourceSection, SourceSpacing, SourceText, SourceToggle},
};
use serde::Deserialize;

use crate::{Project, write_if_changed};

/// What `theme-evaluate.js` prints.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Evaluated {
    fonts: BTreeMap<String, PathBuf>,
    colours: EvaluatedColours,
    text: EvaluatedText,
    spacing: SourceSpacing,
    header: SourceHeader,
    section: SourceSection,
    scroll_bar: SourceScrollBar,
    subtitle: SourceSubtitle,
    #[serde(default)]
    row: SourceRow,
    toggle: Option<EvaluatedToggle>,
    selection: Option<String>,
    field: Option<String>,
    text_input: ink_core::InputAppearance,
    hit_slop: ink_core::HitInsets,
    #[serde(default)]
    variants: BTreeMap<String, Evaluated>,
}

#[derive(Deserialize)]
struct EvaluatedColours {
    dark: SourceRoles,
    light: Option<SourceRoles>,
}

#[derive(Deserialize)]
struct EvaluatedText {
    size: Option<f32>,
}

#[derive(Deserialize)]
struct EvaluatedToggle {
    style: String,
    width: Option<f32>,
    height: Option<f32>,
    svgs: Option<[[String; 4]; 2]>,
}

/// Evaluate the app's theme, returning the encoded theme file and its variant names, or `None`
/// without a theme.
pub(crate) fn compile(project: &Project) -> Result<Option<(Vec<u8>, Vec<String>)>> {
    let Some(path) = project.config.theme.clone() else {
        return Ok(None);
    };
    let script = project.work_path().join("theme-evaluate.js");
    write_if_changed(&script, include_bytes!("theme-evaluate.js"))?;
    let output = Command::new("bun")
        .current_dir(project.root())
        .arg("--no-env-file")
        .arg(&script)
        .arg(&path)
        .output()
        .context("could not run Bun; install Bun and the application's dependencies")?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let mut evaluated: Evaluated = serde_json::from_slice(&output.stdout).context("could not read the evaluated theme")?;

    let mut blobs = Blobs::default();
    let variants = std::mem::take(&mut evaluated.variants);
    let names = variants.keys().cloned().collect();
    let mut source = blobs.source(evaluated)?;
    for (name, variant) in variants {
        source.variants.insert(name, blobs.source(variant)?);
    }
    let blobs = blobs.data;
    let encoded = source.encode(&blobs)?;
    // Decode once so invalid values fail the build rather than the app.
    ink_core::ThemeBundle::decode(&encoded).context("theme.ts")?;
    Ok(Some((encoded, names)))
}

/// Check each literal `theme="…"` in the app's sources names a variant from theme.ts, so a typo
/// fails the build instead of silently using the app theme.
pub(crate) fn check_screen_themes(project: &Project, manifest: &[u8], variants: &[String]) -> Result<()> {
    let manifest: serde_json::Value = serde_json::from_slice(manifest)?;
    let mut problems = Vec::new();
    for input in manifest["inputs"].as_array().into_iter().flatten().filter_map(serde_json::Value::as_str) {
        let path = std::path::Path::new(input);
        let source = path.starts_with(project.root())
            && !path.components().any(|part| matches!(part.as_os_str().to_str(), Some("node_modules" | ".ink")))
            && matches!(path.extension().and_then(|extension| extension.to_str()), Some("tsx" | "ts" | "jsx" | "js"));
        if !source {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else { continue };
        for (number, line) in text.lines().enumerate() {
            for (offset, _) in line.match_indices("theme=") {
                let rest = line[offset + 6..].trim_start_matches('{');
                let Some(quote) = rest.chars().next().filter(|quote| matches!(quote, '"' | '\'' | '`')) else { continue };
                let Some(name) = rest[1..].split(quote).next() else { continue };
                if !variants.iter().any(|variant| variant == name) {
                    let file = path.strip_prefix(project.root()).unwrap_or(path).display();
                    problems.push(if variants.is_empty() {
                        format!("{file}:{}: theme {name:?} is not defined; add it to `variants` in theme.ts", number + 1)
                    } else {
                        format!("{file}:{}: unknown theme {name:?}; theme.ts defines {}", number + 1,
                            variants.iter().map(|variant| format!("{variant:?}")).collect::<Vec<_>>().join(", "))
                    });
                }
            }
        }
    }
    if !problems.is_empty() {
        bail!("{}", problems.join("\n"));
    }
    Ok(())
}

/// Font files and control images, shared between the theme and its variants.
#[derive(Default)]
struct Blobs {
    data: Vec<Vec<u8>>,
    fonts: HashMap<PathBuf, usize>,
    images: HashMap<(String, u32, u32), SourceImage>,
}

impl Blobs {
    fn push(&mut self, bytes: Vec<u8>) -> usize {
        self.data.push(bytes);
        self.data.len() - 1
    }

    fn font(&mut self, role: &str, path: &PathBuf) -> Result<usize> {
        if let Some(&index) = self.fonts.get(path) {
            return Ok(index);
        }
        let bytes = std::fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
        ab_glyph::FontRef::try_from_slice(&bytes).map_err(|_| anyhow::anyhow!("theme.ts: fonts.{role} is not a valid TrueType or OpenType font"))?;
        let index = self.push(bytes);
        self.fonts.insert(path.clone(), index);
        Ok(index)
    }

    fn image(&mut self, svg: &str, width: f32, height: f32) -> Result<SourceImage> {
        let key = (svg.to_owned(), width.to_bits(), height.to_bits());
        if let Some(image) = self.images.get(&key) {
            return Ok(SourceImage { blob: image.blob, width: image.width, height: image.height });
        }
        let (pixels, width, height) = raster(svg, width, height)?;
        let image = SourceImage { blob: self.push(pixels), width, height };
        self.images.insert(key, SourceImage { blob: image.blob, width, height });
        Ok(image)
    }

    fn source(&mut self, evaluated: Evaluated) -> Result<ThemeSource> {
        let mut fonts = SourceFonts::default();
        for (role, path) in &evaluated.fonts {
            let index = Some(self.font(role, path)?);
            match role.as_str() {
                "body" => fonts.body = index,
                "display" => fonts.display = index,
                "input" => fonts.input = index,
                _ => bail!("unknown font role {role}"),
            }
        }
        let toggle = match evaluated.toggle {
            None => None,
            Some(EvaluatedToggle { style, width, height, svgs }) => {
                let images = match (&svgs, width, height) {
                    (Some(svgs), Some(width), Some(height)) => {
                        let mut scheme = |svgs: &[String; 4]| -> Result<[SourceImage; 4]> {
                            Ok([
                                self.image(&svgs[0], width, height)?,
                                self.image(&svgs[1], width, height)?,
                                self.image(&svgs[2], width, height)?,
                                self.image(&svgs[3], width, height)?,
                            ])
                        };
                        Some([scheme(&svgs[0])?, scheme(&svgs[1])?])
                    }
                    _ => None,
                };
                Some(SourceToggle { style, width, height, images })
            }
        };
        Ok(ThemeSource {
            colours: Some(SourceColours { dark: Some(evaluated.colours.dark), light: evaluated.colours.light }),
            fonts,
            text: SourceText { size: evaluated.text.size },
            spacing: evaluated.spacing,
            header: evaluated.header,
            toggle,
            selection: evaluated.selection,
            field: evaluated.field,
            text_input: evaluated.text_input,
            hit_slop: evaluated.hit_slop,
            section: evaluated.section,
            scroll_bar: evaluated.scroll_bar,
            subtitle: evaluated.subtitle,
            row: evaluated.row,
            variants: BTreeMap::new(),
        })
    }
}

/// Rasterise a custom control at twice its pixel size, so it stays sharp on screens up to 2160
/// pixels wide, into zlib-compressed straight RGBA.
fn raster(svg: &str, width: f32, height: f32) -> Result<(Vec<u8>, u32, u32)> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
        .context("theme.ts: toggle.render returned invalid SVG")?;
    let pixel_width = (width * 2.0).round().max(1.0) as u32;
    let pixel_height = (height * 2.0).round().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixel_width, pixel_height).context("could not allocate the toggle image")?;
    let scale_x = pixel_width as f32 / tree.size().width();
    let scale_y = pixel_height as f32 / tree.size().height();
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale_x, scale_y), &mut pixmap.as_mut());
    let pixels: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let pixel = pixel.demultiply();
            [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
        })
        .collect();
    Ok((miniz_oxide::deflate::compress_to_vec_zlib(&pixels, 6), pixel_width, pixel_height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(source: &str) -> (Project, Vec<u8>, PathBuf) {
        let root = std::env::temp_dir().join(format!("ink-theme-check-{}-{}", std::process::id(), source.len()));
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::write(root.join("ink.toml"), "name = \"Check\"\npackage = \"dev.ink.check\"\n").unwrap();
        let screen = root.join("app/reader.tsx");
        std::fs::write(&screen, source).unwrap();
        let project = Project::load(root.join("ink.toml")).unwrap();
        let screen = screen.canonicalize().unwrap();
        let manifest = serde_json::to_vec(&serde_json::json!({ "inputs": [screen] })).unwrap();
        (project, manifest, root)
    }

    #[test]
    fn screen_themes_must_name_a_variant() {
        let (project, manifest, root) = project("<Screen title=\"Chapter\" theme=\"raeder\">\n");
        let error = check_screen_themes(&project, &manifest, &["reader".into()]).unwrap_err().to_string();
        assert_eq!(error, "app/reader.tsx:1: unknown theme \"raeder\"; theme.ts defines \"reader\"");
        assert!(check_screen_themes(&project, &manifest, &[]).unwrap_err().to_string().contains("add it to `variants`"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn defined_and_dynamic_screen_themes_pass() {
        let (project, manifest, root) = project("<Screen theme={\"reader\"} />\n<Screen theme={chosen} />\n");
        check_screen_themes(&project, &manifest, &["reader".into()]).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
