use super::{VulkanCommandBufferPool, VulkanFence, VulkanSemaphore};
use crate::{IVulkanPlatformGraphicsContext, VulkanError, VulkanExceptionExtensions};
use ash::vk;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// A command buffer of a pool, recorded and submitted once.
pub(crate) struct VulkanCommandBuffer {
    pool: Weak<VulkanCommandBufferPool>,
    pool_handle: vk::CommandPool,
    handle: Cell<vk::CommandBuffer>,
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    fence: VulkanFence,
    has_ended: Cell<bool>,
    has_started: Cell<bool>,
    this: Weak<VulkanCommandBuffer>,
}

impl VulkanCommandBuffer {
    pub fn new(
        pool: &Rc<VulkanCommandBufferPool>,
        handle: vk::CommandBuffer,
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
    ) -> Result<Rc<Self>, VulkanError> {
        let fence = VulkanFence::new(context.clone(), vk::FenceCreateFlags::SIGNALED)?;
        Ok(Rc::new_cyclic(|this| Self {
            pool: Rc::downgrade(pool),
            pool_handle: pool.handle(),
            handle: Cell::new(handle),
            context,
            fence,
            has_ended: Cell::new(false),
            has_started: Cell::new(false),
            this: this.clone(),
        }))
    }

    pub fn handle(&self) -> vk::CommandBuffer {
        self.handle.get()
    }

    /// Waits until the commands have run, then frees the buffer.
    pub fn dispose(&self) {
        if self.fence.handle() != vk::Fence::null() {
            // The reference lets a failure of the wait escape; nothing can be
            // done about it while releasing, and the buffer is freed anyway.
            let _ = self.fence.wait();
            self.fence.dispose();
        }
        if self.handle.get() != vk::CommandBuffer::null() {
            self.context.device_api().free_command_buffers(
                self.context.device_handle(),
                self.pool_handle,
                &[self.handle.get()],
            );
            self.handle.set(vk::CommandBuffer::null());
        }
    }

    /// A slot may still hold a submitted buffer after the pool reaped its signalled fence.
    pub fn wait_for_completion(&self) -> Result<(), VulkanError> {
        if self.fence.handle() == vk::Fence::null() { Ok(()) } else { self.fence.wait() }
    }

    pub fn is_finished(&self) -> bool {
        self.fence.is_signaled()
    }

    pub fn begin_recording(&self) -> Result<(), VulkanError> {
        if self.has_started.get() {
            return Ok(());
        }

        let begin_info = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        self.context
            .device_api()
            .begin_command_buffer(self.handle.get(), &begin_info)
            .throw_on_error("vkBeginCommandBuffer")?;
        self.has_started.set(true);
        Ok(())
    }

    pub fn end_recording(&self) -> Result<(), VulkanError> {
        if self.has_started.get() && !self.has_ended.get() {
            self.context.device_api().end_command_buffer(self.handle.get()).throw_on_error("vkEndCommandBuffer")?;
            self.has_ended.set(true);
        }
        Ok(())
    }

    /// Submits the commands without semaphores.
    pub fn submit(&self) -> Result<(), VulkanError> {
        self.submit_with(&[], &[], &[], None)
    }

    /// Submits the commands to the main queue of the device. The fence of
    /// the buffer is signalled when they have run, unless another fence is
    /// given.
    pub fn submit_with(
        &self,
        wait_semaphores: &[&VulkanSemaphore],
        wait_dst_stage_mask: &[vk::PipelineStageFlags],
        signal_semaphores: &[&VulkanSemaphore],
        fence: Option<&VulkanFence>,
    ) -> Result<(), VulkanError> {
        self.end_recording()?;
        let fence_handle = fence.unwrap_or(&self.fence).handle();
        self.context
            .device_api()
            .reset_fences(self.context.device_handle(), &[fence_handle])
            .throw_on_error("vkResetFences")?;

        let wait_semaphores: Vec<vk::Semaphore> = wait_semaphores.iter().map(|semaphore| semaphore.handle()).collect();
        let signal_semaphores: Vec<vk::Semaphore> =
            signal_semaphores.iter().map(|semaphore| semaphore.handle()).collect();

        self.context
            .device_api()
            .queue_submit(
                self.context.main_queue_handle(),
                &wait_semaphores,
                wait_dst_stage_mask,
                &[self.handle.get()],
                &signal_semaphores,
                fence_handle,
            )
            .throw_on_error("vkQueueSubmit")?;

        if let (Some(pool), Some(this)) = (self.pool.upgrade(), self.this.upgrade()) {
            pool.add_submitted_command_buffer(this);
        }
        Ok(())
    }
}
