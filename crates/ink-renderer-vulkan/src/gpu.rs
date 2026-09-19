//! Vulkan resources and presentation. Frames complete before shared resources are reused.
use anyhow::{Context, Result, anyhow, ensure};
use ash::{Entry, vk};
use std::{
    ffi::c_void,
    ops::Range,
    sync::{Arc, Mutex},
};

// Release partially created Vulkan objects if initialisation fails.
struct Cleanup<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Cleanup<F> {
    fn disarm(mut self) {
        self.0.take();
    }
}
impl<F: FnOnce()> Drop for Cleanup<F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.0.take() {
            cleanup();
        }
    }
}

pub type TextureFormat = vk::Format;
pub type Queue = Device;

pub fn surface_lost(error: &anyhow::Error) -> bool {
    error.downcast_ref::<vk::Result>() == Some(&vk::Result::ERROR_SURFACE_LOST_KHR)
}
pub struct SurfaceConfiguration {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}
#[derive(Clone)]
pub struct Device(Arc<Core>, Arc<Mutex<Uploads>>);

#[derive(Default)]
struct Uploads {
    commands: Vec<Upload>,
    bytes: Vec<u8>,
}
enum Upload {
    Clear(Texture),
    Write {
        texture: Texture,
        offset: usize,
        origin: [u32; 2],
        size: [u32; 2],
    },
}
struct Core {
    _entry: Entry,
    instance: ash::Instance,
    device: ash::Device,
    physical: vk::PhysicalDevice,
    queue: vk::Queue,
    memory: vk::PhysicalDeviceMemoryProperties,
    commands: vk::CommandPool,
    upload: vk::CommandBuffer,
    descriptors: vk::DescriptorPool,
    uniform_layout: vk::DescriptorSetLayout,
    texture_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    sampler: vk::Sampler,
}
impl Drop for Core {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_sampler(self.sampler, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_descriptor_pool(self.descriptors, None);
            self.device
                .destroy_descriptor_set_layout(self.texture_layout, None);
            self.device
                .destroy_descriptor_set_layout(self.uniform_layout, None);
            self.device.destroy_command_pool(self.commands, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
impl Core {
    fn memory_type(&self, bits: u32, flags: vk::MemoryPropertyFlags) -> u32 {
        (0..self.memory.memory_type_count)
            .find(|i| {
                bits & (1 << i) != 0
                    && self.memory.memory_types[*i as usize]
                        .property_flags
                        .contains(flags)
            })
            .expect("Vulkan memory type")
    }
    fn set(&self, layout: vk::DescriptorSetLayout) -> vk::DescriptorSet {
        unsafe {
            self.device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(self.descriptors)
                        .set_layouts(&[layout]),
                )
                .expect("allocate descriptor")[0]
        }
    }
    fn immediate(&self, record: impl FnOnce(vk::CommandBuffer)) {
        unsafe {
            self.device
                .reset_command_buffer(self.upload, vk::CommandBufferResetFlags::empty())
                .expect("reset upload");
            self.device
                .begin_command_buffer(
                    self.upload,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .expect("begin upload");
            record(self.upload);
            self.device
                .end_command_buffer(self.upload)
                .expect("end upload");
            self.device
                .queue_submit(
                    self.queue,
                    &[vk::SubmitInfo::default().command_buffers(&[self.upload])],
                    vk::Fence::null(),
                )
                .expect("submit upload");
            self.device
                .queue_wait_idle(self.queue)
                .expect("finish upload");
        }
    }
}
#[derive(Clone)]
pub struct Buffer(Arc<BufferInner>);
struct BufferInner {
    core: Arc<Core>,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: usize,
    set: vk::DescriptorSet,
}
impl Drop for BufferInner {
    fn drop(&mut self) {
        unsafe {
            let d = &self.core.device;
            if self.set != vk::DescriptorSet::null() {
                let _ = d.free_descriptor_sets(self.core.descriptors, &[self.set]);
            }
            d.destroy_buffer(self.buffer, None);
            d.free_memory(self.memory, None);
        }
    }
}
pub enum BindGroup {
    Uniform(Buffer),
    Texture(Texture),
}
impl BindGroup {
    fn set(&self) -> vk::DescriptorSet {
        match self {
            Self::Uniform(b) => b.0.set,
            Self::Texture(t) => t.0.set,
        }
    }
}
impl Buffer {
    pub fn bind_group(&self) -> BindGroup {
        BindGroup::Uniform(self.clone())
    }
}
#[derive(Clone)]
pub struct Texture(Arc<TextureInner>);
struct TextureInner {
    core: Arc<Core>,
    image: vk::Image,
    view: vk::ImageView,
    memory: vk::DeviceMemory,
    set: vk::DescriptorSet,
    width: u32,
    height: u32,
    channels: usize,
}
impl Drop for TextureInner {
    fn drop(&mut self) {
        unsafe {
            let d = &self.core.device;
            let _ = d.free_descriptor_sets(self.core.descriptors, &[self.set]);
            d.destroy_image_view(self.view, None);
            d.destroy_image(self.image, None);
            d.free_memory(self.memory, None);
        }
    }
}
impl Texture {
    pub fn bind_group(&self) -> BindGroup {
        BindGroup::Texture(self.clone())
    }
}
pub struct RenderPipeline {
    core: Arc<Core>,
    pipeline: vk::Pipeline,
}
impl Drop for RenderPipeline {
    fn drop(&mut self) {
        unsafe { self.core.device.destroy_pipeline(self.pipeline, None) }
    }
}
fn colour_range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(1)
}
fn barrier(
    d: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    old: vk::ImageLayout,
    new: vk::ImageLayout,
    src: vk::AccessFlags,
    dst: vk::AccessFlags,
) {
    unsafe {
        d.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[vk::ImageMemoryBarrier::default()
                .old_layout(old)
                .new_layout(new)
                .src_access_mask(src)
                .dst_access_mask(dst)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(colour_range())],
        );
    }
}
impl Device {
    pub fn buffer(&self, size: usize, uniform: bool) -> Buffer {
        unsafe {
            let c = &self.0;
            let d = &c.device;
            let buffer = d
                .create_buffer(
                    &vk::BufferCreateInfo::default()
                        .size(size as u64)
                        .usage(if uniform {
                            vk::BufferUsageFlags::UNIFORM_BUFFER
                        } else {
                            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC
                        }),
                    None,
                )
                .expect("create buffer");
            let req = d.get_buffer_memory_requirements(buffer);
            let memory = d
                .allocate_memory(
                    &vk::MemoryAllocateInfo::default()
                        .allocation_size(req.size)
                        .memory_type_index(c.memory_type(
                            req.memory_type_bits,
                            vk::MemoryPropertyFlags::HOST_VISIBLE
                                | vk::MemoryPropertyFlags::HOST_COHERENT,
                        )),
                    None,
                )
                .expect("allocate buffer");
            d.bind_buffer_memory(buffer, memory, 0)
                .expect("bind buffer");
            let set = if uniform {
                let set = c.set(c.uniform_layout);
                d.update_descriptor_sets(
                    &[vk::WriteDescriptorSet::default()
                        .dst_set(set)
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                        .buffer_info(&[vk::DescriptorBufferInfo::default()
                            .buffer(buffer)
                            .range(size as u64)])],
                    &[],
                );
                set
            } else {
                vk::DescriptorSet::null()
            };
            Buffer(Arc::new(BufferInner {
                core: c.clone(),
                buffer,
                memory,
                size,
                set,
            }))
        }
    }
    pub fn write_buffer(&self, buffer: &Buffer, offset: u64, bytes: &[u8]) {
        unsafe {
            assert!(offset as usize + bytes.len() <= buffer.0.size);
            if bytes.is_empty() {
                return;
            }
            let ptr = self
                .0
                .device
                .map_memory(
                    buffer.0.memory,
                    offset,
                    bytes.len() as u64,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("map buffer");
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast(), bytes.len());
            self.0.device.unmap_memory(buffer.0.memory);
        }
    }
    pub fn texture(&self, width: u32, height: u32, mask: bool) -> Texture {
        unsafe {
            let c = &self.0;
            let d = &c.device;
            let image = d
                .create_image(
                    &vk::ImageCreateInfo::default()
                        .image_type(vk::ImageType::TYPE_2D)
                        .format(if mask {
                            vk::Format::R8_UNORM
                        } else {
                            vk::Format::R8G8B8A8_SRGB
                        })
                        .extent(vk::Extent3D {
                            width,
                            height,
                            depth: 1,
                        })
                        .mip_levels(1)
                        .array_layers(1)
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .tiling(vk::ImageTiling::OPTIMAL)
                        .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST),
                    None,
                )
                .expect("create image");
            let req = d.get_image_memory_requirements(image);
            let memory = d
                .allocate_memory(
                    &vk::MemoryAllocateInfo::default()
                        .allocation_size(req.size)
                        .memory_type_index(c.memory_type(
                            req.memory_type_bits,
                            vk::MemoryPropertyFlags::DEVICE_LOCAL,
                        )),
                    None,
                )
                .expect("allocate image");
            d.bind_image_memory(image, memory, 0).expect("bind image");
            let view = d
                .create_image_view(
                    &vk::ImageViewCreateInfo::default()
                        .image(image)
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(if mask {
                            vk::Format::R8_UNORM
                        } else {
                            vk::Format::R8G8B8A8_SRGB
                        })
                        .subresource_range(colour_range()),
                    None,
                )
                .expect("image view");
            let set = c.set(c.texture_layout);
            d.update_descriptor_sets(
                &[
                    vk::WriteDescriptorSet::default()
                        .dst_set(set)
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                        .image_info(&[vk::DescriptorImageInfo::default()
                            .image_view(view)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]),
                    vk::WriteDescriptorSet::default()
                        .dst_set(set)
                        .dst_binding(1)
                        .descriptor_type(vk::DescriptorType::SAMPLER)
                        .image_info(&[vk::DescriptorImageInfo::default().sampler(c.sampler)]),
                ],
                &[],
            );
            let texture = Texture(Arc::new(TextureInner {
                core: c.clone(),
                image,
                view,
                memory,
                set,
                width,
                height,
                channels: if mask { 1 } else { 4 },
            }));
            self.1
                .lock()
                .unwrap()
                .commands
                .push(Upload::Clear(texture.clone()));
            texture
        }
    }
    pub fn write_texture(&self, texture: &Texture, origin: [u32; 2], size: [u32; 2], bytes: &[u8]) {
        assert!(origin[0] + size[0] <= texture.0.width && origin[1] + size[1] <= texture.0.height);
        assert_eq!(
            bytes.len(),
            size[0] as usize * size[1] as usize * texture.0.channels
        );
        let mut uploads = self.1.lock().unwrap();
        let offset = uploads.bytes.len().next_multiple_of(4);
        uploads.bytes.resize(offset, 0);
        uploads.bytes.extend_from_slice(bytes);
        uploads.commands.push(Upload::Write {
            texture: texture.clone(),
            offset,
            origin,
            size,
        });
    }

    pub fn flush_uploads(&self) {
        let uploads = std::mem::take(&mut *self.1.lock().unwrap());
        if uploads.commands.is_empty() {
            return;
        }
        let staging = if uploads.bytes.is_empty() {
            None
        } else {
            let buffer = self.buffer(uploads.bytes.len(), false);
            self.write_buffer(&buffer, 0, &uploads.bytes);
            Some(buffer)
        };
        let d = &self.0.device;
        self.0.immediate(|cmd| unsafe {
            for upload in &uploads.commands {
                match upload {
                    Upload::Clear(texture) => {
                        let image = texture.0.image;
                        barrier(
                            d,
                            cmd,
                            image,
                            vk::ImageLayout::UNDEFINED,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            vk::AccessFlags::empty(),
                            vk::AccessFlags::TRANSFER_WRITE,
                        );
                        d.cmd_clear_color_image(
                            cmd,
                            image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            &vk::ClearColorValue { float32: [0.; 4] },
                            &[colour_range()],
                        );
                        barrier(
                            d,
                            cmd,
                            image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                            vk::AccessFlags::TRANSFER_WRITE,
                            vk::AccessFlags::SHADER_READ,
                        );
                    }
                    Upload::Write {
                        texture,
                        offset,
                        origin,
                        size,
                    } => {
                        let image = texture.0.image;
                        barrier(
                            d,
                            cmd,
                            image,
                            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            vk::AccessFlags::SHADER_READ,
                            vk::AccessFlags::TRANSFER_WRITE,
                        );
                        d.cmd_copy_buffer_to_image(
                            cmd,
                            staging
                                .as_ref()
                                .expect("upload bytes have a staging buffer")
                                .0
                                .buffer,
                            image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            &[vk::BufferImageCopy::default()
                                .buffer_offset(*offset as u64)
                                .image_subresource(
                                    vk::ImageSubresourceLayers::default()
                                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                                        .layer_count(1),
                                )
                                .image_offset(vk::Offset3D {
                                    x: origin[0] as i32,
                                    y: origin[1] as i32,
                                    z: 0,
                                })
                                .image_extent(vk::Extent3D {
                                    width: size[0],
                                    height: size[1],
                                    depth: 1,
                                })],
                        );
                        barrier(
                            d,
                            cmd,
                            image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                            vk::AccessFlags::TRANSFER_WRITE,
                            vk::AccessFlags::SHADER_READ,
                        );
                    }
                }
            }
        });
        // The upload submission has finished before staging memory or textures are released.
    }
    pub fn pipeline(
        &self,
        format: TextureFormat,
        text: bool,
        image: bool,
    ) -> Result<RenderPipeline> {
        unsafe {
            let d = &self.0.device;
            let (vs, fs, vname, fname) = if !text {
                (
                    include_bytes!(concat!(env!("OUT_DIR"), "/quad_vertex.spv")).as_slice(),
                    include_bytes!(concat!(env!("OUT_DIR"), "/quad_fragment.spv")).as_slice(),
                    c"quad_vertex",
                    c"quad_fragment",
                )
            } else if image {
                (
                    include_bytes!(concat!(env!("OUT_DIR"), "/text_vertex.spv")).as_slice(),
                    include_bytes!(concat!(env!("OUT_DIR"), "/image_fragment.spv")).as_slice(),
                    c"text_vertex",
                    c"image_fragment",
                )
            } else {
                (
                    include_bytes!(concat!(env!("OUT_DIR"), "/text_vertex.spv")).as_slice(),
                    include_bytes!(concat!(env!("OUT_DIR"), "/text_fragment.spv")).as_slice(),
                    c"text_vertex",
                    c"text_fragment",
                )
            };
            let words = |bytes: &[u8]| {
                bytes
                    .chunks_exact(4)
                    .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
                    .collect::<Vec<_>>()
            };
            let vertex = d.create_shader_module(
                &vk::ShaderModuleCreateInfo::default().code(&words(vs)),
                None,
            )?;
            let _vertex = Cleanup(Some(|| d.destroy_shader_module(vertex, None)));
            let fragment = d.create_shader_module(
                &vk::ShaderModuleCreateInfo::default().code(&words(fs)),
                None,
            )?;
            let _fragment = Cleanup(Some(|| d.destroy_shader_module(fragment, None)));
            let pass = render_pass(d, format)?;
            let _pass = Cleanup(Some(|| d.destroy_render_pass(pass, None)));
            let stages = [
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vertex)
                    .name(vname),
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(fragment)
                    .name(fname),
            ];
            let binding = [vk::VertexInputBindingDescription {
                binding: 0,
                stride: if text { 48 } else { 32 },
                input_rate: vk::VertexInputRate::INSTANCE,
            }];
            let attrs = (0..if text { 3 } else { 2 })
                .map(|i| vk::VertexInputAttributeDescription {
                    location: i,
                    binding: 0,
                    format: vk::Format::R32G32B32A32_SFLOAT,
                    offset: i * 16,
                })
                .collect::<Vec<_>>();
            let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&binding)
                .vertex_attribute_descriptions(&attrs);
            let assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
            let viewport = vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1);
            let raster = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .line_width(1.);
            let multisample = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            let attachment = [vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA)];
            let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&attachment);
            let dynamic = vk::PipelineDynamicStateCreateInfo::default()
                .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
            let result = d.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[vk::GraphicsPipelineCreateInfo::default()
                    .stages(&stages)
                    .vertex_input_state(&vertex_input)
                    .input_assembly_state(&assembly)
                    .viewport_state(&viewport)
                    .rasterization_state(&raster)
                    .multisample_state(&multisample)
                    .color_blend_state(&blend)
                    .dynamic_state(&dynamic)
                    .layout(self.0.pipeline_layout)
                    .render_pass(pass)],
                None,
            );
            let pipeline = result.map_err(|(pipelines, error)| {
                for pipeline in pipelines {
                    d.destroy_pipeline(pipeline, None);
                }
                anyhow!("graphics pipeline: {error:?}")
            })?[0];
            Ok(RenderPipeline {
                core: self.0.clone(),
                pipeline,
            })
        }
    }
}
fn render_pass(d: &ash::Device, format: vk::Format) -> Result<vk::RenderPass> {
    unsafe {
        let attachments = [vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];
        let colours = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let subpass = [vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&colours)];
        let dependencies = [vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];
        Ok(d.create_render_pass(
            &vk::RenderPassCreateInfo::default()
                .attachments(&attachments)
                .subpasses(&subpass)
                .dependencies(&dependencies),
            None,
        )?)
    }
}

#[cfg(feature = "perf")]
struct GpuTiming {
    core: Arc<Core>,
    pool: vk::QueryPool,
    period: f64,
    mask: u64,
}

#[cfg(feature = "perf")]
impl GpuTiming {
    fn new(core: Arc<Core>, family: u32) -> Result<Option<Self>> {
        unsafe {
            let bits = core
                .instance
                .get_physical_device_queue_family_properties(core.physical)[family as usize]
                .timestamp_valid_bits;
            if bits == 0 {
                return Ok(None);
            }
            let period = core
                .instance
                .get_physical_device_properties(core.physical)
                .limits
                .timestamp_period as f64;
            let pool = core.device.create_query_pool(
                &vk::QueryPoolCreateInfo::default()
                    .query_type(vk::QueryType::TIMESTAMP)
                    .query_count(2),
                None,
            )?;
            Ok(Some(Self {
                core,
                pool,
                period,
                mask: u64::MAX >> (64 - bits),
            }))
        }
    }
    fn read(&self) -> Result<u64> {
        let mut ticks = [0u64; 2];
        unsafe {
            self.core.device.get_query_pool_results(
                self.pool,
                0,
                &mut ticks,
                vk::QueryResultFlags::TYPE_64,
            )?;
        }
        Ok(((ticks[1].wrapping_sub(ticks[0]) & self.mask) as f64 * self.period) as u64)
    }
}

#[cfg(feature = "perf")]
impl Drop for GpuTiming {
    fn drop(&mut self) {
        unsafe {
            self.core.device.destroy_query_pool(self.pool, None);
        }
    }
}

pub struct Surface {
    core: Arc<Core>,
    loader: ash::khr::surface::Instance,
    swap: ash::khr::swapchain::Device,
    surface: vk::SurfaceKHR,
    chain: vk::SwapchainKHR,
    format: vk::Format,
    extent: vk::Extent2D,
    views: Vec<vk::ImageView>,
    frames: Vec<vk::Framebuffer>,
    pass: vk::RenderPass,
    command: vk::CommandBuffer,
    acquired: vk::Semaphore,
    rendered: Vec<vk::Semaphore>,
    fence: vk::Fence,
    submitted: bool,
    dirty: bool,
    requested: [u32; 2],
    #[cfg(feature = "perf")]
    timing: Option<GpuTiming>,
    #[cfg(feature = "perf")]
    pub gpu_ns: Option<u64>,
    #[cfg(feature = "presentation-timing")]
    display_timing: Option<ash::google::display_timing::Device>,
    #[cfg(feature = "presentation-timing")]
    pub present_id: u32,
}
impl Surface {
    pub unsafe fn new(window: *mut c_void, width: u32, height: u32) -> Result<Self> {
        unsafe {
            let entry = Entry::load().context("load Vulkan")?;
            let app = vk::ApplicationInfo::default()
                .application_name(c"Ink direct Vulkan")
                .api_version(vk::API_VERSION_1_1);
            let extensions = [
                ash::khr::surface::NAME.as_ptr(),
                ash::khr::android_surface::NAME.as_ptr(),
            ];
            let instance = entry.create_instance(
                &vk::InstanceCreateInfo::default()
                    .application_info(&app)
                    .enabled_extension_names(&extensions),
                None,
            )?;
            let cleanup_instance = Cleanup(Some(|| instance.destroy_instance(None)));
            let loader = ash::khr::surface::Instance::new(&entry, &instance);
            let android = ash::khr::android_surface::Instance::new(&entry, &instance);
            let surface = android.create_android_surface(
                &vk::AndroidSurfaceCreateInfoKHR::default().window(window.cast()),
                None,
            )?;
            let cleanup_surface = Cleanup(Some(|| loader.destroy_surface(surface, None)));
            let mut choice = None;
            for physical in instance.enumerate_physical_devices()? {
                if instance
                    .get_physical_device_properties(physical)
                    .api_version
                    < vk::API_VERSION_1_1
                {
                    continue;
                }
                for (i, props) in instance
                    .get_physical_device_queue_family_properties(physical)
                    .iter()
                    .enumerate()
                {
                    if props.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                        && loader
                            .get_physical_device_surface_support(physical, i as u32, surface)?
                    {
                        choice = Some((physical, i as u32));
                        break;
                    }
                }
                if choice.is_some() {
                    break;
                }
            }
            let (physical, family) = choice.context("no Vulkan presentation queue")?;
            let extensions = [ash::khr::swapchain::NAME.as_ptr()];
            #[cfg(feature = "presentation-timing")]
            let (extensions, has_display_timing) = {
                let supported = instance
                    .enumerate_device_extension_properties(physical)?
                    .iter()
                    .any(|extension| {
                        extension.extension_name_as_c_str().ok()
                            == Some(ash::google::display_timing::NAME)
                    });
                let mut extensions = extensions.to_vec();
                if supported {
                    extensions.push(ash::google::display_timing::NAME.as_ptr());
                }
                (extensions, supported)
            };
            let device = instance.create_device(
                physical,
                &vk::DeviceCreateInfo::default()
                    .queue_create_infos(&[vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(family)
                        .queue_priorities(&[1.])])
                    .enabled_extension_names(&extensions),
                None,
            )?;
            let cleanup_device = Cleanup(Some(|| device.destroy_device(None)));
            let queue = device.get_device_queue(family, 0);
            let memory = instance.get_physical_device_memory_properties(physical);
            let commands = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(family)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )?;
            let cleanup_commands = Cleanup(Some(|| device.destroy_command_pool(commands, None)));
            let buffers = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(commands)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(2),
            )?;
            let uniform_layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&[
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(0)
                        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::VERTEX),
                ]),
                None,
            )?;
            let cleanup_uniform_layout = Cleanup(Some(|| {
                device.destroy_descriptor_set_layout(uniform_layout, None)
            }));
            let texture_layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(&[
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(0)
                        .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(1)
                        .descriptor_type(vk::DescriptorType::SAMPLER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                ]),
                None,
            )?;
            let cleanup_texture_layout = Cleanup(Some(|| {
                device.destroy_descriptor_set_layout(texture_layout, None)
            }));
            let pipeline_layout = device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .set_layouts(&[uniform_layout, texture_layout]),
                None,
            )?;
            let cleanup_pipeline_layout = Cleanup(Some(|| {
                device.destroy_pipeline_layout(pipeline_layout, None)
            }));
            let descriptors = device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                    .max_sets(4096)
                    .pool_sizes(&[
                        vk::DescriptorPoolSize {
                            ty: vk::DescriptorType::UNIFORM_BUFFER,
                            descriptor_count: 16,
                        },
                        vk::DescriptorPoolSize {
                            ty: vk::DescriptorType::SAMPLED_IMAGE,
                            descriptor_count: 4096,
                        },
                        vk::DescriptorPoolSize {
                            ty: vk::DescriptorType::SAMPLER,
                            descriptor_count: 4096,
                        },
                    ]),
                None,
            )?;
            let cleanup_descriptors =
                Cleanup(Some(|| device.destroy_descriptor_pool(descriptors, None)));
            let sampler = device.create_sampler(
                &vk::SamplerCreateInfo::default()
                    .mag_filter(vk::Filter::LINEAR)
                    .min_filter(vk::Filter::LINEAR)
                    .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE),
                None,
            )?;
            let cleanup_sampler = Cleanup(Some(|| device.destroy_sampler(sampler, None)));
            let acquired = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;
            let cleanup_acquired = Cleanup(Some(|| device.destroy_semaphore(acquired, None)));
            let fence = device.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?;
            let cleanup_fence = Cleanup(Some(|| device.destroy_fence(fence, None)));
            let swap = ash::khr::swapchain::Device::new(&instance, &device);
            #[cfg(feature = "presentation-timing")]
            let display_timing = has_display_timing
                .then(|| ash::google::display_timing::Device::new(&instance, &device));
            let formats = loader.get_physical_device_surface_formats(physical, surface)?;
            let format = formats
                .iter()
                .find(|f| {
                    matches!(
                        f.format,
                        vk::Format::R8G8B8A8_SRGB | vk::Format::B8G8R8A8_SRGB
                    )
                })
                .context("surface needs an sRGB format")?
                .format;
            let pass = render_pass(&device, format)?;
            let cleanup_pass = Cleanup(Some(|| device.destroy_render_pass(pass, None)));
            cleanup_pass.disarm();
            cleanup_fence.disarm();
            cleanup_acquired.disarm();
            cleanup_sampler.disarm();
            cleanup_descriptors.disarm();
            cleanup_pipeline_layout.disarm();
            cleanup_texture_layout.disarm();
            cleanup_uniform_layout.disarm();
            cleanup_commands.disarm();
            cleanup_device.disarm();
            cleanup_surface.disarm();
            cleanup_instance.disarm();
            let core = Arc::new(Core {
                _entry: entry,
                instance,
                device,
                physical,
                queue,
                memory,
                commands,
                upload: buffers[0],
                descriptors,
                uniform_layout,
                texture_layout,
                pipeline_layout,
                sampler,
            });
            let mut s = Self {
                core,
                loader,
                swap,
                surface,
                chain: vk::SwapchainKHR::null(),
                format,
                extent: vk::Extent2D::default(),
                views: vec![],
                frames: vec![],
                pass,
                command: buffers[1],
                acquired,
                rendered: vec![],
                fence,
                submitted: false,
                dirty: true,
                requested: [width, height],
                #[cfg(feature = "perf")]
                timing: None,
                #[cfg(feature = "perf")]
                gpu_ns: None,
                #[cfg(feature = "presentation-timing")]
                display_timing,
                #[cfg(feature = "presentation-timing")]
                present_id: 0,
            };
            #[cfg(feature = "perf")]
            {
                s.timing = GpuTiming::new(s.core.clone(), family)?;
            }
            s.recreate()?;
            Ok(s)
        }
    }
    pub fn device(&self) -> Device {
        Device(self.core.clone(), Arc::default())
    }
    #[cfg(feature = "presentation-timing")]
    pub fn presentation_times(&self) -> Result<Option<Vec<(u32, u64)>>> {
        let Some(timing) = &self.display_timing else {
            return Ok(None);
        };
        let times = unsafe { timing.get_past_presentation_timing(self.chain)? };
        Ok(Some(
            times
                .into_iter()
                .map(|time| (time.present_id, time.actual_present_time))
                .collect(),
        ))
    }
    pub fn format(&self) -> vk::Format {
        self.format
    }
    pub fn resize(&mut self, w: u32, h: u32) {
        self.requested = [w, h];
        self.dirty = true;
    }
    unsafe fn clear_chain(&mut self) {
        unsafe {
            for f in self.frames.drain(..) {
                self.core.device.destroy_framebuffer(f, None)
            }
            for v in self.views.drain(..) {
                self.core.device.destroy_image_view(v, None)
            }
            for s in self.rendered.drain(..) {
                self.core.device.destroy_semaphore(s, None)
            }
            self.swap.destroy_swapchain(self.chain, None);
            self.chain = vk::SwapchainKHR::null();
        }
    }
    fn recreate(&mut self) -> Result<()> {
        unsafe {
            self.core.device.device_wait_idle()?;
            let caps = self
                .loader
                .get_physical_device_surface_capabilities(self.core.physical, self.surface)?;
            let extent = if caps.current_extent.width != u32::MAX {
                caps.current_extent
            } else {
                vk::Extent2D {
                    width: self.requested[0]
                        .clamp(caps.min_image_extent.width, caps.max_image_extent.width),
                    height: self.requested[1]
                        .clamp(caps.min_image_extent.height, caps.max_image_extent.height),
                }
            };
            ensure!(
                extent.width > 0 && extent.height > 0,
                "zero-sized Vulkan surface"
            );
            let count = if caps.max_image_count == 0 {
                caps.min_image_count + 1
            } else {
                (caps.min_image_count + 1).min(caps.max_image_count)
            };
            let alpha = [
                vk::CompositeAlphaFlagsKHR::OPAQUE,
                vk::CompositeAlphaFlagsKHR::INHERIT,
                vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
                vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
            ]
            .into_iter()
            .find(|a| caps.supported_composite_alpha.contains(*a))
            .context("surface alpha mode")?;
            self.clear_chain();
            self.chain = self.swap.create_swapchain(
                &vk::SwapchainCreateInfoKHR::default()
                    .surface(self.surface)
                    .min_image_count(count)
                    .image_format(self.format)
                    .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                    .image_extent(extent)
                    .image_array_layers(1)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .pre_transform(caps.current_transform)
                    .composite_alpha(alpha)
                    .present_mode(vk::PresentModeKHR::FIFO)
                    .clipped(true),
                None,
            )?;
            self.extent = extent;
            for image in self.swap.get_swapchain_images(self.chain)? {
                let view = self.core.device.create_image_view(
                    &vk::ImageViewCreateInfo::default()
                        .image(image)
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(self.format)
                        .subresource_range(colour_range()),
                    None,
                )?;
                self.views.push(view);
                self.frames.push(
                    self.core.device.create_framebuffer(
                        &vk::FramebufferCreateInfo::default()
                            .render_pass(self.pass)
                            .attachments(&[view])
                            .width(extent.width)
                            .height(extent.height)
                            .layers(1),
                        None,
                    )?,
                );
                self.rendered.push(
                    self.core
                        .device
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?,
                );
            }
            self.dirty = false;
            Ok(())
        }
    }

    pub fn frame_ready(&self) -> Result<bool> {
        Ok(!self.submitted || unsafe { self.core.device.get_fence_status(self.fence)? })
    }

    pub fn wait_for_frame(&mut self) -> Result<()> {
        if self.submitted {
            unsafe {
                self.core
                    .device
                    .wait_for_fences(&[self.fence], true, u64::MAX)?;
            }
            self.submitted = false;
            #[cfg(feature = "perf")]
            {
                self.gpu_ns = self.timing.as_ref().map(GpuTiming::read).transpose()?;
            }
        }
        Ok(())
    }

    pub fn begin(&mut self, light: bool) -> Result<Option<RenderPass<'_>>> {
        unsafe {
            if self.dirty {
                self.recreate()?
            }
            let (index, suboptimal) = match self.swap.acquire_next_image(
                self.chain,
                u64::MAX,
                self.acquired,
                vk::Fence::null(),
            ) {
                Ok(v) => v,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.dirty = true;
                    return Ok(None);
                }
                Err(e) => return Err(anyhow::Error::new(e).context("acquire surface")),
            };
            self.dirty = suboptimal;
            let d = &self.core.device;
            d.reset_command_buffer(self.command, vk::CommandBufferResetFlags::empty())?;
            d.begin_command_buffer(
                self.command,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            let clear = if light { 1. } else { 0. };
            #[cfg(feature = "perf")]
            if let Some(timing) = &self.timing {
                d.cmd_reset_query_pool(self.command, timing.pool, 0, 2);
                d.cmd_write_timestamp(
                    self.command,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    timing.pool,
                    0,
                );
            }
            d.cmd_begin_render_pass(
                self.command,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(self.pass)
                    .framebuffer(self.frames[index as usize])
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D::default(),
                        extent: self.extent,
                    })
                    .clear_values(&[vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [clear, clear, clear, 1.],
                        },
                    }]),
                vk::SubpassContents::INLINE,
            );
            d.cmd_set_viewport(
                self.command,
                0,
                &[vk::Viewport {
                    x: 0.,
                    y: self.extent.height as f32,
                    width: self.extent.width as f32,
                    height: -(self.extent.height as f32),
                    min_depth: 0.,
                    max_depth: 1.,
                }],
            );
            d.cmd_set_scissor(
                self.command,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D::default(),
                    extent: self.extent,
                }],
            );
            Ok(Some(RenderPass {
                surface: self,
                index,
            }))
        }
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            let _ = self.core.device.device_wait_idle();
            self.clear_chain();
            self.core.device.destroy_render_pass(self.pass, None);
            self.core.device.destroy_fence(self.fence, None);
            self.core.device.destroy_semaphore(self.acquired, None);
            self.loader.destroy_surface(self.surface, None);
        }
    }
}
pub struct RenderPass<'a> {
    surface: &'a mut Surface,
    index: u32,
}
impl RenderPass<'_> {
    pub fn set_pipeline(&mut self, p: &RenderPipeline) {
        unsafe {
            self.surface.core.device.cmd_bind_pipeline(
                self.surface.command,
                vk::PipelineBindPoint::GRAPHICS,
                p.pipeline,
            )
        }
    }
    pub fn set_bind_group(&mut self, index: u32, group: &BindGroup) {
        unsafe {
            self.surface.core.device.cmd_bind_descriptor_sets(
                self.surface.command,
                vk::PipelineBindPoint::GRAPHICS,
                self.surface.core.pipeline_layout,
                index,
                &[group.set()],
                &[],
            )
        }
    }
    pub fn set_vertex_buffer(&mut self, index: u32, buffer: &Buffer) {
        unsafe {
            self.surface.core.device.cmd_bind_vertex_buffers(
                self.surface.command,
                index,
                &[buffer.0.buffer],
                &[0],
            )
        }
    }
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        unsafe {
            self.surface.core.device.cmd_set_scissor(
                self.surface.command,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D {
                        x: x as i32,
                        y: y as i32,
                    },
                    extent: vk::Extent2D {
                        width: w,
                        height: h,
                    },
                }],
            )
        }
    }
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        unsafe {
            self.surface.core.device.cmd_draw(
                self.surface.command,
                vertices.end - vertices.start,
                instances.end - instances.start,
                vertices.start,
                instances.start,
            )
        }
    }
    pub fn finish(&mut self) -> Result<()> {
        unsafe {
            let s = &mut self.surface;
            let d = &s.core.device;
            d.cmd_end_render_pass(s.command);
            #[cfg(feature = "perf")]
            if let Some(timing) = &s.timing {
                d.cmd_write_timestamp(
                    s.command,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    timing.pool,
                    1,
                );
            }
            d.end_command_buffer(s.command)?;
            d.reset_fences(&[s.fence])?;
            #[cfg(feature = "perf")]
            let submit_trace = ink_core::PerfTraceSection::new(b"Ink vkQueueSubmit\0");
            d.queue_submit(
                s.core.queue,
                &[vk::SubmitInfo::default()
                    .wait_semaphores(&[s.acquired])
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .command_buffers(&[s.command])
                    .signal_semaphores(&[s.rendered[self.index as usize]])],
                s.fence,
            )?;
            #[cfg(feature = "perf")]
            drop(submit_trace);
            s.submitted = true;
            let semaphores = [s.rendered[self.index as usize]];
            let chains = [s.chain];
            let indices = [self.index];
            let info = vk::PresentInfoKHR::default()
                .wait_semaphores(&semaphores)
                .swapchains(&chains)
                .image_indices(&indices);
            #[cfg(feature = "presentation-timing")]
            let times = {
                s.present_id = s.present_id.wrapping_add(1);
                [vk::PresentTimeGOOGLE {
                    present_id: s.present_id,
                    desired_present_time: 0,
                }]
            };
            #[cfg(feature = "presentation-timing")]
            let mut timing_info = vk::PresentTimesInfoGOOGLE::default().times(&times);
            #[cfg(feature = "presentation-timing")]
            let info = if s.display_timing.is_some() {
                info.push_next(&mut timing_info)
            } else {
                info
            };
            #[cfg(feature = "perf")]
            let present_trace = ink_core::PerfTraceSection::new(b"Ink vkQueuePresent\0");
            let result = s.swap.queue_present(s.core.queue, &info);
            #[cfg(feature = "perf")]
            drop(present_trace);
            match result {
                Ok(suboptimal) => {
                    s.dirty |= suboptimal;
                    Ok(())
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    s.dirty = true;
                    Ok(())
                }
                Err(e) => Err(anyhow::Error::new(e).context("present surface")),
            }
        }
    }
}
