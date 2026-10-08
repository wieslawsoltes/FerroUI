use super::{IBitmapImpl, IPlatformHandle, PlatformGraphicsDrmFormat, PlatformGraphicsExternalImageProperties};
use crate::rendering::composition::{CompositionGpuImportedImageSynchronizationCapabilities, ICompositionImportableSharedGpuContextImage};
use std::rc::Rc;

/// The import of GPU objects created outside of the framework into the
/// render context: a feature of the render interface context.
pub trait IExternalObjectsRenderInterfaceContextFeature {
    /// Returns the list of image handle types supported by the current GPU backend, see [`KnownPlatformGraphicsExternalImageHandleTypes`](super::KnownPlatformGraphicsExternalImageHandleTypes)
    fn supported_image_handle_types(&self) -> Vec<String>;

    /// Returns the list of semaphore types supported by the current GPU backend, see [`KnownPlatformGraphicsExternalSemaphoreHandleTypes`](super::KnownPlatformGraphicsExternalSemaphoreHandleTypes)
    fn supported_semaphore_types(&self) -> Vec<String>;

    /// Returns the DRM format/modifier pairs the current GPU backend can import as
    /// [`KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR`](super::KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR) images,
    /// or `None` when the backend cannot enumerate them.
    fn supported_dma_buf_formats(&self) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        None
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage>;

    fn import_shared_image(
        &self,
        image: Rc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage>;

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore>;

    fn get_synchronization_capabilities(&self, image_handle_type: &str)
        -> CompositionGpuImportedImageSynchronizationCapabilities;

    fn device_uuid(&self) -> Option<Vec<u8>>;

    fn device_luid(&self) -> Option<Vec<u8>>;
}

/// This interface allows proper management of ref-counted platform handles.
/// If we immediately wrap the handle, the caller can destroy its copy immediately after the call
/// This is needed for MoltenVK-based users that can e.g. get an MTLSharedEvent from a VkSemaphore.
/// This does NOT actually increase the ref-counter of MTLSharedEvent, since it's declared as
/// __unsafe_unretained in vulkan headers.
/// Same happens with exporting an IOSurfaceRef from a VkImage.
/// So in a case when the VkSemaphore or VkImage is destroyed, the "handle" which is actually a pointer
/// will be pointing to a dead object.
/// To prevent this we need to increase the reference counter in a handle-specific means
/// synchronously before returning control back to the user.
///
/// This is not needed for fds or DXGI handles, since those are _created_ on demand as proper NT handles
pub trait IExternalObjectsHandleWrapRenderInterfaceContextFeature {
    fn wrap_image_handle_on_any_thread(
        &self,
        handle: &Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Option<Rc<dyn IExternalObjectsWrappedGpuHandle>>;

    fn wrap_semaphore_handle_on_any_thread(
        &self,
        handle: &Rc<dyn IPlatformHandle>,
    ) -> Option<Rc<dyn IExternalObjectsWrappedGpuHandle>>;
}

/// A platform handle that holds a reference of its own to the GPU object;
/// disposing it releases that reference.
pub trait IExternalObjectsWrappedGpuHandle: IPlatformHandle {
    fn dispose(&self);

    /// The handle as a plain platform handle (an explicit upcast).
    fn as_platform_handle(self: Rc<Self>) -> Rc<dyn IPlatformHandle>;
}

/// A GPU object imported into the render context.
pub trait IPlatformRenderInterfaceImportedObject {
    fn dispose(&self);
}

/// A GPU image imported into the render context.
pub trait IPlatformRenderInterfaceImportedImage: IPlatformRenderInterfaceImportedObject {
    fn snapshot_with_keyed_mutex(&self, acquire_index: u32, release_index: u32) -> std::sync::Arc<crate::platform::SharedBitmapImpl>;

    fn snapshot_with_semaphores(
        &self,
        wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> std::sync::Arc<crate::platform::SharedBitmapImpl>;

    fn snapshot_with_timeline_semaphores(
        &self,
        wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        wait_for_value: u64,
        signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        signal_value: u64,
    ) -> std::sync::Arc<crate::platform::SharedBitmapImpl>;

    fn snapshot_with_automatic_sync(&self) -> std::sync::Arc<crate::platform::SharedBitmapImpl>;
}

/// A GPU semaphore imported into the render context.
pub trait IPlatformRenderInterfaceImportedSemaphore: IPlatformRenderInterfaceImportedObject {}
