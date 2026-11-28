use super::{VulkanCommandBufferPool, VulkanFence, VulkanSemaphore};
use crate::{IVulkanPlatformGraphicsContext, VulkanError, VulkanExceptionExtensions};
use ash::vk;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The most semaphores a submission waits for or signals. The present path
/// waits for one and signals one; the submit copies the handles into arrays
/// of this length, so it allocates nothing.
pub(crate) const MAX_SUBMIT_SEMAPHORES: usize = 4;

/// A command buffer of a pool. A buffer of
/// [`create_command_buffer`](VulkanCommandBufferPool::create_command_buffer)
/// is recorded and submitted once and freed by the pool; a reusable one
/// ([`create_reusable_command_buffer`](VulkanCommandBufferPool::create_reusable_command_buffer))
/// belongs to its creator and is recorded again after
/// [`prepare_for_recording`](Self::prepare_for_recording).
pub(crate) struct VulkanCommandBuffer {
    pool: Weak<VulkanCommandBufferPool>,
    pool_handle: vk::CommandPool,
    handle: Cell<vk::CommandBuffer>,
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    fence: VulkanFence,
    has_ended: Cell<bool>,
    has_started: Cell<bool>,
    /// Kept by its creator and recorded again, not handed to the pool when
    /// submitted.
    reusable: bool,
    /// Submitted, and its fence not yet waited for.
    pending: Cell<bool>,
    this: Weak<VulkanCommandBuffer>,
}

impl VulkanCommandBuffer {
    pub fn new(
        pool: &Rc<VulkanCommandBufferPool>,
        handle: vk::CommandBuffer,
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
        reusable: bool,
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
            reusable,
            pending: Cell::new(false),
            this: this.clone(),
        }))
    }

    pub fn handle(&self) -> vk::CommandBuffer {
        self.handle.get()
    }

    /// Waits until the commands have run, then frees the buffer.
    pub fn dispose(&self) {
        if self.fence.handle() != vk::Fence::null() {
            // Disposing may wait: the buffer and its fence can only be freed
            // when the commands have run. A failure of the wait cannot be
            // handled while releasing, and the buffer is freed anyway.
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

    pub fn is_finished(&self) -> bool {
        self.fence.is_signaled()
    }

    /// Whether the buffer was submitted and its fence not yet waited for.
    #[allow(dead_code)] // Read by the tests.
    pub fn is_pending(&self) -> bool {
        self.pending.get()
    }

    /// Makes a reusable buffer ready to be recorded again: waits until the
    /// commands of its last submission have run, then lets
    /// [`begin_recording`](Self::begin_recording) start a new recording
    /// (the pool resets a buffer when it begins).
    ///
    /// The owner of the buffer reuses it frames after its submission (the
    /// frames in flight of a display), so the wait is for an older frame,
    /// never for the frame just submitted; the fence is not waited for at
    /// all when it was waited for already or the buffer was never submitted.
    pub fn prepare_for_recording(&self) -> Result<(), VulkanError> {
        debug_assert!(self.reusable, "only a reusable command buffer is recorded again");
        if self.pending.get() {
            self.fence.wait()?;
            self.pending.set(false);
        }
        self.has_started.set(false);
        self.has_ended.set(false);
        Ok(())
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

        let (wait_semaphores, wait_count) = semaphore_handles(wait_semaphores);
        let (signal_semaphores, signal_count) = semaphore_handles(signal_semaphores);

        self.context
            .device_api()
            .queue_submit(
                self.context.main_queue_handle(),
                &wait_semaphores[..wait_count],
                wait_dst_stage_mask,
                &[self.handle.get()],
                &signal_semaphores[..signal_count],
                fence_handle,
            )
            .throw_on_error("vkQueueSubmit")?;
        // Only the own fence tells when the buffer may be recorded again.
        self.pending.set(fence.is_none());

        if !self.reusable {
            if let (Some(pool), Some(this)) = (self.pool.upgrade(), self.this.upgrade()) {
                pool.add_submitted_command_buffer(this);
            }
        }
        Ok(())
    }
}

/// The handles of at most [`MAX_SUBMIT_SEMAPHORES`] semaphores in an array,
/// and their number.
fn semaphore_handles(semaphores: &[&VulkanSemaphore]) -> ([vk::Semaphore; MAX_SUBMIT_SEMAPHORES], usize) {
    assert!(semaphores.len() <= MAX_SUBMIT_SEMAPHORES, "at most {MAX_SUBMIT_SEMAPHORES} semaphores a submission");
    let mut handles = [vk::Semaphore::null(); MAX_SUBMIT_SEMAPHORES];
    for (handle, semaphore) in handles.iter_mut().zip(semaphores) {
        *handle = semaphore.handle();
    }
    (handles, semaphores.len())
}
