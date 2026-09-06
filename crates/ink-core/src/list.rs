use std::collections::HashMap;

#[derive(Default)]
pub(super) struct ListMetrics {
    pub keys: Vec<String>,
    pub offsets: Vec<f32>,
    pub heights: HashMap<(String, u32, u64), f32>,
    widths: Vec<u32>,
    pub revision: u64,
    pub follow_end: bool,
}

impl ListMetrics {
    pub fn prepare_width(&mut self, width: f32) {
        let width = width.to_bits();
        if !self.widths.contains(&width) {
            self.widths.push(width);
            if self.widths.len() > 2 {
                let discarded = self.widths.remove(0);
                self.heights.retain(|(_, width, _), _| *width != discarded);
            }
        }
    }

    pub fn total(&self) -> f32 {
        self.offsets.last().copied().unwrap_or(0.0)
    }

    pub fn index_at(&self, offset: f32) -> usize {
        self.offsets
            .partition_point(|value| *value <= offset)
            .saturating_sub(1)
            .min(self.keys.len().saturating_sub(1))
    }

    pub fn window(&self, offset: f32, height: f32) -> (usize, usize) {
        let start = self.index_at((offset - height).max(0.0));
        let end = (self.index_at(offset + height * 2.0) + 1).min(self.keys.len());
        (start, end)
    }
}
