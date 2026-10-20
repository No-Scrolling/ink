use std::collections::{HashMap, VecDeque};

#[derive(Clone, Copy, PartialEq)]
enum Queue {
    Filter,
    Staging,
    Core,
}

struct Entry {
    bytes: usize,
    frequency: u8,
    queue: Queue,
}
struct Ghost {
    bytes: usize,
    frequency: u8,
}

// Adapted from Merlin's FIFO/ghost policy: bounded sketch and ghost metadata,
// with large textures admitted to core instead of bypassing retention.
pub(super) struct Merlin {
    capacity: usize,
    entries: HashMap<u64, Entry>,
    ghosts: HashMap<u64, Ghost>,
    filter: VecDeque<u64>,
    staging: VecDeque<u64>,
    core: VecDeque<u64>,
    ghost: VecDeque<u64>,
    sketch: Box<[u8; 4096]>,
    guard: u8,
    evictions: usize,
    epochs: usize,
}

impl Merlin {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            ghosts: HashMap::new(),
            filter: VecDeque::new(),
            staging: VecDeque::new(),
            core: VecDeque::new(),
            ghost: VecDeque::new(),
            sketch: Box::new([0; 4096]),
            guard: 2,
            evictions: 0,
            epochs: 0,
        }
    }

    pub(super) fn contains(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    fn bytes(&self, queue: Queue) -> usize {
        self.entries
            .values()
            .filter(|entry| entry.queue == queue)
            .map(|entry| entry.bytes)
            .sum()
    }

    fn indices(id: u64) -> [usize; 4] {
        let hash = id.wrapping_mul(0x9e3779b97f4a7c15);
        std::array::from_fn(|index| index * 1024 + ((hash >> (index * 16)) as usize & 1023))
    }

    fn observe(&mut self, id: u64) {
        for index in Self::indices(id) {
            self.sketch[index] = self.sketch[index].saturating_add(1);
        }
    }

    fn popularity(&self, id: u64) -> u8 {
        Self::indices(id)
            .iter()
            .map(|&index| self.sketch[index])
            .min()
            .unwrap()
    }

    fn remember(&mut self, id: u64, bytes: usize, frequency: u8) {
        if self.ghosts.insert(id, Ghost { bytes, frequency }).is_none() {
            self.ghost.push_back(id);
        }
    }

    fn forget_ghost(&mut self, id: u64) {
        if self.ghosts.remove(&id).is_some() {
            self.ghost.retain(|key| *key != id);
        }
    }

    pub(super) fn forget(&mut self, id: u64) {
        self.entries.remove(&id);
        self.filter.retain(|key| *key != id);
        self.staging.retain(|key| *key != id);
        self.core.retain(|key| *key != id);
        self.forget_ghost(id);
    }

    fn update_guard(&mut self) {
        let population = |threshold| {
            self.entries
                .values()
                .filter(|entry| entry.frequency >= threshold)
                .count()
                + self
                    .ghosts
                    .values()
                    .filter(|entry| entry.frequency >= threshold)
                    .count()
        };
        while self.guard < 7 && population(self.guard) > self.core.len() {
            self.guard += 1;
        }
        while self.guard > 1 && population(self.guard - 1) < self.core.len() {
            self.guard -= 1;
        }
    }

    fn adjust_core(&mut self) {
        while self.bytes(Queue::Core) > self.capacity * 85 / 100 {
            let id = self.core.pop_front().unwrap();
            self.entries.get_mut(&id).unwrap().queue = Queue::Staging;
            self.staging.push_back(id);
        }
    }

    fn adjust_staging(&mut self) {
        while let Some(&id) = self.staging.front() {
            let entry = self.entries.get_mut(&id).unwrap();
            if entry.frequency == 0 {
                break;
            }
            entry.frequency -= 1;
            entry.queue = Queue::Core;
            self.staging.pop_front();
            self.core.push_back(id);
            self.observe(id);
            self.forget_ghost(id);
        }
    }

    fn filter_wins(&self, id: u64) -> bool {
        self.staging.front().is_some_and(|victim| {
            self.entries[victim].frequency == 0
                && self.popularity(id) <= 32
                && self.popularity(id) > self.popularity(*victim)
        })
    }

    fn evict_staging(&mut self) {
        let id = self.staging.pop_front().unwrap();
        self.entries.remove(&id);
    }

    fn adjust_filter(&mut self) -> bool {
        while let Some(id) = self.filter.pop_front() {
            let entry = self.entries.get_mut(&id).unwrap();
            if entry.frequency >= self.guard {
                entry.frequency = 0;
                entry.queue = Queue::Core;
                self.core.push_back(id);
                self.observe(id);
                continue;
            }
            let (bytes, frequency) = (entry.bytes, entry.frequency);
            let wins = self.filter_wins(id);
            self.remember(id, bytes, frequency);
            self.observe(id);
            if wins {
                let entry = self.entries.get_mut(&id).unwrap();
                entry.frequency = 0;
                entry.queue = Queue::Staging;
                self.staging.push_back(id);
                self.evict_staging();
            } else {
                self.entries.remove(&id);
            }
            return true;
        }
        false
    }

    fn evict(&mut self, incoming: usize) {
        self.update_guard();
        let mut removed = false;
        if self.bytes(Queue::Filter) + incoming > self.capacity / 10 {
            removed = self.adjust_filter();
        }
        while !removed {
            self.adjust_core();
            self.adjust_staging();
            if let Some(&id) = self.filter.front()
                && self.filter_wins(id)
            {
                removed = self.adjust_filter();
            }
            if !removed && !self.staging.is_empty() {
                self.evict_staging();
                removed = true;
            }
            if !removed && !self.filter.is_empty() {
                removed = self.adjust_filter();
            }
            if !removed && self.staging.is_empty() && self.filter.is_empty() {
                let id = self.core.pop_front().unwrap();
                self.entries.get_mut(&id).unwrap().queue = Queue::Staging;
                self.staging.push_back(id);
            }
        }
        while self.ghosts.values().map(|entry| entry.bytes).sum::<usize>() > self.capacity
            || self.ghosts.len() > 1024
        {
            let id = self.ghost.pop_front().unwrap();
            self.ghosts.remove(&id);
            self.observe(id);
        }
        self.evictions += 1;
        if self.evictions >= self.entries.len().max(1) {
            self.evictions = 0;
            self.epochs += 1;
            if self.epochs == 32 {
                self.epochs = 0;
                for count in self.sketch.iter_mut() {
                    *count /= 2;
                }
            }
        }
    }

    pub(super) fn access(&mut self, id: u64, bytes: usize) -> bool {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.frequency = (entry.frequency + 1).min(7);
            return false;
        }
        let historical = self.ghosts.get_mut(&id).map(|entry| {
            entry.frequency = (entry.frequency + 1).min(7);
            entry.frequency
        });
        if bytes > self.capacity {
            return true;
        }
        while self
            .entries
            .values()
            .map(|entry| entry.bytes)
            .sum::<usize>()
            + bytes
            > self.capacity
        {
            self.evict(bytes);
        }
        let (queue, frequency) = if bytes > self.capacity / 10 {
            self.forget_ghost(id);
            (Queue::Core, historical.unwrap_or(0))
        } else if self.filter.is_empty() && self.staging.is_empty() {
            (Queue::Core, 0)
        } else if historical.is_some_and(|frequency| frequency >= self.guard) {
            self.forget_ghost(id);
            (Queue::Core, historical.unwrap())
        } else if historical.is_some() {
            self.observe(id);
            (Queue::Staging, 0)
        } else {
            (Queue::Filter, 0)
        };
        self.entries.insert(
            id,
            Entry {
                bytes,
                frequency,
                queue,
            },
        );
        match queue {
            Queue::Filter => self.filter.push_back(id),
            Queue::Staging => self.staging.push_back(id),
            Queue::Core => self.core.push_back(id),
        }
        true
    }
}
