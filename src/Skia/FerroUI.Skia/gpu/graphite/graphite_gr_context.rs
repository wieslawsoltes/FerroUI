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

/// Every how many flushes without an upload the uploaded images that nothing else holds are
/// dropped.
pub(crate) const UPLOADED_IMAGE_SWEEP_INTERVAL: u64 = 60;
/// Whether flush number `flush` drops the textures of the uploaded images nothing else holds,
/// when the last image was uploaded before flush number `last_upload_flush + 1`: with every
/// flush until UPLOADED_IMAGE_SWEEP_INTERVAL flushes after an upload, then every
/// UPLOADED_IMAGE_SWEEP_INTERVAL flushes.
pub(crate) fn uploaded_image_sweep_due(flush: u64, last_upload_flush: u64) -> bool {
    flush.saturating_sub(last_upload_flush) <= UPLOADED_IMAGE_SWEEP_INTERVAL
        || flush % UPLOADED_IMAGE_SWEEP_INTERVAL == 0
}

/// Every how many flushes Graphite is asked to free what it has not used for
/// DEFERRED_CLEANUP_AGE_MS (scratch textures, atlas pages): about two seconds at 60 frames a
/// second.
pub(crate) const DEFERRED_CLEANUP_INTERVAL: u64 = 120;
/// How long a resource has to be unused before the periodic cleanup frees it.
const DEFERRED_CLEANUP_AGE_MS: i64 = 15_000;

/// The submit info of a flush: a frame boundary with the frame's id (`fMarkBoundary`,
/// `fFrameID`), so that Graphite knows where a frame ends. skia-safe has no setter, so the
/// native fields are set through `skia-bindings`, as for `fRequireOrderedRecordings`
/// (desktop-performance.md, section 9.17).
fn frame_submit_info(frame_id: u64) -> graphite::SubmitInfo {
    // `SubmitInfo` is a struct of the one native field: of the same size, the field is at offset 0.
    const _: () = assert!(
        std::mem::size_of::<graphite::SubmitInfo>()
            == std::mem::size_of::<skia_bindings::skgpu_graphite_SubmitInfo>()
    );
    let mut info = graphite::SubmitInfo::new();
    // SAFETY: by the assertion the value is the native structure; both fields are plain values.
    unsafe {
        let native = &mut *(&mut info as *mut graphite::SubmitInfo).cast::<skia_bindings::skgpu_graphite_SubmitInfo>();
        native.fMarkBoundary = skia_bindings::skgpu_graphite_MarkFrameBoundary::kYes;
        native.fFrameID = frame_id;
    }
    info
}

/// Asks the context and the recorder to free the resources they have not used for
/// DEFERRED_CLEANUP_AGE_MS (`performDeferredCleanup`, which skia-safe does not have). It frees
/// on the CPU what the GPU no longer uses and waits for nothing.
fn perform_deferred_cleanup(context: &mut graphite::Context, recorder: &mut graphite::Recorder) {
    // A `std::chrono::milliseconds` is its count; a disengaged `std::optional` of the longest
    // purge time is all zeros (no limit).
    // SAFETY: the duration is an `i64` in a structure of that size (checked by `transmute`).
    let age = unsafe { std::mem::transmute::<i64, skia_bindings::std_chrono_milliseconds>(DEFERRED_CLEANUP_AGE_MS) };
    let no_limit = || skia_bindings::__BindgenOpaqueArray([0u64; 2]);
    // SAFETY: `RefHandle` is `repr(transparent)` over the pointer to the native object, which
    // both handles own and which is alive for the call; the context is behind its lock.
    unsafe {
        let context = *(context as *mut graphite::Context).cast::<*mut skia_bindings::skgpu_graphite_Context>();
        (*context).performDeferredCleanup(age, no_limit());
        let recorder = *(recorder as *mut graphite::Recorder).cast::<*mut skia_bindings::skgpu_graphite_Recorder>();
        (*recorder).performDeferredCleanup(age, no_limit());
    }
}

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
    /// The flushes of the context so far: the id of the frame each one ends, and the clock of the
    /// periodic cleanup and sweep.
    flushes: u64,
    /// The value of `flushes` when an image was last uploaded.
    last_upload_flush: u64,
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
    /// The context draws with a Vulkan device (#1365): the images of its layers are not held by the
    /// GPU's work ([`retire`](Self::retire)), and its disposal waits for the GPU.
    #[cfg(feature = "graphite-vulkan")]
    vulkan: bool,
    /// What the layers let go of since the last flush and the GPU may still use: dropped after the
    /// next flush has waited for the GPU (Vulkan only).
    #[cfg(feature = "graphite-vulkan")]
    retired: RefCell<Vec<Box<dyn Any>>>,
    /// The window surface the next flush leaves in a given state (Vulkan only).
    #[cfg(feature = "graphite-vulkan")]
    frame_target: RefCell<Option<FrameTarget>>,
}

/// The surface of a window frame and the state its texture is left in once the frame's recording
/// has run (`fTargetSurface` and `fTargetTextureState` of the insertion): on Vulkan the layout the
/// present path expects the image in, which Graphite's record of the layout keeps from then on.
#[cfg_attr(not(feature = "graphite-vulkan"), allow(dead_code))]
pub(crate) struct FrameTarget {
    pub(crate) surface: Surface,
    pub(crate) state: skia_safe::gpu::MutableTextureState,
}

/// Names the target of an insertion and the state its texture is left in once the recording has
/// run (skia-safe sets neither; through `skia-bindings`, as for `fRequireOrderedRecordings`,
/// desktop-performance.md section 9.17). A recording without a deferred canvas draws where it drew:
/// the target only names the surface whose texture is transitioned.
fn set_target_state(info: &mut InsertRecordingInfo, target: &FrameTarget) {
    // The wrapper holds the pointer to the native structure and a marker of no size: of the size of a
    // pointer, the pointer is at offset 0. Surface and state handles are `repr(transparent)` over the
    // pointer to their native objects.
    const _: () = assert!(std::mem::size_of::<InsertRecordingInfo<'static>>() == std::mem::size_of::<*mut ()>());
    // SAFETY: by the assertion the first word of the wrapper is the native structure it owns; the surface and the
    // state outlive the insertion (the caller holds them), which reads them and takes references of its own.
    unsafe {
        let native =
            *(info as *mut InsertRecordingInfo).cast::<*mut skia_bindings::skgpu_graphite_InsertRecordingInfo>();
        (*native).fTargetSurface = *(&target.surface as *const Surface).cast::<*mut skia_bindings::SkSurface>();
        let state = *(&target.state as *const skia_safe::gpu::MutableTextureState)
            .cast::<*mut skia_bindings::skgpu_MutableTextureState>();
        (*native).fTargetTextureState = state as _;
    }
}

impl GraphiteGrContext {
    /// Wraps a Graphite context, creating its recorder. Returns `None` when
    /// the recorder cannot be created.
    pub fn new(mut context: graphite::Context) -> Option<Self> {
        crate::graphite_metal_ownership::report_once();
        let recorder = context.make_recorder(None)?;
        Some(Self {
            state: RefCell::new(Some(GraphiteState {
                bands: BandPool::default(),
                uploaded_images: HashMap::new(),
                recorder,
                context: Arc::new(SharedContext(Mutex::new(context))),
                frame_completion: GpuCompletion::new(),
                completion_handler: None,
                flushes: 0,
                last_upload_flush: 0,
                recording_failed: false,
            })),
            frame_context_id: crate::gpu::next_frame_context_id(),
            uploads: Cell::new(0),
            direct_only_draws: Cell::new(0),
            layer_blit: Cell::new(false),
            last_flush_end: Cell::new(None),
            #[cfg(feature = "graphite-vulkan")]
            vulkan: false,
            #[cfg(feature = "graphite-vulkan")]
            retired: RefCell::new(Vec::new()),
            #[cfg(feature = "graphite-vulkan")]
            frame_target: RefCell::new(None),
        })
    }

    /// Wraps a Graphite context of a Vulkan device (#1365): as [`new`](Self::new), with the threads
    /// of the pipelined layers drawing on no shared target (`BandInsert::with_shared_target`), the
    /// images of the layers kept until the GPU has finished with them, and a disposal that waits
    /// for the GPU.
    #[cfg(feature = "graphite-vulkan")]
    pub(crate) fn on_vulkan(context: graphite::Context) -> Option<Self> {
        let mut this = Self::new(context)?;
        this.vulkan = true;
        if let Some(state) = this.state.get_mut().as_mut() {
            state.bands.forbid_shared_target();
        }
        Some(this)
    }

    /// Keeps what a layer let go of, which the GPU may still use (the image of a layer on Vulkan),
    /// until the next flush has submitted the last use and waited for the GPU. Dropped at once when
    /// the context has been disposed (its disposal waited for the GPU).
    pub(super) fn retire(&self, value: Box<dyn Any>) {
        if self.is_disposed() {
            return;
        }
        #[cfg(feature = "graphite-vulkan")]
        self.retired.borrow_mut().push(value);
        // Only a Vulkan image is retired (Metal's textures are held by the GPU's work).
        #[cfg(not(feature = "graphite-vulkan"))]
        drop(value);
    }

    /// Flushes the recorder, leaving the texture of the window surface `target` in `state` once the
    /// recording has run (the end of a window frame on Vulkan).
    #[cfg(feature = "graphite-vulkan")]
    pub(crate) fn flush_to_target(&self, target: FrameTarget) {
        *self.frame_target.borrow_mut() = Some(target);
        self.flush();
        // A flush that found the context disposed leaves the target behind.
        self.frame_target.borrow_mut().take();
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

    /// Submits the band recordings the threads of the pipelined layers inserted since the last
    /// submit, without waiting for anything (the threads may still be inserting the rest of a
    /// frame; those go with the next submit). Called at the start of a frame, so that the GPU
    /// draws the bands of the frame before while this one is recorded. Nothing happens when no
    /// band was inserted, or when the context is in use.
    pub(crate) fn submit_inserted_bands(&self) {
        let Ok(state) = self.state.try_borrow() else { return };
        let Some(state) = state.as_ref() else { return };
        if state.bands.take_unsubmitted() == 0 {
            return;
        }
        let _ = state.context.lock().submit(None);
    }

    /// Chooses how the band threads, when they are made, insert (for the tests; #1363).
    #[cfg(test)]
    pub(crate) fn set_band_insert_for_tests(&self, mode: super::pipelined_layer::BandInsert) {
        if let Some(state) = self.state.borrow_mut().as_mut() {
            state.bands.set_insert_mode(mode);
        }
    }

    /// How many band recordings the threads inserted that no submit has taken (for the tests;
    /// reading it takes them).
    #[cfg(test)]
    pub(crate) fn take_unsubmitted_band_inserts(&self) -> usize {
        self.state.borrow().as_ref().map_or(0, |state| state.bands.take_unsubmitted())
    }

    /// Runs `f` with the recorder, for creating surfaces that wrap platform
    /// textures.
    pub fn with_recorder<R>(&self, f: impl FnOnce(&mut graphite::Recorder) -> R) -> R {
        self.with_state(|state| f(&mut state.recorder))
    }

    /// Installs native asynchronous completion on this context's actual queue.
    #[cfg_attr(not(target_vendor = "apple"), allow(dead_code))]
    pub(crate) fn set_frame_completion_handler(&self, handler: Box<dyn Fn(GpuCompletion)>) {
        self.with_state(|state| state.completion_handler = Some(handler));
    }

    /// Releases the recorder and the context.
    pub fn dispose(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(state) = state.as_mut() {
            self.flush_now(state);
            #[cfg(feature = "graphite-vulkan")]
            if self.vulkan {
                // The disposal of the GPU waits for it: no image of FerroUI that Graphite drew on (a layer's, a
                // window's) is destroyed while the GPU uses it.
                let _ = state.context.lock().submit_and_wait();
            }
        }
        *state = None;
        drop(state);
        #[cfg(feature = "graphite-vulkan")]
        self.retired.borrow_mut().clear();
    }

    /// Whether the context has been disposed.
    pub fn is_disposed(&self) -> bool {
        self.state.borrow().is_none()
    }

    /// Flushes the recorder and notes when the flush ended.
    #[cfg(not(feature = "graphite-vulkan"))]
    fn flush_now(&self, state: &mut GraphiteState) {
        Self::flush_state(state, self.layer_blit.take(), None);
        self.last_flush_end.set(Some(Instant::now()));
    }

    /// Flushes the recorder and notes when the flush ended; on Vulkan with the frame target, and
    /// with what the layers let go of dropped once the GPU has finished with it.
    #[cfg(feature = "graphite-vulkan")]
    fn flush_now(&self, state: &mut GraphiteState) {
        let retiring = std::mem::take(&mut *self.retired.borrow_mut());
        let target = self.frame_target.borrow_mut().take();
        Self::flush_state(state, self.layer_blit.take(), target.as_ref());
        if !retiring.is_empty() {
            // What the layers let go of since the last flush (the images of layers on Vulkan) goes once the GPU
            // has finished its last use, which this flush submitted: a wait of a resize or of the disposal of a
            // layer, once, never one of an ordinary frame.
            let _ = state.context.lock().submit_and_wait();
        }
        drop(retiring);
        drop(target);
        self.last_flush_end.set(Some(Instant::now()));
    }

    /// `blitted`: a pipelined layer was drawn onto a canvas of the recorder
    /// since the last flush.
    /// `target`: the window surface whose texture the recording leaves in a given state.
    fn flush_state(state: &mut GraphiteState, blitted: bool, target: Option<&FrameTarget>) {
        // The recordings the threads of the pipelined layers were told to
        // insert come before the recording of this recorder, all of them:
        // what it draws may be the texture they are drawing on, or what a
        // frame they are playing sampled, drawn anew. Nothing is waited for
        // when no frame is in the threads.
        state.bands.wait_for_released();
        state.flushes += 1;
        let flush = state.flushes;
        // Every flush ends a frame of the window (or a read-back): Graphite is told where, with the frame's id.
        let submit_info = frame_submit_info(flush);
        let completion = std::mem::take(&mut state.frame_completion);
        if let Some(mut recording) = state.recorder.snap() {
            let mut info = InsertRecordingInfo::new(&mut recording);
            if let Some(target) = target {
                set_target_state(&mut info, target);
            }
            let (inserted, submitted) = {
                let mut context = state.context.lock();
                let inserted = context.insert_recording(&info) == graphite::InsertStatus::Success;
                // The submit takes the band recordings inserted before it too.
                state.bands.take_unsubmitted();
                let submitted = context.submit(Some(&submit_info));
                if flush % DEFERRED_CLEANUP_INTERVAL == 0 {
                    perform_deferred_cleanup(&mut context, &mut state.recorder);
                }
                (inserted, submitted)
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
            let mut context = state.context.lock();
            state.bands.take_unsubmitted();
            let _ = context.submit(Some(&submit_info));
            if flush % DEFERRED_CLEANUP_INTERVAL == 0 {
                perform_deferred_cleanup(&mut context, &mut state.recorder);
            }
            drop(context);
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
        // draws with alive by itself. The map is walked with every flush while
        // images are uploaded (content that changes, whose old images go as
        // fast as the new ones come), and every UPLOADED_IMAGE_SWEEP_INTERVAL
        // flushes once nothing was uploaded for that long.
        if uploaded_image_sweep_due(flush, state.last_upload_flush) {
            state
                .uploaded_images
                .retain(|_, uploaded| uploaded.is_source_in_use());
        }
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
            flushes,
            last_upload_flush,
            ..
        }) = state.as_mut()
        else {
            return image.clone();
        };

        // The contents of an image never change, and its id is not given to
        // another image while the entry holds the image.
        let uploaded = uploaded_images.entry(image.unique_id()).or_insert_with(|| {
            self.uploads.set(self.uploads.get() + 1);
            *last_upload_flush = *flushes;
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

#[cfg(test)]
mod schedule_tests {
    use super::*;

    #[test]
    fn released_images_are_swept_with_every_flush_while_images_are_uploaded() {
        for flush in 1..=UPLOADED_IMAGE_SWEEP_INTERVAL {
            assert!(uploaded_image_sweep_due(flush, 0), "flush {flush}");
        }
        assert!(uploaded_image_sweep_due(1000, 999));
        assert!(uploaded_image_sweep_due(1000 + UPLOADED_IMAGE_SWEEP_INTERVAL, 1000));
    }

    #[test]
    fn without_uploads_released_images_are_swept_every_interval() {
        let last_upload = 10;
        let first_idle = last_upload + UPLOADED_IMAGE_SWEEP_INTERVAL + 1;
        let idle = first_idle..(last_upload + 10 * UPLOADED_IMAGE_SWEEP_INTERVAL);
        let swept: Vec<u64> = idle.filter(|flush| uploaded_image_sweep_due(*flush, last_upload)).collect();
        assert!(swept.iter().all(|flush| flush % UPLOADED_IMAGE_SWEEP_INTERVAL == 0), "{swept:?}");
        // At least one sweep in every interval: a released image goes within that many flushes.
        assert!(swept.windows(2).all(|pair| pair[1] - pair[0] == UPLOADED_IMAGE_SWEEP_INTERVAL));
        assert!(!swept.is_empty());
    }

    #[test]
    fn a_flush_marks_a_frame_boundary_with_its_id() {
        let info = frame_submit_info(42);
        // SAFETY: as in `frame_submit_info`, the value is the native structure.
        let native =
            unsafe { &*(&info as *const graphite::SubmitInfo).cast::<skia_bindings::skgpu_graphite_SubmitInfo>() };
        assert_eq!(native.fMarkBoundary, skia_bindings::skgpu_graphite_MarkFrameBoundary::kYes);
        assert_eq!(native.fFrameID, 42);
        assert_eq!(native.fSync, skia_bindings::skgpu_graphite_SyncToCpu::No);
    }
}
