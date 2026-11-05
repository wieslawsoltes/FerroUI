//! Port of `VulkanDemo/VulkanDmaBufSwapchain.cs`.
//!
//! Presents Vulkan-rendered frames to the EGL/OpenGL compositor by exporting them as Linux dma-bufs.
//! The compositor imports them through EglExternalObjectsFeature, which advertises "Automatic"
//! synchronization for dma-buf, so we don't import any semaphores: instead we make sure the GPU has
//! finished writing each frame (vkQueueWaitIdle) before handing the buffer over for sampling.

use super::vulkan_context::VulkanContext;
use super::vulkan_dma_buf_image::VulkanDmaBufImage;
use super::vulkan_image::VulkanImage;
use super::vulkan_swapchain::Disposals;
use crate::native_methods;
use crate::task_support::{join, present_status, start, ServerJobTaskFuture};
use ash::vk;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::{
    CompositionDrawingSurface, ICompositionGpuInterop, ICompositionImportedGpuImage, ServerJobTask,
};
use ferroui_base::rendering::{ISwapchainImage, SwapchainBase, SwapchainImageFactory, SwapchainImagePresentStatus};
use ferroui_base::threading::JoinHandle;
use ferroui_base::PixelSize;
use std::cell::RefCell;
use std::rc::Rc;

pub struct VulkanDmaBufSwapchain {
    base: SwapchainBase<VulkanDmaBufSwapchainImage>,
    vk: Rc<VulkanContext>,
    interop: Rc<dyn ICompositionGpuInterop>,
    target: CompositionDrawingSurface,
    disposals: Disposals,
}

impl VulkanDmaBufSwapchain {
    pub fn new(vk: &Rc<VulkanContext>, interop: &Rc<dyn ICompositionGpuInterop>, target: &CompositionDrawingSurface) -> Self {
        Self {
            base: SwapchainBase::new(),
            vk: vk.clone(),
            interop: interop.clone(),
            target: target.clone(),
            disposals: Rc::default(),
        }
    }

    /// Starts a frame of `size`: the session, whose disposal presents the frame, and the image
    /// to draw it into.
    pub fn begin_draw(&self, size: PixelSize) -> (Rc<dyn IDisposable>, Rc<VulkanDmaBufSwapchainImage>) {
        self.vk.pool().free_used_command_buffers();
        self.base.begin_draw_core(self, size)
    }

    /// Releases the images of the swapchain once the compositor is done with them.
    pub fn dispose_async(&self) -> JoinHandle<()> {
        self.base.dispose();
        let disposals = std::mem::take(&mut *self.disposals.borrow_mut());
        start(async move {
            for disposal in disposals {
                let _ = join(disposal).await;
            }
        })
    }
}

impl SwapchainImageFactory<VulkanDmaBufSwapchainImage> for VulkanDmaBufSwapchain {
    /// # Panics
    /// Panics when the image cannot be created (the exception upstream, which ends the
    /// drawing of the frame).
    fn create_image(&self, size: PixelSize) -> Rc<VulkanDmaBufSwapchainImage> {
        match VulkanDmaBufSwapchainImage::new(&self.vk, size, &self.interop, &self.target, &self.disposals) {
            Ok(image) => Rc::new(image),
            Err(error) => panic!("{error}"),
        }
    }
}

pub struct VulkanDmaBufSwapchainImage {
    vk: Rc<VulkanContext>,
    interop: Rc<dyn ICompositionGpuInterop>,
    target: CompositionDrawingSurface,
    image: Rc<VulkanDmaBufImage>,
    imported_image: RefCell<Option<Rc<dyn ICompositionImportedGpuImage>>>,
    last_present: RefCell<Option<ServerJobTask<()>>>,
    size: PixelSize,
    disposals: Disposals,
}

impl VulkanDmaBufSwapchainImage {
    fn new(
        vk: &Rc<VulkanContext>,
        size: PixelSize,
        interop: &Rc<dyn ICompositionGpuInterop>,
        target: &CompositionDrawingSurface,
        disposals: &Disposals,
    ) -> super::vulkan_extensions::VulkanDemoResult<Self> {
        // The EGL importer reads back as R8G8B8A8 (ABGR8888 in DRM terms); VulkanDmaBufImage maps the format.
        let image = VulkanDmaBufImage::new(vk, vk::Format::R8G8B8A8_UNORM, size)?;
        Ok(Self {
            vk: vk.clone(),
            interop: interop.clone(),
            target: target.clone(),
            image: Rc::new(image),
            imported_image: RefCell::new(None),
            last_present: RefCell::new(None),
            size,
            disposals: disposals.clone(),
        })
    }

    pub fn image(&self) -> &VulkanImage {
        &self.image
    }

    fn present_core(&self) -> super::vulkan_extensions::VulkanDemoResult<()> {
        // Move the freshly rendered image into a layout the external consumer can read, then make sure
        // all GPU work targeting it has finished before the compositor samples from the dma-buf.
        let buffer = self.vk.pool().create_command_buffer();
        buffer.begin_recording();
        self.image.transition_layout_in(buffer.internal_handle(), vk::ImageLayout::GENERAL, vk::AccessFlags::NONE);
        buffer.submit();
        // SAFETY: the queue belongs to the device; nothing else submits to it meanwhile.
        let _ = unsafe { self.vk.device().queue_wait_idle(self.vk.queue()) };

        if self.imported_image.borrow().is_none() {
            let (handle, properties, fds) = self.image.export_dma_buf()?;
            let imported = self.interop.import_image(handle, properties);
            // EGL dups the file descriptors during import, so release ours once the import has run.
            imported.import_completed().on_completed(move || {
                for fd in fds {
                    native_methods::close(fd);
                }
            });
            *self.imported_image.borrow_mut() = Some(imported);
        }

        let imported_image = self.imported_image.borrow().clone().expect("the imported image");
        *self.last_present.borrow_mut() = Some(self.target.update_async(&*imported_image));
        Ok(())
    }

    fn dispose_async(&self) -> JoinHandle<()> {
        let last_present = self.last_present.borrow().clone();
        let imported_image = self.imported_image.borrow().clone();
        let image = self.image.clone();
        start(async move {
            if let Some(last_present) = last_present {
                // A failed presentation ends the method there, as the exception of the awaited
                // task does upstream.
                if ServerJobTaskFuture::new(last_present).await.is_err() {
                    return;
                }
            }
            if let Some(imported_image) = imported_image {
                let _ = ServerJobTaskFuture::new(imported_image.dispose_async()).await;
            }
            image.dispose();
        })
    }
}

impl ISwapchainImage for VulkanDmaBufSwapchainImage {
    fn size(&self) -> PixelSize {
        self.size
    }

    fn last_present(&self) -> Option<SwapchainImagePresentStatus> {
        present_status(self.last_present.borrow().as_ref())
    }

    fn begin_draw(&self) {
        // Nothing to acquire: synchronization is handled by draining the queue in Present(). The image
        // is reused by SwapchainBase only once its previous present task has completed.
    }

    /// # Panics
    /// Panics when the image cannot be exported (the exception upstream, which ends the
    /// drawing of the frame).
    fn present(&self) {
        self.present_core().unwrap_or_else(|e| panic!("{e}"));
    }

    fn dispose(&self) {
        let disposal = self.dispose_async();
        self.disposals.borrow_mut().push(disposal);
    }
}
