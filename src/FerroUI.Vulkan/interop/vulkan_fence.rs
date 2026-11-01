use crate::{IVulkanPlatformGraphicsContext, VulkanError, VulkanExceptionExtensions, VulkanResultExtensions};
use ash::vk;
use std::cell::Cell;
use std::rc::Rc;

/// A fence of the device of a context.
pub(crate) struct VulkanFence {
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    handle: Cell<vk::Fence>,
}

impl VulkanFence {
    pub fn new(
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
        flags: vk::FenceCreateFlags,
    ) -> Result<Self, VulkanError> {
        let fence_create_info = vk::FenceCreateInfo::default().flags(flags);
        let handle = context
            .device_api()
            .create_fence(context.device_handle(), &fence_create_info)
            .throw_on_error("vkCreateFence")?;
        Ok(Self { context, handle: Cell::new(handle) })
    }

    pub fn handle(&self) -> vk::Fence {
        self.handle.get()
    }

    pub fn dispose(&self) {
        self.context.device_api().destroy_fence(self.context.device_handle(), self.handle.get());
        self.handle.set(vk::Fence::null());
    }

    pub fn is_signaled(&self) -> bool {
        self.context.device_api().get_fence_status(self.context.device_handle(), self.handle.get())
            == vk::Result::SUCCESS
    }

    /// Waits without a time limit.
    pub fn wait(&self) -> Result<(), VulkanError> {
        self.wait_for(u64::MAX)
    }

    pub fn wait_for(&self, timeout: u64) -> Result<(), VulkanError> {
        self.context
            .device_api()
            .wait_for_fences(self.context.device_handle(), &[self.handle.get()], true, timeout)
            .throw_on_error("vkWaitForFences")
    }
}
