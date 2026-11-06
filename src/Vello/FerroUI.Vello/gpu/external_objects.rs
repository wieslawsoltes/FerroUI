//! The import of GPU objects made outside of the backend into the device of the GPU modes on
//! macOS: images of other devices or processes (the frames of videos and cameras, as IOSurfaces
//! the platform imports as Metal textures) and shared events, through the external objects
//! feature of the platform's Metal device ([`IMetalExternalObjectsFeature`]).
//!
//! A frame shared with the backend ([`IPlatformRenderInterfaceImportedImage::acquire_shared_bitmap`])
//! becomes a [`VelloSharedFrameBitmap`]: the producer's storage, wrapped as a texture of the device
//! and converted on it, with the frame's lease kept until the device has read it. Nothing goes
//! through memory. The snapshots of an image (copies synchronized by keyed mutexes or semaphores)
//! are not supported by this backend.

use crate::gpu::{VelloDeviceTexture, VelloSharedFrameBitmap, VelloTextureAlpha, VelloWgpuDevice};
use ferroui_base::platform::{
    IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle, IPlatformRenderInterfaceImportedImage,
    IPlatformRenderInterfaceImportedObject, IPlatformRenderInterfaceImportedSemaphore,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::frames::{
    DeviceIdentity, FrameAlpha, FrameColor, FrameError, FrameFormat, FrameLease,
};
use ferroui_base::rendering::composition::{
    CompositionGpuImportedImageSynchronizationCapabilities, ICompositionImportableSharedGpuContextImage,
};
use ferroui_metal::{IMetalExternalObjectsFeature, IMetalExternalTexture, IMetalSharedEvent};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// The largest frame the backend imports, in pixels on a side.
const MAX_FRAME_SIDE: u32 = 8192;

/// The external objects feature of the device of the GPU modes on a Metal device of the platform.
pub struct VelloExternalObjectsFeature {
    device: Arc<VelloWgpuDevice>,
    /// The generation of the device's graphics resources: a frame made for another generation
    /// (before the device was recreated) is refused.
    generation: u64,
    inner: Rc<dyn IMetalExternalObjectsFeature>,
}

impl VelloExternalObjectsFeature {
    pub(crate) fn new(device: Arc<VelloWgpuDevice>, generation: u64, inner: Rc<dyn IMetalExternalObjectsFeature>) -> Self {
        Self { device, generation, inner }
    }
}

/// A shared event of another device, imported into the Metal device.
pub struct VelloImportedSemaphore {
    event: Rc<dyn IMetalSharedEvent>,
}

impl IPlatformRenderInterfaceImportedObject for VelloImportedSemaphore {
    fn dispose(&self) {
        self.event.dispose();
    }
}

impl IPlatformRenderInterfaceImportedSemaphore for VelloImportedSemaphore {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An image of another device or process, imported as a texture of the Metal device and wrapped
/// as a texture of the device of the GPU modes.
pub struct VelloImportedImage {
    device: Arc<VelloWgpuDevice>,
    generation: u64,
    device_luid: Option<Vec<u8>>,
    imported: Rc<dyn IMetalExternalTexture>,
    /// The wrapped texture, or why the imported texture cannot be wrapped.
    texture: Result<Arc<VelloDeviceTexture>, String>,
    format: PlatformGraphicsExternalImageFormat,
    top_left_origin: bool,
}

impl IPlatformRenderInterfaceImportedObject for VelloImportedImage {
    fn dispose(&self) {
        self.imported.dispose();
    }
}

impl IPlatformRenderInterfaceImportedImage for VelloImportedImage {
    fn acquire_shared_bitmap(&self, frame: FrameLease) -> Result<Arc<SharedBitmapImpl>, FrameError> {
        let m = frame.metadata();
        m.validate(MAX_FRAME_SIDE)?;
        if self.device.is_lost() || !frame.is_current() || m.device_generation != self.generation {
            return Err(FrameError::StaleGeneration);
        }
        let texture = self.texture.as_ref().map_err(|_| FrameError::MissingImporter)?;
        if m.top_left_origin != self.top_left_origin || m.width != texture.width() || m.height != texture.height() {
            return Err(FrameError::InvalidDimensions);
        }
        if m.device.is_none() || m.device != self.device_luid.clone().map(DeviceIdentity::Luid) {
            return Err(FrameError::DeviceMismatch);
        }
        match m.color {
            FrameColor::Srgb => {}
            // Video: the curve of the reference display, for opaque frames.
            FrameColor::Rec709Full if m.alpha == FrameAlpha::Opaque => {}
            _ => return Err(FrameError::UnsupportedColor),
        }
        let format_matches = matches!(
            (m.format, self.format),
            (FrameFormat::Bgra8, PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm)
                | (FrameFormat::Rgba8, PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm)
        );
        if !format_matches || m.alpha == FrameAlpha::Straight {
            return Err(FrameError::UnsupportedFormat);
        }
        frame.counters().gpu_import();
        Ok(Arc::new(VelloSharedFrameBitmap::new(texture.clone(), frame)))
    }

    /// # Panics
    /// Always: Metal has no keyed mutexes.
    fn snapshot_with_keyed_mutex(&self, _acquire_index: u32, _release_index: u32) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: a shared event is a timeline semaphore.
    fn snapshot_with_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: the Vello backend shares frames ([`acquire_shared_bitmap`](Self::acquire_shared_bitmap))
    /// and takes no snapshots of imported images.
    fn snapshot_with_timeline_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _wait_for_value: u64,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_value: u64,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: access to an image is synchronized with shared events.
    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }
}

impl IExternalObjectsRenderInterfaceContextFeature for VelloExternalObjectsFeature {
    fn shared_context_generation(&self) -> u64 {
        self.generation
    }

    fn supported_image_handle_types(&self) -> Vec<String> {
        self.inner.supported_image_handle_types()
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        self.inner.supported_semaphore_types()
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        let (format, top_left_origin) = (properties.format, properties.top_left_origin);
        let imported = self.inner.import_image(handle, properties);
        let alpha = VelloTextureAlpha::Premultiplied;
        let texture = crate::gpu::metal::wrap_external_texture(&self.device, imported.handle())
            .map(|texture| VelloDeviceTexture::from_external(&self.device, texture, alpha));
        Rc::new(VelloImportedImage {
            device: self.device.clone(),
            generation: self.generation,
            device_luid: self.inner.device_luid(),
            imported,
            texture,
            format,
            top_left_origin,
        })
    }

    /// # Panics
    /// Always: a Metal device has no share group.
    fn import_shared_image(
        &self,
        _image: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        panic!("Specified method is not supported.");
    }

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        Rc::new(VelloImportedSemaphore { event: self.inner.import_shared_event(handle) })
    }

    fn get_synchronization_capabilities(&self, image_handle_type: &str) -> CompositionGpuImportedImageSynchronizationCapabilities {
        self.inner.get_synchronization_capabilities(image_handle_type)
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.inner.device_luid()
    }
}
