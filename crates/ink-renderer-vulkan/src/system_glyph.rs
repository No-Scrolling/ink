use crate::gpu;
use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{Result, anyhow, ensure};
use ink_core::{Scene, is_emoji_grapheme};
use unicode_segmentation::UnicodeSegmentation;

pub(crate) const ATLAS_SIZE: u32 = 256;
const PADDING: u32 = 1;
const MAX_ATLAS_PAGES: usize = 32;

#[derive(Debug)]
pub struct SystemGlyphRequest {
    pub id: u64,
    pub grapheme: String,
    pub pixel_size: u16,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct SystemGlyphKey {
    grapheme: String,
    pixel_size: u16,
}

#[derive(Clone, Copy)]
pub(crate) struct CachedSystemGlyph {
    pub page: usize,
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub size: u32,
}

enum SystemGlyphEntry {
    Ready(CachedSystemGlyph),
    Unsupported,
}

struct AtlasPage {
    texture: gpu::Texture,
    bind_group: gpu::BindGroup,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

pub(crate) struct SystemGlyphAtlas {
    pages: Vec<AtlasPage>,
    glyphs: HashMap<SystemGlyphKey, SystemGlyphEntry>,
    next_request_id: u64,
    revision: Option<u64>,
    remaining: VecDeque<SystemGlyphKey>,
    pending: Option<(SystemGlyphKey, u64)>,
    #[cfg(feature = "perf")]
    cache_misses: u64,
    #[cfg(feature = "perf")]
    uploaded_bytes: u64,
}

impl SystemGlyphAtlas {
    #[cfg(feature = "perf")]
    pub fn texture_capacity_bytes(&self) -> usize {
        self.pages.len() * ATLAS_SIZE as usize * ATLAS_SIZE as usize * 4
    }

    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            glyphs: HashMap::new(),
            next_request_id: 1,
            revision: None,
            remaining: VecDeque::new(),
            pending: None,
            #[cfg(feature = "perf")]
            cache_misses: 0,
            #[cfg(feature = "perf")]
            uploaded_bytes: 0,
        }
    }

    // Called after the previous GPU frame completes, before rebuilding prepared UVs.
    pub fn begin_scene(&mut self, scene: &Scene, queue: &gpu::Queue) -> bool {
        if self.revision == Some(scene.revision) {
            return false;
        }
        self.revision = Some(scene.revision);
        self.pending = None;
        self.remaining.clear();
        let mut needed = HashSet::new();
        for run in &scene.text {
            let pixel_size = run.font_size.round().clamp(1.0, u16::MAX as f32) as u16;
            for grapheme in run
                .text
                .graphemes(true)
                .filter(|value| is_emoji_grapheme(value))
            {
                let key = SystemGlyphKey {
                    grapheme: grapheme.to_owned(),
                    pixel_size,
                };
                if needed.insert(key.clone()) {
                    self.remaining.push_back(key);
                }
            }
        }
        self.glyphs.retain(|key, _| needed.contains(key));
        let mut used = vec![false; self.pages.len()];
        for entry in self.glyphs.values() {
            if let SystemGlyphEntry::Ready(glyph) = entry {
                used[glyph.page] = true;
            }
        }
        queue.discard_texture_uploads(
            &self
                .pages
                .iter()
                .zip(&used)
                .filter_map(|(page, used)| (!used).then_some(&page.texture))
                .collect::<Vec<_>>(),
        );
        let mut remap = vec![0; self.pages.len()];
        let mut next = 0;
        self.pages = self
            .pages
            .drain(..)
            .enumerate()
            .filter_map(|(index, page)| {
                if !used[index] {
                    return None;
                }
                remap[index] = next;
                next += 1;
                Some(page)
            })
            .collect();
        let changed = used.iter().any(|used| !used);
        for entry in self.glyphs.values_mut() {
            if let SystemGlyphEntry::Ready(glyph) = entry {
                glyph.page = remap[glyph.page];
            }
        }
        changed
    }

    pub fn request(&mut self, scene: &Scene, queue: &gpu::Queue) -> Option<SystemGlyphRequest> {
        self.begin_scene(scene, queue);
        if self.pending.is_none() {
            while let Some(key) = self.remaining.pop_front() {
                if self.glyphs.contains_key(&key) {
                    continue;
                }
                let id = self.next_request_id;
                self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
                self.pending = Some((key, id));
                #[cfg(feature = "perf")]
                {
                    self.cache_misses += 1;
                }
                break;
            }
        }
        self.pending.as_ref().map(|(key, id)| SystemGlyphRequest {
            id: *id,
            grapheme: key.grapheme.clone(),
            pixel_size: key.pixel_size,
        })
    }

    pub fn glyph(&self, grapheme: &str, pixel_size: u16) -> Option<CachedSystemGlyph> {
        let key = SystemGlyphKey {
            grapheme: grapheme.to_owned(),
            pixel_size,
        };
        match self.glyphs.get(&key) {
            Some(SystemGlyphEntry::Ready(glyph)) => Some(*glyph),
            _ => None,
        }
    }

    pub fn bind_group(&self, page: usize) -> &gpu::BindGroup {
        &self.pages[page].bind_group
    }

    pub fn install(
        &mut self,
        device: &gpu::Device,
        queue: &gpu::Queue,
        request_id: u64,
        pixels: Option<&[u8]>,
    ) -> Result<()> {
        let Some((key, id)) = self.pending.as_ref() else {
            return Err(anyhow!("unknown system glyph request {request_id}"));
        };
        ensure!(
            *id == request_id,
            "unknown system glyph request {request_id}"
        );
        let key = key.clone();
        let size = u32::from(key.pixel_size);
        let expected_length = size as usize * size as usize * 4;
        let Some(pixels) = pixels.filter(|pixels| pixels.len() == expected_length) else {
            self.pending = None;
            self.glyphs.insert(key, SystemGlyphEntry::Unsupported);
            return Ok(());
        };
        if size + PADDING * 2 > ATLAS_SIZE {
            self.pending = None;
            self.glyphs.insert(key, SystemGlyphEntry::Unsupported);
            return Ok(());
        }

        let page = match self.pages.iter().position(|page| page.can_fit(size)) {
            Some(page) => page,
            None => {
                ensure!(
                    self.pages.len() < MAX_ATLAS_PAGES,
                    "current scene exceeds the 8 MiB system glyph atlas limit"
                );
                self.pages.push(AtlasPage::new(device));
                self.pages.len() - 1
            }
        };
        self.pending = None;
        let (atlas_x, atlas_y) = self.pages[page].write(queue, pixels, size);
        self.glyphs.insert(
            key,
            SystemGlyphEntry::Ready(CachedSystemGlyph {
                page,
                atlas_x,
                atlas_y,
                size,
            }),
        );
        #[cfg(feature = "perf")]
        {
            self.uploaded_bytes += pixels.len() as u64;
        }
        Ok(())
    }

    #[cfg(feature = "perf")]
    pub fn take_perf(&mut self) -> (u64, u64) {
        (
            std::mem::take(&mut self.cache_misses),
            std::mem::take(&mut self.uploaded_bytes),
        )
    }
}

impl AtlasPage {
    fn new(device: &gpu::Device) -> Self {
        let texture = device.texture(ATLAS_SIZE, ATLAS_SIZE, false);
        let bind_group = texture.bind_group();
        Self {
            texture,
            bind_group,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
        }
    }

    fn can_fit(&self, size: u32) -> bool {
        let padded = size + PADDING * 2;
        (self.cursor_x + padded <= ATLAS_SIZE && self.cursor_y + padded <= ATLAS_SIZE)
            || (padded <= ATLAS_SIZE && self.cursor_y + self.row_height + padded <= ATLAS_SIZE)
    }

    fn write(&mut self, queue: &gpu::Queue, pixels: &[u8], size: u32) -> (u32, u32) {
        let padded = size + PADDING * 2;
        if self.cursor_x + padded > ATLAS_SIZE {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }
        let atlas_x = self.cursor_x + PADDING;
        let atlas_y = self.cursor_y + PADDING;
        queue.write_texture(&self.texture, [atlas_x, atlas_y], [size, size], pixels);
        self.cursor_x += padded;
        self.row_height = self.row_height.max(padded);
        (atlas_x, atlas_y)
    }
}
