use crate::gpu::{ISkiaGrContext, SkiaGpuBackend};
use ferroui_base::rendering::composition::frames::{FrameError, FrameLease, GpuCompletion};
use skia_safe::gpu::{self, Budgeted, DirectContext, SurfaceOrigin};
use skia_safe::image::CachingHint;
use skia_safe::{Image, ImageInfo, Surface, SurfaceProps};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// A Ganesh direct context.
///
/// Ganesh keeps a shadow copy of the state of the graphics API, so code that
/// uses the API directly has to call
/// [`reset_context`](ISkiaGrContext::reset_context) before Skia draws again.
pub struct GaneshGrContext {
    context: RefCell<Option<DirectContext>>,
    frame_context_id: u64,
    frame_completion: RefCell<GpuCompletion>,
    frame_completion_handler: RefCell<Option<Rc<dyn Fn(GpuCompletion)>>>,
}
impl Drop for GaneshGrContext {
    fn drop(&mut self) {
        self.frame_completion.get_mut().mark_indeterminate();
    }
}

impl GaneshGrContext {
    /// Wraps a direct context.
    pub fn new(context: DirectContext) -> Self {
        Self {
            context: RefCell::new(Some(context)),
            frame_context_id: crate::gpu::next_frame_context_id(),
            frame_completion: RefCell::new(GpuCompletion::new()),
            frame_completion_handler: RefCell::new(None),
        }
    }
    pub(crate) fn set_frame_completion_handler(&self, handler: Rc<dyn Fn(GpuCompletion)>) {
        *self.frame_completion_handler.borrow_mut() = Some(handler);
    }
    fn finish_frame_submission(&self) {
        let completion = std::mem::take(&mut *self.frame_completion.borrow_mut());
        if completion.retained_frames() == 0 {
            completion.complete();
            return;
        }
        let handler = self.frame_completion_handler.borrow().clone();
        if let Some(handler) = handler {
            handler(completion);
        } else {
            completion.mark_indeterminate();
        }
    }

    /// Runs `f` with the direct context, for creating surfaces that wrap
    /// platform framebuffers.
    ///
    /// # Panics
    /// Panics when the context has been disposed.
    pub fn with_context<R>(&self, f: impl FnOnce(&mut DirectContext) -> R) -> R {
        let mut context = self.context.borrow_mut();
        f(context
            .as_mut()
            .expect("the Ganesh context has been disposed"))
    }

    /// Abandons and releases the context. With `release_resources` the GPU
    /// resources are freed first, which needs the platform context to be
    /// alive and current; without it they are only forgotten.
    pub fn dispose(&self, release_resources: bool) {
        if release_resources {
            self.flush();
        } else {
            // Ganesh can flush internally while recording. Losing its context
            // cannot prove that any earlier read completed.
            self.frame_completion.borrow().mark_indeterminate();
        }
        if let Some(mut context) = self.context.borrow_mut().take() {
            if release_resources {
                context.release_resources_and_abandon();
            } else {
                context.abandon();
            }
        }
    }

    /// Whether the context has been disposed.
    pub fn is_disposed(&self) -> bool {
        self.context.borrow().is_none()
    }
}

impl ISkiaGrContext for GaneshGrContext {
    fn frame_context_id(&self) -> Option<u64> {
        Some(self.frame_context_id)
    }
    fn retain_frame(&self, frame: FrameLease) -> Result<(), FrameError> {
        if self.frame_completion_handler.borrow().is_none() {
            return Err(FrameError::MissingSynchronization);
        }
        if self.is_disposed() {
            return Err(FrameError::StaleGeneration);
        }
        self.frame_completion.borrow().retain(frame)
    }
    fn backend(&self) -> SkiaGpuBackend {
        SkiaGpuBackend::Ganesh
    }

    fn create_surface(
        &self,
        image_info: &ImageInfo,
        surface_props: &SurfaceProps,
    ) -> Option<Surface> {
        self.with_context(|context| {
            gpu::surfaces::render_target(
                context,
                Budgeted::No,
                image_info,
                None,
                SurfaceOrigin::BottomLeft,
                Some(surface_props),
                false,
                None,
            )
        })
    }

    fn max_render_target_size(&self) -> Option<i32> {
        Some(self.with_context(|context| context.max_render_target_size()))
    }

    fn flush(&self) {
        if let Some(context) = self.context.borrow_mut().as_mut() {
            context.flush_and_submit();
        }
        self.finish_frame_submission();
    }

    fn reset_context(&self) {
        if let Some(context) = self.context.borrow_mut().as_mut() {
            context.reset(None);
        }
    }

    fn set_resource_cache_limit(&self, max_resource_bytes: i64) {
        if let Some(context) = self.context.borrow_mut().as_mut() {
            context.set_resource_cache_limit(max_resource_bytes.max(0) as usize);
        }
    }

    fn snapshot_to_raster(&self, surface: &mut Surface) -> Option<Image> {
        self.with_context(|context| {
            surface
                .image_snapshot()
                .make_raster_image(context, CachingHint::Disallow)
        })
    }

    fn is_lost(&self) -> bool {
        self.context
            .borrow_mut()
            .as_mut()
            .is_none_or(|context| context.abandoned())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
