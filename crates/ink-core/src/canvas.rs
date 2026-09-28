use super::{
    Action, Colour, Engine, Mask, MaskRun, NodeIdentity, Pointer, PointerOutcome,
    PressTarget, Quad, Rect, TextAlign, TextRun,
};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Drawing {
    Icon { bounds: Rect, mask: Mask, colour: Colour },
    Rectangle {
        bounds: Rect,
        fill: Option<Colour>,
        stroke: Option<Colour>,
        stroke_width: f32,
        action: Option<Action>,
        long_action: Option<Action>,
        drag_action: Option<Action>,
    },
    Text { bounds: Rect, text: String, size: f32, colour: Colour, align: TextAlign, tabular_numbers: bool },
}

impl Engine {
    pub(super) fn layout_canvas(&mut self, group: NodeIdentity, width: f32, height: f32, drawings: &[Drawing], rect: Rect) {
        if !cfg!(feature = "ui-canvas") { return; }
        let scale = (rect.width / width).min(rect.height / height);
        let previous_clip = self.clip;
        self.clip = self.clip.intersection(Rect {
            width: width * scale, height: height * scale, ..rect
        });
        let bounds = |area: Rect| {
            let x = rect.x + area.x * scale;
            let y = rect.y + area.y * scale;
            Rect { x, y,
                width: area.width * scale,
                height: area.height * scale,
            }
        };
        for drawing in drawings {
            match drawing {
                Drawing::Icon { bounds: area, mask, colour } => {
                    self.scene.masks.push(MaskRun {
                        mask: mask.clone(), rect: bounds(*area), clip: self.clip,
                        colour: self.canvas_colour(*colour), scrolling: self.scrolling,
                    });
                }
                Drawing::Rectangle { bounds: area, fill, stroke, stroke_width, action, long_action, drag_action } => {
                    let area = bounds(*area);
                    if let Some(colour) = fill { self.canvas_quad(area, *colour); }
                    if let Some(colour) = stroke {
                        let thickness = stroke_width * scale;
                        let half = thickness / 2.0;
                        self.canvas_quad(Rect { x: area.x - half, y: area.y - half,
                            width: area.width + thickness, height: thickness }, *colour);
                        self.canvas_quad(Rect { x: area.x - half, y: area.y + area.height - half,
                            width: area.width + thickness, height: thickness }, *colour);
                        self.canvas_quad(Rect { x: area.x - half, y: area.y + half,
                            width: thickness, height: (area.height - thickness).max(0.0) }, *colour);
                        self.canvas_quad(Rect { x: area.x + area.width - half, y: area.y + half,
                            width: thickness, height: (area.height - thickness).max(0.0) }, *colour);
                    }
                    if let Some(action) = action {
                        let start = self.hit_regions.len();
                        self.push_press_region(area, action.clone(), long_action.clone());
                        if let Some(region) = self.hit_regions.get_mut(start)
                            && drag_action.is_some() {
                            region.drag_group = Some(group);
                            region.drag_action = drag_action.clone();
                        }
                    }
                }
                Drawing::Text { bounds: area, text, size, colour, align, tabular_numbers } => {
                    let area = bounds(*area);
                    self.scene.text.push(TextRun {
                        text: text.as_str().into(), rect: area, clip: area.intersection(self.clip),
                        font_size: self.scaled_font(size * scale / self.viewport.scale),
                        colour: self.canvas_colour(*colour), align: *align,
                        tabular_numbers: *tabular_numbers, scrolling: self.scrolling,
                    });
                }
            }
        }
        self.clip = previous_clip;
    }

    pub(super) fn move_canvas_pointer(
        &mut self, id: i32, group: NodeIdentity, previous: (f32, f32),
        target: Option<PressTarget>, x: f32, y: f32,
    ) -> PointerOutcome {
        let mut entered: Vec<_> = self.hit_regions.iter()
            .filter(|region| region.drag_group == Some(group))
            .filter_map(|region| {
                let offset = if region.scrolling { self.scroll_offset - self.scroll_origin } else { 0.0 };
                segment_entry(region.rect, (previous.0, previous.1 + offset), (x, y + offset))
                    .and_then(|entry| region.drag_action.clone().map(|action| (entry, region.target, action)))
            }).collect();
        entered.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut last = target;
        let mut activated = false;
        for (_, next, action) in entered {
            if last != Some(next) { activated |= self.apply(action); }
            last = Some(next);
        }
        self.pointers.insert(id, Pointer::CanvasDrag { group, previous: (x, y), target: last });
        PointerOutcome { activated, haptic: activated, ..PointerOutcome::default().captured() }
    }

    fn canvas_quad(&mut self, rect: Rect, colour: Colour) {
        let rect = Rect {
            x: rect.x.round(), y: rect.y.round(),
            width: (rect.x + rect.width).round() - rect.x.round(),
            height: (rect.y + rect.height).round() - rect.y.round(),
        };
        if rect.width > 0.0 && rect.height > 0.0 {
            self.scene.quads.push(Quad {
                rect, clip: self.clip, colour: self.canvas_colour(colour), scrolling: self.scrolling,
            });
        }
    }

    fn canvas_colour(&self, colour: Colour) -> Colour {
        let colour = self.scene.colour(colour);
        let linear = |channel: f32| {
            if channel <= 0.04045 { channel / 12.92 }
            else { ((channel + 0.055) / 1.055).powf(2.4) }
        };
        Colour { red: linear(colour.red), green: linear(colour.green), blue: linear(colour.blue), ..colour }
    }
}

// First intersection along the gesture, including cells crossed between input samples.
fn segment_entry(rect: Rect, from: (f32, f32), to: (f32, f32)) -> Option<f32> {
    let mut entry = 0.0_f32;
    let mut exit = 1.0_f32;
    for (start, end, low, high) in [
        (from.0, to.0, rect.x, rect.x + rect.width),
        (from.1, to.1, rect.y, rect.y + rect.height),
    ] {
        let delta = end - start;
        if delta.abs() < f32::EPSILON {
            if start < low || start >= high { return None; }
        } else {
            let a = (low - start) / delta;
            let b = (high - start) / delta;
            entry = entry.max(a.min(b));
            exit = exit.min(a.max(b));
            if entry > exit { return None; }
        }
    }
    Some(entry)
}
