use std::{
    fs,
    path::{Path, PathBuf},
};

use ab_glyph::{Font, FontArc, ScaleFont, point};
use anyhow::{Context, Result, ensure};
use image::{
    Rgba, RgbaImage,
    imageops::{FilterType, overlay, resize},
};
use ink_compiler::Project;

const WIDTH: u32 = 2572;
const HEIGHT: u32 = 1048;
const BACKGROUND: Rgba<u8> = Rgba([57, 57, 57, 255]);
const FONT: &[u8] = include_bytes!("../../../assets/fonts/PublicSans-Regular.ttf");

pub fn generate(project: &Project, screenshots: &[PathBuf], out: &Path) -> Result<()> {
    let defaults = ['a', 'b', 'c', 'd'].map(|letter| PathBuf::from(format!("assets/{letter}.png")));
    let paths = if screenshots.is_empty() {
        &defaults
    } else {
        screenshots
    };
    let output = project.root().join(out);
    compose(project.name(), project.root(), paths, &output)?;
    println!("{} · header", project.name());
    crate::output::tree_root_field(
        "PNG",
        format!("{} · {WIDTH} × {HEIGHT}", out.display()),
        true,
    );
    Ok(())
}

pub fn create_default(name: &str, root: &Path) -> Result<()> {
    let assets = root.join("assets");
    fs::create_dir_all(&assets)?;
    for (letter, bytes) in [
        ('a', include_bytes!("app-assets/a.png").as_slice()),
        ('b', include_bytes!("app-assets/b.png").as_slice()),
        ('c', include_bytes!("app-assets/c.png").as_slice()),
        ('d', include_bytes!("app-assets/d.png").as_slice()),
    ] {
        fs::write(assets.join(format!("{letter}.png")), bytes)?;
    }
    let paths = ['a', 'b', 'c', 'd'].map(|letter| PathBuf::from(format!("assets/{letter}.png")));
    compose(name, root, &paths, &assets.join("header.png"))
}

fn compose(name: &str, root: &Path, paths: &[PathBuf], output: &Path) -> Result<()> {
    let mut header = RgbaImage::from_pixel(WIDTH, HEIGHT, BACKGROUND);
    for (index, path) in paths.iter().enumerate() {
        let path = root.join(path);
        ensure!(path != output, "output must not overwrite a screenshot");
        let screenshot = image::open(&path)
            .with_context(|| format!("could not read {}", path.display()))?
            .to_rgba8();
        ensure!(
            screenshot.dimensions() == (1080, 1240),
            "{} must be a 1080 × 1240 LP3 screenshot",
            path.display()
        );
        let thumbnail = resize(&screenshot, 573, 658, FilterType::Lanczos3);
        overlay(&mut header, &thumbnail, 80 + index as i64 * 613, 310);
    }

    draw_title(&mut header, name)?;
    fs::create_dir_all(output.parent().context("output needs a parent directory")?)?;
    header
        .save_with_format(output, image::ImageFormat::Png)
        .with_context(|| format!("could not write {}", output.display()))
}

fn draw_title(header: &mut RgbaImage, name: &str) -> Result<()> {
    let font = FontArc::try_from_slice(FONT).context("Public Sans is not valid")?;
    // CSS font-size uses the em square; ab_glyph scales by ascent minus descent.
    let scale = font
        .pt_to_px_scale(64.0 * 72.0 / 96.0)
        .context("font has no em size")?;
    let scaled = font.as_scaled(scale);
    let mut caret = 0.0;
    let mut previous = None;
    let glyphs: Vec<_> = name
        .chars()
        .map(|character| {
            let id = scaled.glyph_id(character);
            if let Some(previous) = previous {
                caret += scaled.kern(previous, id);
            }
            let glyph = id.with_scale_and_position(scale, point(caret, 0.0));
            caret += scaled.h_advance(id);
            previous = Some(id);
            glyph
        })
        .collect();
    let group_width = 150.0 + 24.0 + caret;
    ensure!(
        group_width <= (WIDTH - 160) as f32,
        "app name is too wide for the header"
    );
    let left = ((WIDTH as f32 - group_width) / 2.0).round();
    let icon = ink_compiler::generate_icon_image(name, 150)?;
    overlay(header, &icon, left as i64, 80);
    let baseline = 155.0 + (scaled.ascent() + scaled.descent()) / 2.0;
    for mut glyph in glyphs {
        glyph.position += point(left + 174.0, baseline);
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                let x = bounds.min.x as i32 + x as i32;
                let y = bounds.min.y as i32 + y as i32;
                if x >= 0 && y >= 0 && x < WIDTH as i32 && y < header.height() as i32 {
                    let value = (57.0 + 198.0 * coverage).round() as u8;
                    header.put_pixel(x as u32, y as u32, Rgba([value, value, value, 255]));
                }
            });
        }
    }
    Ok(())
}
