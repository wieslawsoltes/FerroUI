use crate::interop::VulkanMemoryHelper;
use crate::{IVulkanDevice, IVulkanPlatformGraphicsContext, VulkanError, VulkanImageInfo, VulkanResultExtensions};
use ash::vk;
use ferroui_base::PixelSize;
use std::sync::Arc;

/// An image of the device of a context and its memory, for a renderer that draws on it from several
/// threads and keeps its own record of the image's layout (Skia's Graphite: the texture of a
/// pipelined frame layer, which the threads of its bands insert their recordings onto).
///
/// The image is made in the layout `UNDEFINED`, with no command and no wait, single-sampled, with
/// one mip level and optimal tiling. It has no view: the renderer makes the views it needs. The
/// value may move to and be dropped on any thread; dropping it destroys the image and frees its
/// memory, so the owner drops it only after the GPU has finished every use of the image, and
/// before the device is disposed.
pub struct VulkanOffscreenImage {
    device: Arc<dyn IVulkanDevice>,
    device_handle: vk::Device,
    destroy_image: vk::PFN_vkDestroyImage,
    free_memory: vk::PFN_vkFreeMemory,
    handle: vk::Image,
    memory: vk::DeviceMemory,
    memory_size: u64,
    size: PixelSize,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
}

impl VulkanOffscreenImage {
    /// The usage every offscreen image has: drawn to, sampled, and copied from and to.
    pub const DEFAULT_USAGE: u32 = vk::ImageUsageFlags::COLOR_ATTACHMENT.as_raw()
        | vk::ImageUsageFlags::SAMPLED.as_raw()
        | vk::ImageUsageFlags::TRANSFER_SRC.as_raw()
        | vk::ImageUsageFlags::TRANSFER_DST.as_raw();

    /// Makes an image of `size` pixels in `format` (a `VkFormat`) with [`DEFAULT_USAGE`] and
    /// `extra_usage` (`VkImageUsageFlags`), in device-local memory.
    ///
    /// [`DEFAULT_USAGE`]: Self::DEFAULT_USAGE
    pub fn new(
        context: &dyn IVulkanPlatformGraphicsContext,
        size: PixelSize,
        format: u32,
        extra_usage: u32,
    ) -> Result<Self, VulkanError> {
        if size.width <= 0 || size.height <= 0 {
            return Err(VulkanError::new(format!("An offscreen image cannot have the size {size}")));
        }
        let api = context.device_api();
        let device = context.device_handle();
        let format = vk::Format::from_raw(format as i32);
        let usage = vk::ImageUsageFlags::from_raw(Self::DEFAULT_USAGE | extra_usage);

        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D { depth: 1, width: size.width as u32, height: size.height as u32 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let handle = api.create_image(device, &create_info).throw_on_error("vkCreateImage")?;

        let requirements = api.get_image_memory_requirements(device, handle);
        let memory_type = VulkanMemoryHelper::find_suitable_memory_type_index(
            context,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );
        if memory_type < 0 {
            api.destroy_image(device, handle);
            return Err(VulkanError::new("No device-local memory type for an offscreen image"));
        }
        let allocate_info =
            vk::MemoryAllocateInfo::default().allocation_size(requirements.size).memory_type_index(memory_type as u32);
        let memory = match api.allocate_memory(device, &allocate_info).throw_on_error("vkAllocateMemory") {
            Ok(memory) => memory,
            Err(error) => {
                api.destroy_image(device, handle);
                return Err(error);
            }
        };
        if let Err(error) = api.bind_image_memory(device, handle, memory, 0).throw_on_error("vkBindImageMemory") {
            api.destroy_image(device, handle);
            api.free_memory(device, memory);
            return Err(error);
        }

        let (destroy_image, free_memory) = api.destroy_entry_points();
        Ok(Self {
            device: context.device(),
            device_handle: device,
            destroy_image,
            free_memory,
            handle,
            memory,
            memory_size: requirements.size,
            size,
            format,
            usage,
        })
    }

    /// The image: its handles, its size, format, tiling and usage, and the layout `UNDEFINED` it
    /// was made in (the renderer keeps the record of the layout from then on). It has no view.
    pub fn image_info(&self) -> VulkanImageInfo {
        VulkanImageInfo {
            handle: vk::Handle::as_raw(self.handle),
            pixel_size: self.size,
            format: self.format.as_raw() as u32,
            memory_handle: vk::Handle::as_raw(self.memory),
            memory_size: self.memory_size,
            view_handle: 0,
            usage_flags: self.usage.as_raw(),
            layout: vk::ImageLayout::UNDEFINED.as_raw() as u32,
            tiling: vk::ImageTiling::OPTIMAL.as_raw() as u32,
            level_count: 1,
            sample_count: 1,
            is_protected: false,
        }
    }

    /// The device the image belongs to.
    pub fn device(&self) -> &Arc<dyn IVulkanDevice> {
        &self.device
    }
}

impl Drop for VulkanOffscreenImage {
    fn drop(&mut self) {
        // SAFETY: the image and the memory were made by this value on `device_handle`, are given up
        // exactly once, here, and the owner's contract is that the GPU no longer uses them. The two
        // functions need external synchronization of the image and the memory only, which this value
        // owns.
        unsafe {
            (self.destroy_image)(self.device_handle, self.handle, std::ptr::null());
            (self.free_memory)(self.device_handle, self.memory, std::ptr::null());
        }
    }
}

// SAFETY: the value holds handles of Vulkan objects it owns, two entry points (plain function
// pointers) and the device, which is `Send + Sync`. Nothing in it changes after it was made, and
// destroying the image and freeing its memory may happen on any thread.
unsafe impl Send for VulkanOffscreenImage {}
// SAFETY: as above; every method takes `&self` and reads values only.
unsafe impl Sync for VulkanOffscreenImage {}
