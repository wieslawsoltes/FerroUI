use crate::OpenGlException;
use ferroui_base::platform::{
    IPlatformHandle, PlatformGraphicsDrmFormat, PlatformGraphicsExternalImageFormat,
    PlatformGraphicsExternalImageProperties,
};
use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
use ferroui_base::PixelSize;
use std::rc::Rc;

/// The feature of an OpenGL context that imports and exports images and semaphores shared
/// with other graphics APIs.
///
/// A context announces the feature through its optional features:
/// `try_get_feature(TypeId::of::<dyn IGlContextExternalObjectsFeature>())` returns an
/// `Rc<dyn IGlContextExternalObjectsFeature>`.
///
/// The members that create or import an object return the failure the original throws
/// (an unsupported handle type, an OpenGL error) as an `Err`.
pub trait IGlContextExternalObjectsFeature {
    fn supported_importable_external_image_types(&self) -> Vec<String>;

    fn supported_exportable_external_image_types(&self) -> Vec<String>;

    fn supported_importable_external_semaphore_types(&self) -> Vec<String>;

    fn supported_exportable_external_semaphore_types(&self) -> Vec<String>;

    fn get_supported_formats_for_external_memory_type(&self, type_: &str) -> Vec<PlatformGraphicsExternalImageFormat>;

    fn supported_dma_buf_formats(&self) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        None
    }

    fn create_image(
        &self,
        type_: &str,
        size: PixelSize,
        format: PlatformGraphicsExternalImageFormat,
    ) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlException>;

    fn create_semaphore(&self, type_: &str) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlException>;

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Result<Rc<dyn IGlExternalImageTexture>, OpenGlException>;

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Result<Rc<dyn IGlExternalSemaphore>, OpenGlException>;

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities;

    fn device_luid(&self) -> Option<Vec<u8>>;

    fn device_uuid(&self) -> Option<Vec<u8>>;
}

/// A semaphore shared with another graphics API.
pub trait IGlExternalSemaphore {
    fn wait_semaphore(&self, texture: &dyn IGlExternalImageTexture);

    fn signal_semaphore(&self, texture: &dyn IGlExternalImageTexture);

    fn wait_timeline_semaphore(&self, texture: &dyn IGlExternalImageTexture, value: u64);

    fn signal_timeline_semaphore(&self, texture: &dyn IGlExternalImageTexture, value: u64);

    /// Releases the semaphore.
    fn dispose(&self);
}

/// A semaphore created by the context that another graphics API can import.
pub trait IGlExportableExternalSemaphore: IGlExternalSemaphore {
    fn get_handle(&self) -> Rc<dyn IPlatformHandle>;
}

/// A texture backed by an image shared with another graphics API.
pub trait IGlExternalImageTexture {
    fn acquire_keyed_mutex(&self, key: u32);

    fn release_keyed_mutex(&self, key: u32);

    fn texture_id(&self) -> i32;

    fn internal_format(&self) -> i32;

    fn texture_type(&self) -> i32;

    fn properties(&self) -> PlatformGraphicsExternalImageProperties;

    /// Releases the texture.
    fn dispose(&self);
}

/// A texture created by the context whose image another graphics API can import.
pub trait IGlExportableExternalImageTexture: IGlExternalImageTexture {
    fn get_handle(&self) -> Rc<dyn IPlatformHandle>;
}
