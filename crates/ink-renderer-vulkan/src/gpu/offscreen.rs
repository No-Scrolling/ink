use super::*;
use std::ffi::CStr;

pub(super) struct Target {
    core: Arc<Core>,
    image: vk::Image,
    memory: vk::DeviceMemory,
    buffer: vk::Buffer,
    buffer_memory: vk::DeviceMemory,
    mapped: *const u8,
    length: usize,
}

impl Target {
    pub(super) fn copy(
        &self,
        device: &ash::Device,
        command: vk::CommandBuffer,
        extent: vk::Extent2D,
    ) {
        unsafe {
            barrier(
                device,
                command,
                self.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::AccessFlags::TRANSFER_READ,
            );
            device.cmd_copy_image_to_buffer(
                command,
                self.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                self.buffer,
                &[vk::BufferImageCopy::default()
                    .image_subresource(
                        vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1),
                    )
                    .image_extent(vk::Extent3D {
                        width: extent.width,
                        height: extent.height,
                        depth: 1,
                    })],
            );
            device.cmd_pipeline_barrier(
                command,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[],
                &[vk::BufferMemoryBarrier::default()
                    .buffer(self.buffer)
                    .size(vk::WHOLE_SIZE)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::HOST_READ)],
                &[],
            );
        }
    }
}

impl Drop for Target {
    fn drop(&mut self) {
        unsafe {
            let device = &self.core.device;
            device.unmap_memory(self.buffer_memory);
            device.destroy_buffer(self.buffer, None);
            device.free_memory(self.buffer_memory, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}

impl Surface {
    pub fn offscreen(width: u32, height: u32) -> Result<Self> {
        ensure!(width > 0 && height > 0, "zero-sized offscreen frame");
        unsafe { Self::create(load_entry()?, None, width, height) }
    }

    pub fn device_name(&self) -> String {
        unsafe {
            let mut driver = vk::PhysicalDeviceDriverProperties::default();
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut driver);
            self.core
                .instance
                .get_physical_device_properties2(self.core.physical, &mut properties);
            let name = CStr::from_ptr(properties.properties.device_name.as_ptr()).to_string_lossy();
            let version = CStr::from_ptr(driver.driver_info.as_ptr()).to_string_lossy();
            format!("{name} · {version}")
        }
    }

    pub fn pixels(&mut self) -> Result<Vec<u8>> {
        self.wait_for_frame()?;
        let target = self
            .target
            .as_ref()
            .context("pixel readback requires an offscreen renderer")?;
        Ok(unsafe { std::slice::from_raw_parts(target.mapped, target.length) }.to_vec())
    }

    pub(super) fn recreate_offscreen(&mut self) -> Result<()> {
        unsafe {
            self.core.device.device_wait_idle()?;
            self.clear_chain();
            self.target = None;
            let device = &self.core.device;
            let extent = vk::Extent2D {
                width: self.requested[0],
                height: self.requested[1],
            };
            ensure!(
                extent.width > 0 && extent.height > 0,
                "zero-sized offscreen frame"
            );
            let image = device.create_image(
                &vk::ImageCreateInfo::default()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(self.format)
                    .extent(vk::Extent3D {
                        width: extent.width,
                        height: extent.height,
                        depth: 1,
                    })
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(
                        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
                    ),
                None,
            )?;
            let cleanup_image = Cleanup(Some(|| device.destroy_image(image, None)));
            let requirements = device.get_image_memory_requirements(image);
            let memory = device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(self.core.memory_type(
                        requirements.memory_type_bits,
                        vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    )),
                None,
            )?;
            let cleanup_memory = Cleanup(Some(|| device.free_memory(memory, None)));
            device.bind_image_memory(image, memory, 0)?;
            let view = device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(self.format)
                    .subresource_range(colour_range()),
                None,
            )?;
            self.views.push(view);
            self.frames.push(
                device.create_framebuffer(
                    &vk::FramebufferCreateInfo::default()
                        .render_pass(self.pass)
                        .attachments(&[view])
                        .width(extent.width)
                        .height(extent.height)
                        .layers(1),
                    None,
                )?,
            );
            let length = extent.width as usize * extent.height as usize * 4;
            let buffer = device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(length as u64)
                    .usage(vk::BufferUsageFlags::TRANSFER_DST),
                None,
            )?;
            let cleanup_buffer = Cleanup(Some(|| device.destroy_buffer(buffer, None)));
            let requirements = device.get_buffer_memory_requirements(buffer);
            let buffer_memory = device.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(self.core.memory_type(
                        requirements.memory_type_bits,
                        vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    )),
                None,
            )?;
            let cleanup_buffer_memory = Cleanup(Some(|| device.free_memory(buffer_memory, None)));
            device.bind_buffer_memory(buffer, buffer_memory, 0)?;
            let mapped = device
                .map_memory(
                    buffer_memory,
                    0,
                    vk::WHOLE_SIZE,
                    vk::MemoryMapFlags::empty(),
                )?
                .cast();
            self.target = Some(Target {
                core: self.core.clone(),
                image,
                memory,
                buffer,
                buffer_memory,
                mapped,
                length,
            });
            cleanup_buffer_memory.disarm();
            cleanup_buffer.disarm();
            cleanup_memory.disarm();
            cleanup_image.disarm();
            self.extent = extent;
            self.dirty = false;
            Ok(())
        }
    }
}

fn load_entry() -> Result<VulkanEntry> {
    unsafe {
        if let Some(path) = std::env::var_os("INK_VULKAN_LIBRARY") {
            return load_library(&path).context("load INK_VULKAN_LIBRARY");
        }
        let directory = std::env::var_os("INK_DESIGN_RENDERER_DIR")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                let cache = std::env::var_os("XDG_CACHE_HOME")
                    .map(std::path::PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(|home| std::path::PathBuf::from(home).join(".cache"))
                    })?;
                Some(cache.join("ink/design-renderer/mesa-3fc652a060"))
            })
            .context("set INK_DESIGN_RENDERER_DIR to the standalone design runtime directory")?;
        let library = if cfg!(target_os = "macos") {
            "libvulkan_lvp.dylib"
        } else if cfg!(target_os = "windows") {
            "vulkan_lvp.dll"
        } else {
            "libvulkan_lvp.so"
        };
        load_library(directory.join("lib").join(library).as_os_str())
            .context("run scripts/setup-design-renderer once to build the standalone Mesa runtime, or set INK_VULKAN_LIBRARY to a desktop Vulkan library")
    }
}

unsafe fn load_library(path: &std::ffi::OsStr) -> Result<VulkanEntry> {
    if let Ok(entry) = unsafe { Entry::load_from(path) } {
        return Ok(entry.into());
    }
    let library = unsafe { libloading::Library::new(path) }?;
    let get_instance_proc_addr =
        *unsafe { library.get::<vk::PFN_vkGetInstanceProcAddr>(b"vk_icdGetInstanceProcAddr\0") }?;
    let entry = unsafe {
        Entry::from_static_fn(ash::StaticFn {
            get_instance_proc_addr,
        })
    };
    Ok(VulkanEntry {
        entry,
        _library: Some(library),
    })
}
