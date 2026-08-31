use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Range,
};

use ab_glyph::{Font, FontRef, GlyphId, PxScale, ScaleFont};
use anyhow::{Context, Result, anyhow};
use bytemuck::{Pod, Zeroable};
use ink_core::{
    Colour, ImageData, ImageFit, Mask, PUBLIC_SANS, Rect, Scene, TextAlign, TextRun,
    is_emoji_grapheme,
};
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
struct QuadVertex {
    position: [f32; 2],
    colour: [f32; 4],
}

#[derive(Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C)]
struct TextVertex {
    position: [f32; 2],
    uv: [f32; 2],
    colour: [f32; 4],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct TransformUniform {
    translation: [f32; 4],
}

struct VertexBuffer<T> {
    buffer: wgpu::Buffer,
    capacity: usize,
    vertices: Vec<T>,
    label: &'static str,
}

impl<T: Pod + PartialEq> VertexBuffer<T> {
    fn new(device: &wgpu::Device, label: &'static str, capacity: usize) -> Self {
        Self {
            buffer: raw_vertex_buffer::<T>(device, label, capacity),
            capacity,
            vertices: Vec::new(),
            label,
        }
    }

    fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[T]) {
        let resized = vertices.len() > self.capacity;
        if resized {
            self.capacity = vertices.len().next_power_of_two();
            self.buffer = raw_vertex_buffer::<T>(device, self.label, self.capacity);
        }
        let start = if resized {
            0
        } else {
            self.vertices
                .iter()
                .zip(vertices)
                .position(|(current, next)| current != next)
                .unwrap_or(self.vertices.len().min(vertices.len()))
        };
        let end = if resized || self.vertices.len() != vertices.len() {
            vertices.len()
        } else {
            self.vertices
                .iter()
                .zip(vertices)
                .rposition(|(current, next)| current != next)
                .map_or(start, |index| index + 1)
        };
        if start < end {
            queue.write_buffer(
                &self.buffer,
                (start * size_of::<T>()) as u64,
                bytemuck::cast_slice(&vertices[start..end]),
            );
        }
        self.vertices.clear();
        self.vertices.extend_from_slice(vertices);
    }
}

#[derive(Default)]
struct PreparedScene {
    revision: u64,
    ready: bool,
    quad_fixed: Range<u32>,
    quad_scroll: Range<u32>,
    text_fixed: Range<u32>,
    text_scroll: Range<u32>,
    image_draws: Vec<ImageDraw>,
    system_glyph_draws: Vec<SystemGlyphDraw>,
    text_runs: Vec<PreparedTextRun>,
}

struct PreparedTextRun {
    run: TextRun,
    text: Vec<TextVertex>,
    system_glyphs: Vec<SystemGlyphVertices>,
}

struct SystemGlyphVertices {
    page: usize,
    vertices: Vec<TextVertex>,
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
    bytes: usize,
    last_used: u64,
}

struct ImageCache {
    bind_group_layout: wgpu::BindGroupLayout,
    images: HashMap<u64, CachedImage>,
    frame: u64,
}

impl ImageCache {
    fn new(bind_group_layout: wgpu::BindGroupLayout) -> Self {
        Self {
            bind_group_layout,
            images: HashMap::new(),
            frame: 0,
        }
    }

    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &ImageData,
    ) -> Result<&CachedImage> {
        let id = image.id();
        if !self.images.contains_key(&id) {
            let (pixels, width, height) = match image {
                ImageData::Asset(asset) => (
                    Cow::Owned(
                        miniz_oxide::inflate::decompress_to_vec_zlib(
                            asset.compressed_pixels.as_ref(),
                        )
                        .map_err(|error| {
                            anyhow!("an Ink image could not be decompressed: {error:?}")
                        })?,
                    ),
                    asset.width,
                    asset.height,
                ),
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
                    bytes: expected_length,
                    last_used: self.frame,
                },
            );
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

struct ImageDraw {
    id: u64,
    vertices: Range<u32>,
    scrolling: bool,
}

struct SystemGlyphDraw {
    page: usize,
    vertices: Range<u32>,
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
        Ok(cached)
    }
}

#[derive(Debug)]
pub enum RenderOutcome {
    Presented,
    Skipped,
    SurfaceLost,
    NeedsSystemGlyph(SystemGlyphRequest),
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
    quad_buffer: VertexBuffer<QuadVertex>,
    overlay_buffer: VertexBuffer<QuadVertex>,
    overlay_vertices: Vec<QuadVertex>,
    text_pipeline: wgpu::RenderPipeline,
    text_buffer: VertexBuffer<TextVertex>,
    image_pipeline: wgpu::RenderPipeline,
    image_buffer: VertexBuffer<TextVertex>,
    image_cache: ImageCache,
    system_glyph_buffer: VertexBuffer<TextVertex>,
    system_glyph_atlas: SystemGlyphAtlas,
    glyph_atlas: GlyphAtlas,
    prepared: PreparedScene,
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
        let image_fragment_shader = spirv_shader(
            &device,
            "image_fragment",
            include_bytes!(concat!(env!("OUT_DIR"), "/image_fragment.spv")),
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
                    array_stride: size_of::<QuadVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: size_of::<[f32; 2]>() as u64,
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
                    array_stride: size_of::<TextVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: size_of::<[f32; 2]>() as u64,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: size_of::<[f32; 4]>() as u64,
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
        let image_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Ink image bind group layout"),
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
        let image_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ink image pipeline layout"),
            bind_group_layouts: &[Some(&transform_layout), Some(&image_bind_group_layout)],
            immediate_size: 0,
        });
        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ink image pipeline"),
            layout: Some(&image_layout),
            vertex: wgpu::VertexState {
                module: &text_vertex_shader,
                entry_point: Some("text_vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<TextVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: size_of::<[f32; 2]>() as u64,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: size_of::<[f32; 4]>() as u64,
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
        });
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
        let quad_buffer = VertexBuffer::new(&device, "Ink quad vertices", MAX_QUADS * 6);
        let overlay_buffer = VertexBuffer::new(&device, "Ink overlay vertices", 18);
        let text_buffer = VertexBuffer::new(&device, "Ink text vertices", MAX_GLYPHS * 6);
        let image_buffer = VertexBuffer::new(&device, "Ink image vertices", MAX_QUADS * 6);
        let system_glyph_atlas = SystemGlyphAtlas::new(image_bind_group_layout.clone());
        let image_cache = ImageCache::new(image_bind_group_layout);
        let system_glyph_buffer =
            VertexBuffer::new(&device, "Ink system glyph vertices", MAX_GLYPHS * 6);
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
            overlay_vertices: Vec::with_capacity(12),
            text_pipeline,
            text_buffer,
            image_pipeline,
            image_buffer,
            image_cache,
            system_glyph_buffer,
            system_glyph_atlas,
            glyph_atlas,
            prepared: PreparedScene::default(),
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
            self.prepare(scene)?;
        }
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
        self.overlay_vertices.clear();
        if text_cursor_visible && let Some(cursor) = &scene.text_cursor {
            push_quad_vertices(&mut self.overlay_vertices, scene, cursor);
        }
        let cursor_end = self.overlay_vertices.len() as u32;
        push_scrollbar_vertices(scene, &mut self.overlay_vertices);
        let overlay_end = self.overlay_vertices.len() as u32;
        self.overlay_buffer
            .write(&self.device, &self.queue, &self.overlay_vertices);

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
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
                pass.set_vertex_buffer(0, self.quad_buffer.buffer.slice(..));
                pass.draw(self.prepared.quad_fixed.clone(), 0..1);
            }
            if !self.prepared.quad_scroll.is_empty() {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_bind_group(0, &self.scroll_transform, &[]);
                pass.set_vertex_buffer(0, self.quad_buffer.buffer.slice(..));
                set_scroll_scissor(&mut pass, scene);
                pass.draw(self.prepared.quad_scroll.clone(), 0..1);
                reset_scissor(&mut pass, scene);
            }
            if !self.prepared.image_draws.is_empty() {
                pass.set_pipeline(&self.image_pipeline);
                pass.set_vertex_buffer(0, self.image_buffer.buffer.slice(..));
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
                    pass.draw(draw.vertices.clone(), 0..1);
                }
                reset_scissor(&mut pass, scene);
            }
            if !self.prepared.system_glyph_draws.is_empty() {
                pass.set_pipeline(&self.image_pipeline);
                pass.set_vertex_buffer(0, self.system_glyph_buffer.buffer.slice(..));
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
                    pass.draw(draw.vertices.clone(), 0..1);
                }
                reset_scissor(&mut pass, scene);
            }
            if !self.prepared.text_fixed.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.fixed_transform, &[]);
                pass.set_bind_group(1, &self.glyph_atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, self.text_buffer.buffer.slice(..));
                pass.draw(self.prepared.text_fixed.clone(), 0..1);
            }
            if !self.prepared.text_scroll.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.scroll_transform, &[]);
                pass.set_bind_group(1, &self.glyph_atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, self.text_buffer.buffer.slice(..));
                set_scroll_scissor(&mut pass, scene);
                pass.draw(self.prepared.text_scroll.clone(), 0..1);
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
                pass.set_vertex_buffer(0, self.overlay_buffer.buffer.slice(..));
                if scrolling {
                    set_scroll_scissor(&mut pass, scene);
                }
                pass.draw(0..cursor_end, 0..1);
                if scrolling {
                    reset_scissor(&mut pass, scene);
                }
            }
            if cursor_end < overlay_end {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_bind_group(0, &self.fixed_transform, &[]);
                pass.set_vertex_buffer(0, self.overlay_buffer.buffer.slice(..));
                pass.draw(cursor_end..overlay_end, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(RenderOutcome::Presented)
    }

    pub fn install_system_glyph(&mut self, request_id: u64, pixels: Option<&[u8]>) -> Result<()> {
        self.system_glyph_atlas
            .install(&self.device, &self.queue, request_id, pixels)
    }

    fn prepare(&mut self, scene: &Scene) -> Result<()> {
        let mut quads = quad_vertices(scene, false);
        let quad_fixed_end = quads.len() as u32;
        quads.extend(quad_vertices(scene, true));

        let text_runs = self.prepare_text_runs(scene)?;
        let mut text = text_runs
            .iter()
            .filter(|run| !run.run.scrolling)
            .flat_map(|run| run.text.iter().copied())
            .collect::<Vec<_>>();
        text.extend(self.mask_vertices(scene, false)?);
        let text_fixed_end = text.len() as u32;
        text.extend(
            text_runs
                .iter()
                .filter(|run| run.run.scrolling)
                .flat_map(|run| run.text.iter().copied()),
        );
        text.extend(self.mask_vertices(scene, true)?);

        let mut system_glyph_groups = HashMap::<(bool, usize), Vec<TextVertex>>::new();
        for run in &text_runs {
            for glyphs in &run.system_glyphs {
                system_glyph_groups
                    .entry((run.run.scrolling, glyphs.page))
                    .or_default()
                    .extend_from_slice(&glyphs.vertices);
            }
        }
        let mut system_glyph_groups = system_glyph_groups.into_iter().collect::<Vec<_>>();
        system_glyph_groups.sort_by_key(|((scrolling, page), _)| (*scrolling, *page));
        let mut system_glyph_vertices = Vec::new();
        let mut system_glyph_draws = Vec::with_capacity(system_glyph_groups.len());
        for ((scrolling, page), vertices) in system_glyph_groups {
            let start = system_glyph_vertices.len() as u32;
            system_glyph_vertices.extend(vertices);
            system_glyph_draws.push(SystemGlyphDraw {
                page,
                vertices: start..system_glyph_vertices.len() as u32,
                scrolling,
            });
        }

        self.image_cache.begin_frame();
        let (mut images, mut image_draws) = self.image_vertices(scene, false)?;
        let (scroll_images, mut scroll_draws) = self.image_vertices(scene, true)?;
        let scroll_start = images.len() as u32;
        images.extend(scroll_images);
        for draw in &mut scroll_draws {
            draw.vertices.start += scroll_start;
            draw.vertices.end += scroll_start;
        }
        image_draws.extend(scroll_draws);
        self.image_cache.trim(
            &image_draws
                .iter()
                .map(|draw| draw.id)
                .collect::<HashSet<_>>(),
        );

        self.quad_buffer.write(&self.device, &self.queue, &quads);
        self.text_buffer.write(&self.device, &self.queue, &text);
        self.image_buffer.write(&self.device, &self.queue, &images);
        self.system_glyph_buffer
            .write(&self.device, &self.queue, &system_glyph_vertices);
        self.prepared = PreparedScene {
            revision: scene.revision,
            ready: true,
            quad_fixed: 0..quad_fixed_end,
            quad_scroll: quad_fixed_end..quads.len() as u32,
            text_fixed: 0..text_fixed_end,
            text_scroll: text_fixed_end..text.len() as u32,
            image_draws,
            system_glyph_draws,
            text_runs,
        };
        Ok(())
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
        let mut vertices = Vec::new();
        let mut system_glyphs = HashMap::<usize, Vec<TextVertex>>::new();
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
                            &mut vertices,
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
            text: vertices,
            system_glyphs: system_glyphs
                .into_iter()
                .map(|(page, vertices)| SystemGlyphVertices { page, vertices })
                .collect(),
        })
    }

    fn mask_vertices(&mut self, scene: &Scene, scrolling: bool) -> Result<Vec<TextVertex>> {
        let mut vertices = Vec::with_capacity(scene.masks.len() * 6);
        for run in scene.masks.iter().filter(|run| run.scrolling == scrolling) {
            let mask = self.glyph_atlas.mask(&self.queue, &run.mask)?;
            push_mask_quad(&mut vertices, scene, run.rect, run.clip, mask, run.colour);
        }
        Ok(vertices)
    }

    fn image_vertices(
        &mut self,
        scene: &Scene,
        scrolling: bool,
    ) -> Result<(Vec<TextVertex>, Vec<ImageDraw>)> {
        let mut vertices = Vec::with_capacity(scene.images.len() * 6);
        let mut draws = Vec::with_capacity(scene.images.len());
        for run in scene.images.iter().filter(|run| run.scrolling == scrolling) {
            let image = self
                .image_cache
                .prepare(&self.device, &self.queue, &run.image)?;
            let start = vertices.len() as u32;
            push_image_quad(
                &mut vertices,
                scene,
                run.rect,
                run.clip,
                image.width,
                image.height,
                run.fit,
            );
            draws.push(ImageDraw {
                id: run.image.id(),
                vertices: start..vertices.len() as u32,
                scrolling,
            });
        }
        Ok((vertices, draws))
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

fn raw_vertex_buffer<T>(
    device: &wgpu::Device,
    label: &'static str,
    vertices: usize,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (vertices.max(1) * size_of::<T>()) as u64,
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

fn quad_vertices(scene: &Scene, scrolling: bool) -> Vec<QuadVertex> {
    let mut vertices = Vec::with_capacity(scene.quads.len() * 6);
    for quad in scene
        .quads
        .iter()
        .filter(|quad| quad.scrolling == scrolling)
    {
        push_quad_vertices(&mut vertices, scene, quad);
    }
    vertices
}

fn push_quad_vertices(vertices: &mut Vec<QuadVertex>, scene: &Scene, quad: &ink_core::Quad) {
    let rect = intersect(quad.rect, quad.clip);
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let [left, top] = position(scene, rect.x, rect.y);
    let [right, bottom] = position(scene, rect.x + rect.width, rect.y + rect.height);
    let colour = colour(quad.colour);
    vertices.extend_from_slice(&[
        QuadVertex {
            position: [left, top],
            colour,
        },
        QuadVertex {
            position: [left, bottom],
            colour,
        },
        QuadVertex {
            position: [right, bottom],
            colour,
        },
        QuadVertex {
            position: [left, top],
            colour,
        },
        QuadVertex {
            position: [right, bottom],
            colour,
        },
        QuadVertex {
            position: [right, top],
            colour,
        },
    ]);
}

fn push_scrollbar_vertices(scene: &Scene, vertices: &mut Vec<QuadVertex>) {
    let Some(scrollbar) = scene.scroll_bar else {
        return;
    };
    let thumb = scrollbar.thumb_rect(scene.scroll_offset, scene.scroll_max);
    let clip = scene.scroll_clip.unwrap_or(scrollbar.track);
    for quad in [
        ink_core::Quad {
            rect: scrollbar.track,
            clip,
            colour: Colour::WHITE,
            scrolling: false,
        },
        ink_core::Quad {
            rect: thumb,
            clip,
            colour: Colour::WHITE,
            scrolling: false,
        },
    ] {
        push_quad_vertices(vertices, scene, &quad);
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
    vertices: &mut Vec<TextVertex>,
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
    vertices.extend_from_slice(&[
        TextVertex {
            position: [ndc_left, ndc_top],
            uv: [u0, v0],
            colour,
        },
        TextVertex {
            position: [ndc_left, ndc_bottom],
            uv: [u0, v1],
            colour,
        },
        TextVertex {
            position: [ndc_right, ndc_bottom],
            uv: [u1, v1],
            colour,
        },
        TextVertex {
            position: [ndc_left, ndc_top],
            uv: [u0, v0],
            colour,
        },
        TextVertex {
            position: [ndc_right, ndc_bottom],
            uv: [u1, v1],
            colour,
        },
        TextVertex {
            position: [ndc_right, ndc_top],
            uv: [u1, v0],
            colour,
        },
    ]);
}

fn push_mask_quad(
    vertices: &mut Vec<TextVertex>,
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
    vertices.extend_from_slice(&[
        TextVertex {
            position: [ndc_left, ndc_top],
            uv: [u0, v0],
            colour,
        },
        TextVertex {
            position: [ndc_left, ndc_bottom],
            uv: [u0, v1],
            colour,
        },
        TextVertex {
            position: [ndc_right, ndc_bottom],
            uv: [u1, v1],
            colour,
        },
        TextVertex {
            position: [ndc_left, ndc_top],
            uv: [u0, v0],
            colour,
        },
        TextVertex {
            position: [ndc_right, ndc_bottom],
            uv: [u1, v1],
            colour,
        },
        TextVertex {
            position: [ndc_right, ndc_top],
            uv: [u1, v0],
            colour,
        },
    ]);
}

fn push_image_quad(
    vertices: &mut Vec<TextVertex>,
    scene: &Scene,
    mut rect: Rect,
    clip: Rect,
    image_width: u32,
    image_height: u32,
    fit: ImageFit,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let image_aspect = image_width as f32 / image_height as f32;
    let rect_aspect = rect.width / rect.height;
    let (mut u0, mut v0, mut u1, mut v1) = (0.0, 0.0, 1.0, 1.0);
    match fit {
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
    vertices.extend_from_slice(&[
        TextVertex {
            position: [left, top],
            uv: [clipped_u0, clipped_v0],
            colour,
        },
        TextVertex {
            position: [left, bottom],
            uv: [clipped_u0, clipped_v1],
            colour,
        },
        TextVertex {
            position: [right, bottom],
            uv: [clipped_u1, clipped_v1],
            colour,
        },
        TextVertex {
            position: [left, top],
            uv: [clipped_u0, clipped_v0],
            colour,
        },
        TextVertex {
            position: [right, bottom],
            uv: [clipped_u1, clipped_v1],
            colour,
        },
        TextVertex {
            position: [right, top],
            uv: [clipped_u1, clipped_v0],
            colour,
        },
    ]);
}

fn push_system_glyph_quad(
    vertices: &mut Vec<TextVertex>,
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
    vertices.extend_from_slice(&[
        TextVertex {
            position: [left, top],
            uv: [clipped_u0, clipped_v0],
            colour,
        },
        TextVertex {
            position: [left, bottom],
            uv: [clipped_u0, clipped_v1],
            colour,
        },
        TextVertex {
            position: [right, bottom],
            uv: [clipped_u1, clipped_v1],
            colour,
        },
        TextVertex {
            position: [left, top],
            uv: [clipped_u0, clipped_v0],
            colour,
        },
        TextVertex {
            position: [right, bottom],
            uv: [clipped_u1, clipped_v1],
            colour,
        },
        TextVertex {
            position: [right, top],
            uv: [clipped_u1, clipped_v0],
            colour,
        },
    ]);
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
