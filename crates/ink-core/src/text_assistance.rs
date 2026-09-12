use super::*;
use serde_json::{Value, json};

#[derive(Default)]
pub(super) struct Assistance {
    state: Option<StateId>,
    text: String,
    pub ranges: Vec<(usize, usize)>,
    pub selected: Option<usize>,
    pub menu: bool,
    hits: Vec<(usize, Rect, bool)>,
}

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
        let Some(state) = self.focused_input else {
            return String::new();
        };
        let Some(StateValue::String(text)) = self.state.get(state.0) else {
            return String::new();
        };
        let (_, auto_correct, spell_check) = self.text_input_options(state).unwrap_or_default();
        json!({
            "id": state.0,
            "text": text,
            "cursor": text[..text_cursor_boundary(text, self.focused_input_cursor)].encode_utf16().count(),
            "autoCorrect": auto_correct,
            "spellCheck": spell_check,
            "menu": self.assistance.menu,
            "selected": self.assistance.selected.map(|offset| text[..text_cursor_boundary(text, offset)].encode_utf16().count()),
        }).to_string()
    }

    pub(super) fn apply_text_assistance(&mut self, payload: &str) -> bool {
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
        if data["dismiss"].as_bool() == Some(true) {
            self.assistance.selected = None;
            self.assistance.menu = false;
            return false;
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
            self.assistance = Assistance::default();
            self.reveal_text_cursor(state);
        } else if let Some(replacement) = data["replacement"].as_str() {
            let Some(cursor) = data["cursor"]
                .as_u64()
                .and_then(|n| byte_offset(text, n as usize))
            else {
                return false;
            };
            if cursor != self.focused_input_cursor {
                return false;
            }
            let Some(start) = data["start"]
                .as_u64()
                .and_then(|n| byte_offset(text, n as usize))
            else {
                return false;
            };
            let Some(end) = data["end"]
                .as_u64()
                .and_then(|n| byte_offset(text, n as usize))
            else {
                return false;
            };
            if start > end
                || replacement.chars().any(|ch| {
                    ch.is_control()
                        && !(ch == '\n' && self.focused_input_action == TextInputAction::Return)
                })
                || (self.text_input_numeric()
                    && !replacement.bytes().all(|byte| byte.is_ascii_digit()))
            {
                return false;
            }
            let mut updated = text.clone();
            updated.replace_range(start..end, replacement);
            self.focused_input_cursor = if cursor >= end {
                cursor - (end - start) + replacement.len()
            } else if cursor > start {
                start + replacement.len()
            } else {
                cursor
            };
            self.state[state.0] = StateValue::String(updated);
            self.assistance.selected = None;
            self.assistance.menu = false;
            self.rebase_spelling(state);
            self.reveal_text_cursor(state);
        } else {
            self.assistance.state = Some(state);
            self.assistance.text = text.clone();
            self.assistance.ranges = data["ranges"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|range| {
                    let start = byte_offset(text, range[0].as_u64()? as usize)?;
                    let end = byte_offset(text, range[1].as_u64()? as usize)?;
                    (start < end).then_some((start, end))
                })
                .collect();
            self.assistance.selected = self.assistance.selected.filter(|selected| {
                self.assistance
                    .ranges
                    .iter()
                    .any(|(start, _)| start == selected)
            });
        }
        self.relayout_scene();
        true
    }

    pub(super) fn select_spelling(&mut self, x: f32, y: f32) -> bool {
        let Some(state) = self.focused_input else {
            return false;
        };
        let Some(StateValue::String(text)) = self.state.get(state.0) else {
            return false;
        };
        if self.assistance.state != Some(state) || self.assistance.text != *text {
            return false;
        }
        self.assistance.selected = self
            .assistance
            .hits
            .iter()
            .find(|(_, rect, scrolling)| {
                rect.contains(
                    x,
                    if *scrolling {
                        y + self.scroll_offset - self.scroll_origin
                    } else {
                        y
                    },
                )
            })
            .map(|(start, _, _)| *start);
        self.assistance.selected.is_some()
    }

    pub(super) fn rebase_spelling(&mut self, state: StateId) {
        if self.assistance.state != Some(state) {
            return;
        }
        let Some(StateValue::String(text)) = self.state.get(state.0) else {
            return;
        };
        let previous = &self.assistance.text;
        if previous == text {
            return;
        }
        let prefix = previous
            .chars()
            .zip(text.chars())
            .take_while(|(a, b)| a == b)
            .map(|(ch, _)| ch.len_utf8())
            .sum::<usize>();
        let suffix = previous[prefix..]
            .chars()
            .rev()
            .zip(text[prefix..].chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(ch, _)| ch.len_utf8())
            .sum::<usize>();
        let old_end = previous.len() - suffix;
        let new_end = text.len() - suffix;
        self.assistance.ranges.retain_mut(|(start, end)| {
            if *end < prefix {
                return true;
            }
            if *start > old_end {
                *start = *start - old_end + new_end;
                *end = *end - old_end + new_end;
                return true;
            }
            false
        });
        self.assistance.text = text.clone();
    }

    pub(super) fn layout_spelling(&mut self) {
        self.assistance.hits.clear();
        let Some(state) = self.assistance.state else {
            return;
        };
        let Some(StateValue::String(text)) = self.state.get(state.0) else {
            return;
        };
        if *text != self.assistance.text
            || !self
                .text_input_options(state)
                .is_some_and(|(_, _, check)| check)
        {
            return;
        }
        let Some(input) = self
            .text_inputs
            .iter()
            .find(|input| input.state == state)
            .copied()
        else {
            return;
        };
        let lines = if input.action == TextInputAction::Return {
            self.input_lines(text, input.text_rect.width)
        } else {
            vec![(0, text.len())]
        };
        let font_size = self.scaled_font(TEXT_INPUT_TEXT_SIZE);
        let dot = self.scaled(1.0);
        for (index, (line_start, line_end)) in lines.into_iter().enumerate() {
            let run = &self.scene.text[input.text_run + index];
            for &(start, end) in &self.assistance.ranges {
                let word_start = start;
                let start = start.max(line_start);
                let end = end.min(line_end);
                if start >= end {
                    continue;
                }
                let left = run.rect.x + self.text_width(&text[line_start..start], font_size);
                let right = run.rect.x + self.text_width(&text[line_start..end], font_size);
                self.assistance.hits.push((
                    word_start,
                    Rect {
                        x: left,
                        width: right - left,
                        ..run.rect
                    }
                    .intersection(run.clip),
                    run.scrolling,
                ));
                let mut x = left;
                while x < right {
                    self.scene.quads.push(Quad {
                        rect: Rect {
                            x,
                            y: run.rect.y + run.rect.height - dot * 2.0,
                            width: dot.min(right - x),
                            height: dot,
                        },
                        clip: run.clip,
                        colour: self.scene.colour(Colour::WHITE),
                        scrolling: run.scrolling,
                    });
                    x += dot * 3.0;
                }
            }
        }
    }
}
