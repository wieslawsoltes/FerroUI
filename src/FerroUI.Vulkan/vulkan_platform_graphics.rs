use crate::interop::{VulkanDevice, VulkanInstance};
use crate::vulkan_context::VulkanContext;
use crate::{
    IVulkanDevice, IVulkanInstance, VulkanDeviceCreationOptions, VulkanError, VulkanOptions,
    VulkanPlatformSpecificOptions,
};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
use std::rc::Rc;
use std::sync::Arc;

/// The platform graphics of Vulkan: creates the graphics contexts of the
/// threads that render, each over a device of its own or all over the device
/// of the application.
pub struct VulkanPlatformGraphics {
    factory: Arc<dyn IVulkanDeviceFactory>,
    platform_options: VulkanPlatformSpecificOptions,
}

/// Where the devices of the contexts come from.
pub trait IVulkanDeviceFactory: Send + Sync {
    /// Whether all contexts share one device.
    fn uses_shaded_device(&self) -> bool;

    /// The device all contexts share.
    ///
    /// # Panics
    /// Panics when the factory creates a device per context.
    fn get_shared_device(&self, platform_options: &VulkanPlatformSpecificOptions) -> Arc<dyn IVulkanDevice>;

    /// Creates the device of a context.
    ///
    /// # Panics
    /// Panics when the contexts share one device.
    fn create_device(
        &self,
        platform_options: &VulkanPlatformSpecificOptions,
    ) -> Result<Arc<dyn IVulkanDevice>, VulkanError>;
}

struct DefaultDeviceFactory {
    instance: Arc<dyn IVulkanInstance>,
    device_options: VulkanDeviceCreationOptions,
}

impl IVulkanDeviceFactory for DefaultDeviceFactory {
    fn uses_shaded_device(&self) -> bool {
        false
    }

    fn get_shared_device(&self, _platform_options: &VulkanPlatformSpecificOptions) -> Arc<dyn IVulkanDevice> {
        panic!("Specified method is not supported.");
    }

    fn create_device(
        &self,
        platform_options: &VulkanPlatformSpecificOptions,
    ) -> Result<Arc<dyn IVulkanDevice>, VulkanError> {
        VulkanDevice::create(self.instance.clone(), &self.device_options, platform_options)
    }
}

struct CustomSharedDeviceFactory {
    device: Arc<dyn IVulkanDevice>,
}

impl IVulkanDeviceFactory for CustomSharedDeviceFactory {
    fn uses_shaded_device(&self) -> bool {
        true
    }

    fn get_shared_device(&self, _platform_options: &VulkanPlatformSpecificOptions) -> Arc<dyn IVulkanDevice> {
        self.device.clone()
    }

    fn create_device(
        &self,
        _platform_options: &VulkanPlatformSpecificOptions,
    ) -> Result<Arc<dyn IVulkanDevice>, VulkanError> {
        panic!("Specified method is not supported.");
    }
}

impl VulkanPlatformGraphics {
    /// Platform graphics over a device factory.
    pub fn new(factory: Arc<dyn IVulkanDeviceFactory>, platform_options: VulkanPlatformSpecificOptions) -> Self {
        Self { factory, platform_options }
    }

    /// The platform graphics for the options of the application and of the
    /// platform: over the device of the application when it gave one, else
    /// over a new instance, on which a device is created once to see that
    /// one can be. A failure is logged in the area `Vulkan`, and there are
    /// no platform graphics then.
    pub fn try_create(options: &VulkanOptions, platform_options: VulkanPlatformSpecificOptions) -> Option<Arc<Self>> {
        if let Some(custom_shared_device) = &options.custom_shared_device {
            return Some(Arc::new(Self::new(
                Arc::new(CustomSharedDeviceFactory { device: custom_shared_device.clone() }),
                platform_options,
            )));
        }

        let create = || -> Result<Arc<Self>, VulkanError> {
            let instance = VulkanInstance::create(&options.vulkan_instance_creation_options, &platform_options)?;

            let dev_opts = options.vulkan_device_creation_options.clone();
            VulkanDevice::create(instance.clone(), &dev_opts, &platform_options)?.dispose();

            Ok(Arc::new(Self::new(
                Arc::new(DefaultDeviceFactory { instance, device_options: dev_opts }),
                platform_options.clone(),
            )))
        };
        match create() {
            Ok(graphics) => Some(graphics),
            Err(e) => {
                //instance?.Dispose();
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Vulkan") {
                    logger.log_with_values(None, "Unable to initialize Vulkan rendering: {0}", &[&e]);
                }
                None
            }
        }
    }
}

impl IPlatformGraphics for VulkanPlatformGraphics {
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        let context = self
            .factory
            .create_device(&self.platform_options)
            .and_then(|device| VulkanContext::new(device, &self.platform_options.platform_features));
        match context {
            Ok(context) => context,
            Err(error) => panic!("{error}"),
        }
    }

    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        // The reference keeps the context it made last and makes a new one
        // at every call all the same: nothing reads the one it kept.
        let device = self.factory.get_shared_device(&self.platform_options);
        match VulkanContext::new(device, &self.platform_options.platform_features) {
            Ok(context) => context,
            Err(error) => panic!("{error}"),
        }
    }

    fn uses_shared_context(&self) -> bool {
        self.factory.uses_shaded_device()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IVulkanPlatformGraphicsContext;
    use ferroui_base::platform::IOptionalFeatureProvider;
    use ferroui_base::reactive::{Disposable, IDisposable};
    use std::any::{Any, TypeId};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Instance;

    impl IVulkanInstance for Instance {
        fn handle(&self) -> isize {
            0x1000
        }

        fn get_instance_proc_address(&self, _instance: isize, _name: &str) -> isize {
            0
        }

        fn get_device_proc_address(&self, _device: isize, _name: &str) -> isize {
            0
        }

        fn enabled_extensions(&self) -> Vec<String> {
            Vec::new()
        }
    }

    #[derive(Default)]
    struct Device {
        locks: AtomicUsize,
    }

    impl IOptionalFeatureProvider for Device {
        fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
            None
        }
    }

    impl IVulkanDevice for Device {
        fn handle(&self) -> isize {
            0x2000
        }

        fn physical_device_handle(&self) -> isize {
            0x3000
        }

        fn main_queue_handle(&self) -> isize {
            0x4000
        }

        fn graphics_queue_family_index(&self) -> u32 {
            7
        }

        fn instance(&self) -> Arc<dyn IVulkanInstance> {
            Arc::new(Instance)
        }

        fn is_lost(&self) -> bool {
            false
        }

        fn lock(&self) -> Rc<dyn IDisposable> {
            self.locks.fetch_add(1, Ordering::SeqCst);
            Disposable::empty()
        }

        fn enabled_extensions(&self) -> Vec<String> {
            Vec::new()
        }

        fn dispose(&self) {}
    }

    #[test]
    fn a_custom_device_is_shared_by_the_contexts() {
        let device = Arc::new(Device::default());
        let options = VulkanOptions { custom_shared_device: Some(device.clone()), ..Default::default() };
        let graphics = VulkanPlatformGraphics::try_create(&options, Default::default()).expect("no instance is needed");
        assert!(graphics.uses_shared_context());

        // No entry point is called to make a context: the tables are filled with what the
        // instance answers, which is nothing here, and the device has no extensions.
        let context = graphics.get_shared_context();
        assert!(device.locks.load(Ordering::SeqCst) >= 1);
        assert!(!context.is_lost());

        let features: &dyn IOptionalFeatureProvider = &*context;
        let vulkan = features.try_get::<dyn IVulkanPlatformGraphicsContext>().expect("the context is a Vulkan context");
        assert_eq!(vulkan.device().handle(), 0x2000);
        assert_eq!(vulkan.instance().handle(), 0x1000);
        assert_eq!(ash::vk::Handle::as_raw(vulkan.device_handle()), 0x2000);
        assert_eq!(ash::vk::Handle::as_raw(vulkan.physical_device_handle()), 0x3000);
        assert_eq!(ash::vk::Handle::as_raw(vulkan.instance_handle()), 0x1000);
        assert_eq!(ash::vk::Handle::as_raw(vulkan.main_queue_handle()), 0x4000);
        assert_eq!(vulkan.graphics_queue_family_index(), 7);
        // Without the extensions there is no external objects feature, and no platform
        // registered a surface factory.
        assert!(features.try_get::<dyn crate::IVulkanContextExternalObjectsFeature>().is_none());
        assert!(features.try_get_shared::<dyn crate::IVulkanKhrSurfacePlatformSurfaceFactory>().is_none());
        context.dispose();
    }

    #[test]
    #[should_panic(expected = "Specified method is not supported.")]
    fn a_custom_device_factory_creates_no_devices() {
        let options = VulkanOptions { custom_shared_device: Some(Arc::new(Device::default())), ..Default::default() };
        let graphics = VulkanPlatformGraphics::try_create(&options, Default::default()).unwrap();
        let _ = graphics.create_context();
    }

    #[test]
    fn without_a_way_to_the_entry_points_there_are_no_platform_graphics() {
        assert!(VulkanPlatformGraphics::try_create(&VulkanOptions::default(), Default::default()).is_none());
    }

    #[test]
    #[should_panic(expected = "Unable to find a suitable platform surface")]
    fn a_context_without_a_suitable_surface_creates_no_render_target() {
        let options = VulkanOptions { custom_shared_device: Some(Arc::new(Device::default())), ..Default::default() };
        let graphics = VulkanPlatformGraphics::try_create(&options, Default::default()).unwrap();
        let context = graphics.get_shared_context();
        let features: &dyn IOptionalFeatureProvider = &*context;
        let vulkan = features.try_get::<dyn IVulkanPlatformGraphicsContext>().unwrap();
        let _ = vulkan.create_render_target(&[]);
    }

    #[test]
    fn the_platform_graphics_are_shared_by_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<VulkanPlatformGraphics>();
        assert_send_sync::<Arc<dyn IVulkanDevice>>();
        assert_send_sync::<Arc<dyn IVulkanInstance>>();
    }
}
