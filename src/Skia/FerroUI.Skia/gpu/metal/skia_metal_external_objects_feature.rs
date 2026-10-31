use super::skia_metal_gpu::{MetalGrContext, SharedContext};
use crate::gpu::ISkiaGrContext;
use crate::immutable_bitmap::ImmutableBitmap;
use crate::metal::{IMetalExternalObjectsFeature, IMetalExternalTexture, IMetalSharedEvent};
use ferroui_base::platform::{
    IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle, IPlatformRenderInterfaceImportedImage,
    IPlatformRenderInterfaceImportedObject, IPlatformRenderInterfaceImportedSemaphore,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::{
    CompositionGpuImportedImageSynchronizationCapabilities, ICompositionImportableSharedGpuContextImage,
};
#[cfg(ferro_skia_apple_ganesh)]
use skia_safe::gpu::{backend_render_targets, mtl, surfaces, SurfaceOrigin};
#[cfg(not(ferro_skia_apple_ganesh))]
use skia_safe::gpu::graphite::mtl::backend_textures;
#[cfg(not(ferro_skia_apple_ganesh))]
use skia_safe::gpu::graphite::{surfaces, Recorder};
#[cfg(not(ferro_skia_apple_ganesh))]
use skia_safe::gpu::Mipmapped;
use skia_safe::ColorType;
#[cfg(not(ferro_skia_apple_ganesh))]
use skia_safe::{BlendMode, Image, Paint, SamplingOptions};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// Imports images and shared events that were created outside of the
/// framework into the Graphite context (the Ganesh context in the
/// configuration `ferro_skia_apple_ganesh`) of a Metal Skia GPU, through the
/// external objects feature of the Metal device.
///
/// The feature and everything it imports are objects of the render
/// interface: they hold the Graphite context of the GPU and objects of the
/// device, and are used under the lock of the compositor they belong to.
// The original holds its `SkiaMetalGpu`; the GPU owns the feature here, so
// the feature holds what it reads of the GPU instead: the Skia context the
// GPU shares with its render targets.
pub struct SkiaMetalExternalObjectsFeature {
    context: SharedContext,
    inner: Rc<dyn IMetalExternalObjectsFeature>,
}

impl SkiaMetalExternalObjectsFeature {
    pub(super) fn new(context: SharedContext, inner: Rc<dyn IMetalExternalObjectsFeature>) -> Self {
        Self { context, inner }
    }
}

/// A shared event of another device, imported into the Metal device.
pub struct ImportedSemaphore {
    ev: Rc<dyn IMetalSharedEvent>,
}

impl ImportedSemaphore {
    pub fn event(&self) -> &Rc<dyn IMetalSharedEvent> {
        &self.ev
    }
}

impl IPlatformRenderInterfaceImportedObject for ImportedSemaphore {
    fn dispose(&self) {
        self.ev.dispose();
    }
}

impl IPlatformRenderInterfaceImportedSemaphore for ImportedSemaphore {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The imported semaphore behind the contract (the cast of the original).
fn imported_semaphore(semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>) -> &ImportedSemaphore {
    semaphore
        .as_any()
        .downcast_ref::<ImportedSemaphore>()
        .unwrap_or_else(|| panic!("The semaphore was not imported by the Metal Skia GPU."))
}

/// An image of another device or process, imported as a texture of the
/// Metal device. A snapshot copies the texture into an image of the
/// Skia context.
pub struct ImportedImage {
    context: SharedContext,
    feature: Rc<dyn IMetalExternalObjectsFeature>,
    texture: Rc<dyn IMetalExternalTexture>,
    color_type: ColorType,
    top_left_origin: bool,
}

impl ImportedImage {
    /// The Skia context of the GPU.
    ///
    /// # Panics
    /// Panics when the GPU has been disposed.
    fn gr_context(&self) -> Rc<MetalGrContext> {
        self.context.borrow().clone().expect("SkiaMetalGpu has been disposed")
    }

    /// A copy of `image` with its rows in the opposite order.
    #[cfg(not(ferro_skia_apple_ganesh))]
    fn flip_vertically(recorder: &mut Recorder, image: &Image) -> Option<Image> {
        let mut surface = surfaces::render_target(recorder, image.image_info(), Mipmapped::No, None, None)?;
        let mut paint = Paint::default();
        paint.set_blend_mode(BlendMode::Src);
        let canvas = surface.canvas();
        canvas.translate((0.0, image.height() as f32));
        canvas.scale((1.0, -1.0));
        canvas.draw_image_with_sampling_options(image, (0.0, 0.0), SamplingOptions::default(), Some(&paint));
        surfaces::as_image_copy(&surface, None, Mipmapped::No)
    }
}

impl IPlatformRenderInterfaceImportedObject for ImportedImage {
    fn dispose(&self) {
        self.texture.dispose();
    }
}

impl IPlatformRenderInterfaceImportedImage for ImportedImage {
    #[cfg(not(ferro_skia_apple_ganesh))]
    fn acquire_shared_bitmap(
        &self,
        frame: ferroui_base::rendering::composition::frames::FrameLease,
    ) -> Result<Arc<SharedBitmapImpl>, ferroui_base::rendering::composition::frames::FrameError> {
        use ferroui_base::rendering::composition::frames::{DeviceIdentity, FrameAlpha, FrameColor, FrameError, FrameFormat};
        let m = frame.metadata();
        m.validate(8192)?;
        if !m.top_left_origin || m.width != self.texture.width() as u32 || m.height != self.texture.height() as u32 {
            return Err(FrameError::InvalidDimensions);
        }
        if m.device != self.feature.device_luid().map(DeviceIdentity::Luid) || m.device.is_none() {
            return Err(FrameError::DeviceMismatch);
        }
        crate::leased_gpu_bitmap::frame_color_filter(m.color)?;
        if m.color == FrameColor::Rec709Full && m.alpha != FrameAlpha::Opaque {
            return Err(FrameError::UnsupportedColor);
        }
        if !matches!((m.format, self.color_type), (FrameFormat::Bgra8, ColorType::BGRA8888)
            | (FrameFormat::Rgba8, ColorType::RGBA8888)) { return Err(FrameError::UnsupportedFormat); }
        let alpha = match m.alpha {
            FrameAlpha::Opaque => skia_safe::AlphaType::Opaque,
            FrameAlpha::Premultiplied => skia_safe::AlphaType::Premul,
            FrameAlpha::Straight => return Err(FrameError::UnsupportedFormat),
        };
        let gr = self.gr_context();
        let backend = unsafe { backend_textures::make_metal((self.texture.width(), self.texture.height()), self.texture.handle()) };
        // Wrap, never snapshot: the frame owns producer storage and each draw
        // registers it with the context's native GPU completion handler.
        let image = gr.with_recorder(|recorder| skia_safe::gpu::graphite::images::wrap_texture(
            recorder, &backend, self.color_type, alpha, None)).ok_or(FrameError::MissingImporter)?;
        let id = gr.frame_context_id().ok_or(FrameError::MissingSynchronization)?;
        frame.counters().gpu_import();
        Ok(Arc::new(crate::leased_gpu_bitmap::LeasedGpuBitmap::new(image, frame, id)))
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
    /// Panics when the GPU has been disposed, when a semaphore was not
    /// imported by this backend and when the texture cannot be wrapped or
    /// copied.
    #[cfg(not(ferro_skia_apple_ganesh))]
    fn snapshot_with_timeline_semaphores(
        &self,
        wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        wait_for_value: u64,
        signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        signal_value: u64,
    ) -> Arc<SharedBitmapImpl> {
        let gr_context = self.gr_context();

        gr_context.flush();
        self.feature.submit_wait(&**imported_semaphore(wait_for_semaphore).event(), wait_for_value);

        // SAFETY: the imported texture keeps the `id<MTLTexture>` it hands
        // out alive until it is disposed; the surface that wraps the texture
        // is released before this function returns, and the recording that
        // copies from it holds the texture by itself.
        let backend_texture =
            unsafe { backend_textures::make_metal((self.texture.width(), self.texture.height()), self.texture.handle()) };

        // Deviation (DEVIATIONS.md, Skia backend): upstream wraps the
        // texture as a Ganesh render target with the origin of the image
        // and takes its snapshot, which is a copy that carries the origin.
        // Graphite has no surface origin: the copy of the surface is taken
        // as it is for an image whose first row is the top one, and with its
        // rows reversed otherwise.
        let image = gr_context.with_recorder(|recorder| {
            let surface = surfaces::wrap_backend_texture(recorder, &backend_texture, self.color_type, None, None)?;
            let image = surfaces::as_image_copy(&surface, None, Mipmapped::No)?;
            if self.top_left_origin {
                Some(image)
            } else {
                Self::flip_vertically(recorder, &image)
            }
        });
        let Some(image) = image else {
            panic!("Unable to create a Skia surface for the Metal texture.");
        };

        let rv = ImmutableBitmap::from_image(image, None);
        gr_context.flush();
        self.feature.submit_signal(&**imported_semaphore(signal_semaphore).event(), signal_value);
        Arc::new(rv)
    }

    /// # Panics
    /// Panics when the GPU has been disposed, when a semaphore was not
    /// imported by this backend and when the texture cannot be wrapped.
    #[cfg(ferro_skia_apple_ganesh)]
    fn snapshot_with_timeline_semaphores(
        &self,
        wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        wait_for_value: u64,
        signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        signal_value: u64,
    ) -> Arc<SharedBitmapImpl> {
        let gr_context = self.gr_context();

        // `Flush(true, false)` of the original: submitted, not waited for.
        gr_context.flush();
        self.feature.submit_wait(&**imported_semaphore(wait_for_semaphore).event(), wait_for_value);

        // SAFETY: the imported texture keeps the `id<MTLTexture>` it hands
        // out alive until it is disposed; the render target and the surface
        // that wrap the texture are released before this function returns,
        // and the snapshot is a copy of its own (the surface wraps a render
        // target, which an image cannot share).
        let texture_info = unsafe { mtl::TextureInfo::new(self.texture.handle() as _) };
        let backend_target =
            backend_render_targets::make_mtl((self.texture.width(), self.texture.height()), &texture_info);
        let origin = if self.top_left_origin { SurfaceOrigin::TopLeft } else { SurfaceOrigin::BottomLeft };
        let surface = gr_context.with_context(|context| {
            surfaces::wrap_backend_render_target(context, &backend_target, origin, self.color_type, None, None)
        });
        let Some(mut surface) = surface else {
            panic!("Unable to create a Skia surface for the Metal texture.");
        };
        let rv = ImmutableBitmap::from_image(surface.image_snapshot(), None);
        drop(surface);
        drop(backend_target);
        gr_context.flush();
        self.feature.submit_signal(&**imported_semaphore(signal_semaphore).event(), signal_value);
        Arc::new(rv)
    }

    /// # Panics
    /// Always: access to an image is synchronized with shared events.
    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }
}

impl IExternalObjectsRenderInterfaceContextFeature for SkiaMetalExternalObjectsFeature {
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
        // Every format of the enumeration is supported: the original
        // refuses the values it does not name.
        let format = match properties.format {
            PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm => ColorType::RGBA8888,
            PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm => ColorType::BGRA8888,
        };
        let top_left_origin = properties.top_left_origin;

        Rc::new(ImportedImage {
            context: self.context.clone(),
            feature: self.inner.clone(),
            texture: self.inner.import_image(handle, properties),
            color_type: format,
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
        Rc::new(ImportedSemaphore { ev: self.inner.import_shared_event(handle) })
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        self.inner.get_synchronization_capabilities(image_handle_type)
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.inner.device_luid()
    }
}

#[cfg(all(test, feature="native-frame-tests", not(ferro_skia_apple_ganesh)))]
#[path="shared_frame_tests.rs"]
mod shared_frame_tests;
