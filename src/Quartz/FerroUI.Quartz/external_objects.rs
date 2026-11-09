//! GPU-only interop through the Apple platform Metal contracts. Producer leases live through actual GPU completion.
use crate::{bitmap::Bitmap, ffi};
use ferroui_base::platform::{
    IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle,
    IPlatformRenderInterfaceImportedImage, IPlatformRenderInterfaceImportedObject,
    IPlatformRenderInterfaceImportedSemaphore, PlatformGraphicsExternalImageFormat,
    PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::frames::{
    DeviceIdentity, FrameAlpha, FrameColor, FrameError, FrameFormat, FrameLease,
};
use ferroui_base::rendering::composition::{
    CompositionGpuImportedImageSynchronizationCapabilities,
    ICompositionImportableSharedGpuContextImage,
};
use ferroui_metal::{IMetalExternalObjectsFeature, IMetalExternalTexture, IMetalSharedEvent};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// The largest frame the backend imports, in pixels on a side.
const MAX_FRAME_SIDE: u32 = 8192;

/// The external objects feature of the device of the GPU modes on a Metal device of the platform.
pub struct ExternalObjects {
    device: Rc<dyn ferroui_metal::IMetalDevice>,
    /// The generation of the device's graphics resources: a frame made for another generation
    /// (before the device was recreated) is refused.
    generation: u64,
    backend_generation: u64,
    inner: Rc<dyn IMetalExternalObjectsFeature>,
}

impl ExternalObjects {
    pub(crate) fn new(
        device: Rc<dyn ferroui_metal::IMetalDevice>,
        generation: u64,
        backend_generation: u64,
        inner: Rc<dyn IMetalExternalObjectsFeature>,
    ) -> Self {
        Self {
            device,
            generation,
            backend_generation,
            inner,
        }
    }
}

/// An in-process Metal texture for composition import, retained independently
/// of the producer's wrapper. Synchronize mutable storage with shared events.
/// No pixel mapping, IOSurface or CPU copy is required.
pub struct SharedMetalImage {
    source: std::sync::Mutex<Option<ffi::Owned>>,
    properties: PlatformGraphicsExternalImageProperties,
    generation: u64,
}
impl SharedMetalImage {
    /// Retains an `id<MTLTexture>` and validates its sampling format and device.
    ///
    /// # Safety
    /// `texture` must be a live Objective-C Metal texture for this call. Its
    /// producer must not overwrite it until the snapshot's signal event fires,
    /// or until the acquired frame lease retires after actual GPU completion.
    pub unsafe fn from_raw_texture(
        texture: ffi::Ptr,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Result<Arc<Self>, FrameError> {
        if properties.width <= 0
            || properties.height <= 0
            || properties.width > MAX_FRAME_SIDE as i32
            || properties.height > MAX_FRAME_SIDE as i32
        {
            return Err(FrameError::InvalidDimensions);
        }
        let format = match properties.format {
            PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm => 1,
            PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm => 0,
        };
        match ffi::qm_validate_texture(texture, properties.width, properties.height, format) {
            1 => {}
            -1 => return Err(FrameError::DeviceMismatch),
            -2 => return Err(FrameError::InvalidDimensions),
            _ => return Err(FrameError::UnsupportedFormat),
        }
        if properties.memory_offset != 0 {
            return Err(FrameError::UnsupportedFormat);
        }
        let generation = ffi::qm_generation();
        if ffi::qm_is_lost(generation) != 0 {
            return Err(FrameError::StaleGeneration);
        }
        Ok(Arc::new(Self {
            source: std::sync::Mutex::new(Some(ffi::Owned::new(ffi::q_retain(texture)))),
            properties,
            generation,
        }))
    }
    fn texture(&self) -> Rc<dyn IMetalExternalTexture> {
        assert_eq!(
            unsafe { ffi::qm_is_lost(self.generation) },
            0,
            "Shared Metal texture belongs to a lost generation"
        );
        let source = self.source.lock().unwrap();
        let source = source.as_ref().expect("Shared Metal image was disposed");
        Rc::new(RetainedTexture {
            source: std::cell::RefCell::new(Some(unsafe {
                ffi::Owned::new(ffi::q_retain(source.0))
            })),
            width: self.properties.width,
            height: self.properties.height,
        })
    }
}
impl ICompositionImportableSharedGpuContextImage for SharedMetalImage {
    fn dispose(&self) {
        self.source.lock().unwrap().take();
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
struct RetainedTexture {
    source: std::cell::RefCell<Option<ffi::Owned>>,
    width: i32,
    height: i32,
}
impl IMetalExternalTexture for RetainedTexture {
    fn width(&self) -> i32 {
        self.width
    }
    fn height(&self) -> i32 {
        self.height
    }
    fn samples(&self) -> i32 {
        1
    }
    fn handle(&self) -> ffi::Ptr {
        self.source
            .borrow()
            .as_ref()
            .expect("Imported Metal texture was disposed")
            .0
    }
    fn dispose(&self) {
        self.source.borrow_mut().take();
    }
}

/// A shared event of another device, imported into the Metal device.
pub struct ImportedSemaphore {
    event: Rc<dyn IMetalSharedEvent>,
}

impl IPlatformRenderInterfaceImportedObject for ImportedSemaphore {
    fn dispose(&self) {
        self.event.dispose();
    }
}

impl IPlatformRenderInterfaceImportedSemaphore for ImportedSemaphore {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An image of another device or process, imported as a texture of the Metal device and wrapped
/// as a texture of the device of the GPU modes.
pub struct ImportedImage {
    device: Rc<dyn ferroui_metal::IMetalDevice>,
    generation: u64,
    backend_generation: u64,
    device_luid: Option<Vec<u8>>,
    imported: Rc<dyn IMetalExternalTexture>,
    /// The wrapped texture, or why the imported texture cannot be wrapped.
    properties: PlatformGraphicsExternalImageProperties,
    format: PlatformGraphicsExternalImageFormat,
    top_left_origin: bool,
}

impl IPlatformRenderInterfaceImportedObject for ImportedImage {
    fn dispose(&self) {
        self.imported.dispose();
    }
}

impl IPlatformRenderInterfaceImportedImage for ImportedImage {
    fn acquire_shared_bitmap(
        &self,
        frame: FrameLease,
    ) -> Result<Arc<SharedBitmapImpl>, FrameError> {
        let m = frame.metadata();
        m.validate(MAX_FRAME_SIDE)?;
        if self.device.is_lost()
            || unsafe { ffi::qm_is_lost(self.backend_generation) != 0 }
            || !frame.is_current()
            || m.device_generation != self.generation
        {
            return Err(FrameError::StaleGeneration);
        }

        if m.top_left_origin != self.top_left_origin
            || m.width != self.properties.width as u32
            || m.height != self.properties.height as u32
            || self.imported.width() != self.properties.width
            || self.imported.height() != self.properties.height
            || self.imported.samples() != 1
        {
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
            (
                FrameFormat::Bgra8,
                PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm
            ) | (
                FrameFormat::Rgba8,
                PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm
            )
        );
        if !format_matches || m.alpha == FrameAlpha::Straight {
            return Err(FrameError::UnsupportedFormat);
        }
        frame.counters().gpu_import();
        Ok(Arc::new(SharedFrameBitmap::new(
            self.imported.handle(),
            frame,
            self.backend_generation,
        )))
    }

    /// # Panics
    /// Always: Metal has no keyed mutexes.
    fn snapshot_with_keyed_mutex(
        &self,
        _acquire_index: u32,
        _release_index: u32,
    ) -> Arc<SharedBitmapImpl> {
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

    /// Takes an independent GPU copy, ordered between the producer's shared events.
    fn snapshot_with_timeline_semaphores(
        &self,
        wait: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        wait_value: u64,
        signal: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        signal_value: u64,
    ) -> Arc<SharedBitmapImpl> {
        assert!(
            !self.device.is_lost() && unsafe { ffi::qm_is_lost(self.backend_generation) == 0 },
            "The Metal device has been lost"
        );
        let wait = wait
            .as_any()
            .downcast_ref::<ImportedSemaphore>()
            .expect("Timeline event must be imported by Quartz Metal");
        let signal = signal
            .as_any()
            .downcast_ref::<ImportedSemaphore>()
            .expect("Timeline event must be imported by Quartz Metal");
        let bitmap = Bitmap::new(
            ferroui_base::PixelSize::new(self.properties.width, self.properties.height),
            ferroui_base::Vector::new(96., 96.),
        );
        let copied = bitmap.context(|p| unsafe {
            ffi::qm_snapshot_external(
                p,
                self.imported.handle(),
                (!self.top_left_origin) as i32,
                wait.event.handle(),
                wait_value,
                signal.event.handle(),
                signal_value,
            )
        });
        assert_ne!(copied, 0, "Invalid Metal texture or shared event");
        Arc::new(bitmap)
    }

    /// # Panics
    /// Always: access to an image is synchronized with shared events.
    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }
}

impl IExternalObjectsRenderInterfaceContextFeature for ExternalObjects {
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
        let imported = self.inner.import_image(handle, properties.clone());
        Rc::new(ImportedImage {
            device: self.device.clone(),
            generation: self.generation,
            backend_generation: self.backend_generation,
            device_luid: self.inner.device_luid(),
            imported,
            properties,
            format,
            top_left_origin,
        })
    }

    fn import_shared_image(
        &self,
        image: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        let image = image
            .as_any()
            .downcast_ref::<SharedMetalImage>()
            .expect("Expected a Quartz SharedMetalImage");
        let properties = image.properties.clone();
        Rc::new(ImportedImage {
            device: self.device.clone(),
            generation: self.generation,
            backend_generation: self.backend_generation,
            device_luid: self.inner.device_luid(),
            imported: image.texture(),
            format: properties.format,
            top_left_origin: properties.top_left_origin,
            properties,
        })
    }

    fn import_semaphore(
        &self,
        handle: Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        Rc::new(ImportedSemaphore {
            event: self.inner.import_shared_event(handle),
        })
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        self.inner
            .get_synchronization_capabilities(image_handle_type)
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.inner.device_luid()
    }
}

/// Wraps producer storage until first draw, then caches a GPU-only color/format conversion.
/// The producer allocation retires after Metal's completion callback, never on submission.
pub(crate) struct SharedFrameBitmap {
    size: ferroui_base::PixelSize,
    state: std::sync::Mutex<FrameState>,
    generation: u64,
}
struct FrameState {
    source: Option<ffi::Owned>,
    frame: Option<FrameLease>,
    converted: Option<Bitmap>,
}
impl SharedFrameBitmap {
    fn new(texture: ffi::Ptr, frame: FrameLease, generation: u64) -> Self {
        let m = frame.metadata();
        Self {
            size: ferroui_base::PixelSize::new(m.width as i32, m.height as i32),
            generation,
            state: std::sync::Mutex::new(FrameState {
                // The Metal texture's immutable sampling view can be retained on the render thread
                // and later released on any thread. Access is serialized by this mutex.
                source: Some(unsafe { ffi::Owned::new(ffi::q_retain(texture)) }),
                frame: Some(frame),
                converted: None,
            }),
        }
    }
    pub(crate) fn bitmap(&self) -> Option<Bitmap> {
        if unsafe { ffi::qm_is_lost(self.generation) != 0 } {
            return None;
        }
        let mut state = self.state.lock().unwrap();
        if let Some(b) = &state.converted {
            return Some(b.clone());
        }
        let frame = state.frame.as_ref()?.clone();
        if !frame.is_current() {
            return None;
        }
        let m = frame.metadata();
        let rec709 = m.color == FrameColor::Rec709Full;
        let flipped = !m.top_left_origin;
        let opaque = m.alpha == FrameAlpha::Opaque;
        let reason = if rec709 {
            ferroui_base::rendering::composition::frames::GpuTransferReason::ColorSpaceConversion
        } else {
            ferroui_base::rendering::composition::frames::GpuTransferReason::PixelFormatConversion
        };
        let completion = ferroui_base::rendering::composition::frames::GpuCompletion::new();
        completion.retain(frame.clone()).ok()?;
        let output = Bitmap::new(self.size, ferroui_base::Vector::new(96., 96.));
        let callback = Box::into_raw(Box::new(completion.clone())).cast();
        let submitted = output.context(|p| unsafe {
            ffi::qm_convert_frame(
                p,
                state.source.as_ref().unwrap().0,
                rec709 as i32,
                flipped as i32,
                opaque as i32,
                callback,
                frame_completed,
            )
        });
        if submitted == 0 {
            unsafe {
                drop(Box::from_raw(
                    callback.cast::<ferroui_base::rendering::composition::frames::GpuCompletion>(),
                ));
            }
            completion.complete(); // No GPU read was submitted.
            return None;
        }
        frame.counters().gpu_conversion(reason);
        state.frame = None;
        state.source = None;
        state.converted = Some(output.clone());
        Some(output)
    }
}
extern "C" fn frame_completed(context: ffi::Ptr, success: i32) {
    // Metal invokes exactly once, on its completion thread. Only Send/Sync frame ownership
    // crosses this boundary; no Rc, UI objects, native session or render context is accessed.
    let completion = unsafe {
        Box::from_raw(context.cast::<ferroui_base::rendering::composition::frames::GpuCompletion>())
    };
    if success != 0 {
        completion.complete();
    } else {
        completion.mark_indeterminate();
    }
}
impl ferroui_base::platform::IBitmapImpl for SharedFrameBitmap {
    fn dpi(&self) -> ferroui_base::Vector {
        ferroui_base::Vector::new(96., 96.)
    }
    fn pixel_size(&self) -> ferroui_base::PixelSize {
        self.size
    }
    fn version(&self) -> i32 {
        1
    }
    fn save(
        &self,
        _: &mut dyn std::io::Write,
        _: &ferroui_base::media::imaging::BitmapEncoderOptions,
    ) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "GPU frame export requires explicit counted capture",
        ))
    }
    fn dispose(&self) {
        let mut state = self.state.lock().unwrap();
        state.source = None;
        state.frame = None;
        state.converted = None;
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
#[path = "external_objects_tests.rs"]
mod tests;
