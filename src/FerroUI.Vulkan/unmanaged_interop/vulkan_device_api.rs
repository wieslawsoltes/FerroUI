use super::vulkan_global_api::{proc_loader, read_into_vec};
use crate::IVulkanDevice;
use ash::vk;
use std::ffi::c_void;
use std::ptr;

/// The entry points of Vulkan that are found through a device.
///
/// The handles given to a member have to be handles of the device the table
/// was made for, which is what makes the calls sound; the members take
/// typed handles, and the objects of this crate pass none but those they
/// received from the same device. A structure with a lifetime covers what
/// it points to.
pub struct VulkanDeviceApi {
    fp: ash::DeviceFnV1_0,
    swapchain: ash::khr::swapchain::DeviceFn,
    external_semaphore_fd: ash::khr::external_semaphore_fd::DeviceFn,
    external_semaphore_win32: ash::khr::external_semaphore_win32::DeviceFn,
}

/// Turns the result of a creating function and what it wrote into a value.
fn created<T>(result: vk::Result, value: T) -> Result<T, vk::Result> {
    match result {
        vk::Result::SUCCESS => Ok(value),
        error => Err(error),
    }
}

impl VulkanDeviceApi {
    /// Resolves the entry points of `device`: each is asked of the device
    /// and, when the device does not have it, of its instance. One neither
    /// has (the swap chain functions without `VK_KHR_swapchain`, for
    /// instance) is replaced by a function that stops the program when it
    /// is called.
    pub fn new(device: &dyn IVulkanDevice) -> Self {
        let instance = device.instance();
        let device_handle = device.handle();
        let instance_handle = instance.handle();
        let lookup = |name: &str| {
            let addr = instance.get_device_proc_address(device_handle, name);
            if addr != 0 {
                return addr;
            }
            instance.get_instance_proc_address(instance_handle, name)
        };
        Self {
            fp: ash::DeviceFnV1_0::load(proc_loader(lookup)),
            swapchain: ash::khr::swapchain::DeviceFn::load(proc_loader(lookup)),
            external_semaphore_fd: ash::khr::external_semaphore_fd::DeviceFn::load(proc_loader(lookup)),
            external_semaphore_win32: ash::khr::external_semaphore_win32::DeviceFn::load(proc_loader(lookup)),
        }
    }

    /// `vkCreateFence`.
    pub fn create_fence(&self, device: vk::Device, create_info: &vk::FenceCreateInfo<'_>) -> Result<vk::Fence, vk::Result> {
        let mut fence = vk::Fence::null();
        // SAFETY: the structure outlives the call; the result is a local.
        let result = unsafe { (self.fp.create_fence)(device, create_info, ptr::null(), &mut fence) };
        created(result, fence)
    }

    /// `vkDestroyFence`.
    pub fn destroy_fence(&self, device: vk::Device, fence: vk::Fence) {
        // SAFETY: the caller owns the fence and gives it up.
        unsafe { (self.fp.destroy_fence)(device, fence, ptr::null()) };
    }

    /// `vkCreateCommandPool`.
    pub fn create_command_pool(
        &self,
        device: vk::Device,
        create_info: &vk::CommandPoolCreateInfo<'_>,
    ) -> Result<vk::CommandPool, vk::Result> {
        let mut pool = vk::CommandPool::null();
        // SAFETY: the structure outlives the call; the result is a local.
        let result = unsafe { (self.fp.create_command_pool)(device, create_info, ptr::null(), &mut pool) };
        created(result, pool)
    }

    /// `vkDestroyCommandPool`.
    pub fn destroy_command_pool(&self, device: vk::Device, pool: vk::CommandPool) {
        // SAFETY: the caller owns the pool and gives it up.
        unsafe { (self.fp.destroy_command_pool)(device, pool, ptr::null()) };
    }

    /// `vkAllocateCommandBuffers`: as many buffers as the structure asks
    /// for.
    pub fn allocate_command_buffers(
        &self,
        device: vk::Device,
        allocate_info: &vk::CommandBufferAllocateInfo<'_>,
    ) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
        let mut buffers = vec![vk::CommandBuffer::null(); allocate_info.command_buffer_count as usize];
        // SAFETY: the buffer has room for the count of the structure.
        let result = unsafe { (self.fp.allocate_command_buffers)(device, allocate_info, buffers.as_mut_ptr()) };
        created(result, buffers)
    }

    /// `vkFreeCommandBuffers`.
    pub fn free_command_buffers(&self, device: vk::Device, command_pool: vk::CommandPool, buffers: &[vk::CommandBuffer]) {
        // SAFETY: the pointer and the count are those of the slice.
        unsafe { (self.fp.free_command_buffers)(device, command_pool, buffers.len() as u32, buffers.as_ptr()) };
    }

    /// `vkWaitForFences`.
    pub fn wait_for_fences(&self, device: vk::Device, fences: &[vk::Fence], wait_all: bool, timeout: u64) -> vk::Result {
        // SAFETY: the pointer and the count are those of the slice.
        unsafe {
            (self.fp.wait_for_fences)(device, fences.len() as u32, fences.as_ptr(), wait_all as vk::Bool32, timeout)
        }
    }

    /// `vkGetFenceStatus`.
    pub fn get_fence_status(&self, device: vk::Device, fence: vk::Fence) -> vk::Result {
        // SAFETY: a call with handles only.
        unsafe { (self.fp.get_fence_status)(device, fence) }
    }

    /// `vkBeginCommandBuffer`.
    pub fn begin_command_buffer(
        &self,
        command_buffer: vk::CommandBuffer,
        begin_info: &vk::CommandBufferBeginInfo<'_>,
    ) -> vk::Result {
        // SAFETY: the structure outlives the call.
        unsafe { (self.fp.begin_command_buffer)(command_buffer, begin_info) }
    }

    /// `vkEndCommandBuffer`.
    pub fn end_command_buffer(&self, command_buffer: vk::CommandBuffer) -> vk::Result {
        // SAFETY: a call with a handle only.
        unsafe { (self.fp.end_command_buffer)(command_buffer) }
    }

    /// `vkCreateSemaphore`.
    pub fn create_semaphore(
        &self,
        device: vk::Device,
        create_info: &vk::SemaphoreCreateInfo<'_>,
    ) -> Result<vk::Semaphore, vk::Result> {
        let mut semaphore = vk::Semaphore::null();
        // SAFETY: the structure outlives the call; the result is a local.
        let result = unsafe { (self.fp.create_semaphore)(device, create_info, ptr::null(), &mut semaphore) };
        created(result, semaphore)
    }

    /// `vkDestroySemaphore`.
    pub fn destroy_semaphore(&self, device: vk::Device, semaphore: vk::Semaphore) {
        // SAFETY: the caller owns the semaphore and gives it up.
        unsafe { (self.fp.destroy_semaphore)(device, semaphore, ptr::null()) };
    }

    /// `vkResetFences`.
    pub fn reset_fences(&self, device: vk::Device, fences: &[vk::Fence]) -> vk::Result {
        // SAFETY: the pointer and the count are those of the slice.
        unsafe { (self.fp.reset_fences)(device, fences.len() as u32, fences.as_ptr()) }
    }

    /// `vkQueueSubmit`: one batch of command buffers that waits for
    /// `wait_semaphores` (each at the stage of the same index of
    /// `wait_dst_stage_mask`, which may be longer) and signals
    /// `signal_semaphores` and `fence`.
    ///
    /// # Panics
    /// Panics when there are fewer stage masks than semaphores to wait for.
    pub fn queue_submit(
        &self,
        queue: vk::Queue,
        wait_semaphores: &[vk::Semaphore],
        wait_dst_stage_mask: &[vk::PipelineStageFlags],
        command_buffers: &[vk::CommandBuffer],
        signal_semaphores: &[vk::Semaphore],
        fence: vk::Fence,
    ) -> vk::Result {
        assert!(wait_dst_stage_mask.len() >= wait_semaphores.len(), "a stage mask for every semaphore to wait for");
        let submit_info = vk::SubmitInfo {
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_dst_stage_mask.as_ptr(),
            command_buffer_count: command_buffers.len() as u32,
            p_command_buffers: command_buffers.as_ptr(),
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        };
        // SAFETY: the pointers and counts of the structure are those of the
        // slices, which outlive the call.
        unsafe { (self.fp.queue_submit)(queue, 1, &submit_info, fence) }
    }

    /// `vkCreateImage`.
    pub fn create_image(&self, device: vk::Device, create_info: &vk::ImageCreateInfo<'_>) -> Result<vk::Image, vk::Result> {
        let mut image = vk::Image::null();
        // SAFETY: the lifetime of the structure covers its chain; the
        // result is a local.
        let result = unsafe { (self.fp.create_image)(device, create_info, ptr::null(), &mut image) };
        created(result, image)
    }

    /// `vkDestroyImage`.
    pub fn destroy_image(&self, device: vk::Device, image: vk::Image) {
        // SAFETY: the caller owns the image and gives it up.
        unsafe { (self.fp.destroy_image)(device, image, ptr::null()) };
    }

    /// `vkGetImageMemoryRequirements`.
    pub fn get_image_memory_requirements(&self, device: vk::Device, image: vk::Image) -> vk::MemoryRequirements {
        let mut requirements = vk::MemoryRequirements::default();
        // SAFETY: the result is written to a local.
        unsafe { (self.fp.get_image_memory_requirements)(device, image, &mut requirements) };
        requirements
    }

    /// `vkAllocateMemory`.
    pub fn allocate_memory(
        &self,
        device: vk::Device,
        allocate_info: &vk::MemoryAllocateInfo<'_>,
    ) -> Result<vk::DeviceMemory, vk::Result> {
        let mut memory = vk::DeviceMemory::null();
        // SAFETY: the lifetime of the structure covers its chain; the
        // result is a local.
        let result = unsafe { (self.fp.allocate_memory)(device, allocate_info, ptr::null(), &mut memory) };
        created(result, memory)
    }

    /// `vkFreeMemory`.
    pub fn free_memory(&self, device: vk::Device, memory: vk::DeviceMemory) {
        // SAFETY: the caller owns the memory and gives it up.
        unsafe { (self.fp.free_memory)(device, memory, ptr::null()) };
    }

    /// `vkBindImageMemory`.
    pub fn bind_image_memory(
        &self,
        device: vk::Device,
        image: vk::Image,
        memory: vk::DeviceMemory,
        memory_offset: vk::DeviceSize,
    ) -> vk::Result {
        // SAFETY: a call with handles only.
        unsafe { (self.fp.bind_image_memory)(device, image, memory, memory_offset) }
    }

    /// `vkCreateImageView`.
    pub fn create_image_view(
        &self,
        device: vk::Device,
        create_info: &vk::ImageViewCreateInfo<'_>,
    ) -> Result<vk::ImageView, vk::Result> {
        let mut view = vk::ImageView::null();
        // SAFETY: the structure outlives the call; the result is a local.
        let result = unsafe { (self.fp.create_image_view)(device, create_info, ptr::null(), &mut view) };
        created(result, view)
    }

    /// `vkDestroyImageView`.
    pub fn destroy_image_view(&self, device: vk::Device, image_view: vk::ImageView) {
        // SAFETY: the caller owns the view and gives it up.
        unsafe { (self.fp.destroy_image_view)(device, image_view, ptr::null()) };
    }

    /// `vkCreateBuffer`.
    pub fn create_buffer(&self, device: vk::Device, create_info: &vk::BufferCreateInfo<'_>) -> Result<vk::Buffer, vk::Result> {
        let mut buffer = vk::Buffer::null();
        // SAFETY: the structure outlives the call; the result is a local.
        let result = unsafe { (self.fp.create_buffer)(device, create_info, ptr::null(), &mut buffer) };
        created(result, buffer)
    }

    /// `vkDestroyBuffer`.
    pub fn destroy_buffer(&self, device: vk::Device, buffer: vk::Buffer) {
        // SAFETY: the caller owns the buffer and gives it up.
        unsafe { (self.fp.destroy_buffer)(device, buffer, ptr::null()) };
    }

    /// `vkGetBufferMemoryRequirements`.
    pub fn get_buffer_memory_requirements(&self, device: vk::Device, buffer: vk::Buffer) -> vk::MemoryRequirements {
        let mut requirements = vk::MemoryRequirements::default();
        // SAFETY: the result is written to a local.
        unsafe { (self.fp.get_buffer_memory_requirements)(device, buffer, &mut requirements) };
        requirements
    }

    /// `vkBindBufferMemory`.
    pub fn bind_buffer_memory(
        &self,
        device: vk::Device,
        buffer: vk::Buffer,
        memory: vk::DeviceMemory,
        memory_offset: vk::DeviceSize,
    ) -> vk::Result {
        // SAFETY: a call with handles only.
        unsafe { (self.fp.bind_buffer_memory)(device, buffer, memory, memory_offset) }
    }

    /// `vkMapMemory`: the address of the mapped range. Reading and writing
    /// through it is the caller's to justify.
    pub fn map_memory(
        &self,
        device: vk::Device,
        memory: vk::DeviceMemory,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
        flags: u32,
    ) -> Result<*mut c_void, vk::Result> {
        let mut data = ptr::null_mut();
        // SAFETY: the result is written to a local.
        let result = unsafe {
            (self.fp.map_memory)(device, memory, offset, size, vk::MemoryMapFlags::from_raw(flags), &mut data)
        };
        created(result, data)
    }

    /// `vkUnmapMemory`. Addresses of the mapping must not be used
    /// afterwards.
    pub fn unmap_memory(&self, device: vk::Device, memory: vk::DeviceMemory) {
        // SAFETY: a call with handles only.
        unsafe { (self.fp.unmap_memory)(device, memory) };
    }

    /// `vkCmdCopyImageToBuffer`.
    pub fn cmd_copy_image_to_buffer(
        &self,
        command_buffer: vk::CommandBuffer,
        src_image: vk::Image,
        src_image_layout: vk::ImageLayout,
        dst_buffer: vk::Buffer,
        regions: &[vk::BufferImageCopy],
    ) {
        // SAFETY: the pointer and the count are those of the slice.
        unsafe {
            (self.fp.cmd_copy_image_to_buffer)(
                command_buffer,
                src_image,
                src_image_layout,
                dst_buffer,
                regions.len() as u32,
                regions.as_ptr(),
            )
        };
    }

    /// `vkCmdPipelineBarrier` with image memory barriers only.
    pub fn cmd_pipeline_barrier(
        &self,
        command_buffer: vk::CommandBuffer,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        dependency_flags: vk::DependencyFlags,
        image_memory_barriers: &[vk::ImageMemoryBarrier<'_>],
    ) {
        // SAFETY: the pointer and the count are those of the slice; there
        // are no other barriers.
        unsafe {
            (self.fp.cmd_pipeline_barrier)(
                command_buffer,
                src_stage_mask,
                dst_stage_mask,
                dependency_flags,
                0,
                ptr::null(),
                0,
                ptr::null(),
                image_memory_barriers.len() as u32,
                image_memory_barriers.as_ptr(),
            )
        };
    }

    /// `vkCreateSwapchainKHR`.
    pub fn create_swapchain_khr(
        &self,
        device: vk::Device,
        create_info: &vk::SwapchainCreateInfoKHR<'_>,
    ) -> Result<vk::SwapchainKHR, vk::Result> {
        let mut swapchain = vk::SwapchainKHR::null();
        // SAFETY: the structure outlives the call; the result is a local.
        let result = unsafe { (self.swapchain.create_swapchain_khr)(device, create_info, ptr::null(), &mut swapchain) };
        created(result, swapchain)
    }

    /// `vkDestroySwapchainKHR`.
    pub fn destroy_swapchain_khr(&self, device: vk::Device, swapchain: vk::SwapchainKHR) {
        // SAFETY: the caller owns the swap chain and gives it up.
        unsafe { (self.swapchain.destroy_swapchain_khr)(device, swapchain, ptr::null()) };
    }

    /// `vkGetSwapchainImagesKHR`.
    pub fn get_swapchain_images_khr(
        &self,
        device: vk::Device,
        swapchain: vk::SwapchainKHR,
    ) -> Result<Vec<vk::Image>, vk::Result> {
        // SAFETY: the function writes at most `count` handles into the
        // buffer `read_into_vec` gives it.
        read_into_vec(|count, data| unsafe { (self.swapchain.get_swapchain_images_khr)(device, swapchain, count, data) })
    }

    /// `vkDeviceWaitIdle`.
    pub fn device_wait_idle(&self, device: vk::Device) -> vk::Result {
        // SAFETY: a call with a handle only.
        unsafe { (self.fp.device_wait_idle)(device) }
    }

    /// `vkQueueWaitIdle`.
    pub fn queue_wait_idle(&self, queue: vk::Queue) -> vk::Result {
        // SAFETY: a call with a handle only.
        unsafe { (self.fp.queue_wait_idle)(queue) }
    }

    /// `vkAcquireNextImageKHR`: the result and the index of the image.
    pub fn acquire_next_image_khr(
        &self,
        device: vk::Device,
        swapchain: vk::SwapchainKHR,
        timeout: u64,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> (vk::Result, u32) {
        let mut image_index = 0;
        // SAFETY: the result is written to a local.
        let result = unsafe {
            (self.swapchain.acquire_next_image_khr)(device, swapchain, timeout, semaphore, fence, &mut image_index)
        };
        (result, image_index)
    }

    /// `vkCmdCopyImage`.
    pub fn cmd_copy_image(
        &self,
        command_buffer: vk::CommandBuffer,
        src_image: vk::Image,
        src_image_layout: vk::ImageLayout,
        dst_image: vk::Image,
        dst_image_layout: vk::ImageLayout,
        regions: &[vk::ImageCopy],
    ) {
        // SAFETY: the pointer and the count are those of the slice.
        unsafe {
            (self.fp.cmd_copy_image)(
                command_buffer,
                src_image,
                src_image_layout,
                dst_image,
                dst_image_layout,
                regions.len() as u32,
                regions.as_ptr(),
            )
        };
    }

    /// `vkCmdBlitImage`.
    #[allow(clippy::too_many_arguments)]
    pub fn cmd_blit_image(
        &self,
        command_buffer: vk::CommandBuffer,
        src_image: vk::Image,
        src_image_layout: vk::ImageLayout,
        dst_image: vk::Image,
        dst_image_layout: vk::ImageLayout,
        regions: &[vk::ImageBlit],
        filter: vk::Filter,
    ) {
        // SAFETY: the pointer and the count are those of the slice.
        unsafe {
            (self.fp.cmd_blit_image)(
                command_buffer,
                src_image,
                src_image_layout,
                dst_image,
                dst_image_layout,
                regions.len() as u32,
                regions.as_ptr(),
                filter,
            )
        };
    }

    /// `vkQueuePresentKHR`: presents the image of one swap chain after
    /// `wait_semaphore` was signalled. The first result is that of the call,
    /// the second that of the swap chain.
    pub fn vk_queue_present_khr(
        &self,
        queue: vk::Queue,
        wait_semaphore: vk::Semaphore,
        swapchain: vk::SwapchainKHR,
        image_index: u32,
    ) -> (vk::Result, vk::Result) {
        let mut result = vk::Result::SUCCESS;
        let present_info = vk::PresentInfoKHR {
            wait_semaphore_count: 1,
            p_wait_semaphores: &wait_semaphore,
            swapchain_count: 1,
            p_swapchains: &swapchain,
            p_image_indices: &image_index,
            p_results: &mut result,
            ..Default::default()
        };
        // SAFETY: the structure points at locals and parameters that outlive
        // the call, one of each as its counts say.
        let call_result = unsafe { (self.swapchain.queue_present_khr)(queue, &present_info) };
        (call_result, result)
    }

    /// `vkImportSemaphoreFdKHR`.
    pub fn import_semaphore_fd_khr(&self, device: vk::Device, info: &vk::ImportSemaphoreFdInfoKHR<'_>) -> vk::Result {
        // SAFETY: the structure outlives the call.
        unsafe { (self.external_semaphore_fd.import_semaphore_fd_khr)(device, info) }
    }

    /// `vkImportSemaphoreWin32HandleKHR`.
    pub fn import_semaphore_win32_handle_khr(
        &self,
        device: vk::Device,
        info: &vk::ImportSemaphoreWin32HandleInfoKHR<'_>,
    ) -> vk::Result {
        // SAFETY: the structure outlives the call.
        unsafe { (self.external_semaphore_win32.import_semaphore_win32_handle_khr)(device, info) }
    }
}
