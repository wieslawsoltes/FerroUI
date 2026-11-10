use super::pipelined_layer::{BandParts, BandPool};
use super::GraphitePrewarmContext;
use crate::gpu::{ISkiaGrContext, SkiaGpuBackend, SkiaPrewarm};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::rendering::composition::frames::{FrameError, FrameLease, GpuCompletion};
use skia_safe::gpu::graphite::{self, InsertRecordingInfo};
use skia_safe::gpu::Mipmapped;
use skia_safe::{
    images, AlphaType, BlendMode, ConditionallySend, Data, Image, ImageInfo, Paint,
    SamplingOptions, Surface, SurfaceProps,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

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

/// The context behind a lock: the render thread and the threads of the
/// pipelined frame layers, which insert their own recordings
/// (`pipelined_layer.rs`), use it one at a time.
pub(super) struct SharedContext(Mutex<graphite::Context>);

// SAFETY: a Graphite context is not bound to the thread that made it; Skia
// asks only that one thread uses it at a time, which the lock gives. It is
// dropped on the render thread: the threads of the bands, the only other
// holders, are joined before the state lets go of its reference.
unsafe impl Send for SharedContext {}
// SAFETY: as above; every use goes through the lock.
unsafe impl Sync for SharedContext {}

impl SharedContext {
    pub(super) fn lock(&self) -> MutexGuard<'_, graphite::Context> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn try_lock(&self) -> Option<MutexGuard<'_, graphite::Context>> {
        match self.0.try_lock() {
            Ok(guard) => Some(guard),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }
}

/// The mutable halves of a Graphite GPU context. The uploaded images are
/// declared first so that they are released before the recorder they live
/// on, and the recorder before the context that created it. The threads
/// that play the frames of the pipelined layers come before all of them:
/// each owns a recorder of the context, and they are joined before the
/// context goes.
struct GraphiteState {
    /// The threads of the pipelined frame layers (`pipelined_layer.rs`);
    /// none until a layer pipelines its first frame.
    bands: BandPool,
    /// The textures of the raster images that were drawn, by the unique id
    /// of the image.
    uploaded_images: HashMap<u32, UploadedImage>,
    recorder: graphite::Recorder,
    context: Arc<SharedContext>,
    frame_completion: GpuCompletion,
    completion_handler: Option<Box<dyn Fn(GpuCompletion)>>,
    /// A recording failed to insert. With ordered recordings (section 9.17) the context refuses
    /// every later recording of the recorder, so the context reports itself lost and is made
    /// again, as after a lost device.
    recording_failed: bool,
}

impl Drop for GraphiteState {
    fn drop(&mut self) {
        // This token covers only commands not yet snapped/submitted; submitted
        // tokens have moved to native completion callbacks in flush_state.
        self.frame_completion.complete();
    }
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
    frame_context_id: u64,
    /// How many raster images were uploaded (or tried).
    uploads: Cell<u64>,
    /// How many draws only this recorder draws as an ordinary layer would
    /// were made with the context
    /// ([`note_direct_only_draw`](ISkiaGrContext::note_direct_only_draw)):
    /// what a pipelined frame layer compares between and over its frames
    /// (`pipelined_layer.rs`).
    direct_only_draws: Cell<u64>,
    /// A pipelined layer was drawn onto a canvas of the recorder since the
    /// last flush: that flush is the one the frame in the threads of the
    /// layer waits for.
    layer_blit: Cell<bool>,
    /// When the last flush ended: where the cost of a frame that a layer
    /// drew directly ends.
    last_flush_end: Cell<Option<Instant>>,
}

impl GraphiteGrContext {
    /// Wraps a Graphite context, creating its recorder. Returns `None` when
    /// the recorder cannot be created.
    pub fn new(mut context: graphite::Context) -> Option<Self> {
        let recorder = context.make_recorder(None)?;
        Some(Self {
            state: RefCell::new(Some(GraphiteState {
                bands: BandPool::default(),
                uploaded_images: HashMap::new(),
                recorder,
                context: Arc::new(SharedContext(Mutex::new(context))),
                frame_completion: GpuCompletion::new(),
                completion_handler: None,
                recording_failed: false,
            })),
            frame_context_id: crate::gpu::next_frame_context_id(),
            uploads: Cell::new(0),
            direct_only_draws: Cell::new(0),
            layer_blit: Cell::new(false),
            last_flush_end: Cell::new(None),
        })
    }

    /// The count of the draws only this recorder draws as an ordinary layer
    /// would (see the field).
    pub(super) fn direct_only_draws(&self) -> u64 {
        self.direct_only_draws.get()
    }

    /// A pipelined layer was drawn onto a canvas of the recorder.
    pub(super) fn note_layer_blit(&self) {
        self.layer_blit.set(true);
    }

    /// When the last flush of the context ended; `None` before the first.
    pub(super) fn last_flush_end(&self) -> Option<Instant> {
        self.last_flush_end.get()
    }

    /// Runs `f` with what pipelining a frame of a layer needs: the threads,
    /// the recorder and the context. `None` when the context has been
    /// disposed or is in use (a frame that ends inside another use of the
    /// context), which the layer answers by not using the threads.
    pub(super) fn with_band_parts<R>(&self, f: impl FnOnce(BandParts<'_>) -> R) -> Option<R> {
        let Ok(mut state) = self.state.try_borrow_mut() else {
            return None;
        };
        let state = state.as_mut()?;
        Some(f(BandParts {
            pool: &mut state.bands,
            recorder: &mut state.recorder,
            context: &state.context,
            recording_failed: &mut state.recording_failed,
        }))
    }

    fn with_state<R>(&self, f: impl FnOnce(&mut GraphiteState) -> R) -> R {
        let mut state = self.state.borrow_mut();
        f(state
            .as_mut()
            .expect("the Graphite context has been disposed"))
    }

    /// Runs `f` with the recorder, for creating surfaces that wrap platform
    /// textures.
    pub fn with_recorder<R>(&self, f: impl FnOnce(&mut graphite::Recorder) -> R) -> R {
        self.with_state(|state| f(&mut state.recorder))
    }

    /// Installs native asynchronous completion on this context's actual queue.
    pub(crate) fn set_frame_completion_handler(&self, handler: Box<dyn Fn(GpuCompletion)>) {
        self.with_state(|state| state.completion_handler = Some(handler));
    }

    /// Releases the recorder and the context.
    pub fn dispose(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(state) = state.as_mut() {
            self.flush_now(state);
        }
        *state = None;
    }

    /// Whether the context has been disposed.
    pub fn is_disposed(&self) -> bool {
        self.state.borrow().is_none()
    }

    /// Flushes the recorder and notes when the flush ended.
    fn flush_now(&self, state: &mut GraphiteState) {
        Self::flush_state(state, self.layer_blit.take());
        self.last_flush_end.set(Some(Instant::now()));
    }

    /// `blitted`: a pipelined layer was drawn onto a canvas of the recorder
    /// since the last flush.
    fn flush_state(state: &mut GraphiteState, blitted: bool) {
        // The recordings the threads of the pipelined layers were told to
        // insert come before the recording of this recorder, all of them:
        // what it draws may be the texture they are drawing on, or what a
        // frame they are playing sampled, drawn anew. Nothing is waited for
        // when no frame is in the threads.
        state.bands.wait_for_released();
        let completion = std::mem::take(&mut state.frame_completion);
        if let Some(mut recording) = state.recorder.snap() {
            let info = InsertRecordingInfo::new(&mut recording);
            let (inserted, submitted) = {
                let mut context = state.context.lock();
                let inserted = context.insert_recording(&info) == graphite::InsertStatus::Success;
                (inserted, context.submit(None))
            };
            state.recording_failed |= !inserted;
            if inserted && submitted {
                if completion.retained_frames() == 0 {
                    completion.complete();
                } else if let Some(handler) = &state.completion_handler {
                    handler(completion);
                } else {
                    completion.mark_indeterminate();
                }
            } else {
                // Submission failure cannot establish whether commands ran.
                completion.mark_indeterminate();
            }
        } else {
            // Preserve the context's ordinary empty-flush processing too.
            let _ = state.context.lock().submit(None);
            // A fully clipped draw can retain a lease without recording a GPU
            // read. No recording means no read was submitted, so no fence is
            // required and an empty-submit result must not quarantine storage.
            completion.complete();
        }

        if blitted {
            // The draw of the layer is in the context: the threads may
            // insert the frame they played after it.
            state.bands.release();
        }

        // The texture of an image nothing else holds cannot be asked for
        // again. The recording that was just inserted keeps the textures it
        // draws with alive by itself.
        state
            .uploaded_images
            .retain(|_, uploaded| uploaded.is_source_in_use());
    }

    /// Makes a copy of a texture image that has mipmaps.
    ///
    /// The Skia binding in use uploads an image without mipmaps only. A
    /// copy of a surface can be asked for with mipmaps, so the texture is
    /// drawn onto a surface of its size and the copy of that is taken.
    fn make_mipmapped_texture(recorder: &mut graphite::Recorder, texture: &Image) -> Option<Image> {
        let mut surface = graphite::surfaces::render_target(
            recorder,
            texture.image_info(),
            Mipmapped::No,
            None,
            None,
        )?;
        let mut paint = Paint::default();
        paint.set_blend_mode(BlendMode::Src);
        surface.canvas().draw_image_with_sampling_options(
            texture,
            (0.0, 0.0),
            SamplingOptions::default(),
            Some(&paint),
        );
        graphite::surfaces::as_image_copy(&surface, None, Mipmapped::Yes)
    }

    fn log_upload_failure(image: &Image, message_template: &str) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
            logger.log_with_values(None, message_template, &[&image.width(), &image.height()]);
        }
    }

    /// The number of raster images the context keeps an upload for.
    pub fn uploaded_image_count(&self) -> usize {
        self.state
            .borrow()
            .as_ref()
            .map_or(0, |state| state.uploaded_images.len())
    }

    /// How many raster images the context uploaded (or tried to) since it
    /// was created: constant over frames that draw only images it holds the
    /// texture of.
    pub fn upload_count(&self) -> u64 {
        self.uploads.get()
    }
}

impl ISkiaGrContext for GraphiteGrContext {
    fn frame_context_id(&self) -> Option<u64> {
        Some(self.frame_context_id)
    }
    fn retain_frame(&self, frame: FrameLease) -> Result<(), FrameError> {
        let mut state = self.state.borrow_mut();
        let state = state.as_mut().ok_or(FrameError::StaleGeneration)?;
        if state.completion_handler.is_none() {
            return Err(FrameError::MissingSynchronization);
        }
        // The storage is held until the commands of the next flush of this
        // recorder have run: the draw that reads it belongs to that flush.
        self.direct_only_draws.set(self.direct_only_draws.get().wrapping_add(1));
        state.frame_completion.retain(frame)
    }

    fn backend(&self) -> SkiaGpuBackend {
        SkiaGpuBackend::Graphite
    }

    fn create_surface(
        &self,
        image_info: &ImageInfo,
        surface_props: &SurfaceProps,
    ) -> Option<Surface> {
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
            self.flush_now(state);
        }
    }

    fn note_direct_only_draw(&self) {
        self.direct_only_draws.set(self.direct_only_draws.get().wrapping_add(1));
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
            self.flush_now(state);

            let info = surface.image_info().with_alpha_type(AlphaType::Premul);
            let row_bytes = info.min_row_bytes();
            let mut pixels = vec![0u8; row_bytes * info.height().max(0) as usize];

            if !state
                .context
                .lock()
                .read_pixels(surface, &info, &mut pixels, row_bytes, (0, 0))
            {
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
        let Some(GraphiteState {
            uploaded_images,
            recorder,
            ..
        }) = state.as_mut()
        else {
            return image.clone();
        };

        // The contents of an image never change, and its id is not given to
        // another image while the entry holds the image.
        let uploaded = uploaded_images.entry(image.unique_id()).or_insert_with(|| {
            self.uploads.set(self.uploads.get() + 1);
            let texture = graphite::images::texture_from_image(recorder, image);
            if texture.is_none() {
                Self::log_upload_failure(
                    image,
                    "Unable to upload a {Width}x{Height} image to the GPU; it is not drawn.",
                );
            }
            UploadedImage {
                source: image.clone(),
                texture,
                mipmapped_texture: None,
            }
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
        self.state
            .borrow()
            .as_ref()
            .is_some_and(|state| {
                // A thread of a pipelined layer that inserts holds the
                // context for a moment; the answer then waits for the next
                // ask.
                state.context.try_lock().is_some_and(|context| context.is_device_lost()) || state.recording_failed
            })
    }

    /// A recorder of its own, made by the context, whose pipelines compile
    /// on a thread of its own (`graphite_prewarm.rs`).
    fn begin_prewarm(&self) -> Option<SkiaPrewarm> {
        let recorder = {
            let mut state = self.state.borrow_mut();
            let state = state.as_mut()?;
            let mut context = state.context.lock();
            if context.is_device_lost() {
                return None;
            }
            context.make_recorder(None)?
        };
        let prewarm = Rc::new(GraphitePrewarmContext::new(recorder));
        let finishing = prewarm.clone();
        Some(SkiaPrewarm {
            gr_context: prewarm,
            finish: Box::new(move || {
                // The thread runs on by itself.
                let _ = finishing.finish();
            }),
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
