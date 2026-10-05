use crate::gpu::{ISkiaGrContext, SkiaGpuBackend};
use skia_safe::gpu::graphite::{self, InsertRecordingInfo};
use skia_safe::gpu::Mipmapped;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use skia_safe::{
    images, AlphaType, BlendMode, ConditionallySend, Data, Image, ImageInfo, Paint, SamplingOptions, Surface,
    SurfaceProps,
};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;

/// An image that was uploaded for drawing: the image it was made from and
/// its textures on the recorder.
struct UploadedImage {
    /// Keeps the unique id of the image from being reused and tells, by its
    /// reference count, whether anything else still holds the image.
    source: Image,
    /// The texture of the image; `None` when the upload failed. The failure
    /// is kept, so that it is reported once and not tried again with every
    /// draw.
    texture: Option<Image>,
    /// The texture with mipmaps, made when a draw first samples the image
    /// with mipmaps; the inner `None` when it could not be made.
    mipmapped_texture: Option<Option<Image>>,
}

impl UploadedImage {
    /// Whether the image is still held by something other than this entry.
    fn is_source_in_use(&self) -> bool {
        // A handle may move to another thread exactly when it is the only
        // reference to its object: that is the reference count test.
        !self.source.can_send()
    }
}

/// The mutable halves of a Graphite GPU context. The uploaded images are
/// declared first so that they are released before the recorder they live
/// on, and the recorder before the context that created it.
struct GraphiteState {
    /// The textures of the raster images that were drawn, by the unique id
    /// of the image.
    uploaded_images: HashMap<u32, UploadedImage>,
    recorder: graphite::Recorder,
    context: graphite::Context,
}

/// A Graphite context and the recorder all drawing of the backend goes
/// through.
///
/// Graphite splits what Ganesh calls a context in two: a recorder that
/// surfaces are created from and that accumulates draw commands, and a
/// context that executes snapped recordings. [`flush`](ISkiaGrContext::flush)
/// snaps the recorder, inserts the recording and submits it.
///
/// A Graphite recorder draws only images that live on it: a raster image is
/// dropped from the drawing unless the client converts it, either with an
/// image provider in the recorder options or by making a texture image from
/// it. The recorder options of the Skia binding in use carry no image
/// provider, so the context does what such a provider does:
/// [`drawable_image`](ISkiaGrContext::drawable_image) uploads a raster image
/// once and hands out the texture for as long as the image is alive.
pub struct GraphiteGrContext {
    state: RefCell<Option<GraphiteState>>,
}

impl GraphiteGrContext {
    /// Wraps a Graphite context, creating its recorder. Returns `None` when
    /// the recorder cannot be created.
    pub fn new(mut context: graphite::Context) -> Option<Self> {
        let recorder = context.make_recorder(None)?;
        Some(Self { state: RefCell::new(Some(GraphiteState { uploaded_images: HashMap::new(), recorder, context })) })
    }

    fn with_state<R>(&self, f: impl FnOnce(&mut GraphiteState) -> R) -> R {
        let mut state = self.state.borrow_mut();
        f(state.as_mut().expect("the Graphite context has been disposed"))
    }

    /// Runs `f` with the recorder, for creating surfaces that wrap platform
    /// textures.
    pub fn with_recorder<R>(&self, f: impl FnOnce(&mut graphite::Recorder) -> R) -> R {
        self.with_state(|state| f(&mut state.recorder))
    }

    /// Releases the recorder and the context.
    pub fn dispose(&self) {
        *self.state.borrow_mut() = None;
    }

    /// Whether the context has been disposed.
    pub fn is_disposed(&self) -> bool {
        self.state.borrow().is_none()
    }

    fn flush_state(state: &mut GraphiteState) {
        if let Some(mut recording) = state.recorder.snap() {
            let info = InsertRecordingInfo::new(&mut recording);
            state.context.insert_recording(&info);
        }
        state.context.submit(None);

        // The texture of an image nothing else holds cannot be asked for
        // again. The recording that was just inserted keeps the textures it
        // draws with alive by itself.
        state.uploaded_images.retain(|_, uploaded| uploaded.is_source_in_use());
    }

    /// Makes a copy of a texture image that has mipmaps.
    ///
    /// The Skia binding in use uploads an image without mipmaps only. A
    /// copy of a surface can be asked for with mipmaps, so the texture is
    /// drawn onto a surface of its size and the copy of that is taken.
    fn make_mipmapped_texture(recorder: &mut graphite::Recorder, texture: &Image) -> Option<Image> {
        let mut surface =
            graphite::surfaces::render_target(recorder, texture.image_info(), Mipmapped::No, None, None)?;
        let mut paint = Paint::default();
        paint.set_blend_mode(BlendMode::Src);
        surface.canvas().draw_image_with_sampling_options(texture, (0.0, 0.0), SamplingOptions::default(), Some(&paint));
        graphite::surfaces::as_image_copy(&surface, None, Mipmapped::Yes)
    }

    fn log_upload_failure(image: &Image, message_template: &str) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
            logger.log_with_values(None, message_template, &[&image.width(), &image.height()]);
        }
    }

    /// The number of raster images the context keeps an upload for.
    pub fn uploaded_image_count(&self) -> usize {
        self.state.borrow().as_ref().map_or(0, |state| state.uploaded_images.len())
    }
}

impl ISkiaGrContext for GraphiteGrContext {
    fn backend(&self) -> SkiaGpuBackend {
        SkiaGpuBackend::Graphite
    }

    fn create_surface(&self, image_info: &ImageInfo, surface_props: &SurfaceProps) -> Option<Surface> {
        self.with_state(|state| {
            graphite::surfaces::render_target(
                &mut state.recorder,
                image_info,
                Mipmapped::No,
                Some(surface_props),
                None,
            )
        })
    }

    fn max_render_target_size(&self) -> Option<i32> {
        // Graphite does not report its texture size limit.
        None
    }

    fn flush(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(state) = state.as_mut() {
            Self::flush_state(state);
        }
    }

    fn reset_context(&self) {
        // Graphite keeps no shadow copy of the graphics API state.
    }

    fn set_resource_cache_limit(&self, _max_resource_bytes: i64) {
        // The Graphite resource budget is fixed when the context is created.
    }

    fn snapshot_to_raster(&self, surface: &mut Surface) -> Option<Image> {
        self.with_state(|state| {
            // The readback does not see work that is still on the recorder.
            Self::flush_state(state);

            let info = surface.image_info().with_alpha_type(AlphaType::Premul);
            let row_bytes = info.min_row_bytes();
            let mut pixels = vec![0u8; row_bytes * info.height().max(0) as usize];

            if !state.context.read_pixels(surface, &info, &mut pixels, row_bytes, (0, 0)) {
                return None;
            }

            images::raster_from_data(&info, Data::new_copy(&pixels), row_bytes)
        })
    }

    fn drawable_image(&self, image: &Image, mipmapped: bool) -> Image {
        if image.is_texture_backed() {
            return image.clone();
        }

        let mut state = self.state.borrow_mut();
        let Some(GraphiteState { uploaded_images, recorder, .. }) = state.as_mut() else {
            return image.clone();
        };

        // The contents of an image never change, and its id is not given to
        // another image while the entry holds the image.
        let uploaded = uploaded_images.entry(image.unique_id()).or_insert_with(|| {
            let texture = graphite::images::texture_from_image(recorder, image);
            if texture.is_none() {
                Self::log_upload_failure(image, "Unable to upload a {Width}x{Height} image to the GPU; it is not drawn.");
            }
            UploadedImage { source: image.clone(), texture, mipmapped_texture: None }
        });

        let Some(texture) = &uploaded.texture else {
            return image.clone();
        };
        if !mipmapped {
            return texture.clone();
        }

        let mipmapped_texture = uploaded.mipmapped_texture.get_or_insert_with(|| {
            let mipmapped_texture = Self::make_mipmapped_texture(recorder, texture);
            if mipmapped_texture.is_none() {
                Self::log_upload_failure(
                    image,
                    "Unable to make the mipmaps of a {Width}x{Height} image on the GPU; it is drawn without them.",
                );
            }
            mipmapped_texture
        });
        mipmapped_texture.as_ref().unwrap_or(texture).clone()
    }

    fn is_lost(&self) -> bool {
        self.state.borrow().as_ref().is_some_and(|state| state.context.is_device_lost())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
