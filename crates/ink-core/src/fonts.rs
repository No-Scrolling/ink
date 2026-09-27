use std::sync::OnceLock;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use unicode_segmentation::UnicodeSegmentation;

use crate::{PUBLIC_SANS, is_emoji_grapheme};

pub fn text_graphemes(text: &str) -> impl Iterator<Item = &str> {
    enum Graphemes<'a> {
        Ascii(&'a str),
        Unicode(unicode_segmentation::Graphemes<'a>),
    }

    impl<'a> Iterator for Graphemes<'a> {
        type Item = &'a str;

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Self::Ascii(text) => {
                    if text.is_empty() { return None; }
                    // CRLF is the only multi-byte grapheme entirely within ASCII.
                    let length = if text.starts_with("\r\n") { 2 } else { 1 };
                    let (grapheme, rest) = text.split_at(length);
                    *text = rest;
                    Some(grapheme)
                }
                Self::Unicode(text) => text.next(),
            }
        }
    }

    if text.is_ascii() { Graphemes::Ascii(text) } else { Graphemes::Unicode(text.graphemes(true)) }
}

pub fn font_for_character(character: char) -> (usize, &'static FontArc) {
    static PRIMARY: OnceLock<FontArc> = OnceLock::new();
    let primary = PRIMARY.get_or_init(|| {
        FontArc::try_from_slice(PUBLIC_SANS).expect("bundled Public Sans is valid")
    });
    if primary.glyph_id(character).0 != 0 {
        return (0, primary);
    }
    for (index, font) in system_fonts() {
        if font.glyph_id(character).0 != 0 {
            return (index, font);
        }
    }
    (0, primary)
}

#[cfg(target_os = "android")]
fn system_fonts() -> impl Iterator<Item = (usize, &'static FontArc)> {
    struct SystemFont {
        path: &'static str,
        data: OnceLock<Option<memmap2::Mmap>>,
        font: OnceLock<Option<FontArc>>,
    }

    impl SystemFont {
        const fn new(path: &'static str) -> Self {
            Self { path, data: OnceLock::new(), font: OnceLock::new() }
        }

        fn get(&'static self) -> Option<&'static FontArc> {
            self.font.get_or_init(|| {
                let data = self.data.get_or_init(|| {
                    let file = std::fs::File::open(self.path).ok()?;
                    // Android's system fonts are read-only. Keep the mapping for the
                    // cached font's lifetime, and let the OS page in only what is used.
                    unsafe { memmap2::Mmap::map(&file).ok() }
                }).as_ref()?;
                FontArc::try_from_slice(data).ok()
            }).as_ref()
        }
    }

    static FONTS: [SystemFont; 5] = [
        SystemFont::new("/system/fonts/NotoSansSymbols-Regular-Subsetted.ttf"),
        SystemFont::new("/system/fonts/NotoSansSymbols-Regular-Subsetted2.ttf"),
        SystemFont::new("/system/fonts/NotoSansSymbols-Regular.ttf"),
        SystemFont::new("/system/fonts/NotoSansSymbols2-Regular.ttf"),
        SystemFont::new("/system/fonts/NotoSansCJK-Regular.ttc"),
    ];
    FONTS.iter().enumerate().filter_map(|(index, font)| font.get().map(|font| (index + 1, font)))
}

#[cfg(not(target_os = "android"))]
fn system_fonts() -> impl Iterator<Item = (usize, &'static FontArc)> {
    std::iter::empty()
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
    TextWidth::new(size, tabular_numbers).push(text)
}

pub(crate) struct TextWidth {
    size: f32,
    tabular_numbers: bool,
    previous: Option<(usize, ab_glyph::GlyphId)>,
    width: f32,
}

impl TextWidth {
    pub fn new(size: f32, tabular_numbers: bool) -> Self {
        Self {
            size,
            tabular_numbers,
            previous: None,
            width: 0.0,
        }
    }

    pub fn push(&mut self, text: &str) -> f32 {
        for grapheme in text_graphemes(text) {
            if is_emoji_grapheme(grapheme) {
                self.previous = None;
                self.width += self.size;
                continue;
            }
            for character in grapheme.chars() {
                if self.tabular_numbers && character.is_ascii_digit() {
                    self.width += tabular_digit_width(self.size);
                    self.previous = None;
                    continue;
                }
                let (index, font) = font_for_character(character);
                let scaled = font.as_scaled(PxScale::from(self.size));
                let glyph = scaled.glyph_id(character);
                if let Some((previous_index, previous_glyph)) = self.previous {
                    if previous_index == index {
                        self.width += scaled.kern(previous_glyph, glyph);
                    }
                }
                self.width += scaled.h_advance(glyph);
                self.previous = Some((index, glyph));
            }
        }
        self.width
    }
}
