use super::VulkanCommandBuffer;
use crate::{IVulkanPlatformGraphicsContext, VulkanError, VulkanResultExtensions};
use ash::vk;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// A command pool of the device of a context, and the command buffers of it
/// that were submitted and not freed yet.
pub(crate) struct VulkanCommandBufferPool {
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    auto_free: bool,
    command_buffers: RefCell<VecDeque<Rc<VulkanCommandBuffer>>>,
    handle: Cell<vk::CommandPool>,
    this: Weak<VulkanCommandBufferPool>,
}

impl VulkanCommandBufferPool {
    /// Creates a pool. With `auto_free` the buffers whose commands have run
    /// are freed whenever a new buffer is created.
    pub fn new(context: Rc<dyn IVulkanPlatformGraphicsContext>, auto_free: bool) -> Result<Rc<Self>, VulkanError> {
        let create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(context.graphics_queue_family_index());

        let handle = context
            .device_api()
            .create_command_pool(context.device_handle(), &create_info)
            .throw_on_error("vkCreateCommandPool")?;

        Ok(Rc::new_cyclic(|this| Self {
            context,
            auto_free,
            command_buffers: RefCell::new(VecDeque::new()),
            handle: Cell::new(handle),
            this: this.clone(),
        }))
    }

    pub fn handle(&self) -> vk::CommandPool {
        self.handle.get()
    }

    /// Frees every submitted buffer, waiting for its commands.
    pub fn free_used_command_buffers(&self) {
        loop {
            let next = self.command_buffers.borrow_mut().pop_front();
            match next {
                Some(buffer) => buffer.dispose(),
                None => break,
            }
        }
    }

    /// Frees the submitted buffers whose commands have run, in the order of
    /// submission up to the first that has not.
    pub fn free_finished_command_buffers(&self) {
        loop {
            let next = {
                let mut command_buffers = self.command_buffers.borrow_mut();
                match command_buffers.front() {
                    Some(next) if next.is_finished() => command_buffers.pop_front(),
                    _ => None,
                }
            };
            match next {
                Some(buffer) => buffer.dispose(),
                None => return,
            }
        }
    }

    pub fn dispose(&self) {
        self.free_used_command_buffers();

        if self.handle.get() != vk::CommandPool::null() {
            self.context.device_api().destroy_command_pool(self.context.device_handle(), self.handle.get());
        }
        self.handle.set(vk::CommandPool::null());
    }

    pub fn create_command_buffer(&self) -> Result<Rc<VulkanCommandBuffer>, VulkanError> {
        if self.auto_free {
            self.free_finished_command_buffers();
        }

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.handle.get())
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);

        let buffer_handle = self
            .context
            .device_api()
            .allocate_command_buffers(self.context.device_handle(), &command_buffer_allocate_info)
            .throw_on_error("vkAllocateCommandBuffers")?[0];

        let this = self.this.upgrade().expect("a pool that is called is alive");
        VulkanCommandBuffer::new(&this, buffer_handle, self.context.clone())
    }

    pub fn add_submitted_command_buffer(&self, buffer: Rc<VulkanCommandBuffer>) {
        self.command_buffers.borrow_mut().push_back(buffer);
    }
}
