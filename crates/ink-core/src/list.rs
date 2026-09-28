use std::collections::HashMap;

#[derive(Default)]
pub(super) struct ListMetrics {
    pub keys: Vec<String>,
    pub mounted: std::ops::Range<usize>,
    versions: Vec<u64>,
    heights: HashMap<(String, u32, u64), f32>,
    widths: Vec<(u32, HeightIndex)>,
    active_width: usize,
    gap: f32,
    initial_estimate: f32,
    pub revision: u64,
    pub follow_end: bool,
}

// Fenwick sums track measured heights and counts separately. Changing the estimate
// for unseen rows then needs no rewrite of the remaining row offsets.
#[derive(Default)]
struct HeightIndex {
    measured: Vec<Option<f32>>,
    sums: Vec<f64>,
    counts: Vec<usize>,
    total: f64,
    count: usize,
}

impl HeightIndex {
    fn new(measured: Vec<Option<f32>>) -> Self {
        let mut index = Self {
            sums: vec![0.0; measured.len() + 1],
            counts: vec![0; measured.len() + 1],
            measured,
            ..Self::default()
        };
        for row in 1..index.sums.len() {
            if let Some(height) = index.measured[row - 1] {
                index.sums[row] += f64::from(height);
                index.counts[row] += 1;
                index.total += f64::from(height);
                index.count += 1;
            }
            let parent = row + (row & row.wrapping_neg());
            if parent < index.sums.len() {
                index.sums[parent] += index.sums[row];
                index.counts[parent] += index.counts[row];
            }
        }
        index
    }

    fn measure(&mut self, row: usize, height: f32) {
        let previous = self.measured[row];
        self.measured[row] = Some(height);
        let delta = f64::from(height) - f64::from(previous.unwrap_or(0.0));
        let count = usize::from(previous.is_none());
        self.total += delta;
        self.count += count;
        let mut node = row + 1;
        while node < self.sums.len() {
            self.sums[node] += delta;
            self.counts[node] += count;
            node += node & node.wrapping_neg();
        }
    }

    fn invalidate(&mut self, row: usize) {
        let Some(height) = self.measured[row].take() else { return; };
        self.total -= f64::from(height);
        self.count -= 1;
        let mut node = row + 1;
        while node < self.sums.len() {
            self.sums[node] -= f64::from(height);
            self.counts[node] -= 1;
            node += node & node.wrapping_neg();
        }
    }

    fn prefix(&self, mut end: usize) -> (f64, usize) {
        let (mut sum, mut count) = (0.0, 0);
        while end > 0 {
            sum += self.sums[end];
            count += self.counts[end];
            end &= end - 1;
        }
        (sum, count)
    }
}

impl ListMetrics {
    pub fn patch(&mut self, previous_revision: u64, revision: u64, rows: &[usize]) {
        if self.revision != previous_revision { return; }
        for &row in rows {
            for (width, index) in &mut self.widths {
                self.heights.remove(&(self.keys[row].clone(), *width, self.versions[row]));
                index.invalidate(row);
            }
            self.versions[row] = revision;
        }
        self.revision = revision;
    }

    pub fn prepare(
        &mut self,
        keys: &[String],
        versions: &[u64],
        revision: u64,
        width: f32,
        gap: f32,
        initial_estimate: f32,
    ) {
        if self.widths.is_empty() || self.revision != revision {
            let current: HashMap<_, _> = keys.iter().zip(versions).collect();
            self.heights.retain(|(key, _, version), _| {
                current
                    .get(key)
                    .is_some_and(|current| **current == *version)
            });
            keys.clone_into(&mut self.keys);
            versions.clone_into(&mut self.versions);
            for (width, index) in &mut self.widths {
                let measured = self
                    .keys
                    .iter()
                    .zip(&self.versions)
                    .map(|(key, version)| {
                        self.heights.get(&(key.clone(), *width, *version)).copied()
                    })
                    .collect();
                *index = HeightIndex::new(measured);
            }
            self.revision = revision;
        }
        self.gap = gap;
        self.initial_estimate = initial_estimate;
        let width = width.to_bits();
        self.active_width = if let Some(index) = self
            .widths
            .iter()
            .position(|(candidate, _)| *candidate == width)
        {
            index
        } else {
            if self.widths.len() == 2 {
                let (discarded, _) = self.widths.remove(0);
                self.heights.retain(|(_, width, _), _| *width != discarded);
            }
            let measured = self
                .keys
                .iter()
                .zip(&self.versions)
                .map(|(key, version)| self.heights.get(&(key.clone(), width, *version)).copied())
                .collect();
            self.widths.push((width, HeightIndex::new(measured)));
            self.widths.len() - 1
        };
    }

    pub fn measure(&mut self, row: usize, height: f32) {
        let (width, index) = &mut self.widths[self.active_width];
        if index.measured[row] != Some(height) {
            self.heights
                .insert((self.keys[row].clone(), *width, self.versions[row]), height);
            index.measure(row, height);
        }
    }

    fn estimate(&self) -> f64 {
        let index = &self.widths[self.active_width].1;
        if index.count == 0 {
            f64::from(self.initial_estimate)
        } else {
            index.total / index.count as f64
        }
    }

    pub fn offset(&self, row: usize) -> f32 {
        let (sum, count) = self.widths[self.active_width].1.prefix(row);
        (sum + (row - count) as f64 * self.estimate()
            + row.min(self.keys.len().saturating_sub(1)) as f64 * f64::from(self.gap))
            as f32
    }

    pub fn total(&self) -> f32 {
        self.offset(self.keys.len())
    }

    pub fn index_at(&self, offset: f32) -> usize {
        let index = &self.widths[self.active_width].1;
        let estimate = self.estimate();
        let (mut row, mut sum, mut count) = (0, 0.0, 0);
        let mut step = index.measured.len().next_power_of_two();
        while step > 0 {
            let next = row + step;
            if next <= index.measured.len() {
                let next_sum = sum + index.sums[next];
                let next_count = count + index.counts[next];
                let position = next_sum
                    + (next - next_count) as f64 * estimate
                    + next.min(self.keys.len().saturating_sub(1)) as f64 * f64::from(self.gap);
                if position as f32 <= offset {
                    row = next;
                    sum = next_sum;
                    count = next_count;
                }
            }
            step >>= 1;
        }
        row.min(self.keys.len().saturating_sub(1))
    }

    pub fn window(&self, offset: f32, height: f32) -> (usize, usize) {
        let start = self.index_at((offset - height * super::SCROLL_OVERSCAN).max(0.0));
        let end = (self.index_at(offset + height * (1.0 + super::SCROLL_OVERSCAN)) + 1).min(self.keys.len());
        (start, end)
    }
}
