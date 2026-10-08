use ferroui_base::platform::{IPlatformHandle, PlatformGraphicsExternalImageProperties};
use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
use std::ffi::c_void;
use std::rc::Rc;

/// Lets a Metal device import images and shared events that were created
/// outside of it (by another device or another process).
///
/// A graphics context that is a Metal device announces the feature through
/// its optional features: `try_get_feature(TypeId::of::<dyn
/// IMetalExternalObjectsFeature>())` returns an `Rc<dyn
/// IMetalExternalObjectsFeature>`.
///
/// This API is private to the framework and its backends.
pub trait IMetalExternalObjectsFeature {
    /// The kinds of image handles the device can import.
    fn supported_image_handle_types(&self) -> Vec<String>;

    /// The kinds of semaphore handles the device can import.
    fn supported_semaphore_types(&self) -> Vec<String>;

    /// The locally unique identifier of the device, when it has one.
    fn device_luid(&self) -> Option<Vec<u8>>;

    /// How access to an imported image of the given handle type can be
    /// synchronized.
    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities;

    /// Imports an image as a texture of the device.
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IMetalExternalTexture>;

    /// Imports a shared event.
    fn import_shared_event(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IMetalSharedEvent>;

    /// Makes the commands submitted next wait until the event has reached
    /// `wait_for_value`.
    fn submit_wait(&self, event: &dyn IMetalSharedEvent, wait_for_value: u64);

    /// Makes the commands submitted so far set the event to `signal_value`
    /// when they have completed.
    fn submit_signal(&self, event: &dyn IMetalSharedEvent, signal_value: u64);
}

/// A texture of a Metal device that was imported from an external image.
///
/// This API is private to the framework and its backends.
pub trait IMetalExternalTexture {
    /// The width in pixels.
    fn width(&self) -> i32;

    /// The height in pixels.
    fn height(&self) -> i32;

    /// The number of samples per pixel.
    fn samples(&self) -> i32;

    /// The `id<MTLTexture>`.
    fn handle(&self) -> *mut c_void;

    /// Releases the texture.
    fn dispose(&self);
}

/// A shared event of a Metal device that was imported from a handle.
///
/// This API is private to the framework and its backends.
pub trait IMetalSharedEvent {
    /// The `id<MTLSharedEvent>`.
    fn handle(&self) -> *mut c_void;

    /// Releases the event.
    fn dispose(&self);
}
