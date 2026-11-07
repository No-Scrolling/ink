//! Reuse layout and primitive ranges for subtrees without external side effects.
use super::*;
use rustc_hash::FxHashMap;
use std::ops::Range;

#[derive(Default)]
pub(super) struct Reuse {
    entries: FxHashMap<NodeIdentity, Entry>,
    output: Output,
    pub(super) fixed: Vec<(NodeIdentity, FixedTextRun)>,
    scale: f32,
    frame: u64,
    #[cfg(test)]
    disabled: bool,
    #[cfg(test)]
    hits: usize,
}

struct Entry {
    node: Node,
    measurements: Vec<(Rect, MeasuredSize)>,
    paint: Option<Paint>,
    used: bool,
    checked_frame: u64,
    source: usize,
}

struct Paint {
    context: Context,
    ranges: Ranges,
    frame: u64,
}

#[derive(PartialEq)]
struct Context {
    rect: Rect,
    clip: Rect,
    scrolling: bool,
    light: bool,
}

#[derive(Default)]
struct Output {
    quads: Vec<Quad>,
    lines: Vec<Line>,
    text: Vec<TextRun>,
    masks: Vec<MaskRun>,
    hits: Vec<HitRegion>,
    fixed: Vec<(NodeIdentity, FixedTextRun)>,
}

struct Ranges {
    quads: Range<usize>,
    lines: Range<usize>,
    text: Range<usize>,
    masks: Range<usize>,
    hits: Range<usize>,
    fixed: Range<usize>,
}

impl Ranges {
    fn start(engine: &Engine) -> Self {
        Self {
            quads: engine.scene.quads.len()..0,
            lines: engine.scene.lines.len()..0,
            text: engine.scene.text.len()..0,
            masks: engine.scene.masks.len()..0,
            hits: engine.hit_regions.len()..0,
            fixed: engine.reuse.fixed.len()..0,
        }
    }

    fn finish(&mut self, engine: &Engine) {
        self.quads.end = engine.scene.quads.len();
        self.lines.end = engine.scene.lines.len();
        self.text.end = engine.scene.text.len();
        self.masks.end = engine.scene.masks.len();
        self.hits.end = engine.hit_regions.len();
        self.fixed.end = engine.reuse.fixed.len();
    }

    fn rebase(&mut self, old: &Self, new: &Self) {
        fn rebase(range: &mut Range<usize>, old: &Range<usize>, new: &Range<usize>) {
            range.start = new.start + (range.start - old.start);
            range.end = new.start + (range.end - old.start);
        }
        rebase(&mut self.quads, &old.quads, &new.quads);
        rebase(&mut self.lines, &old.lines, &new.lines);
        rebase(&mut self.text, &old.text, &new.text);
        rebase(&mut self.masks, &old.masks, &new.masks);
        rebase(&mut self.hits, &old.hits, &new.hits);
        rebase(&mut self.fixed, &old.fixed, &new.fixed);
    }
}

fn children(node: &Node) -> &[Node] {
    match &node.kind {
        NodeKind::Stack { children, .. }
        | NodeKind::Row { children, .. }
        | NodeKind::Pressable { children, .. }
        | NodeKind::LinkPreview { children }
        | NodeKind::RowTitle { children, .. } => children,
        #[cfg(feature = "ui-messages")]
        NodeKind::Message { children, .. }
        | NodeKind::MessageQuote { children } => children,
        _ => &[],
    }
}

// Linked text inherits its enclosing hit region's long-press action. Stateful
// nodes register live resources, so they and their ancestors use normal layout.
fn reusable(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Text { links, .. } => links.is_empty(),
        NodeKind::Button { .. }
        | NodeKind::Field { .. }
        | NodeKind::Icon { .. }
        | NodeKind::Toggle { .. }
        | NodeKind::PitchIndicator { .. } => true,
        NodeKind::Stack { .. }
        | NodeKind::Row { .. }
        | NodeKind::Pressable { .. }
        | NodeKind::LinkPreview { .. }
        | NodeKind::RowTitle { .. } => children(node).iter().all(reusable),
        #[cfg(feature = "ui-messages")]
        NodeKind::Message { .. }
        | NodeKind::MessageQuote { .. } => children(node).iter().all(reusable),
        _ => false,
    }
}

impl Reuse {
    #[cfg(feature = "ui-messages")]
    pub(super) fn invalidate_hits(&mut self, changed: Range<usize>) {
        for entry in self.entries.values_mut() {
            if entry.paint.as_ref().is_some_and(|paint| {
                paint.frame == self.frame
                    && paint.ranges.hits.start < changed.end
                    && paint.ranges.hits.end > changed.start
            }) {
                entry.paint = None;
            }
        }
    }

    fn prepare(&mut self, node: &Node) -> bool {
        // Content includes descendants and actions, after React operations and
        // native list patches. Generated nodes can share an owner, so identity
        // alone cannot establish cleanliness, even within the current pass.
        let source = std::ptr::from_ref(node) as usize;
        if let Some(entry) = self.entries.get_mut(&node.identity)
            && ((entry.checked_frame == self.frame && entry.source == source)
                || entry.node == *node)
        {
            // Nodes are immutably borrowed from the native tree throughout a
            // layout pass. Validate content once per source node and pass.
            entry.checked_frame = self.frame;
            entry.source = source;
            entry.used = true;
            return true;
        }
        self.entries.remove(&node.identity);
        if !reusable(node) {
            return false;
        }
        self.entries.insert(
            node.identity,
            Entry {
                node: node.clone(),
                measurements: Vec::new(),
                paint: None,
                used: true,
                checked_frame: self.frame,
                source,
            },
        );
        true
    }

    fn retain_descendants(&mut self, node: &Node, ranges: Option<(&Ranges, &Ranges)>) {
        for child in children(node) {
            if let Some(entry) = self.entries.get_mut(&child.identity) {
                entry.used = true;
                if let Some((old, new)) = ranges
                    && let Some(paint) = &mut entry.paint
                    && paint.frame == self.frame.wrapping_sub(1)
                    && entry.node == *child
                {
                    paint.ranges.rebase(old, new);
                    paint.frame = self.frame;
                }
            }
            self.retain_descendants(child, ranges);
        }
    }
}

impl Engine {
    pub(super) fn begin_reuse(&mut self) {
        let cache = &mut self.reuse;
        cache.frame = cache.frame.wrapping_add(1);
        if cache.scale != self.viewport.scale {
            cache.entries.clear();
            cache.scale = self.viewport.scale;
        }
        for entry in cache.entries.values_mut() {
            entry.used = false;
        }
        // Two allocations per primitive family. Every ancestor records ranges
        // into this output, rather than retaining another copy of its contents.
        std::mem::swap(&mut cache.output.quads, &mut self.scene.quads);
        std::mem::swap(&mut cache.output.lines, &mut self.scene.lines);
        std::mem::swap(&mut cache.output.text, &mut self.scene.text);
        std::mem::swap(&mut cache.output.masks, &mut self.scene.masks);
        std::mem::swap(&mut cache.output.hits, &mut self.hit_regions);
        std::mem::swap(&mut cache.output.fixed, &mut cache.fixed);
        cache.fixed.clear();
    }

    pub(super) fn finish_reuse(&mut self) {
        let frame = self.reuse.frame;
        self.reuse.entries.retain(|_, entry| {
            if entry
                .paint
                .as_ref()
                .is_some_and(|paint| paint.frame != frame)
            {
                entry.paint = None;
            }
            entry.used
        });
    }

    pub(super) fn measure_reused(&mut self, node: &Node, available: Rect) -> MeasuredSize {
        #[cfg(test)]
        if self.reuse.disabled {
            return self.measure_inner(node, available);
        }
        if node.identity.0 == 0 || !self.reuse.prepare(node) {
            return self.measure_inner(node, available);
        }
        // Eligible measurements depend on dimensions, not position. Editors,
        // images and platform surfaces, which can depend on y, are excluded.
        let constraints = Rect {
            x: 0.0,
            y: 0.0,
            ..available
        };
        let cached = self.reuse.entries[&node.identity]
            .measurements
            .iter()
            .find(|(rect, _)| *rect == constraints)
            .map(|(_, size)| *size);
        if let Some(size) = cached {
            self.reuse.retain_descendants(node, None);
            return size;
        }
        let size = self.measure_inner(node, available);
        self.reuse.prepare(node);
        let measurements = &mut self
            .reuse
            .entries
            .get_mut(&node.identity)
            .unwrap()
            .measurements;
        // Layout may measure one node with several constraints in a single pass.
        if measurements.len() == 8 {
            measurements.remove(0);
        }
        measurements.push((constraints, size));
        size
    }

    pub(super) fn layout_reused(&mut self, node: &Node, rect: Rect, bottom_inset: bool) {
        #[cfg(test)]
        if self.reuse.disabled {
            self.layout_node_inner(node, rect, bottom_inset);
            return;
        }
        if node.identity.0 == 0 || !self.reuse.prepare(node) {
            self.layout_node_inner(node, rect, bottom_inset);
            return;
        }
        let context = Context {
            rect,
            clip: self.clip,
            scrolling: self.scrolling,
            light: self.scene.light,
        };
        let frame = self.reuse.frame;
        let previous = self
            .reuse
            .entries
            .get_mut(&node.identity)
            .unwrap()
            .paint
            .take()
            .filter(|paint| paint.frame == frame.wrapping_sub(1) && paint.context == context);
        let mut ranges = Ranges::start(self);
        if let Some(paint) = previous {
            #[cfg(test)]
            {
                self.reuse.hits += 1;
            }
            let output = &self.reuse.output;
            self.scene
                .quads
                .extend_from_slice(&output.quads[paint.ranges.quads.clone()]);
            self.scene
                .lines
                .extend_from_slice(&output.lines[paint.ranges.lines.clone()]);
            self.scene
                .text
                .extend_from_slice(&output.text[paint.ranges.text.clone()]);
            self.scene
                .masks
                .extend_from_slice(&output.masks[paint.ranges.masks.clone()]);
            self.hit_regions
                .extend_from_slice(&output.hits[paint.ranges.hits.clone()]);
            for (id, run) in &output.fixed[paint.ranges.fixed.clone()] {
                let mut run = run.clone();
                run.run = ranges.text.start + (run.run - paint.ranges.text.start);
                self.fixed_text_runs.insert(*id, run.clone());
                self.reuse.fixed.push((*id, run));
            }
            ranges.finish(self);
            // A later child edit must still be able to reuse its clean siblings.
            // Rebase descendant ranges when copying an entire parent block.
            self.reuse
                .retain_descendants(node, Some((&paint.ranges, &ranges)));
        } else {
            self.layout_node_inner(node, rect, bottom_inset);
            ranges.finish(self);
        }
        self.reuse.prepare(node);
        self.reuse.entries.get_mut(&node.identity).unwrap().paint = Some(Paint {
            context,
            ranges,
            frame,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    struct Pair {
        incremental: Engine,
        full: Engine,
        trees: [ReactTree; 2],
    }

    impl Pair {
        fn new() -> Self {
            let mut pair = Self {
                incremental: Engine::new(),
                full: Engine::new(),
                trees: std::array::from_fn(|_| ReactTree::default()),
            };
            pair.full.reuse.disabled = true;
            pair.incremental.set_viewport(1080, 1240);
            pair.full.set_viewport(1080, 1240);
            pair
        }

        fn commit(&mut self, operations: Value) {
            self.trees[0]
                .apply(
                    serde_json::from_value(operations.clone()).unwrap(),
                    &mut self.incremental,
                )
                .unwrap();
            self.trees[1]
                .apply(serde_json::from_value(operations).unwrap(), &mut self.full)
                .unwrap();
            // The oracle always takes the original recursive production path,
            // including when React used the fixed-width text shortcut.
            self.full.relayout_scene();
            self.check();
        }

        fn check(&self) {
            let mut actual = self.incremental.scene.clone();
            actual.revision = self.full.scene.revision;
            assert_eq!(actual, self.full.scene);
            assert_eq!(
                format!("{:?}", self.incremental.hit_regions),
                format!("{:?}", self.full.hit_regions)
            );
            assert_eq!(
                format!("{:?}", self.incremental.text_inputs),
                format!("{:?}", self.full.text_inputs)
            );
            assert_eq!(
                format!("{:?}", self.incremental.swipe_regions),
                format!("{:?}", self.full.swipe_regions)
            );
            assert_eq!(
                self.incremental.react_list_positions,
                self.full.react_list_positions
            );
            assert_eq!(self.incremental.scroll_max, self.full.scroll_max);
            for (id, run) in &self.incremental.fixed_text_runs {
                assert_eq!(run.run, self.full.fixed_text_runs[id].run);
            }
        }
    }

    fn create(id: usize, kind: &str, props: Value, parent: usize) -> Vec<Value> {
        vec![
            json!({"op":"create", "id":id, "type":kind, "props":props}),
            json!({"op":"insert", "id":id, "parent":parent}),
        ]
    }

    #[test]
    fn a_child_edit_reuses_siblings_after_several_clean_parent_frames() {
        let mut pair = Pair::new();
        let mut initial = create(1, "Screen", json!({"title":"Siblings"}), 0);
        initial.extend(create(2, "Stack", json!({"gap":4}), 1));
        for id in 3..9 {
            initial.extend(create(id, "Text", json!({"text":format!("Child {id}")}), 2));
        }
        pair.commit(json!(initial));
        for _ in 0..3 {
            pair.incremental.relayout_scene();
            pair.full.relayout_scene();
            pair.check();
        }
        let before = pair.incremental.reuse.hits;
        pair.commit(json!([{"op":"text","ids":[3],"values":["Changed"]}]));
        assert!(
            pair.incremental.reuse.hits >= before + 5,
            "clean sibling ranges must survive parent reuse"
        );
    }

    #[test]
    fn commits_and_external_invalidations_match_full_layout() {
        let mut pair = Pair::new();
        let mut initial = create(
            1,
            "Screen",
            json!({"title":"Reuse", "pinnedHeader":true}),
            0,
        );
        initial.extend(create(2, "Stack", json!({"gap":7}), 1));
        for id in 10..42 {
            initial.extend(create(
                id,
                "Text",
                json!({"text":format!("Row {id}: café العربية 🙂"), "size":14, "maxLines":2}),
                2,
            ));
        }
        initial.extend(create(
            50,
            "Button",
            json!({"label":"Press", "onPress":true}),
            1,
        ));
        initial.extend(create(51, "#text", json!({"text":"Press"}), 50));
        pair.commit(json!(initial));
        pair.incremental.relayout_scene();
        pair.full.relayout_scene();
        pair.check();
        assert!(
            pair.incremental.reuse.hits > 0,
            "must exercise cached scene ranges"
        );
        let mut seed = 0x8bad_f00du32;
        for step in 0..320 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let id = 10 + (seed as usize % 32);
            match step % 10 {
                0..=3 => pair.commit(json!([{"op":"text", "ids":[id], "values":[format!("{step} changed text {}", "wrapped ".repeat(seed as usize % 11))]}])),
                4 => pair.commit(json!([{"op":"hidden", "id":id, "value":seed & 1 == 0}])),
                5 => pair.commit(json!([{"op":"insert", "id":id, "parent":2, "before":10 + ((id - 9) % 32)}])),
                6 => {
                    for engine in [&mut pair.incremental, &mut pair.full] { engine.set_colour_scheme(seed & 1 == 0); }
                }
                7 => {
                    for engine in [&mut pair.incremental, &mut pair.full] { engine.set_keyboard_inset((seed % 3) * 100); }
                }
                8 => {
                    for engine in [&mut pair.incremental, &mut pair.full] { engine.scroll_by((seed % 300) as f32 - 150.0); }
                }
                _ => {
                    for engine in [&mut pair.incremental, &mut pair.full] {
                        engine.set_viewport(800 + (seed % 3) * 140, 1240);
                        engine.set_display_cutouts(vec![Rect { x: 400.0, y: 0.0, width: (seed % 80) as f32, height: 100.0 }]);
                    }
                }
            }
            pair.check();
        }
    }

    #[test]
    fn reused_ranges_remap_fixed_text_and_survive_removal_and_reparenting() {
        let mut pair = Pair::new();
        let mut initial = create(1, "Screen", json!({"title":"Fixed"}), 0);
        initial.extend(create(2, "Stack", json!({"gap":4}), 1));
        initial.extend(create(3, "Text", json!({"text":"before", "size":14}), 1));
        initial.extend(create(
            4,
            "Text",
            json!({"text":"000", "size":14, "width":80, "maxLines":1}),
            2,
        ));
        initial.extend(create(
            5,
            "Text",
            json!({"text":"unchanged", "size":14, "onPress":true}),
            2,
        ));
        pair.commit(json!(initial));
        for text in ["001", "999999999999999999", "🙂", "002"] {
            pair.commit(json!([{"op":"text", "ids":[4], "values":[text]}]));
            pair.commit(
                json!([{"op":"text", "ids":[3], "values":[format!("prefix {text}\nnext")]}]),
            );
        }
        pair.commit(json!([{"op":"insert", "id":4, "parent":1, "before":3}]));
        pair.commit(json!([{"op":"remove", "id":5, "parent":2}]));
        pair.commit(json!([{"op":"text", "ids":[4], "values":["003"]}]));
    }

    #[test]
    fn image_arrivals_and_overlays_match_full_layout() {
        let mut pair = Pair::new();
        let mut initial = create(1, "Screen", json!({"title":"Images"}), 0);
        initial.extend(create(2, "Stack", json!({"gap":4}), 1));
        initial.extend(create(3, "Text", json!({"text":"Before images"}), 2));
        initial.extend(create(4, "Image", json!({"src":"https://example.com/a.png","width":200,"height":100,"preload":["https://example.com/b.png"],"onSwipe":true}), 2));
        initial.extend(create(5, "ScreenOverlay", json!({}), 1));
        initial.extend(create(6, "Text", json!({"text":"Overlay"}), 5));
        pair.commit(json!(initial));
        for engine in [&mut pair.incremental, &mut pair.full] {
            while let Some(request) = engine.take_native_request() {
                engine.complete_native_image(request.id(), 1, 1, vec![20, 40, 60, 255], None);
            }
        }
        pair.full.relayout_scene();
        pair.check();
        pair.commit(json!([{"op":"text","ids":[3],"values":["After images"]}]));
        pair.commit(json!([{"op":"hidden","id":5,"value":true}]));
        pair.commit(json!([{"op":"hidden","id":5,"value":false}]));
    }

    #[test]
    fn dynamic_children_keep_their_full_layout_side_effects() {
        let mut pair = Pair::new();
        let mut initial = create(1, "Screen", json!({"title":"Editor"}), 0);
        initial.extend(create(2, "Stack", json!({"gap":4}), 1));
        initial.extend(create(3, "Text", json!({"text":"Static neighbour"}), 2));
        initial.extend(create(4, "VideoView", json!({"controller":7}), 2));
        initial.extend(create(
            5,
            "PlayingProgress",
            json!({"position":10,"duration":100,"playing":false,"showTimes":true}),
            2,
        ));
        pair.commit(json!(initial));
        for step in 0..20 {
            pair.commit(json!([{"op":"text", "ids":[3], "values":[format!("Neighbour {step}")]}]));
            pair.incremental.relayout_scene();
            pair.full.relayout_scene();
            pair.check();
        }
    }

    #[test]
    fn list_height_changes_and_key_patches_match_full_layout() {
        let mut pair = Pair::new();
        let mut initial = create(1, "Screen", json!({"title":"List", "pinnedHeader":true}), 0);
        let mut keys: Vec<_> = (0..80).map(|index| index.to_string()).collect();
        let mut versions = vec![0; keys.len()];
        initial.extend(create(
            2,
            "List",
            json!({"keys":keys,"contentVersions":versions,"revision":0,"start":0,"gap":8}),
            1,
        ));
        for id in 10..90 {
            initial.extend(create(
                id,
                "Text",
                json!({"text":format!("Item {id}"),"onPress":true}),
                2,
            ));
        }
        pair.commit(json!(initial));
        for step in 0..40 {
            for engine in [&mut pair.incremental, &mut pair.full] {
                engine.scroll_by(100.0);
            }
            versions[step] += 1;
            pair.commit(json!([
                {"op":"text", "ids":[10 + step],"values":[format!("Changed {step}\nExtra height\nThird line")]},
                {"op":"update","id":2,"props":{"keys":keys,"contentVersions":versions,"revision":step+1,"start":0,"gap":8}}
            ]));
        }
        keys.reverse();
        versions.reverse();
        let mut reorder = vec![
            json!({"op":"update","id":2,"props":{"keys":keys,"contentVersions":versions,"revision":100,"start":0,"gap":8}}),
        ];
        for id in (10..90).rev() {
            reorder.push(json!({"op":"insert","id":id,"parent":2}));
        }
        pair.commit(json!(reorder));
        pair.commit(json!([{"op":"remove","id":40,"parent":2}, {"op":"update","id":2,"props":{"keys":keys.iter().filter(|key| *key != "30").collect::<Vec<_>>(),"contentVersions":vec![0;79],"revision":101,"start":0,"gap":8}}]));
    }
}
