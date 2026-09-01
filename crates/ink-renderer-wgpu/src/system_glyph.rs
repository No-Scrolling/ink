use std::collections::HashMap;

use anyhow::{Result, anyhow};
use ink_core::{Scene, is_emoji_grapheme};
use unicode_segmentation::UnicodeSegmentation;

pub(crate) const ATLAS_SIZE: u32 = 256;
const PADDING: u32 = 1;

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
    Pending(u64),
    Ready(CachedSystemGlyph),
    Unsupported,
}

struct AtlasPage {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

pub(crate) struct SystemGlyphAtlas {
    bind_group_layout: wgpu::BindGroupLayout,
    pages: Vec<AtlasPage>,
    glyphs: HashMap<SystemGlyphKey, SystemGlyphEntry>,
    next_request_id: u64,
    #[cfg(feature = "perf")]
    cache_misses: u64,
    #[cfg(feature = "perf")]
    uploaded_bytes: u64,
}

impl SystemGlyphAtlas {
    pub fn new(bind_group_layout: wgpu::BindGroupLayout) -> Self {
        Self {
            bind_group_layout,
            pages: Vec::new(),
            glyphs: HashMap::new(),
            next_request_id: 1,
            #[cfg(feature = "perf")]
            cache_misses: 0,
            #[cfg(feature = "perf")]
            uploaded_bytes: 0,
        }
    }

    pub fn request(&mut self, scene: &Scene) -> Option<SystemGlyphRequest> {
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
                match self.glyphs.get(&key) {
                    Some(SystemGlyphEntry::Pending(id)) => {
                        return Some(SystemGlyphRequest {
                            id: *id,
                            grapheme: key.grapheme,
                            pixel_size,
                        });
                    }
                    Some(SystemGlyphEntry::Ready(_) | SystemGlyphEntry::Unsupported) => continue,
                    None => {}
                }
                let id = self.next_request_id;
                self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
                self.glyphs
                    .insert(key.clone(), SystemGlyphEntry::Pending(id));
                #[cfg(feature = "perf")]
                {
                    self.cache_misses += 1;
                }
                return Some(SystemGlyphRequest {
                    id,
                    grapheme: key.grapheme,
                    pixel_size,
                });
            }
        }
        None
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

    pub fn bind_group(&self, page: usize) -> &wgpu::BindGroup {
        &self.pages[page].bind_group
    }

    pub fn install(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        request_id: u64,
        pixels: Option<&[u8]>,
    ) -> Result<()> {
        let Some(key) = self.glyphs.iter().find_map(|(key, entry)| match entry {
            SystemGlyphEntry::Pending(id) if *id == request_id => Some(key.clone()),
            _ => None,
        }) else {
            return Err(anyhow!("unknown system glyph request {request_id}"));
        };
        let size = u32::from(key.pixel_size);
        let expected_length = size as usize * size as usize * 4;
        let Some(pixels) = pixels.filter(|pixels| pixels.len() == expected_length) else {
            self.glyphs.insert(key, SystemGlyphEntry::Unsupported);
            return Ok(());
        };
        if size + PADDING * 2 > ATLAS_SIZE {
            self.glyphs.insert(key, SystemGlyphEntry::Unsupported);
            return Ok(());
        }

        let page = self
            .pages
            .iter_mut()
            .position(|page| page.can_fit(size))
            .unwrap_or_else(|| {
                self.pages
                    .push(AtlasPage::new(device, &self.bind_group_layout));
                self.pages.len() - 1
            });
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
    fn new(device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ink system glyph atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Ink system glyph sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ink system glyph bind group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
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

    fn write(&mut self, queue: &wgpu::Queue, pixels: &[u8], size: u32) -> (u32, u32) {
        let padded = size + PADDING * 2;
        if self.cursor_x + padded > ATLAS_SIZE {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }
        let atlas_x = self.cursor_x + PADDING;
        let atlas_y = self.cursor_y + PADDING;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: atlas_x,
                    y: atlas_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size * 4),
                rows_per_image: Some(size),
            },
            wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        );
        self.cursor_x += padded;
        self.row_height = self.row_height.max(padded);
        (atlas_x, atlas_y)
    }
}
