use super::{
    Action, Colour, Engine, Mask, MaskRun, NodeIdentity, Pointer, PointerOutcome,
    PlaybackProgress, PressTarget, Quad, Rect, TextAlign, TextRun,
};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Drawing {
    Range {
        id: NodeIdentity, bounds: Rect, spec: RangeSpec, fill: Colour, inner_fill: Colour, progress_fill: Colour,
        start_event: bool, commit_event: bool, snapshot_event: bool, seek_event: bool,
    },
    Icon { bounds: Rect, mask: Mask, colour: Colour },
    Rectangle {
        bounds: Rect,
        fill: Option<Colour>,
        stroke: Option<Colour>,
        stroke_width: f32,
        action: Option<Action>,
        long_action: Option<Action>,
        drag_action: Option<Action>,
        drag_start: Option<Action>,
        drag_end: Option<Action>,
        haptic: bool,
    },
    Text { bounds: Rect, text: String, size: f32, colour: Colour, align: TextAlign, tabular_numbers: bool },
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RangeSpec {
    pub total: f64,
    pub span: f64,
    pub initial_start: f64,
    pub step: f64,
    pub snap_points: Vec<f64>,
    pub snap_distance: f32,
    pub tap_snap_distance: f32,
    pub hit_slop: f32,
    #[serde(default)]
    pub min_drag_width: f32,
    pub inset: f32,
    #[serde(default)]
    pub drag_only: bool,
    #[serde(default)]
    pub snap_on_tap: bool,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub progress_duration_ms: f64,
    #[serde(default)]
    pub progress_playing: bool,
    #[serde(default)]
    pub progress_loop: bool,
}

#[derive(Clone, Debug)]
pub(super) struct RangeState {
    spec: RangeSpec,
    start: f64,
    reported: f64,
    snapped: Option<f64>,
    area: Rect,
    scale: f32,
    quads: Vec<(usize, f32, f32)>,
    visible: bool,
    start_event: bool,
    commit_event: bool,
    snapshot_event: bool,
    seek_event: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RangePointer {
    pub range: NodeIdentity,
    down_x: f32,
    down_y: f32,
    grab: f64,
    dragging: bool,
    selection_drag: bool,
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
                Drawing::Range { id, bounds: area, spec, fill, inner_fill, progress_fill, start_event, commit_event, snapshot_event, seek_event } => {
                    let area = bounds(*area);
                    // initialStart is a seed, not a controlled value. Playback renders
                    // and delayed JS acknowledgements cannot rewind native movement.
                    let state = self.canvas_ranges.entry(*id).or_insert_with(|| RangeState {
                        spec: spec.clone(), start: spec.initial_start.clamp(0.0, spec.total - spec.span),
                        reported: spec.initial_start,
                        snapped: None, area, scale, quads: Vec::new(),
                        visible: true, start_event: *start_event, commit_event: *commit_event, snapshot_event: *snapshot_event, seek_event: *seek_event,
                    });
                    let progress_changed = (state.spec.progress, state.spec.progress_duration_ms, state.spec.progress_playing, state.spec.progress_loop)
                        != (spec.progress, spec.progress_duration_ms, spec.progress_playing, spec.progress_loop);
                    state.spec = spec.clone();
                    state.start = state.start.clamp(0.0, spec.total - spec.span);
                    state.area = area;
                    state.scale = scale;
                    state.visible = true;
                    state.start_event = *start_event;
                    state.commit_event = *commit_event;
                    state.snapshot_event = *snapshot_event;
                    state.seek_event = *seek_event;
                    state.quads.clear();
                    let start = state.start;
                    let selection = Rect { x: area.x + (start / spec.total) as f32 * area.width,
                        width: (spec.span / spec.total) as f32 * area.width, ..area };
                    let inset = (spec.inset * scale).min(selection.width / 4.0).min(selection.height / 4.0);
                    let mut quads = Vec::with_capacity(3);
                    let layers = [(0.0, *fill, 1.0), (inset, *inner_fill, 1.0),
                        (inset, *progress_fill, spec.progress.clamp(0.0, 1.0) as f32)];
                    for &(inset, colour, fraction) in layers.iter().take(if spec.progress_duration_ms > 0.0 { 2 } else { 3 }) {
                        let quad = self.scene.quads.len();
                        self.canvas_quad(Rect { x: selection.x + inset, y: selection.y + inset,
                            width: (selection.width - inset * 2.0).max(0.0) * fraction, height: (selection.height - inset * 2.0).max(0.0) }, colour);
                        if self.scene.quads.len() > quad { quads.push((quad, inset, fraction)); }
                    }
                    self.canvas_ranges.get_mut(id).expect("range exists").quads = quads;
                    if spec.progress_duration_ms > 0.0 {
                        let x = (selection.x + inset).round();
                        let y = (selection.y + inset).round();
                        let width = (selection.x + selection.width - inset).round() - x;
                        let height = (selection.y + selection.height - inset).round() - y;
                        let quad = self.scene.quads.len();
                        self.scene.quads.push(Quad { rect: Rect { x, y, width: 0.0, height },
                            clip: self.clip, colour: self.canvas_colour(*progress_fill), scrolling: self.scrolling });
                        let mut edge = self.scene.quads[quad].clone();
                        edge.rect.width = 1.0;
                        edge.colour.alpha = 0.0;
                        self.scene.quads.push(edge);
                        let duration = (spec.progress_duration_ms / 1000.0) as f32;
                        let position = spec.progress as f32 * duration;
                        // Share Playing's clock interpolation and shaded subpixel edge.
                        let progress = self.playback_progress.entry(*id).or_insert_with(|| PlaybackProgress {
                            clock: None, speed: 1.0, position, duration, playing: spec.progress_playing,
                            looping: spec.progress_loop, started: Instant::now(), quad, width,
                            visible: true, elapsed_label: None,
                        });
                        if progress_changed {
                            progress.position = position;
                            progress.duration = duration;
                            progress.playing = spec.progress_playing;
                            progress.looping = spec.progress_loop;
                            progress.started = Instant::now();
                        }
                        progress.quad = quad;
                        progress.width = width;
                        progress.visible = true;
                    }
                    let region = self.hit_regions.len();
                    self.push_hit_region(Rect { y: area.y - spec.hit_slop * scale,
                        height: area.height + spec.hit_slop * scale * 2.0, ..area }, Action::CanvasRange { id: id.0 });
                    if let Some(region) = self.hit_regions.get_mut(region) { region.haptic = false; }
                }
                Drawing::Icon { bounds: area, mask, colour } => {
                    self.scene.masks.push(MaskRun {
                        mask: mask.clone(), rect: bounds(*area), clip: self.clip,
                        colour: self.canvas_colour(*colour), scrolling: self.scrolling,
                    });
                }
                Drawing::Rectangle { bounds: area, fill, stroke, stroke_width, action, long_action, drag_action, drag_start, drag_end, haptic } => {
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
                        if let Some(region) = self.hit_regions.get_mut(start) {
                            region.haptic = *haptic;
                            if drag_action.is_some() {
                                region.drag_group = Some(group);
                                region.drag_action = drag_action.clone();
                                region.drag_start = drag_start.clone();
                                region.drag_end = drag_end.clone();
                            }
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
                    .and_then(|entry| region.drag_action.clone().map(|action| (entry, region.target, action, region.haptic)))
            }).collect();
        entered.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut last = target;
        let mut activated = false;
        let mut haptic = false;
        for (_, next, action, feedback) in entered {
            if last != Some(next) {
                let applied = self.apply(action);
                activated |= applied;
                haptic |= applied && feedback;
            }
            last = Some(next);
        }
        self.pointers.insert(id, Pointer::CanvasDrag { group, previous: (x, y), target: last });
        PointerOutcome { activated, haptic, ..PointerOutcome::default().captured() }
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

impl Engine {
    pub(super) fn reset_canvas_range_layout(&mut self) {
        for range in self.canvas_ranges.values_mut() { range.visible = false; }
    }

    pub(super) fn retain_canvas_ranges(&mut self) {
        self.canvas_ranges.retain(|_, range| range.visible);
    }

    pub(super) fn snapshot_canvas_ranges(&mut self) {
        let ids: Vec<_> = self.canvas_ranges.keys().copied().collect();
        for id in ids { self.emit_canvas_range(id, "onRangeSnapshot"); }
    }

    pub(super) fn emit_canvas_range(&mut self, id: NodeIdentity, event: &'static str) -> bool {
        let Some(range) = self.canvas_ranges.get_mut(&id).filter(|range| range.visible) else { return false; };
        let enabled = match event {
            "onRangeStart" => range.start_event,
            "onRangeCommit" => range.commit_event,
            "onRangeSnapshot" => range.snapshot_event && range.reported != range.start,
            _ => false,
        };
        if !enabled { return false; }
        let start = range.start;
        if event != "onRangeStart" { range.reported = start; }
        self.apply(super::react::event(id.0, event, vec![serde_json::json!(start)]))
    }

    pub(super) fn begin_canvas_range(&mut self, id: i32, range: NodeIdentity, x: f32, y: f32) -> PointerOutcome {
        // One pointer owns the range, as for image cropping. Moves never queue JS.
        if self.pointers.values().any(|pointer| matches!(pointer, Pointer::CanvasRange(_))) {
            return PointerOutcome::default().captured();
        }
        let Some(state) = self.canvas_ranges.get(&range) else { return PointerOutcome::default(); };
        let position = range_position(state, x);
        let drag_width = range_distance(state, state.spec.min_drag_width).max(state.spec.span).min(state.spec.total);
        let drag_start = (state.start + (state.spec.span - drag_width) / 2.0).clamp(0.0, state.spec.total - drag_width);
        let inside = position >= drag_start && position <= drag_start + drag_width;
        let marker = state.spec.snap_on_tap && nearest_range_snap(state, position, state.spec.tap_snap_distance).is_some();
        if state.spec.drag_only && !inside && !marker { return PointerOutcome::default().captured(); }
        let selection_drag = inside || !state.spec.drag_only;
        let deferred_start = state.spec.drag_only;
        // Keep the initial offset even when grabbed outside the visible selection.
        let grab = if inside { position - state.start } else { state.spec.span / 2.0 };
        self.canvas_ranges.get_mut(&range).expect("range exists").snapped = None;
        self.pointers.insert(id, Pointer::CanvasRange(RangePointer { range, down_x: x, down_y: y, grab, dragging: false, selection_drag }));
        self.claim_pointer(id);
        let activated = !deferred_start && self.emit_canvas_range(range, "onRangeStart");
        PointerOutcome { activated, ..PointerOutcome::default().captured() }
    }

    pub(super) fn move_canvas_range(&mut self, id: i32, mut pointer: RangePointer, x: f32, y: f32) -> PointerOutcome {
        let was_dragging = pointer.dragging;
        pointer.dragging |= (x - pointer.down_x).abs() > self.scaled(super::TAP_SLOP)
            || (y - pointer.down_y).abs() > self.scaled(super::TAP_SLOP);
        self.pointers.insert(id, Pointer::CanvasRange(pointer));
        if !pointer.dragging || !pointer.selection_drag { return PointerOutcome::default().captured(); }
        let activated = !was_dragging
            && self.canvas_ranges.get(&pointer.range).is_some_and(|range| range.spec.drag_only)
            && self.emit_canvas_range(pointer.range, "onRangeStart");
        let Some(state) = self.canvas_ranges.get(&pointer.range) else { return PointerOutcome::default().captured(); };
        let raw = (range_position(state, x) - pointer.grab).clamp(0.0, state.spec.total - state.spec.span);
        let snapped = state.snapped.filter(|snap| *snap <= state.spec.total - state.spec.span
            && (raw - snap).abs() <= range_distance(state, state.spec.snap_distance * 1.5))
            .or_else(|| nearest_range_snap(state, raw, state.spec.snap_distance));
        let haptic = snapped.is_some() && snapped != state.snapped;
        let start = snapped.unwrap_or_else(|| rounded_range_start(state, raw));
        let changed = self.set_canvas_range_start(pointer.range, start, snapped);
        PointerOutcome { changed, haptic, activated, ..PointerOutcome::default().captured() }
    }

    pub(super) fn end_canvas_range(&mut self, id: i32, pointer: RangePointer, x: f32, y: f32) -> PointerOutcome {
        // Consume the up coordinates too, even if the host coalesced the last move.
        let mut outcome = self.move_canvas_range(id, pointer, x, y);
        if matches!(self.pointers.get(&id), Some(Pointer::CanvasRange(pointer)) if pointer.dragging && !pointer.selection_drag) {
            self.pointers.remove(&id);
            return outcome;
        }
        if matches!(self.pointers.get(&id), Some(Pointer::CanvasRange(pointer)) if !pointer.dragging) {
            if let Some(state) = self.canvas_ranges.get(&pointer.range).filter(|state| state.spec.drag_only) {
                let position = range_position(state, x);
                let snap = state.spec.snap_on_tap.then(|| nearest_range_snap(state, position, state.spec.tap_snap_distance)).flatten();
                if let Some(start) = snap {
                    outcome.changed |= self.set_canvas_range_start(pointer.range, start, Some(start));
                    self.pointers.remove(&id);
                    outcome.haptic = true;
                    outcome.activated |= self.emit_canvas_range(pointer.range, "onRangeStart");
                    outcome.activated |= self.emit_canvas_range(pointer.range, "onRangeCommit");
                    return outcome;
                }
                let seek = state.seek_event && position >= state.start && position <= state.start + state.spec.span;
                let relative = (position - state.start).round().clamp(0.0, state.spec.span);
                self.pointers.remove(&id);
                outcome.activated = seek && self.apply(super::react::event(pointer.range.0, "onRangeSeek", vec![serde_json::json!(relative)]));
                return outcome;
            }
            // A tap chooses its nearest line afresh; drag hysteresis and grab
            // offset must not centre a highlight or retain a neighbouring snap.
            if let Some(state) = self.canvas_ranges.get(&pointer.range) {
                let position = range_position(state, x);
                let snap = nearest_range_snap(state, position, state.spec.tap_snap_distance);
                let start = snap.unwrap_or_else(|| rounded_range_start(state, position - state.spec.span / 2.0));
                outcome.changed |= self.set_canvas_range_start(pointer.range, start, snap);
                outcome.haptic |= snap.is_some();
            }
        }
        self.pointers.remove(&id);
        outcome.activated |= self.emit_canvas_range(pointer.range, "onRangeCommit");
        outcome
    }

    pub(super) fn cancel_canvas_range(&mut self, pointer: RangePointer) -> bool {
        if !pointer.selection_drag { return false; }
        if !pointer.dragging && self.canvas_ranges.get(&pointer.range).is_some_and(|range| range.spec.drag_only) {
            return false;
        }
        self.emit_canvas_range(pointer.range, "onRangeCommit")
    }

    fn set_canvas_range_start(&mut self, id: NodeIdentity, start: f64, snapped: Option<f64>) -> bool {
        let Some(range) = self.canvas_ranges.get_mut(&id) else { return false; };
        range.snapped = snapped;
        if range.start == start { return false; }
        range.start = start;
        let left = range.area.x + (start / range.spec.total) as f32 * range.area.width;
        let width = (range.spec.span / range.spec.total) as f32 * range.area.width;
        for &(index, inset, fraction) in &range.quads {
            let quad = &mut self.scene.quads[index];
            let x = left + inset;
            quad.rect.x = x.round();
            quad.rect.width = (x + (width - inset * 2.0).max(0.0) * fraction).round() - x.round();
        }
        if let Some(progress) = self.playback_progress.get_mut(&id) {
            let inset = (range.spec.inset * range.scale).min(width / 4.0).min(range.area.height / 4.0);
            let x = (left + inset).round();
            progress.width = (left + width - inset).round() - x;
            progress.position = 0.0;
            progress.playing = false;
            self.scene.quads[progress.quad].rect.x = x;
            self.scene.quads[progress.quad].rect.width = 0.0;
            self.scene.quads[progress.quad + 1].rect.x = x;
            self.scene.quads[progress.quad + 1].colour.alpha = 0.0;
        }
        self.scene.revision = self.scene.revision.wrapping_add(1);
        true
    }
}

fn range_position(range: &RangeState, x: f32) -> f64 {
    ((x - range.area.x) / range.area.width) as f64 * range.spec.total
}

fn range_distance(range: &RangeState, logical: f32) -> f64 {
    (logical * range.scale / range.area.width) as f64 * range.spec.total
}

fn nearest_range_snap(range: &RangeState, value: f64, logical: f32) -> Option<f64> {
    let distance = range_distance(range, logical);
    range.spec.snap_points.iter().copied()
        .filter(|point| *point >= 0.0 && *point <= range.spec.total - range.spec.span && (value - point).abs() <= distance)
        .min_by(|a, b| (value - a).abs().total_cmp(&(value - b).abs()))
}

fn rounded_range_start(range: &RangeState, value: f64) -> f64 {
    let last = range.spec.total - range.spec.span;
    if value <= 0.0 { return 0.0; }
    if value >= last { return last; }
    ((value / range.spec.step).round() * range.spec.step).clamp(0.0, last)
}
