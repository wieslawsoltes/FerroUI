use crate::gpu::{ISkiaGrContext, SkiaGpuBackend};
use skia_safe::gpu::{self, Budgeted, DirectContext, SurfaceOrigin};
use skia_safe::image::CachingHint;
use skia_safe::{Image, ImageInfo, Surface, SurfaceProps};
use std::any::Any;
use std::cell::RefCell;

/// A Ganesh direct context.
///
/// Ganesh keeps a shadow copy of the state of the graphics API, so code that
/// uses the API directly has to call
/// [`reset_context`](ISkiaGrContext::reset_context) before Skia draws again.
pub struct GaneshGrContext {
    context: RefCell<Option<DirectContext>>,
}

impl GaneshGrContext {
    /// Wraps a direct context.
    pub fn new(context: DirectContext) -> Self {
        Self { context: RefCell::new(Some(context)) }
    }

    /// Runs `f` with the direct context, for creating surfaces that wrap
    /// platform framebuffers.
    ///
    /// # Panics
    /// Panics when the context has been disposed.
    pub fn with_context<R>(&self, f: impl FnOnce(&mut DirectContext) -> R) -> R {
        let mut context = self.context.borrow_mut();
        f(context.as_mut().expect("the Ganesh context has been disposed"))
    }

    /// Abandons and releases the context. With `release_resources` the GPU
    /// resources are freed first, which needs the platform context to be
    /// alive and current; without it they are only forgotten.
    pub fn dispose(&self, release_resources: bool) {
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
    fn backend(&self) -> SkiaGpuBackend {
        SkiaGpuBackend::Ganesh
    }

    fn create_surface(&self, image_info: &ImageInfo, surface_props: &SurfaceProps) -> Option<Surface> {
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
        self.with_context(|context| surface.image_snapshot().make_raster_image(context, CachingHint::Disallow))
    }

    fn is_lost(&self) -> bool {
        self.context.borrow_mut().as_mut().is_some_and(|context| context.abandoned())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
