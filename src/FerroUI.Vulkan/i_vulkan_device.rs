use crate::unmanaged_interop::{VulkanDeviceApi, VulkanInstanceApi};
use crate::IVulkanRenderTarget;
use ash::vk;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext};
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;
use std::sync::Arc;

/// A Vulkan device with the queue the framework renders with.
///
/// A device is shared by threads (an application may hand one device to the
/// platform graphics, which every render thread then uses), so it is held in
/// an `Arc`; its queue is only used while the device is locked.
pub trait IVulkanDevice: IOptionalFeatureProvider + Send + Sync {
    /// The `VkDevice`.
    fn handle(&self) -> isize;

    /// The `VkPhysicalDevice` the device was created for.
    fn physical_device_handle(&self) -> isize;

    /// The `VkQueue` commands are submitted to. Only valid while the device
    /// is locked by the calling thread.
    fn main_queue_handle(&self) -> isize;

    /// The index of the queue family of the main queue.
    fn graphics_queue_family_index(&self) -> u32;

    /// The instance the device belongs to.
    fn instance(&self) -> Arc<dyn IVulkanInstance>;

    /// Whether the device was lost.
    fn is_lost(&self) -> bool;

    /// Takes the device for the calling thread until the result is
    /// disposed. A thread may lock a device it has locked already.
    fn lock(&self) -> Rc<dyn IDisposable>;

    /// The names of the device extensions that were enabled.
    fn enabled_extensions(&self) -> Vec<String>;

    /// Destroys the device.
    fn dispose(&self);
}

/// A Vulkan instance and the way to its entry points.
pub trait IVulkanInstance: Send + Sync {
    /// The `VkInstance`.
    fn handle(&self) -> isize;

    /// The address of an instance-level entry point (`vkGetInstanceProcAddr`);
    /// zero when the instance does not have it.
    fn get_instance_proc_address(&self, instance: isize, name: &str) -> isize;

    /// The address of a device-level entry point (`vkGetDeviceProcAddr`);
    /// zero when the device does not have it.
    fn get_device_proc_address(&self, device: isize, name: &str) -> isize;

    /// The names of the instance extensions that were enabled.
    fn enabled_extensions(&self) -> Vec<String>;
}

/// A platform graphics context that renders with Vulkan.
///
/// A graphics context that is a Vulkan context announces it through its
/// optional features: `try_get_feature(TypeId::of::<dyn
/// IVulkanPlatformGraphicsContext>())` returns an `Rc<dyn
/// IVulkanPlatformGraphicsContext>`.
///
/// The contract is implemented by this crate only: the members after
/// [`instance`](Self::instance) up to
/// [`graphics_queue_family_index`](Self::graphics_queue_family_index) are
/// what the objects of the crate (images, command buffers, the swap chain)
/// read of their context.
pub trait IVulkanPlatformGraphicsContext: IPlatformGraphicsContext {
    /// The device of the context.
    fn device(&self) -> Arc<dyn IVulkanDevice>;

    /// The instance of the device.
    fn instance(&self) -> Arc<dyn IVulkanInstance>;

    /// The instance-level entry points.
    fn instance_api(&self) -> &VulkanInstanceApi;

    /// The device-level entry points.
    fn device_api(&self) -> &VulkanDeviceApi;

    /// The device as a typed handle.
    fn device_handle(&self) -> vk::Device;

    /// The physical device as a typed handle.
    fn physical_device_handle(&self) -> vk::PhysicalDevice;

    /// The instance as a typed handle.
    fn instance_handle(&self) -> vk::Instance;

    /// The main queue as a typed handle. Only valid while the device is
    /// locked by the calling thread.
    fn main_queue_handle(&self) -> vk::Queue;

    /// The index of the queue family of the main queue.
    fn graphics_queue_family_index(&self) -> u32;

    /// Creates a render target for the first of the surfaces the context
    /// can render to.
    ///
    /// # Panics
    /// Panics when none of the surfaces is suitable, and when the swap chain
    /// of the surface cannot be created.
    fn create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IVulkanRenderTarget>;
}

impl dyn IVulkanPlatformGraphicsContext {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
