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
        let y = rect.y + if input.scrolling { self.scroll_origin - self.scroll_offset } else { 0.0 };
        let font = self.font.as_scaled(PxScale::from(self.scaled_font(TEXT_INPUT_TEXT_SIZE)));
        json!({
            "id": state.0,
            "text": text,
            "cursor": text[..text_cursor_boundary(text, self.focused_input_cursor)].encode_utf16().count(),
            "nativeEditor": self.native_editor_state == Some(state),
            "editor": {
                "x": rect.x, "y": y, "width": rect.width, "height": rect.height,
                "fontSize": self.scaled_font(TEXT_INPUT_TEXT_SIZE)
                    * self.font.units_per_em().unwrap_or(self.font.height_unscaled()) / self.font.height_unscaled(),
                "lineHeight": self.scaled(TEXT_INPUT_HEIGHT - TEXT_INPUT_BOTTOM_PADDING),
                "baseline": (rect.height - font.height()) / 2.0 + font.ascent(),
                "bottomPadding": self.scaled(TEXT_INPUT_BOTTOM_PADDING),
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
            self.native_editor_lines = Some((state, data["value"].as_str().unwrap_or(text).to_owned(), (lines as usize).clamp(1, TEXT_INPUT_MAX_LINES)));
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
