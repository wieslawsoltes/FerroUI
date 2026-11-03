use super::{
    VulkanCommandBuffer, VulkanCommandBufferPool, VulkanImage, VulkanKhrSurface, VulkanMemoryHelper,
    VulkanSemaphore,
};
use crate::{
    IVulkanKhrSurfacePlatformSurface, IVulkanPlatformGraphicsContext, VulkanError, VulkanExceptionExtensions,
    VulkanResultExtensions,
};
use ash::vk;
use ferroui_base::PixelSize;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

/// The swap chain of a platform surface and what presenting a frame to it
/// needs.
pub(crate) struct VulkanDisplay {
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    acquire_slots: RefCell<Vec<AcquireSlot>>,
    slot: Cell<usize>,
    next_image: Cell<u32>,
    surface: RefCell<Option<VulkanKhrSurface>>,
    surface_format: Cell<vk::SurfaceFormatKHR>,
    swapchain: Cell<vk::SwapchainKHR>,
    swapchain_extent: Cell<vk::Extent2D>,
    platform_surface: Rc<dyn IVulkanKhrSurfacePlatformSurface>,
    swapchain_images: RefCell<Vec<SwapchainImage>>,
    disposed: Cell<bool>,
    command_buffer_pool: Rc<VulkanCommandBufferPool>,
    size: Cell<PixelSize>,
    /// What the swap chain was created with.
    parameters: Cell<SwapchainParameters>,
    /// What the results of the presentation engine said of the swap chain
    /// since it was last checked, before a frame.
    state: Cell<SwapchainState>,
}

/// The acquire signal can be reused once its consuming queue submission is complete. Keep at
/// most two frames in flight, and wait only when a slot is about to be reused.
struct AcquireSlot {
    available: VulkanSemaphore,
    last_submission: Option<Rc<VulkanCommandBuffer>>,
}

impl Drop for AcquireSlot {
    fn drop(&mut self) {
        self.available.dispose();
    }
}

/// Presentation may still be waiting on the signal after the blit fence fires. Reacquiring
/// this particular image establishes when it is safe to signal its semaphore again.
struct SwapchainImage {
    handle: vk::Image,
    finished: VulkanSemaphore,
}

impl Drop for SwapchainImage {
    fn drop(&mut self) {
        self.finished.dispose();
    }
}

/// What a result of `vkAcquireNextImageKHR` or `vkQueuePresentKHR` means for
/// the swap chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SwapchainResult {
    /// The image was acquired or presented.
    Success,
    /// The image was acquired or presented, and the swap chain no longer
    /// matches the surface exactly (`VK_SUBOPTIMAL_KHR`): a success.
    Suboptimal,
    /// The swap chain can no longer be used (`VK_ERROR_OUT_OF_DATE_KHR`):
    /// nothing was acquired or presented.
    OutOfDate,
    /// The surface is gone (`VK_ERROR_SURFACE_LOST_KHR`).
    SurfaceLost,
    /// Any other result, a failure.
    Failed(vk::Result),
}

impl SwapchainResult {
    pub fn of(result: vk::Result) -> Self {
        match result {
            vk::Result::SUCCESS => Self::Success,
            vk::Result::SUBOPTIMAL_KHR => Self::Suboptimal,
            vk::Result::ERROR_OUT_OF_DATE_KHR => Self::OutOfDate,
            vk::Result::ERROR_SURFACE_LOST_KHR => Self::SurfaceLost,
            other => Self::Failed(other),
        }
    }

    fn severity(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Suboptimal => 1,
            Self::OutOfDate => 2,
            Self::SurfaceLost => 3,
            Self::Failed(_) => 4,
        }
    }

    /// What a present means: the result of the call and the result for the
    /// one swap chain presented, the worse of the two.
    pub fn of_present(call_result: vk::Result, result: vk::Result) -> Self {
        let (call, swapchain) = (Self::of(call_result), Self::of(result));
        if call.severity() >= swapchain.severity() { call } else { swapchain }
    }
}

/// What is known of the swap chain between two frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SwapchainState {
    /// Nothing was reported.
    Current,
    /// An image was acquired or presented with `VK_SUBOPTIMAL_KHR`: the swap
    /// chain is recreated before the next frame if the surface now asks for
    /// other parameters than it was created with.
    Suboptimal,
    /// A present answered `VK_ERROR_OUT_OF_DATE_KHR`: the swap chain is
    /// recreated before the next frame.
    OutOfDate,
}

impl SwapchainState {
    /// The state after `result`, which was a success or out of date.
    pub fn after(self, result: SwapchainResult) -> Self {
        match (self, result) {
            (_, SwapchainResult::OutOfDate) | (Self::OutOfDate, _) => Self::OutOfDate,
            (_, SwapchainResult::Suboptimal) | (Self::Suboptimal, _) => Self::Suboptimal,
            _ => Self::Current,
        }
    }

    /// Whether the swap chain is recreated before a frame: when the size of
    /// the surface is not the size of the swap chain, when it is out of
    /// date, and when it was reported suboptimal and the surface now asks
    /// for other parameters (`parameters_changed`, asked only then).
    pub fn needs_recreation(
        self,
        size_differs: bool,
        parameters_changed: impl FnOnce() -> Result<bool, VulkanError>,
    ) -> Result<bool, VulkanError> {
        Ok(match self {
            _ if size_differs => true,
            Self::OutOfDate => true,
            Self::Suboptimal => parameters_changed()?,
            Self::Current => false,
        })
    }
}

/// What a swap chain is created with, as far as it follows from the
/// capabilities of the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SwapchainParameters {
    pub image_count: u32,
    pub extent: vk::Extent2D,
    pub present_mode: vk::PresentModeKHR,
    pub pre_transform: vk::SurfaceTransformFlagsKHR,
    pub composite_alpha: vk::CompositeAlphaFlagsKHR,
}

impl SwapchainParameters {
    /// One image more than the surface needs, within its limit; the extent
    /// of the surface, or the size of the platform surface within the
    /// limits when the surface leaves the extent open; mailbox before FIFO
    /// before immediate presentation; no rotation by the swap chain when
    /// the surface is rotated by a quarter turn and can do without;
    /// premultiplied alpha when the surface composes it.
    pub fn new(
        capabilities: &vk::SurfaceCapabilitiesKHR,
        modes: &[vk::PresentModeKHR],
        surface_size: PixelSize,
    ) -> Self {
        let mut image_count = capabilities.min_image_count.wrapping_add(1);
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        let supports_identity_transform =
            capabilities.supported_transforms.contains(vk::SurfaceTransformFlagsKHR::IDENTITY);
        let is_rotated = capabilities.current_transform.contains(vk::SurfaceTransformFlagsKHR::ROTATE_90)
            || capabilities.current_transform.contains(vk::SurfaceTransformFlagsKHR::ROTATE_270);

        let extent = if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            let width = capabilities
                .min_image_extent
                .width
                .max(capabilities.max_image_extent.width.min(surface_size.width as u32));
            let height = capabilities
                .min_image_extent
                .height
                .max(capabilities.max_image_extent.height.min(surface_size.height as u32));
            vk::Extent2D { width, height }
        };

        let present_mode = if modes.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else if modes.contains(&vk::PresentModeKHR::FIFO) {
            vk::PresentModeKHR::FIFO
        } else {
            vk::PresentModeKHR::IMMEDIATE
        };

        Self {
            image_count,
            extent,
            present_mode,
            pre_transform: if supports_identity_transform && is_rotated {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                capabilities.current_transform
            },
            composite_alpha: if capabilities
                .supported_composite_alpha
                .contains(vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED)
            {
                vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED
            } else {
                vk::CompositeAlphaFlagsKHR::OPAQUE
            },
        }
    }
}

impl VulkanDisplay {
    fn new(
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
        surface: VulkanKhrSurface,
        swapchain: vk::SwapchainKHR,
        parameters: SwapchainParameters,
        platform_surface: Rc<dyn IVulkanKhrSurfacePlatformSurface>,
    ) -> Result<Self, VulkanError> {
        let command_buffer_pool = match VulkanCommandBufferPool::new(context.clone(), false) {
            Ok(pool) => pool,
            Err(error) => {
                context.device_api().destroy_swapchain_khr(context.device_handle(), swapchain);
                surface.dispose();
                return Err(error);
            }
        };
        let display = Self {
            context,
            acquire_slots: RefCell::new(Vec::new()),
            slot: Cell::new(0),
            next_image: Cell::new(0),
            surface: RefCell::new(Some(surface)),
            surface_format: Cell::new(vk::SurfaceFormatKHR::default()),
            swapchain: Cell::new(swapchain),
            swapchain_extent: Cell::new(parameters.extent),
            platform_surface,
            swapchain_images: RefCell::new(Vec::new()),
            disposed: Cell::new(false),
            command_buffer_pool,
            size: Cell::new(PixelSize::default()),
            parameters: Cell::new(parameters),
            state: Cell::new(SwapchainState::Current),
        };
        display.create_swapchain_images()?;
        Ok(display)
    }

    pub fn command_buffer_pool(&self) -> &Rc<VulkanCommandBufferPool> {
        &self.command_buffer_pool
    }

    pub fn size(&self) -> PixelSize {
        self.size.get()
    }

    /// The format of the surface, asked once.
    pub fn surface_format(&self) -> Result<vk::SurfaceFormatKHR, VulkanError> {
        if self.surface_format.get().format == vk::Format::UNDEFINED {
            if let Some(surface) = self.surface.borrow().as_ref() {
                self.surface_format.set(surface.get_surface_format()?);
            }
        }
        Ok(self.surface_format.get())
    }

    fn create_swapchain(
        context: &Rc<dyn IVulkanPlatformGraphicsContext>,
        surface: &VulkanKhrSurface,
        old_display: Option<&VulkanDisplay>,
    ) -> Result<(vk::SwapchainKHR, SwapchainParameters), VulkanError> {
        while !surface.can_surface_present()? {
            thread::sleep(Duration::from_millis(16));
        }

        let capabilities = context
            .instance_api()
            .get_physical_device_surface_capabilities_khr(context.physical_device_handle(), surface.handle())
            .throw_on_error("vkGetPhysicalDeviceSurfaceCapabilitiesKHR")?;

        let modes = context
            .instance_api()
            .get_physical_device_surface_present_modes_khr(context.physical_device_handle(), surface.handle())
            .throw_on_error("vkGetPhysicalDeviceSurfacePresentModesKHR")?;

        let surface_format = surface.get_surface_format()?;
        let parameters = SwapchainParameters::new(&capabilities, &modes, surface.size());

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.handle())
            .min_image_count(parameters.image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(parameters.extent)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .image_array_layers(1)
            .pre_transform(parameters.pre_transform)
            .composite_alpha(parameters.composite_alpha)
            .present_mode(parameters.present_mode)
            .clipped(true)
            .old_swapchain(old_display.map_or(vk::SwapchainKHR::null(), |display| display.swapchain.get()));
        let swapchain = context
            .device_api()
            .create_swapchain_khr(context.device_handle(), &swapchain_create_info)
            .throw_on_error("vkCreateSwapchainKHR")?;
        if let Some(old_display) = old_display {
            old_display.destroy_swapchain();
        }
        Ok((swapchain, parameters))
    }

    /// Whether the surface now asks for other parameters of a swap chain
    /// than the current one was created with (the present mode aside).
    fn parameters_changed(&self) -> Result<bool, VulkanError> {
        let surface = self.surface.borrow();
        let Some(surface) = surface.as_ref() else { return Ok(true) };
        let capabilities = self
            .context
            .instance_api()
            .get_physical_device_surface_capabilities_khr(self.context.physical_device_handle(), surface.handle())
            .throw_on_error("vkGetPhysicalDeviceSurfaceCapabilitiesKHR")?;
        let current = self.parameters.get();
        Ok(SwapchainParameters::new(&capabilities, &[current.present_mode], surface.size()) != current)
    }

    fn destroy_swapchain(&self) {
        if self.swapchain.get() != vk::SwapchainKHR::null() {
            self.context.device_api().destroy_swapchain_khr(self.context.device_handle(), self.swapchain.get());
        }
        self.swapchain.set(vk::SwapchainKHR::null());
    }

    /// Creates the surface of the platform surface and a swap chain for it.
    pub fn create_display(
        context: &Rc<dyn IVulkanPlatformGraphicsContext>,
        surface: Rc<dyn IVulkanKhrSurfacePlatformSurface>,
    ) -> Result<VulkanDisplay, VulkanError> {
        let khr_surface = VulkanKhrSurface::new(context.clone(), surface.clone());
        let (swapchain, parameters) = match Self::create_swapchain(context, &khr_surface, None) {
            Ok(created) => created,
            Err(error) => {
                khr_surface.dispose();
                return Err(error);
            }
        };
        VulkanDisplay::new(context.clone(), khr_surface, swapchain, parameters, surface)
    }

    fn create_swapchain_images(&self) -> Result<(), VulkanError> {
        let handles = self.context.device_api()
            .get_swapchain_images_khr(self.context.device_handle(), self.swapchain.get())
            .throw_on_error("vkGetSwapchainImagesKHR")?;
        if handles.is_empty() {
            return Err(VulkanError::new("The swap chain has no images"));
        }
        let slot_count = handles.len().min(2);
        let mut slots = Vec::with_capacity(slot_count);
        for _ in 0..slot_count {
            slots.push(AcquireSlot { available: VulkanSemaphore::new(self.context.clone())?, last_submission: None });
        }
        let mut images = Vec::with_capacity(handles.len());
        for handle in handles {
            images.push(SwapchainImage { handle, finished: VulkanSemaphore::new(self.context.clone())? });
        }
        // The old generation is idle before replacement. Temporaries clean themselves up on failure.
        self.swapchain_images.replace(images);
        self.acquire_slots.replace(slots);
        self.slot.set(0);
        let extent = self.swapchain_extent.get();
        self.size.set(PixelSize::new(extent.width as i32, extent.height as i32));
        Ok(())
    }

    fn recreate_swapchain(&self) -> Result<(), VulkanError> {
        self.state.set(SwapchainState::OutOfDate);
        if self.surface.borrow().is_none() {
            return self.recreate_surface();
        }
        let _ = self.context.device_api().device_wait_idle(self.context.device_handle());
        let (swapchain, parameters) = {
            let surface = self.surface.borrow();
            let surface = surface.as_ref().expect("the surface was checked above");
            Self::create_swapchain(&self.context, surface, Some(self))?
        };
        self.swapchain.set(swapchain);
        self.swapchain_extent.set(parameters.extent);
        self.parameters.set(parameters);
        self.create_swapchain_images()?;
        self.state.set(SwapchainState::Current);
        Ok(())
    }

    fn recreate_surface(&self) -> Result<(), VulkanError> {
        let _ = self.context.device_api().device_wait_idle(self.context.device_handle());
        self.destroy_swapchain();
        self.surface_format.set(vk::SurfaceFormatKHR::default());
        if let Some(surface) = self.surface.borrow_mut().take() {
            surface.dispose();
        }
        *self.surface.borrow_mut() =
            Some(VulkanKhrSurface::new(self.context.clone(), self.platform_surface.clone()));
        self.recreate_swapchain()
    }

    /// Recreates the swap chain when the size of the surface is not the
    /// size of the swap chain anymore, or when a result of the last frame
    /// asked for it (`SwapchainState`), and tells that it did.
    pub fn ensure_swapchain_available(&self) -> Result<bool, VulkanError> {
        let surface_size = self.surface.borrow().as_ref().map(|surface| surface.size());
        let state = self.state.replace(SwapchainState::Current);
        if state.needs_recreation(Some(self.size.get()) != surface_size, || self.parameters_changed())? {
            self.recreate_swapchain()?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Acquires the next image of the swap chain and starts the commands
    /// that fill it.
    pub fn start_presentation(&self) -> Result<Rc<VulkanCommandBuffer>, VulkanError> {
        self.next_image.set(0);
        loop {
            if let Some(previous) = self.acquire_slots.borrow_mut()[self.slot.get()].last_submission.take() {
                previous.wait_for_completion()?;
            }
            let (acquire_result, next_image) = self.context.device_api().acquire_next_image_khr(
                self.context.device_handle(),
                self.swapchain.get(),
                u64::MAX,
                self.acquire_slots.borrow()[self.slot.get()].available.handle(),
                vk::Fence::null(),
            );
            self.next_image.set(next_image);
            match SwapchainResult::of(acquire_result) {
                SwapchainResult::Success => break,
                // The image is acquired and its semaphore will be signalled: it is used, and the
                // swap chain is looked at again before the next frame.
                SwapchainResult::Suboptimal => {
                    self.state.set(self.state.get().after(SwapchainResult::Suboptimal));
                    break;
                }
                SwapchainResult::OutOfDate => self.recreate_swapchain()?,
                SwapchainResult::SurfaceLost => self.recreate_surface()?,
                SwapchainResult::Failed(result) => result.throw_on_error("vkAcquireNextImageKHR")?,
            }
        }

        // If recording or submission fails, the acquire semaphore is already signalled.
        // Retire this generation on retry rather than signal an unconsumed semaphore.
        let retire_on_error = |_: &VulkanError| self.state.set(SwapchainState::OutOfDate);
        let command_buffer = self.command_buffer_pool.create_command_buffer().inspect_err(retire_on_error)?;
        command_buffer.begin_recording().inspect_err(retire_on_error)?;
        VulkanMemoryHelper::transition_layout(
            &*self.context,
            &command_buffer,
            self.current_image(),
            vk::ImageLayout::UNDEFINED,
            vk::AccessFlags::NONE,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
            1,
        );
        Ok(command_buffer)
    }

    fn current_image(&self) -> vk::Image {
        self.swapchain_images.borrow()[self.next_image.get() as usize].handle
    }

    /// Records the copy of `image`, scaled to the size of the swap chain,
    /// into the acquired image.
    pub fn blit_image_to_current_image(&self, command_buffer: &VulkanCommandBuffer, image: &VulkanImage) {
        VulkanMemoryHelper::transition_layout(
            &*self.context,
            command_buffer,
            image.handle(),
            image.current_layout(),
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::AccessFlags::TRANSFER_READ,
            image.mip_levels(),
        );

        let size = self.size.get();
        let src_blit_region = vk::ImageBlit {
            src_offsets: [
                vk::Offset3D::default(),
                vk::Offset3D { x: image.size().width, y: image.size().height, z: 1 },
            ],
            dst_offsets: [vk::Offset3D::default(), vk::Offset3D { x: size.width, y: size.height, z: 1 }],
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                layer_count: 1,
                ..Default::default()
            },
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                layer_count: 1,
                ..Default::default()
            },
        };

        self.context.device_api().cmd_blit_image(
            command_buffer.handle(),
            image.handle(),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            self.current_image(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[src_blit_region],
            vk::Filter::LINEAR,
        );

        VulkanMemoryHelper::transition_layout(
            &*self.context,
            command_buffer,
            image.handle(),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::AccessFlags::TRANSFER_READ,
            image.current_layout(),
            vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            image.mip_levels(),
        );
    }

    /// Submits the commands and presents the acquired image.
    pub fn end_presentation(&self, command_buffer: &Rc<VulkanCommandBuffer>) -> Result<(), VulkanError> {
        VulkanMemoryHelper::transition_layout(
            &*self.context,
            command_buffer,
            self.current_image(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::AccessFlags::NONE,
            1,
        );
        let images = self.swapchain_images.borrow();
        let finished = &images[self.next_image.get() as usize].finished;
        let mut slots = self.acquire_slots.borrow_mut();
        let slot_index = self.slot.get();
        let slot = &mut slots[slot_index];
        command_buffer.submit_with(
            &[&slot.available],
            // Include the opening layout transition as well as the transfer blit.
            &[vk::PipelineStageFlags::ALL_COMMANDS],
            &[finished],
            None,
        ).inspect_err(|_| self.state.set(SwapchainState::OutOfDate))?;
        slot.last_submission = Some(command_buffer.clone());
        self.slot.set((slot_index + 1) % slots.len());

        let (call_result, result) = self.context.device_api().vk_queue_present_khr(
            self.context.main_queue_handle(),
            finished.handle(),
            self.swapchain.get(),
            self.next_image.get(),
        );
        match SwapchainResult::of_present(call_result, result) {
            SwapchainResult::Success => Ok(()),
            // Presented (suboptimal), or not presented because the swap chain is out of date: the
            // frame is not a failure, and the swap chain is looked at before the next one.
            outcome @ (SwapchainResult::Suboptimal | SwapchainResult::OutOfDate) => {
                self.state.set(self.state.get().after(outcome));
                Ok(())
            }
            SwapchainResult::SurfaceLost => {
                self.state.set(SwapchainState::OutOfDate);
                vk::Result::ERROR_SURFACE_LOST_KHR.throw_on_error("vkQueuePresentKHR")
            }
            SwapchainResult::Failed(result) => {
                self.state.set(SwapchainState::OutOfDate);
                result.throw_on_error("vkQueuePresentKHR")
            }
        }
    }

    pub fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let _ = self.context.device_api().device_wait_idle(self.context.device_handle());
        self.swapchain_images.borrow_mut().clear();
        self.acquire_slots.borrow_mut().clear();
        self.destroy_swapchain();
        self.command_buffer_pool.dispose();
        if let Some(surface) = self.surface.borrow_mut().take() {
            surface.dispose();
        }
    }
}

impl Drop for VulkanDisplay {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities() -> vk::SurfaceCapabilitiesKHR {
        vk::SurfaceCapabilitiesKHR {
            min_image_count: 2,
            max_image_count: 0,
            current_extent: vk::Extent2D { width: 800, height: 600 },
            min_image_extent: vk::Extent2D { width: 1, height: 1 },
            max_image_extent: vk::Extent2D { width: 4096, height: 4096 },
            max_image_array_layers: 1,
            supported_transforms: vk::SurfaceTransformFlagsKHR::IDENTITY,
            current_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
            supported_composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            supported_usage_flags: vk::ImageUsageFlags::COLOR_ATTACHMENT,
        }
    }

    const FIFO: &[vk::PresentModeKHR] = &[vk::PresentModeKHR::FIFO];

    #[test]
    fn one_image_more_than_the_minimum_within_the_maximum() {
        let mut caps = capabilities();
        assert_eq!(SwapchainParameters::new(&caps, FIFO, PixelSize::new(1, 1)).image_count, 3);
        caps.max_image_count = 8;
        assert_eq!(SwapchainParameters::new(&caps, FIFO, PixelSize::new(1, 1)).image_count, 3);
        caps.max_image_count = 2;
        assert_eq!(SwapchainParameters::new(&caps, FIFO, PixelSize::new(1, 1)).image_count, 2);
    }

    #[test]
    fn the_extent_of_the_surface_is_taken_when_it_has_one() {
        let caps = capabilities();
        let parameters = SwapchainParameters::new(&caps, FIFO, PixelSize::new(320, 200));
        assert_eq!(parameters.extent, vk::Extent2D { width: 800, height: 600 });
    }

    #[test]
    fn an_open_extent_is_the_size_of_the_platform_surface_within_the_limits() {
        let mut caps = capabilities();
        caps.current_extent = vk::Extent2D { width: u32::MAX, height: u32::MAX };
        caps.min_image_extent = vk::Extent2D { width: 16, height: 16 };
        caps.max_image_extent = vk::Extent2D { width: 1000, height: 500 };
        let extent = |w, h| SwapchainParameters::new(&caps, FIFO, PixelSize::new(w, h)).extent;
        assert_eq!(extent(320, 200), vk::Extent2D { width: 320, height: 200 });
        assert_eq!(extent(4000, 3000), vk::Extent2D { width: 1000, height: 500 });
        assert_eq!(extent(1, 2), vk::Extent2D { width: 16, height: 16 });
    }

    #[test]
    fn mailbox_before_fifo_before_immediate() {
        let caps = capabilities();
        let mode = |modes: &[vk::PresentModeKHR]| SwapchainParameters::new(&caps, modes, PixelSize::new(1, 1)).present_mode;
        assert_eq!(
            mode(&[vk::PresentModeKHR::IMMEDIATE, vk::PresentModeKHR::FIFO, vk::PresentModeKHR::MAILBOX]),
            vk::PresentModeKHR::MAILBOX
        );
        assert_eq!(mode(&[vk::PresentModeKHR::IMMEDIATE, vk::PresentModeKHR::FIFO]), vk::PresentModeKHR::FIFO);
        assert_eq!(mode(&[vk::PresentModeKHR::FIFO_RELAXED]), vk::PresentModeKHR::IMMEDIATE);
        assert_eq!(mode(&[]), vk::PresentModeKHR::IMMEDIATE);
    }

    #[test]
    fn a_quarter_turn_is_left_to_the_compositor_when_the_surface_allows() {
        let mut caps = capabilities();
        caps.current_transform = vk::SurfaceTransformFlagsKHR::ROTATE_90;
        caps.supported_transforms = vk::SurfaceTransformFlagsKHR::IDENTITY | vk::SurfaceTransformFlagsKHR::ROTATE_90;
        let transform = |caps: &vk::SurfaceCapabilitiesKHR| {
            SwapchainParameters::new(caps, FIFO, PixelSize::new(1, 1)).pre_transform
        };
        assert_eq!(transform(&caps), vk::SurfaceTransformFlagsKHR::IDENTITY);
        caps.current_transform = vk::SurfaceTransformFlagsKHR::ROTATE_270;
        assert_eq!(transform(&caps), vk::SurfaceTransformFlagsKHR::IDENTITY);
        // A half turn is not handled by the swap chain either way.
        caps.current_transform = vk::SurfaceTransformFlagsKHR::ROTATE_180;
        assert_eq!(transform(&caps), vk::SurfaceTransformFlagsKHR::ROTATE_180);
        // Without the identity among the supported transforms the current one stays.
        caps.current_transform = vk::SurfaceTransformFlagsKHR::ROTATE_90;
        caps.supported_transforms = vk::SurfaceTransformFlagsKHR::ROTATE_90;
        assert_eq!(transform(&caps), vk::SurfaceTransformFlagsKHR::ROTATE_90);
    }

    #[test]
    fn the_results_of_the_presentation_engine_are_classified() {
        assert_eq!(SwapchainResult::of(vk::Result::SUCCESS), SwapchainResult::Success);
        assert_eq!(SwapchainResult::of(vk::Result::SUBOPTIMAL_KHR), SwapchainResult::Suboptimal);
        assert_eq!(SwapchainResult::of(vk::Result::ERROR_OUT_OF_DATE_KHR), SwapchainResult::OutOfDate);
        assert_eq!(SwapchainResult::of(vk::Result::ERROR_SURFACE_LOST_KHR), SwapchainResult::SurfaceLost);
        assert_eq!(
            SwapchainResult::of(vk::Result::ERROR_DEVICE_LOST),
            SwapchainResult::Failed(vk::Result::ERROR_DEVICE_LOST)
        );
        // Not reached with an infinite timeout, and no image: a failure, as every result but success was.
        assert_eq!(SwapchainResult::of(vk::Result::TIMEOUT), SwapchainResult::Failed(vk::Result::TIMEOUT));
    }

    #[test]
    fn a_present_is_the_worse_of_its_two_results() {
        let present = SwapchainResult::of_present;
        assert_eq!(present(vk::Result::SUCCESS, vk::Result::SUCCESS), SwapchainResult::Success);
        assert_eq!(present(vk::Result::SUBOPTIMAL_KHR, vk::Result::SUBOPTIMAL_KHR), SwapchainResult::Suboptimal);
        assert_eq!(present(vk::Result::SUCCESS, vk::Result::SUBOPTIMAL_KHR), SwapchainResult::Suboptimal);
        assert_eq!(
            present(vk::Result::ERROR_OUT_OF_DATE_KHR, vk::Result::SUBOPTIMAL_KHR),
            SwapchainResult::OutOfDate
        );
        assert_eq!(
            present(vk::Result::ERROR_DEVICE_LOST, vk::Result::SUCCESS),
            SwapchainResult::Failed(vk::Result::ERROR_DEVICE_LOST)
        );
        assert_eq!(
            present(vk::Result::SUBOPTIMAL_KHR, vk::Result::ERROR_SURFACE_LOST_KHR),
            SwapchainResult::SurfaceLost
        );
    }

    #[test]
    fn the_state_keeps_the_strongest_report_until_it_is_checked() {
        use SwapchainState::*;
        assert_eq!(Current.after(SwapchainResult::Success), Current);
        assert_eq!(Current.after(SwapchainResult::Suboptimal), Suboptimal);
        assert_eq!(Suboptimal.after(SwapchainResult::Success), Suboptimal);
        assert_eq!(Suboptimal.after(SwapchainResult::OutOfDate), OutOfDate);
        assert_eq!(OutOfDate.after(SwapchainResult::Suboptimal), OutOfDate);
    }

    #[test]
    fn a_suboptimal_swap_chain_is_recreated_only_when_the_surface_asks_for_other_parameters() {
        use SwapchainState::*;
        let never = || -> Result<bool, VulkanError> { panic!("the parameters are not asked for") };
        assert!(!Current.needs_recreation(false, never).unwrap());
        assert!(Current.needs_recreation(true, never).unwrap());
        assert!(OutOfDate.needs_recreation(false, never).unwrap());
        assert!(Suboptimal.needs_recreation(true, never).unwrap());
        assert!(!Suboptimal.needs_recreation(false, || Ok(false)).unwrap());
        assert!(Suboptimal.needs_recreation(false, || Ok(true)).unwrap());
    }

    #[test]
    fn a_rotated_surface_asks_for_the_same_parameters_again() {
        // Android while the screen is rotated: the swap chain keeps the identity transform, every
        // present answers VK_SUBOPTIMAL_KHR, and the capabilities give the parameters it was
        // created with, so it is not recreated for each frame.
        let mut caps = capabilities();
        caps.current_extent = vk::Extent2D { width: 2400, height: 1080 };
        caps.current_transform = vk::SurfaceTransformFlagsKHR::ROTATE_90;
        caps.supported_transforms = vk::SurfaceTransformFlagsKHR::IDENTITY | vk::SurfaceTransformFlagsKHR::ROTATE_90;
        let created = SwapchainParameters::new(&caps, &[vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::FIFO], PixelSize::new(2400, 1080));
        let again = SwapchainParameters::new(&caps, &[created.present_mode], PixelSize::new(2400, 1080));
        assert_eq!(created, again);
        // The screen rotated back: the extent differs, and the swap chain is recreated.
        caps.current_extent = vk::Extent2D { width: 1080, height: 2400 };
        caps.current_transform = vk::SurfaceTransformFlagsKHR::IDENTITY;
        assert_ne!(SwapchainParameters::new(&caps, &[created.present_mode], PixelSize::new(1080, 2400)), created);
    }

    #[test]
    fn premultiplied_alpha_when_the_surface_composes_it() {
        let mut caps = capabilities();
        let alpha = |caps: &vk::SurfaceCapabilitiesKHR| {
            SwapchainParameters::new(caps, FIFO, PixelSize::new(1, 1)).composite_alpha
        };
        assert_eq!(alpha(&caps), vk::CompositeAlphaFlagsKHR::OPAQUE);
        caps.supported_composite_alpha = vk::CompositeAlphaFlagsKHR::OPAQUE | vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED;
        assert_eq!(alpha(&caps), vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED);
        caps.supported_composite_alpha = vk::CompositeAlphaFlagsKHR::INHERIT;
        assert_eq!(alpha(&caps), vk::CompositeAlphaFlagsKHR::OPAQUE);
    }
}
