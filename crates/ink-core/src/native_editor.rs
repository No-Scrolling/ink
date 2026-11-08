use super::*;
use serde_json::{Value, json};

fn byte_offset(text: &str, utf16: usize) -> Option<usize> {
    let mut units = 0;
    for (index, ch) in text.char_indices() {
        if units == utf16 {
            return Some(index);
        }
        units += ch.len_utf16();
    }
    (units == utf16).then_some(text.len())
}

impl Engine {
    pub fn text_input_context(&self) -> String {
        let Some(state) = self.focused_input.or(self.native_editor_state) else {
            return String::new();
        };
        let Some(input) = self.text_inputs.iter().find(|input| input.state == state) else {
            return String::new();
        };
        let Some(StateValue::String(text)) = self.state.get(state.0) else {
            return String::new();
        };
        let rect = input.text_rect;
        let Some(Node { kind: NodeKind::TextInput { placeholder, options, .. }, .. }) = self.text_input_node(state) else {
            return String::new();
        };
        let y = rect.y + if input.scrolling { self.scroll_origin - self.scroll_offset } else { 0.0 };
        let style = self.theme.text_input.resolve(self.focused_input == Some(state), options);
        let role = style.font(&self.theme);
        let font_size = self.scaled_font_in(style.size(), role).round();
        let body = primary_font(role);
        let font = body.as_scaled(PxScale::from(font_size));
        let hint = style.colour(&self.scene, &style.placeholder_colour, Colour::MUTED);
        let srgb = |linear: f32| {
            let value = if linear <= 0.0031308 { linear * 12.92 } else { 1.055 * linear.powf(1.0 / 2.4) - 0.055 };
            (value * 255.0).round() as u32
        };
        let argb = |colour: Colour| ((colour.alpha * 255.0).round() as u32) << 24 | srgb(colour.red) << 16 | srgb(colour.green) << 8 | srgb(colour.blue);
        let hint_colour = argb(hint);
        let text_colour = argb(style.colour(&self.scene, &style.text_colour, Colour::WHITE));
        json!({
            "id": state.0,
            "text": text,
            "placeholder": placeholder,
            "hintColour": hint_colour as i32,
            "textColour": text_colour as i32,
            "fontSlot": role.index().map_or(-1, i32::from),
            "cursorColour": argb(style.colour(&self.scene, &style.cursor_colour, Colour::WHITE)) as i32,
            "selectionColour": style.selection_colour.as_ref().map_or(0x66888888u32, |paint| argb(paint.resolve(&self.scene))) as i32,
            "align": match style.align.unwrap_or(input::InputAlign::Start) { input::InputAlign::Start => "start", input::InputAlign::Center => "center", input::InputAlign::End => "end" },
            "minLines": options.min_lines,
            "maxLines": options.max_lines,
            "autoCapitalise": options.auto_capitalise,
            "label": options.label,
            "description": options.description,
            "error": options.error,
            "cursor": text[..text_cursor_boundary(text, self.focused_input_cursor)].encode_utf16().count(),
            "nativeEditor": self.native_editor_state == Some(state),
            "editor": {
                "x": rect.x, "y": y, "width": rect.width, "height": rect.height,
                "fontSize": font_size
                    * body.units_per_em().unwrap_or(body.height_unscaled()) / body.height_unscaled(),
                "lineHeight": self.scaled(style.line_height()),
                "baseline": (rect.height - font.height()) / 2.0 + font.ascent(),
                "bottomPadding": self.scaled(style.bottom_padding()),
            },
        }).to_string()
    }

    pub(super) fn apply_editor_update(&mut self, payload: &str) -> bool {
        let Ok(data) = serde_json::from_str::<Value>(payload) else {
            return false;
        };
        let Some(state) = self.focused_input else {
            return false;
        };
        let Some(StateValue::String(text)) = self.state.get(state.0) else {
            return false;
        };
        if data["id"].as_u64() != Some(state.0 as u64) || data["text"].as_str() != Some(text) {
            return false;
        }
        if data["nativeEditor"].as_bool() == Some(true) {
            self.native_editor_state = Some(state);
        }
        if let Some(lines) = data["lines"].as_u64() {
            let max = match self.text_input_node(state).map(|node| &node.kind) {
                Some(NodeKind::TextInput { options, .. }) => options.max_lines,
                _ => TEXT_INPUT_MAX_LINES,
            };
            self.native_editor_lines = Some((state, data["value"].as_str().unwrap_or(text).to_owned(), (lines as usize).clamp(1, max)));
        }
        if let Some(value) = data["value"].as_str() {
            let Some(cursor) = data["selection"]
                .as_u64()
                .and_then(|n| byte_offset(value, n as usize))
            else {
                return false;
            };
            if value.chars().any(|ch| {
                ch.is_control()
                    && !(ch == '\n' && self.focused_input_action == TextInputAction::Return)
            }) || (self.text_input_numeric() && !value.bytes().all(|byte| byte.is_ascii_digit()))
            {
                return false;
            }
            self.focused_input_cursor = cursor;
            self.state[state.0] = StateValue::String(value.to_owned());
        }
        self.relayout_scene();
        true
    }
}
