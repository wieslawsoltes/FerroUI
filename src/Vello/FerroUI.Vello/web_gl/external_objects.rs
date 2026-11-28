//! The shared frames of videos and cameras on the WebGL2 context of the canvas (#575, the browser
//! step).
//!
//! A producer of the page hands each frame out as a texture of the context (`SHARED_GL_TEXTURE`,
//! its owner a [`SharedFrameTexture`] the lease carries), and a render worker receives a media
//! frame it uploads into a texture of the context (`BROWSER_TRANSFERRED_FRAME`, #601). Those are
//! names of the OpenGL contracts, and the renderer of `vello_gpu` paints with objects of `web-sys`,
//! so the frame is copied on the GPU (`blitFramebuffer`) into a texture of the pool, its first row
//! the top one, and becomes a [`VelloWebGlFrameBitmap`]. Nothing goes through memory.
//!
//! **Lifetimes.** The lease of a frame of the context is retained by a fence of the context
//! ([`IGlContext::schedule_frame_completion`], a WebGL2 `fenceSync`) until the copy is done, so the
//! producer can reuse its storage from then on. The objects of the context belong to the render
//! thread and a bitmap is `Send + Sync`, so the bitmap holds a number and the texture stays in a
//! table of that thread; a bitmap dropped on any thread queues its number, and the render thread
//! lets the texture go back to the pool at its next frame.

use super::{VelloWebGlGpu, VelloWebGlTexture};
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::platform::{
    IBitmapImpl, IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle, IPlatformRenderInterfaceImportedImage,
    IPlatformRenderInterfaceImportedObject, IPlatformRenderInterfaceImportedSemaphore,
    KnownPlatformGraphicsExternalImageHandleTypes as HandleTypes, PlatformGraphicsExternalImageFormat,
    PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::frames::{
    DeviceIdentity, FrameAlpha, FrameColor, FrameError, FrameFormat, FrameLease, GpuCompletion, GpuTransferReason,
};
use ferroui_base::rendering::composition::{
    CompositionGpuImportedImageSynchronizationCapabilities, ICompositionImportableSharedGpuContextImage,
};
use ferroui_base::{PixelSize, Vector};
use ferroui_opengl::{IGlContext, SharedFrameTexture};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// The largest frame the backend imports, in pixels on a side (as on the other routes).
const MAX_FRAME_SIDE: u32 = 8192;

static NEXT_BITMAP: AtomicU64 = AtomicU64::new(1);
/// The bitmaps that were dropped, whose textures the render thread has not let go of yet.
static RETIRED: Mutex<Vec<u64>> = Mutex::new(Vec::new());

ferroui_base::ferro_thread_local! {
    /// The textures of the frame bitmaps of this thread, by the number of the bitmap.
    static TEXTURES: RefCell<HashMap<u64, Rc<VelloWebGlTexture>>> = RefCell::new(HashMap::new());
}

/// Gives the textures of the bitmaps that were dropped back to the pool. On the render thread.
pub(super) fn release_retired() {
    let retired = std::mem::take(&mut *RETIRED.lock().unwrap_or_else(|error| error.into_inner()));
    if retired.is_empty() {
        return;
    }
    TEXTURES.with(|textures| {
        let mut textures = textures.borrow_mut();
        for id in retired {
            textures.remove(&id);
        }
    });
}

/// Whether the bitmap numbered `id` still holds its texture, on the render thread.
#[cfg(feature = "media3d")]
pub(super) fn is_held(id: u64) -> bool {
    TEXTURES.try_with(|textures| textures.borrow().contains_key(&id)).unwrap_or(false)
}

/// A frame of a producer as a bitmap of the WebGL2 route: a texture of the pool of the context,
/// which only the render thread reads. Drawn elsewhere it draws nothing: there is no path through
/// memory.
pub struct VelloWebGlFrameBitmap {
    id: u64,
    /// The GPU whose pool holds the texture ([`VelloWebGlGpu::identity`]).
    gpu: u64,
    pixel_size: PixelSize,
}

impl VelloWebGlFrameBitmap {
    fn new(gpu: u64, texture: Rc<VelloWebGlTexture>, pixel_size: PixelSize) -> Self {
        let id = NEXT_BITMAP.fetch_add(1, Ordering::Relaxed);
        TEXTURES.with(|textures| textures.borrow_mut().insert(id, texture));
        Self { id, gpu, pixel_size }
    }

    /// The texture of the frame and the size of the frame in it, on the render thread of `gpu`.
    pub(crate) fn texture(&self, gpu: &VelloWebGlGpu) -> Option<(Rc<VelloWebGlTexture>, (u32, u32))> {
        if self.gpu != gpu.identity() {
            return None;
        }
        let texture = TEXTURES.try_with(|textures| textures.borrow().get(&self.id).cloned()).ok()??;
        Some((texture, (self.pixel_size.width as u32, self.pixel_size.height as u32)))
    }

    /// What names the bitmap among those of the process.
    #[cfg(feature = "media3d")]
    pub(super) fn number(&self) -> u64 {
        self.id
    }

    /// The GPU whose pool holds its texture ([`VelloWebGlGpu::identity`]).
    #[cfg(feature = "media3d")]
    pub(super) fn gpu_identity(&self) -> u64 {
        self.gpu
    }

    /// Its texture of the pool and the size of the frame in it, on the render thread.
    #[cfg(feature = "media3d")]
    pub(super) fn pool_texture(&self) -> Option<(Rc<VelloWebGlTexture>, (u32, u32))> {
        let texture = TEXTURES.try_with(|textures| textures.borrow().get(&self.id).cloned()).ok()??;
        Some((texture, (self.pixel_size.width as u32, self.pixel_size.height as u32)))
    }

    fn retire(&self) {
        RETIRED.lock().unwrap_or_else(|error| error.into_inner()).push(self.id);
    }
}

impl Drop for VelloWebGlFrameBitmap {
    fn drop(&mut self) {
        self.retire();
    }
}

impl IBitmapImpl for VelloWebGlFrameBitmap {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn pixel_size(&self) -> PixelSize {
        self.pixel_size
    }

    fn version(&self) -> i32 {
        1
    }

    fn save(&self, _: &mut dyn Write, _: &BitmapEncoderOptions) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "GPU frame export requires an explicit counted capture"))
    }

    fn dispose(&self) {
        self.retire();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl crate::i_drawable_bitmap_impl::IDrawableBitmapImpl for VelloWebGlFrameBitmap {
    /// None: a frame of a producer is drawn on its context only.
    fn image(&self) -> Option<peniko::ImageData> {
        None
    }
}

/// The external objects feature of the WebGL2 context of the canvas.
pub struct VelloWebGlExternalObjectsFeature {
    gpu: Weak<VelloWebGlGpu>,
    context: Rc<dyn IGlContext>,
    /// The context whose textures are frames, when it shares them.
    context_id: Option<u64>,
    generation: u64,
    /// Whether the context takes frames transferred from the page's thread.
    transfers: bool,
}

impl VelloWebGlExternalObjectsFeature {
    /// The feature over `context`; `None` for a context that takes no frames.
    pub(super) fn new(gpu: Weak<VelloWebGlGpu>, context: Rc<dyn IGlContext>) -> Option<Self> {
        let context_id = context.shared_frame_context_id();
        let transfers = context.transferred_frame_context_id().is_some();
        if context_id.is_none() && !transfers {
            return None;
        }
        let generation = context.shared_frame_context_generation();
        Some(Self { gpu, context, context_id, generation, transfers })
    }
}

/// Where the frames of an imported image are.
enum Source {
    /// A texture of the context, the producer's.
    Texture { name: u32, context_id: u64, generation: u64 },
    /// A frame transferred to this thread, uploaded once when it is first acquired.
    Transferred { frame: u32, bitmap: RefCell<Option<Arc<SharedBitmapImpl>>> },
}

/// A frame of the context as an image of the backend.
pub struct VelloWebGlImportedImage {
    gpu: Weak<VelloWebGlGpu>,
    context: Rc<dyn IGlContext>,
    properties: PlatformGraphicsExternalImageProperties,
    source: Source,
}

impl IPlatformRenderInterfaceImportedObject for VelloWebGlImportedImage {
    /// The storage is the producer's, and what a bitmap holds goes with the bitmap.
    fn dispose(&self) {}
}

impl VelloWebGlImportedImage {
    /// What every frame must be to be copied as it is: RGBA8 of the size of the image, in sRGB,
    /// opaque or premultiplied.
    fn check(&self, frame: &FrameLease) -> Result<(), FrameError> {
        let m = frame.metadata();
        m.validate(MAX_FRAME_SIDE)?;
        if !frame.is_current() {
            return Err(FrameError::StaleGeneration);
        }
        if m.width as i32 != self.properties.width
            || m.height as i32 != self.properties.height
            || m.top_left_origin != self.properties.top_left_origin
        {
            return Err(FrameError::InvalidDimensions);
        }
        let format = self.properties.format;
        if m.format != FrameFormat::Rgba8 || format != PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm {
            return Err(FrameError::UnsupportedFormat);
        }
        if m.color != FrameColor::Srgb {
            return Err(FrameError::UnsupportedColor);
        }
        if m.alpha == FrameAlpha::Straight {
            return Err(FrameError::UnsupportedFormat);
        }
        Ok(())
    }

    /// The bitmap of the texture `name` of the context, copied into a texture of the pool.
    fn copied(&self, gpu: &VelloWebGlGpu, name: u32, frame: &FrameLease) -> Result<Arc<SharedBitmapImpl>, FrameError> {
        let m = frame.metadata();
        let copied = gpu.copy_frame_texture(name, (m.width, m.height), !m.top_left_origin).map_err(|error| {
            super::log_render_failure(&error);
            FrameError::MissingImporter
        })?;
        frame.counters().gpu_import();
        frame.counters().gpu_copy(GpuTransferReason::DeviceCompatibility);
        let size = PixelSize::new(m.width as i32, m.height as i32);
        Ok(Arc::new(VelloWebGlFrameBitmap::new(gpu.identity(), copied, size)))
    }
}

impl IPlatformRenderInterfaceImportedImage for VelloWebGlImportedImage {
    fn acquire_shared_bitmap(&self, frame: FrameLease) -> Result<Arc<SharedBitmapImpl>, FrameError> {
        self.check(&frame)?;
        let gpu = self.gpu.upgrade().ok_or(FrameError::StaleGeneration)?;
        if gpu.is_context_lost() {
            return Err(FrameError::StaleGeneration);
        }
        match &self.source {
            Source::Texture { name, context_id, generation } => {
                let owner = frame.resource::<SharedFrameTexture>().ok_or(FrameError::MissingImporter)?;
                let m = frame.metadata();
                if m.device_generation != *generation
                    || owner.generation() != *generation
                    || self.context.shared_frame_context_generation() != *generation
                {
                    return Err(FrameError::StaleGeneration);
                }
                if m.device != Some(DeviceIdentity::Context(*context_id))
                    || owner.context_id() != *context_id
                    || owner.texture() != *name
                {
                    return Err(FrameError::DeviceMismatch);
                }
                // Retained before the first command that reads it, until the fence after the copy.
                let completion = GpuCompletion::new();
                if let Err(error) = completion.retain(frame.clone()) {
                    completion.complete();
                    return Err(error);
                }
                let bitmap = self.copied(&gpu, *name, &frame);
                self.context.schedule_frame_completion(completion);
                bitmap
            }
            Source::Transferred { frame: number, bitmap } => {
                if let Some(bitmap) = bitmap.borrow().as_ref() {
                    return Ok(bitmap.clone());
                }
                let (width, height) = (self.properties.width as u32, self.properties.height as u32);
                let current = self.context.ensure_current();
                let uploaded = self.context.import_transferred_frame(*number, width, height);
                let copied = uploaded.map_err(|error| {
                    super::log_render_failure(&error);
                    FrameError::MissingImporter
                });
                let result = copied.and_then(|name| {
                    let result = self.copied(&gpu, name, &frame);
                    self.context.release_transferred_frame_texture(name);
                    result
                });
                current.dispose();
                let made = result?;
                *bitmap.borrow_mut() = Some(made.clone());
                Ok(made)
            }
        }
    }

    /// # Panics
    /// Always: the frames of the context are shared, not copied by a keyed mutex.
    fn snapshot_with_keyed_mutex(&self, _acquire_index: u32, _release_index: u32) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: the order of the commands of the context needs no semaphore.
    fn snapshot_with_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: the order of the commands of the context needs no semaphore.
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
    /// Always: the frames are shared ([`acquire_shared_bitmap`](Self::acquire_shared_bitmap)).
    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }
}

impl IExternalObjectsRenderInterfaceContextFeature for VelloWebGlExternalObjectsFeature {
    fn shared_context_id(&self) -> Option<u64> {
        self.context_id
    }

    fn shared_context_generation(&self) -> u64 {
        self.generation
    }

    fn supported_image_handle_types(&self) -> Vec<String> {
        let mut types = Vec::new();
        if self.context_id.is_some() {
            types.push(HandleTypes::SHARED_GL_TEXTURE.into());
        }
        if self.transfers {
            types.push(HandleTypes::BROWSER_TRANSFERRED_FRAME.into());
        }
        types
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    /// # Panics
    /// Panics for a handle the context does not take, as the other backends do.
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        let descriptor = handle.handle_descriptor();
        let source = if descriptor == Some(HandleTypes::SHARED_GL_TEXTURE) {
            let context_id = self.context_id.expect("Direct frame sampling is unavailable");
            let name = u32::try_from(handle.handle()).ok().filter(|id| *id != 0).expect("Invalid frame texture name");
            Source::Texture { name, context_id, generation: self.generation }
        } else if descriptor == Some(HandleTypes::BROWSER_TRANSFERRED_FRAME) && self.transfers {
            let frame =
                u32::try_from(handle.handle()).ok().filter(|id| *id != 0).expect("Invalid transferred frame number");
            Source::Transferred { frame, bitmap: RefCell::new(None) }
        } else {
            panic!("Importing this platform handle is not supported");
        };
        Rc::new(VelloWebGlImportedImage { gpu: self.gpu.clone(), context: self.context.clone(), properties, source })
    }

    /// # Panics
    /// Always: the frames are imported by their texture.
    fn import_shared_image(
        &self,
        _image: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: the order of the commands of the context needs no semaphore.
    fn import_semaphore(&self, _handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        panic!("Specified method is not supported.");
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        let shared = image_handle_type == HandleTypes::SHARED_GL_TEXTURE && self.context_id.is_some();
        let transferred = image_handle_type == HandleTypes::BROWSER_TRANSFERRED_FRAME && self.transfers;
        if shared || transferred {
            return CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC;
        }
        CompositionGpuImportedImageSynchronizationCapabilities::default()
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        None
    }
}
