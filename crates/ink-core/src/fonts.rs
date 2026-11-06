use std::sync::{
    OnceLock,
    atomic::{AtomicPtr, Ordering},
};

use ab_glyph::{Font, FontArc, GlyphId, PxScale, ScaleFont};
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

/// A loaded font. Themes install their fonts into numbered slots; `FontRole::DEFAULT` is the
/// bundled Public Sans.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontRole(u8);

impl FontRole {
    pub const DEFAULT: Self = Self(u8::MAX);

    pub const fn slot(index: u8) -> Self {
        Self(index)
    }

    /// The slot index, or `None` for Public Sans.
    pub fn index(self) -> Option<u8> {
        (self != Self::DEFAULT).then_some(self.0)
    }
}

impl Default for FontRole {
    fn default() -> Self {
        Self::DEFAULT
    }
}

struct ThemeFont {
    bytes: &'static [u8],
    font: FontArc,
    index: usize,
    ascii: [GlyphId; 128],
    tabular: f32,
    raster_scale: f32,
}

pub const MAX_THEME_FONTS: usize = 16;
// Theme fonts are installed once per process before layout, so lookups stay a single atomic load.
static THEME_FONTS: [AtomicPtr<ThemeFont>; MAX_THEME_FONTS] = [const { AtomicPtr::new(std::ptr::null_mut()) }; MAX_THEME_FONTS];
const THEME_FONT_INDEX: usize = 64;

/// Install a theme's fonts into slots 0, 1, 2… in order.
pub fn set_theme_fonts(fonts: Vec<Vec<u8>>) -> anyhow::Result<()> {
    anyhow::ensure!(fonts.len() <= MAX_THEME_FONTS, "a theme can use at most {MAX_THEME_FONTS} font files");
    for (slot, bytes) in fonts.into_iter().enumerate() {
        // Hosts may install the same fonts more than once; keep the loaded copy.
        let installed = unsafe { THEME_FONTS[slot].load(Ordering::Acquire).as_ref() };
        if installed.is_some_and(|font| font.bytes == bytes.as_slice()) {
            continue;
        }
        let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let font = FontArc::try_from_slice(bytes).map_err(|_| anyhow::anyhow!("invalid theme font in slot {slot}"))?;
        let scaled = font.as_scaled(PxScale::from(1.0));
        let tabular = ('0'..='9').map(|digit| scaled.h_advance(font.glyph_id(digit))).fold(0.0, f32::max);
        let raster_scale = font.height_unscaled() / font.units_per_em().unwrap_or(1000.0);
        let ascii = std::array::from_fn(|code| font.glyph_id(code as u8 as char));
        let font = Box::leak(Box::new(ThemeFont { bytes, font, index: THEME_FONT_INDEX + slot, ascii, tabular, raster_scale }));
        THEME_FONTS[slot].store(font, Ordering::Release);
    }
    Ok(())
}

fn theme_font(role: FontRole) -> Option<&'static ThemeFont> {
    let slot = THEME_FONTS.get(role.index()? as usize)?;
    unsafe { slot.load(Ordering::Acquire).as_ref() }
}

/// Converts a CSS-style font size to the scale ab_glyph rasterises at for this role's font.
pub fn raster_scale(role: FontRole) -> Option<f32> {
    theme_font(role).map(|font| font.raster_scale)
}

/// A font file's bytes, for platform text editors.
pub fn font_bytes(role: FontRole) -> &'static [u8] {
    theme_font(role).map_or(PUBLIC_SANS, |font| font.bytes)
}

/// The primary font for a role, used for line metrics.
pub fn primary_font(role: FontRole) -> &'static FontArc {
    theme_font(role).map_or_else(public_sans, |font| &font.font)
}

fn public_sans() -> &'static FontArc {
    static PRIMARY: OnceLock<FontArc> = OnceLock::new();
    PRIMARY.get_or_init(|| {
        FontArc::try_from_slice(PUBLIC_SANS).expect("bundled Public Sans is valid")
    })
}

pub fn font_for_character_in(character: char, role: FontRole) -> (usize, &'static FontArc) {
    let (index, font, _) = glyph_for_character(character, role);
    (index, font)
}

fn glyph_for_character(character: char, role: FontRole) -> (usize, &'static FontArc, GlyphId) {
    if let Some(theme) = theme_font(role) {
        let glyph = if character.is_ascii() { theme.ascii[character as usize] } else { theme.font.glyph_id(character) };
        if glyph.0 != 0 {
            return (theme.index, &theme.font, glyph);
        }
    }
    let primary = public_sans();
    let glyph = if character.is_ascii() {
        static ASCII: OnceLock<[GlyphId; 128]> = OnceLock::new();
        ASCII.get_or_init(|| std::array::from_fn(|code| primary.glyph_id(code as u8 as char)))[character as usize]
    } else {
        primary.glyph_id(character)
    };
    if glyph.0 != 0 {
        return (0, primary, glyph);
    }
    for (index, font) in system_fonts() {
        let glyph = font.glyph_id(character);
        if glyph.0 != 0 {
            return (index, font, glyph);
        }
    }
    (0, primary, glyph)
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

    static FONTS: [SystemFont; 6] = [
        SystemFont::new("/system/fonts/NotoSansSymbols-Regular-Subsetted.ttf"),
        SystemFont::new("/system/fonts/NotoSansSymbols-Regular-Subsetted2.ttf"),
        SystemFont::new("/system/fonts/NotoSansSymbols-Regular.ttf"),
        SystemFont::new("/system/fonts/NotoSansSymbols2-Regular.ttf"),
        SystemFont::new("/system/fonts/NotoSansCJK-Regular.ttc"),
        SystemFont::new("/system/fonts/NotoNaskhArabic-Regular.ttf"),
    ];
    FONTS.iter().enumerate().filter_map(|(index, font)| font.get().map(|font| (index + 1, font)))
}

#[cfg(all(not(target_os = "android"), not(feature = "design-fonts")))]
fn system_fonts() -> impl Iterator<Item = (usize, &'static FontArc)> {
    std::iter::empty()
}

#[cfg(all(not(target_os = "android"), feature = "design-fonts"))]
fn system_fonts() -> impl Iterator<Item = (usize, &'static FontArc)> {
    static FONTS: OnceLock<[FontArc; 2]> = OnceLock::new();
    FONTS.get_or_init(|| [
        FontArc::try_from_slice(include_bytes!("../../../assets/fonts/NotoSansSymbols-Regular-Subsetted.ttf")).expect("bundled Noto Sans Symbols is valid"),
        FontArc::try_from_slice(include_bytes!("../../../assets/fonts/NotoSansSymbols-Regular-Subsetted2.ttf")).expect("bundled Noto Sans Symbols is valid"),
    ]).iter().enumerate().map(|(index, font)| (index + 1, font))
}

pub fn tabular_digit_width_in(size: f32, role: FontRole) -> f32 {
    if let Some(theme) = theme_font(role) {
        return size * theme.tabular;
    }
    static WIDTH: OnceLock<f32> = OnceLock::new();
    size * WIDTH.get_or_init(|| {
        let (_, font) = font_for_character_in('0', FontRole::DEFAULT);
        let scaled = font.as_scaled(PxScale::from(1.0));
        ('0'..='9').map(|digit| scaled.h_advance(scaled.glyph_id(digit))).fold(0.0, f32::max)
    })
}

pub fn text_width_in(text: &str, size: f32, tabular_numbers: bool, role: FontRole) -> f32 {
    visual_text_width_in(&visual_text(text), size, tabular_numbers, role)
}

pub fn visual_text_width_in(text: &str, size: f32, tabular_numbers: bool, role: FontRole) -> f32 {
    TextWidth::new_in(size, tabular_numbers, role).push(text)
}

pub(crate) fn has_rtl(text: &str) -> bool {
    !text.is_ascii() && text.chars().any(|character| matches!(
        unicode_bidi::bidi_class(character), unicode_bidi::BidiClass::R | unicode_bidi::BidiClass::AL
    ))
}

pub fn visual_text(text: &str) -> std::borrow::Cow<'_, str> {
    if !has_rtl(text) { return std::borrow::Cow::Borrowed(text); }
    static SHAPER: OnceLock<arabic_reshaper::ArabicReshaper<'static>> = OnceLock::new();
    let shaper = SHAPER.get_or_init(|| {
        let mut shaper = arabic_reshaper::ArabicReshaper::new();
        shaper.configuration.insert("delete_harakat", false);
        shaper
    });
    let shaped = shaper.reshape(text);
    let bidi = unicode_bidi::BidiInfo::new(&shaped, None);
    let mut visual = String::with_capacity(shaped.len());
    for paragraph in &bidi.paragraphs {
        visual.push_str(&bidi.reorder_line(paragraph, paragraph.range.clone()));
    }
    std::borrow::Cow::Owned(visual)
}

pub(crate) struct TextWidth {
    size: f32,
    role: FontRole,
    tabular_width: Option<f32>,
    previous: Option<(usize, ab_glyph::GlyphId)>,
    width: f32,
}

impl TextWidth {
    pub fn new_in(size: f32, tabular_numbers: bool, role: FontRole) -> Self {
        Self {
            size,
            role,
            tabular_width: tabular_numbers.then(|| tabular_digit_width_in(size, role)),
            previous: None,
            width: 0.0,
        }
    }

    pub fn push(&mut self, text: &str) -> f32 {
        if text.is_ascii() {
            for character in text.bytes() {
                self.push_character(character as char);
            }
        } else {
            for grapheme in text_graphemes(text) {
                if is_emoji_grapheme(grapheme) {
                    self.previous = None;
                    self.width += self.size;
                    continue;
                }
                for character in grapheme.chars() {
                    self.push_character(character);
                }
            }
        }
        self.width
    }

    fn push_character(&mut self, character: char) {
        if let Some(width) = self.tabular_width
            && character.is_ascii_digit()
        {
            self.width += width;
            self.previous = None;
            return;
        }
        let (index, font, glyph) = glyph_for_character(character, self.role);
        let scaled = font.as_scaled(PxScale::from(self.size));
        if let Some((previous_index, previous_glyph)) = self.previous {
            if previous_index == index {
                self.width += scaled.kern(previous_glyph, glyph);
            }
        }
        self.width += scaled.h_advance(glyph);
        self.previous = Some((index, glyph));
    }
}
