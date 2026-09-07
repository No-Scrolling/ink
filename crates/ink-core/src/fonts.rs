use std::sync::OnceLock;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use unicode_segmentation::UnicodeSegmentation;

use crate::{PUBLIC_SANS, is_emoji_grapheme};

pub fn font_for_character(character: char) -> (usize, &'static FontArc) {
    static PRIMARY: OnceLock<FontArc> = OnceLock::new();
    static FALLBACKS: OnceLock<Vec<FontArc>> = OnceLock::new();
    let primary = PRIMARY.get_or_init(|| {
        FontArc::try_from_slice(PUBLIC_SANS).expect("bundled Public Sans is valid")
    });
    if primary.glyph_id(character).0 != 0 {
        return (0, primary);
    }
    let fallbacks = FALLBACKS.get_or_init(system_fonts);
    for (index, font) in fallbacks.iter().enumerate() {
        if font.glyph_id(character).0 != 0 {
            return (index + 1, font);
        }
    }
    (0, primary)
}

#[cfg(target_os = "android")]
fn system_fonts() -> Vec<FontArc> {
    [
        "/system/fonts/NotoSansSymbols-Regular-Subsetted.ttf",
        "/system/fonts/NotoSansSymbols-Regular-Subsetted2.ttf",
        "/system/fonts/NotoSansSymbols-Regular.ttf",
        "/system/fonts/NotoSansSymbols2-Regular.ttf",
    ]
    .into_iter()
    .filter_map(|path| std::fs::read(path).ok())
    .filter_map(|bytes| FontArc::try_from_vec(bytes).ok())
    .collect()
}

#[cfg(not(target_os = "android"))]
fn system_fonts() -> Vec<FontArc> {
    Vec::new()
}

pub fn text_width(text: &str, size: f32) -> f32 {
    text_width_with_numbers(text, size, false)
}

pub fn tabular_digit_width(size: f32) -> f32 {
    static WIDTH: OnceLock<f32> = OnceLock::new();
    size * WIDTH.get_or_init(|| {
        let (_, font) = font_for_character('0');
        let scaled = font.as_scaled(PxScale::from(1.0));
        ('0'..='9').map(|digit| scaled.h_advance(scaled.glyph_id(digit))).fold(0.0, f32::max)
    })
}

pub fn text_width_with_numbers(text: &str, size: f32, tabular_numbers: bool) -> f32 {
    let mut previous = None;
    let mut width = 0.0;
    for grapheme in text.graphemes(true) {
        if is_emoji_grapheme(grapheme) {
            previous = None;
            width += size;
            continue;
        }
        for character in grapheme.chars() {
            if tabular_numbers && character.is_ascii_digit() {
                width += tabular_digit_width(size);
                previous = None;
                continue;
            }
            let (index, font) = font_for_character(character);
            let scaled = font.as_scaled(PxScale::from(size));
            let glyph = scaled.glyph_id(character);
            if let Some((previous_index, previous_glyph)) = previous {
                if previous_index == index {
                    width += scaled.kern(previous_glyph, glyph);
                }
            }
            width += scaled.h_advance(glyph);
            previous = Some((index, glyph));
        }
    }
    width
}
