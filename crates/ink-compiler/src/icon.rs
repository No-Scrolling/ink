use std::{io::Cursor, path::Path};

use ab_glyph::{Font, FontArc, Glyph, PxScale, ScaleFont, point};
use anyhow::{Context, Result, bail};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use unicode_segmentation::UnicodeSegmentation;

const ICONS: [(&str, u32); 5] = [
    ("mipmap-mdpi", 48),
    ("mipmap-hdpi", 72),
    ("mipmap-xhdpi", 96),
    ("mipmap-xxhdpi", 144),
    ("mipmap-xxxhdpi", 192),
];
const PUBLIC_SANS: &[u8] = include_bytes!("../../../assets/fonts/PublicSans-Regular.ttf");

pub fn generate(name: &str, resources: &Path) -> Result<()> {
    let label = name
        .graphemes(true)
        .find(|grapheme| grapheme.chars().any(char::is_alphanumeric))
        .context("the app name must contain a letter or number")?
        .to_uppercase();
    let font = FontArc::try_from_vec(PUBLIC_SANS.to_vec()).context("Public Sans is not valid")?;

    for (directory, size) in ICONS {
        let image = render(&font, &label, size)?;
        let output_directory = resources.join(directory);
        std::fs::create_dir_all(&output_directory)
            .with_context(|| format!("could not create {}", output_directory.display()))?;

        let bytes = encode_png(image)?;
        for file_name in ["ic_launcher.png", "ic_launcher_round.png"] {
            let path = output_directory.join(file_name);
            write_if_changed(&path, &bytes)?;
        }
    }

    Ok(())
}

fn encode_png(image: RgbaImage) -> Result<Vec<u8>> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .context("could not encode launcher icon")?;
    Ok(bytes.into_inner())
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if std::fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    std::fs::write(path, bytes).with_context(|| format!("could not write {}", path.display()))
}

fn render(font: &FontArc, label: &str, size: u32) -> Result<RgbaImage> {
    let scale = PxScale::from(size as f32 * 0.854);
    let scaled = font.as_scaled(scale);
    let mut caret = 0.0;
    let mut glyphs = Vec::new();

    for character in label.chars() {
        let id = scaled.glyph_id(character);
        let glyph = id.with_scale_and_position(scale, point(caret, 0.0));
        caret += scaled.h_advance(id);
        glyphs.push(glyph);
    }

    let bounds =
        glyph_bounds(font, &glyphs).context("the icon letter is missing from Public Sans")?;
    let centre_x = (bounds.0 + bounds.2) as f32 / 2.0;
    let centre_y = (bounds.1 + bounds.3) as f32 / 2.0;
    let offset_x = size as f32 / 2.0 - centre_x;
    let offset_y = size as f32 / 2.0 - centre_y;
    let mut image = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 255]));

    for mut glyph in glyphs {
        glyph.position.x += offset_x;
        glyph.position.y += offset_y;
        let Some(outlined) = font.outline_glyph(glyph) else {
            continue;
        };
        let bounds = outlined.px_bounds();
        outlined.draw(|x, y, coverage| {
            let x = bounds.min.x as i32 + x as i32;
            let y = bounds.min.y as i32 + y as i32;
            if x >= 0 && y >= 0 && x < size as i32 && y < size as i32 {
                let value = (coverage * 255.0).round() as u8;
                image.put_pixel(x as u32, y as u32, Rgba([value, value, value, 255]));
            }
        });
    }

    if image.pixels().all(|pixel| pixel.0 == [0, 0, 0, 255]) {
        bail!("Public Sans did not produce an icon glyph for {label:?}");
    }

    Ok(image)
}

fn glyph_bounds(font: &FontArc, glyphs: &[Glyph]) -> Option<(i32, i32, i32, i32)> {
    glyphs
        .iter()
        .filter_map(|glyph| font.outline_glyph(glyph.clone()))
        .map(|outlined| outlined.px_bounds())
        .fold(None, |bounds, glyph| {
            Some(match bounds {
                None => (
                    glyph.min.x as i32,
                    glyph.min.y as i32,
                    glyph.max.x as i32,
                    glyph.max.y as i32,
                ),
                Some((min_x, min_y, max_x, max_y)) => (
                    min_x.min(glyph.min.x as i32),
                    min_y.min(glyph.min.y as i32),
                    max_x.max(glyph.max.x as i32),
                    max_y.max(glyph.max.y as i32),
                ),
            })
        })
}
