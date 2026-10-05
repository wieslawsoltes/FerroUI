use super::{ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaGrContext};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use skia_safe::{Canvas, Surface};
use std::rc::Rc;

/// Custom Skia GPU instance.
///
/// A graphics context that already is a Skia GPU announces it through its
/// optional features: `try_get_feature(TypeId::of::<dyn ISkiaGpu>())` returns
/// an `Rc<dyn ISkiaGpu>`.
pub trait ISkiaGpu: IPlatformGraphicsContext {
    /// The platform graphics context the GPU renders with, if any.
    fn platform_graphics_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>>;

    /// Attempts to create a custom render target from the given surfaces.
    fn try_create_render_target(&self, surfaces: &[Rc<dyn IPlatformRenderSurface>])
        -> Option<Rc<dyn ISkiaGpuRenderTarget>>;

    /// Whether a render target can be created from the given surfaces right
    /// now.
    fn is_ready_to_create_render_target(&self, surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> bool;

    /// Creates an offscreen render target surface.
    ///
    /// `session` is an optional current render session that might be affected
    /// by the surface creation.
    fn try_create_surface(
        &self,
        size: PixelSize,
        session: Option<&Rc<dyn ISkiaGpuRenderSession>>,
    ) -> Option<Rc<dyn ISkiaSurface>>;

    /// The Skia GPU context, made current for as long as the result is not
    /// disposed.
    fn try_get_gr_context(&self) -> Option<ScopedGrContext>;
}

/// A Skia surface owned by a GPU, optionally with a faster way of copying it
/// to a canvas than drawing it.
pub trait ISkiaSurface {
    /// The Skia surface.
    fn surface(&self) -> Surface;

    /// Whether [`blit`](Self::blit) is supported.
    fn can_blit(&self) -> bool;

    /// Copies the surface to the canvas.
    fn blit(&self, canvas: &Canvas);

    /// Releases the surface.
    fn dispose(&self);
}

/// A Skia GPU context together with what has to be undone when the caller is
/// done with it (typically restoring the previously current platform
/// context).
pub struct ScopedGrContext {
    value: Rc<dyn ISkiaGrContext>,
    on_dispose: Option<Rc<dyn IDisposable>>,
}

impl ScopedGrContext {
    /// Creates a scoped context. `on_dispose` is disposed with the scope.
    pub fn new(value: Rc<dyn ISkiaGrContext>, on_dispose: Option<Rc<dyn IDisposable>>) -> Self {
        Self { value, on_dispose }
    }

    /// The Skia GPU context.
    pub fn value(&self) -> &Rc<dyn ISkiaGrContext> {
        &self.value
    }

    /// Ends the scope.
    pub fn dispose(mut self) {
        if let Some(on_dispose) = self.on_dispose.take() {
            on_dispose.dispose();
        }
    }
}
