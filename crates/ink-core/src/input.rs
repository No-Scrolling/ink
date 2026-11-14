use super::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct InputColour {
    name: String,
    colour: Option<Colour>,
}

impl TryFrom<String> for InputColour {
    type Error = anyhow::Error;
    fn try_from(name: String) -> Result<Self> {
        let colour = match name.as_str() {
            "foreground" | "background" | "muted" => None,
            _ => Some(parse_colour(&name)?),
        };
        Ok(Self { name, colour })
    }
}

impl From<InputColour> for String {
    fn from(value: InputColour) -> Self { value.name }
}

impl InputColour {
    pub(super) fn resolve(&self, scene: &Scene) -> Colour {
        self.colour.map_or_else(|| scene.colour(match self.name.as_str() {
            "background" => Colour::BLACK,
            "muted" => Colour::MUTED,
            _ => Colour::WHITE,
        }), |colour| scene.explicit_colour(colour))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputBorder { Underline, Outline, None }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputFont { Body, Display, Input }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputAlign { Start, Center, End }

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputStyle {
    pub border: Option<InputBorder>,
    pub border_width: Option<f32>,
    pub border_colour: Option<InputColour>,
    pub radius: Option<f32>,
    pub background: Option<InputColour>,
    pub padding_x: Option<f32>,
    pub padding_y: Option<f32>,
    /// Override `padding_y` above or below the text.
    pub padding_top: Option<f32>,
    pub padding_bottom: Option<f32>,
    pub min_height: Option<f32>,
    pub font_size: Option<f32>,
    pub font: Option<InputFont>,
    pub align: Option<InputAlign>,
    pub text_colour: Option<InputColour>,
    pub placeholder_colour: Option<InputColour>,
    pub affix_colour: Option<InputColour>,
    pub cursor_colour: Option<InputColour>,
    pub selection_colour: Option<InputColour>,
    pub clear_colour: Option<InputColour>,
    pub clear_size: Option<f32>,
    pub clear_padding: Option<f32>,
    pub clear_gap: Option<f32>,
}

impl InputStyle {
    pub(super) fn merge(&mut self, other: &Self) {
        macro_rules! fields { ($($field:ident),*) => { $(if other.$field.is_some() { self.$field = other.$field.clone(); })* }; }
        fields!(border, border_width, border_colour, radius, background, padding_x, padding_y,
            padding_top, padding_bottom, min_height, font_size, font, align, text_colour, placeholder_colour, affix_colour,
            cursor_colour, selection_colour, clear_colour, clear_size, clear_padding, clear_gap);
    }

    fn scale(&mut self, divisor: f32) -> Result<()> {
        for (name, value) in [
            ("borderWidth", &mut self.border_width), ("radius", &mut self.radius),
            ("paddingX", &mut self.padding_x), ("paddingY", &mut self.padding_y),
            ("paddingTop", &mut self.padding_top), ("paddingBottom", &mut self.padding_bottom),
            ("minHeight", &mut self.min_height), ("fontSize", &mut self.font_size),
            ("clearSize", &mut self.clear_size), ("clearPadding", &mut self.clear_padding), ("clearGap", &mut self.clear_gap),
        ] {
            if let Some(value) = value {
                ensure!(value.is_finite() && *value >= 0.0 && *value <= 10000.0, "textInput.{name} must be between 0 and 10000");
                ensure!(!matches!(name, "fontSize" | "clearSize") || *value > 0.0, "textInput.{name} must be positive");
                *value /= divisor;
            }
        }
        Ok(())
    }

    pub(super) fn font(&self, theme: &Theme) -> FontRole {
        match self.font.unwrap_or(InputFont::Input) {
            InputFont::Body => theme.body_font,
            InputFont::Display => theme.display_font,
            InputFont::Input => theme.input_font,
        }
    }

    pub(super) fn size(&self) -> f32 { self.font_size.unwrap_or(TEXT_INPUT_TEXT_SIZE) }
    pub(super) fn line_height(&self) -> f32 { self.size() * (TEXT_INPUT_HEIGHT - TEXT_INPUT_BOTTOM_PADDING) / TEXT_INPUT_TEXT_SIZE }
    pub(super) fn bottom_padding(&self) -> f32 {
        if self.padding_y.is_some() || self.padding_top.is_some() || self.padding_bottom.is_some() { 0.0 } else { TEXT_INPUT_BOTTOM_PADDING }
    }
    pub(super) fn pad_top(&self) -> f32 { self.padding_top.or(self.padding_y).unwrap_or(0.0) }
    pub(super) fn pad_bottom(&self) -> f32 { self.padding_bottom.or(self.padding_y).unwrap_or(0.0) }
    pub(super) fn colour(&self, scene: &Scene, paint: &Option<InputColour>, fallback: Colour) -> Colour {
        paint.as_ref().map_or_else(|| scene.colour(fallback), |paint| paint.resolve(scene))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputAppearance {
    #[serde(flatten)]
    pub style: InputStyle,
    pub focused: Option<InputStyle>,
    pub invalid: Option<InputStyle>,
    pub disabled: Option<InputStyle>,
    pub read_only: Option<InputStyle>,
}

impl InputAppearance {
    pub fn in_units(mut self, divisor: f32) -> Result<Self> {
        self.style.scale(divisor)?;
        for style in [&mut self.focused, &mut self.invalid, &mut self.disabled, &mut self.read_only].into_iter().flatten() {
            style.scale(divisor)?;
        }
        Ok(self)
    }

    pub(super) fn resolve(&self, focused: bool, options: &InputOptions) -> InputStyle {
        let custom = &options.appearance;
        let mut style = self.style.clone();
        style.merge(&custom.style);
        for (active, theme, custom) in [
            (focused, &self.focused, &custom.focused),
            (options.invalid, &self.invalid, &custom.invalid),
            (options.read_only, &self.read_only, &custom.read_only),
            (options.disabled, &self.disabled, &custom.disabled),
        ] {
            if active {
                if let Some(theme) = theme { style.merge(theme); }
                if let Some(custom) = custom { style.merge(custom); }
            }
        }
        style
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct InputOptions {
    pub appearance: InputAppearance,
    pub disabled: bool,
    pub read_only: bool,
    pub auto_capitalise: bool,
    /// Taps outside the input leave it focused; only Back or a submit removes focus.
    pub keep_focus: bool,
    pub invalid: bool,
    pub clearable: bool,
    pub min_lines: usize,
    pub max_lines: usize,
    pub leading_icon: Option<Mask>,
    pub label: String,
    pub description: String,
    pub error: String,
}

impl Default for InputOptions {
    fn default() -> Self {
        Self { appearance: InputAppearance::default(), disabled: false, read_only: false, auto_capitalise: true, keep_focus: false, invalid: false,
            clearable: true, min_lines: 1, max_lines: TEXT_INPUT_MAX_LINES, leading_icon: None,
            label: String::new(), description: String::new(), error: String::new() }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HitInsets { pub top: f32, pub right: f32, pub bottom: f32, pub left: f32 }

impl HitInsets {
    pub fn parse(value: &serde_json::Value) -> Result<Self> {
        let value = if let Some(value) = value.as_f64() {
            Self { top: value as f32, right: value as f32, bottom: value as f32, left: value as f32 }
        } else { serde_json::from_value(value.clone())? };
        ensure!([value.top, value.right, value.bottom, value.left].iter().all(|v| v.is_finite() && v.abs() <= 10000.0), "hitSlop sides must be finite numbers between -10000 and 10000");
        Ok(value)
    }
    pub fn in_units(self, divisor: f32) -> Self {
        Self { top: self.top / divisor, right: self.right / divisor, bottom: self.bottom / divisor, left: self.left / divisor }
    }
    pub(super) fn expand(self, rect: Rect, scale: f32) -> Rect {
        Rect { x: rect.x - self.left * scale, y: rect.y - self.top * scale,
            width: (rect.width + (self.left + self.right) * scale).max(0.0),
            height: (rect.height + (self.top + self.bottom) * scale).max(0.0) }
    }
}

pub(super) fn input_alignment_offset(style: &InputStyle, width: f32, text_width: f32) -> f32 {
    let space = (width - text_width).max(0.0);
    match style.align.unwrap_or(InputAlign::Start) { InputAlign::Start => 0.0, InputAlign::Center => space / 2.0, InputAlign::End => space }
}

impl Engine {
    pub fn set_debug_hitboxes(&mut self, enabled: bool) -> bool {
        if self.debug_hitboxes == enabled { return false; }
        self.debug_hitboxes = enabled;
        self.relayout_scene();
        true
    }
    pub(super) fn measure_input(&self, state: StateId, action: TextInputAction, prefix: &str, suffix: &str, options: &InputOptions, available: Rect) -> MeasuredSize {
        let style = self.theme.text_input.resolve(self.focused_input == Some(state), options);
        let padding = self.scaled(style.padding_x.unwrap_or(0.0));
        let width = (available.width - padding * 2.0).max(0.0);
        let (prefix_width, suffix_width) = self.input_affix_widths(prefix, suffix, width, &style);
        let icon_width = if options.leading_icon.is_some() { self.scaled(style.size() + TEXT_INPUT_AFFIX_GAP) } else { 0.0 };
        let value = match self.state.get(state.0) { Some(StateValue::String(value)) => value.as_str(), _ => "" };
        let lines = if action == TextInputAction::Return {
            self.native_editor_lines.as_ref().filter(|(id, text, _)| *id == state && text == value)
                .map(|(_, _, lines)| *lines)
                .unwrap_or_else(|| self.input_lines(value, (width - prefix_width - suffix_width - icon_width - self.scaled(1.0)).max(0.0), &style).len())
                .clamp(options.min_lines, options.max_lines)
        } else { 1 };
        let line_height = self.scaled(style.line_height());
        let line_height = if self.native_editor_state == Some(state) { line_height.round() } else { line_height };
        let height = line_height * lines as f32 + self.scaled(style.bottom_padding() + style.pad_top() + style.pad_bottom());
        MeasuredSize { width: available.width, height: height.max(self.scaled(style.min_height.unwrap_or(0.0))).min(available.height) }
    }

    pub(super) fn input_lines(&self, value: &str, width: f32, style: &InputStyle) -> Vec<(usize, usize)> {
        let role = style.font(&self.theme);
        let font_size = self.scaled_font_in(style.size(), role);
        let mut lines = Vec::new();
        let mut offset = 0;
        for paragraph in value.split('\n') {
            let breakpoints = linebreaks(paragraph).map(|(index, _)| index).collect::<Vec<_>>();
            let mut start = 0;
            if paragraph.is_empty() { lines.push((offset, offset)); }
            while start < paragraph.len() {
                let end = breakpoints.iter().copied().filter(|end| *end > start)
                    .take_while(|end| text_width_in(&paragraph[start..*end], font_size, false, role) <= width)
                    .last().unwrap_or_else(|| Self::forced_text_break(paragraph, start, font_size, width, false, role));
                lines.push((offset + start, offset + end));
                start = end;
            }
            offset += paragraph.len() + 1;
        }
        lines
    }

    fn input_affix_widths(&self, prefix: &str, suffix: &str, width: f32, style: &InputStyle) -> (f32, f32) {
        let role = style.font(&self.theme);
        let font_size = self.scaled_font_in(style.font_size.unwrap_or(TEXT_INPUT_AFFIX_SIZE), role);
        let measure = |text: &str| if text.is_empty() { 0.0 } else {
            (text_width_in(text, font_size, false, role) + self.scaled(TEXT_INPUT_AFFIX_GAP)).min(width / 3.0)
        };
        (measure(prefix), measure(suffix))
    }

    fn input_surface_mask(&mut self, rect: Rect, radius: f32, border: f32, colour: Colour) {
        let width = rect.width.round().max(1.0) as u32;
        let height = rect.height.round().max(1.0) as u32;
        let key = (width, height, radius.to_bits(), border.to_bits());
        if !self.input_masks.contains_key(&key) {
            if self.input_masks.len() >= 64 { self.input_masks.clear(); }
            let mask = if border > 0.0 { masks::outlined_rect(width, height, radius, border) }
                else { masks::rounded_rect(width, height, radius) };
            self.input_masks.insert(key, mask);
        }
        self.scene.background_masks.push(MaskRun { mask: self.input_masks[&key].clone(), rect,
            clip: self.clip, colour, scrolling: self.scrolling });
    }

    fn input_surface(&mut self, rect: Rect, style: &InputStyle) {
        let radius = self.scaled(style.radius.unwrap_or(0.0)).min(rect.width.min(rect.height) / 2.0);
        if let Some(background) = &style.background {
            self.input_surface_mask(rect, radius, 0.0, background.resolve(&self.scene));
        }
        let border = self.scaled(style.border_width.unwrap_or(CONTROL_LINE_HEIGHT)).min(rect.width.min(rect.height) / 2.0);
        if border <= 0.0 { return; }
        let colour = style.colour(&self.scene, &style.border_colour, Colour::WHITE);
        match style.border.unwrap_or(InputBorder::Underline) {
            InputBorder::None => {},
            InputBorder::Outline => self.input_surface_mask(rect, radius, border, colour),
            InputBorder::Underline => {
                let rect = Rect { y: (rect.y + rect.height).round() - border, height: border, ..rect };
                if style.background.is_some() {
                    self.scene.background_masks.push(MaskRun { mask: Mask::solid(), rect, clip: self.clip, colour, scrolling: self.scrolling });
                } else {
                    self.scene.quads.push(Quad { rect, clip: self.clip, colour, scrolling: self.scrolling });
                }
            }
        }
    }

    pub(super) fn layout_text_input(&mut self, placeholder: &str, state: StateId, action: TextInputAction,
        prefix: &str, suffix: &str, clear: &Mask, options: &InputOptions, rect: Rect, trailing_width: f32) {
        if !cfg!(feature = "text-input") { return; }
        let style = self.theme.text_input.resolve(self.focused_input == Some(state), options);
        self.input_surface(rect, &style);
        let padding_x = self.scaled(style.padding_x.unwrap_or(0.0)).min(rect.width / 2.0);
        let padding_top = self.scaled(style.pad_top()).min(rect.height / 2.0);
        let padding_bottom = self.scaled(style.pad_bottom()).min(rect.height / 2.0);
        let inner = Rect { x: rect.x + padding_x, y: rect.y + padding_top,
            width: (rect.width - padding_x * 2.0).max(0.0), height: (rect.height - padding_top - padding_bottom - self.scaled(style.bottom_padding())).max(0.0) };
        let role = style.font(&self.theme);
        let font_size = self.scaled_font_in(style.size(), role);
        let line_height = self.scaled(style.line_height());
        let multiline = action == TextInputAction::Return;
        let text_height = if multiline { inner.height } else { line_height.min(inner.height) };
        let inner = Rect { y: inner.y + if multiline { 0.0 } else { (inner.height - text_height) / 2.0 }, height: text_height, ..inner };
        let icon_width = if let Some(icon) = &options.leading_icon {
            let size = self.scaled(style.size());
            self.scene.masks.push(MaskRun { mask: icon.clone(),
                rect: Rect { y: inner.y + (line_height - size) / 2.0, width: size, height: size, ..inner }, clip: inner.intersection(self.clip),
                colour: style.colour(&self.scene, &style.affix_colour, Colour::WHITE), scrolling: self.scrolling });
            self.scaled(style.size() + TEXT_INPUT_AFFIX_GAP)
        } else { 0.0 };
        let inner = Rect { x: inner.x + icon_width, width: (inner.width - icon_width).max(0.0), ..inner };
        let (prefix_width, suffix_width) = self.input_affix_widths(prefix, suffix, (inner.width - trailing_width).max(0.0), &style);
        let gap = self.scaled(TEXT_INPUT_AFFIX_GAP);
        for (text, x, width) in [(prefix, inner.x, prefix_width), (suffix, inner.x + inner.width - trailing_width - suffix_width + gap, suffix_width)] {
            if text.is_empty() { continue; }
            let bounds = Rect { x, width: (width - gap).max(0.0), height: line_height, ..inner };
            self.scene.text.push(TextRun { font: role, tabular_numbers: false, text: text.into(), rect: bounds,
                clip: bounds.intersection(self.clip), font_size: self.scaled_font_in(style.font_size.unwrap_or(TEXT_INPUT_AFFIX_SIZE), role),
                colour: style.colour(&self.scene, &style.affix_colour, Colour::WHITE), align: TextAlign::Start, scrolling: self.scrolling });
        }
        let value = match self.state.get(state.0) { Some(StateValue::String(value)) => value.clone(), _ => String::new() };
        let editable = !options.disabled && !options.read_only;
        let clearable = editable && options.clearable && !multiline && !value.is_empty() && trailing_width == 0.0;
        let clear_size = self.scaled(style.clear_size.unwrap_or(TEXT_INPUT_CLEAR_ICON_SIZE));
        let clear_padding = self.scaled(style.clear_padding.unwrap_or(TEXT_INPUT_CLEAR_PADDING));
        let clear_width = if clearable { clear_size + clear_padding * 2.0 } else { 0.0 };
        let clear_gap = if clearable { self.scaled(style.clear_gap.unwrap_or(TEXT_INPUT_CLEAR_GAP)) } else { 0.0 };
        let viewport = Rect { x: inner.x + prefix_width,
            width: (inner.width - prefix_width - suffix_width - trailing_width - clear_width - clear_gap - self.scaled(1.0)).max(0.0), ..inner };
        let focused = self.focused_input == Some(state);
        let text = if value.is_empty() { placeholder } else { &value };
        let lines = if multiline { self.input_lines(&value, viewport.width, &style) } else { vec![(0, value.len())] };
        let cursor = if focused { self.focused_input_cursor } else { value.len() };
        let cursor_line = lines.iter().rposition(|(start, _)| *start <= cursor).unwrap_or(0);
        let text_width = text_width_in(text, font_size, false, role);
        let scroll_max = if multiline { (lines.len() as f32 * line_height - viewport.height).max(0.0) }
            else if value.is_empty() { 0.0 } else { (text_width - viewport.width).max(0.0) };
        let default_scroll = if multiline { (cursor_line + 1) as f32 * line_height - viewport.height } else { scroll_max };
        let scroll_offset = self.text_input_scroll_offsets.get(&state).copied().unwrap_or(default_scroll).clamp(0.0, scroll_max);
        let text_run = self.scene.text.len();
        let align = match style.align.unwrap_or(InputAlign::Start) { InputAlign::Start => TextAlign::Start, InputAlign::Center => TextAlign::Centre, InputAlign::End => TextAlign::End };
        let colour = if value.is_empty() { style.colour(&self.scene, &style.placeholder_colour, Colour::MUTED) }
            else { style.colour(&self.scene, &style.text_colour, if options.disabled { Colour::MUTED } else { Colour::WHITE }) };
        for (index, (start, end)) in lines.iter().copied().enumerate() {
            self.scene.text.push(TextRun { font: role, tabular_numbers: false,
                text: if self.native_editor_state == Some(state) { "" } else if value.is_empty() { placeholder } else { &value[start..end] }.into(),
                rect: if multiline { Rect { y: (viewport.y + index as f32 * line_height - scroll_offset + (line_height - font_size) / 2.0).round(), height: line_height, ..viewport } }
                    else { Rect { x: viewport.x - scroll_offset, y: viewport.y.round(), width: text_width.max(viewport.width), ..viewport } },
                clip: viewport.intersection(self.clip), font_size, colour, align, scrolling: self.scrolling });
        }
        if editable {
            let focus = Action::FocusTextInput { state, action };
            let hit_rect = self.hit_rect(Rect { width: (rect.width - suffix_width - trailing_width - clear_width).max(0.0), ..rect }, &focus);
            self.text_inputs.push(TextInputLayout { node: self.layout_owner, state, action, text_run, rect, text_rect: viewport,
                hit_rect, scroll_offset, scroll_max, scrolling: self.scrolling });
            self.push_hit_region(Rect { width: (rect.width - trailing_width).max(0.0), ..rect }, focus);
        }
        if clearable {
            let clear_rect = Rect { x: inner.x + inner.width - suffix_width - clear_width, y: rect.y, width: clear_width, height: rect.height };
            self.scene.masks.push(MaskRun { mask: clear.clone(),
                rect: Rect { x: clear_rect.x + clear_padding, y: inner.y + (line_height - clear_size) / 2.0, width: clear_size, height: clear_size },
                clip: rect.intersection(self.clip), colour: style.colour(&self.scene, &style.clear_colour, Colour::WHITE), scrolling: self.scrolling });
            self.push_hit_region(clear_rect, Action::ClearInput { state });
        }
    }
}
