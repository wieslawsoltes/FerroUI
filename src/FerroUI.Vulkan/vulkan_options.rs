use crate::{IVulkanDevice, IVulkanInstance, IVulkanKhrSurfacePlatformSurfaceFactory, VulkanError};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// The address of an instance-level entry point of Vulkan by its name
/// (`vkGetInstanceProcAddr`): the instance, zero for the entry points that
/// need none, and the name; zero when there is no such entry point.
pub type VkGetInstanceProcAddressDelegate = Arc<dyn Fn(isize, &str) -> isize + Send + Sync>;

/// Creates a `VkSurfaceKHR` on an instance, for asking the physical devices
/// whether they can present. Zero stands for no surface: the devices are
/// not asked then. A failure is the failure of the creation of the device.
pub type VulkanDeviceCheckSurfaceFactory =
    Arc<dyn Fn(&Arc<dyn IVulkanInstance>) -> Result<u64, VulkanError> + Send + Sync>;

/// A version number of three parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: i32,
    pub minor: i32,
    pub build: i32,
}

impl Version {
    pub const fn new(major: i32, minor: i32, build: i32) -> Self {
        Self { major, minor, build }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.build)
    }
}

/// How an application wants Vulkan to be set up.
#[derive(Clone, Default)]
pub struct VulkanOptions {
    pub vulkan_instance_creation_options: VulkanInstanceCreationOptions,
    pub vulkan_device_creation_options: VulkanDeviceCreationOptions,
    /// A device of the application that every context renders with, in
    /// place of an instance and devices the platform graphics create.
    pub custom_shared_device: Option<Arc<dyn IVulkanDevice>>,
}

#[derive(Clone)]
pub struct VulkanInstanceCreationOptions {
    /// Replaces the way the platform finds the entry points of Vulkan.
    pub custom_get_proc_address_delegate: Option<VkGetInstanceProcAddressDelegate>,

    /// Sets the application name of the vulkan instance
    pub application_name: Option<String>,

    /// Specifies the vulkan api version to use
    pub vulkan_version: Version,

    /// Specifies additional extensions to enable if available on the instance
    pub instance_extensions: Vec<String>,

    /// Specifies layers to enable if available on the instance
    pub enabled_layers: Vec<String>,

    /// Enables the debug layer
    pub use_debug: bool,

    /// Requires and enables VK_KHR_surface on the created instance. Disable for offscreen-only
    /// rendering with drivers that don't expose the extension.
    pub require_surface_extension: bool,
}

impl Default for VulkanInstanceCreationOptions {
    fn default() -> Self {
        Self {
            custom_get_proc_address_delegate: None,
            application_name: None,
            vulkan_version: Version::new(1, 1, 0),
            instance_extensions: Vec::new(),
            enabled_layers: Vec::new(),
            use_debug: false,
            require_surface_extension: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VulkanDeviceCreationOptions {
    /// Specifies extensions to enable if available on the logical device
    pub device_extensions: Vec<String>,

    /// Selects the first suitable discrete gpu available
    pub prefer_discrete_gpu: bool,

    pub require_compute_bit: bool,

    /// Requires and enables VK_KHR_swapchain on the created device. Disable for offscreen-only
    /// rendering with drivers that don't expose the extension.
    pub require_swapchain_extension: bool,
}

impl Default for VulkanDeviceCreationOptions {
    fn default() -> Self {
        Self {
            device_extensions: Vec::new(),
            prefer_discrete_gpu: false,
            require_compute_bit: false,
            require_swapchain_extension: true,
        }
    }
}

/// What a windowing platform adds to the options of the application.
#[derive(Clone, Default)]
pub struct VulkanPlatformSpecificOptions {
    /// The instance extensions the surfaces of the platform need.
    pub required_instance_extensions: Vec<String>,

    /// How the platform finds the entry points of Vulkan.
    pub get_proc_address_delegate: Option<VkGetInstanceProcAddressDelegate>,

    /// Creates a surface the physical devices are asked about: a device
    /// whose queues cannot present to it is not taken.
    pub device_check_surface_factory: Option<VulkanDeviceCheckSurfaceFactory>,

    /// The features every context of the platform has, by the type that
    /// names the feature. A value holds an `Arc` of that type (for
    /// `TypeId::of::<dyn IFoo>()` an `Arc<dyn IFoo>`).
    pub platform_features: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl VulkanPlatformSpecificOptions {
    /// Registers the factory of the surfaces of the platform among the
    /// platform features.
    pub fn set_khr_surface_factory(&mut self, factory: Arc<dyn IVulkanKhrSurfacePlatformSurfaceFactory>) {
        self.platform_features
            .insert(TypeId::of::<dyn IVulkanKhrSurfacePlatformSurfaceFactory>(), Arc::new(factory));
    }

    /// The factory of the surfaces of the platform, when one is registered.
    pub fn khr_surface_factory(&self) -> Option<Arc<dyn IVulkanKhrSurfacePlatformSurfaceFactory>> {
        khr_surface_factory_of(&self.platform_features)
    }
}

/// The factory of the surfaces of a platform among its platform features.
pub(crate) fn khr_surface_factory_of(
    platform_features: &HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
) -> Option<Arc<dyn IVulkanKhrSurfacePlatformSurfaceFactory>> {
    platform_features
        .get(&TypeId::of::<dyn IVulkanKhrSurfacePlatformSurfaceFactory>())?
        .downcast_ref::<Arc<dyn IVulkanKhrSurfacePlatformSurfaceFactory>>()
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IVulkanKhrSurfacePlatformSurface, IVulkanPlatformGraphicsContext};
    use ferroui_base::platform::surfaces::IPlatformRenderSurface;
    use std::rc::Rc;

    #[test]
    fn the_defaults_are_those_of_the_reference() {
        let options = VulkanOptions::default();
        let instance = &options.vulkan_instance_creation_options;
        assert!(instance.custom_get_proc_address_delegate.is_none());
        assert_eq!(instance.application_name, None);
        assert_eq!(instance.vulkan_version, Version::new(1, 1, 0));
        assert!(instance.instance_extensions.is_empty());
        assert!(instance.enabled_layers.is_empty());
        assert!(!instance.use_debug);
        assert!(instance.require_surface_extension);

        let device = &options.vulkan_device_creation_options;
        assert!(device.device_extensions.is_empty());
        assert!(!device.prefer_discrete_gpu);
        assert!(!device.require_compute_bit);
        assert!(device.require_swapchain_extension);

        assert!(options.custom_shared_device.is_none());

        let platform = VulkanPlatformSpecificOptions::default();
        assert!(platform.required_instance_extensions.is_empty());
        assert!(platform.get_proc_address_delegate.is_none());
        assert!(platform.device_check_surface_factory.is_none());
        assert!(platform.platform_features.is_empty());
        assert!(platform.khr_surface_factory().is_none());
    }

    #[test]
    fn a_version_prints_its_three_parts() {
        assert_eq!(Version::new(1, 1, 0).to_string(), "1.1.0");
    }

    struct Factory;

    impl IVulkanKhrSurfacePlatformSurfaceFactory for Factory {
        fn can_render_to_surface(
            &self,
            _context: &dyn IVulkanPlatformGraphicsContext,
            _surface: &Arc<dyn IPlatformRenderSurface>,
        ) -> bool {
            false
        }

        fn create_surface(
            &self,
            _context: &dyn IVulkanPlatformGraphicsContext,
            _surface: &Arc<dyn IPlatformRenderSurface>,
        ) -> Rc<dyn IVulkanKhrSurfacePlatformSurface> {
            unreachable!()
        }
    }

    #[test]
    fn the_surface_factory_is_a_platform_feature() {
        let mut platform = VulkanPlatformSpecificOptions::default();
        let factory: Arc<dyn IVulkanKhrSurfacePlatformSurfaceFactory> = Arc::new(Factory);
        platform.set_khr_surface_factory(factory.clone());

        assert_eq!(platform.platform_features.len(), 1);
        let found = platform.khr_surface_factory().expect("the factory is registered");
        assert!(Arc::ptr_eq(&found, &factory));

        // The options are copied into the platform graphics: the copy has the feature too.
        let copy = platform.clone();
        assert!(Arc::ptr_eq(&copy.khr_surface_factory().unwrap(), &factory));
    }
}
