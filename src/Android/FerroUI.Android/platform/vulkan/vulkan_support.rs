use super::vulkan_native_interop::{AndroidVulkanInterface, VkAndroidSurfaceCreateInfoKHR};
use crate::platform::skia_platform::SurfaceViewHandle;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::PixelSize;
use ferroui_controls::platform::INativePlatformHandleSurface;
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
const VULKAN_LIBRARY: &str = "libvulkan.so";

/// The Vulkan rendering mode of the platform: the platform graphics of
/// Vulkan over the loader of the system, presenting to the native window of
/// the surface view through `VK_KHR_android_surface`.
pub(crate) struct VulkanSupport;

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
    /// Vulkan loader, no instance with the surface extensions or no suitable
    /// device (each logged in the area `Vulkan`).
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
            required_instance_extensions: vec!["VK_KHR_android_surface".to_string()],
            get_proc_address_delegate: Some(get_proc_address),
            ..Default::default()
        };
        platform_options.set_khr_surface_factory(Arc::new(VulkanSurfaceFactory));

        VulkanPlatformGraphics::try_create(&options.unwrap_or_default(), platform_options)
    }

    fn create_android_surface(handle: isize, instance: &dyn IVulkanInstance) -> Result<u64, VulkanError> {
        if handle == 0 {
            panic!("Surface handle can't be 0x0 (Parameter 'handle')");
        }
        let vulkan_android = AndroidVulkanInterface::new(instance);
        let create_info = VkAndroidSurfaceCreateInfoKHR::default().window(handle as *mut vk::ANativeWindow);

        let (result, surface) = vulkan_android.vk_create_android_surface_khr(instance.handle(), &create_info);
        VulkanError::throw_on_error("vkCreateAndroidSurfaceKHR", result)?;
        Ok(surface)
    }
}

/// Turns the render surface of a top-level (the handle of the native window
/// of its surface view) into a surface Vulkan presents to.
pub(crate) struct VulkanSurfaceFactory;

/// The surface of a top-level as the native handle surface it is.
fn handle_surface(surface: &dyn IPlatformRenderSurface) -> Option<&SurfaceViewHandle> {
    surface.as_any().downcast_ref::<SurfaceViewHandle>()
}

impl IVulkanKhrSurfacePlatformSurfaceFactory for VulkanSurfaceFactory {
    fn can_render_to_surface(
        &self,
        _context: &dyn IVulkanPlatformGraphicsContext,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> bool {
        handle_surface(&**surface).is_some()
    }

    fn create_surface(
        &self,
        _context: &dyn IVulkanPlatformGraphicsContext,
        handle: &Arc<dyn IPlatformRenderSurface>,
    ) -> Rc<dyn IVulkanKhrSurfacePlatformSurface> {
        Rc::new(AndroidVulkanSurface::new(handle.clone()))
    }
}

struct AndroidVulkanSurface {
    handle: Arc<dyn IPlatformRenderSurface>,
}

impl AndroidVulkanSurface {
    fn new(handle: Arc<dyn IPlatformRenderSurface>) -> Self {
        Self { handle }
    }

    fn handle(&self) -> &SurfaceViewHandle {
        handle_surface(&*self.handle).expect("the factory creates the surface for a handle surface only")
    }
}

impl IPlatformRenderSurface for AndroidVulkanSurface {
    fn is_ready(&self) -> bool {
        self.handle.is_ready()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IVulkanKhrSurfacePlatformSurface for AndroidVulkanSurface {
    fn scaling(&self) -> f64 {
        INativePlatformHandleSurface::scaling(self.handle())
    }

    fn size(&self) -> PixelSize {
        INativePlatformHandleSurface::size(self.handle())
    }

    fn create_surface(&self, context: &dyn IVulkanPlatformGraphicsContext) -> u64 {
        // A reference to the native window is held for the time of the call
        // (the surface of Vulkan takes one of its own), so the UI thread may
        // release the surface view's reference meanwhile.
        let window = self.handle().native_window();
        let handle = window.as_ref().map_or(0, |window| window.handle());
        match VulkanSupport::create_android_surface(handle, &*context.instance()) {
            Ok(surface) => surface,
            Err(error) => panic!("{error}"),
        }
    }

    fn dispose(&self) {
        // No-op
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_loader_is_opened_by_the_name_of_the_reference() {
        assert_eq!(VULKAN_LIBRARY, "libvulkan.so");
    }
}
