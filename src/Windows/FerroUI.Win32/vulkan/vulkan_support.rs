use super::vulkan_native_interop::{VkWin32SurfaceCreateInfoKHR, Win32VulkanInterface};
use crate::interop::unmanaged_methods::get_module_handle;
use crate::offscreen_parent_window::OffscreenParentWindow;
use crate::platform_constants::PlatformConstants;
use crate::window_impl::WindowImplPlatformHandle;
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
use std::sync::Arc;

/// The name the Vulkan loader of the system is opened by.
const VULKAN_LIBRARY: &str = "vulkan-1.dll";

/// The Vulkan rendering mode of the platform: the platform graphics of
/// Vulkan over the loader of the system, presenting to windows through
/// `VK_KHR_win32_surface`.
pub struct VulkanSupport;

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

    /// The platform graphics of Vulkan, or `None` when the system has no
    /// Vulkan loader, no instance with the surface extensions or no device
    /// that presents to a window (each logged in the area `Vulkan`).
    ///
    /// Called on the UI thread: the physical devices are asked about the
    /// hidden parent window of the calling thread.
    pub fn try_initialize(options: Option<VulkanOptions>) -> Option<Arc<VulkanPlatformGraphics>> {
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

        let mut platform_options = VulkanPlatformSpecificOptions {
            required_instance_extensions: vec!["VK_KHR_win32_surface".to_string()],
            get_proc_address_delegate: Some(get_proc_address),
            device_check_surface_factory: Some(Arc::new(|instance: &Arc<dyn IVulkanInstance>| {
                Self::create_hwnd_surface(OffscreenParentWindow::handle(), &**instance)
            })),
            ..Default::default()
        };
        platform_options.set_khr_surface_factory(Arc::new(VulkanSurfaceFactory));

        VulkanPlatformGraphics::try_create(&options.unwrap_or_default(), platform_options)
    }

    fn create_hwnd_surface(window: isize, instance: &dyn IVulkanInstance) -> Result<u64, VulkanError> {
        let vulkan_win32 = Win32VulkanInterface::new(instance);
        let create_info = VkWin32SurfaceCreateInfoKHR::default().hinstance(get_module_handle(None)).hwnd(window);

        let (result, surface) = vulkan_win32.vk_create_win32_surface_khr(instance.handle(), &create_info);
        VulkanError::throw_on_error("vkCreateWin32SurfaceKHR", result)?;
        Ok(surface)
    }
}

/// Turns the render surface of a window (a handle described as `HWND`)
/// into a surface Vulkan presents to.
pub(crate) struct VulkanSurfaceFactory;

/// The surface of a window as the native handle surface it is, when its
/// handle is a window.
fn hwnd_surface(surface: &dyn IPlatformRenderSurface) -> Option<&WindowImplPlatformHandle> {
    surface
        .as_any()
        .downcast_ref::<WindowImplPlatformHandle>()
        .filter(|handle| handle.handle_descriptor() == Some(PlatformConstants::WINDOW_HANDLE_TYPE))
}

impl IVulkanKhrSurfacePlatformSurfaceFactory for VulkanSurfaceFactory {
    fn can_render_to_surface(
        &self,
        _context: &dyn IVulkanPlatformGraphicsContext,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> bool {
        hwnd_surface(&**surface).is_some()
    }

    fn create_surface(
        &self,
        _context: &dyn IVulkanPlatformGraphicsContext,
        handle: &Arc<dyn IPlatformRenderSurface>,
    ) -> Rc<dyn IVulkanKhrSurfacePlatformSurface> {
        Rc::new(HwndVulkanSurface::new(handle.clone()))
    }
}

struct HwndVulkanSurface {
    handle: Arc<dyn IPlatformRenderSurface>,
}

impl HwndVulkanSurface {
    fn new(handle: Arc<dyn IPlatformRenderSurface>) -> Self {
        Self { handle }
    }

    fn handle(&self) -> &WindowImplPlatformHandle {
        hwnd_surface(&*self.handle).expect("the factory creates the surface for a window handle only")
    }
}

impl IPlatformRenderSurface for HwndVulkanSurface {
    fn is_ready(&self) -> bool {
        self.handle.is_ready()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IVulkanKhrSurfacePlatformSurface for HwndVulkanSurface {
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
        let window = IPlatformHandle::handle(self.handle());
        match VulkanSupport::create_hwnd_surface(window, &*context.instance()) {
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
        assert!(hwnd_surface(&OtherSurface).is_none());
    }

    #[test]
    fn the_loader_is_opened_by_the_name_of_the_reference() {
        assert_eq!(VULKAN_LIBRARY, "vulkan-1.dll");
        // Whether the system has the loader is its own matter: either answer is one, and a
        // missing loader is a reason, not a crash.
        if let Err(reason) = VulkanSupport::vk_get_instance_proc_addr() {
            assert!(!reason.is_empty());
        }
    }
}
