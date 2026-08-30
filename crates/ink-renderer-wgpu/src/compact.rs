use std::{borrow::Cow, collections::HashMap, ops::Range};

use ab_glyph::{Font, FontRef, GlyphId, PxScale, ScaleFont};
use anyhow::{Context, Result, anyhow};
use bytemuck::{Pod, Zeroable};
use ink_core::{
    APPLE_EMOJI_ATLAS, Colour, ImageAsset, ImageFit, Mask, PUBLIC_SANS, Rect, Scene, TextAlign,
    emoji_index,
};
use unicode_segmentation::UnicodeSegmentation;

const MAX_QUADS: usize = 64;
const MAX_GLYPHS: usize = 512;
const ATLAS_SIZE: u32 = 1024;
const ATLAS_PADDING: u32 = 1;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct QuadVertex {
    position: [f32; 2],
    colour: [f32; 4],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct TextVertex {
    position: [f32; 2],
    uv: [f32; 2],
    colour: [f32; 4],
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
}

struct ImageCache {
    bind_group_layout: wgpu::BindGroupLayout,
    images: HashMap<u64, CachedImage>,
}

impl ImageCache {
    fn new(bind_group_layout: wgpu::BindGroupLayout) -> Self {
        Self {
            bind_group_layout,
            images: HashMap::new(),
        }
    }

    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        asset: ImageAsset,
    ) -> Result<&CachedImage> {
        if !self.images.contains_key(&asset.id) {
            let pixels = miniz_oxide::inflate::decompress_to_vec_zlib(asset.compressed_pixels)
                .map_err(|error| anyhow!("an Ink image could not be decompressed: {error:?}"))?;
            let width = asset.width;
            let height = asset.height;
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
                asset.id,
                CachedImage {
                    _texture: texture,
                    bind_group,
                    width,
                    height,
                },
            );
        }
        self.images
            .get(&asset.id)
            .context("an Ink image was not cached")
    }
}

struct ImageDraw {
    id: u64,
    vertices: Range<u32>,
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

    fn mask(&mut self, queue: &wgpu::Queue, mask: Mask) -> Result<CachedMask> {
        if let Some(mask) = self.masks.get(&mask.id) {
            return Ok(*mask);
        }

        let width = u32::from(mask.width);
        let height = u32::from(mask.height);
        if mask.pixels.len() != (width * height) as usize {
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
            mask.pixels,
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
}

pub struct Renderer {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    quad_pipeline: wgpu::RenderPipeline,
    quad_buffer: wgpu::Buffer,
    text_pipeline: wgpu::RenderPipeline,
    text_buffer: wgpu::Buffer,
    image_pipeline: wgpu::RenderPipeline,
    image_buffer: wgpu::Buffer,
    image_cache: ImageCache,
    glyph_atlas: GlyphAtlas,
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
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        let shader = unsafe {
            device.create_shader_module_passthrough(wgpu::ShaderModuleDescriptorPassthrough {
                label: Some("Ink SPIR-V shader"),
                entry_points: Cow::Borrowed(&[
                    shader_entry_point("quad_vertex"),
                    shader_entry_point("quad_fragment"),
                    shader_entry_point("text_vertex"),
                    shader_entry_point("text_fragment"),
                    shader_entry_point("image_fragment"),
                ]),
                spirv: Some(wgpu::util::make_spirv_raw(include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/ink.spv"
                )))),
                ..Default::default()
            })
        };
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
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ink quad pipeline"),
            layout: Some(&quad_layout),
            vertex: wgpu::VertexState {
                module: &shader,
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
                module: &shader,
                entry_point: Some("quad_fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(colour_target(format))],
            }),
            multiview_mask: None,
            cache: None,
        });
        let text_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ink text pipeline layout"),
            bind_group_layouts: &[Some(&glyph_bind_group_layout)],
            immediate_size: 0,
        });
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ink text pipeline"),
            layout: Some(&text_layout),
            vertex: wgpu::VertexState {
                module: &shader,
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
                module: &shader,
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
            bind_group_layouts: &[Some(&image_bind_group_layout)],
            immediate_size: 0,
        });
        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ink image pipeline"),
            layout: Some(&image_layout),
            vertex: wgpu::VertexState {
                module: &shader,
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
                module: &shader,
                entry_point: Some("image_fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(colour_target(format))],
            }),
            multiview_mask: None,
            cache: None,
        });
        let quad_buffer = vertex_buffer::<QuadVertex>(&device, "Ink quad vertices", MAX_QUADS);
        let text_buffer = vertex_buffer::<TextVertex>(&device, "Ink text vertices", MAX_GLYPHS);
        let image_buffer = vertex_buffer::<TextVertex>(&device, "Ink image vertices", MAX_QUADS);
        let image_cache = ImageCache::new(image_bind_group_layout);
        let glyph_atlas = GlyphAtlas::new(&device, &glyph_bind_group_layout)?;

        Ok(Self {
            _instance: instance,
            surface,
            device,
            queue,
            config,
            quad_pipeline,
            quad_buffer,
            text_pipeline,
            text_buffer,
            image_pipeline,
            image_buffer,
            image_cache,
            glyph_atlas,
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

    pub fn render(&mut self, scene: &Scene) -> Result<RenderOutcome> {
        if scene.width == 0 || scene.height == 0 {
            return Ok(RenderOutcome::Skipped);
        }
        if self.config.width != scene.width || self.config.height != scene.height {
            self.resize(scene.width, scene.height);
        }

        let quads = quad_vertices(scene);
        if quads.len() > MAX_QUADS * 6 {
            return Err(anyhow!("scene exceeds the prototype quad budget"));
        }
        let (mut text, emoji) = self.text_vertices(scene)?;
        text.extend(self.mask_vertices(scene)?);
        let (mut images, mut image_draws) = self.image_vertices(scene)?;
        if !emoji.is_empty() {
            self.image_cache
                .prepare(&self.device, &self.queue, APPLE_EMOJI_ATLAS)?;
            let start = images.len() as u32;
            images.extend(emoji);
            image_draws.push(ImageDraw {
                id: APPLE_EMOJI_ATLAS.id,
                vertices: start..images.len() as u32,
            });
        }
        if text.len() > MAX_GLYPHS * 6 {
            return Err(anyhow!("scene exceeds the prototype glyph budget"));
        }
        if !quads.is_empty() {
            self.queue
                .write_buffer(&self.quad_buffer, 0, bytemuck::cast_slice(&quads));
        }
        if !text.is_empty() {
            self.queue
                .write_buffer(&self.text_buffer, 0, bytemuck::cast_slice(&text));
        }
        if !images.is_empty() {
            self.queue
                .write_buffer(&self.image_buffer, 0, bytemuck::cast_slice(&images));
        }

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
            if !quads.is_empty() {
                pass.set_pipeline(&self.quad_pipeline);
                pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
                pass.draw(0..quads.len() as u32, 0..1);
            }
            if !images.is_empty() {
                pass.set_pipeline(&self.image_pipeline);
                pass.set_vertex_buffer(0, self.image_buffer.slice(..));
                for draw in &image_draws {
                    let image = self
                        .image_cache
                        .images
                        .get(&draw.id)
                        .expect("prepared image stays cached");
                    pass.set_bind_group(0, &image.bind_group, &[]);
                    pass.draw(draw.vertices.clone(), 0..1);
                }
            }
            if !text.is_empty() {
                pass.set_pipeline(&self.text_pipeline);
                pass.set_bind_group(0, &self.glyph_atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, self.text_buffer.slice(..));
                pass.draw(0..text.len() as u32, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(RenderOutcome::Presented)
    }

    fn text_vertices(&mut self, scene: &Scene) -> Result<(Vec<TextVertex>, Vec<TextVertex>)> {
        let font = self.glyph_atlas.font.clone();
        let mut vertices = Vec::new();
        let mut emoji = Vec::new();
        for run in &scene.text {
            let size = run.font_size.round().clamp(1.0, u16::MAX as f32) as u16;
            let scaled = font.as_scaled(PxScale::from(size as f32));
            let width = text_run_width(&scaled, &run.text, size as f32);
            let mut pen_x = match run.align {
                TextAlign::Start => run.rect.x,
                TextAlign::Centre => run.rect.x + (run.rect.width - width).max(0.0) / 2.0,
                TextAlign::End => run.rect.x + (run.rect.width - width).max(0.0),
            };
            let baseline = run.rect.y + (run.rect.height - scaled.height()) / 2.0 + scaled.ascent();
            let mut previous = None;
            for grapheme in run.text.graphemes(true) {
                if let Some(index) = emoji_index(grapheme) {
                    let emoji_size = size as f32 * 0.9;
                    push_atlas_quad(
                        &mut emoji,
                        scene,
                        Rect {
                            x: pen_x + (size as f32 - emoji_size) / 2.0,
                            y: run.rect.y + (run.rect.height - emoji_size) / 2.0,
                            width: emoji_size,
                            height: emoji_size,
                        },
                        intersect(run.rect, run.clip),
                        index,
                    );
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
                            intersect(run.rect, run.clip),
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
            }
        }
        Ok((vertices, emoji))
    }

    fn mask_vertices(&mut self, scene: &Scene) -> Result<Vec<TextVertex>> {
        let mut vertices = Vec::with_capacity(scene.masks.len() * 6);
        for run in &scene.masks {
            let mask = self.glyph_atlas.mask(&self.queue, run.mask)?;
            push_mask_quad(&mut vertices, scene, run.rect, run.clip, mask, run.colour);
        }
        Ok(vertices)
    }

    fn image_vertices(&mut self, scene: &Scene) -> Result<(Vec<TextVertex>, Vec<ImageDraw>)> {
        let mut vertices = Vec::with_capacity(scene.images.len() * 6);
        let mut draws = Vec::with_capacity(scene.images.len());
        for run in &scene.images {
            let image = self
                .image_cache
                .prepare(&self.device, &self.queue, run.asset)?;
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
                id: run.asset.id,
                vertices: start..vertices.len() as u32,
            });
        }
        Ok((vertices, draws))
    }
}

fn text_run_width<F: Font>(font: &impl ScaleFont<F>, text: &str, emoji_size: f32) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for grapheme in text.graphemes(true) {
        if emoji_index(grapheme).is_some() {
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

fn vertex_buffer<T>(device: &wgpu::Device, label: &'static str, primitives: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (primitives * 6 * size_of::<T>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn quad_vertices(scene: &Scene) -> Vec<QuadVertex> {
    let mut vertices = Vec::with_capacity(scene.quads.len() * 6);
    for quad in &scene.quads {
        let rect = intersect(quad.rect, quad.clip);
        if rect.width <= 0.0 || rect.height <= 0.0 {
            continue;
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
    vertices
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

fn push_atlas_quad(
    vertices: &mut Vec<TextVertex>,
    scene: &Scene,
    rect: Rect,
    clip: Rect,
    index: usize,
) {
    let visible = intersect(rect, clip);
    if visible.width <= 0.0 || visible.height <= 0.0 {
        return;
    }
    let column = index % 8;
    let row = index / 8;
    let cell_u = 1.0 / 8.0;
    let cell_v = 1.0 / 3.0;
    let u0 = column as f32 * cell_u;
    let v0 = row as f32 * cell_v;
    let u1 = u0 + cell_u;
    let v1 = v0 + cell_v;
    let clipped_u0 = u0 + (visible.x - rect.x) / rect.width * cell_u;
    let clipped_v0 = v0 + (visible.y - rect.y) / rect.height * cell_v;
    let clipped_u1 = u1 - (rect.x + rect.width - visible.x - visible.width) / rect.width * cell_u;
    let clipped_v1 =
        v1 - (rect.y + rect.height - visible.y - visible.height) / rect.height * cell_v;
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
