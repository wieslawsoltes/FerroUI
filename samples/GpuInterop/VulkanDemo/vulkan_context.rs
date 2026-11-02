//! Port of `VulkanDemo/VulkanContext.cs`: the Vulkan instance and device the demo draws with,
//! chosen to match the device of the compositor and to have the extensions the sharing of
//! images with the compositor needs on this platform.
//!
//! Upstream holds the API object of Silk.NET (`Vk`), which loads the entry points of the
//! instance and of the device it was last used with. Here the loader, the instance and the
//! device are the three tables of ash ([`entry`](VulkanContext::entry),
//! [`instance`](VulkanContext::instance), [`device`](VulkanContext::device)).

use super::byte_string::ByteStringList;
use super::vulkan_command_buffer_pool::VulkanCommandBufferPool;
use super::vulkan_extensions::{ThrowOnError, VulkanDemoError, VulkanDemoResult};
use ash::vk;
use ferroui_base::platform::{KnownPlatformGraphicsExternalImageHandleTypes, KnownPlatformGraphicsExternalSemaphoreHandleTypes};
use ferroui_base::rendering::composition::ICompositionGpuInterop;
use std::cell::RefCell;
use std::ffi::{c_void, CStr};
use std::rc::Rc;

/// The Skia context over the device of the demo (`GRContext` upstream), which
/// [`VulkanImage::save_texture`](super::VulkanImage::save_texture) reads images back with. It
/// needs Skia with its Vulkan back end, which this build has on Linux, and on Windows with the
/// feature `vulkan` of the sample.
#[cfg(gpu_interop_skia_vulkan)]
pub type GrContext = skia_safe::gpu::DirectContext;

/// The Skia context over the device of the demo (`GRContext` upstream). This build has no Skia
/// with Vulkan, so there is never one: see the type of a build that has.
#[cfg(not(gpu_interop_skia_vulkan))]
pub enum GrContext {}

/// The Direct3D 11 device on the adapter of the Vulkan device, which makes the textures the
/// images of the demo are imported from where the compositor takes Direct3D 11 textures
/// (`ComPtr<ID3D11Device>` upstream).
#[cfg(windows)]
pub type D3DDevice = Option<ferroui_microcom::ComPtr<crate::d3d_demo::d3d11::ID3D11Device>>;

/// No Direct3D off Windows.
#[cfg(not(windows))]
pub type D3DDevice = ();

pub struct VulkanContext {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    queue_family_index: u32,
    pool: VulkanCommandBufferPool,
    gr_context: RefCell<Option<GrContext>>,
    descriptor_pool: vk::DescriptorPool,
    d3d_device: RefCell<D3DDevice>,
    /// The extensions the device was created with: what `TryGetDeviceExtension` of Silk.NET
    /// checks before it loads the entry points of an extension.
    enabled_device_extensions: Vec<String>,
}

/// What `TryCreate` has created so far, released by its `finally` unless the context was made.
#[derive(Default)]
struct Partial {
    device: Option<ash::Device>,
    descriptor_pool: vk::DescriptorPool,
    pool: Option<VulkanCommandBufferPool>,
}

impl Partial {
    fn release(self) {
        if let Some(pool) = &self.pool {
            pool.dispose();
        }
        if let Some(device) = &self.device {
            // SAFETY: the pool of descriptors and the device are not used by anything else.
            unsafe {
                if self.descriptor_pool != vk::DescriptorPool::null() {
                    device.destroy_descriptor_pool(self.descriptor_pool, None);
                }
                device.destroy_device(None);
            }
        }
    }
}

const MAC_VULKAN_SDK_GLOBAL_PATH: &str = "/usr/local/lib/libvulkan.dylib";

/// Where Homebrew installs the loader on Apple silicon (`vulkan-loader` with `molten-vk`); not
/// upstream's, which knows the path of the Vulkan SDK only.
const MAC_HOMEBREW_VULKAN_PATH: &str = "/opt/homebrew/lib/libvulkan.dylib";

fn contains(list: &[String], value: &str) -> bool {
    list.iter().any(|item| item == value)
}

impl VulkanContext {
    pub fn entry(&self) -> &ash::Entry {
        &self.entry
    }

    pub fn instance(&self) -> &ash::Instance {
        &self.instance
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    pub fn device(&self) -> &ash::Device {
        &self.device
    }

    pub fn queue(&self) -> vk::Queue {
        self.queue
    }

    pub fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }

    pub fn pool(&self) -> &VulkanCommandBufferPool {
        &self.pool
    }

    pub fn gr_context(&self) -> &RefCell<Option<GrContext>> {
        &self.gr_context
    }

    pub fn descriptor_pool(&self) -> vk::DescriptorPool {
        self.descriptor_pool
    }

    pub fn d3d_device(&self) -> std::cell::Ref<'_, D3DDevice> {
        self.d3d_device.borrow()
    }

    /// Whether the device was created with the extension `name` (`TryGetDeviceExtension`).
    pub fn has_device_extension(&self, name: &CStr) -> bool {
        name.to_str().is_ok_and(|name| contains(&self.enabled_device_extensions, name))
    }

    /// Creates the context for the compositor `gpu_interop`: `None` with the reason when the
    /// compositor cannot take the images of a Vulkan device of this platform or there is no
    /// such device, the name of the device otherwise. With `dma_buf` the images are shared as
    /// Linux dma-bufs.
    pub fn try_create(gpu_interop: &dyn ICompositionGpuInterop, dma_buf: bool) -> (Option<Rc<VulkanContext>>, String) {
        let app_name = c"GpuInterop";
        let _engine_name = c"Test";
        // Upstream names the engine with the name of the application, too.
        let application_info = vk::ApplicationInfo::default()
            .application_name(app_name)
            .api_version(vk::make_api_version(0, 1, 1, 0))
            .engine_name(app_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .application_version(vk::make_api_version(0, 1, 0, 0));

        let mut enabled_extensions = vec!["VK_KHR_get_physical_device_properties2".to_string()];
        if cfg!(target_os = "macos") {
            enabled_extensions.push("VK_KHR_portability_enumeration".to_string());
        }

        if !cfg!(target_os = "macos") {
            enabled_extensions
                .extend(["VK_KHR_external_memory_capabilities".to_string(), "VK_KHR_external_semaphore_capabilities".to_string()]);
        }

        let mut enabled_layers: Vec<String> = Vec::new();

        // Upstream these two are outside the `try` of the method: what they throw reaches the
        // demo, which shows it, as the text returned here is shown.
        let api = match get_api() {
            Ok(api) => api,
            Err(error) => return (None, error.to_string()),
        };
        enabled_extensions.push("VK_EXT_debug_utils".to_string());
        match is_layer_available(&api, "VK_LAYER_KHRONOS_validation") {
            Ok(true) => enabled_layers.push("VK_LAYER_KHRONOS_validation".to_string()),
            Ok(false) => {}
            Err(error) => return (None, error.to_string()),
        }

        let mut partial = Partial::default();
        let result = Self::try_create_core(
            &api,
            &application_info,
            &enabled_extensions,
            &mut enabled_layers,
            gpu_interop,
            dma_buf,
            &mut partial,
        );
        match result {
            Ok((Some(context), info)) => (Some(Rc::new(context)), info),
            Ok((None, info)) => {
                partial.release();
                (None, info)
            }
            Err(error) => {
                partial.release();
                (None, error.to_string())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn try_create_core(
        api: &ash::Entry,
        application_info: &vk::ApplicationInfo<'_>,
        enabled_extensions: &[String],
        enabled_layers: &mut Vec<String>,
        gpu_interop: &dyn ICompositionGpuInterop,
        dma_buf: bool,
        partial: &mut Partial,
    ) -> VulkanDemoResult<(Option<VulkanContext>, String)> {
        enabled_layers.clear();
        let p_required_extensions = ByteStringList::new(enabled_extensions);
        let p_enabled_layers = ByteStringList::new(enabled_layers.iter());

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(application_info)
            .enabled_extension_names(p_required_extensions.as_ptrs())
            .enabled_layer_names(p_enabled_layers.as_ptrs())
            .flags(if cfg!(target_os = "macos") {
                vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
            } else {
                vk::InstanceCreateFlags::empty()
            });

        // SAFETY: the structure and the strings it points to live for the call.
        let vk_instance = unsafe { api.create_instance(&instance_create_info, None) }.throw_on_error()?;

        if contains(enabled_extensions, "VK_EXT_debug_utils") {
            let debug_utils = ash::ext::debug_utils::Instance::new(api, &vk_instance);
            let debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(log_callback));

            // Upstream keeps neither the messenger nor the result.
            // SAFETY: the callback is a function of the program and needs no user data.
            let _ = unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None) };
        }

        let mut require_device_extensions: Vec<String> = Vec::new();
        if !cfg!(target_os = "macos") {
            require_device_extensions.extend(["VK_KHR_external_memory".to_string(), "VK_KHR_external_semaphore".to_string()]);
        }

        let supported_image_handle_types = gpu_interop.supported_image_handle_types();
        if cfg!(windows) {
            if !(contains(
                &supported_image_handle_types,
                KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE,
            ) || contains(&supported_image_handle_types, KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_NT_HANDLE))
            {
                return Ok((None, "Image sharing is not supported by the current backend".to_string()));
            }
            require_device_extensions.push(ext_name(ash::khr::external_memory_win32::NAME));
            require_device_extensions.push(ext_name(ash::khr::external_semaphore_win32::NAME));
            require_device_extensions.push("VK_KHR_dedicated_allocation".to_string());
            require_device_extensions.push("VK_KHR_get_memory_requirements2".to_string());
        } else if cfg!(target_os = "macos") {
            if !contains(&supported_image_handle_types, KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF) {
                return Ok((None, "Image sharing is not supported by the current backend".to_string()));
            }
            require_device_extensions.extend(["VK_EXT_metal_objects".to_string(), "VK_KHR_timeline_semaphore".to_string()]);
        } else if dma_buf {
            if !contains(&supported_image_handle_types, KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR) {
                return Ok((None, "dma-buf image sharing is not supported by the current backend".to_string()));
            }
            // VK_KHR_external_memory_fd is needed to obtain the dma-buf fd, VK_EXT_external_memory_dma_buf
            // marks the allocation as dma-buf exportable and VK_EXT_image_drm_format_modifier lets us
            // create the image with an explicit DRM tiling layout that the EGL importer understands.
            // (bind_memory2/get_memory_requirements2/image_format_list dependencies are core in Vulkan 1.1.)
            require_device_extensions.push(ext_name(ash::khr::external_memory_fd::NAME));
            require_device_extensions.push("VK_EXT_external_memory_dma_buf".to_string());
            require_device_extensions.push("VK_EXT_image_drm_format_modifier".to_string());
        } else {
            if !contains(
                &supported_image_handle_types,
                KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR,
            ) || !contains(
                &gpu_interop.supported_semaphore_types(),
                KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR,
            ) {
                return Ok((None, "Image sharing is not supported by the current backend".to_string()));
            }
            require_device_extensions.push(ext_name(ash::khr::external_memory_fd::NAME));
            require_device_extensions.push(ext_name(ash::khr::external_semaphore_fd::NAME));
        }

        // SAFETY: the instance is alive.
        let physical_devices = unsafe { vk_instance.enumerate_physical_devices() }.throw_on_error()?;

        for &physical_device in &physical_devices {
            if require_device_extensions
                .iter()
                .any(|ext| !is_device_extension_present(&vk_instance, physical_device, ext))
            {
                continue;
            }

            let mut physical_device_id_properties = vk::PhysicalDeviceIDProperties::default();
            let properties = {
                let mut physical_device_properties2 =
                    vk::PhysicalDeviceProperties2::default().push_next(&mut physical_device_id_properties);
                // SAFETY: the physical device belongs to the instance, and the chain is valid.
                unsafe { vk_instance.get_physical_device_properties2(physical_device, &mut physical_device_properties2) };
                physical_device_properties2.properties
            };

            let device_luid = gpu_interop.device_luid();
            let device_uuid = gpu_interop.device_uuid();
            if let (Some(luid), true) = (&device_luid, physical_device_id_properties.device_luid_valid == vk::TRUE) {
                if physical_device_id_properties.device_luid[..] != luid[..] {
                    continue;
                }
            } else if let Some(uuid) = &device_uuid {
                if physical_device_id_properties.device_uuid[..] != uuid[..] {
                    continue;
                }
            }

            let name = properties.device_name_as_c_str().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();

            // SAFETY: the physical device belongs to the instance.
            let family_properties =
                unsafe { vk_instance.get_physical_device_queue_family_properties(physical_device) };
            for (queue_family_index, family) in family_properties.iter().enumerate() {
                let queue_family_index = queue_family_index as u32;
                if !family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    continue;
                }

                let queue_priorities = vec![1f32; family.queue_count as usize];

                let features = vk::PhysicalDeviceFeatures::default();

                let queue_create_info = vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(&queue_priorities);

                let p_enabled_device_extensions = ByteStringList::new(&require_device_extensions);
                let queue_create_infos = [queue_create_info];
                let device_create_info = vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queue_create_infos)
                    .enabled_extension_names(p_enabled_device_extensions.as_ptrs())
                    .enabled_features(&features);

                // SAFETY: the structure and everything it points to live for the call.
                let device = unsafe { vk_instance.create_device(physical_device, &device_create_info, None) }
                    .throw_on_error()?;
                partial.device = Some(device.clone());

                // SAFETY: the device has a queue of the family.
                let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

                let descriptor_pool_size =
                    vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER, descriptor_count: 16 };
                let pool_sizes = [descriptor_pool_size];
                let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
                    .pool_sizes(&pool_sizes)
                    .max_sets(16)
                    .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

                // SAFETY: the structure is valid.
                let descriptor_pool =
                    unsafe { device.create_descriptor_pool(&descriptor_pool_info, None) }.throw_on_error()?;
                partial.descriptor_pool = descriptor_pool;

                let pool = VulkanCommandBufferPool::new(&device, queue, queue_family_index)?;
                partial.pool = Some(pool.clone());
                let mut gr_context: Option<GrContext> = None;
                if !cfg!(target_os = "macos") {
                    gr_context = create_gr_context(api, &vk_instance, physical_device, &device, queue, queue_family_index);
                    if gr_context.is_none() && cfg!(gpu_interop_skia_vulkan) {
                        return Ok((None, "Can't create Skia GrContext, device is likely broken".to_string()));
                    }
                }

                let d3d_device = create_d3d_device(&physical_device_id_properties, &supported_image_handle_types)?;
                // The context owns what was created from here on.
                *partial = Partial::default();
                return Ok((
                    Some(VulkanContext {
                        entry: api.clone(),
                        device,
                        instance: vk_instance,
                        physical_device,
                        queue,
                        queue_family_index,
                        pool,
                        descriptor_pool,
                        gr_context: RefCell::new(gr_context),
                        d3d_device: RefCell::new(d3d_device),
                        enabled_device_extensions: require_device_extensions,
                    }),
                    name,
                ));
            }
            return Ok((None, "No suitable device queue found".to_string()));
        }

        Ok((None, "Suitable device not found".to_string()))
    }

    pub fn dispose(&self) {
        let _ = self.d3d_device.take();
        drop(self.gr_context.take());
        self.pool.dispose();
        // SAFETY: nothing of the device is used any more.
        unsafe {
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_device(None);
        }
    }
}

fn ext_name(name: &CStr) -> String {
    name.to_string_lossy().into_owned()
}

/// `IsDeviceExtensionPresent` of Silk.NET.
fn is_device_extension_present(instance: &ash::Instance, physical_device: vk::PhysicalDevice, extension: &str) -> bool {
    // SAFETY: the physical device belongs to the instance.
    let properties = unsafe { instance.enumerate_device_extension_properties(physical_device) }.unwrap_or_default();
    properties
        .iter()
        .any(|p| p.extension_name_as_c_str().is_ok_and(|name| name.to_bytes() == extension.as_bytes()))
}

fn is_layer_available(api: &ash::Entry, layer_name: &str) -> VulkanDemoResult<bool> {
    // SAFETY: the call has no preconditions.
    let layer_properties = unsafe { api.enumerate_instance_layer_properties() }.throw_on_error()?;

    for properties in &layer_properties {
        let current_layer_name = properties.layer_name_as_c_str().ok().map(CStr::to_string_lossy);

        if current_layer_name.as_deref() == Some(layer_name) {
            return Ok(true);
        }
    }

    Ok(false)
}

unsafe extern "system" fn log_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    if message_severity != vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE && !p_callback_data.is_null() {
        // SAFETY: the implementation passes valid callback data with a zero-terminated message.
        let message = unsafe { (*p_callback_data).p_message };
        if !message.is_null() {
            // SAFETY: as above.
            println!("{}", unsafe { CStr::from_ptr(message) }.to_string_lossy());
        }
    }

    vk::FALSE
}

/// The loader of Vulkan: on macOS the one of the Vulkan SDK (or of Homebrew) when it is
/// installed, otherwise the one the system finds.
fn get_api() -> VulkanDemoResult<ash::Entry> {
    let load = |path: Option<&str>| {
        // SAFETY: loading the Vulkan loader runs its initialization, which has no preconditions.
        let entry = unsafe {
            match path {
                Some(path) => ash::Entry::load_from(path),
                None => ash::Entry::load(),
            }
        };
        entry.map_err(|error| VulkanDemoError::new(format!("Unable to load the Vulkan loader: {error}")))
    };
    if cfg!(target_os = "macos") {
        for path in [MAC_VULKAN_SDK_GLOBAL_PATH, MAC_HOMEBREW_VULKAN_PATH] {
            if std::path::Path::new(path).exists() {
                return load(Some(path));
            }
        }
    }
    load(None)
}

/// `GRContext.CreateVulkan` over the device, with the entry points looked up on the device, then
/// on the instance, then without an instance.
#[cfg(gpu_interop_skia_vulkan)]
fn create_gr_context(
    entry: &ash::Entry,
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: &ash::Device,
    queue: vk::Queue,
    queue_family_index: u32,
) -> Option<GrContext> {
    use ash::vk::Handle;
    use skia_safe::gpu::vk::{BackendContext, GetProcOf};

    let device_handle = device.handle();
    let instance_handle = instance.handle();
    let get_proc = |of: GetProcOf| -> *const c_void {
        let name = match of {
            GetProcOf::Instance(_, name) | GetProcOf::Device(_, name) => name,
        };
        // SAFETY: Skia passes the name of an entry point as a zero-terminated string that lives
        // for the call; the handles are those of the live device and instance.
        unsafe {
            if let Some(rv) = instance.get_device_proc_addr(device_handle, name) {
                return rv as *const c_void;
            }
            if let Some(rv) = entry.get_instance_proc_addr(instance_handle, name) {
                return rv as *const c_void;
            }
            match entry.get_instance_proc_addr(vk::Instance::null(), name) {
                Some(rv) => rv as *const c_void,
                None => std::ptr::null(),
            }
        }
    };
    // SAFETY: the handles are those of the device of the context, which outlives the Skia
    // context: `VulkanContext::dispose` releases Skia's first.
    let backend_context = unsafe {
        BackendContext::new_builder(
            instance_handle.as_raw() as usize as _,
            physical_device.as_raw() as usize as _,
            device_handle.as_raw() as usize as _,
            (queue.as_raw() as usize as _, queue_family_index as usize),
            &get_proc,
            // Upstream gives no version either.
            None,
        )
        .build()
    };
    skia_safe::gpu::direct_contexts::make_vulkan(&backend_context, None)
}

/// No Skia with Vulkan in this build: there is no context to create.
#[cfg(not(gpu_interop_skia_vulkan))]
fn create_gr_context(
    _entry: &ash::Entry,
    _instance: &ash::Instance,
    _physical_device: vk::PhysicalDevice,
    _device: &ash::Device,
    _queue: vk::Queue,
    _queue_family_index: u32,
) -> Option<GrContext> {
    None
}

/// The Direct3D 11 device on the adapter of the Vulkan device, where the compositor takes
/// Direct3D 11 textures and not the opaque handles of Vulkan.
#[cfg(windows)]
fn create_d3d_device(
    physical_device_id_properties: &vk::PhysicalDeviceIDProperties<'_>,
    supported_image_handle_types: &[String],
) -> VulkanDemoResult<D3DDevice> {
    if physical_device_id_properties.device_luid_valid == vk::TRUE
        && !contains(supported_image_handle_types, KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_NT_HANDLE)
    {
        let luid = physical_device_id_properties.device_luid;
        return super::d3d_memory_helper::D3DMemoryHelper::create_device_by_luid(luid).map(Some);
    }
    Ok(None)
}

/// No Direct3D off Windows.
#[cfg(not(windows))]
fn create_d3d_device(
    _physical_device_id_properties: &vk::PhysicalDeviceIDProperties<'_>,
    _supported_image_handle_types: &[String],
) -> VulkanDemoResult<D3DDevice> {
    Ok(())
}
