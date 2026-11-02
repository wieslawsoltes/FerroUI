use super::{VulkanCommandBufferPool, VulkanMemoryHelper};
use crate::{
    IVulkanPlatformGraphicsContext, VulkanError, VulkanExceptionExtensions, VulkanImageInfo,
    VulkanResultExtensions,
};
use ash::vk;
use ferroui_base::PixelSize;
use std::cell::Cell;
use std::rc::Rc;

/// How the memory of an image is obtained: the image, the size and the
/// allowed memory types the device asks for.
pub(crate) type CreateMemory<'a> = &'a dyn Fn(vk::Image, u64, u32) -> Result<vk::DeviceMemory, VulkanError>;

/// An image of the device of a context with its memory and its view.
pub(crate) struct VulkanImageBase {
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    current_access_flags: Cell<vk::AccessFlags>,
    usage_flags: vk::ImageUsageFlags,
    image_view: Cell<vk::ImageView>,
    image_memory: Cell<vk::DeviceMemory>,
    handle: Cell<vk::Image>,
    command_buffer_pool: Rc<VulkanCommandBufferPool>,
    format: vk::Format,
    aspect_flags: Cell<vk::ImageAspectFlags>,
    mip_levels: Cell<u32>,
    size: PixelSize,
    memory_size: Cell<u64>,
    current_layout: Cell<vk::ImageLayout>,
}

/// Memory to import for an image.
#[allow(dead_code)] // Declared by the reference, which does not use it either.
pub(crate) struct MemoryImportInfo {
    pub next: isize,
    pub memory_size: u64,
    pub memory_offset: u64,
}

impl VulkanImageBase {
    /// Describes an image; [`initialize`](Self::initialize) creates it.
    /// With `mip_levels` zero the image gets the levels of its size.
    pub fn new(
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
        command_buffer_pool: Rc<VulkanCommandBufferPool>,
        format: vk::Format,
        size: PixelSize,
        mip_levels: u32,
    ) -> Self {
        Self {
            context,
            current_access_flags: Cell::new(vk::AccessFlags::empty()),
            usage_flags: vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::SAMPLED,
            image_view: Cell::new(vk::ImageView::null()),
            image_memory: Cell::new(vk::DeviceMemory::null()),
            handle: Cell::new(vk::Image::null()),
            command_buffer_pool,
            format,
            aspect_flags: Cell::new(vk::ImageAspectFlags::empty()),
            mip_levels: Cell::new(mip_levels),
            size,
            memory_size: Cell::new(0),
            current_layout: Cell::new(vk::ImageLayout::UNDEFINED),
        }
    }

    #[allow(dead_code)] // Read by the tests.
    pub fn usage_flags(&self) -> vk::ImageUsageFlags {
        self.usage_flags
    }

    pub fn handle(&self) -> vk::Image {
        self.handle.get()
    }

    #[allow(dead_code)]
    pub fn format(&self) -> vk::Format {
        self.format
    }

    #[allow(dead_code)]
    pub fn aspect_flags(&self) -> vk::ImageAspectFlags {
        self.aspect_flags.get()
    }

    pub fn mip_levels(&self) -> u32 {
        self.mip_levels.get()
    }

    pub fn size(&self) -> PixelSize {
        self.size
    }

    pub fn memory_size(&self) -> u64 {
        self.memory_size.get()
    }

    pub fn current_layout(&self) -> vk::ImageLayout {
        self.current_layout.get()
    }

    /// Records the layout an image is in that was brought there by another
    /// user of its memory.
    pub fn set_current_layout(&self, layout: vk::ImageLayout) {
        self.current_layout.set(layout);
    }

    pub fn memory_handle(&self) -> vk::DeviceMemory {
        self.image_memory.get()
    }

    pub fn tiling(&self) -> vk::ImageTiling {
        vk::ImageTiling::OPTIMAL
    }

    pub fn image_info(&self) -> VulkanImageInfo {
        VulkanImageInfo {
            handle: vk::Handle::as_raw(self.handle()),
            pixel_size: self.size,
            format: self.format.as_raw() as u32,
            memory_handle: vk::Handle::as_raw(self.memory_handle()),
            memory_size: self.memory_size(),
            view_handle: vk::Handle::as_raw(self.image_view.get()),
            usage_flags: self.usage_flags.as_raw(),
            layout: self.current_layout().as_raw() as u32,
            tiling: self.tiling().as_raw() as u32,
            level_count: self.mip_levels(),
            sample_count: 1,
            is_protected: false,
        }
    }

    /// Device-local memory of the device for an image.
    pub fn create_memory(&self, _image: vk::Image, size: u64, memory_type_bits: u32) -> Result<vk::DeviceMemory, VulkanError> {
        let memory_allocate_info = vk::MemoryAllocateInfo::default().allocation_size(size).memory_type_index(
            VulkanMemoryHelper::find_suitable_memory_type_index(
                &*self.context,
                memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ) as u32,
        );

        self.context
            .device_api()
            .allocate_memory(self.context.device_handle(), &memory_allocate_info)
            .throw_on_error("vkAllocateMemory")
    }

    /// Creates the image, its memory and its view, unless that was done
    /// already. `external_memory` describes the handles the memory of the
    /// image may be imported from; `create_memory` replaces
    /// [`create_memory`](Self::create_memory).
    pub fn initialize(
        &self,
        external_memory: Option<&mut vk::ExternalMemoryImageCreateInfo<'_>>,
        create_memory: Option<CreateMemory<'_>>,
    ) -> Result<(), VulkanError> {
        if self.handle() != vk::Image::null() {
            return Ok(());
        }
        let api = self.context.device_api();
        let device = self.context.device_handle();

        self.mip_levels.set(mip_levels_for(self.mip_levels.get(), self.size));

        let mut create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(self.format)
            .extent(vk::Extent3D { depth: 1, width: self.size.width as u32, height: self.size.height as u32 })
            .mip_levels(self.mip_levels.get())
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(self.tiling())
            .usage(self.usage_flags)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .flags(vk::ImageCreateFlags::MUTABLE_FORMAT);
        if let Some(external_memory) = external_memory {
            create_info = create_info.push_next(external_memory);
        }

        let handle = api.create_image(device, &create_info).throw_on_error("vkCreateImage")?;
        self.handle.set(handle);

        let memory_requirements = api.get_image_memory_requirements(device, handle);

        let memory = match create_memory {
            Some(create_memory) => create_memory(handle, memory_requirements.size, memory_requirements.memory_type_bits),
            None => self.create_memory(handle, memory_requirements.size, memory_requirements.memory_type_bits),
        };
        let memory = match memory {
            Ok(memory) => memory,
            Err(error) => {
                api.destroy_image(device, handle);
                return Err(error);
            }
        };
        self.image_memory.set(memory);

        api.bind_image_memory(device, handle, memory, 0).throw_on_error("vkBindImageMemory")?;

        self.memory_size.set(memory_requirements.size);
        self.aspect_flags.set(vk::ImageAspectFlags::COLOR);

        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .components(vk::ComponentMapping::default())
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: self.aspect_flags.get(),
                level_count: self.mip_levels.get(),
                layer_count: 1,
                ..Default::default()
            })
            .format(self.format)
            .image(handle)
            .view_type(vk::ImageViewType::TYPE_2D);

        let image_view = api.create_image_view(device, &image_view_create_info).throw_on_error("vkCreateImageView")?;
        self.image_view.set(image_view);

        self.current_layout.set(vk::ImageLayout::UNDEFINED);
        Ok(())
    }

    /// Submits the change of the layout of the image and records the new
    /// layout.
    pub fn transition_layout(
        &self,
        destination_layout: vk::ImageLayout,
        destination_access_flags: vk::AccessFlags,
    ) -> Result<(), VulkanError> {
        let command_buffer = self.command_buffer_pool.create_command_buffer()?;
        command_buffer.begin_recording()?;

        VulkanMemoryHelper::transition_layout(
            &*self.context,
            &command_buffer,
            self.handle(),
            self.current_layout.get(),
            self.current_access_flags.get(),
            destination_layout,
            destination_access_flags,
            self.mip_levels.get(),
        );

        command_buffer.end_recording()?;

        command_buffer.submit()?;

        self.current_layout.set(destination_layout);
        self.current_access_flags.set(destination_access_flags);
        Ok(())
    }

    /// [`transition_layout`](Self::transition_layout) with the numeric
    /// values of the layout and of the access flags.
    #[allow(dead_code)] // The public overload of the reference, which nothing calls there either.
    pub fn transition_layout_raw(
        &self,
        destination_layout: u32,
        destination_access_flags: u32,
    ) -> Result<(), VulkanError> {
        self.transition_layout(
            vk::ImageLayout::from_raw(destination_layout as i32),
            vk::AccessFlags::from_raw(destination_access_flags),
        )
    }

    pub fn dispose(&self) {
        let api = self.context.device_api();
        let d = self.context.device_handle();
        if self.image_view.get() != vk::ImageView::null() {
            api.destroy_image_view(d, self.image_view.get());
            self.image_view.set(vk::ImageView::null());
        }
        if self.handle.get() != vk::Image::null() {
            api.destroy_image(d, self.handle.get());
            self.handle.set(vk::Image::null());
        }
        if self.image_memory.get() != vk::DeviceMemory::null() {
            api.free_memory(d, self.image_memory.get());
            self.image_memory.set(vk::DeviceMemory::null());
        }
    }
}

/// The number of mip levels of an image: the given one, or for zero the
/// whole part of the binary logarithm of the larger side.
pub(crate) fn mip_levels_for(mip_levels: u32, size: PixelSize) -> u32 {
    if mip_levels != 0 {
        mip_levels
    } else {
        // A side of zero or less has no logarithm; the conversion of the
        // result to an unsigned number gives zero.
        (size.width.max(size.height) as f64).log2().floor() as u32
    }
}

/// An image the framework renders a frame to.
pub(crate) struct VulkanImage {
    base: VulkanImageBase,
}

impl VulkanImage {
    /// Creates the image and brings it into the layout of a colour
    /// attachment.
    pub fn new(
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
        command_buffer_pool: Rc<VulkanCommandBufferPool>,
        format: vk::Format,
        size: PixelSize,
        mip_levels: u32,
    ) -> Result<Self, VulkanError> {
        let base = VulkanImageBase::new(context, command_buffer_pool, format, size, mip_levels);
        base.initialize(None, None)?;
        base.transition_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::AccessFlags::NONE_KHR)?;
        Ok(Self { base })
    }
}

impl std::ops::Deref for VulkanImage {
    type Target = VulkanImageBase;

    fn deref(&self) -> &VulkanImageBase {
        &self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_mip_levels_are_kept() {
        assert_eq!(mip_levels_for(1, PixelSize::new(1024, 768)), 1);
        assert_eq!(mip_levels_for(4, PixelSize::new(2, 2)), 4);
    }

    #[test]
    fn zero_mip_levels_are_the_logarithm_of_the_larger_side() {
        assert_eq!(mip_levels_for(0, PixelSize::new(1024, 768)), 10);
        assert_eq!(mip_levels_for(0, PixelSize::new(640, 1023)), 9);
        assert_eq!(mip_levels_for(0, PixelSize::new(2, 1)), 1);
        // The reference computes zero levels for one pixel.
        assert_eq!(mip_levels_for(0, PixelSize::new(1, 1)), 0);
        assert_eq!(mip_levels_for(0, PixelSize::new(0, 0)), 0);
    }
}
