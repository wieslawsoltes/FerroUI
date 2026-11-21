use super::sk_image_info;
use crate::gpu::ganesh::GaneshGrContext;
use crate::gpu::ISkiaGrContext;
use crate::immutable_bitmap::ImmutableBitmap;
use ferroui_base::platform::{
    IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle, IPlatformRenderInterfaceImportedImage,
    IPlatformRenderInterfaceImportedObject, IPlatformRenderInterfaceImportedSemaphore,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::{
    CompositionGpuImportedImageSynchronizationCapabilities, ICompositionImportableSharedGpuContextImage,
};
use ferroui_vulkan::{
    IVulkanContextExternalObjectsFeature, IVulkanExternalImage, IVulkanExternalSemaphore,
    IVulkanPlatformGraphicsContext,
};
use skia_safe::gpu::{backend_render_targets, surfaces, SurfaceOrigin};
use skia_safe::{ColorSpace, ColorType};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Imports images and semaphores that were created outside of the framework
/// into the Ganesh context of a Vulkan Skia GPU.
// The original holds its `VulkanSkiaGpu`; the GPU owns the feature here, so
// the feature holds what it reads of the GPU instead: the two contexts.
pub struct VulkanSkiaExternalObjectsFeature {
    gr_context: Rc<GaneshGrContext>,
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    feature: Rc<dyn IVulkanContextExternalObjectsFeature>,
}

impl VulkanSkiaExternalObjectsFeature {
    pub fn new(
        gr_context: Rc<GaneshGrContext>,
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
        feature: Rc<dyn IVulkanContextExternalObjectsFeature>,
    ) -> Self {
        Self { gr_context, context, feature }
    }
}

impl IExternalObjectsRenderInterfaceContextFeature for VulkanSkiaExternalObjectsFeature {
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        Rc::new(Image {
            gr_context: self.gr_context.clone(),
            context: self.context.clone(),
            inner: RefCell::new(Some(self.feature.import_image(handle, properties.clone()))),
            properties,
        })
    }

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        Rc::new(Semaphore { inner: RefCell::new(Some(self.feature.import_semaphore(handle))) })
    }

    /// # Panics
    /// Always: images of a context of the framework are not shared with a
    /// Vulkan device.
    fn import_shared_image(
        &self,
        _image: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        panic!("Specified method is not supported.");
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        self.feature.get_synchronization_capabilities(image_handle_type)
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        self.feature.device_uuid()
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.feature.device_luid()
    }

    fn supported_image_handle_types(&self) -> Vec<String> {
        self.feature.supported_image_handle_types()
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        self.feature.supported_semaphore_types()
    }
}

struct Semaphore {
    inner: RefCell<Option<Rc<dyn IVulkanExternalSemaphore>>>,
}

impl Semaphore {
    /// # Panics
    /// Panics when the semaphore has been disposed.
    fn inner(&self) -> Rc<dyn IVulkanExternalSemaphore> {
        self.inner
            .borrow()
            .clone()
            .unwrap_or_else(|| panic!("Cannot access a disposed object.\nObject name: 'IVulkanExternalSemaphore'."))
    }
}

impl IPlatformRenderInterfaceImportedObject for Semaphore {
    fn dispose(&self) {
        if let Some(inner) = self.inner.borrow_mut().take() {
            inner.dispose();
        }
    }
}

impl IPlatformRenderInterfaceImportedSemaphore for Semaphore {
}

/// The imported semaphore behind the contract (the cast of the original).
fn imported_semaphore(semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>) -> &Semaphore {
    semaphore
        .as_any()
        .downcast_ref::<Semaphore>()
        .unwrap_or_else(|| panic!("The semaphore was not imported by the Vulkan Skia GPU."))
}

struct Image {
    gr_context: Rc<GaneshGrContext>,
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    inner: RefCell<Option<Rc<dyn IVulkanExternalImage>>>,
    properties: PlatformGraphicsExternalImageProperties,
}

impl IPlatformRenderInterfaceImportedObject for Image {
    fn dispose(&self) {
        if let Some(inner) = self.inner.borrow_mut().take() {
            inner.dispose();
        }
    }
}

impl IPlatformRenderInterfaceImportedImage for Image {
    fn snapshot_with_keyed_mutex(&self, _acquire_index: u32, _release_index: u32) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Panics when the image has been disposed, when a semaphore was not
    /// imported by this backend and when Skia cannot wrap the image (the
    /// exceptions of the original).
    fn snapshot_with_semaphores(
        &self,
        wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> Arc<SharedBitmapImpl> {
        let inner = self.inner.borrow().clone().unwrap_or_else(|| panic!("The imported image has been disposed."));
        let info = inner.info();

        self.gr_context.reset_context();
        imported_semaphore(wait_for_semaphore).inner().submit_wait_semaphore();
        let image_info = sk_image_info(&info, self.context.device().graphics_queue_family_index());
        let render_target =
            backend_render_targets::make_vk((self.properties.width, self.properties.height), &image_info);
        let surface = self.gr_context.with_context(|context| {
            surfaces::wrap_backend_render_target(
                context,
                &render_target,
                if self.properties.top_left_origin { SurfaceOrigin::TopLeft } else { SurfaceOrigin::BottomLeft },
                if self.properties.format == PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm {
                    ColorType::RGBA8888
                } else {
                    ColorType::BGRA8888
                },
                ColorSpace::new_srgb(),
                None,
            )
        });
        let mut surface =
            surface.unwrap_or_else(|| panic!("Surface can't be created with the provided render target"));
        let image = surface.image_snapshot();
        self.gr_context.flush();
        //surface.Canvas.Flush();
        drop(surface);
        drop(render_target);
        imported_semaphore(signal_semaphore).inner().submit_signal_semaphore();

        Arc::new(ImmutableBitmap::from_image(image, None))
    }

    fn snapshot_with_timeline_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _wait_for_value: u64,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_value: u64,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }
}
