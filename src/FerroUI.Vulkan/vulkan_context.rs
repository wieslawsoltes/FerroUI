use crate::unmanaged_interop::{VulkanDeviceApi, VulkanInstanceApi};
use crate::vulkan_options::khr_surface_factory_of;
use crate::{
    try_get_vulkan_khr_surface, try_get_vulkan_render_target_surface, IVulkanContextExternalObjectsFeature,
    IVulkanDevice, IVulkanInstance, IVulkanKhrSurfacePlatformSurfaceFactory, IVulkanPlatformGraphicsContext,
    IVulkanRenderTarget, VulkanError, VulkanExternalObjectsFeature, VulkanKhrRenderTarget,
};
use ash::vk;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext};
use ferroui_base::reactive::IDisposable;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// The graphics context of a Vulkan device: the entry points of the device
/// and of its instance, the features of the platform, and the render
/// targets of surfaces.
///
/// A context belongs to the thread that created it.
pub(crate) struct VulkanContext {
    surface_factory: Option<Arc<dyn IVulkanKhrSurfacePlatformSurfaceFactory>>,
    external_objects_feature: RefCell<Option<Rc<VulkanExternalObjectsFeature>>>,
    device: Arc<dyn IVulkanDevice>,
    instance_api: VulkanInstanceApi,
    device_api: VulkanDeviceApi,
    this: Weak<VulkanContext>,
}

impl VulkanContext {
    pub fn new(
        device: Arc<dyn IVulkanDevice>,
        platform_features: &HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    ) -> Result<Rc<Self>, VulkanError> {
        let (instance_api, device_api, surface_factory) = {
            let lock = device.lock();
            let instance_api = VulkanInstanceApi::new(device.instance());
            let device_api = VulkanDeviceApi::new(&*device);
            let surface_factory = khr_surface_factory_of(platform_features);
            lock.dispose();
            (instance_api, device_api, surface_factory)
        };

        let context = Rc::new_cyclic(|this| Self {
            surface_factory,
            external_objects_feature: RefCell::new(None),
            device,
            instance_api,
            device_api,
            this: this.clone(),
        });

        let instance_extensions = context.instance().enabled_extensions();
        let device_extensions = context.device.enabled_extensions();
        if has_all(&instance_extensions, VulkanExternalObjectsFeature::REQUIRED_INSTANCE_EXTENSIONS)
            && has_all(&device_extensions, VulkanExternalObjectsFeature::required_device_extensions())
        {
            // The feature holds its context, and the context its feature:
            // `dispose` lets go of the feature.
            let feature = VulkanExternalObjectsFeature::new(context.clone())?;
            *context.external_objects_feature.borrow_mut() = Some(Rc::new(feature));
        }

        Ok(context)
    }

    fn this(&self) -> Rc<dyn IVulkanPlatformGraphicsContext> {
        self.this.upgrade().expect("a context that is called is alive")
    }
}

/// Whether every name of `required` is among `enabled`.
pub(crate) fn has_all(enabled: &[String], required: &[&str]) -> bool {
    required.iter().all(|ext| enabled.iter().any(|enabled| enabled == ext))
}

impl IOptionalFeatureProvider for VulkanContext {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IVulkanContextExternalObjectsFeature>() {
            let feature: Rc<dyn IVulkanContextExternalObjectsFeature> =
                self.external_objects_feature.borrow().clone()?;
            return Some(Rc::new(feature));
        }

        if feature_type == TypeId::of::<dyn IVulkanKhrSurfacePlatformSurfaceFactory>() {
            return Some(Rc::new(self.surface_factory.clone()?));
        }

        // The context as a Vulkan context: how a render backend recognises
        // it among the graphics contexts.
        if feature_type == TypeId::of::<dyn IVulkanPlatformGraphicsContext>() {
            return Some(Rc::new(self.this.upgrade()? as Rc<dyn IVulkanPlatformGraphicsContext>));
        }

        None
    }
}

impl IPlatformGraphicsContext for VulkanContext {
    fn is_lost(&self) -> bool {
        self.device.is_lost()
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.device.lock()
    }

    fn dispose(&self) {
        if let Some(feature) = self.external_objects_feature.borrow_mut().take() {
            let lock = self.device.lock();
            feature.dispose();
            lock.dispose();
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IVulkanPlatformGraphicsContext for VulkanContext {
    fn device(&self) -> Arc<dyn IVulkanDevice> {
        self.device.clone()
    }

    fn instance(&self) -> Arc<dyn IVulkanInstance> {
        self.device.instance()
    }

    fn instance_api(&self) -> &VulkanInstanceApi {
        &self.instance_api
    }

    fn device_api(&self) -> &VulkanDeviceApi {
        &self.device_api
    }

    fn device_handle(&self) -> vk::Device {
        vk::Handle::from_raw(self.device.handle() as u64)
    }

    fn physical_device_handle(&self) -> vk::PhysicalDevice {
        vk::Handle::from_raw(self.device.physical_device_handle() as u64)
    }

    fn instance_handle(&self) -> vk::Instance {
        vk::Handle::from_raw(self.instance().handle() as u64)
    }

    fn main_queue_handle(&self) -> vk::Queue {
        vk::Handle::from_raw(self.device.main_queue_handle() as u64)
    }

    fn graphics_queue_family_index(&self) -> u32 {
        self.device.graphics_queue_family_index()
    }

    fn create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IVulkanRenderTarget> {
        let this = self.this();
        for surf in surfaces {
            if let Some(render_target_surface) = try_get_vulkan_render_target_surface(&**surf) {
                return render_target_surface.create_render_target(&this);
            }

            let khr_surface = if let Some(khr) = try_get_vulkan_khr_surface(&**surf) {
                khr
            } else if let Some(factory) =
                self.surface_factory.as_ref().filter(|factory| factory.can_render_to_surface(self, surf))
            {
                factory.create_surface(self, surf)
            } else {
                continue;
            };
            return match VulkanKhrRenderTarget::new(khr_surface, this) {
                Ok(target) => Rc::new(target),
                Err(error) => panic!("{error}"),
            };
        }

        panic!("{}", VulkanError::new("Unable to find a suitable platform surface"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_required_names_have_to_be_enabled() {
        let enabled = vec!["VK_A".to_string(), "VK_B".to_string()];
        assert!(has_all(&enabled, &["VK_B", "VK_A"]));
        assert!(has_all(&enabled, &[]));
        assert!(!has_all(&enabled, &["VK_A", "VK_C"]));
        assert!(!has_all(&[], &["VK_A"]));
    }
}
