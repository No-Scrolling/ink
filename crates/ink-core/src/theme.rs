//! App themes compiled from `theme.ts`.
//!
//! The compiler evaluates the theme once and writes `ink-theme-v1.bin`: a JSON description
//! followed by font and image blobs. Hosts decode it before the first layout, so the renderer
//! only reads resolved numbers and never runs theme code.

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use crate::{Colour, FontRole, ImageAsset, ImageAssetEncoding, ImageData, LP3_REFERENCE_SCALE};

const MAGIC: &[u8; 4] = b"INKT";

/// Default body text size in pixels of the 1080 × 1240 export frame.
pub const DEFAULT_TEXT_PX: f32 = 30.0 * LP3_REFERENCE_SCALE;

/// The colours a theme assigns to Ink's three roles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Roles {
    pub background: Colour,
    pub foreground: Colour,
    pub muted: Colour,
    /// The scroll bar's colour, when it differs from the foreground.
    pub scroll_bar: Option<Colour>,
}

impl Roles {
    pub const DARK: Self = Self { background: Colour::BLACK, foreground: Colour::WHITE, muted: Colour::MUTED, scroll_bar: None };

    fn inverted(self) -> Self {
        Self {
            background: crate::invert_colour(self.background),
            foreground: crate::invert_colour(self.foreground),
            // Inverting a mid-grey in linear light gives a near-white; inverting it in sRGB keeps
            // muted text as far from the light background as it is from the dark one.
            muted: invert_srgb(self.muted),
            scroll_bar: self.scroll_bar.map(crate::invert_colour),
        }
    }
}

/// Inverts a linear colour in sRGB space.
fn invert_srgb(colour: Colour) -> Colour {
    let srgb = |linear: f32| if linear <= 0.0031308 { linear * 12.92 } else { 1.055 * linear.powf(1.0 / 2.4) - 0.055 };
    Colour { red: 1.0 - srgb(colour.red), green: 1.0 - srgb(colour.green), blue: 1.0 - srgb(colour.blue), ..colour }.to_linear()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub dark: Roles,
    pub light: Roles,
}

impl Default for Palette {
    fn default() -> Self {
        Self { dark: Roles::DARK, light: Roles::DARK.inverted() }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeaderLayout {
    #[default]
    Centred,
    Large,
}

/// How screens treat display cutouts at the top of the screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CutoutMode {
    /// The header fits in the widest space beside the cutouts.
    #[default]
    Beside,
    /// The screen starts below the cutouts.
    Below,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ToggleStyle {
    #[default]
    Line,
    Switch,
    /// Pre-rasterised control images, indexed by `[light][on + 2 × disabled]`, with their size in
    /// reference units.
    Custom { width: f32, height: f32, images: Box<[[ImageData; 4]; 2]> },
}

/// When the scroll bar shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollBarVisibility {
    #[default]
    Always,
    /// Shows while the screen scrolls, then fades out.
    Scrolling,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionStyle {
    #[default]
    Underline,
    Check,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldLayout {
    #[default]
    Stacked,
    Inline,
}

/// A theme with every value resolved. Sizes are Ink reference units, already divided by the
/// theme's scale, so layout can keep using `Engine::scaled`.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Multiplies every reference size. Derived from the body text size.
    pub scale: f32,
    pub body_font: FontRole,
    /// Screen titles; the body font unless the theme sets one.
    pub display_font: FontRole,
    pub input_font: FontRole,
    pub text_input: crate::InputAppearance,
    pub hit_slop: crate::HitInsets,
    /// Named variants for individual screens, each a complete theme.
    pub variants: BTreeMap<String, Arc<Theme>>,
    pub inset: Option<f32>,
    pub gap: Option<f32>,
    pub row_gap: Option<f32>,
    pub palette: Palette,
    pub header: HeaderLayout,
    pub header_size: Option<f32>,
    pub header_top: Option<f32>,
    pub header_bottom: Option<f32>,
    pub header_cutout: CutoutMode,
    pub header_icon_size: Option<f32>,
    /// Show Back at the end of the header instead of the start.
    pub back_at_end: bool,
    /// Use an arrow for Back instead of a chevron.
    pub back_arrow: bool,
    pub toggle: ToggleStyle,
    pub selection: SelectionStyle,
    pub field: FieldLayout,
    pub section_uppercase: bool,
    pub section_size: Option<f32>,
    pub section_gap: Option<f32>,
    /// Size and tone of secondary lines under titles and toggle labels.
    pub subtitle_size: Option<f32>,
    pub subtitle_muted: Option<bool>,
    pub scroll_bar_visibility: ScrollBarVisibility,
    pub scroll_bar_track: bool,
    /// Thumb width and distance from the screen edge to the bar, in reference units.
    pub scroll_bar_width: Option<f32>,
    pub scroll_bar_inset: Option<f32>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            scale: 1.0,
            body_font: FontRole::DEFAULT,
            display_font: FontRole::DEFAULT,
            input_font: FontRole::DEFAULT,
            text_input: crate::InputAppearance::default(),
            hit_slop: crate::HitInsets::default(),
            variants: BTreeMap::new(),
            inset: None,
            gap: None,
            row_gap: None,
            palette: Palette::default(),
            header: HeaderLayout::Centred,
            header_size: None,
            header_top: None,
            header_bottom: None,
            header_cutout: CutoutMode::Beside,
            header_icon_size: None,
            back_at_end: false,
            back_arrow: false,
            toggle: ToggleStyle::Line,
            selection: SelectionStyle::Underline,
            field: FieldLayout::Stacked,
            section_uppercase: true,
            section_size: None,
            section_gap: None,
            subtitle_size: None,
            subtitle_muted: None,
            scroll_bar_visibility: ScrollBarVisibility::Always,
            scroll_bar_track: true,
            scroll_bar_width: None,
            scroll_bar_inset: None,
        }
    }
}

/// The theme description the compiler writes. Sizes are export-frame pixels.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeSource {
    #[serde(default)]
    pub colours: Option<SourceColours>,
    #[serde(default)]
    pub fonts: SourceFonts,
    #[serde(default)]
    pub text: SourceText,
    #[serde(default)]
    pub spacing: SourceSpacing,
    #[serde(default)]
    pub header: SourceHeader,
    #[serde(default)]
    pub toggle: Option<SourceToggle>,
    #[serde(default)]
    pub selection: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub section: SourceSection,
    #[serde(default)]
    pub scroll_bar: SourceScrollBar,
    #[serde(default)]
    pub subtitle: SourceSubtitle,
    #[serde(default)]
    pub text_input: crate::InputAppearance,
    #[serde(default)]
    pub hit_slop: crate::HitInsets,
    /// Complete descriptions of each named variant, sharing this file's blobs.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub variants: BTreeMap<String, ThemeSource>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSubtitle {
    pub size: Option<f32>,
    pub tone: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceScrollBar {
    pub colour: Option<String>,
    pub visibility: Option<String>,
    pub track: Option<bool>,
    pub width: Option<f32>,
    pub inset: Option<f32>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceColours {
    pub dark: Option<SourceRoles>,
    pub light: Option<SourceRoles>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRoles {
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub muted: Option<String>,
}

/// Blob indices for each font.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFonts {
    pub body: Option<usize>,
    pub display: Option<usize>,
    pub input: Option<usize>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceText {
    pub size: Option<f32>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSpacing {
    pub inset: Option<f32>,
    pub gap: Option<f32>,
    pub row_gap: Option<f32>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceHeader {
    pub layout: Option<String>,
    pub size: Option<f32>,
    pub top: Option<f32>,
    pub bottom: Option<f32>,
    pub cutout: Option<String>,
    pub back: Option<SourceBack>,
    #[serde(rename = "iconSize")]
    pub icon_size: Option<f32>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBack {
    pub position: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceToggle {
    pub style: String,
    /// Custom controls: size in export pixels and the image blob for each `[scheme][state]`.
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub images: Option<[[SourceImage; 4]; 2]>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceImage {
    pub blob: usize,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSection {
    pub uppercase: Option<bool>,
    pub size: Option<f32>,
    pub gap: Option<f32>,
}

/// A decoded theme and the fonts it installs.
pub struct ThemeBundle {
    pub theme: Theme,
    /// Font files for slots 0, 1, 2…, for `set_theme_fonts`.
    pub fonts: Vec<Vec<u8>>,
}

impl ThemeSource {
    /// Encode the description and its blobs. Image blobs are zlib-compressed RGBA.
    pub fn encode(&self, blobs: &[Vec<u8>]) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(&(self, blobs.iter().map(Vec::len).collect::<Vec<_>>()))?;
        let mut bytes = Vec::with_capacity(8 + json.len() + blobs.iter().map(Vec::len).sum::<usize>());
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&u32::try_from(json.len())?.to_le_bytes());
        bytes.extend_from_slice(&json);
        for blob in blobs {
            bytes.extend_from_slice(blob);
        }
        Ok(bytes)
    }
}

impl ThemeBundle {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() >= 8 && &bytes[..4] == MAGIC, "invalid theme file");
        let length = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
        let json = bytes.get(8..8 + length).context("truncated theme file")?;
        let (source, lengths): (ThemeSource, Vec<usize>) = serde_json::from_slice(json)?;
        let mut blobs = Vec::with_capacity(lengths.len());
        let mut offset = 8 + length;
        for length in lengths {
            blobs.push(bytes.get(offset..offset + length).context("truncated theme blob")?);
            offset += length;
        }
        let mut slots = Vec::new();
        let mut theme = source.resolve(&blobs, &mut slots)?;
        let variants = std::mem::take(&mut theme.1);
        for (name, variant) in variants {
            let (variant, nested) = variant.resolve(&blobs, &mut slots)?;
            ensure!(nested.is_empty(), "theme variant {name:?} cannot have its own variants");
            theme.0.variants.insert(name, Arc::new(variant));
        }
        ensure!(slots.len() <= crate::fonts::MAX_THEME_FONTS, "a theme can use at most {} font files", crate::fonts::MAX_THEME_FONTS);
        Ok(ThemeBundle {
            theme: theme.0,
            fonts: slots.iter().map(|&blob| blobs[blob].to_vec()).collect(),
        })
    }
}

impl ThemeSource {
    /// Resolve this description, assigning a font slot to each distinct font blob.
    fn resolve(self, blobs: &[&[u8]], slots: &mut Vec<usize>) -> Result<(Theme, BTreeMap<String, ThemeSource>)> {
        let blob = |index: usize| blobs.get(index).copied().context("missing theme blob");
        let mut slot = |index: Option<usize>| -> Result<Option<FontRole>> {
            let Some(index) = index else { return Ok(None) };
            blob(index)?;
            let position = slots.iter().position(|&existing| existing == index).unwrap_or_else(|| {
                slots.push(index);
                slots.len() - 1
            });
            Ok(Some(FontRole::slot(position as u8)))
        };
        let body_font = slot(self.fonts.body)?.unwrap_or(FontRole::DEFAULT);
        let display_font = slot(self.fonts.display)?.unwrap_or(body_font);
        let input_font = slot(self.fonts.input)?.unwrap_or(body_font);
        let scale = match self.text.size {
            Some(size) => {
                ensure!(size.is_finite() && size > 0.0, "text.size must be positive");
                size / DEFAULT_TEXT_PX
            }
            None => 1.0,
        };
        // Export pixels to reference units at this theme's scale.
        let units = |px: f32| px / (LP3_REFERENCE_SCALE * scale);
        let dark = self.colours.as_ref().and_then(|colours| colours.dark.clone()).unwrap_or_default();
        let mut dark = resolve_roles(&dark, Roles::DARK)?;
        dark.scroll_bar = self.scroll_bar.colour.as_deref().map(parse_colour).transpose()?;
        let light = match self.colours.as_ref().and_then(|colours| colours.light.as_ref()) {
            Some(light) => resolve_roles(light, dark.inverted())?,
            None => dark.inverted(),
        };
        let toggle = match &self.toggle {
            None => ToggleStyle::Line,
            Some(toggle) => match toggle.style.as_str() {
                "line" => ToggleStyle::Line,
                "switch" => ToggleStyle::Switch,
                "custom" => {
                    let (Some(width), Some(height), Some(images)) = (toggle.width, toggle.height, &toggle.images) else {
                        bail!("custom toggle requires width, height and images");
                    };
                    let image = |source: &SourceImage| -> Result<ImageData> {
                        let bytes = blob(source.blob)?;
                        Ok(ImageData::Asset(ImageAsset {
                            id: crate::masks::hash_bytes(bytes),
                            width: source.width,
                            height: source.height,
                            encoding: ImageAssetEncoding::RgbaZlib,
                            bytes: crate::AssetBytes::Owned(Arc::from(bytes)),
                        }))
                    };
                    let scheme = |images: &[SourceImage; 4]| -> Result<[ImageData; 4]> {
                        Ok([image(&images[0])?, image(&images[1])?, image(&images[2])?, image(&images[3])?])
                    };
                    ToggleStyle::Custom {
                        width: units(width),
                        height: units(height),
                        images: Box::new([scheme(&images[0])?, scheme(&images[1])?]),
                    }
                }
                style => bail!("toggle style must be \"line\", \"switch\" or a custom control, got {style:?}"),
            },
        };
        let theme = Theme {
            scale,
            body_font,
            display_font,
            input_font,
            text_input: self.text_input.in_units(LP3_REFERENCE_SCALE * scale)?,
            hit_slop: self.hit_slop.in_units(LP3_REFERENCE_SCALE * scale),
            variants: BTreeMap::new(),
            inset: self.spacing.inset.map(units),
            gap: self.spacing.gap.map(units),
            row_gap: self.spacing.row_gap.map(units),
            palette: Palette { dark, light },
            header: match self.header.layout.as_deref() {
                None | Some("centred") => HeaderLayout::Centred,
                Some("large") => HeaderLayout::Large,
                Some(layout) => bail!("header.layout must be \"centred\" or \"large\", got {layout:?}"),
            },
            header_size: self.header.size.map(units),
            header_top: self.header.top.map(units),
            header_bottom: self.header.bottom.map(units),
            header_cutout: match self.header.cutout.as_deref() {
                None | Some("beside") => CutoutMode::Beside,
                Some("below") => CutoutMode::Below,
                Some(mode) => bail!("header.cutout must be \"beside\" or \"below\", got {mode:?}"),
            },
            header_icon_size: self.header.icon_size.map(units),
            back_at_end: match self.header.back.as_ref().and_then(|back| back.position.as_deref()) {
                None | Some("start") => false,
                Some("end") => true,
                Some(position) => bail!("header.back.position must be \"start\" or \"end\", got {position:?}"),
            },
            back_arrow: match self.header.back.as_ref().and_then(|back| back.icon.as_deref()) {
                None | Some("chevron") => false,
                Some("arrow") => true,
                Some(icon) => bail!("header.back.icon must be \"chevron\" or \"arrow\", got {icon:?}"),
            },
            toggle,
            selection: match self.selection.as_deref() {
                None | Some("underline") => SelectionStyle::Underline,
                Some("check") => SelectionStyle::Check,
                Some(style) => bail!("selection must be \"underline\" or \"check\", got {style:?}"),
            },
            field: match self.field.as_deref() {
                None | Some("stacked") => FieldLayout::Stacked,
                Some("inline") => FieldLayout::Inline,
                Some(layout) => bail!("field must be \"stacked\" or \"inline\", got {layout:?}"),
            },
            section_uppercase: self.section.uppercase.unwrap_or(true),
            section_size: self.section.size.map(units),
            section_gap: self.section.gap.map(units),
            subtitle_size: self.subtitle.size.map(units),
            subtitle_muted: match self.subtitle.tone.as_deref() {
                None => None,
                Some("muted") => Some(true),
                Some("foreground") => Some(false),
                Some(tone) => bail!("subtitle.tone must be \"foreground\" or \"muted\", got {tone:?}"),
            },
            scroll_bar_visibility: match self.scroll_bar.visibility.as_deref() {
                None | Some("always") => ScrollBarVisibility::Always,
                Some("scrolling") => ScrollBarVisibility::Scrolling,
                Some(visibility) => bail!("scrollBar.visibility must be \"always\" or \"scrolling\", got {visibility:?}"),
            },
            scroll_bar_track: self.scroll_bar.track.unwrap_or(true),
            scroll_bar_width: self.scroll_bar.width.map(units),
            scroll_bar_inset: self.scroll_bar.inset.map(units),
        };
        Ok((theme, self.variants))
    }
}

fn resolve_roles(source: &SourceRoles, fallback: Roles) -> Result<Roles> {
    let colour = |value: &Option<String>, fallback| value.as_deref().map_or(Ok(fallback), parse_colour);
    Ok(Roles {
        background: colour(&source.background, fallback.background)?,
        foreground: colour(&source.foreground, fallback.foreground)?,
        muted: colour(&source.muted, fallback.muted)?,
        scroll_bar: fallback.scroll_bar,
    })
}

/// Parses `#rgb`, `#rrggbb` or `#rrggbbaa` into the linear colour the scene uses.
pub fn parse_colour(value: &str) -> Result<Colour> {
    parse_srgb(value).map(Colour::to_linear)
}

fn parse_srgb(value: &str) -> Result<Colour> {
    let hex = value.strip_prefix('#').with_context(|| format!("colour {value:?} must start with #"))?;
    let digits: Vec<u8> = match hex.len() {
        3 => hex.chars().flat_map(|digit| [digit, digit]).collect::<String>().into_bytes(),
        6 | 8 => hex.as_bytes().to_vec(),
        _ => bail!("colour {value:?} must be #rgb, #rrggbb or #rrggbbaa"),
    };
    let channel = |index: usize| -> Result<f32> {
        let text = std::str::from_utf8(&digits[index * 2..index * 2 + 2])?;
        Ok(u8::from_str_radix(text, 16).with_context(|| format!("invalid colour {value:?}"))? as f32 / 255.0)
    };
    Ok(Colour {
        red: channel(0)?,
        green: channel(1)?,
        blue: channel(2)?,
        alpha: if digits.len() == 8 { channel(3)? } else { 1.0 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(source: ThemeSource, blobs: &[Vec<u8>]) -> Result<ThemeBundle> {
        ThemeBundle::decode(&source.encode(blobs)?)
    }

    #[test]
    fn an_empty_theme_resolves_to_the_default_theme() {
        assert_eq!(decode(ThemeSource::default(), &[]).unwrap().theme, Theme::default());
    }

    #[test]
    fn light_colours_are_derived_from_dark_colours_unless_set() {
        let source = ThemeSource {
            colours: Some(SourceColours {
                dark: Some(SourceRoles { foreground: Some("#c8ff00".into()), ..Default::default() }),
                light: Some(SourceRoles { muted: Some("#123456".into()), ..Default::default() }),
            }),
            ..Default::default()
        };
        let palette = decode(source, &[]).unwrap().theme.palette;
        let foreground = parse_srgb("#c8ff00").unwrap().to_linear();
        assert_eq!(palette.dark.foreground, foreground);
        // Derived light colours invert in linear space, like Ink's default light mode.
        assert_eq!(palette.light.foreground, crate::invert_colour(foreground));
        assert_eq!(palette.light.muted, parse_srgb("#123456").unwrap().to_linear());
        assert_eq!(palette.light.background, Colour::WHITE);
    }

    #[test]
    fn default_light_muted_text_is_the_srgb_inverse_of_dark_muted_text() {
        // Dark muted #898989 becomes #767676, which has 4.5:1 contrast on white.
        let expected = parse_colour("#767676").unwrap();
        let muted = Palette::default().light.muted;
        assert!((muted.red - expected.red).abs() < 0.002, "{muted:?}");
        assert!((muted.green - expected.green).abs() < 0.002 && (muted.blue - expected.blue).abs() < 0.002);
    }

    #[test]
    fn pixel_sizes_become_reference_units_at_the_theme_scale() {
        let source = ThemeSource {
            text: SourceText { size: Some(DEFAULT_TEXT_PX / 2.0) },
            spacing: SourceSpacing { inset: Some(51.0), ..Default::default() },
            ..Default::default()
        };
        let theme = decode(source, &[]).unwrap().theme;
        assert_eq!(theme.scale, 0.5);
        // 51 export pixels at half scale: 51 / (2.55 × 0.5) = 40 reference units.
        assert!((theme.inset.unwrap() - 40.0).abs() < 1e-4);
    }

    #[test]
    fn invalid_values_name_the_setting_and_the_choices() {
        let source = ThemeSource { header: SourceHeader { layout: Some("left".into()), ..Default::default() }, ..Default::default() };
        let error = decode(source, &[]).err().unwrap().to_string();
        assert_eq!(error, "header.layout must be \"centred\" or \"large\", got \"left\"");
        assert!(parse_colour("red").is_err());
        assert!(parse_colour("#12345").is_err());
    }

    #[test]
    fn truncated_files_are_rejected() {
        let bytes = ThemeSource::default().encode(&[vec![1, 2, 3]]).unwrap();
        assert!(ThemeBundle::decode(&bytes[..bytes.len() - 1]).is_err());
    }
}
