//! The Vulkan GPU of the backend against the Vulkan implementation of the
//! machine, on a headless surface: no window system is needed.
//!
//! Without a Vulkan loader, a device or the headless surface extension the
//! test prints why it does nothing and passes; with
//! `FERROUI_VULKAN_TEST_REQUIRE=1` it fails instead (the CI job with a
//! software implementation sets it).
#![cfg(any(ferro_skia_vulkan, ferro_skia_graphite_vulkan))]

use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext, RenderTargetSceneInfo,
};
use ferroui_base::PixelSize;
use ferroui_skia::gpu::vulkan::VulkanSkiaGpu;
use ferroui_skia::gpu::ISkiaGpu;
use ferroui_vulkan::ash::{self, vk};
use ferroui_vulkan::{
    IVulkanInstance, IVulkanKhrSurfacePlatformSurface, IVulkanKhrSurfacePlatformSurfaceFactory,
    IVulkanPlatformGraphicsContext, VkGetInstanceProcAddressDelegate, VulkanOptions, VulkanPlatformGraphics,
    VulkanPlatformSpecificOptions,
};
use skia_safe::Color;
use std::any::Any;
use std::ffi::{c_void, CStr, CString};
use std::rc::Rc;
use std::sync::Arc;

fn skip(reason: &str) {
    if std::env::var("FERROUI_VULKAN_TEST_REQUIRE").is_ok_and(|value| value == "1") {
        panic!("{reason} (FERROUI_VULKAN_TEST_REQUIRE=1)");
    }
    eprintln!("passed over: {reason}");
}

fn loader() -> Result<VkGetInstanceProcAddressDelegate, String> {
    // SAFETY: the library that is opened is the Vulkan loader, whose
    // initialisation has no preconditions.
    let entry = unsafe { ash::Entry::load() }.map_err(|error| format!("no Vulkan loader: {error}"))?;
    let entry = Arc::new(entry);
    Ok(Arc::new(move |instance: isize, name: &str| {
        let Ok(name) = CString::new(name) else { return 0 };
        // SAFETY: the name is a zero-terminated string and the instance is
        // null or an instance of this loader, which the entry keeps open.
        let address = unsafe {
            (entry.static_fn().get_instance_proc_addr)(vk::Handle::from_raw(instance as u64), name.as_ptr())
        };
        address.map_or(0, |address| address as usize as isize)
    }))
}

struct TestSurface;

impl IPlatformRenderSurface for TestSurface {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct HeadlessSurface;

impl IPlatformRenderSurface for HeadlessSurface {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IVulkanKhrSurfacePlatformSurface for HeadlessSurface {
    fn scaling(&self) -> f64 {
        2.0
    }

    fn size(&self) -> PixelSize {
        PixelSize::new(200, 120)
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
        _surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> Rc<dyn IVulkanKhrSurfacePlatformSurface> {
        Rc::new(HeadlessSurface)
    }
}

#[test]
fn skia_draws_a_frame_into_the_image_of_a_vulkan_session_and_presents_it() {
    let loader = match loader() {
        Ok(loader) => loader,
        Err(reason) => return skip(&reason),
    };
    let global = ferroui_vulkan::unmanaged_interop::VulkanGlobalApi::new(loader.clone());
    let headless = global.vk_enumerate_instance_extension_properties().is_ok_and(|extensions| {
        extensions.iter().any(|properties| {
            // SAFETY: the name of an extension is a zero-terminated string within its array.
            unsafe { CStr::from_ptr(properties.extension_name.as_ptr()) }.to_bytes() == b"VK_EXT_headless_surface"
        })
    });
    if !headless {
        return skip("the Vulkan implementation has no VK_EXT_headless_surface");
    }

    let mut platform_options = VulkanPlatformSpecificOptions {
        required_instance_extensions: vec!["VK_EXT_headless_surface".to_string()],
        get_proc_address_delegate: Some(loader),
        device_check_surface_factory: Some(Arc::new(|instance: &Arc<dyn IVulkanInstance>| Ok(create_headless_surface(instance)))),
        ..Default::default()
    };
    platform_options.set_khr_surface_factory(Arc::new(HeadlessSurfaceFactory));
    let Some(graphics) = VulkanPlatformGraphics::try_create(&VulkanOptions::default(), platform_options) else {
        return skip("the Vulkan implementation has no device that presents to a headless surface");
    };

    let context = graphics.create_context();
    let features: &dyn IOptionalFeatureProvider = &*context;
    let vulkan = features.try_get::<dyn IVulkanPlatformGraphicsContext>().expect("the context is a Vulkan context");
    let gpu = VulkanSkiaGpu::new(vulkan, Some(16 * 1024 * 1024), None);
    assert!(!gpu.is_lost());

    let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![Arc::new(TestSurface)];
    assert!(gpu.is_ready_to_create_render_target(&surfaces));
    assert!(!gpu.is_ready_to_create_render_target(&[]));
    let target = gpu.try_create_render_target(&surfaces).expect("a render target for the surface");
    assert!(target.state().is_ready);

    let scene_info = RenderTargetSceneInfo::new(PixelSize::new(200, 120), 2.0, Default::default());
    for color in [Color::RED, Color::from_argb(255, 0, 128, 255)] {
        let session = target.begin_rendering_session(&scene_info, &skia_safe::SurfaceProps::default());
        assert_eq!(session.scale_factor(), 2.0);
        let mut surface = session.sk_surface();
        assert_eq!((surface.width(), surface.height()), (200, 120));
        surface.canvas().clear(color);

        // The frame is read back from the device before it is presented.
        let gr_context = session.gr_context();
        #[cfg(ferro_skia_graphite_vulkan)]
        assert_eq!(gr_context.backend(), ferroui_skia::gpu::SkiaGpuBackend::Graphite);
        gr_context.flush();
        let image = gr_context.snapshot_to_raster(&mut surface).expect("the image of the frame");
        let pixels = image.peek_pixels().expect("a raster image");
        for point in [(0, 0), (100, 60), (199, 119)] {
            assert_eq!(pixels.get_color(point), color, "the pixel at {point:?}");
        }
        drop(surface);
        session.dispose();
    }

    // An offscreen surface of the GPU context of the GPU draws and reads back as well.
    let scoped = gpu.try_get_gr_context().expect("the GPU context");
    let info = skia_safe::ImageInfo::new_n32_premul((32, 32), None);
    let mut offscreen = scoped
        .value()
        .create_surface(&info, &skia_safe::SurfaceProps::default())
        .expect("an offscreen surface of the device");
    offscreen.canvas().clear(Color::GREEN);
    let image = scoped.value().snapshot_to_raster(&mut offscreen).expect("the image of the surface");
    assert_eq!(image.peek_pixels().expect("a raster image").get_color((16, 16)), Color::GREEN);
    drop(offscreen);
    scoped.dispose();

    target.dispose();
    gpu.dispose();
}
