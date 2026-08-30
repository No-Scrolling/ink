use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use anyhow::{Context, Result, anyhow, bail};
use std::sync::OnceLock;

const MATERIAL_SYMBOLS_OUTLINED_FONT: &[u8] =
    include_bytes!("../../../assets/icons/MaterialSymbolsOutlined-300.ttf.zlib");
const MATERIAL_SYMBOLS_FILLED_FONT: &[u8] =
    include_bytes!("../../../assets/icons/MaterialSymbolsOutlined-Fill1-400.ttf.zlib");
const MATERIAL_SYMBOLS_CODEPOINTS: &str =
    include_str!("../../../assets/icons/MaterialSymbolsOutlined.codepoints");

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
    raster_variant(name, logical_size, false)
}

pub fn raster_filled(name: &str, logical_size: f32) -> Result<RasterIcon> {
    raster_variant(name, logical_size, true)
}

fn raster_variant(name: &str, logical_size: f32, filled: bool) -> Result<RasterIcon> {
    let character = find(name).with_context(|| format!("unknown Material Symbol {name:?}"))?;
    let dimension = (logical_size * LP3_REFERENCE_SCALE).round().max(1.0) as u32;
    if dimension > MAX_ICON_PIXELS {
        bail!("Material Symbol {name:?} exceeds Ink's maximum size");
    }

    let font = material_symbols_font(filled)?;
    let scaled = font.as_scaled(PxScale::from(dimension as f32));
    let glyph = scaled.glyph_id(character).with_scale(dimension as f32);
    let outlined = font
        .outline_glyph(glyph)
        .with_context(|| format!("Material Symbol {name:?} has no outline"))?;
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

    let id = if filled {
        hash(&format!("filled:{name}"), dimension)
    } else {
        hash(name, dimension)
    };

    Ok(RasterIcon {
        id,
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

fn find(name: &str) -> Option<char> {
    MATERIAL_SYMBOLS_CODEPOINTS.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let candidate = fields.next()?;
        let codepoint = fields.next()?;
        (candidate == name)
            .then(|| {
                u32::from_str_radix(codepoint, 16)
                    .ok()
                    .and_then(char::from_u32)
            })
            .flatten()
    })
}

fn material_symbols_font(filled: bool) -> Result<&'static FontArc> {
    static OUTLINED_FONT: OnceLock<Result<FontArc, String>> = OnceLock::new();
    static FILLED_FONT: OnceLock<Result<FontArc, String>> = OnceLock::new();
    let (font, compressed) = if filled {
        (&FILLED_FONT, MATERIAL_SYMBOLS_FILLED_FONT)
    } else {
        (&OUTLINED_FONT, MATERIAL_SYMBOLS_OUTLINED_FONT)
    };
    font.get_or_init(|| {
        let bytes = miniz_oxide::inflate::decompress_to_vec_zlib(compressed)
            .map_err(|error| format!("could not decompress Material Symbols: {error:?}"))?;
        FontArc::try_from_vec(bytes)
            .map_err(|error| format!("Material Symbols font is invalid: {error}"))
    })
    .as_ref()
    .map_err(|error| anyhow!(error.clone()))
}

fn hash(name: &str, dimension: u32) -> u64 {
    let mut value = 0xcbf29ce484222325_u64;
    for byte in name.bytes().chain(dimension.to_le_bytes()) {
        value ^= u64::from(byte);
        value = value.wrapping_mul(0x100000001b3);
    }
    value
}
