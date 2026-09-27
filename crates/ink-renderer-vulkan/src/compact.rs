use crate::gpu;
#[cfg(feature = "perf")]
use std::time::Instant;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
};

use ab_glyph::{Font, FontArc, FontRef, GlyphId, PxScale, ScaleFont};
use anyhow::{Context, Result, anyhow};
use bytemuck::{Pod, Zeroable};
use ink_core::{
    Colour, ImageAssetEncoding, ImageData, ImageFit, ImageRun, Mask, PUBLIC_SANS, Rect, Scene,
    TextAlign, TextRun, font_for_character, is_emoji_grapheme, tabular_digit_width,
    text_width_with_numbers,
};
#[cfg(feature = "perf")]
use ink_core::{PerfTraceSection, perf_trace_counter};

use crate::system_glyph::{
    ATLAS_SIZE as SYSTEM_GLYPH_ATLAS_SIZE, CachedSystemGlyph, SystemGlyphAtlas, SystemGlyphRequest,
};

const MAX_QUADS: usize = 64;
const MAX_GLYPHS: usize = 512;
const ATLAS_SIZE: u32 = 1024;
const MAX_ATLAS_SIZE: u32 = 4096;
const ATLAS_PADDING: u32 = 1;
const IMAGE_CACHE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C)]
struct QuadInstance {
    rect: [f32; 4],
    colour: [f32; 4],
}

#[derive(Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C)]
struct TextInstance {
    rect: [f32; 4],
    uv: [f32; 4],
    colour: [f32; 4],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct TransformUniform {
    translation: [f32; 4],
}

struct InstanceBuffer<T> {
    buffer: Option<gpu::Buffer>,
    capacity: usize,
    instances: Vec<T>,
    label: &'static str,
}

impl<T: Pod + PartialEq> InstanceBuffer<T> {
    #[cfg(feature = "perf")]
    fn capacity_bytes(&self) -> usize {
        self.capacity * size_of::<T>()
    }

    fn new(device: &gpu::Device, label: &'static str, capacity: usize) -> Self {
        Self {
            buffer: Some(raw_instance_buffer::<T>(device, label, capacity)),
            capacity,
            instances: Vec::new(),
            label,
        }
    }

    fn lazy(label: &'static str) -> Self {
        Self {
            buffer: None,
            capacity: 0,
            instances: Vec::new(),
            label,
        }
    }

    fn buffer(&self) -> &gpu::Buffer {
        self.buffer
            .as_ref()
            .expect("instances uploaded before drawing")
    }

    fn write(&mut self, device: &gpu::Device, queue: &gpu::Queue, instances: &[T]) -> usize {
        let resized = instances.len() > self.capacity;
        if resized {
            self.capacity = instances.len().next_power_of_two();
            self.buffer = Some(raw_instance_buffer::<T>(device, self.label, self.capacity));
        }
        let start = if resized {
            0
        } else {
            self.instances
                .iter()
                .zip(instances)
                .position(|(current, next)| current != next)
                .unwrap_or(self.instances.len().min(instances.len()))
        };
        let end = if resized || self.instances.len() != instances.len() {
            instances.len()
        } else {
            self.instances
                .iter()
                .zip(instances)
                .rposition(|(current, next)| current != next)
                .map_or(start, |index| index + 1)
        };
        let uploaded_bytes = if start < end {
            queue.write_buffer(
                self.buffer(),
                (start * size_of::<T>()) as u64,
                bytemuck::cast_slice(&instances[start..end]),
            );
            (end - start) * size_of::<T>()
        } else {
            0
        };
        self.instances.clear();
        self.instances.extend_from_slice(instances);
        uploaded_bytes
    }
}

#[derive(Default)]
struct PreparedScene {
    revision: u64,
    image_revision: u64,
    width: u32,
    height: u32,
    ready: bool,
    quads: Vec<ink_core::Quad>,
    masks: Vec<PreparedMaskRun>,
    images: Vec<PreparedImageRun>,
    preloaded_images: Vec<(u64, u64)>,
    quad_fixed: Range<u32>,
    quad_scroll: Range<u32>,
    text_fixed: Range<u32>,
    text_scroll: Range<u32>,
    image_draws: Vec<ImageDraw>,
    system_glyph_draws: Vec<SystemGlyphDraw>,
    text_runs: Vec<PreparedTextRun>,
}

#[derive(Clone, Copy, PartialEq)]
struct PreparedMaskRun {
    id: u64,
    width: u16,
    height: u16,
    rect: Rect,
    clip: Rect,
    colour: Colour,
    scrolling: bool,
}

impl From<&ink_core::MaskRun> for PreparedMaskRun {
    fn from(run: &ink_core::MaskRun) -> Self {
        Self {
            id: run.mask.id,
            width: run.mask.width,
            height: run.mask.height,
            rect: run.rect,
            clip: run.clip,
            colour: run.colour,
            scrolling: run.scrolling,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct PreparedImageRun {
    id: u64,
    generation: u64,
    zoom_id: Option<usize>,
    rect: Rect,
    clip: Rect,
    fit: ImageFit,
    scrolling: bool,
    transform: ink_core::ImageTransform,
}

impl From<&ImageRun> for PreparedImageRun {
    fn from(run: &ImageRun) -> Self {
        Self {
            id: run.image.id(),
            generation: run.image.generation(),
            zoom_id: run.zoom_id,
            rect: run.rect,
            clip: run.clip,
            fit: run.fit,
            scrolling: run.scrolling,
            transform: run.transform,
        }
    }
}

struct PreparedTextRun {
    run: TextRun,
    text: Vec<TextInstance>,
    system_glyphs: Vec<SystemGlyphInstances>,
}

struct SystemGlyphInstances {
    page: usize,
    instances: Vec<TextInstance>,
}

#[derive(Clone, Copy)]
struct CachedGlyph {
    atlas_x: u32,
    atlas_y: u32,
    width: u32,
    height: u32,
    offset_x: f32,
    offset_y: f32,
}

#[derive(Clone, Copy)]
struct CachedCharacter {
    font_index: usize,
    font: &'static FontArc,
    id: GlyphId,
    advance: f32,
    bitmap: Option<CachedGlyph>,
}

#[derive(Clone, Copy)]
struct CachedMask {
    atlas_x: u32,
    atlas_y: u32,
    width: u32,
    height: u32,
}

struct CachedImage {
    _texture: gpu::Texture,
    bind_group: gpu::BindGroup,
    width: u32,
    height: u32,
    generation: u64,
    bytes: usize,
    last_used: u64,
}

struct ImageCache {
    images: HashMap<u64, CachedImage>,
    frame: u64,
    #[cfg(feature = "perf")]
    cache_misses: u64,
    #[cfg(feature = "perf")]
    uploaded_bytes: u64,
}

impl ImageCache {
    fn new() -> Self {
        Self {
            images: HashMap::new(),
            frame: 0,
            #[cfg(feature = "perf")]
            cache_misses: 0,
            #[cfg(feature = "perf")]
            uploaded_bytes: 0,
        }
    }

    fn prepare(
        &mut self,
        device: &gpu::Device,
        queue: &gpu::Queue,
        image: &ImageData,
    ) -> Result<&CachedImage> {
        let id = image.id();
        let generation = image.generation();
        if self
            .images
            .get(&id)
            .is_some_and(|cached| cached.generation != generation)
        {
            self.images.remove(&id);
        }
        if !self.images.contains_key(&id) {
            let (pixels, width, height) = match image {
                ImageData::Asset(asset) => match asset.encoding {
                    ImageAssetEncoding::RgbaZlib => (
                        Cow::Owned(
                            miniz_oxide::inflate::decompress_to_vec_zlib(asset.bytes.as_ref())
                                .map_err(|error| {
                                    anyhow!("an Ink image could not be decompressed: {error:?}")
                                })?,
                        ),
                        asset.width,
                        asset.height,
                    ),
                    ImageAssetEncoding::Jpeg => {
                        let (width, height, pixels) = decode_encoded_image(asset.bytes.as_ref())?;
                        (Cow::Owned(pixels), width, height)
                    }
                },
                ImageData::Remote(image) => (
                    Cow::Borrowed(image.pixels.as_ref()),
                    image.width,
                    image.height,
                ),
            };
            if width == 0 || height == 0 {
                return Err(anyhow!("an Ink image has invalid dimensions"));
            }
            let expected_length = width as usize * height as usize * 4;
            if pixels.len() != expected_length {
                return Err(anyhow!("an Ink image has invalid pixel data"));
            }
            let texture = device.texture(width, height, false);
            queue.write_texture(&texture, [0, 0], [width, height], &pixels);
            let bind_group = texture.bind_group();
            self.images.insert(
                id,
                CachedImage {
                    _texture: texture,
                    bind_group,
                    width,
                    height,
                    generation,
                    bytes: expected_length,
                    last_used: self.frame,
                },
            );
            #[cfg(feature = "perf")]
            {
                self.cache_misses += 1;
                self.uploaded_bytes += expected_length as u64;
            }
        }
        self.images
            .get_mut(&id)
            .map(|image| {
                image.last_used = self.frame;
                &*image
            })
            .context("an Ink image was not cached")
    }

    fn begin_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    #[cfg(feature = "perf")]
    fn take_perf(&mut self) -> (u64, u64) {
        (
            std::mem::take(&mut self.cache_misses),
            std::mem::take(&mut self.uploaded_bytes),
        )
    }

    fn trim(&mut self, protected: &HashSet<u64>) {
        let mut bytes = self.images.values().map(|image| image.bytes).sum::<usize>();
        while bytes > IMAGE_CACHE_BYTES {
            let candidate = self
                .images
                .iter()
                .filter(|(id, _)| !protected.contains(id))
                .min_by_key(|(_, image)| image.last_used)
                .map(|(id, _)| *id);
            let Some(id) = candidate else {
                break;
            };
            if let Some(image) = self.images.remove(&id) {
                bytes = bytes.saturating_sub(image.bytes);
            }
        }
    }
}

#[cfg(target_os = "android")]
fn decode_encoded_image(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    crate::android_image::decode(bytes)
}

#[cfg(not(target_os = "android"))]
fn decode_encoded_image(_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    Err(anyhow!("encoded bundled images require Android"))
}

struct ImageDraw {
    id: u64,
    instances: Range<u32>,
    scrolling: bool,
}

struct SystemGlyphDraw {
    page: usize,
    instances: Range<u32>,
    scrolling: bool,
}

#[derive(Debug)]
struct AtlasFull;

impl std::fmt::Display for AtlasFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the Ink glyph atlas is full")
    }
}

impl std::error::Error for AtlasFull {}

struct GlyphAtlas {
    size: u32,
    font: FontRef<'static>,
    texture: gpu::Texture,
    bind_group: gpu::BindGroup,
    glyphs: HashMap<(usize, GlyphId, u16), CachedGlyph>,
    characters: HashMap<(char, u16), CachedCharacter>,
    masks: HashMap<u64, CachedMask>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    #[cfg(feature = "perf")]
    cache_misses: u64,
    #[cfg(feature = "perf")]
    uploaded_bytes: u64,
}

impl GlyphAtlas {
    fn new(device: &gpu::Device, size: u32) -> Result<Self> {
        let font = FontRef::try_from_slice(PUBLIC_SANS).context("Public Sans is invalid")?;
        let texture = device.texture(size, size, true);
        let bind_group = texture.bind_group();

        Ok(Self {
            size,
            font,
            texture,
            bind_group,
            glyphs: HashMap::new(),
            characters: HashMap::new(),
            masks: HashMap::new(),
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            #[cfg(feature = "perf")]
            cache_misses: 0,
            #[cfg(feature = "perf")]
            uploaded_bytes: 0,
        })
    }

    fn character(&mut self, queue: &gpu::Queue, character: char, size: u16) -> Result<CachedCharacter> {
        if let Some(cached) = self.characters.get(&(character, size)) {
            return Ok(*cached);
        }
        let (font_index, font) = font_for_character(character);
        let id = font.glyph_id(character);
        let cached = CachedCharacter {
            font_index,
            font,
            id,
            advance: font.as_scaled(PxScale::from(size as f32)).h_advance(id),
            bitmap: self.glyph(queue, font_index, font, id, size)?,
        };
        if self.characters.len() == 4096 { self.characters.clear(); }
        self.characters.insert((character, size), cached);
        Ok(cached)
    }

    fn glyph(
        &mut self,
        queue: &gpu::Queue,
        font_index: usize,
        font: &impl Font,
        id: GlyphId,
        size: u16,
    ) -> Result<Option<CachedGlyph>> {
        if let Some(glyph) = self.glyphs.get(&(font_index, id, size)) {
            return Ok(Some(*glyph));
        }

        let Some(outlined) = font.outline_glyph(id.with_scale(size as f32)) else {
            return Ok(None);
        };
        let bounds = outlined.px_bounds();
        let glyph_width = bounds.width() as u32;
        let glyph_height = bounds.height() as u32;
        if glyph_width == 0 || glyph_height == 0 {
            return Ok(None);
        }
        let width = glyph_width + ATLAS_PADDING * 2;
        let height = glyph_height + ATLAS_PADDING * 2;
        if width > self.size || height > self.size {
            return Err(AtlasFull.into());
        }
        if self.cursor_x + width > self.size {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }
        if self.cursor_y + height > self.size {
            return Err(AtlasFull.into());
        }

        let mut pixels = vec![0; (glyph_width * glyph_height) as usize];
        outlined.draw(|x, y, coverage| {
            pixels[(y * glyph_width + x) as usize] = (coverage * 255.0).round() as u8;
        });
        queue.write_texture(
            &self.texture,
            [self.cursor_x + ATLAS_PADDING, self.cursor_y + ATLAS_PADDING],
            [glyph_width, glyph_height],
            &pixels,
        );

        let glyph = CachedGlyph {
            atlas_x: self.cursor_x,
            atlas_y: self.cursor_y,
            width,
            height,
            offset_x: bounds.min.x - ATLAS_PADDING as f32,
            offset_y: bounds.min.y - ATLAS_PADDING as f32,
        };
        self.cursor_x += width;
        self.row_height = self.row_height.max(height);
        self.glyphs.insert((font_index, id, size), glyph);
        #[cfg(feature = "perf")]
        {
            self.cache_misses += 1;
            self.uploaded_bytes += pixels.len() as u64;
        }
        Ok(Some(glyph))
    }

    fn mask(&mut self, queue: &gpu::Queue, mask: &Mask) -> Result<CachedMask> {
        if let Some(mask) = self.masks.get(&mask.id) {
            return Ok(*mask);
        }

        let width = u32::from(mask.width);
        let height = u32::from(mask.height);
        if mask.pixels.as_ref().len() != (width * height) as usize {
            return Err(anyhow!("an Ink mask has invalid dimensions"));
        }
        let padded_width = width + ATLAS_PADDING * 2;
        let padded_height = height + ATLAS_PADDING * 2;
        if padded_width > self.size || padded_height > self.size {
            return Err(AtlasFull.into());
        }
        if self.cursor_x + padded_width > self.size {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }
        if self.cursor_y + padded_height > self.size {
            return Err(AtlasFull.into());
        }

        queue.write_texture(
            &self.texture,
            [self.cursor_x + ATLAS_PADDING, self.cursor_y + ATLAS_PADDING],
            [width, height],
            mask.pixels.as_ref(),
        );

        let cached = CachedMask {
            atlas_x: self.cursor_x + ATLAS_PADDING,
            atlas_y: self.cursor_y + ATLAS_PADDING,
            width,
            height,
        };
        self.cursor_x += padded_width;
        self.row_height = self.row_height.max(padded_height);
        self.masks.insert(mask.id, cached);
        #[cfg(feature = "perf")]
        {
            self.cache_misses += 1;
            self.uploaded_bytes += mask.pixels.as_ref().len() as u64;
        }
        Ok(cached)
    }

    #[cfg(feature = "perf")]
    fn take_perf(&mut self) -> (u64, u64) {
        (
            std::mem::take(&mut self.cache_misses),
            std::mem::take(&mut self.uploaded_bytes),
        )
    }
}

#[derive(Debug)]
pub enum RenderOutcome {
    Presented,
    Skipped,
    SurfaceLost,
    NeedsSystemGlyph(SystemGlyphRequest),
}

#[cfg(feature = "perf")]
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderPerfMetrics {
    pub gpu_ns: Option<u64>,
    pub prepare_ns: u64,
    pub upload_ns: u64,
    pub acquire_ns: u64,
    pub encode_ns: u64,
    pub submit_present_ns: u64,
    pub frame_ns: u64,
    pub instances: u64,
    pub uploaded_bytes: u64,
    pub draw_calls: u64,
    pub cache_misses: u64,
}

#[cfg(feature = "perf")]
pub struct RenderMemoryMetrics {
    pub instance_buffer_capacity_bytes: usize,
    pub instance_snapshot_capacity_bytes: usize,
    pub font_texture_bytes: usize,
    pub image_texture_bytes: usize,
    pub system_glyph_texture_bytes: usize,
}

pub struct Renderer {
    surface: gpu::Surface,
    device: gpu::Device,
    queue: gpu::Queue,
    config: gpu::SurfaceConfiguration,
    fixed_transform: gpu::BindGroup,
    scroll_transform: gpu::BindGroup,
    scroll_transform_buffer: gpu::Buffer,
    quad_pipeline: gpu::RenderPipeline,
    quad_buffer: InstanceBuffer<QuadInstance>,
    overlay_buffer: InstanceBuffer<QuadInstance>,
    overlay_instances: Vec<QuadInstance>,
    text_pipeline: gpu::RenderPipeline,
    text_buffer: InstanceBuffer<TextInstance>,
    image_buffer: InstanceBuffer<TextInstance>,
    image_cache: ImageCache,
    system_glyph_buffer: InstanceBuffer<TextInstance>,
    system_glyph_atlas: SystemGlyphAtlas,
    glyph_atlas: GlyphAtlas,
    prepared: PreparedScene,
    #[cfg(feature = "perf")]
    perf: RenderPerfMetrics,
}

impl Renderer {
    pub unsafe fn new(window: *mut std::ffi::c_void, width: u32, height: u32) -> Result<Self> {
        let surface = unsafe { gpu::Surface::new(window, width, height)? };
        let device = surface.device();
        let queue = device.clone();
        let config = gpu::SurfaceConfiguration {
            width,
            height,
            format: surface.format(),
        };
        let quad_pipeline = device.pipeline(config.format, false)?;
        let text_pipeline = device.pipeline(config.format, true)?;
        let fixed_transform_buffer = transform_buffer(&device, "Ink fixed transform");
        let scroll_transform_buffer = transform_buffer(&device, "Ink scroll transform");
        let fixed_transform = fixed_transform_buffer.bind_group();
        let scroll_transform = scroll_transform_buffer.bind_group();
        queue.write_buffer(
            &fixed_transform_buffer,
            0,
            bytemuck::bytes_of(&TransformUniform {
                translation: [0.0; 4],
            }),
        );
        let quad_buffer = InstanceBuffer::new(&device, "Ink quad instances", MAX_QUADS);
        let overlay_buffer = InstanceBuffer::new(&device, "Ink overlay instances", 3);
        let text_buffer = InstanceBuffer::new(&device, "Ink text instances", MAX_GLYPHS);
        let image_buffer = InstanceBuffer::lazy("Ink image instances");
        let system_glyph_atlas = SystemGlyphAtlas::new();
        let image_cache = ImageCache::new();
        let system_glyph_buffer = InstanceBuffer::lazy("Ink system glyph instances");
        let glyph_atlas = GlyphAtlas::new(&device, ATLAS_SIZE)?;

        Ok(Self {
            surface,
            device,
            queue,
            config,
            fixed_transform,
            scroll_transform,
            scroll_transform_buffer,
            quad_pipeline,
            quad_buffer,
            overlay_buffer,
            overlay_instances: Vec::with_capacity(2),
            text_pipeline,
            text_buffer,
            image_buffer,
            image_cache,
            system_glyph_buffer,
            system_glyph_atlas,
            glyph_atlas,
            prepared: PreparedScene::default(),
            #[cfg(feature = "perf")]
            perf: RenderPerfMetrics::default(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.resize(self.config.width, self.config.height);
    }

    pub fn frame_ready(&self) -> bool {
        // Let render report device errors through its existing recovery path.
        self.surface.frame_ready().unwrap_or(true)
    }

    pub fn render(&mut self, scene: &Scene) -> Result<RenderOutcome> {
        match self.render_frame(scene) {
            Err(error) if gpu::surface_lost(&error) => Ok(RenderOutcome::SurfaceLost),
            result => result,
        }
    }

    fn render_frame(&mut self, scene: &Scene) -> Result<RenderOutcome> {
        #[cfg(feature = "perf")]
        let frame_started = Instant::now();
        #[cfg(feature = "perf")]
        let frame_trace = PerfTraceSection::new(b"Ink frame\0");
        // Shared buffers and cached textures cannot change until the previous draw completes.
        #[cfg(feature = "perf")]
        let wait_trace = PerfTraceSection::new(b"Ink previous frame\0");
        self.surface.wait_for_frame()?;
        #[cfg(feature = "perf")]
        drop(wait_trace);
        if scene.width == 0 || scene.height == 0 {
            return Ok(RenderOutcome::Skipped);
        }
        if self.config.width != scene.width || self.config.height != scene.height {
            self.resize(scene.width, scene.height);
        }
        if !self.prepared.ready || self.prepared.revision != scene.revision {
            if let Some(request) = self.system_glyph_atlas.request(scene) {
                return Ok(RenderOutcome::NeedsSystemGlyph(request));
            }
            #[cfg(feature = "perf")]
            let prepare_started = Instant::now();
            #[cfg(feature = "perf")]
            let prepare_trace = PerfTraceSection::new(b"Ink prepare\0");
            self.prepare_with_atlas_recovery(scene)?;
            #[cfg(feature = "perf")]
            {
                drop(prepare_trace);
                self.perf.prepare_ns += elapsed_ns(prepare_started);
            }
        } else if self.prepared.image_revision != scene.image_revision {
            #[cfg(feature = "perf")]
            let prepare_started = Instant::now();
            #[cfg(feature = "perf")]
            let prepare_trace = PerfTraceSection::new(b"Ink prepare\0");
            self.refresh_images(scene)?;
            #[cfg(feature = "perf")]
            {
                drop(prepare_trace);
                self.perf.prepare_ns += elapsed_ns(prepare_started);
            }
        }
        #[cfg(feature = "perf")]
        let upload_started = Instant::now();
        #[cfg(feature = "perf")]
        let upload_trace = PerfTraceSection::new(b"Ink upload\0");
        let scroll_y = if scene.height == 0 {
            0.0
        } else {
            (scene.scroll_offset - scene.scroll_origin) * 2.0 / scene.height as f32
        };
        self.queue.write_buffer(
            &self.scroll_transform_buffer,
            0,
            bytemuck::bytes_of(&TransformUniform {
                translation: [0.0, scroll_y, 0.0, 0.0],
            }),
        );
        self.overlay_instances.clear();
        push_scrollbar_instances(scene, &mut self.overlay_instances);
        let overlay_end = self.overlay_instances.len() as u32;
        let _overlay_uploaded_bytes =
            self.overlay_buffer
                .write(&self.device, &self.queue, &self.overlay_instances);
        #[cfg(feature = "perf")]
        {
            drop(upload_trace);
            self.perf.upload_ns += elapsed_ns(upload_started);
            self.perf.uploaded_bytes +=
                (size_of::<TransformUniform>() + _overlay_uploaded_bytes) as u64;
            let (image_misses, image_bytes) = self.image_cache.take_perf();
            let (glyph_misses, glyph_bytes) = self.glyph_atlas.take_perf();
            let (system_glyph_misses, system_glyph_bytes) = self.system_glyph_atlas.take_perf();
            self.perf.cache_misses += image_misses + glyph_misses + system_glyph_misses;
            self.perf.uploaded_bytes += image_bytes + glyph_bytes + system_glyph_bytes;
        }

        #[cfg(feature = "perf")]
        let acquire_started = Instant::now();
        #[cfg(feature = "perf")]
        let acquire_trace = PerfTraceSection::new(b"Ink acquire\0");
        let Some(mut pass) = self.surface.begin(scene.light)? else {
            return Ok(RenderOutcome::Skipped);
        };
        #[cfg(feature = "perf")]
        {
            drop(acquire_trace);
            self.perf.acquire_ns += elapsed_ns(acquire_started);
        }
        #[cfg(feature = "perf")]
        let upload_started = Instant::now();
        #[cfg(feature = "perf")]
        let upload_trace = PerfTraceSection::new(b"Ink upload\0");
        pass.upload(&self.queue);
        #[cfg(feature = "perf")]
        {
            drop(upload_trace);
            self.perf.upload_ns += elapsed_ns(upload_started);
        }
        #[cfg(feature = "perf")]
        let encode_started = Instant::now();
        #[cfg(feature = "perf")]
        let encode_trace = PerfTraceSection::new(b"Ink encode\0");
        {
            if !self.prepared.quad_fixed.is_empty() {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_bind_group(0, &self.fixed_transform);
                pass.set_vertex_buffer(0, self.quad_buffer.buffer());
                pass.draw(0..6, self.prepared.quad_fixed.clone());
            }
            if !self.prepared.quad_scroll.is_empty() {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_bind_group(0, &self.scroll_transform);
                pass.set_vertex_buffer(0, self.quad_buffer.buffer());
                set_scroll_scissor(&mut pass, scene);
                pass.draw(0..6, self.prepared.quad_scroll.clone());
                reset_scissor(&mut pass, scene);
            }
            if !self.prepared.image_draws.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_vertex_buffer(0, self.image_buffer.buffer());
                for draw in &self.prepared.image_draws {
                    let image = self
                        .image_cache
                        .images
                        .get(&draw.id)
                        .expect("prepared image stays cached");
                    pass.set_bind_group(
                        0,
                        if draw.scrolling {
                            &self.scroll_transform
                        } else {
                            &self.fixed_transform
                        },
                    );
                    pass.set_bind_group(1, &image.bind_group);
                    if draw.scrolling {
                        set_scroll_scissor(&mut pass, scene);
                    } else {
                        reset_scissor(&mut pass, scene);
                    }
                    pass.draw(0..6, draw.instances.clone());
                }
                reset_scissor(&mut pass, scene);
            }
            if !self.prepared.system_glyph_draws.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_vertex_buffer(0, self.system_glyph_buffer.buffer());
                for draw in &self.prepared.system_glyph_draws {
                    pass.set_bind_group(
                        0,
                        if draw.scrolling {
                            &self.scroll_transform
                        } else {
                            &self.fixed_transform
                        },
                    );
                    pass.set_bind_group(1, self.system_glyph_atlas.bind_group(draw.page));
                    if draw.scrolling {
                        set_scroll_scissor(&mut pass, scene);
                    } else {
                        reset_scissor(&mut pass, scene);
                    }
                    pass.draw(0..6, draw.instances.clone());
                }
                reset_scissor(&mut pass, scene);
            }
            if !self.prepared.text_fixed.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.fixed_transform);
                pass.set_bind_group(1, &self.glyph_atlas.bind_group);
                pass.set_vertex_buffer(0, self.text_buffer.buffer());
                pass.draw(0..6, self.prepared.text_fixed.clone());
            }
            if !self.prepared.text_scroll.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.scroll_transform);
                pass.set_bind_group(1, &self.glyph_atlas.bind_group);
                pass.set_vertex_buffer(0, self.text_buffer.buffer());
                set_scroll_scissor(&mut pass, scene);
                pass.draw(0..6, self.prepared.text_scroll.clone());
                reset_scissor(&mut pass, scene);
            }
            if overlay_end > 0 {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_bind_group(0, &self.fixed_transform);
                pass.set_vertex_buffer(0, self.overlay_buffer.buffer());
                pass.draw(0..6, 0..overlay_end);
            }
        }

        #[cfg(feature = "perf")]
        {
            drop(encode_trace);
            self.perf.encode_ns += elapsed_ns(encode_started);
            let range_length = |range: &Range<u32>| u64::from(range.end - range.start);
            let mut instances = range_length(&self.prepared.quad_fixed)
                + range_length(&self.prepared.quad_scroll)
                + range_length(&self.prepared.text_fixed)
                + range_length(&self.prepared.text_scroll)
                + u64::from(overlay_end);
            instances += self
                .prepared
                .image_draws
                .iter()
                .map(|draw| range_length(&draw.instances))
                .sum::<u64>();
            instances += self
                .prepared
                .system_glyph_draws
                .iter()
                .map(|draw| range_length(&draw.instances))
                .sum::<u64>();
            let draw_calls = u64::from(!self.prepared.quad_fixed.is_empty())
                + u64::from(!self.prepared.quad_scroll.is_empty())
                + self.prepared.image_draws.len() as u64
                + self.prepared.system_glyph_draws.len() as u64
                + u64::from(!self.prepared.text_fixed.is_empty())
                + u64::from(!self.prepared.text_scroll.is_empty())
                + u64::from(overlay_end > 0);
            self.perf.instances += instances;
            self.perf.draw_calls += draw_calls;
            perf_trace_counter(b"Ink instances\0", instances);
            perf_trace_counter(b"Ink uploaded bytes\0", self.perf.uploaded_bytes);
            perf_trace_counter(b"Ink draw calls\0", draw_calls);
            perf_trace_counter(b"Ink cache misses\0", self.perf.cache_misses);
        }
        #[cfg(feature = "perf")]
        let submit_present_started = Instant::now();
        #[cfg(feature = "perf")]
        let submit_present_trace = PerfTraceSection::new(b"Ink submit and present\0");
        pass.finish()?;
        #[cfg(feature = "perf")]
        {
            drop(submit_present_trace);
            self.perf.submit_present_ns += elapsed_ns(submit_present_started);
            self.perf.frame_ns += elapsed_ns(frame_started);
            drop(frame_trace);
        }
        Ok(RenderOutcome::Presented)
    }

    #[cfg(feature = "perf")]
    pub fn take_perf_metrics(&mut self) -> RenderPerfMetrics {
        self.perf.gpu_ns = self.surface.gpu_ns.take();
        std::mem::take(&mut self.perf)
    }

    #[cfg(feature = "presentation-timing")]
    pub fn presentation_times(&self) -> Result<Option<Vec<(u32, u64)>>> {
        self.surface.presentation_times()
    }

    #[cfg(feature = "presentation-timing")]
    pub fn present_id(&self) -> u32 {
        self.surface.present_id
    }

    #[cfg(feature = "perf")]
    pub fn memory_metrics(&self) -> RenderMemoryMetrics {
        RenderMemoryMetrics {
            instance_buffer_capacity_bytes: self.quad_buffer.capacity_bytes()
                + self.overlay_buffer.capacity_bytes()
                + self.text_buffer.capacity_bytes()
                + self.image_buffer.capacity_bytes()
                + self.system_glyph_buffer.capacity_bytes(),
            instance_snapshot_capacity_bytes: (self.quad_buffer.instances.capacity()
                + self.overlay_buffer.instances.capacity())
                * size_of::<QuadInstance>()
                + (self.text_buffer.instances.capacity()
                    + self.image_buffer.instances.capacity()
                    + self.system_glyph_buffer.instances.capacity())
                    * size_of::<TextInstance>(),
            font_texture_bytes: self.glyph_atlas.size as usize * self.glyph_atlas.size as usize,
            image_texture_bytes: self
                .image_cache
                .images
                .values()
                .map(|image| image.bytes)
                .sum(),
            system_glyph_texture_bytes: self.system_glyph_atlas.texture_capacity_bytes(),
        }
    }

    pub fn install_system_glyph(&mut self, request_id: u64, pixels: Option<&[u8]>) -> Result<()> {
        self.surface.wait_for_frame()?;
        self.system_glyph_atlas
            .install(&self.device, &self.queue, request_id, pixels)
    }

    fn prepare_with_atlas_recovery(&mut self, scene: &Scene) -> Result<()> {
        let mut reclaimed = false;
        loop {
            match self.prepare(scene) {
                Ok(()) => return Ok(()),
                Err(error) if error.is::<AtlasFull>() => {
                    // The previous GPU frame has finished. Rebuild every cached UV before drawing.
                    self.prepared = PreparedScene::default();
                    if !reclaimed {
                        self.glyph_atlas.glyphs.clear();
                        self.glyph_atlas.characters.clear();
                        self.glyph_atlas.masks.clear();
                        self.glyph_atlas.cursor_x = 0;
                        self.glyph_atlas.cursor_y = 0;
                        self.glyph_atlas.row_height = 0;
                        let size = self.glyph_atlas.size;
                        self.queue.write_texture(
                            &self.glyph_atlas.texture,
                            [0, 0],
                            [size, size],
                            &vec![0; size as usize * size as usize],
                        );
                        reclaimed = true;
                    } else if self.glyph_atlas.size < MAX_ATLAS_SIZE {
                        self.glyph_atlas = GlyphAtlas::new(&self.device, self.glyph_atlas.size * 2)?;
                    } else {
                        return Err(
                            error.context("current scene exceeds the 16 MiB glyph atlas limit")
                        );
                    }
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn prepare(&mut self, scene: &Scene) -> Result<()> {
        let viewport_changed =
            self.prepared.width != scene.width || self.prepared.height != scene.height;
        if viewport_changed {
            self.prepared.text_runs.clear();
        }
        if !self.prepared.ready || viewport_changed || self.prepared.quads != scene.quads {
            let mut quads = quad_instances(scene, false);
            let fixed_end = quads.len() as u32;
            quads.extend(quad_instances(scene, true));
            let _uploaded_bytes = self.quad_buffer.write(&self.device, &self.queue, &quads);
            #[cfg(feature = "perf")]
            {
                self.perf.uploaded_bytes += _uploaded_bytes as u64;
            }
            self.prepared.quad_fixed = 0..fixed_end;
            self.prepared.quad_scroll = fixed_end..quads.len() as u32;
            self.prepared.quads.clone_from(&scene.quads);
        }

        let text_changed = self.prepared.text_runs.len() != scene.text.len()
            || self
                .prepared
                .text_runs
                .iter()
                .zip(&scene.text)
                .any(|(prepared, run)| prepared.run != *run);
        if !self.prepared.ready
            || viewport_changed
            || text_changed
            || !prepared_masks_match(&self.prepared.masks, &scene.masks)
        {
            let text_runs = self.prepare_text_runs(scene)?;
            let mut text = text_runs
                .iter()
                .filter(|run| !run.run.scrolling)
                .flat_map(|run| run.text.iter().copied())
                .collect::<Vec<_>>();
            text.extend(self.mask_instances(scene, false)?);
            let fixed_end = text.len() as u32;
            text.extend(
                text_runs
                    .iter()
                    .filter(|run| run.run.scrolling)
                    .flat_map(|run| run.text.iter().copied()),
            );
            text.extend(self.mask_instances(scene, true)?);

            let mut groups = HashMap::<(bool, usize), Vec<TextInstance>>::new();
            for run in &text_runs {
                for glyphs in &run.system_glyphs {
                    groups
                        .entry((run.run.scrolling, glyphs.page))
                        .or_default()
                        .extend_from_slice(&glyphs.instances);
                }
            }
            let mut groups = groups.into_iter().collect::<Vec<_>>();
            groups.sort_by_key(|((scrolling, page), _)| (*scrolling, *page));
            let mut system_glyph_instances = Vec::new();
            let mut system_glyph_draws = Vec::with_capacity(groups.len());
            for ((scrolling, page), instances) in groups {
                let start = system_glyph_instances.len() as u32;
                system_glyph_instances.extend(instances);
                system_glyph_draws.push(SystemGlyphDraw {
                    page,
                    instances: start..system_glyph_instances.len() as u32,
                    scrolling,
                });
            }

            let _text_uploaded_bytes = self.text_buffer.write(&self.device, &self.queue, &text);
            let _system_glyph_uploaded_bytes =
                self.system_glyph_buffer
                    .write(&self.device, &self.queue, &system_glyph_instances);
            #[cfg(feature = "perf")]
            {
                self.perf.uploaded_bytes +=
                    (_text_uploaded_bytes + _system_glyph_uploaded_bytes) as u64;
            }
            self.prepared.text_fixed = 0..fixed_end;
            self.prepared.text_scroll = fixed_end..text.len() as u32;
            self.prepared.text_runs = text_runs;
            self.prepared.system_glyph_draws = system_glyph_draws;
            self.prepared.masks.clear();
            self.prepared
                .masks
                .extend(scene.masks.iter().map(PreparedMaskRun::from));
        }

        if !self.prepared.ready
            || viewport_changed
            || self.prepared.image_revision != scene.image_revision
            || !self.prepared.preloaded_images.iter().copied().eq(scene.preloaded_images.iter().map(|image| (image.id(), image.generation())))
            || !prepared_images_match(&self.prepared.images, &scene.images)
        {
            let (images, draws) = self.image_scene_instances(scene)?;
            let _uploaded_bytes = self.image_buffer.write(&self.device, &self.queue, &images);
            #[cfg(feature = "perf")]
            {
                self.perf.uploaded_bytes += _uploaded_bytes as u64;
            }
            self.prepared.image_draws = draws;
            self.prepared.images.clear();
            self.prepared
                .images
                .extend(scene.images.iter().map(PreparedImageRun::from));
        }

        self.prepared.revision = scene.revision;
        self.prepared.image_revision = scene.image_revision;
        self.prepared.width = scene.width;
        self.prepared.height = scene.height;
        self.prepared.ready = true;
        Ok(())
    }

    fn refresh_images(&mut self, scene: &Scene) -> Result<()> {
        let (images, draws) = self.image_scene_instances(scene)?;
        let _uploaded_bytes = self.image_buffer.write(&self.device, &self.queue, &images);
        #[cfg(feature = "perf")]
        {
            self.perf.uploaded_bytes += _uploaded_bytes as u64;
        }
        self.prepared.image_draws = draws;
        self.prepared.images.clear();
        self.prepared
            .images
            .extend(scene.images.iter().map(PreparedImageRun::from));
        self.prepared.image_revision = scene.image_revision;
        Ok(())
    }

    fn image_scene_instances(
        &mut self,
        scene: &Scene,
    ) -> Result<(Vec<TextInstance>, Vec<ImageDraw>)> {
        self.prepared.preloaded_images = scene.preloaded_images.iter().map(|image| (image.id(), image.generation())).collect();
        self.image_cache.begin_frame();
        let (mut images, mut draws) = self.image_instances(scene, false)?;
        let (scroll_images, mut scroll_draws) = self.image_instances(scene, true)?;
        for image in &scene.preloaded_images {
            self.image_cache.prepare(&self.device, &self.queue, image)?;
        }
        let scroll_start = images.len() as u32;
        images.extend(scroll_images);
        for draw in &mut scroll_draws {
            draw.instances.start += scroll_start;
            draw.instances.end += scroll_start;
        }
        draws.extend(scroll_draws);
        self.image_cache
            .trim(&draws.iter().map(|draw| draw.id).chain(scene.preloaded_images.iter().map(ImageData::id)).collect::<HashSet<_>>());
        Ok((images, draws))
    }

    fn prepare_text_runs(&mut self, scene: &Scene) -> Result<Vec<PreparedTextRun>> {
        let mut previous = HashMap::<Arc<str>, Vec<PreparedTextRun>>::new();
        for prepared in std::mem::take(&mut self.prepared.text_runs) {
            previous
                .entry(prepared.run.text.clone())
                .or_default()
                .push(prepared);
        }
        scene
            .text
            .iter()
            .map(|run| {
                if let Some(candidates) = previous.get_mut(&run.text)
                    && let Some(index) = candidates.iter().position(|prepared| {
                        let old = &prepared.run;
                        old == run
                            || (old.font_size == run.font_size
                                && old.colour == run.colour
                                && old.align == run.align
                                && old.tabular_numbers == run.tabular_numbers
                                && old.rect.width == run.rect.width
                                && old.rect.height == run.rect.height
                                && contains(old.clip, old.rect)
                                && contains(run.clip, run.rect))
                    })
                {
                    let mut prepared = candidates.swap_remove(index);
                    let dx = (run.rect.x - prepared.run.rect.x) * 2.0 / scene.width as f32;
                    let dy = (prepared.run.rect.y - run.rect.y) * 2.0 / scene.height as f32;
                    for instance in prepared.text.iter_mut().chain(
                        prepared
                            .system_glyphs
                            .iter_mut()
                            .flat_map(|glyphs| &mut glyphs.instances),
                    ) {
                        instance.rect[0] += dx;
                        instance.rect[2] += dx;
                        instance.rect[1] += dy;
                        instance.rect[3] += dy;
                    }
                    prepared.run.clone_from(run);
                    return Ok(prepared);
                }
                self.prepare_text_run(scene, run)
            })
            .collect()
    }

    fn prepare_text_run(&mut self, scene: &Scene, run: &TextRun) -> Result<PreparedTextRun> {
        let font = self.glyph_atlas.font.clone();
        let mut instances = Vec::new();
        let mut system_glyphs = HashMap::<usize, Vec<TextInstance>>::new();
        let clip = intersect(run.rect, run.clip);
        if clip.width > 0.0 && clip.height > 0.0 {
            let size = run.font_size.round().clamp(1.0, u16::MAX as f32) as u16;
            let scaled = font.as_scaled(PxScale::from(size as f32));
            let justified_space = if run.align == TextAlign::Justify {
                let spaces = ink_core::text_graphemes(&run.text)
                    .filter(|grapheme| grapheme.chars().all(char::is_whitespace))
                    .count();
                (spaces > 0).then(|| {
                    (run.rect.width
                        - text_width_with_numbers(&run.text, size as f32, run.tabular_numbers))
                    .max(0.0)
                        / spaces as f32
                })
            } else {
                None
            };
            let mut pen_x = match run.align {
                TextAlign::Start | TextAlign::Justify => run.rect.x,
                TextAlign::Centre => {
                    let width =
                        text_width_with_numbers(&run.text, size as f32, run.tabular_numbers);
                    run.rect.x + (run.rect.width - width).max(0.0) / 2.0
                }
                TextAlign::End => {
                    let width =
                        text_width_with_numbers(&run.text, size as f32, run.tabular_numbers);
                    run.rect.x + (run.rect.width - width).max(0.0)
                }
            };
            let baseline = run.rect.y + (run.rect.height - scaled.height()) / 2.0 + scaled.ascent();
            let mut previous = None;
            for grapheme in ink_core::text_graphemes(&run.text) {
                if is_emoji_grapheme(grapheme) {
                    if let Some(glyph) = self.system_glyph_atlas.glyph(grapheme, size) {
                        let glyph_size = size as f32 * 0.9;
                        push_system_glyph_quad(
                            system_glyphs.entry(glyph.page).or_default(),
                            scene,
                            Rect {
                                x: pen_x + (size as f32 - glyph_size) / 2.0,
                                y: run.rect.y + (run.rect.height - glyph_size) / 2.0,
                                width: glyph_size,
                                height: glyph_size,
                            },
                            clip,
                            glyph,
                        );
                    }
                    pen_x += size as f32;
                    previous = None;
                    continue;
                }
                for character in grapheme.chars() {
                    let cached = self.glyph_atlas.character(&self.queue, character, size)?;
                    let font_index = cached.font_index;
                    let scaled = cached.font.as_scaled(PxScale::from(size as f32));
                    let id = cached.id;
                    let tabular = run.tabular_numbers && character.is_ascii_digit();
                    let glyph_advance = cached.advance;
                    let advance = if tabular {
                        tabular_digit_width(size as f32)
                    } else {
                        glyph_advance
                    };
                    if tabular {
                        previous = None;
                    }
                    if let Some((previous_index, previous_glyph)) = previous {
                        if previous_index == font_index {
                            pen_x += scaled.kern(previous_glyph, id);
                        }
                    }
                    if let Some(glyph) = cached.bitmap {
                        push_text_quad(
                            &mut instances,
                            scene,
                            clip,
                            Rect {
                                x: pen_x + glyph.offset_x + (advance - glyph_advance) / 2.0,
                                y: baseline + glyph.offset_y,
                                width: glyph.width as f32,
                                height: glyph.height as f32,
                            },
                            glyph,
                            run.colour,
                            self.glyph_atlas.size,
                        );
                    }
                    pen_x += advance;
                    previous = if tabular {
                        None
                    } else {
                        Some((font_index, id))
                    };
                }
                if grapheme.chars().all(char::is_whitespace) {
                    pen_x += justified_space.unwrap_or_default();
                }
            }
        }
        Ok(PreparedTextRun {
            run: run.clone(),
            text: instances,
            system_glyphs: system_glyphs
                .into_iter()
                .map(|(page, instances)| SystemGlyphInstances { page, instances })
                .collect(),
        })
    }

    fn mask_instances(&mut self, scene: &Scene, scrolling: bool) -> Result<Vec<TextInstance>> {
        let mut instances = Vec::with_capacity(scene.masks.len());
        for run in scene.masks.iter().filter(|run| run.scrolling == scrolling) {
            let mask = self.glyph_atlas.mask(&self.queue, &run.mask)?;
            push_mask_quad(
                &mut instances,
                scene,
                run.rect,
                run.clip,
                mask,
                run.colour,
                self.glyph_atlas.size,
            );
        }
        Ok(instances)
    }

    fn image_instances(
        &mut self,
        scene: &Scene,
        scrolling: bool,
    ) -> Result<(Vec<TextInstance>, Vec<ImageDraw>)> {
        let mut instances = Vec::with_capacity(scene.images.len());
        let mut draws: Vec<ImageDraw> = Vec::with_capacity(scene.images.len());
        for run in scene.images.iter().filter(|run| run.scrolling == scrolling) {
            let image = self
                .image_cache
                .prepare(&self.device, &self.queue, &run.image)?;
            let start = instances.len() as u32;
            push_image_quad(&mut instances, scene, run, image.width, image.height);
            let end = instances.len() as u32;
            if start == end {
                continue;
            }
            if let Some(previous) = draws.last_mut()
                && previous.id == run.image.id()
            {
                previous.instances.end = end;
                continue;
            }
            draws.push(ImageDraw {
                id: run.image.id(),
                instances: start..end,
                scrolling,
            });
        }
        Ok((instances, draws))
    }
}

fn prepared_masks_match(prepared: &[PreparedMaskRun], current: &[ink_core::MaskRun]) -> bool {
    prepared.len() == current.len()
        && prepared
            .iter()
            .zip(current)
            .all(|(prepared, current)| *prepared == PreparedMaskRun::from(current))
}

fn prepared_images_match(prepared: &[PreparedImageRun], current: &[ImageRun]) -> bool {
    prepared.len() == current.len()
        && prepared
            .iter()
            .zip(current)
            .all(|(prepared, current)| *prepared == PreparedImageRun::from(current))
}

fn raw_instance_buffer<T>(
    device: &gpu::Device,
    _label: &'static str,
    instances: usize,
) -> gpu::Buffer {
    device.buffer(instances.max(1) * size_of::<T>(), false)
}
fn transform_buffer(device: &gpu::Device, _label: &'static str) -> gpu::Buffer {
    device.buffer(size_of::<TransformUniform>(), true)
}
fn quad_instances(scene: &Scene, scrolling: bool) -> Vec<QuadInstance> {
    let mut instances = Vec::with_capacity(scene.quads.len());
    for quad in scene
        .quads
        .iter()
        .filter(|quad| quad.scrolling == scrolling)
    {
        push_quad_instance(&mut instances, scene, quad);
    }
    instances
}

fn push_quad_instance(instances: &mut Vec<QuadInstance>, scene: &Scene, quad: &ink_core::Quad) {
    let rect = intersect(quad.rect, quad.clip);
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let [left, top] = position(scene, rect.x, rect.y);
    let [right, bottom] = position(scene, rect.x + rect.width, rect.y + rect.height);
    let colour = colour(quad.colour);
    instances.push(QuadInstance {
        rect: [left, top, right, bottom],
        colour,
    });
}

fn push_scrollbar_instances(scene: &Scene, instances: &mut Vec<QuadInstance>) {
    let Some(scrollbar) = scene.scroll_bar else {
        return;
    };
    let thumb = scrollbar.thumb_rect(scene.scroll_offset, scene.scroll_max);
    let clip = scene.scroll_clip.unwrap_or(scrollbar.track);
    for quad in [
        ink_core::Quad {
            rect: scrollbar.track,
            clip,
            colour: scene.colour(Colour::WHITE),
            scrolling: false,
        },
        ink_core::Quad {
            rect: thumb,
            clip,
            colour: scene.colour(Colour::WHITE),
            scrolling: false,
        },
    ] {
        push_quad_instance(instances, scene, &quad);
    }
}

fn set_scroll_scissor(pass: &mut gpu::RenderPass<'_>, scene: &Scene) {
    let Some(clip) = scene.scroll_clip else {
        return;
    };
    let x = clip.x.max(0.0).floor() as u32;
    let y = clip.y.max(0.0).floor() as u32;
    let right = (clip.x + clip.width)
        .min(scene.width as f32)
        .ceil()
        .max(x as f32) as u32;
    let bottom = (clip.y + clip.height)
        .min(scene.height as f32)
        .ceil()
        .max(y as f32) as u32;
    pass.set_scissor_rect(x, y, right - x, bottom - y);
}

fn reset_scissor(pass: &mut gpu::RenderPass<'_>, scene: &Scene) {
    pass.set_scissor_rect(0, 0, scene.width, scene.height);
}

fn push_text_quad(
    instances: &mut Vec<TextInstance>,
    scene: &Scene,
    clip: Rect,
    rect: Rect,
    glyph: CachedGlyph,
    glyph_colour: Colour,
    atlas_size: u32,
) {
    let left = rect.x.max(clip.x);
    let top = rect.y.max(clip.y);
    let right = (rect.x + rect.width).min(clip.x + clip.width);
    let bottom = (rect.y + rect.height).min(clip.y + clip.height);
    if left >= right || top >= bottom {
        return;
    }

    let atlas_left = glyph.atlas_x as f32 / atlas_size as f32;
    let atlas_top = glyph.atlas_y as f32 / atlas_size as f32;
    let atlas_width = glyph.width as f32 / atlas_size as f32;
    let atlas_height = glyph.height as f32 / atlas_size as f32;
    let u0 = atlas_left + (left - rect.x) / rect.width * atlas_width;
    let v0 = atlas_top + (top - rect.y) / rect.height * atlas_height;
    let u1 = atlas_left + (right - rect.x) / rect.width * atlas_width;
    let v1 = atlas_top + (bottom - rect.y) / rect.height * atlas_height;
    let top_left = position(scene, left, top);
    let bottom_right = position(scene, right, bottom);
    let [ndc_left, ndc_top] = top_left;
    let [ndc_right, ndc_bottom] = bottom_right;
    let colour = colour(glyph_colour);
    instances.push(TextInstance {
        rect: [ndc_left, ndc_top, ndc_right, ndc_bottom],
        uv: [u0, v0, u1, v1],
        colour,
    });
}

fn push_mask_quad(
    instances: &mut Vec<TextInstance>,
    scene: &Scene,
    rect: Rect,
    clip: Rect,
    mask: CachedMask,
    mask_colour: Colour,
    atlas_size: u32,
) {
    let visible = intersect(rect, clip);
    if visible.width <= 0.0 || visible.height <= 0.0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let atlas_left = mask.atlas_x as f32 / atlas_size as f32;
    let atlas_top = mask.atlas_y as f32 / atlas_size as f32;
    let atlas_width = mask.width as f32 / atlas_size as f32;
    let atlas_height = mask.height as f32 / atlas_size as f32;
    let u0 = atlas_left + (visible.x - rect.x) / rect.width * atlas_width;
    let v0 = atlas_top + (visible.y - rect.y) / rect.height * atlas_height;
    let u1 = atlas_left + (visible.x + visible.width - rect.x) / rect.width * atlas_width;
    let v1 = atlas_top + (visible.y + visible.height - rect.y) / rect.height * atlas_height;
    let [ndc_left, ndc_top] = position(scene, visible.x, visible.y);
    let [ndc_right, ndc_bottom] =
        position(scene, visible.x + visible.width, visible.y + visible.height);
    let colour = colour(mask_colour);
    instances.push(TextInstance {
        rect: [ndc_left, ndc_top, ndc_right, ndc_bottom],
        uv: [u0, v0, u1, v1],
        colour,
    });
}

fn push_image_quad(
    instances: &mut Vec<TextInstance>,
    scene: &Scene,
    run: &ImageRun,
    image_width: u32,
    image_height: u32,
) {
    let mut rect = run.rect;
    let clip = if run.zoom_id.is_some() {
        intersect(run.clip, run.rect)
    } else {
        run.clip
    };
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let image_aspect = image_width as f32 / image_height as f32;
    let rect_aspect = rect.width / rect.height;
    let (mut u0, mut v0, mut u1, mut v1) = (0.0, 0.0, 1.0, 1.0);
    match run.fit {
        ImageFit::Cover if image_aspect > rect_aspect => {
            let visible = rect_aspect / image_aspect;
            u0 = (1.0 - visible) / 2.0;
            u1 = 1.0 - u0;
        }
        ImageFit::Cover => {
            let visible = image_aspect / rect_aspect;
            v0 = (1.0 - visible) / 2.0;
            v1 = 1.0 - v0;
        }
        ImageFit::Contain if image_aspect > rect_aspect => {
            let height = rect.width / image_aspect;
            rect.y += (rect.height - height) / 2.0;
            rect.height = height;
        }
        ImageFit::Contain => {
            let width = rect.height * image_aspect;
            rect.x += (rect.width - width) / 2.0;
            rect.width = width;
        }
    }

    rect = Rect {
        x: rect.x * run.transform.scale + run.transform.translation_x,
        y: rect.y * run.transform.scale + run.transform.translation_y,
        width: rect.width * run.transform.scale,
        height: rect.height * run.transform.scale,
    };

    let visible = intersect(rect, clip);
    if visible.width <= 0.0 || visible.height <= 0.0 {
        return;
    }
    let source_width = u1 - u0;
    let source_height = v1 - v0;
    let clipped_u0 = u0 + (visible.x - rect.x) / rect.width * source_width;
    let clipped_v0 = v0 + (visible.y - rect.y) / rect.height * source_height;
    let clipped_u1 = u0 + (visible.x + visible.width - rect.x) / rect.width * source_width;
    let clipped_v1 = v0 + (visible.y + visible.height - rect.y) / rect.height * source_height;
    let [left, top] = position(scene, visible.x, visible.y);
    let [right, bottom] = position(scene, visible.x + visible.width, visible.y + visible.height);
    let colour = [1.0; 4];
    instances.push(TextInstance {
        rect: [left, top, right, bottom],
        uv: [clipped_u0, clipped_v0, clipped_u1, clipped_v1],
        colour,
    });
}

fn push_system_glyph_quad(
    instances: &mut Vec<TextInstance>,
    scene: &Scene,
    rect: Rect,
    clip: Rect,
    glyph: CachedSystemGlyph,
) {
    let visible = intersect(rect, clip);
    if visible.width <= 0.0 || visible.height <= 0.0 {
        return;
    }
    let atlas_size = SYSTEM_GLYPH_ATLAS_SIZE as f32;
    let u0 = glyph.atlas_x as f32 / atlas_size;
    let v0 = glyph.atlas_y as f32 / atlas_size;
    let glyph_uv_size = glyph.size as f32 / atlas_size;
    let clipped_u0 = u0 + (visible.x - rect.x) / rect.width * glyph_uv_size;
    let clipped_v0 = v0 + (visible.y - rect.y) / rect.height * glyph_uv_size;
    let clipped_u1 = u0 + (visible.x + visible.width - rect.x) / rect.width * glyph_uv_size;
    let clipped_v1 = v0 + (visible.y + visible.height - rect.y) / rect.height * glyph_uv_size;
    let [left, top] = position(scene, visible.x, visible.y);
    let [right, bottom] = position(scene, visible.x + visible.width, visible.y + visible.height);
    let colour = [1.0; 4];
    instances.push(TextInstance {
        rect: [left, top, right, bottom],
        uv: [clipped_u0, clipped_v0, clipped_u1, clipped_v1],
        colour,
    });
}

fn position(scene: &Scene, x: f32, y: f32) -> [f32; 2] {
    [
        x / scene.width as f32 * 2.0 - 1.0,
        1.0 - y / scene.height as f32 * 2.0,
    ]
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
}

fn intersect(first: Rect, second: Rect) -> Rect {
    let left = first.x.max(second.x);
    let top = first.y.max(second.y);
    let right = (first.x + first.width).min(second.x + second.width);
    let bottom = (first.y + first.height).min(second.y + second.height);
    Rect {
        x: left,
        y: top,
        width: (right - left).max(0.0),
        height: (bottom - top).max(0.0),
    }
}

fn colour(colour: Colour) -> [f32; 4] {
    [colour.red, colour.green, colour.blue, colour.alpha]
}

#[cfg(feature = "perf")]
fn elapsed_ns(started: Instant) -> u64 {
    started.elapsed().as_nanos() as u64
}
