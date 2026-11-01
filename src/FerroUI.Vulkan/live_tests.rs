//! Tests against the Vulkan implementation of the machine, through the
//! loader of the system.
//!
//! Where no loader can be opened (or no device is found) a test prints why
//! it does nothing and passes; with `FERROUI_VULKAN_TEST_REQUIRE=1` it fails
//! instead, which is how the CI job with a software implementation makes
//! sure the tests ran. Parts that need something an implementation may lack
//! (a headless surface, exportable semaphores or memory) say so when they
//! are passed over.

use crate::interop::{VulkanCommandBufferPool, VulkanFence, VulkanImage};
use crate::{
    IVulkanContextExternalObjectsFeature, IVulkanDevice, IVulkanInstance, IVulkanKhrSurfacePlatformSurface,
    IVulkanKhrSurfacePlatformSurfaceFactory, IVulkanPlatformGraphicsContext, VkGetInstanceProcAddressDelegate,
    VulkanInstanceCreationOptions, VulkanOptions, VulkanPlatformGraphics, VulkanPlatformSpecificOptions,
};
use ash::vk;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext,
    KnownPlatformGraphicsExternalImageHandleTypes, KnownPlatformGraphicsExternalSemaphoreHandleTypes, PlatformHandle,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
};
use ferroui_base::PixelSize;
use std::any::Any;
use std::ffi::{c_void, CStr, CString};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn required() -> bool {
    std::env::var("FERROUI_VULKAN_TEST_REQUIRE").is_ok_and(|value| value == "1")
}

fn skip(test: &str, reason: &str) {
    if required() {
        panic!("{test}: {reason} (FERROUI_VULKAN_TEST_REQUIRE=1)");
    }
    eprintln!("{test}: passed over: {reason}");
}

/// The lookup of the loader of the system, when there is one.
fn loader() -> Result<VkGetInstanceProcAddressDelegate, String> {
    // SAFETY: the library that is opened is the Vulkan loader, whose
    // initialisation has no preconditions.
    let entry = unsafe { ash::Entry::load() }.map_err(|error| format!("no Vulkan loader: {error}"))?;
    let entry = Arc::new(entry);
    Ok(Arc::new(move |instance: isize, name: &str| {
        let Ok(name) = CString::new(name) else { return 0 };
        // SAFETY: the name is a zero-terminated string and the instance is
        // null or an instance of this loader; the entry keeps the library
        // open for as long as the lookup lives.
        let address = unsafe {
            (entry.static_fn().get_instance_proc_addr)(vk::Handle::from_raw(instance as u64), name.as_ptr())
        };
        address.map_or(0, |address| address as usize as isize)
    }))
}

struct Live {
    graphics: Arc<VulkanPlatformGraphics>,
}

/// Platform graphics without a window system: no surface extension is
/// required of the instance and no swap chain extension of the device,
/// unless `platform_options` bring a headless surface.
fn live(test: &str, mut platform_options: VulkanPlatformSpecificOptions, with_swapchain: bool) -> Option<Live> {
    let get_proc_address = match loader() {
        Ok(loader) => loader,
        Err(reason) => {
            skip(test, &reason);
            return None;
        }
    };
    platform_options.get_proc_address_delegate = Some(get_proc_address);
    let mut options = VulkanOptions {
        vulkan_instance_creation_options: VulkanInstanceCreationOptions {
            application_name: Some("ferroui-vulkan tests".to_string()),
            require_surface_extension: with_swapchain,
            ..Default::default()
        },
        ..Default::default()
    };
    options.vulkan_device_creation_options.require_swapchain_extension = with_swapchain;
    match VulkanPlatformGraphics::try_create(&options, platform_options) {
        Some(graphics) => Some(Live { graphics }),
        None => {
            skip(test, "the Vulkan implementation has no instance or no device for these options");
            None
        }
    }
}

fn vulkan_context(context: &Rc<dyn IPlatformGraphicsContext>) -> Rc<dyn IVulkanPlatformGraphicsContext> {
    let features: &dyn IOptionalFeatureProvider = &**context;
    features.try_get::<dyn IVulkanPlatformGraphicsContext>().expect("the context is a Vulkan context")
}

#[test]
fn an_instance_and_a_device_are_created_and_the_context_reads_them() {
    let Some(live) = live("instance and device", Default::default(), false) else { return };
    assert!(!live.graphics.uses_shared_context());

    let context = live.graphics.create_context();
    let vulkan = vulkan_context(&context);
    assert!(!context.is_lost());
    let device = vulkan.device();
    assert_ne!(device.handle(), 0);
    assert_ne!(device.physical_device_handle(), 0);
    assert_ne!(vulkan.instance().handle(), 0);
    assert_eq!(vk::Handle::as_raw(vulkan.device_handle()) as isize, device.handle());
    assert_eq!(vulkan.graphics_queue_family_index(), device.graphics_queue_family_index());

    // The entry points are found through the instance, by name.
    let instance = vulkan.instance();
    assert_ne!(instance.get_instance_proc_address(instance.handle(), "vkCreateDevice"), 0);
    assert_ne!(instance.get_device_proc_address(device.handle(), "vkQueueSubmit"), 0);
    assert_eq!(instance.get_device_proc_address(device.handle(), "vkNoSuchFunction"), 0);

    // The queue family the device was created for has graphics.
    let families = vulkan.instance_api().get_physical_device_queue_family_properties(vulkan.physical_device_handle());
    let family = &families[vulkan.graphics_queue_family_index() as usize];
    assert!(family.queue_flags.contains(vk::QueueFlags::GRAPHICS));

    // The queue is read under the lock only.
    let lock = context.ensure_current();
    assert_ne!(vk::Handle::as_raw(vulkan.main_queue_handle()), 0);
    lock.dispose();

    context.dispose();
    device.dispose();
    assert_eq!(device.handle(), 0);
}

#[test]
fn the_queue_of_a_device_is_refused_without_the_lock() {
    let Some(live) = live("queue without the lock", Default::default(), false) else { return };
    let context = live.graphics.create_context();
    let device = vulkan_context(&context).device();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| device.main_queue_handle()));
    let message = result.err().and_then(|payload| payload.downcast_ref::<&str>().map(|s| s.to_string()));
    assert_eq!(message.as_deref(), Some("This class is only usable when locked"));
}

#[test]
fn the_lock_of_a_device_is_reentrant_and_excludes_other_threads() {
    let Some(live) = live("device lock", Default::default(), false) else { return };
    let context = live.graphics.create_context();
    let device: Arc<dyn IVulkanDevice> = vulkan_context(&context).device();

    let outer = device.lock();
    let inner = device.lock();
    assert_ne!(device.main_queue_handle(), 0);
    inner.dispose();
    // Still locked by the outer lock.
    assert_ne!(device.main_queue_handle(), 0);

    let (locked, has_locked) = mpsc::channel();
    let other = {
        let device = device.clone();
        std::thread::spawn(move || {
            let lock = device.lock();
            let queue = device.main_queue_handle();
            locked.send(queue).unwrap();
            lock.dispose();
        })
    };
    assert!(has_locked.recv_timeout(Duration::from_millis(100)).is_err(), "the other thread waits");
    outer.dispose();
    assert_ne!(has_locked.recv_timeout(Duration::from_secs(10)).expect("the other thread locks"), 0);
    other.join().unwrap();
}

#[test]
fn commands_are_submitted_and_an_image_is_created_and_transitioned() {
    let Some(live) = live("commands and images", Default::default(), false) else { return };
    let context = live.graphics.create_context();
    let vulkan = vulkan_context(&context);
    let lock = context.ensure_current();

    let fence = VulkanFence::new(vulkan.clone(), vk::FenceCreateFlags::SIGNALED).unwrap();
    assert!(fence.is_signaled());
    fence.wait().unwrap();
    fence.dispose();

    let pool = VulkanCommandBufferPool::new(vulkan.clone(), false).unwrap();
    let image =
        VulkanImage::new(vulkan.clone(), pool.clone(), vk::Format::R8G8B8A8_UNORM, PixelSize::new(64, 48), 0).unwrap();
    let info = image.image_info();
    assert_ne!(info.handle, 0);
    assert_ne!(info.memory_handle, 0);
    assert_ne!(info.view_handle, 0);
    assert!(info.memory_size >= 64 * 48 * 4);
    assert_eq!(info.pixel_size, PixelSize::new(64, 48));
    assert_eq!(info.format, vk::Format::R8G8B8A8_UNORM.as_raw() as u32);
    assert_eq!(info.layout, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL.as_raw() as u32);
    assert_eq!(info.tiling, vk::ImageTiling::OPTIMAL.as_raw() as u32);
    assert_eq!(info.level_count, 6);
    assert_eq!(info.sample_count, 1);
    assert!(!info.is_protected);
    assert_eq!(info.usage_flags, image.usage_flags().as_raw());

    image.transition_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, vk::AccessFlags::TRANSFER_READ).unwrap();
    assert_eq!(image.current_layout(), vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
    assert_eq!(image.image_info().layout, vk::ImageLayout::TRANSFER_SRC_OPTIMAL.as_raw() as u32);

    // The two transitions were submitted; freeing them waits for their fences.
    pool.free_used_command_buffers();
    let buffer = pool.create_command_buffer().unwrap();
    buffer.begin_recording().unwrap();
    buffer.submit().unwrap();
    pool.free_finished_command_buffers();

    assert_eq!(vulkan.device_api().device_wait_idle(vulkan.device_handle()), vk::Result::SUCCESS);
    image.dispose();
    assert_eq!(image.image_info().handle, 0);
    pool.dispose();
    lock.dispose();
    context.dispose();
}

/// A surface of the tests: a size that can change.
struct TestSurface {
    size: Mutex<PixelSize>,
}

impl IPlatformRenderSurface for TestSurface {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The Vulkan view of a [`TestSurface`]: a headless surface of the
/// instance.
struct HeadlessSurface {
    surface: Arc<dyn IPlatformRenderSurface>,
}

impl IPlatformRenderSurface for HeadlessSurface {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IVulkanKhrSurfacePlatformSurface for HeadlessSurface {
    fn scaling(&self) -> f64 {
        1.5
    }

    fn size(&self) -> PixelSize {
        let surface = self.surface.as_any().downcast_ref::<TestSurface>().unwrap();
        *surface.size.lock().unwrap()
    }

    fn create_surface(&self, context: &dyn IVulkanPlatformGraphicsContext) -> u64 {
        create_headless_surface(&context.instance())
    }

    fn dispose(&self) {}
}

fn create_headless_surface(instance: &Arc<dyn IVulkanInstance>) -> u64 {
    let handle = instance.handle();
    let fp = ash::ext::headless_surface::InstanceFn::load(|name: &CStr| {
        instance.get_instance_proc_address(handle, name.to_str().unwrap()) as *const c_void
    });
    let info = vk::HeadlessSurfaceCreateInfoEXT::default();
    let mut surface = vk::SurfaceKHR::null();
    // SAFETY: the instance has the headless surface extension (the test
    // enabled it), the structure is complete and the result is a local.
    let result = unsafe {
        (fp.create_headless_surface_ext)(vk::Handle::from_raw(handle as u64), &info, std::ptr::null(), &mut surface)
    };
    assert_eq!(result, vk::Result::SUCCESS);
    vk::Handle::as_raw(surface)
}

struct HeadlessSurfaceFactory;

impl IVulkanKhrSurfacePlatformSurfaceFactory for HeadlessSurfaceFactory {
    fn can_render_to_surface(
        &self,
        _context: &dyn IVulkanPlatformGraphicsContext,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> bool {
        surface.as_any().is::<TestSurface>()
    }

    fn create_surface(
        &self,
        _context: &dyn IVulkanPlatformGraphicsContext,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> Rc<dyn IVulkanKhrSurfacePlatformSurface> {
        Rc::new(HeadlessSurface { surface: surface.clone() })
    }
}

fn has_instance_extension(name: &str) -> Result<bool, String> {
    let loader = loader()?;
    let api = crate::unmanaged_interop::VulkanGlobalApi::new(loader);
    let extensions = api.vk_enumerate_instance_extension_properties().map_err(|error| format!("{error:?}"))?;
    Ok(extensions
        .iter()
        .any(|properties| crate::unmanaged_interop::vulkan_global_api::fixed_name(&properties.extension_name) == name))
}

#[test]
fn frames_are_presented_through_a_swap_chain_and_the_swap_chain_follows_the_surface() {
    const TEST: &str = "swap chain on a headless surface";
    match has_instance_extension("VK_EXT_headless_surface") {
        Ok(true) => {}
        Ok(false) => return skip(TEST, "the Vulkan implementation has no VK_EXT_headless_surface"),
        Err(reason) => return skip(TEST, &reason),
    }
    let mut platform_options = VulkanPlatformSpecificOptions {
        required_instance_extensions: vec!["VK_EXT_headless_surface".to_string()],
        // The devices are asked whether they present to a headless surface, as a platform asks
        // about a window of its own.
        device_check_surface_factory: Some(Arc::new(|instance: &Arc<dyn IVulkanInstance>| Ok(create_headless_surface(instance)))),
        ..Default::default()
    };
    platform_options.set_khr_surface_factory(Arc::new(HeadlessSurfaceFactory));
    let Some(live) = live(TEST, platform_options, true) else { return };

    let context = live.graphics.create_context();
    let vulkan = vulkan_context(&context);
    assert!(vulkan.instance().enabled_extensions().iter().any(|name| name == "VK_KHR_surface"));
    assert!(vulkan.device().enabled_extensions().iter().any(|name| name == "VK_KHR_swapchain"));
    let features: &dyn IOptionalFeatureProvider = &*context;
    assert!(features.try_get_shared::<dyn IVulkanKhrSurfacePlatformSurfaceFactory>().is_some());

    let surface = Arc::new(TestSurface { size: Mutex::new(PixelSize::new(320, 200)) });
    let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![surface.clone()];
    let target = vulkan.create_render_target(&surfaces);
    assert!(target.state().is_ready);

    let mut first_image = 0;
    // Reuse both acquire slots repeatedly and wrap the swap chain.
    for frame in 0..24 {
        let session = target.begin_draw();
        // The device is locked for the time of the frame.
        assert_ne!(vk::Handle::as_raw(vulkan.main_queue_handle()), 0);
        assert_eq!(session.size(), PixelSize::new(320, 200));
        assert_eq!(session.scaling(), 1.5);
        assert!(session.is_y_flipped());
        let info = session.image_info();
        assert_ne!(info.handle, 0);
        assert_eq!(info.pixel_size, PixelSize::new(320, 200));
        assert_eq!(info.level_count, 1, "the blit source contains one presentable mip level");
        assert_eq!(info.layout, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL.as_raw() as u32);
        let expected = if session.is_rgba() { vk::Format::R8G8B8A8_UNORM } else { vk::Format::B8G8R8A8_UNORM };
        assert_eq!(info.format, expected.as_raw() as u32);
        if frame == 0 {
            first_image = info.handle;
        } else {
            assert_eq!(info.handle, first_image, "the image is kept while the size is");
        }
        session.dispose();
        // Disposing twice presents once.
        session.dispose();
    }

    // The lock was released with the session.
    let device = vulkan.device();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| device.main_queue_handle())).is_err());

    // The surface changes its size: the swap chain and the image follow.
    *surface.size.lock().unwrap() = PixelSize::new(400, 240);
    let session = target.begin_draw();
    assert_eq!(session.size(), PixelSize::new(400, 240));
    assert_eq!(session.image_info().pixel_size, PixelSize::new(400, 240));
    session.dispose();

    target.dispose();
    context.dispose();
}

fn external_objects(
    test: &str,
) -> Option<(Rc<dyn IPlatformGraphicsContext>, Rc<dyn IVulkanPlatformGraphicsContext>, Rc<dyn IVulkanContextExternalObjectsFeature>)>
{
    let live = live(test, Default::default(), false)?;
    let context = live.graphics.create_context();
    let vulkan = vulkan_context(&context);
    let features: &dyn IOptionalFeatureProvider = &*context;
    let Some(feature) = features.try_get::<dyn IVulkanContextExternalObjectsFeature>() else {
        eprintln!("{test}: passed over: the device lacks the extensions of the external objects feature");
        return None;
    };
    Some((context, vulkan, feature))
}

/// The device of a context with the entry points of the bindings, for what
/// the tests create themselves.
fn ash_device(vulkan: &Rc<dyn IVulkanPlatformGraphicsContext>) -> ash::Device {
    let instance = vulkan.instance();
    let device = vulkan.device().handle();
    // SAFETY: the handle is the device of the context, and the lookup is
    // that of its instance.
    unsafe {
        ash::Device::load_with(
            |name| instance.get_device_proc_address(device, name.to_str().unwrap()) as *const c_void,
            vulkan.device_handle(),
        )
    }
}

fn ash_instance(vulkan: &Rc<dyn IVulkanPlatformGraphicsContext>) -> ash::Instance {
    let instance = vulkan.instance();
    let handle = instance.handle();
    // SAFETY: the handle is the instance of the context, and the lookup is
    // its own.
    unsafe {
        ash::Instance::load_with(
            |name| instance.get_instance_proc_address(handle, name.to_str().unwrap()) as *const c_void,
            vulkan.instance_handle(),
        )
    }
}

#[test]
fn the_external_objects_feature_names_what_it_imports() {
    const TEST: &str = "external objects";
    let Some((context, _vulkan, feature)) = external_objects(TEST) else { return };
    if cfg!(windows) {
        assert_eq!(feature.supported_image_handle_types(), ["VulkanOpaqueNtHandle", "VulkanOpaqueKmtHandle"]);
    } else {
        assert_eq!(feature.supported_image_handle_types(), ["VulkanOpaquePosixFileDescriptor"]);
        assert_eq!(feature.supported_semaphore_types(), ["VulkanOpaquePosixFileDescriptor"]);
    }
    let kind = feature.supported_image_handle_types()[0].clone();
    let capabilities = feature.get_synchronization_capabilities(&kind);
    assert_eq!(
        capabilities,
        ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities::SEMAPHORES
    );
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        feature.get_synchronization_capabilities("IOSurfaceRef")
    }))
    .is_err());
    // A device has a universally unique identifier; the locally unique one exists on Windows.
    assert_eq!(feature.device_uuid().map(|uuid| uuid.len()), Some(16));
    if let Some(luid) = feature.device_luid() {
        assert_eq!(luid.len(), 8);
    }
    // A handle of another kind is refused before anything is asked of the device.
    let foreign: Rc<dyn ferroui_base::platform::IPlatformHandle> = Rc::new(PlatformHandle::new(1, Some("IOSurfaceRef")));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = feature.import_semaphore(foreign.clone());
    }))
    .is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = feature.import_image(foreign.clone(), Default::default());
    }))
    .is_err());
    context.dispose();
}

#[cfg(unix)]
#[test]
fn a_semaphore_exported_as_a_file_descriptor_is_imported_signalled_and_waited_for() {
    const TEST: &str = "semaphore import";
    let Some((context, vulkan, feature)) = external_objects(TEST) else { return };
    let instance = ash_instance(&vulkan);
    let device = ash_device(&vulkan);
    let handle_type = vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD;

    let query = vk::PhysicalDeviceExternalSemaphoreInfo::default().handle_type(handle_type);
    let mut properties = vk::ExternalSemaphoreProperties::default();
    // SAFETY: the structures are complete and outlive the call.
    unsafe {
        instance.get_physical_device_external_semaphore_properties(
            vulkan.physical_device_handle(),
            &query,
            &mut properties,
        )
    };
    let needed = vk::ExternalSemaphoreFeatureFlags::EXPORTABLE | vk::ExternalSemaphoreFeatureFlags::IMPORTABLE;
    if !properties.external_semaphore_features.contains(needed) {
        eprintln!("{TEST}: passed over: the device neither exports nor imports opaque file descriptors of semaphores");
        context.dispose();
        return;
    }

    let lock = context.ensure_current();
    let mut export = vk::ExportSemaphoreCreateInfo::default().handle_types(handle_type);
    let create_info = vk::SemaphoreCreateInfo::default().push_next(&mut export);
    // SAFETY: the device is that of the context, locked by this thread; the
    // structures are complete.
    let exported = unsafe { device.create_semaphore(&create_info, None) }.unwrap();
    let fns = ash::khr::external_semaphore_fd::Device::new(&instance, &device);
    let get = vk::SemaphoreGetFdInfoKHR::default().semaphore(exported).handle_type(handle_type);
    // SAFETY: as above.
    let fd = unsafe { fns.get_semaphore_fd(&get) }.unwrap();

    let handle = Rc::new(PlatformHandle::new(
        fd as isize,
        Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR),
    ));
    let imported = feature.import_semaphore(handle);
    assert_ne!(imported.handle(), 0);
    assert_ne!(imported.handle(), vk::Handle::as_raw(exported));

    // A batch signals the semaphore, a second one waits for it: both run to their end.
    imported.submit_signal_semaphore();
    imported.submit_wait_semaphore();
    assert_eq!(vulkan.device_api().device_wait_idle(vulkan.device_handle()), vk::Result::SUCCESS);

    imported.dispose();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| imported.handle())).is_err());
    // SAFETY: nothing uses the semaphore anymore.
    unsafe { device.destroy_semaphore(exported, None) };
    lock.dispose();
    context.dispose();
}

#[cfg(unix)]
#[test]
fn the_memory_of_an_image_exported_as_a_file_descriptor_is_imported() {
    const TEST: &str = "image import";
    let Some((context, vulkan, feature)) = external_objects(TEST) else { return };
    let instance = ash_instance(&vulkan);
    let device = ash_device(&vulkan);
    let handle_type = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
    let format = vk::Format::R8G8B8A8_UNORM;
    let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT
        | vk::ImageUsageFlags::TRANSFER_DST
        | vk::ImageUsageFlags::TRANSFER_SRC
        | vk::ImageUsageFlags::SAMPLED;
    let flags = vk::ImageCreateFlags::MUTABLE_FORMAT;

    let mut external_query = vk::PhysicalDeviceExternalImageFormatInfo::default().handle_type(handle_type);
    let query = vk::PhysicalDeviceImageFormatInfo2::default()
        .format(format)
        .ty(vk::ImageType::TYPE_2D)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(usage)
        .flags(flags)
        .push_next(&mut external_query);
    let mut external_properties = vk::ExternalImageFormatProperties::default();
    let mut properties = vk::ImageFormatProperties2::default().push_next(&mut external_properties);
    // SAFETY: the structures are complete and outlive the call.
    let supported = unsafe {
        instance.get_physical_device_image_format_properties2(vulkan.physical_device_handle(), &query, &mut properties)
    };
    let needed = vk::ExternalMemoryFeatureFlags::EXPORTABLE | vk::ExternalMemoryFeatureFlags::IMPORTABLE;
    if supported.is_err() || !external_properties.external_memory_properties.external_memory_features.contains(needed) {
        eprintln!("{TEST}: passed over: the device neither exports nor imports opaque file descriptors of image memory");
        context.dispose();
        return;
    }

    let lock = context.ensure_current();
    let size = PixelSize::new(32, 16);
    let mut external = vk::ExternalMemoryImageCreateInfo::default().handle_types(handle_type);
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D { width: 32, height: 16, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .flags(flags)
        .push_next(&mut external);
    // SAFETY: the device is that of the context, locked by this thread; the
    // structures are complete. The same holds for the calls below.
    let image = unsafe { device.create_image(&image_info, None) }.unwrap();
    let requirements = unsafe { device.get_image_memory_requirements(image) };
    let memory_properties = unsafe { instance.get_physical_device_memory_properties(vulkan.physical_device_handle()) };
    let memory_type = crate::interop::VulkanMemoryHelper::find_suitable_memory_type_index_in(
        &memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );
    assert!(memory_type >= 0);
    let mut export = vk::ExportMemoryAllocateInfo::default().handle_types(handle_type);
    let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
    let allocate_info = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type as u32)
        .push_next(&mut export)
        .push_next(&mut dedicated);
    let memory = unsafe { device.allocate_memory(&allocate_info, None) }.unwrap();
    unsafe { device.bind_image_memory(image, memory, 0) }.unwrap();
    let fns = ash::khr::external_memory_fd::Device::new(&instance, &device);
    let get = vk::MemoryGetFdInfoKHR::default().memory(memory).handle_type(handle_type);
    let fd = unsafe { fns.get_memory_fd(&get) }.unwrap();

    let handle = Rc::new(PlatformHandle::new(
        fd as isize,
        Some(KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR),
    ));
    let properties = PlatformGraphicsExternalImageProperties {
        width: size.width,
        height: size.height,
        format: PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm,
        memory_size: requirements.size,
        memory_offset: 0,
        top_left_origin: true,
        ..Default::default()
    };
    let imported = feature.import_image(handle, properties.clone());
    let info = imported.info();
    assert_ne!(info.handle, 0);
    assert_ne!(info.handle, vk::Handle::as_raw(image));
    assert_ne!(info.memory_handle, 0);
    assert_eq!(info.memory_size, requirements.size);
    assert_eq!(info.pixel_size, size);
    assert_eq!(info.format, format.as_raw() as u32);
    assert_eq!(info.level_count, 1);
    assert_eq!(info.layout, vk::ImageLayout::TRANSFER_SRC_OPTIMAL.as_raw() as u32);

    // A size that is not the size the device computes is refused, and so is an offset.
    let wrong = PlatformGraphicsExternalImageProperties { memory_size: requirements.size + 1, ..properties };
    let fd = unsafe { fns.get_memory_fd(&get) }.unwrap();
    let handle = Rc::new(PlatformHandle::new(
        fd as isize,
        Some(KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR),
    ));
    let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = feature.import_image(handle, wrong);
    }));
    let message = refused.err().and_then(|payload| payload.downcast_ref::<String>().cloned());
    assert_eq!(message.as_deref(), Some("Invalid memory size"));
    // The descriptor that was not imported is still owned here.
    // SAFETY: the descriptor was returned by the device and given to nobody.
    unsafe { libc_close(fd) };

    assert_eq!(vulkan.device_api().device_wait_idle(vulkan.device_handle()), vk::Result::SUCCESS);
    imported.dispose();
    unsafe {
        device.destroy_image(image, None);
        device.free_memory(memory, None);
    }
    lock.dispose();
    context.dispose();
}

#[cfg(unix)]
unsafe fn libc_close(fd: i32) {
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    // SAFETY: the caller owns the descriptor.
    unsafe { close(fd) };
}
