use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use anyhow::{Context, Result, bail};
use material_icons::{ALL_ICONS, FONT, Icon, icon_to_char, icon_to_html_name};

const LP3_REFERENCE_SCALE: f32 = 2.55;
const MAX_ICON_PIXELS: u32 = 512;
const TOGGLE_CIRCLE_SIZE: f32 = 9.8;
const TOGGLE_CIRCLE_BORDER: f32 = 2.5;
const TOGGLE_MASK_PADDING: u32 = 1;
const TOGGLE_SAMPLES: u32 = 4;

pub struct RasterIcon {
    pub id: u64,
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

pub fn exists(name: &str) -> bool {
    find(name).is_some()
}

pub fn raster(name: &str, logical_size: f32) -> Result<RasterIcon> {
    let icon = find(name).with_context(|| format!("unknown Material icon {name:?}"))?;
    let dimension = (logical_size * LP3_REFERENCE_SCALE).round().max(1.0) as u32;
    if dimension > MAX_ICON_PIXELS {
        bail!("Material icon {name:?} exceeds Ink's maximum size");
    }

    let font = FontArc::try_from_slice(FONT).context("Material Icons font is invalid")?;
    let scaled = font.as_scaled(PxScale::from(dimension as f32));
    let glyph = scaled
        .glyph_id(icon_to_char(icon))
        .with_scale(dimension as f32);
    let outlined = font
        .outline_glyph(glyph)
        .with_context(|| format!("Material icon {name:?} has no outline"))?;
    let bounds = outlined.px_bounds();
    let glyph_width = bounds.width().ceil().max(0.0) as u32;
    let glyph_height = bounds.height().ceil().max(0.0) as u32;
    let offset_x = dimension.saturating_sub(glyph_width) / 2;
    let offset_y = dimension.saturating_sub(glyph_height) / 2;
    let mut pixels = vec![0; (dimension * dimension) as usize];
    outlined.draw(|x, y, coverage| {
        let x = x + offset_x;
        let y = y + offset_y;
        if x < dimension && y < dimension {
            pixels[(y * dimension + x) as usize] = (coverage * 255.0).round() as u8;
        }
    });

    Ok(RasterIcon {
        id: hash(name, dimension),
        width: dimension as u16,
        height: dimension as u16,
        pixels,
    })
}

pub fn toggle_circle(filled: bool) -> RasterIcon {
    let circle_dimension = (TOGGLE_CIRCLE_SIZE * LP3_REFERENCE_SCALE).round() as u32;
    let dimension = circle_dimension + TOGGLE_MASK_PADDING * 2;
    let outer_radius = circle_dimension as f32 / 2.0;
    let inner_radius = outer_radius - TOGGLE_CIRCLE_BORDER * LP3_REFERENCE_SCALE;
    let centre = dimension as f32 / 2.0;
    let mut pixels = vec![0; (dimension * dimension) as usize];

    for y in 0..dimension {
        for x in 0..dimension {
            let mut covered = 0;
            for sample_y in 0..TOGGLE_SAMPLES {
                for sample_x in 0..TOGGLE_SAMPLES {
                    let sample_x = x as f32 + (sample_x as f32 + 0.5) / TOGGLE_SAMPLES as f32;
                    let sample_y = y as f32 + (sample_y as f32 + 0.5) / TOGGLE_SAMPLES as f32;
                    let distance =
                        ((sample_x - centre).powi(2) + (sample_y - centre).powi(2)).sqrt();
                    if distance <= outer_radius && (filled || distance >= inner_radius) {
                        covered += 1;
                    }
                }
            }
            pixels[(y * dimension + x) as usize] =
                (covered * 255 / (TOGGLE_SAMPLES * TOGGLE_SAMPLES)) as u8;
        }
    }

    RasterIcon {
        id: hash(
            if filled {
                "ink_toggle_on"
            } else {
                "ink_toggle_off"
            },
            dimension,
        ),
        width: dimension as u16,
        height: dimension as u16,
        pixels,
    }
}

fn find(name: &str) -> Option<Icon> {
    ALL_ICONS
        .iter()
        .copied()
        .find(|icon| icon_to_html_name(icon) == name)
}

fn hash(name: &str, dimension: u32) -> u64 {
    let mut value = 0xcbf29ce484222325_u64;
    for byte in name.bytes().chain(dimension.to_le_bytes()) {
        value ^= u64::from(byte);
        value = value.wrapping_mul(0x100000001b3);
    }
    value
}
