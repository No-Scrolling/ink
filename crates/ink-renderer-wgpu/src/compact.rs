#[cfg(feature = "perf")]
use std::time::Instant;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Range,
};

use ab_glyph::{Font, FontRef, GlyphId, PxScale, ScaleFont};
use anyhow::{Context, Result, anyhow};
use bytemuck::{Pod, Zeroable};
use ink_core::{
    Colour, ImageAssetEncoding, ImageData, ImageFit, ImageRun, Mask, PUBLIC_SANS, Rect, Scene,
    TextAlign, TextRun, is_emoji_grapheme,
};
#[cfg(feature = "perf")]
use ink_core::{PerfTraceSection, perf_trace_counter};
use unicode_segmentation::UnicodeSegmentation;

use crate::system_glyph::{
    ATLAS_SIZE as SYSTEM_GLYPH_ATLAS_SIZE, CachedSystemGlyph, SystemGlyphAtlas, SystemGlyphRequest,
};

const MAX_QUADS: usize = 64;
const MAX_GLYPHS: usize = 512;
const ATLAS_SIZE: u32 = 1024;
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
    buffer: Option<wgpu::Buffer>,
    capacity: usize,
    instances: Vec<T>,
    label: &'static str,
}

impl<T: Pod + PartialEq> InstanceBuffer<T> {
    fn new(device: &wgpu::Device, label: &'static str, capacity: usize) -> Self {
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

    fn buffer(&self) -> &wgpu::Buffer {
        self.buffer
            .as_ref()
            .expect("instances uploaded before drawing")
    }

    fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, instances: &[T]) -> usize {
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
struct CachedMask {
    atlas_x: u32,
    atlas_y: u32,
    width: u32,
    height: u32,
}

struct CachedImage {
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
    generation: u64,
    bytes: usize,
    last_used: u64,
}

struct ImageCache {
    bind_group_layout: wgpu::BindGroupLayout,
    images: HashMap<u64, CachedImage>,
    frame: u64,
    #[cfg(feature = "perf")]
    cache_misses: u64,
    #[cfg(feature = "perf")]
    uploaded_bytes: u64,
}

impl ImageCache {
    fn new(bind_group_layout: wgpu::BindGroupLayout) -> Self {
        Self {
            bind_group_layout,
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
        device: &wgpu::Device,
        queue: &wgpu::Queue,
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
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Ink image"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Ink image sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Ink image bind group"),
                layout: &self.bind_group_layout,
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

struct GlyphAtlas {
    font: FontRef<'static>,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    glyphs: HashMap<(GlyphId, u16), CachedGlyph>,
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
    fn new(device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout) -> Result<Self> {
        let font = FontRef::try_from_slice(PUBLIC_SANS).context("Public Sans is invalid")?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ink glyph atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Ink glyph sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ink glyph atlas bind group"),
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

        Ok(Self {
            font,
            texture,
            bind_group,
            glyphs: HashMap::new(),
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

    fn glyph(
        &mut self,
        queue: &wgpu::Queue,
        id: GlyphId,
        size: u16,
    ) -> Result<Option<CachedGlyph>> {
        if let Some(glyph) = self.glyphs.get(&(id, size)) {
            return Ok(Some(*glyph));
        }

        let Some(outlined) = self.font.outline_glyph(id.with_scale(size as f32)) else {
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
        if width > ATLAS_SIZE || height > ATLAS_SIZE {
            return Err(anyhow!(
                "a Public Sans glyph exceeds the Ink atlas dimensions"
            ));
        }
        if self.cursor_x + width > ATLAS_SIZE {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }
        if self.cursor_y + height > ATLAS_SIZE {
            return Err(anyhow!("the Ink glyph atlas is full"));
        }

        let mut pixels = vec![0; (glyph_width * glyph_height) as usize];
        outlined.draw(|x, y, coverage| {
            pixels[(y * glyph_width + x) as usize] = (coverage * 255.0).round() as u8;
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: self.cursor_x + ATLAS_PADDING,
                    y: self.cursor_y + ATLAS_PADDING,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(glyph_width),
                rows_per_image: Some(glyph_height),
            },
            wgpu::Extent3d {
                width: glyph_width,
                height: glyph_height,
                depth_or_array_layers: 1,
            },
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
        self.glyphs.insert((id, size), glyph);
        #[cfg(feature = "perf")]
        {
            self.cache_misses += 1;
            self.uploaded_bytes += pixels.len() as u64;
        }
        Ok(Some(glyph))
    }

    fn mask(&mut self, queue: &wgpu::Queue, mask: &Mask) -> Result<CachedMask> {
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
        if padded_width > ATLAS_SIZE || padded_height > ATLAS_SIZE {
            return Err(anyhow!("an Ink mask exceeds the atlas dimensions"));
        }
        if self.cursor_x + padded_width > ATLAS_SIZE {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }
        if self.cursor_y + padded_height > ATLAS_SIZE {
            return Err(anyhow!("the Ink glyph atlas is full"));
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: self.cursor_x + ATLAS_PADDING,
                    y: self.cursor_y + ATLAS_PADDING,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            mask.pixels.as_ref(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
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
    pub prepare_ns: u64,
    pub upload_ns: u64,
    pub acquire_ns: u64,
    pub encode_ns: u64,
    pub queue_submit_cpu_ns: u64,
    pub queue_present_cpu_ns: u64,
    pub frame_ns: u64,
    pub instances: u64,
    pub uploaded_bytes: u64,
    pub draw_calls: u64,
    pub cache_misses: u64,
}

pub struct Renderer {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    fixed_transform: wgpu::BindGroup,
    scroll_transform: wgpu::BindGroup,
    scroll_transform_buffer: wgpu::Buffer,
    quad_pipeline: wgpu::RenderPipeline,
    quad_buffer: InstanceBuffer<QuadInstance>,
    overlay_buffer: InstanceBuffer<QuadInstance>,
    overlay_instances: Vec<QuadInstance>,
    text_pipeline: wgpu::RenderPipeline,
    text_buffer: InstanceBuffer<TextInstance>,
    image_pipeline: Option<wgpu::RenderPipeline>,
    transform_layout: wgpu::BindGroupLayout,
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
    pub async fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .context("no Vulkan adapter supports the Android surface")?;
        if !adapter
            .features()
            .contains(wgpu::Features::PASSTHROUGH_SHADERS)
        {
            return Err(anyhow!(
                "the Vulkan adapter does not support SPIR-V passthrough"
            ));
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::PASSTHROUGH_SHADERS,
                ..Default::default()
            })
            .await
            .context("failed to create the Vulkan device")?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| anyhow!("the Android surface exposes no texture formats"))?;
        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .or_else(|| capabilities.present_modes.first().copied())
            .ok_or_else(|| anyhow!("the Android surface exposes no present modes"))?;
        let alpha_mode = capabilities
            .alpha_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::CompositeAlphaMode::Opaque)
            .or_else(|| capabilities.alpha_modes.first().copied())
            .ok_or_else(|| anyhow!("the Android surface exposes no alpha modes"))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            alpha_mode,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 1,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        let quad_vertex_shader = spirv_shader(
            &device,
            "quad_vertex",
            include_bytes!(concat!(env!("OUT_DIR"), "/quad_vertex.spv")),
        );
        let quad_fragment_shader = spirv_shader(
            &device,
            "quad_fragment",
            include_bytes!(concat!(env!("OUT_DIR"), "/quad_fragment.spv")),
        );
        let text_vertex_shader = spirv_shader(
            &device,
            "text_vertex",
            include_bytes!(concat!(env!("OUT_DIR"), "/text_vertex.spv")),
        );
        let text_fragment_shader = spirv_shader(
            &device,
            "text_fragment",
            include_bytes!(concat!(env!("OUT_DIR"), "/text_fragment.spv")),
        );
        let transform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Ink transform bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let glyph_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Ink glyph bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let quad_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ink quad pipeline layout"),
            bind_group_layouts: &[Some(&transform_layout)],
            immediate_size: 0,
        });
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ink quad pipeline"),
            layout: Some(&quad_layout),
            vertex: wgpu::VertexState {
                module: &quad_vertex_shader,
                entry_point: Some("quad_vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<QuadInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: size_of::<[f32; 4]>() as u64,
                            shader_location: 1,
                        },
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &quad_fragment_shader,
                entry_point: Some("quad_fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(colour_target(format))],
            }),
            multiview_mask: None,
            cache: None,
        });
        let text_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ink text pipeline layout"),
            bind_group_layouts: &[Some(&transform_layout), Some(&glyph_bind_group_layout)],
            immediate_size: 0,
        });
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ink text pipeline"),
            layout: Some(&text_layout),
            vertex: wgpu::VertexState {
                module: &text_vertex_shader,
                entry_point: Some("text_vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<TextInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: size_of::<[f32; 4]>() as u64,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: size_of::<[f32; 8]>() as u64,
                            shader_location: 2,
                        },
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &text_fragment_shader,
                entry_point: Some("text_fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(colour_target(format))],
            }),
            multiview_mask: None,
            cache: None,
        });
        let image_bind_group_layout = glyph_bind_group_layout.clone();
        let fixed_transform_buffer = transform_buffer(&device, "Ink fixed transform");
        let scroll_transform_buffer = transform_buffer(&device, "Ink scroll transform");
        let fixed_transform = transform_bind_group(
            &device,
            &transform_layout,
            &fixed_transform_buffer,
            "Ink fixed transform",
        );
        let scroll_transform = transform_bind_group(
            &device,
            &transform_layout,
            &scroll_transform_buffer,
            "Ink scroll transform",
        );
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
        let system_glyph_atlas = SystemGlyphAtlas::new(image_bind_group_layout.clone());
        let image_cache = ImageCache::new(image_bind_group_layout);
        let system_glyph_buffer = InstanceBuffer::lazy("Ink system glyph instances");
        let glyph_atlas = GlyphAtlas::new(&device, &glyph_bind_group_layout)?;

        Ok(Self {
            _instance: instance,
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
            image_pipeline: None,
            transform_layout,
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
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self, scene: &Scene, text_cursor_visible: bool) -> Result<RenderOutcome> {
        #[cfg(feature = "perf")]
        let frame_started = Instant::now();
        #[cfg(feature = "perf")]
        let frame_trace = PerfTraceSection::new(b"Ink frame\0");
        if scene.width == 0 || scene.height == 0 {
            return Ok(RenderOutcome::Skipped);
        }
        if self.config.width != scene.width || self.config.height != scene.height {
            self.resize(scene.width, scene.height);
        }
        if !scene.images.is_empty() {
            self.prepare_image_pipeline();
        }
        if !self.prepared.ready || self.prepared.revision != scene.revision {
            if let Some(request) = self.system_glyph_atlas.request(scene) {
                self.prepare_image_pipeline();
                return Ok(RenderOutcome::NeedsSystemGlyph(request));
            }
            #[cfg(feature = "perf")]
            let prepare_started = Instant::now();
            #[cfg(feature = "perf")]
            let prepare_trace = PerfTraceSection::new(b"Ink prepare\0");
            self.prepare(scene)?;
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
        if text_cursor_visible && let Some(cursor) = &scene.text_cursor {
            push_quad_instance(&mut self.overlay_instances, scene, cursor);
        }
        let cursor_end = self.overlay_instances.len() as u32;
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
        let mut retried_outdated_surface = false;
        let frame = loop {
            match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame)
                | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => break frame,
                wgpu::CurrentSurfaceTexture::Outdated if !retried_outdated_surface => {
                    retried_outdated_surface = true;
                    self.surface.configure(&self.device, &self.config);
                }
                wgpu::CurrentSurfaceTexture::Outdated
                | wgpu::CurrentSurfaceTexture::Timeout
                | wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::Skipped),
                wgpu::CurrentSurfaceTexture::Lost => return Ok(RenderOutcome::SurfaceLost),
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err(anyhow!("surface validation failed"));
                }
            }
        };
        #[cfg(feature = "perf")]
        {
            drop(acquire_trace);
            self.perf.acquire_ns += elapsed_ns(acquire_started);
        }
        #[cfg(feature = "perf")]
        let encode_started = Instant::now();
        #[cfg(feature = "perf")]
        let encode_trace = PerfTraceSection::new(b"Ink encode\0");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Ink frame encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ink frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(if scene.light {
                            wgpu::Color::WHITE
                        } else {
                            wgpu::Color::BLACK
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if !self.prepared.quad_fixed.is_empty() {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_bind_group(0, &self.fixed_transform, &[]);
                pass.set_vertex_buffer(0, self.quad_buffer.buffer().slice(..));
                pass.draw(0..6, self.prepared.quad_fixed.clone());
            }
            if !self.prepared.quad_scroll.is_empty() {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_bind_group(0, &self.scroll_transform, &[]);
                pass.set_vertex_buffer(0, self.quad_buffer.buffer().slice(..));
                set_scroll_scissor(&mut pass, scene);
                pass.draw(0..6, self.prepared.quad_scroll.clone());
                reset_scissor(&mut pass, scene);
            }
            if !self.prepared.image_draws.is_empty() {
                pass.set_pipeline(
                    self.image_pipeline
                        .as_ref()
                        .expect("image pipeline prepared before drawing"),
                );
                pass.set_vertex_buffer(0, self.image_buffer.buffer().slice(..));
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
                        &[],
                    );
                    pass.set_bind_group(1, &image.bind_group, &[]);
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
                pass.set_pipeline(
                    self.image_pipeline
                        .as_ref()
                        .expect("image pipeline prepared before drawing"),
                );
                pass.set_vertex_buffer(0, self.system_glyph_buffer.buffer().slice(..));
                for draw in &self.prepared.system_glyph_draws {
                    pass.set_bind_group(
                        0,
                        if draw.scrolling {
                            &self.scroll_transform
                        } else {
                            &self.fixed_transform
                        },
                        &[],
                    );
                    pass.set_bind_group(1, self.system_glyph_atlas.bind_group(draw.page), &[]);
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
                pass.set_bind_group(0, &self.fixed_transform, &[]);
                pass.set_bind_group(1, &self.glyph_atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, self.text_buffer.buffer().slice(..));
                pass.draw(0..6, self.prepared.text_fixed.clone());
            }
            if !self.prepared.text_scroll.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.scroll_transform, &[]);
                pass.set_bind_group(1, &self.glyph_atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, self.text_buffer.buffer().slice(..));
                set_scroll_scissor(&mut pass, scene);
                pass.draw(0..6, self.prepared.text_scroll.clone());
                reset_scissor(&mut pass, scene);
            }
            if cursor_end > 0 {
                pass.set_pipeline(&self.quad_pipeline);
                let scrolling = scene
                    .text_cursor
                    .as_ref()
                    .is_some_and(|cursor| cursor.scrolling);
                pass.set_bind_group(
                    0,
                    if scrolling {
                        &self.scroll_transform
                    } else {
                        &self.fixed_transform
                    },
                    &[],
                );
                pass.set_vertex_buffer(0, self.overlay_buffer.buffer().slice(..));
                if scrolling {
                    set_scroll_scissor(&mut pass, scene);
                }
                pass.draw(0..6, 0..cursor_end);
                if scrolling {
                    reset_scissor(&mut pass, scene);
                }
            }
            if cursor_end < overlay_end {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_bind_group(0, &self.fixed_transform, &[]);
                pass.set_vertex_buffer(0, self.overlay_buffer.buffer().slice(..));
                pass.draw(0..6, cursor_end..overlay_end);
            }
        }
        let command_buffer = encoder.finish();
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
                + u64::from(cursor_end > 0)
                + u64::from(cursor_end < overlay_end);
            self.perf.instances += instances;
            self.perf.draw_calls += draw_calls;
            perf_trace_counter(b"Ink instances\0", instances);
            perf_trace_counter(b"Ink uploaded bytes\0", self.perf.uploaded_bytes);
            perf_trace_counter(b"Ink draw calls\0", draw_calls);
            perf_trace_counter(b"Ink cache misses\0", self.perf.cache_misses);
        }
        #[cfg(feature = "perf")]
        let queue_submit_started = Instant::now();
        #[cfg(feature = "perf")]
        let queue_submit_trace = PerfTraceSection::new(b"Ink queue submit\0");
        self.queue.submit(Some(command_buffer));
        #[cfg(feature = "perf")]
        {
            drop(queue_submit_trace);
            self.perf.queue_submit_cpu_ns += elapsed_ns(queue_submit_started);
        }
        #[cfg(feature = "perf")]
        let queue_present_started = Instant::now();
        #[cfg(feature = "perf")]
        let queue_present_trace = PerfTraceSection::new(b"Ink queue present\0");
        self.queue.present(frame);
        #[cfg(feature = "perf")]
        {
            drop(queue_present_trace);
            self.perf.queue_present_cpu_ns += elapsed_ns(queue_present_started);
            self.perf.frame_ns += elapsed_ns(frame_started);
            drop(frame_trace);
        }
        Ok(RenderOutcome::Presented)
    }

    #[cfg(feature = "perf")]
    pub fn take_perf_metrics(&mut self) -> RenderPerfMetrics {
        std::mem::take(&mut self.perf)
    }

    pub fn install_system_glyph(&mut self, request_id: u64, pixels: Option<&[u8]>) -> Result<()> {
        self.system_glyph_atlas
            .install(&self.device, &self.queue, request_id, pixels)
    }

    fn prepare_image_pipeline(&mut self) {
        if self.image_pipeline.is_none() {
            self.image_pipeline = Some(image_pipeline(
                &self.device,
                self.config.format,
                &self.transform_layout,
                &self.image_cache.bind_group_layout,
            ));
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
        self.image_cache.begin_frame();
        let (mut images, mut draws) = self.image_instances(scene, false)?;
        let (scroll_images, mut scroll_draws) = self.image_instances(scene, true)?;
        let scroll_start = images.len() as u32;
        images.extend(scroll_images);
        for draw in &mut scroll_draws {
            draw.instances.start += scroll_start;
            draw.instances.end += scroll_start;
        }
        draws.extend(scroll_draws);
        self.image_cache
            .trim(&draws.iter().map(|draw| draw.id).collect::<HashSet<_>>());
        Ok((images, draws))
    }

    fn prepare_text_runs(&mut self, scene: &Scene) -> Result<Vec<PreparedTextRun>> {
        let mut previous = std::mem::take(&mut self.prepared.text_runs).into_iter();
        scene
            .text
            .iter()
            .map(|run| match previous.next() {
                Some(prepared) if prepared.run == *run => Ok(prepared),
                _ => self.prepare_text_run(scene, run),
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
                let spaces = run
                    .text
                    .graphemes(true)
                    .filter(|grapheme| grapheme.chars().all(char::is_whitespace))
                    .count();
                (spaces > 0).then(|| {
                    (run.rect.width - text_run_width(&scaled, &run.text, size as f32)).max(0.0)
                        / spaces as f32
                })
            } else {
                None
            };
            let mut pen_x = match run.align {
                TextAlign::Start | TextAlign::Justify => run.rect.x,
                TextAlign::Centre => {
                    let width = text_run_width(&scaled, &run.text, size as f32);
                    run.rect.x + (run.rect.width - width).max(0.0) / 2.0
                }
                TextAlign::End => {
                    let width = text_run_width(&scaled, &run.text, size as f32);
                    run.rect.x + (run.rect.width - width).max(0.0)
                }
            };
            let baseline = run.rect.y + (run.rect.height - scaled.height()) / 2.0 + scaled.ascent();
            let mut previous = None;
            for grapheme in run.text.graphemes(true) {
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
                    let id = scaled.glyph_id(character);
                    if let Some(previous) = previous {
                        pen_x += scaled.kern(previous, id);
                    }
                    if let Some(glyph) = self.glyph_atlas.glyph(&self.queue, id, size)? {
                        push_text_quad(
                            &mut instances,
                            scene,
                            clip,
                            Rect {
                                x: pen_x + glyph.offset_x,
                                y: baseline + glyph.offset_y,
                                width: glyph.width as f32,
                                height: glyph.height as f32,
                            },
                            glyph,
                            run.colour,
                        );
                    }
                    pen_x += scaled.h_advance(id);
                    previous = Some(id);
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
            push_mask_quad(&mut instances, scene, run.rect, run.clip, mask, run.colour);
        }
        Ok(instances)
    }

    fn image_instances(
        &mut self,
        scene: &Scene,
        scrolling: bool,
    ) -> Result<(Vec<TextInstance>, Vec<ImageDraw>)> {
        let mut instances = Vec::with_capacity(scene.images.len());
        let mut draws = Vec::with_capacity(scene.images.len());
        for run in scene.images.iter().filter(|run| run.scrolling == scrolling) {
            let image = self
                .image_cache
                .prepare(&self.device, &self.queue, &run.image)?;
            let start = instances.len() as u32;
            push_image_quad(&mut instances, scene, run, image.width, image.height);
            draws.push(ImageDraw {
                id: run.image.id(),
                instances: start..instances.len() as u32,
                scrolling,
            });
        }
        Ok((instances, draws))
    }
}

fn text_run_width<F: Font>(font: &impl ScaleFont<F>, text: &str, emoji_size: f32) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for grapheme in text.graphemes(true) {
        if is_emoji_grapheme(grapheme) {
            width += emoji_size;
            previous = None;
            continue;
        }
        for character in grapheme.chars() {
            let glyph = font.glyph_id(character);
            width += previous
                .map(|previous| font.kern(previous, glyph))
                .unwrap_or_default();
            width += font.h_advance(glyph);
            previous = Some(glyph);
        }
    }
    width
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

fn image_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    transform_layout: &wgpu::BindGroupLayout,
    image_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let text_vertex_shader = spirv_shader(
        device,
        "text_vertex",
        include_bytes!(concat!(env!("OUT_DIR"), "/text_vertex.spv")),
    );
    let image_fragment_shader = spirv_shader(
        device,
        "image_fragment",
        include_bytes!(concat!(env!("OUT_DIR"), "/image_fragment.spv")),
    );
    let image_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Ink image pipeline layout"),
        bind_group_layouts: &[Some(transform_layout), Some(image_bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Ink image pipeline"),
        layout: Some(&image_layout),
        vertex: wgpu::VertexState {
            module: &text_vertex_shader,
            entry_point: Some("text_vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<TextInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: size_of::<[f32; 4]>() as u64,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: size_of::<[f32; 8]>() as u64,
                        shader_location: 2,
                    },
                ],
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &image_fragment_shader,
            entry_point: Some("image_fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(colour_target(format))],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn colour_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

const fn shader_entry_point(name: &'static str) -> wgpu::PassthroughShaderEntryPoint<'static> {
    wgpu::PassthroughShaderEntryPoint {
        name: Cow::Borrowed(name),
        workgroup_size: (0, 0, 0),
    }
}

fn spirv_shader(
    device: &wgpu::Device,
    entry_point: &'static str,
    spirv: &'static [u8],
) -> wgpu::ShaderModule {
    unsafe {
        device.create_shader_module_passthrough(wgpu::ShaderModuleDescriptorPassthrough {
            label: Some(entry_point),
            entry_points: Cow::Owned(vec![shader_entry_point(entry_point)]),
            spirv: Some(wgpu::util::make_spirv_raw(spirv)),
            ..Default::default()
        })
    }
}

fn raw_instance_buffer<T>(
    device: &wgpu::Device,
    label: &'static str,
    instances: usize,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (instances.max(1) * size_of::<T>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn transform_buffer(device: &wgpu::Device, label: &'static str) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size_of::<TransformUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn transform_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
    label: &'static str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
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

fn set_scroll_scissor(pass: &mut wgpu::RenderPass<'_>, scene: &Scene) {
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

fn reset_scissor(pass: &mut wgpu::RenderPass<'_>, scene: &Scene) {
    pass.set_scissor_rect(0, 0, scene.width, scene.height);
}

fn push_text_quad(
    instances: &mut Vec<TextInstance>,
    scene: &Scene,
    clip: Rect,
    rect: Rect,
    glyph: CachedGlyph,
    glyph_colour: Colour,
) {
    let left = rect.x.max(clip.x);
    let top = rect.y.max(clip.y);
    let right = (rect.x + rect.width).min(clip.x + clip.width);
    let bottom = (rect.y + rect.height).min(clip.y + clip.height);
    if left >= right || top >= bottom {
        return;
    }

    let atlas_left = glyph.atlas_x as f32 / ATLAS_SIZE as f32;
    let atlas_top = glyph.atlas_y as f32 / ATLAS_SIZE as f32;
    let atlas_width = glyph.width as f32 / ATLAS_SIZE as f32;
    let atlas_height = glyph.height as f32 / ATLAS_SIZE as f32;
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
) {
    let visible = intersect(rect, clip);
    if visible.width <= 0.0 || visible.height <= 0.0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let atlas_left = mask.atlas_x as f32 / ATLAS_SIZE as f32;
    let atlas_top = mask.atlas_y as f32 / ATLAS_SIZE as f32;
    let atlas_width = mask.width as f32 / ATLAS_SIZE as f32;
    let atlas_height = mask.height as f32 / ATLAS_SIZE as f32;
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
