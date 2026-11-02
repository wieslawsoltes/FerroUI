use super::vulkan_native_interop::{VkXlibSurfaceCreateInfoKHR, X11VulkanInterface};
use crate::x11_info::X11Info;
use crate::x11_window::SurfacePlatformHandle;
use crate::xlib::{self, XDisplay, XID};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::PixelSize;
use ferroui_controls::platform::{INativePlatformHandleSurface, IPlatformHandle};
use ferroui_vulkan::ash::{self, vk};
use ferroui_vulkan::{
    IVulkanInstance, IVulkanKhrSurfacePlatformSurface, IVulkanKhrSurfacePlatformSurfaceFactory,
    IVulkanPlatformGraphicsContext, VkGetInstanceProcAddressDelegate, VulkanError, VulkanOptions,
    VulkanPlatformGraphics, VulkanPlatformSpecificOptions,
};
use std::any::Any;
use std::ffi::CString;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// The name the Vulkan loader of the system is opened by.
const VULKAN_LIBRARY: &str = "libvulkan.so.1";

/// The Vulkan rendering mode of the platform: the platform graphics of
/// Vulkan over the loader of the system, presenting to the windows of the
/// server through `VK_KHR_xlib_surface`.
pub(crate) struct VulkanSupport;

/// The window the physical devices are asked about (`s_offscreenWindow`).
static OFFSCREEN_WINDOW: AtomicU64 = AtomicU64::new(0);

impl VulkanSupport {
    /// `vkGetInstanceProcAddr` of the loader of the system, which is opened
    /// here and stays open for as long as the lookup lives.
    fn vk_get_instance_proc_addr() -> Result<VkGetInstanceProcAddressDelegate, String> {
        // SAFETY: the library is the Vulkan loader, whose initialisation has
        // no preconditions.
        let entry = unsafe { ash::Entry::load_from(VULKAN_LIBRARY) }.map_err(|error| error.to_string())?;
        let entry = Arc::new(entry);
        Ok(Arc::new(move |instance: isize, name: &str| {
            let Ok(name) = CString::new(name) else { return 0 };
            // SAFETY: the name is a zero-terminated string, and the instance
            // is null or an instance of this loader.
            let address = unsafe {
                (entry.static_fn().get_instance_proc_addr)(vk::Handle::from_raw(instance as u64), name.as_ptr())
            };
            address.map_or(0, |address| address as usize as isize)
        }))
    }

    /// The platform graphics of Vulkan for the connection, or `None` when
    /// the system has no Vulkan loader, no instance with the surface
    /// extensions or no device that presents to a window of the server
    /// (each logged in the area `Vulkan`).
    pub fn try_initialize(info: &X11Info, options: Option<VulkanOptions>) -> Option<Arc<VulkanPlatformGraphics>> {
        let display = info.deferred_display();
        // The reference finds the loader when its first entry point is
        // called, inside the creation of the instance, and logs the
        // exception there.
        let get_proc_address = match Self::vk_get_instance_proc_addr() {
            Ok(get_proc_address) => get_proc_address,
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Vulkan") {
                    logger.log_with_values(None, "Unable to initialize Vulkan rendering: {0}", &[&e]);
                }
                return None;
            }
        };

        let offscreen_window =
            xlib::x_create_simple_window(display, xlib::x_default_root_window(display), 0, 0, 1, 1, 1, 0, 0);
        OFFSCREEN_WINDOW.store(offscreen_window as u64, Ordering::SeqCst);
        xlib::x_sync(display, true);

        let mut platform_options = VulkanPlatformSpecificOptions {
            required_instance_extensions: vec!["VK_KHR_xlib_surface".to_string()],
            get_proc_address_delegate: Some(get_proc_address),
            device_check_surface_factory: Some(Arc::new(move |instance: &Arc<dyn IVulkanInstance>| {
                Self::create_dummy_surface(display, &**instance)
            })),
            ..Default::default()
        };
        platform_options.set_khr_surface_factory(Arc::new(VulkanSurfaceFactory::new(display)));

        VulkanPlatformGraphics::try_create(&options.unwrap_or_default(), platform_options)
    }

    fn create_dummy_surface(display: XDisplay, instance: &dyn IVulkanInstance) -> Result<u64, VulkanError> {
        let surf = Self::create_xlib_surface(display, OFFSCREEN_WINDOW.load(Ordering::SeqCst) as XID, instance)?;
        xlib::x_sync(display, true);
        Ok(surf)
    }

    fn create_xlib_surface(display: XDisplay, window: XID, instance: &dyn IVulkanInstance) -> Result<u64, VulkanError> {
        let vulkan_xlib = X11VulkanInterface::new(instance);
        let create_info = VkXlibSurfaceCreateInfoKHR::default().dpy(display.as_ptr().cast()).window(window as vk::Window);

        let (result, surface) = vulkan_xlib.vk_create_xlib_surface_khr(instance.handle(), &create_info);
        VulkanError::throw_on_error("vkCreateXlibSurfaceKHR", result)?;
        Ok(surface)
    }
}

/// Turns the render surface of a window (a handle described as `XID`) into
/// a surface Vulkan presents to.
pub(crate) struct VulkanSurfaceFactory {
    display: XDisplay,
}

impl VulkanSurfaceFactory {
    pub fn new(display: XDisplay) -> Self {
        Self { display }
    }
}

/// The surface of a window as the native handle surface it is, when its
/// handle is a window of the server.
fn xid_surface(surface: &dyn IPlatformRenderSurface) -> Option<&SurfacePlatformHandle> {
    surface.as_any().downcast_ref::<SurfacePlatformHandle>().filter(|handle| handle.handle_descriptor() == Some("XID"))
}

impl IVulkanKhrSurfacePlatformSurfaceFactory for VulkanSurfaceFactory {
    fn can_render_to_surface(
        &self,
        _context: &dyn IVulkanPlatformGraphicsContext,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> bool {
        xid_surface(&**surface).is_some()
    }

    fn create_surface(
        &self,
        _context: &dyn IVulkanPlatformGraphicsContext,
        handle: &Arc<dyn IPlatformRenderSurface>,
    ) -> Rc<dyn IVulkanKhrSurfacePlatformSurface> {
        Rc::new(XidSurface { display: self.display, handle: handle.clone() })
    }
}

struct XidSurface {
    display: XDisplay,
    handle: Arc<dyn IPlatformRenderSurface>,
}

impl XidSurface {
    fn handle(&self) -> &SurfacePlatformHandle {
        xid_surface(&*self.handle).expect("the factory creates the surface for a window handle only")
    }
}

impl IPlatformRenderSurface for XidSurface {
    fn is_ready(&self) -> bool {
        self.handle.is_ready()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IVulkanKhrSurfacePlatformSurface for XidSurface {
    fn dispose(&self) {
        // No-op
    }

    fn scaling(&self) -> f64 {
        INativePlatformHandleSurface::scaling(self.handle())
    }

    fn size(&self) -> PixelSize {
        INativePlatformHandleSurface::size(self.handle())
    }

    fn create_surface(&self, context: &dyn IVulkanPlatformGraphicsContext) -> u64 {
        let window = IPlatformHandle::handle(self.handle()) as XID;
        match VulkanSupport::create_xlib_surface(self.display, window, &*context.instance()) {
            Ok(surface) => surface,
            Err(error) => panic!("{error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    struct OtherSurface;

    impl IPlatformRenderSurface for OtherSurface {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn a_surface_that_is_not_a_window_handle_is_not_a_vulkan_surface() {
        assert!(xid_surface(&OtherSurface).is_none());
    }

    #[test]
    fn the_loader_is_opened_by_the_name_of_the_reference() {
        assert_eq!(VULKAN_LIBRARY, "libvulkan.so.1");
        // Whether the system has the loader is its own matter: either answer is one, and a
        // missing loader is a reason, not a crash.
        if let Err(reason) = VulkanSupport::vk_get_instance_proc_addr() {
            assert!(!reason.is_empty());
        }
    }
}
