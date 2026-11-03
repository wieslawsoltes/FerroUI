use crate::{IVulkanPlatformGraphicsContext, VulkanError, VulkanResultExtensions};
use ash::vk;
use std::cell::Cell;
use std::rc::Rc;

/// A semaphore of the device of a context.
pub(crate) struct VulkanSemaphore {
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    handle: Cell<vk::Semaphore>,
}

impl VulkanSemaphore {
    /// Creates a semaphore.
    pub fn new(context: Rc<dyn IVulkanPlatformGraphicsContext>) -> Result<Self, VulkanError> {
        let info = vk::SemaphoreCreateInfo::default();
        let handle = context
            .device_api()
            .create_semaphore(context.device_handle(), &info)
            .throw_on_error("vkCreateSemaphore")?;
        Ok(Self { context, handle: Cell::new(handle) })
    }

    /// Takes over a semaphore of the device.
    #[allow(dead_code)] // The second constructor of the reference, which nothing calls there either.
    pub fn from_handle(context: Rc<dyn IVulkanPlatformGraphicsContext>, handle: vk::Semaphore) -> Self {
        Self { context, handle: Cell::new(handle) }
    }

    pub fn handle(&self) -> vk::Semaphore {
        self.handle.get()
    }

    pub fn dispose(&self) {
        if self.handle.get() != vk::Semaphore::null() {
            self.context.device_api().destroy_semaphore(self.context.device_handle(), self.handle.get());
            self.handle.set(vk::Semaphore::null());
        }
    }
}
