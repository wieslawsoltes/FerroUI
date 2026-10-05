use crate::gpu::{ISkiaGrContext, SkiaGpuBackend};
use skia_safe::gpu::graphite::{self, InsertRecordingInfo};
use skia_safe::gpu::Mipmapped;
use skia_safe::{images, AlphaType, Data, Image, ImageInfo, Surface, SurfaceProps};
use std::any::Any;
use std::cell::RefCell;

/// The mutable halves of a Graphite GPU context. The recorder is declared
/// first so that it is released before the context that created it.
struct GraphiteState {
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
pub struct GraphiteGrContext {
    state: RefCell<Option<GraphiteState>>,
}

impl GraphiteGrContext {
    /// Wraps a Graphite context, creating its recorder. Returns `None` when
    /// the recorder cannot be created.
    pub fn new(mut context: graphite::Context) -> Option<Self> {
        let recorder = context.make_recorder(None)?;
        Some(Self { state: RefCell::new(Some(GraphiteState { recorder, context })) })
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

    fn is_lost(&self) -> bool {
        self.state.borrow().as_ref().is_some_and(|state| state.context.is_device_lost())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
