//! The shared frames of videos and cameras as DMA-BUFs on the Vulkan device of the GPU modes on
//! Linux (#575).
//!
//! **Behind a switch.** The environment variable `FERROUI_VELLO_DMA_BUF_IMPORT=1` turns this route
//! on, until a run in the Linux virtual machine has proved it. Without it, or where the device
//! lacks one of [`EXTENSIONS`], the device is made by `request_device` exactly as before and
//! reports no import. Every extension here is optional.
//!
//! A frame's buffer is imported as a `VkImage` with its explicit DRM format modifier, wrapped as a
//! texture of the device (`hal::vulkan::Device::texture_from_raw`) and becomes a
//! [`VelloSharedFrameBitmap`], converted on the device by one pass with its lease given back when
//! the queue has done that pass, as on macOS. Nothing goes through memory. An import that fails
//! refuses the frame; there is no CPU path.
//!
//! **Order.** The producer hands a frame over only once the implicit fences of its buffer have
//! signalled (`ferro_linux_frame_ready`). Each image is then taken from the foreign queue family
//! into `SHADER_READ_ONLY_OPTIMAL` by a command buffer submitted before the first pass reads it.
//!
//! This file has `unsafe` for the raw objects of Vulkan.

use crate::gpu::{VelloDeviceTexture, VelloSharedFrameBitmap, VelloTextureAlpha, VelloWgpuDevice};
use ash::{ext, khr, vk};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::{
    IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle, IPlatformRenderInterfaceImportedImage,
    IPlatformRenderInterfaceImportedObject, IPlatformRenderInterfaceImportedSemaphore,
    KnownPlatformGraphicsExternalImageHandleTypes as HandleTypes, PlatformGraphicsDrmFormat,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::frames::{
    DeviceIdentity, FrameAlpha, FrameColor, FrameError, FrameFormat, FrameLease,
};
use ferroui_base::rendering::composition::{
    CompositionGpuImportedImageSynchronizationCapabilities, ICompositionImportableSharedGpuContextImage,
};
use std::ffi::CStr;
use std::os::fd::{AsRawFd, BorrowedFd, IntoRawFd};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use wgpu::hal;

/// The largest frame the backend imports, in pixels on a side (as on the other routes).
const MAX_FRAME_SIDE: u32 = 8192;

/// `DRM_FORMAT_XRGB8888`: B, G, R, X in memory.
const DRM_XR24: u32 = 0x3432_5258;
/// `DRM_FORMAT_XBGR8888`: R, G, B, X in memory.
const DRM_XB24: u32 = 0x3432_4258;

/// The device extensions of the import. All of them, or the route is not offered.
const EXTENSIONS: [&CStr; 5] = [
    khr::external_memory_fd::NAME,
    ext::external_memory_dma_buf::NAME,
    ext::image_drm_format_modifier::NAME,
    ext::queue_family_foreign::NAME,
    khr::image_format_list::NAME,
];

/// Generations of the feature: a frame made for a device that was replaced is refused.
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Whether the route is turned on.
fn switched_on() -> bool {
    std::env::var("FERROUI_VELLO_DMA_BUF_IMPORT").is_ok_and(|value| value == "1")
}

/// The device of `adapter` with the extensions of the import, when the route is turned on and the
/// adapter has them all. `None` otherwise, and on any failure: the caller makes its device as it
/// always does.
pub(crate) fn request_device(
    adapter: &wgpu::Adapter,
    descriptor: &wgpu::DeviceDescriptor<'_>,
) -> Option<(wgpu::Device, wgpu::Queue)> {
    if !switched_on() {
        return None;
    }
    // SAFETY: the adapter is only read, and the device opened on it is handed to `wgpu` at once.
    unsafe {
        let open = {
            let hal_adapter = adapter.as_hal::<hal::api::Vulkan>()?;
            let instance = hal_adapter.shared_instance().raw_instance();
            let available = instance.enumerate_device_extension_properties(hal_adapter.raw_physical_device()).ok()?;
            let has = |name: &CStr| {
                available.iter().any(|extension| extension.extension_name_as_c_str().is_ok_and(|found| found == name))
            };
            if !EXTENSIONS.iter().all(|name| has(*name)) {
                return None;
            }
            let callback: Box<hal::vulkan::CreateDeviceCallback<'_>> = Box::new(|args| {
                for name in EXTENSIONS {
                    if !args.extensions.contains(&name) {
                        args.extensions.push(name);
                    }
                }
            });
            hal_adapter
                .open_with_callback(
                    descriptor.required_features,
                    &descriptor.required_limits,
                    &descriptor.memory_hints,
                    Some(callback),
                )
                .ok()?
        };
        adapter.create_device_from_hal(open, descriptor).ok()
    }
}

/// The command buffers that take images from the foreign queue family.
struct Transitions {
    pool: vk::CommandPool,
    /// Those submitted, until their fence signals.
    pending: Vec<(vk::CommandBuffer, vk::Fence)>,
}

/// The external objects feature of a Vulkan device made by [`request_device`].
pub struct VelloDmaBufExternalObjectsFeature {
    device: Arc<VelloWgpuDevice>,
    api: ash::Device,
    memory_fd: khr::external_memory_fd::Device,
    queue_family: u32,
    generation: u64,
    uuid: Vec<u8>,
    formats: Vec<PlatformGraphicsDrmFormat>,
    transitions: Mutex<Transitions>,
}

impl VelloDmaBufExternalObjectsFeature {
    /// The feature of `device`, which [`request_device`] made. `None` when it samples no explicit
    /// single-plane layout of the two formats.
    pub(crate) fn new(device: Arc<VelloWgpuDevice>) -> Option<Self> {
        // SAFETY: the raw objects are those of `device`, which the feature holds.
        unsafe {
            let (api, memory_fd, queue_family, uuid, formats) = {
                let hal_device = device.device().as_hal::<hal::api::Vulkan>()?;
                let instance = hal_device.shared_instance().raw_instance();
                let physical = hal_device.raw_physical_device();
                let api = hal_device.raw_device().clone();
                let memory_fd = khr::external_memory_fd::Device::new(instance, &api);
                let mut id = vk::PhysicalDeviceIDProperties::default();
                let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut id);
                instance.get_physical_device_properties2(physical, &mut properties);
                let uuid = id.device_uuid.to_vec();
                let mut formats = Vec::new();
                for (drm, format) in [(DRM_XR24, vk::Format::B8G8R8A8_UNORM), (DRM_XB24, vk::Format::R8G8B8A8_UNORM)] {
                    for modifier in sampled_modifiers(instance, physical, format) {
                        formats.push(PlatformGraphicsDrmFormat::new(drm, modifier));
                    }
                }
                (api, memory_fd, hal_device.queue_family_index(), uuid, formats)
            };
            if formats.is_empty() {
                return None;
            }
            let info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::TRANSIENT)
                .queue_family_index(queue_family);
            let pool = api.create_command_pool(&info, None).ok()?;
            Some(Self {
                device,
                api,
                memory_fd,
                queue_family,
                generation: NEXT_GENERATION.fetch_add(1, Ordering::Relaxed),
                uuid,
                formats,
                transitions: Mutex::new(Transitions { pool, pending: Vec::new() }),
            })
        }
    }

    /// Frees the command buffers the device has finished.
    fn reap(&self, transitions: &mut Transitions) {
        let (api, pool) = (&self.api, transitions.pool);
        transitions.pending.retain(|&(command_buffer, fence)| {
            // SAFETY: the fence and command buffer are this pool's; a signalled one is not in use.
            unsafe {
                if !matches!(api.get_fence_status(fence), Ok(true) | Err(_)) {
                    return true;
                }
                api.destroy_fence(fence, None);
                api.free_command_buffers(pool, &[command_buffer]);
            }
            false
        });
    }

    /// Records and submits the transition of `image` from the foreign queue family. The queue
    /// runs it before any pass submitted later.
    fn acquire_from_foreign(&self, image: vk::Image) -> Result<(), String> {
        let mut transitions = self.transitions.lock().unwrap_or_else(|error| error.into_inner());
        self.reap(&mut transitions);
        let api = &self.api;
        let allocate = vk::CommandBufferAllocateInfo::default()
            .command_pool(transitions.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let failed = |what: &str, error: vk::Result| format!("{what} failed: {error:?}");
        // SAFETY: the command buffer is new and used once; the pool is locked; the queue is this
        // device's, and the render thread is the one that submits to it.
        unsafe {
            let command_buffer =
                api.allocate_command_buffers(&allocate).map_err(|error| failed("vkAllocateCommandBuffers", error))?[0];
            let free = |error: String| {
                api.free_command_buffers(transitions.pool, &[command_buffer]);
                error
            };
            let begin = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            api.begin_command_buffer(command_buffer, &begin)
                .map_err(|error| free(failed("vkBeginCommandBuffer", error)))?;
            let range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1);
            let barrier = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::GENERAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_FOREIGN_EXT)
                .dst_queue_family_index(self.queue_family)
                .image(image)
                .subresource_range(range);
            api.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
            api.end_command_buffer(command_buffer).map_err(|error| free(failed("vkEndCommandBuffer", error)))?;
            let fence = api
                .create_fence(&vk::FenceCreateInfo::default(), None)
                .map_err(|error| free(failed("vkCreateFence", error)))?;
            let buffers = [command_buffer];
            let submit = vk::SubmitInfo::default().command_buffers(&buffers);
            let queue = {
                let hal_device = self.device.device().as_hal::<hal::api::Vulkan>();
                hal_device.map(|hal_device| hal_device.raw_queue())
            };
            let Some(queue) = queue else {
                api.destroy_fence(fence, None);
                return Err(free("the device of the GPU modes is not one of Vulkan".to_owned()));
            };
            if let Err(error) = api.queue_submit(queue, &[submit], fence) {
                api.destroy_fence(fence, None);
                return Err(free(failed("vkQueueSubmit", error)));
            }
            transitions.pending.push((command_buffer, fence));
        }
        Ok(())
    }

    /// The buffer `fd` as a texture of the device.
    fn import(
        &self,
        fd: i32,
        (width, height): (u32, u32),
        drm_format: u32,
        modifier: u64,
        (offset, stride): (u32, u32),
    ) -> Result<Arc<VelloDeviceTexture>, String> {
        let (format, wgpu_format) = match drm_format {
            DRM_XR24 => (vk::Format::B8G8R8A8_UNORM, wgpu::TextureFormat::Bgra8Unorm),
            DRM_XB24 => (vk::Format::R8G8B8A8_UNORM, wgpu::TextureFormat::Rgba8Unorm),
            other => return Err(format!("the DRM format {other:#x} is not imported")),
        };
        if !self.formats.contains(&PlatformGraphicsDrmFormat::new(drm_format, modifier)) {
            return Err(format!("the modifier {modifier:#x} is not sampled by this device"));
        }
        let api = &self.api;
        let handle_type = vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT;
        let failed = |what: &str, error: vk::Result| format!("{what} failed: {error:?}");
        // SAFETY: `fd` is a DMA-BUF the lease of the frame keeps open while it is duplicated; the
        // duplicate is given to Vulkan only by a successful allocation. Each object made here is
        // destroyed on the failure that follows it, or by the drop callback of the texture.
        unsafe {
            let owned = BorrowedFd::borrow_raw(fd).try_clone_to_owned().map_err(|error| error.to_string())?;
            let mut fd_properties = vk::MemoryFdPropertiesKHR::default();
            self.memory_fd
                .get_memory_fd_properties(handle_type, owned.as_raw_fd(), &mut fd_properties)
                .map_err(|error| failed("vkGetMemoryFdPropertiesKHR", error))?;
            let planes = [vk::SubresourceLayout::default().offset(u64::from(offset)).row_pitch(u64::from(stride))];
            let mut explicit = vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
                .drm_format_modifier(modifier)
                .plane_layouts(&planes);
            let mut external = vk::ExternalMemoryImageCreateInfo::default().handle_types(handle_type);
            let info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(format)
                .extent(vk::Extent3D { width, height, depth: 1 })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT)
                .usage(vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .push_next(&mut explicit)
                .push_next(&mut external);
            let image = api.create_image(&info, None).map_err(|error| failed("vkCreateImage", error))?;
            let requirements = api.get_image_memory_requirements(image);
            let bits = requirements.memory_type_bits & fd_properties.memory_type_bits;
            if bits == 0 {
                api.destroy_image(image, None);
                return Err("the buffer has no memory type the image can be bound to".to_owned());
            }
            let mut import = vk::ImportMemoryFdInfoKHR::default().handle_type(handle_type).fd(owned.as_raw_fd());
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
            let allocate = vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(bits.trailing_zeros())
                .push_next(&mut import)
                .push_next(&mut dedicated);
            let memory = match api.allocate_memory(&allocate, None) {
                Ok(memory) => memory,
                Err(error) => {
                    api.destroy_image(image, None);
                    return Err(failed("vkAllocateMemory", error));
                }
            };
            // The allocation owns the descriptor now.
            let _ = owned.into_raw_fd();
            let bound = api.bind_image_memory(image, memory, 0).map_err(|error| failed("vkBindImageMemory", error));
            let transitioned = bound.and_then(|()| self.acquire_from_foreign(image));
            if let Err(error) = transitioned {
                api.destroy_image(image, None);
                api.free_memory(memory, None);
                return Err(error);
            }
            let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
            let hal_descriptor = hal::TextureDescriptor {
                label: Some("FerroUI Vello shared frame"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu_format,
                usage: wgpu::TextureUses::RESOURCE,
                memory_flags: hal::MemoryFlags::empty(),
                view_formats: Vec::new(),
            };
            let release = api.clone();
            let hal_texture = {
                let Some(hal_device) = self.device.device().as_hal::<hal::api::Vulkan>() else {
                    // The transition may still run: the device is lost or replaced with its objects.
                    return Err("the device of the GPU modes is not one of Vulkan".to_owned());
                };
                hal_device.texture_from_raw(
                    image,
                    &hal_descriptor,
                    Some(Box::new(move || {
                        release.destroy_image(image, None);
                        release.free_memory(memory, None);
                    })),
                    hal::vulkan::TextureMemory::External,
                )
            };
            let descriptor = wgpu::TextureDescriptor {
                label: Some("FerroUI Vello shared frame"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu_format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            };
            let texture = self.device.device().create_texture_from_hal::<hal::api::Vulkan>(
                hal_texture,
                &descriptor,
                wgpu::TextureUses::RESOURCE,
            );
            Ok(VelloDeviceTexture::from_external(&self.device, texture, VelloTextureAlpha::Premultiplied))
        }
    }
}

impl Drop for VelloDmaBufExternalObjectsFeature {
    fn drop(&mut self) {
        let transitions = self.transitions.get_mut().unwrap_or_else(|error| error.into_inner());
        // SAFETY: the objects are this feature's; each is waited for before it goes.
        unsafe {
            for (_, fence) in transitions.pending.drain(..) {
                let _ = self.api.wait_for_fences(&[fence], true, u64::MAX);
                self.api.destroy_fence(fence, None);
            }
            self.api.destroy_command_pool(transitions.pool, None);
        }
    }
}

/// The explicit single-plane modifiers `format` is sampled with.
///
/// # Safety
/// `physical` is a physical device of `instance`, with the extension of the modifiers.
unsafe fn sampled_modifiers(instance: &ash::Instance, physical: vk::PhysicalDevice, format: vk::Format) -> Vec<u64> {
    let mut list = vk::DrmFormatModifierPropertiesListEXT::default();
    let mut properties = vk::FormatProperties2::default().push_next(&mut list);
    // SAFETY: as the function says; the first call asks for the count.
    unsafe { instance.get_physical_device_format_properties2(physical, format, &mut properties) };
    let count = list.drm_format_modifier_count as usize;
    let mut modifiers = vec![vk::DrmFormatModifierPropertiesEXT::default(); count];
    let mut list = vk::DrmFormatModifierPropertiesListEXT::default().drm_format_modifier_properties(&mut modifiers);
    let mut properties = vk::FormatProperties2::default().push_next(&mut list);
    // SAFETY: the list points to `count` elements that live for the call.
    unsafe { instance.get_physical_device_format_properties2(physical, format, &mut properties) };
    modifiers
        .iter()
        .filter(|entry| {
            entry.drm_format_modifier_plane_count == 1
                && entry.drm_format_modifier != 0x00ff_ffff_ffff_ffff
                && entry.drm_format_modifier_tiling_features.contains(vk::FormatFeatureFlags::SAMPLED_IMAGE)
        })
        .map(|entry| entry.drm_format_modifier)
        .collect()
}

/// Reports once that a buffer was not imported: its frames are refused.
fn report_import_failure(error: &str) {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
        logger.log_with_values(None, "The Vello backend could not import a DMA-BUF frame: {Error}", &[&error]);
    }
}

/// A frame's buffer imported as a texture of the device.
pub struct VelloDmaBufImportedImage {
    device: Arc<VelloWgpuDevice>,
    generation: u64,
    uuid: Vec<u8>,
    /// The wrapped texture, or why the buffer could not be imported.
    texture: Result<Arc<VelloDeviceTexture>, String>,
    format: PlatformGraphicsExternalImageFormat,
}

impl IPlatformRenderInterfaceImportedObject for VelloDmaBufImportedImage {
    /// The buffer is the producer's, and the image goes with the texture.
    fn dispose(&self) {}
}

impl IPlatformRenderInterfaceImportedImage for VelloDmaBufImportedImage {
    fn acquire_shared_bitmap(&self, frame: FrameLease) -> Result<Arc<SharedBitmapImpl>, FrameError> {
        let m = frame.metadata();
        m.validate(MAX_FRAME_SIDE)?;
        if self.device.is_lost() || !frame.is_current() || m.device_generation != self.generation {
            return Err(FrameError::StaleGeneration);
        }
        let texture = self.texture.as_ref().map_err(|_| FrameError::MissingImporter)?;
        if !m.top_left_origin || m.width != texture.width() || m.height != texture.height() {
            return Err(FrameError::InvalidDimensions);
        }
        if m.device != Some(DeviceIdentity::Uuid(self.uuid.clone())) {
            return Err(FrameError::DeviceMismatch);
        }
        match m.color {
            FrameColor::Srgb => {}
            FrameColor::Rec709Full if m.alpha == FrameAlpha::Opaque => {}
            _ => return Err(FrameError::UnsupportedColor),
        }
        let format_matches = matches!(
            (m.format, self.format),
            (FrameFormat::Bgra8, PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm)
                | (FrameFormat::Rgba8, PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm)
        );
        if !format_matches || m.alpha == FrameAlpha::Straight {
            return Err(FrameError::UnsupportedFormat);
        }
        frame.counters().gpu_import();
        Ok(Arc::new(VelloSharedFrameBitmap::new(texture.clone(), frame)))
    }

    /// # Panics
    /// Always: frames are shared, not copied.
    fn snapshot_with_keyed_mutex(&self, _acquire_index: u32, _release_index: u32) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: frames are shared, not copied.
    fn snapshot_with_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: frames are shared, not copied.
    fn snapshot_with_timeline_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _wait_for_value: u64,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_value: u64,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: frames are shared ([`acquire_shared_bitmap`](Self::acquire_shared_bitmap)).
    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }
}

impl IExternalObjectsRenderInterfaceContextFeature for VelloDmaBufExternalObjectsFeature {
    fn shared_context_generation(&self) -> u64 {
        self.generation
    }

    fn supported_image_handle_types(&self) -> Vec<String> {
        vec![HandleTypes::DMA_BUF_FILE_DESCRIPTOR.into()]
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    fn supported_dma_buf_formats(&self) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        Some(self.formats.clone())
    }

    /// A buffer that cannot be imported gives an image whose frames are refused.
    ///
    /// # Panics
    /// Panics for a handle that is not a DMA-BUF.
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        if handle.handle_descriptor() != Some(HandleTypes::DMA_BUF_FILE_DESCRIPTOR) {
            panic!("Importing this platform handle is not supported");
        }
        let (width, height) = (properties.width.max(0) as u32, properties.height.max(0) as u32);
        let texture = match &properties.dma_buf_properties {
            Some(dma) if dma.plane_count == 1 => {
                let plane = |values: &Option<Vec<u32>>| values.as_ref().and_then(|values| values.first().copied());
                match (plane(&dma.plane_offsets), plane(&dma.plane_strides)) {
                    (Some(offset), Some(stride)) => self.import(
                        handle.handle() as i32,
                        (width, height),
                        dma.drm_format,
                        dma.drm_modifier,
                        (offset, stride),
                    ),
                    _ => Err("the buffer has no plane layout".to_owned()),
                }
            }
            _ => Err("only single-plane buffers are imported".to_owned()),
        };
        if let Err(error) = &texture {
            report_import_failure(error);
        }
        Rc::new(VelloDmaBufImportedImage {
            device: self.device.clone(),
            generation: self.generation,
            uuid: self.uuid.clone(),
            texture,
            format: properties.format,
        })
    }

    /// # Panics
    /// Always: frames are imported by their buffer.
    fn import_shared_image(
        &self,
        _image: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: the buffer's implicit fences have signalled when the frame is handed over.
    fn import_semaphore(&self, _handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        panic!("Specified method is not supported.");
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        if image_handle_type == HandleTypes::DMA_BUF_FILE_DESCRIPTOR {
            return CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC;
        }
        CompositionGpuImportedImageSynchronizationCapabilities::default()
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        Some(self.uuid.clone())
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        None
    }
}
