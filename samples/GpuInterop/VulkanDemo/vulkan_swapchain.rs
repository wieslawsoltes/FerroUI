//! Port of `VulkanDemo/VulkanSwapchain.cs`: the images the Vulkan demo draws its frames into,
//! one after another, while the compositor still shows the previous ones, and the presentation
//! of a frame to the drawing surface with the synchronization of the platform (semaphores of
//! Vulkan, a keyed mutex of Direct3D 11 on Windows, a timeline semaphore that is a Metal shared
//! event on macOS).

use super::vulkan_command_buffer_pool::KeyedMutexSubmitInfo;
use super::vulkan_context::VulkanContext;
use super::vulkan_extensions::VulkanDemoResult;
use super::vulkan_image::VulkanImage;
use super::vulkan_semaphore_pair::VulkanSemaphorePair;
use super::vulkan_timeline_semaphore::VulkanTimelineSemaphore;
use crate::task_support::{join, present_status, start, ServerJobTaskFuture};
use ash::vk;
use ferroui_base::platform::{
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
    PlatformGraphicsExternalImageVulkanProperties,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::{
    CompositionDrawingSurface, ICompositionGpuInterop, ICompositionImportedGpuImage, ICompositionImportedGpuSemaphore,
    ServerJobTask,
};
use ferroui_base::rendering::{ISwapchainImage, SwapchainBase, SwapchainImageFactory, SwapchainImagePresentStatus};
use ferroui_base::threading::JoinHandle;
use ferroui_base::PixelSize;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The disposals of the images of a swapchain that are under way: what `DisposeAsync` of the
/// swapchain awaits upstream, where it awaits the disposal of every image.
pub(super) type Disposals = Rc<RefCell<Vec<JoinHandle<()>>>>;

pub struct VulkanSwapchain {
    base: SwapchainBase<VulkanSwapchainImage>,
    vk: Rc<VulkanContext>,
    interop: Rc<dyn ICompositionGpuInterop>,
    target: CompositionDrawingSurface,
    disposals: Disposals,
}

impl VulkanSwapchain {
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
    pub fn begin_draw(&self, size: PixelSize) -> (Rc<dyn IDisposable>, Rc<VulkanSwapchainImage>) {
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

impl SwapchainImageFactory<VulkanSwapchainImage> for VulkanSwapchain {
    /// # Panics
    /// Panics when the image cannot be created (the exception upstream, which ends the
    /// drawing of the frame).
    fn create_image(&self, size: PixelSize) -> Rc<VulkanSwapchainImage> {
        match VulkanSwapchainImage::new(&self.vk, size, &self.interop, &self.target, &self.disposals) {
            Ok(image) => Rc::new(image),
            Err(error) => panic!("{error}"),
        }
    }
}

pub struct VulkanSwapchainImage {
    vk: Rc<VulkanContext>,
    interop: Rc<dyn ICompositionGpuInterop>,
    target: CompositionDrawingSurface,
    image: Rc<VulkanImage>,
    semaphore_pair: Option<Rc<VulkanSemaphorePair>>,
    timeline_semaphore: Option<Rc<VulkanTimelineSemaphore>>,
    timeline_counter: Cell<u64>,
    available_semaphore: RefCell<Option<Rc<dyn ICompositionImportedGpuSemaphore>>>,
    render_completed_semaphore: RefCell<Option<Rc<dyn ICompositionImportedGpuSemaphore>>>,
    imported_timeline_semaphore: RefCell<Option<Rc<dyn ICompositionImportedGpuSemaphore>>>,
    imported_image: RefCell<Option<Rc<dyn ICompositionImportedGpuImage>>>,
    last_present: RefCell<Option<ServerJobTask<()>>>,
    initial: Cell<bool>,
    size: PixelSize,
    disposals: Disposals,
}

impl VulkanSwapchainImage {
    fn new(
        vk: &Rc<VulkanContext>,
        size: PixelSize,
        interop: &Rc<dyn ICompositionGpuInterop>,
        target: &CompositionDrawingSurface,
        disposals: &Disposals,
    ) -> VulkanDemoResult<Self> {
        let format = if cfg!(target_os = "macos") { vk::Format::B8G8R8A8_UNORM } else { vk::Format::R8G8B8A8_UNORM };
        let image = VulkanImage::new(vk, format, size, true, &interop.supported_image_handle_types())?;
        let (timeline_semaphore, semaphore_pair) = if cfg!(target_os = "macos") {
            (Some(Rc::new(VulkanTimelineSemaphore::new(vk)?)), None)
        } else {
            (None, Some(Rc::new(VulkanSemaphorePair::new(vk, true)?)))
        };
        Ok(Self {
            vk: vk.clone(),
            interop: interop.clone(),
            target: target.clone(),
            image: Rc::new(image),
            semaphore_pair,
            timeline_semaphore,
            timeline_counter: Cell::new(0),
            available_semaphore: RefCell::new(None),
            render_completed_semaphore: RefCell::new(None),
            imported_timeline_semaphore: RefCell::new(None),
            imported_image: RefCell::new(None),
            last_present: RefCell::new(None),
            initial: Cell::new(true),
            size,
            disposals: disposals.clone(),
        })
    }

    pub fn image(&self) -> &VulkanImage {
        &self.image
    }

    /// `DisposeAsync`: waits for the last presentation, then releases what the compositor
    /// imported and the objects of the image.
    fn dispose_async(&self) -> JoinHandle<()> {
        let last_present = self.last_present.borrow().clone();
        let imported_image = self.imported_image.borrow().clone();
        let available_semaphore = self.available_semaphore.borrow().clone();
        let render_completed_semaphore = self.render_completed_semaphore.borrow().clone();
        let semaphore_pair = self.semaphore_pair.clone();
        let timeline_semaphore = self.timeline_semaphore.clone();
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

            if let Some(available_semaphore) = available_semaphore {
                let _ = ServerJobTaskFuture::new(available_semaphore.dispose_async()).await;
            }
            if let Some(render_completed_semaphore) = render_completed_semaphore {
                let _ = ServerJobTaskFuture::new(render_completed_semaphore.dispose_async()).await;
            }
            if let Some(semaphore_pair) = semaphore_pair {
                semaphore_pair.dispose();
            }
            if let Some(timeline_semaphore) = timeline_semaphore {
                timeline_semaphore.dispose();
            }
            image.dispose();
        })
    }

    fn present_core(&self) -> VulkanDemoResult<()> {
        let buffer = self.vk.pool().create_command_buffer();
        buffer.begin_recording();
        self.image.transition_layout_in(
            buffer.internal_handle(),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
        );

        if self.image.is_direct_x_backed() {
            buffer.submit_with(
                &[],
                &[],
                &[],
                None,
                Some(&KeyedMutexSubmitInfo { device_memory: self.image.device_memory(), release_key: Some(1), acquire_key: None }),
                std::ptr::null(),
            );
        } else if let Some(timeline_semaphore) = &self.timeline_semaphore {
            let signal = self.timeline_counter.get() + 1;
            let submit_info = vk::TimelineSemaphoreSubmitInfo {
                p_signal_semaphore_values: &signal,
                signal_semaphore_value_count: 1,
                ..Default::default()
            };
            let signal_semaphores = [timeline_semaphore.handle()];

            buffer.submit_with(
                &[],
                &[],
                &signal_semaphores,
                None,
                None,
                (&submit_info as *const vk::TimelineSemaphoreSubmitInfo).cast(),
            );
        } else {
            let semaphore_pair = self.semaphore_pair.as_ref().expect("the semaphores of the image");
            buffer.submit_with(&[], &[], &[semaphore_pair.render_finished_semaphore()], None, None, std::ptr::null());
        }

        if let Some(timeline_semaphore) = &self.timeline_semaphore {
            if self.imported_timeline_semaphore.borrow().is_none() {
                *self.imported_timeline_semaphore.borrow_mut() = Some(self.interop.import_semaphore(timeline_semaphore.export()?));
            }
        } else if !self.image.is_direct_x_backed() {
            let semaphore_pair = self.semaphore_pair.as_ref().expect("the semaphores of the image");
            if self.available_semaphore.borrow().is_none() {
                *self.available_semaphore.borrow_mut() = Some(self.interop.import_semaphore(semaphore_pair.export(false)?));
            }

            if self.render_completed_semaphore.borrow().is_none() {
                *self.render_completed_semaphore.borrow_mut() =
                    Some(self.interop.import_semaphore(semaphore_pair.export(true)?));
            }
        }

        if self.imported_image.borrow().is_none() {
            let imported = self.interop.import_image(
                self.image.export()?,
                PlatformGraphicsExternalImageProperties {
                    format: if cfg!(target_os = "macos") {
                        PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm
                    } else {
                        PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm
                    },
                    width: self.size.width,
                    height: self.size.height,
                    memory_size: self.image.memory_size(),
                    vulkan_properties: Some(PlatformGraphicsExternalImageVulkanProperties {
                        layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL.as_raw(),
                    }),
                    ..Default::default()
                },
            );
            *self.imported_image.borrow_mut() = Some(imported);
        }
        let imported_image = self.imported_image.borrow().clone().expect("the imported image");
        let imported_timeline_semaphore = self.imported_timeline_semaphore.borrow().clone();
        let last_present = if let Some(imported_timeline_semaphore) = imported_timeline_semaphore {
            let counter = self.timeline_counter.get();
            let task = self.target.update_with_timeline_semaphores_async(
                &*imported_image,
                &*imported_timeline_semaphore,
                counter + 1,
                &*imported_timeline_semaphore,
                counter + 2,
            );
            self.timeline_counter.set(counter + 2);
            task
        } else if self.image.is_direct_x_backed() {
            self.target.update_with_keyed_mutex_async(&*imported_image, 1, 0)
        } else {
            let render_completed_semaphore = self.render_completed_semaphore.borrow().clone().expect("the semaphores");
            let available_semaphore = self.available_semaphore.borrow().clone().expect("the semaphores");
            self.target.update_with_semaphores_async(&*imported_image, &*render_completed_semaphore, &*available_semaphore)
        };
        *self.last_present.borrow_mut() = Some(last_present);
        Ok(())
    }
}

impl ISwapchainImage for VulkanSwapchainImage {
    fn size(&self) -> PixelSize {
        self.size
    }

    fn last_present(&self) -> Option<SwapchainImagePresentStatus> {
        present_status(self.last_present.borrow().as_ref())
    }

    fn begin_draw(&self) {
        let buffer = self.vk.pool().create_command_buffer();
        buffer.begin_recording();

        self.image.transition_layout_in(
            buffer.internal_handle(),
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::AccessFlags::COLOR_ATTACHMENT_READ,
        );

        if self.image.is_direct_x_backed() {
            buffer.submit_with(
                &[],
                &[],
                &[],
                None,
                Some(&KeyedMutexSubmitInfo { acquire_key: Some(0), release_key: None, device_memory: self.image.device_memory() }),
                std::ptr::null(),
            );
        } else if let Some(timeline_semaphore) = &self.timeline_semaphore {
            let wait = self.timeline_counter.get();
            let submit_info = vk::TimelineSemaphoreSubmitInfo {
                p_wait_semaphore_values: &wait,
                wait_semaphore_value_count: 1,
                ..Default::default()
            };
            let wait_semaphores = [timeline_semaphore.handle()];

            buffer.submit_with(
                &wait_semaphores,
                &[],
                &[],
                None,
                None,
                (&submit_info as *const vk::TimelineSemaphoreSubmitInfo).cast(),
            );
        } else if self.initial.get() {
            self.initial.set(false);
            buffer.submit();
        } else {
            let semaphore_pair = self.semaphore_pair.as_ref().expect("the semaphores of the image");
            buffer.submit_with(
                &[semaphore_pair.image_available_semaphore()],
                &[vk::PipelineStageFlags::ALL_GRAPHICS],
                &[],
                None,
                None,
                std::ptr::null(),
            );
        }
    }

    /// # Panics
    /// Panics when the image or a semaphore cannot be exported (the exception upstream, which
    /// ends the drawing of the frame).
    fn present(&self) {
        self.present_core().unwrap_or_else(|e| panic!("{e}"));
    }

    fn dispose(&self) {
        let disposal = self.dispose_async();
        self.disposals.borrow_mut().push(disposal);
    }
}
