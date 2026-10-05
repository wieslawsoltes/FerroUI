use super::ServerJobTask;
use crate::platform::{IPlatformHandle, PlatformGraphicsDrmFormat, PlatformGraphicsExternalImageProperties};
use bitflags::bitflags;
use std::rc::Rc;

/// The import of GPU objects created outside of the framework into the
/// compositor.
pub trait ICompositionGpuInterop {
    /// Returns the list of image handle types supported by the current GPU backend, see [`KnownPlatformGraphicsExternalImageHandleTypes`](crate::platform::KnownPlatformGraphicsExternalImageHandleTypes)
    fn supported_image_handle_types(&self) -> Vec<String>;

    /// Returns the list of semaphore types supported by the current GPU backend, see [`KnownPlatformGraphicsExternalSemaphoreHandleTypes`](crate::platform::KnownPlatformGraphicsExternalSemaphoreHandleTypes)
    fn supported_semaphore_types(&self) -> Vec<String>;

    /// Returns the DRM format/modifier pairs the current GPU backend can import as
    /// [`KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR`](crate::platform::KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR) images,
    /// or `None` when the backend cannot enumerate them.
    fn supported_dma_buf_formats(&self) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        None
    }

    /// Returns the supported ways to synchronize access to the imported GPU image
    fn get_synchronization_capabilities(&self, image_handle_type: &str)
        -> CompositionGpuImportedImageSynchronizationCapabilities;

    /// Asynchronously imports a texture. The returned object is immediately usable.
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn ICompositionImportedGpuImage>;

    /// Asynchronously imports a texture. The returned object is immediately usable.
    /// If import operation fails, the caller is responsible for destroying the handle
    ///
    /// `image` is an image that belongs to the same GPU context or the same GPU context sharing group as one used by compositor
    fn import_shared_image(&self, image: Rc<dyn ICompositionImportableSharedGpuContextImage>)
        -> Rc<dyn ICompositionImportedGpuImage>;

    /// Asynchronously imports a semaphore object. The returned object is immediately usable.
    /// If import operation fails, the caller is responsible for destroying the handle
    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn ICompositionImportedGpuSemaphore>;

    /// Asynchronously imports a semaphore object. The returned object is immediately usable.
    ///
    /// `image` is a semaphore that belongs to the same GPU context or the same GPU context sharing group as one used by compositor
    fn import_shared_semaphore(
        &self,
        image: Rc<dyn ICompositionImportableSharedGpuContextSemaphore>,
    ) -> Rc<dyn ICompositionImportedGpuImage>;

    /// Indicates if the device context this instance is associated with is no longer available
    fn is_lost(&self) -> bool;

    /// The LUID of the graphics adapter used by the compositor
    fn device_luid(&self) -> Option<Vec<u8>>;

    fn set_device_luid(&self, value: Option<Vec<u8>>);

    /// The UUID of the graphics adapter used by the compositor
    fn device_uuid(&self) -> Option<Vec<u8>>;

    fn set_device_uuid(&self, value: Option<Vec<u8>>);
}

bitflags! {
    /// The ways the access to an imported GPU image can be synchronized.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct CompositionGpuImportedImageSynchronizationCapabilities: i32 {
        /// Pre-render and after-render semaphores must be provided alongside with the image
        const SEMAPHORES = 1;
        /// Image must be created with D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX or in other compatible way
        const KEYED_MUTEX = 2;
        /// Synchronization and ordering is somehow handled by the underlying platform
        const AUTOMATIC = 4;
        /// Pre-render and after-render timeline semaphores must be provided alongside with the image
        const TIMELINE_SEMAPHORES = 8;
    }
}

/// An imported GPU object that's usable by composition APIs
pub trait ICompositionGpuImportedObject {
    /// Tracks the import status of the object. Once the task is completed,
    /// the user code is allowed to free the resource owner in case when a non-owning
    /// sharing handle was used.
    fn import_completed(&self) -> ServerJobTask<()>;

    /// Indicates if the device context this instance is associated with is no longer available
    fn is_lost(&self) -> bool;

    /// Releases the object on the render thread (`DisposeAsync`).
    fn dispose_async(&self) -> ServerJobTask<()>;
}

/// An imported GPU image object that's usable by composition APIs
pub trait ICompositionImportedGpuImage: ICompositionGpuImportedObject {
    /// The concrete object behind the interface (the cast to the class
    /// upstream performs on it).
    #[doc(hidden)]
    fn as_imported_gpu_image(&self) -> Option<&super::CompositionImportedGpuImage> {
        None
    }
}

/// An imported GPU semaphore object that's usable by composition APIs
pub trait ICompositionImportedGpuSemaphore: ICompositionGpuImportedObject {
    /// The concrete object behind the interface (the cast to the class
    /// upstream performs on it).
    #[doc(hidden)]
    fn as_imported_gpu_semaphore(&self) -> Option<&super::CompositionImportedGpuSemaphore> {
        None
    }
}

/// An GPU object descriptor obtained from a context from the same share group as one used by the compositor
pub trait ICompositionImportableSharedGpuContextObject {
    fn dispose(&self);
}

/// An GPU image descriptor obtained from a context from the same share group as one used by the compositor
pub trait ICompositionImportableSharedGpuContextImage {
    fn dispose(&self);
}

/// An GPU semaphore descriptor obtained from a context from the same share group as one used by the compositor
pub trait ICompositionImportableSharedGpuContextSemaphore {
    fn dispose(&self);
}
