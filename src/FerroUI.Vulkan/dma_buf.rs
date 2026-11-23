//! The import of DMA-BUF frames into the Vulkan device of a graphics context (#575, #608): what a
//! render backend over the contracts of this crate needs to sample the frames of a GStreamer
//! session on Linux.
//!
//! **Behind a switch.** `FERROUI_VULKAN_DMA_BUF=1` asks the device for [`DMA_BUF_DEVICE_EXTENSIONS`],
//! each only where the device has it, until a run in the Linux virtual machine has proved the
//! route. Without the switch, or where one is missing, [`VulkanDmaBufImporter::new`] gives `None`
//! and the device behaves as before.
//!
//! A plane is imported as a `VkImage` with its explicit DRM format modifier, in the foreign queue
//! family: the renderer takes it from there. An image a renderer may still read cannot be destroyed
//! when its last owner goes, so [`VulkanDmaBufPlanes`] hands it to the importer, which destroys it
//! once the queue has done everything submitted after the renderer's next flush
//! ([`VulkanDmaBufImporter::collect`]).
//!
//! This file has `unsafe` for the raw objects of Vulkan.

use crate::IVulkanPlatformGraphicsContext;
use ash::vk::{self, Handle};
use ferroui_base::platform::PlatformGraphicsDrmFormat;
use std::ffi::{c_void, CStr};
use std::os::fd::{AsRawFd, BorrowedFd, IntoRawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// `DRM_FORMAT_XRGB8888`: B, G, R, X in memory.
pub const DRM_XR24: u32 = 0x3432_5258;
/// `DRM_FORMAT_XBGR8888`: R, G, B, X in memory.
pub const DRM_XB24: u32 = 0x3432_4258;
/// `DRM_FORMAT_NV12`: a plane of luma and a plane of interleaved chroma at half the size.
pub const DRM_NV12: u32 = 0x3231_564e;

/// The device extensions of the import. All of them, or the route is not offered.
pub const DMA_BUF_DEVICE_EXTENSIONS: [&str; 5] = [
    "VK_KHR_external_memory_fd",
    "VK_EXT_external_memory_dma_buf",
    "VK_EXT_image_drm_format_modifier",
    "VK_EXT_queue_family_foreign",
    "VK_KHR_image_format_list",
];

/// Whether the route is turned on.
pub fn dma_buf_switched_on() -> bool {
    std::env::var("FERROUI_VULKAN_DMA_BUF").is_ok_and(|value| value == "1")
}

/// One plane of a buffer as an image of the device. Destroyed with its [`VulkanDmaBufPlanes`].
pub struct VulkanDmaBufPlane {
    api: ash::Device,
    image: vk::Image,
    memory: vk::DeviceMemory,
    allocation_size: u64,
    format: vk::Format,
    size: (u32, u32),
}

impl VulkanDmaBufPlane {
    /// The `VkImage`.
    pub fn image(&self) -> vk::Image {
        self.image
    }

    /// Its memory.
    pub fn memory(&self) -> vk::DeviceMemory {
        self.memory
    }

    /// The size of its memory in bytes.
    pub fn allocation_size(&self) -> u64 {
        self.allocation_size
    }

    /// Its format.
    pub fn format(&self) -> vk::Format {
        self.format
    }

    /// Its size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }
}

impl Drop for VulkanDmaBufPlane {
    fn drop(&mut self) {
        // SAFETY: the importer drops a plane once the queue has done what may read it.
        unsafe {
            self.api.destroy_image(self.image, None);
            self.api.free_memory(self.memory, None);
        }
    }
}

/// What the planes of the buffers go to when their last owner goes.
#[derive(Default)]
struct Retired {
    planes: Mutex<Vec<Vec<VulkanDmaBufPlane>>>,
    /// The device is gone or going: its objects are left to it.
    closed: AtomicBool,
}

/// The planes of one buffer. When the last owner goes they wait in the importer for the queue.
pub struct VulkanDmaBufPlanes {
    planes: Vec<VulkanDmaBufPlane>,
    retired: Arc<Retired>,
}

impl VulkanDmaBufPlanes {
    /// The planes, in the order of the format.
    pub fn planes(&self) -> &[VulkanDmaBufPlane] {
        &self.planes
    }
}

impl Drop for VulkanDmaBufPlanes {
    fn drop(&mut self) {
        let planes = std::mem::take(&mut self.planes);
        if self.retired.closed.load(Ordering::Acquire) {
            // Destroying them would call into a device that has been destroyed.
            std::mem::forget(planes);
            return;
        }
        self.retired.planes.lock().unwrap_or_else(|error| error.into_inner()).push(planes);
    }
}

/// Planes that wait for the queue.
struct Waiting {
    planes: Vec<VulkanDmaBufPlane>,
    /// The flush count of the renderer when they were found retired.
    at_flush: u64,
    /// Signalled when the queue has done everything submitted before it.
    fence: Option<vk::Fence>,
}

/// The import of DMA-BUF planes into the device of a context.
pub struct VulkanDmaBufImporter {
    context: std::rc::Rc<dyn IVulkanPlatformGraphicsContext>,
    api: ash::Device,
    get_memory_fd_properties: vk::PFN_vkGetMemoryFdPropertiesKHR,
    formats: Vec<PlatformGraphicsDrmFormat>,
    uuid: Vec<u8>,
    retired: Arc<Retired>,
    waiting: Mutex<Vec<Waiting>>,
}

/// The explicit single-plane modifiers `format` is sampled with.
///
/// # Safety
/// `get` is `vkGetPhysicalDeviceFormatProperties2` of the instance of `physical`.
unsafe fn sampled_modifiers(
    get: vk::PFN_vkGetPhysicalDeviceFormatProperties2,
    physical: vk::PhysicalDevice,
    format: vk::Format,
) -> Vec<u64> {
    let mut list = vk::DrmFormatModifierPropertiesListEXT::default();
    let mut properties = vk::FormatProperties2::default().push_next(&mut list);
    // SAFETY: as the function says; the first call asks for the count.
    unsafe { get(physical, format, &mut properties) };
    let count = list.drm_format_modifier_count as usize;
    let mut modifiers = vec![vk::DrmFormatModifierPropertiesEXT::default(); count];
    let mut list = vk::DrmFormatModifierPropertiesListEXT::default().drm_format_modifier_properties(&mut modifiers);
    let mut properties = vk::FormatProperties2::default().push_next(&mut list);
    // SAFETY: the list points to `count` elements that live for the call.
    unsafe { get(physical, format, &mut properties) };
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

impl VulkanDmaBufImporter {
    /// The importer of the device of `context`, when the route is turned on and the device has
    /// every extension and samples at least one layout.
    pub fn new(context: std::rc::Rc<dyn IVulkanPlatformGraphicsContext>) -> Option<Self> {
        if !dma_buf_switched_on() {
            return None;
        }
        let device = context.device();
        let enabled = device.enabled_extensions();
        if !DMA_BUF_DEVICE_EXTENSIONS.iter().all(|name| enabled.iter().any(|found| found == name)) {
            return None;
        }
        let instance = device.instance();
        let (instance_handle, device_handle) = (instance.handle(), device.handle());
        let physical = vk::PhysicalDevice::from_raw(device.physical_device_handle() as u64);
        let device_proc = |name: &str| instance.get_device_proc_address(device_handle, name) as *const c_void;
        let instance_proc = |name: &str| instance.get_instance_proc_address(instance_handle, name) as *const c_void;
        let (fd_properties, format_properties, id_properties) = (
            device_proc("vkGetMemoryFdPropertiesKHR"),
            instance_proc("vkGetPhysicalDeviceFormatProperties2"),
            instance_proc("vkGetPhysicalDeviceProperties2"),
        );
        if fd_properties.is_null() || format_properties.is_null() || id_properties.is_null() {
            return None;
        }
        // SAFETY: the addresses of the entry points of those names, with their signatures.
        let (get_memory_fd_properties, get_format, get_properties) = unsafe {
            (
                std::mem::transmute::<*const c_void, vk::PFN_vkGetMemoryFdPropertiesKHR>(fd_properties),
                std::mem::transmute::<*const c_void, vk::PFN_vkGetPhysicalDeviceFormatProperties2>(format_properties),
                std::mem::transmute::<*const c_void, vk::PFN_vkGetPhysicalDeviceProperties2>(id_properties),
            )
        };
        let mut id = vk::PhysicalDeviceIDProperties::default();
        {
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut id);
            // SAFETY: a valid handle, and a structure the call writes.
            unsafe { get_properties(physical, &mut properties) };
        }
        let mut formats = Vec::new();
        // SAFETY: `get_format` is the entry point of this instance.
        unsafe {
            for (drm, format) in [(DRM_XR24, vk::Format::B8G8R8A8_UNORM), (DRM_XB24, vk::Format::R8G8B8A8_UNORM)] {
                for modifier in sampled_modifiers(get_format, physical, format) {
                    formats.push(PlatformGraphicsDrmFormat::new(drm, modifier));
                }
            }
            let chroma = sampled_modifiers(get_format, physical, vk::Format::R8G8_UNORM);
            for modifier in sampled_modifiers(get_format, physical, vk::Format::R8_UNORM) {
                if chroma.contains(&modifier) {
                    formats.push(PlatformGraphicsDrmFormat::new(DRM_NV12, modifier));
                }
            }
        }
        if formats.is_empty() {
            return None;
        }
        // SAFETY: the entry points of the device, resolved by name from its instance.
        let api = unsafe {
            ash::Device::load_with(
                |name: &CStr| {
                    name.to_str().map_or(std::ptr::null(), |name| {
                        instance.get_device_proc_address(device_handle, name) as *const c_void
                    })
                },
                vk::Device::from_raw(device_handle as u64),
            )
        };
        Some(Self {
            context,
            api,
            get_memory_fd_properties,
            formats,
            uuid: id.device_uuid.to_vec(),
            retired: Arc::new(Retired::default()),
            waiting: Mutex::new(Vec::new()),
        })
    }

    /// The format and modifier pairs of the device.
    pub fn formats(&self) -> &[PlatformGraphicsDrmFormat] {
        &self.formats
    }

    /// The UUID of the device, which names it in the frames of a producer.
    pub fn device_uuid(&self) -> &[u8] {
        &self.uuid
    }

    /// Imports one buffer: one plane for XRGB and XBGR, luma (`R8`) and chroma (`R8G8`) for NV12.
    /// `planes` are the descriptor, byte offset and row pitch of each.
    pub fn import(
        &self,
        drm_format: u32,
        modifier: u64,
        (width, height): (u32, u32),
        planes: &[(i32, u32, u32)],
    ) -> Result<Arc<VulkanDmaBufPlanes>, String> {
        if !self.formats.contains(&PlatformGraphicsDrmFormat::new(drm_format, modifier)) {
            return Err(format!("the format {drm_format:#x} with modifier {modifier:#x} is not sampled by this device"));
        }
        let layout: &[(vk::Format, (u32, u32))] = match drm_format {
            DRM_XR24 => &[(vk::Format::B8G8R8A8_UNORM, (width, height))],
            DRM_XB24 => &[(vk::Format::R8G8B8A8_UNORM, (width, height))],
            DRM_NV12 => &[
                (vk::Format::R8_UNORM, (width, height)),
                (vk::Format::R8G8_UNORM, (width.div_ceil(2), height.div_ceil(2))),
            ],
            other => return Err(format!("the DRM format {other:#x} is not imported")),
        };
        if planes.len() != layout.len() {
            return Err("the buffer has another number of planes than its format".to_owned());
        }
        let mut imported = Vec::with_capacity(planes.len());
        for (&(fd, offset, stride), &(format, size)) in planes.iter().zip(layout) {
            imported.push(self.import_plane(fd, format, size, modifier, offset, stride)?);
        }
        Ok(Arc::new(VulkanDmaBufPlanes { planes: imported, retired: self.retired.clone() }))
    }

    fn import_plane(
        &self,
        fd: i32,
        format: vk::Format,
        (width, height): (u32, u32),
        modifier: u64,
        offset: u32,
        stride: u32,
    ) -> Result<VulkanDmaBufPlane, String> {
        let api = &self.api;
        let handle_type = vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT;
        let failed = |what: &str, error: vk::Result| format!("{what} failed: {error:?}");
        // SAFETY: `fd` is a DMA-BUF the caller keeps open for the call; its duplicate is given to
        // Vulkan only by a successful allocation. Each object is destroyed on a later failure.
        unsafe {
            let owned = BorrowedFd::borrow_raw(fd).try_clone_to_owned().map_err(|error| error.to_string())?;
            let mut fd_properties = vk::MemoryFdPropertiesKHR::default();
            (self.get_memory_fd_properties)(api.handle(), handle_type, owned.as_raw_fd(), &mut fd_properties)
                .result()
                .map_err(|error| failed("vkGetMemoryFdPropertiesKHR", error))?;
            let layouts = [vk::SubresourceLayout::default().offset(u64::from(offset)).row_pitch(u64::from(stride))];
            let mut explicit = vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
                .drm_format_modifier(modifier)
                .plane_layouts(&layouts);
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
            if let Err(error) = api.bind_image_memory(image, memory, 0) {
                api.destroy_image(image, None);
                api.free_memory(memory, None);
                return Err(failed("vkBindImageMemory", error));
            }
            Ok(VulkanDmaBufPlane {
                api: api.clone(),
                image,
                memory,
                allocation_size: requirements.size,
                format,
                size: (width, height),
            })
        }
    }

    /// Destroys the planes the queue has finished with. `flush_count` is the number of flushes of
    /// the renderer: planes retired since its last flush may be read by a recording not yet
    /// submitted, so they wait for the next one. Takes the device for the empty submission.
    pub fn collect(&self, flush_count: u64) {
        let mut waiting = self.waiting.lock().unwrap_or_else(|error| error.into_inner());
        let retired = std::mem::take(&mut *self.retired.planes.lock().unwrap_or_else(|error| error.into_inner()));
        waiting.extend(retired.into_iter().map(|planes| Waiting { planes, at_flush: flush_count, fence: None }));
        if waiting.is_empty() {
            return;
        }
        let device = self.context.device();
        let lock = device.lock();
        let queue = vk::Queue::from_raw(device.main_queue_handle() as u64);
        let api = &self.api;
        waiting.retain_mut(|entry| {
            // SAFETY: the fence is this importer's, and the device is locked for the queue.
            unsafe {
                if entry.fence.is_none() && flush_count > entry.at_flush {
                    if let Ok(fence) = api.create_fence(&vk::FenceCreateInfo::default(), None) {
                        if api.queue_submit(queue, &[], fence).is_ok() {
                            entry.fence = Some(fence);
                        } else {
                            api.destroy_fence(fence, None);
                        }
                    }
                }
                match entry.fence {
                    Some(fence) if !matches!(api.get_fence_status(fence), Ok(false)) => {
                        api.destroy_fence(fence, None);
                        false
                    }
                    _ => true,
                }
            }
        });
        lock.dispose();
    }

    /// Leaves every plane to a device that was lost: nothing more is called on it.
    pub fn abandon(&self) {
        self.retired.closed.store(true, Ordering::Release);
        let mut waiting = self.waiting.lock().unwrap_or_else(|error| error.into_inner());
        for entry in waiting.drain(..) {
            std::mem::forget(entry.planes);
        }
        for planes in self.retired.planes.lock().unwrap_or_else(|error| error.into_inner()).drain(..) {
            std::mem::forget(planes);
        }
    }

    /// Waits for the queue and destroys every plane that was retired: the renderer is going. A
    /// plane whose last owner goes later is left to the device.
    pub fn release_all(&self) {
        self.retired.closed.store(true, Ordering::Release);
        let device = self.context.device();
        let lock = device.lock();
        let queue = vk::Queue::from_raw(device.main_queue_handle() as u64);
        // SAFETY: the device is locked for the queue.
        let _ = unsafe { self.api.queue_wait_idle(queue) };
        let mut waiting = self.waiting.lock().unwrap_or_else(|error| error.into_inner());
        for entry in waiting.drain(..) {
            if let Some(fence) = entry.fence {
                // SAFETY: the queue is idle, so the fence is not in use.
                unsafe { self.api.destroy_fence(fence, None) };
            }
        }
        self.retired.planes.lock().unwrap_or_else(|error| error.into_inner()).clear();
        lock.dispose();
    }
}
