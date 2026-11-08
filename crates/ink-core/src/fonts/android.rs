use std::{path::Path, sync::{OnceLock, RwLock}};

use ab_glyph::{Font, FontArc, FontRef, GlyphId};
use rustc_hash::FxHashMap;

#[derive(Default)]
pub(super) struct SystemFonts {
    fonts: Vec<SystemFont>,
    characters: RwLock<FxHashMap<char, Option<(usize, GlyphId)>>>,
}

struct SystemFont {
    path: std::path::PathBuf,
    index: u32,
    data: OnceLock<Option<memmap2::Mmap>>,
    font: OnceLock<Option<FontArc>>,
}

impl SystemFont {
    fn get(&'static self) -> Option<&'static FontArc> {
        self.font.get_or_init(|| {
            let data = self.data.get_or_init(|| {
                let file = std::fs::File::open(&self.path).ok()?;
                // System fonts are read-only. The static collection owns the mapping
                // for the font's lifetime; the OS pages in only what is used.
                unsafe { memmap2::Mmap::map(&file).ok() }
            }).as_ref()?;
            FontRef::try_from_slice_and_index(data, self.index).ok().map(FontArc::new)
        }).as_ref()
    }
}

impl SystemFonts {
    #[cfg(target_os = "android")]
    pub(super) fn load() -> Self {
        ["/system/etc/font_fallback.xml", "/system/etc/fonts.xml"].into_iter()
            .find_map(|path| {
                let xml = std::fs::read_to_string(path).ok()?;
                Self::from_xml(&xml, Path::new("/system/fonts"))
            })
            .unwrap_or_default()
    }

    pub(super) fn from_xml(xml: &str, directory: &Path) -> Option<Self> {
        let document = roxmltree::Document::parse(xml).ok()?;
        let root = document.root_element();
        if !root.has_tag_name("familyset") { return None; }
        let mut fonts = Vec::new();
        let families: Vec<_> = root.children().filter(|node| node.has_tag_name("family")).collect();
        for (position, family) in families.iter().enumerate() {
            if matches!(family.attribute("ignore"), Some("true" | "1")) { continue; }
            // The first family is Android's default. Other named families are
            // alternatives (serif, monospace, etc.), not the sans-serif fallback.
            if position != 0 && family.has_attribute("name") { continue; }
            // Prefer non-UI metrics, as TextView does for apps targeting API 35+.
            // Keep compact-only families on devices without an elegant counterpart.
            if family.attribute("variant") == Some("compact") && families.iter().any(|other| {
                other.attribute("variant") == Some("elegant")
                    && other.attribute("lang") == family.attribute("lang")
                    && !matches!(other.attribute("ignore"), Some("true" | "1"))
            }) { continue; }
            let candidates: Vec<_> = family.children()
                .filter(|node| node.has_tag_name("font"))
                .filter(|node| node.attribute("style").unwrap_or("normal") == "normal")
                .filter(|node| matches!(node.attribute("fallbackFor"), None | Some("sans-serif")))
                .collect();
            let specific = candidates.iter().any(|node| node.attribute("fallbackFor") == Some("sans-serif"));
            let font = candidates.into_iter()
                .filter(|node| !specific || node.attribute("fallbackFor") == Some("sans-serif"))
                .min_by_key(|node| node.attribute("weight").unwrap_or("400").parse::<u16>()
                    .unwrap_or(400).abs_diff(400));
            let Some(font) = font else { continue; };
            // Only direct text belongs to the filename; <axis> children carry metadata.
            let filename: String = font.children().filter(|node| node.is_text())
                .filter_map(|node| node.text()).collect();
            let filename = filename.trim();
            if filename.is_empty() { continue; }
            let index = font.attribute("index").unwrap_or("0").parse().ok()?;
            let path = directory.join(filename);
            if fonts.iter().any(|font: &SystemFont| font.path == path && font.index == index) { continue; }
            fonts.push(SystemFont { path, index, data: OnceLock::new(), font: OnceLock::new() });
        }
        Some(Self { fonts, characters: RwLock::new(FxHashMap::default()) })
    }

    pub(super) fn resolve(&'static self, character: char) -> Option<(usize, &'static FontArc, GlyphId)> {
        let cached = self.characters.read().unwrap().get(&character).copied();
        let resolved = cached.unwrap_or_else(|| {
            // Cache misses too: unsupported characters must not scan the chain each frame.
            let resolved = self.fonts.iter().enumerate().find_map(|(index, font)| {
                let font = font.get()?;
                let glyph = font.glyph_id(character);
                // Colour emoji fonts precede some symbol fonts in Android's chain, but
                // their bitmap glyphs have no outline to rasterise; keep looking.
                let drawable = character.is_whitespace() || font.outline(glyph).is_some();
                (glyph.0 != 0 && drawable).then_some((index + 1, glyph))
            });
            self.characters.write().unwrap().insert(character, resolved);
            resolved
        });
        let (index, glyph) = resolved?;
        // Index zero always belongs to Public Sans. Failed files never renumber later fonts.
        Some((index, self.fonts[index - 1].get()?, glyph))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(xml: &str) -> &'static SystemFonts {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts");
        Box::leak(Box::new(SystemFonts::from_xml(xml, &directory).unwrap()))
    }

    #[test]
    fn configured_thai_fallback_rasterises_a_glyph_from_the_selected_collection_face() {
        let fonts = fixture(include_str!("../../tests/fixtures/fonts/fonts.xml"));
        assert!(fonts.fonts.iter().all(|font| font.data.get().is_none()));
        assert_eq!(fonts.resolve('A').unwrap().0, 2);
        assert!(fonts.fonts[2].data.get().is_none(), "later fonts stay unopened");
        let (index, font, glyph) = fonts.resolve('ฅ').expect("Thai must resolve through fonts.xml");
        assert_eq!(index, 3, "missing files must not renumber atlas keys");
        assert_eq!(glyph, GlyphId(1));
        assert_eq!(font.glyph_id('ฅ'), glyph);
        assert_eq!(font.glyph_id('A'), GlyphId(0), "TTC face zero is the Latin font");
        let outline = font.outline_glyph(glyph.with_scale(40.0)).unwrap();
        let mut covered = 0;
        outline.draw(|_, _, coverage| { if coverage > 0.5 { covered += 1; } });
        assert!(covered > 200, "the synthetic triangle must produce visible pixels");
        let again = fonts.resolve('ฅ').unwrap();
        assert_eq!((again.0, again.2), (index, glyph));
        assert!(std::ptr::eq(again.1, font));
        assert!(fonts.fonts[3].data.get().is_none(), "lookup stops at the matching font");
        assert_eq!(fonts.characters.read().unwrap().get(&'ฅ'), Some(&Some((3, GlyphId(1)))));
        assert!(fonts.resolve('\u{10ffff}').is_none());
        assert_eq!(fonts.characters.read().unwrap().get(&'\u{10ffff}'), Some(&None));
    }

    #[test]
    fn modern_config_defaults_and_sans_specific_fallback_keep_their_order() {
        let fonts = fixture(r#"<familyset>
            <family name="sans-serif"><font supportedAxes="wght,ital">missing.ttf</font></family>
            <family lang="und-Thai" ignore="1"><font index="1">fallback.ttc</font></family>
            <family lang="und-Thai">
                <font>fallback.ttc</font>
                <font fallbackFor="serif">fallback.ttc</font>
                <font fallbackFor="sans-serif" index="1" supportedAxes="wght">fallback.ttc</font>
            </family>
            <family lang="und-Thai"><font index="1">fallback.ttc</font></family>
            <family lang="und-Latn" variant="compact"><font>fallback.ttc</font></family>
        </familyset>"#);
        assert_eq!(fonts.resolve('ฅ').unwrap().0, 2);
        assert_eq!(fonts.resolve('A').unwrap().0, 3);
        assert_eq!(fonts.fonts.len(), 3, "duplicate file/face pairs share an atlas index");
    }

    #[test]
    fn symbols_skip_an_earlier_colour_emoji_font_without_outlines() {
        let fonts = fixture(r#"<familyset>
            <family name="sans-serif"><font>missing.ttf</font></family>
            <family lang="und-Zsye"><font index="2">fallback.ttc</font></family>
            <family lang="und-Zsym"><font index="3">fallback.ttc</font></family>
        </familyset>"#);
        let (index, font, glyph) = fonts.resolve('♥').expect("the symbol font draws the heart");
        assert_eq!(index, 3);
        assert!(font.outline(glyph).is_some());
    }

    #[test]
    fn malformed_config_does_not_produce_a_partial_chain() {
        let directory = Path::new("/system/fonts");
        assert!(SystemFonts::from_xml("<familyset><family>", directory).is_none());
        assert!(SystemFonts::from_xml("<unrelated/>", directory).is_none());
    }
}
