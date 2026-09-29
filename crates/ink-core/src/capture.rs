use super::*;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaptureKind { Recording, Level, Pitch }

impl CaptureKind {
    pub(super) fn size(self) -> f32 { if self == Self::Recording { DEFAULT_TEXT_SIZE } else { 18.0 } }
}

#[derive(Clone, PartialEq)]
struct Reading { kind: CaptureKind, text: String }

#[derive(Clone, Default)]
pub(super) struct CaptureReadings {
    values: HashMap<u64, Reading>,
    runs: Vec<(u64, CaptureKind, usize)>,
}

impl Engine {
    pub fn update_capture_state(&mut self, controller: u64, value: &Value) -> bool {
        let number = |key| value.get(key).and_then(Value::as_f64).filter(|value| value.is_finite()).unwrap_or(0.0);
        let active = value.get("status").and_then(Value::as_str) == Some("active");
        let reading = if value.get("durationMs").is_some() {
            Reading { kind: CaptureKind::Recording, text: playback_time((number("durationMs").max(0.0) / 1000.0) as u64) }
        } else if value.get("rms").is_some() {
            Reading { kind: CaptureKind::Level, text: format!("RMS {:.3} · peak {:.3}", number("rms"), number("peak")) }
        } else {
            Reading { kind: CaptureKind::Pitch, text: if active {
                format!("{}{} · {:.1} Hz · {:+.0} cents", value.get("note").and_then(Value::as_str).unwrap_or(""),
                    number("octave") as i32, number("frequencyHz"), number("cents"))
            } else { "—".into() } }
        };
        if self.capture_readings.values.get(&controller) == Some(&reading) { return false; }
        let mut changed = false;
        for &(id, kind, run) in &self.capture_readings.runs {
            if id != controller || kind != reading.kind { continue; }
            let text = &mut self.scene.text[run];
            let next = Self::capture_text(&reading.text, text.font_size, text.rect.width);
            if text.text.as_str() != next {
                text.text = next.into();
                changed = true;
            }
        }
        self.capture_readings.values.insert(controller, reading);
        if changed { self.scene.revision = self.scene.revision.wrapping_add(1); }
        changed
    }

    pub fn remove_capture_state(&mut self, controller: u64) {
        self.capture_readings.values.remove(&controller);
        self.capture_readings.runs.retain(|(id, _, _)| *id != controller);
    }

    pub(super) fn clear_capture_runs(&mut self) { self.capture_readings.runs.clear(); }

    fn capture_text(text: &str, size: f32, width: f32) -> String {
        if text_width_with_numbers(text, size, true) <= width { text.to_owned() }
        else { Self::ellipsize_forced(text, size, width, true) }
    }

    pub(super) fn layout_capture(&mut self, controller: u64, kind: CaptureKind, rect: Rect) {
        let text = self.capture_readings.values.get(&controller).filter(|reading| reading.kind == kind).map(|reading| reading.text.as_str()).unwrap_or("—");
        let run = self.scene.text.len();
        let size = self.scaled_font(kind.size());
        self.scene.text.push(TextRun { text: Self::capture_text(text, size, rect.width).into(),
            rect, clip: self.clip, font_size: size, colour: self.scene.colour(Colour::WHITE),
            align: TextAlign::Start, tabular_numbers: true, scrolling: self.scrolling });
        if rect.intersection(self.clip).height > 0.0 { self.capture_readings.runs.push((controller, kind, run)); }
    }
}
