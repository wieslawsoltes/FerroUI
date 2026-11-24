//! Graphite on the Vulkan device of the backend (#1365, the feature `graphite-vulkan`) against the
//! Vulkan implementation of the machine: an offscreen surface and a window image of a swap chain on
//! a headless surface, drawn and read back.
//!
//! Without a Vulkan loader, a device or the headless surface extension the
//! test prints why it does nothing and passes; with
//! `FERROUI_VULKAN_TEST_REQUIRE=1` it fails instead (the CI job with a
//! software implementation sets it).
#![cfg(all(ferro_skia_vulkan, feature = "graphite-vulkan"))]

use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext,
};
use ferroui_base::PixelSize;
use ferroui_skia::gpu::vulkan::GraphiteVulkanContext;
use ferroui_vulkan::ash::{self, vk};
use ferroui_vulkan::{
    IVulkanInstance, IVulkanKhrSurfacePlatformSurface, IVulkanKhrSurfacePlatformSurfaceFactory,
    IVulkanPlatformGraphicsContext, IVulkanRenderSession, IVulkanRenderTarget, VkGetInstanceProcAddressDelegate,
    VulkanOptions, VulkanPlatformGraphics,
    VulkanPlatformSpecificOptions,
};
use skia_safe::Color;
use std::any::Any;
use std::ffi::{c_void, CStr, CString};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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

/// A headless surface whose size the test changes.
struct HeadlessSurface {
    size: Arc<Mutex<PixelSize>>,
}

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
        *self.size.lock().unwrap()
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

struct HeadlessSurfaceFactory {
    size: Arc<Mutex<PixelSize>>,
}

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
        Rc::new(HeadlessSurface { size: self.size.clone() })
    }
}

fn color_of(pixels: &[u8], width: i32, x: i32, y: i32) -> [u8; 4] {
    let at = ((y * width + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
}

#[test]
fn graphite_draws_on_the_vulkan_device_offscreen_and_on_a_window_image() {
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
        device_check_surface_factory: Some(Arc::new(|instance: &Arc<dyn IVulkanInstance>| {
            Ok(create_headless_surface(instance))
        })),
        ..Default::default()
    };
    let size = Arc::new(Mutex::new(PixelSize::new(200, 120)));
    platform_options.set_khr_surface_factory(Arc::new(HeadlessSurfaceFactory { size }));
    let Some(graphics) = VulkanPlatformGraphics::try_create(&VulkanOptions::default(), platform_options) else {
        return skip("the Vulkan implementation has no device that presents to a headless surface");
    };
    let context = graphics.create_context();
    let features: &dyn IOptionalFeatureProvider = &*context;
    let vulkan = features.try_get::<dyn IVulkanPlatformGraphicsContext>().expect("the context is a Vulkan context");
    let Some(mut graphite) = GraphiteVulkanContext::new(vulkan.clone()) else {
        return skip("Skia makes no Graphite context for the device");
    };
    assert!(!graphite.is_lost());
    let props = skia_safe::SurfaceProps::default();

    // Offscreen.
    let info = skia_safe::ImageInfo::new_n32_premul((32, 24), None);
    let mut surface = graphite.make_surface(&info, &props).expect("an offscreen Graphite surface");
    surface.canvas().clear(Color::from_argb(255, 0, 128, 255));
    let mut pixels = vec![0u8; 32 * 24 * 4];
    assert!(graphite.read_pixels(&mut surface, &info, &mut pixels, 32 * 4));
    let expected = if cfg!(target_endian = "little") && info.color_type() == skia_safe::ColorType::BGRA8888 {
        [255, 128, 0, 255]
    } else {
        [0, 128, 255, 255]
    };
    assert_eq!(color_of(&pixels, 32, 16, 12), expected);
    drop(surface);

    // The image of a swap chain target, with the usage Graphite asks for.
    let surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![Arc::new(TestSurface)];
    let target = vulkan.create_render_target(&surfaces);
    target.request_image_usage(vk::ImageUsageFlags::INPUT_ATTACHMENT.as_raw());
    let session = target.begin_draw();
    let image = session.image_info();
    assert!(vk::ImageUsageFlags::from_raw(image.usage_flags).contains(vk::ImageUsageFlags::INPUT_ATTACHMENT));
    let color_type = if session.is_rgba() { skia_safe::ColorType::RGBA8888 } else { skia_safe::ColorType::BGRA8888 };
    let mut window = graphite.wrap_image(&image, color_type, &props).expect("a Graphite surface on the window image");
    window.canvas().clear(Color::RED);
    let info = skia_safe::ImageInfo::new((200, 120), color_type, skia_safe::AlphaType::Premul, None);
    let mut pixels = vec![0u8; 200 * 120 * 4];
    assert!(graphite.read_pixels(&mut window, &info, &mut pixels, 200 * 4));
    let red = if session.is_rgba() { [255, 0, 0, 255] } else { [0, 0, 255, 255] };
    assert_eq!(color_of(&pixels, 200, 100, 60), red);
    drop(window);
    session.dispose();

    target.dispose();
    drop(graphite);
    context.dispose();
}
