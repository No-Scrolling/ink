//! Vulkan resources and presentation. Frames complete before shared resources are reused.
use anyhow::{Context, Result, anyhow, ensure};
use ash::{Entry, vk};

#[cfg(feature = "offscreen")]
mod offscreen;
use std::{
    ffi::c_void,
    ops::Range,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
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
const MAX_RETAINED_UPLOAD_BYTES: usize = 256 * 1024;
const MAX_RETAINED_STAGING_BYTES: usize = 8 * 1024 * 1024;
const STAGING_UNDERUSED_FRAMES: u8 = 32;
const STAGING_IDLE_RETENTION: Duration = Duration::from_secs(5);

#[cfg(feature = "perf")]
#[derive(Default)]
pub struct UploadPerf {
    pub staging_allocations: u64,
    /// Vulkan memory requirements of newly allocated staging buffers.
    pub staging_allocation_bytes: u64,
    /// Bytes used in staging buffers, including alignment padding.
    pub staging_upload_bytes: u64,
    pub staging_buffer_capacity_bytes: u64,
}

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
// Keep uploaded textures and staging memory alive until the frame fence signals.
struct UploadBatch {
    _staging: Option<Buffer>,
    _commands: Vec<Upload>,
}
#[derive(Default)]
struct UploadStaging {
    buffer: Option<Buffer>,
    previous_large: bool,
    underused_frames: u8,
    last_frame: Option<Instant>,
}
impl UploadStaging {
    fn prepare(
        &mut self,
        device: &Device,
        size: usize,
        #[cfg(feature = "perf")] perf: &mut UploadPerf,
    ) -> Option<Buffer> {
        let now = Instant::now();
        if self.last_frame.is_some_and(|last| now.duration_since(last) >= STAGING_IDLE_RETENTION) {
            // Expire on existing work; trimming must never schedule an idle frame.
            if self.buffer.as_ref().is_some_and(|buffer| buffer.0.size > MAX_RETAINED_UPLOAD_BYTES) {
                self.buffer = None;
            }
            self.previous_large = false;
            self.underused_frames = 0;
        }
        self.last_frame = Some(now);
        let underused = self.buffer.as_ref().is_some_and(|buffer| {
            buffer.0.size > MAX_RETAINED_UPLOAD_BYTES
                && (size <= buffer.0.size / 4 || size > buffer.0.size)
        });
        self.underused_frames = if underused { self.underused_frames + 1 } else { 0 };
        if self.underused_frames == STAGING_UNDERUSED_FRAMES {
            self.buffer = None;
            self.underused_frames = 0;
        }
        let repeated_large = self.previous_large;
        self.previous_large = size > MAX_RETAINED_UPLOAD_BYTES && size <= MAX_RETAINED_STAGING_BYTES;
        if size == 0 {
            return None;
        }
        if let Some(buffer) = self.buffer.as_ref().filter(|buffer| buffer.0.size >= size) {
            return Some(buffer.clone());
        }
        // Keep one slot. A one-off large upload stays transient; repeated demand can
        // replace the slot, with exact large capacity rather than power-of-two growth.
        let retain = size <= MAX_RETAINED_UPLOAD_BYTES
            || (repeated_large && size <= MAX_RETAINED_STAGING_BYTES);
        if retain {
            self.buffer = None;
        }
        let capacity = if size <= MAX_RETAINED_UPLOAD_BYTES { size.next_power_of_two() } else { size };
        let buffer = device.buffer(capacity, false);
        #[cfg(feature = "perf")]
        {
            perf.staging_allocations += 1;
            perf.staging_allocation_bytes += buffer.0.allocation_bytes;
        }
        // Memory requirements may exceed buffer capacity on some drivers.
        if retain && buffer.0.allocation_bytes <= MAX_RETAINED_STAGING_BYTES as u64 {
            self.buffer = Some(buffer.clone());
        }
        Some(buffer)
    }
}
enum Upload {
    Clear(Texture),
    Write {
        texture: Texture,
        source: UploadSource,
        origin: [u32; 2],
        size: [u32; 2],
    },
}
enum UploadSource {
    /// Offset into `Uploads::bytes`.
    Copied(usize),
    /// Shared pixels, copied straight into the staging buffer.
    Shared(Arc<[u8]>),
}
struct Core {
    _entry: VulkanEntry,
    instance: ash::Instance,
    device: ash::Device,
    physical: vk::PhysicalDevice,
    family: u32,
    queue: vk::Queue,
    memory: vk::PhysicalDeviceMemoryProperties,
    pipeline_cache: vk::PipelineCache,
    commands: vk::CommandPool,
    upload: vk::CommandBuffer,
    descriptors: vk::DescriptorPool,
    uniform_layout: vk::DescriptorSetLayout,
    texture_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    sampler: vk::Sampler,
}

struct VulkanEntry {
    entry: Entry,
    #[cfg(feature = "offscreen")]
    _library: Option<libloading::Library>,
}

impl From<Entry> for VulkanEntry {
    fn from(entry: Entry) -> Self {
        Self {
            entry,
            #[cfg(feature = "offscreen")]
            _library: None,
        }
    }
}
impl Drop for Core {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_pipeline_cache(self.pipeline_cache, None);
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
    fn record_uploads(&self, record: impl FnOnce(vk::CommandBuffer)) {
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
        }
    }
}
#[derive(Clone)]
pub struct Buffer(Arc<BufferInner>);
struct BufferInner {
    core: Arc<Core>,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    mapped: *mut u8,
    size: usize,
    allocation_bytes: u64,
    set: vk::DescriptorSet,
}
impl Drop for BufferInner {
    fn drop(&mut self) {
        unsafe {
            let d = &self.core.device;
            if self.set != vk::DescriptorSet::null() {
                let _ = d.free_descriptor_sets(self.core.descriptors, &[self.set]);
            }
            d.unmap_memory(self.memory);
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
#[cfg(feature = "offscreen")]
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
/// Splits copies into ordered steps whose regions don't overlap, so later writes still win.
fn copy_steps(regions: &[vk::BufferImageCopy]) -> Vec<Vec<vk::BufferImageCopy>> {
    let bounds = |region: &vk::BufferImageCopy| {
        let offset = region.image_offset;
        let extent = region.image_extent;
        [offset.x, offset.y, offset.x + extent.width as i32, offset.y + extent.height as i32]
    };
    let overlap = |first: [i32; 4], second: [i32; 4]| {
        first[0] < second[2] && second[0] < first[2] && first[1] < second[3] && second[1] < first[3]
    };
    // Sweep down the texture: only regions spanning the current top can overlap.
    let mut order = (0..regions.len()).collect::<Vec<_>>();
    order.sort_unstable_by_key(|&index| bounds(&regions[index])[1]);
    let mut active = Vec::<[i32; 4]>::new();
    let disjoint = order.iter().all(|&index| {
        let next = bounds(&regions[index]);
        active.retain(|region| region[3] > next[1]);
        let clear = active.iter().all(|&region| !overlap(region, next));
        active.push(next);
        clear
    });
    if regions.is_empty() {
        return Vec::new();
    }
    if disjoint {
        return vec![regions.to_vec()];
    }
    let mut steps: Vec<Vec<vk::BufferImageCopy>> = Vec::new();
    for region in regions {
        let next = bounds(region);
        match steps.last_mut() {
            Some(step) if step.iter().all(|other| !overlap(bounds(other), next)) => step.push(*region),
            _ => steps.push(vec![*region]),
        }
    }
    steps
}
fn check_write(texture: &Texture, origin: [u32; 2], size: [u32; 2], bytes: &[u8]) {
    assert!(origin[0] + size[0] <= texture.0.width && origin[1] + size[1] <= texture.0.height);
    assert_eq!(
        bytes.len(),
        size[0] as usize * size[1] as usize * texture.0.channels
    );
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
                mapped: d.map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                    .expect("map buffer").cast(),
                size,
                allocation_bytes: req.size,
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
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(), buffer.0.mapped.add(offset as usize), bytes.len(),
            );
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
                        // Glyph coverage becomes alpha in the shared textured shader.
                        .components(if mask {
                            vk::ComponentMapping {
                                r: vk::ComponentSwizzle::ONE,
                                g: vk::ComponentSwizzle::ONE,
                                b: vk::ComponentSwizzle::ONE,
                                a: vk::ComponentSwizzle::R,
                            }
                        } else {
                            vk::ComponentMapping::default()
                        })
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
        check_write(texture, origin, size, bytes);
        let mut uploads = self.1.lock().unwrap();
        let offset = uploads.bytes.len().next_multiple_of(4);
        uploads.bytes.resize(offset, 0);
        uploads.bytes.extend_from_slice(bytes);
        uploads.commands.push(Upload::Write {
            texture: texture.clone(),
            source: UploadSource::Copied(offset),
            origin,
            size,
        });
    }

    /// Like `write_texture`, but keeps `pixels` until upload instead of copying them now.
    pub fn write_texture_shared(&self, texture: &Texture, origin: [u32; 2], size: [u32; 2], pixels: Arc<[u8]>) {
        check_write(texture, origin, size, &pixels);
        self.1.lock().unwrap().commands.push(Upload::Write {
            texture: texture.clone(),
            source: UploadSource::Shared(pixels),
            origin,
            size,
        });
    }

    /// Clears `texture` to zero on the GPU, replacing its pending uploads.
    pub fn clear_texture(&self, texture: &Texture) {
        self.discard_texture_uploads(&[texture]);
        self.1.lock().unwrap().commands.push(Upload::Clear(texture.clone()));
    }

    pub fn discard_texture_uploads(&self, textures: &[&Texture]) {
        if textures.is_empty() {
            return;
        }
        let mut uploads = self.1.lock().unwrap();
        let before = uploads.commands.len();
        uploads.commands.retain(|command| {
            let texture = match command {
                Upload::Clear(texture) | Upload::Write { texture, .. } => texture,
            };
            !textures
                .iter()
                .any(|discarded| Arc::ptr_eq(&texture.0, &discarded.0))
        });
        if uploads.commands.len() == before {
            return;
        }
        let Uploads { bytes, commands } = &mut *uploads;
        let previous = std::mem::take(bytes);
        for command in commands {
            if let Upload::Write {
                texture,
                source: UploadSource::Copied(offset),
                size,
                ..
            } = command
            {
                let length = size[0] as usize * size[1] as usize * texture.0.channels;
                let next = bytes.len().next_multiple_of(4);
                bytes.resize(next, 0);
                bytes.extend_from_slice(&previous[*offset..*offset + length]);
                *offset = next;
            }
        }
    }

    fn prepare_uploads(
        &self,
        reusable: &mut UploadStaging,
        #[cfg(feature = "perf")] perf: &mut UploadPerf,
    ) -> Option<UploadBatch> {
        let mut pending = self.1.lock().unwrap();
        if pending.commands.is_empty() {
            drop(pending);
            let _ = reusable.prepare(
                self,
                0,
                #[cfg(feature = "perf")]
                perf,
            );
            return None;
        }
        let mut uploads = std::mem::take(&mut *pending);
        drop(pending);
        // Shared pixels follow the copied bytes in the staging buffer.
        let mut size = uploads.bytes.len();
        let mut shared_offsets = Vec::new();
        for upload in &uploads.commands {
            if let Upload::Write { source: UploadSource::Shared(pixels), .. } = upload {
                size = size.next_multiple_of(4);
                shared_offsets.push(size);
                size += pixels.len();
            }
        }
        let staging = reusable.prepare(
            self,
            size,
            #[cfg(feature = "perf")]
            perf,
        );
        if let Some(buffer) = &staging {
            #[cfg(feature = "perf")]
            {
                perf.staging_upload_bytes += size as u64;
                perf.staging_buffer_capacity_bytes = perf.staging_buffer_capacity_bytes.max(buffer.0.size as u64);
            }
            self.write_buffer(buffer, 0, &uploads.bytes);
            let mut offsets = shared_offsets.iter();
            for upload in &uploads.commands {
                if let Upload::Write { source: UploadSource::Shared(pixels), .. } = upload {
                    self.write_buffer(buffer, *offsets.next().unwrap() as u64, pixels);
                }
            }
        }
        // Group work by texture. A clear supersedes earlier operations on its texture, and
        // later copies run as one command unless regions overlap, which needs ordered steps.
        struct Steps {
            image: vk::Image,
            cleared: bool,
            regions: Vec<vk::BufferImageCopy>,
        }
        let mut textures: Vec<Steps> = Vec::new();
        let mut shared_offsets = shared_offsets.into_iter();
        for upload in &uploads.commands {
            let texture = match upload {
                Upload::Clear(texture) | Upload::Write { texture, .. } => texture,
            };
            let index = match textures.iter().position(|entry| entry.image == texture.0.image) {
                Some(index) => index,
                None => {
                    textures.push(Steps { image: texture.0.image, cleared: false, regions: Vec::new() });
                    textures.len() - 1
                }
            };
            let entry = &mut textures[index];
            match upload {
                Upload::Clear(_) => {
                    entry.cleared = true;
                    entry.regions.clear();
                }
                &Upload::Write { ref source, origin, size, .. } => {
                    let offset = match source {
                        UploadSource::Copied(offset) => *offset,
                        UploadSource::Shared(_) => shared_offsets.next().unwrap(),
                    };
                    entry.regions.push(
                        vk::BufferImageCopy::default()
                            .buffer_offset(offset as u64)
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
                            }),
                    );
                }
            }
        }
        // Each texture runs its clear, then its copy steps; rounds share transfer barriers.
        let steps = textures
            .iter()
            .map(|entry| copy_steps(&entry.regions))
            .collect::<Vec<_>>();
        let d = &self.0.device;
        let source = staging.as_ref().map(|buffer| buffer.0.buffer);
        self.0.record_uploads(|cmd| unsafe {
            let transitions = |entries: &mut dyn Iterator<Item = (vk::Image, vk::ImageLayout, vk::ImageLayout, vk::AccessFlags, vk::AccessFlags)>| {
                entries
                    .map(|(image, old, new, src, dst)| {
                        vk::ImageMemoryBarrier::default()
                            .old_layout(old)
                            .new_layout(new)
                            .src_access_mask(src)
                            .dst_access_mask(dst)
                            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                            .image(image)
                            .subresource_range(colour_range())
                    })
                    .collect::<Vec<_>>()
            };
            // Earlier frames only sampled these textures in fragment shaders.
            d.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &transitions(&mut textures.iter().map(|entry| {
                    if entry.cleared {
                        (entry.image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::AccessFlags::empty(), vk::AccessFlags::TRANSFER_WRITE)
                    } else {
                        (entry.image, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::AccessFlags::SHADER_READ, vk::AccessFlags::TRANSFER_WRITE)
                    }
                })),
            );
            let rounds = textures
                .iter()
                .zip(&steps)
                .map(|(entry, steps)| usize::from(entry.cleared) + steps.len())
                .max()
                .unwrap_or(0);
            for round in 0..rounds {
                if round > 0 {
                    d.cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[vk::MemoryBarrier::default()
                            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)],
                        &[],
                        &[],
                    );
                }
                for (entry, steps) in textures.iter().zip(&steps) {
                    let step = if entry.cleared { round.checked_sub(1) } else { Some(round) };
                    match step {
                        None => d.cmd_clear_color_image(
                            cmd,
                            entry.image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            &vk::ClearColorValue { float32: [0.; 4] },
                            &[colour_range()],
                        ),
                        Some(step) if step < steps.len() => d.cmd_copy_buffer_to_image(
                            cmd,
                            source.expect("upload bytes have a staging buffer"),
                            entry.image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            &steps[step],
                        ),
                        Some(_) => {}
                    }
                }
            }
            d.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &transitions(&mut textures.iter().map(|entry| {
                    (entry.image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::AccessFlags::TRANSFER_WRITE, vk::AccessFlags::SHADER_READ)
                })),
            );
        });
        if uploads.bytes.capacity() <= MAX_RETAINED_UPLOAD_BYTES {
            uploads.bytes.clear();
            self.1.lock().unwrap().bytes = uploads.bytes;
        }
        Some(UploadBatch {
            _staging: staging,
            _commands: uploads.commands,
        })
    }
    /// Serialised pipeline cache, for reuse by a later process.
    pub fn pipeline_cache_data(&self) -> Result<Vec<u8>> {
        Ok(unsafe { self.0.device.get_pipeline_cache_data(self.0.pipeline_cache)? })
    }
    pub fn pipeline(&self, format: TextureFormat, text: bool) -> Result<RenderPipeline> {
        unsafe {
            let d = &self.0.device;
            let (vs, fs, vname, fname) = if !text {
                (
                    include_bytes!(concat!(env!("OUT_DIR"), "/quad_vertex.spv")).as_slice(),
                    include_bytes!(concat!(env!("OUT_DIR"), "/quad_fragment.spv")).as_slice(),
                    c"quad_vertex",
                    c"quad_fragment",
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
                self.0.pipeline_cache,
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
    render_pass_layout(d, format, vk::ImageLayout::PRESENT_SRC_KHR)
}

fn render_pass_layout(
    d: &ash::Device,
    format: vk::Format,
    final_layout: vk::ImageLayout,
) -> Result<vk::RenderPass> {
    unsafe {
        let attachments = [vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(final_layout)];
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
    #[cfg(feature = "offscreen")]
    target: Option<offscreen::Target>,
    core: Arc<Core>,
    loader: ash::khr::surface::Instance,
    android: ash::khr::android_surface::Instance,
    swap: ash::khr::swapchain::Device,
    surface: vk::SurfaceKHR,
    /// Renders into an offscreen image rather than an Android window.
    windowless: bool,
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
    uploads: Option<UploadBatch>,
    upload_staging: UploadStaging,
    dirty: bool,
    requested: [u32; 2],
    #[cfg(feature = "perf")]
    timing: Option<GpuTiming>,
    #[cfg(feature = "perf")]
    pub gpu_ns: Option<u64>,
    #[cfg(feature = "perf")]
    pub upload_perf: UploadPerf,
    #[cfg(feature = "presentation-timing")]
    display_timing: Option<ash::google::display_timing::Device>,
    #[cfg(feature = "presentation-timing")]
    pub present_id: u32,
}
/// What a surface presents to when it is created.
#[derive(Clone, Copy)]
pub(crate) enum Presentation {
    Window(*mut c_void),
    #[cfg_attr(not(feature = "offscreen"), allow(dead_code))]
    Offscreen,
}

/// Preferred swapchain format; reattached windows must support the format the render pass was built for.
const PRESENT_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

impl Surface {
    pub unsafe fn new(window: *mut c_void, width: u32, height: u32, pipeline_cache: &[u8]) -> Result<Self> {
        unsafe {
            Self::create(
                Entry::load().context("load Vulkan")?.into(),
                Presentation::Window(window),
                width,
                height,
                pipeline_cache,
            )
        }
    }

    pub fn attached(&self) -> bool {
        self.windowless || self.surface != vk::SurfaceKHR::null()
    }

    /// Present to `window`, keeping the device, render pass and uploaded resources.
    ///
    /// # Safety
    /// `window` must point to a live `ANativeWindow` retained until [`Self::detach`] or drop.
    pub unsafe fn attach(&mut self, window: *mut c_void, width: u32, height: u32) -> Result<()> {
        ensure!(!self.windowless && !self.attached(), "Vulkan surface is already attached");
        unsafe {
            let surface = self.android.create_android_surface(
                &vk::AndroidSurfaceCreateInfoKHR::default().window(window.cast()),
                None,
            )?;
            let compatible = (|| -> Result<()> {
                ensure!(
                    self.loader.get_physical_device_surface_support(self.core.physical, self.core.family, surface)?,
                    "Vulkan queue cannot present to this window"
                );
                ensure!(
                    self.loader.get_physical_device_surface_formats(self.core.physical, surface)?.iter()
                        .any(|f| f.format == self.format && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR),
                    "window does not support the prepared sRGB format"
                );
                Ok(())
            })();
            if let Err(error) = compatible {
                self.loader.destroy_surface(surface, None);
                return Err(error);
            }
            self.surface = surface;
            self.requested = [width, height];
            self.dirty = true;
            if let Err(error) = self.recreate() {
                self.detach();
                return Err(error);
            }
            Ok(())
        }
    }

    /// Release the window and swapchain. The device and its resources remain for the next attach.
    pub fn detach(&mut self) {
        if self.windowless || self.surface == vk::SurfaceKHR::null() {
            return;
        }
        unsafe {
            let _ = self.core.device.device_wait_idle();
            self.clear_chain();
            self.loader.destroy_surface(self.surface, None);
        }
        self.surface = vk::SurfaceKHR::null();
        self.submitted = false;
        self.uploads = None;
        self.upload_staging = UploadStaging::default();
        self.dirty = true;
    }

    unsafe fn create(
        runtime: VulkanEntry,
        target: Presentation,
        width: u32,
        height: u32,
        pipeline_cache: &[u8],
    ) -> Result<Self> {
        let window = match target {
            Presentation::Window(window) => Some(window),
            Presentation::Offscreen => None,
        };
        let presentable = !matches!(target, Presentation::Offscreen);
        unsafe {
            let entry = &runtime.entry;
            let app = vk::ApplicationInfo::default()
                .application_name(c"Ink direct Vulkan")
                .api_version(vk::API_VERSION_1_1);
            let mut extensions = if presentable {
                vec![
                    ash::khr::surface::NAME.as_ptr(),
                    ash::khr::android_surface::NAME.as_ptr(),
                ]
            } else {
                vec![]
            };
            let portability = !presentable
                && entry
                    .enumerate_instance_extension_properties(None)?
                    .iter()
                    .any(|extension| {
                        extension.extension_name_as_c_str().ok()
                            == Some(ash::khr::portability_enumeration::NAME)
                    });
            if portability {
                extensions.push(ash::khr::portability_enumeration::NAME.as_ptr());
            }
            let instance = entry.create_instance(
                &vk::InstanceCreateInfo::default()
                    .application_info(&app)
                    .flags(if portability {
                        vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
                    } else {
                        vk::InstanceCreateFlags::empty()
                    })
                    .enabled_extension_names(&extensions),
                None,
            )?;
            let cleanup_instance = Cleanup(Some(|| instance.destroy_instance(None)));
            let loader = ash::khr::surface::Instance::new(entry, &instance);
            let android = ash::khr::android_surface::Instance::new(entry, &instance);
            let surface = if let Some(window) = window {
                android.create_android_surface(
                    &vk::AndroidSurfaceCreateInfoKHR::default().window(window.cast()),
                    None,
                )?
            } else {
                vk::SurfaceKHR::null()
            };
            let cleanup_surface = Cleanup(Some(|| {
                if surface != vk::SurfaceKHR::null() {
                    loader.destroy_surface(surface, None);
                }
            }));
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
                        && (surface == vk::SurfaceKHR::null()
                            || loader
                                .get_physical_device_surface_support(physical, i as u32, surface)?)
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
            let mut extensions = if presentable {
                vec![ash::khr::swapchain::NAME.as_ptr()]
            } else {
                vec![]
            };
            if instance
                .enumerate_device_extension_properties(physical)?
                .iter()
                .any(|extension| {
                    extension.extension_name_as_c_str().ok()
                        == Some(ash::khr::portability_subset::NAME)
                })
            {
                extensions.push(ash::khr::portability_subset::NAME.as_ptr());
            }
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
            // Drivers ignore incompatible cache data and start empty.
            let cache = device.create_pipeline_cache(
                &vk::PipelineCacheCreateInfo::default().initial_data(pipeline_cache),
                None,
            )?;
            let cleanup_cache = Cleanup(Some(|| device.destroy_pipeline_cache(cache, None)));
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
            let format = if window.is_some() {
                let formats = loader.get_physical_device_surface_formats(physical, surface)?;
                [PRESENT_FORMAT, vk::Format::B8G8R8A8_SRGB]
                    .into_iter()
                    .find(|format| formats.iter().any(|f| f.format == *format))
                    .context("surface needs an sRGB format")?
            } else {
                PRESENT_FORMAT
            };
            let pass = render_pass_layout(
                &device,
                format,
                if presentable {
                    vk::ImageLayout::PRESENT_SRC_KHR
                } else {
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL
                },
            )?;
            let cleanup_pass = Cleanup(Some(|| device.destroy_render_pass(pass, None)));
            cleanup_pass.disarm();
            cleanup_fence.disarm();
            cleanup_acquired.disarm();
            cleanup_sampler.disarm();
            cleanup_cache.disarm();
            cleanup_descriptors.disarm();
            cleanup_pipeline_layout.disarm();
            cleanup_texture_layout.disarm();
            cleanup_uniform_layout.disarm();
            cleanup_commands.disarm();
            cleanup_device.disarm();
            cleanup_surface.disarm();
            cleanup_instance.disarm();
            let core = Arc::new(Core {
                _entry: runtime,
                instance,
                device,
                physical,
                family,
                queue,
                memory,
                pipeline_cache: cache,
                commands,
                upload: buffers[0],
                descriptors,
                uniform_layout,
                texture_layout,
                pipeline_layout,
                sampler,
            });
            let mut s = Self {
                #[cfg(feature = "offscreen")]
                target: None,
                core,
                loader,
                android,
                swap,
                surface,
                windowless: !presentable,
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
                uploads: None,
                upload_staging: UploadStaging::default(),
                dirty: true,
                requested: [width, height],
                #[cfg(feature = "perf")]
                timing: None,
                #[cfg(feature = "perf")]
                gpu_ns: None,
                #[cfg(feature = "perf")]
                upload_perf: UploadPerf::default(),
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
            if self.chain != vk::SwapchainKHR::null() {
                self.swap.destroy_swapchain(self.chain, None);
            }
            self.chain = vk::SwapchainKHR::null();
        }
    }
    fn recreate(&mut self) -> Result<()> {
        if self.surface == vk::SurfaceKHR::null() {
            #[cfg(feature = "offscreen")]
            if self.windowless {
                return self.recreate_offscreen();
            }
            anyhow::bail!("Vulkan surface is detached");
        }
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

    #[cfg(feature = "perf")]
    /// Cached and in-flight capacities can describe the same allocation.
    pub fn staging_memory(&self) -> (usize, u64, usize) {
        (
            self.upload_staging.buffer.as_ref().map_or(0, |buffer| buffer.0.size),
            self.upload_staging.buffer.as_ref().map_or(0, |buffer| buffer.0.allocation_bytes),
            self.uploads.as_ref().and_then(|batch| batch._staging.as_ref()).map_or(0, |buffer| buffer.0.size),
        )
    }

    pub fn wait_for_frame(&mut self) -> Result<()> {
        if self.submitted {
            unsafe {
                self.core
                    .device
                    .wait_for_fences(&[self.fence], true, u64::MAX)?;
            }
            self.submitted = false;
            self.uploads = None;
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
            let (index, suboptimal) = if self.surface == vk::SurfaceKHR::null() {
                (0, false)
            } else {
                match self.swap.acquire_next_image(
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
                }
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
            if self.surface != vk::SurfaceKHR::null() {
                self.loader.destroy_surface(self.surface, None);
            }
        }
    }
}
pub struct RenderPass<'a> {
    surface: &'a mut Surface,
    index: u32,
}
impl RenderPass<'_> {
    pub fn upload(&mut self, queue: &Queue) {
        self.surface.uploads = queue.prepare_uploads(
            &mut self.surface.upload_staging,
            #[cfg(feature = "perf")]
            &mut self.surface.upload_perf,
        );
    }
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
    pub fn finish(&mut self) -> Result<bool> {
        unsafe {
            let s = &mut self.surface;
            let d = &s.core.device;
            d.cmd_end_render_pass(s.command);
            #[cfg(feature = "offscreen")]
            if let Some(target) = &s.target {
                target.copy(d, s.command, s.extent);
            }
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
            let commands = [s.core.upload, s.command];
            let commands = if s.uploads.is_some() {
                &commands[..]
            } else {
                &commands[1..]
            };
            if s.surface == vk::SurfaceKHR::null() {
                d.queue_submit(
                    s.core.queue,
                    &[vk::SubmitInfo::default().command_buffers(commands)],
                    s.fence,
                )?;
                s.submitted = true;
                return Ok(true);
            }
            d.queue_submit(
                s.core.queue,
                &[vk::SubmitInfo::default()
                    .wait_semaphores(&[s.acquired])
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .command_buffers(commands)
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
                    Ok(!suboptimal)
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    s.dirty = true;
                    Ok(false)
                }
                Err(e) => Err(anyhow::Error::new(e).context("present surface")),
            }
        }
    }
}
