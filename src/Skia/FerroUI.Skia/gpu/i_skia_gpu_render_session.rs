use super::ISkiaGrContext;
use skia_safe::Surface;
use std::rc::Rc;

/// Where the first row of a GPU surface is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SkiaSurfaceOrigin {
    #[default]
    TopLeft,
    BottomLeft,
}

/// Custom render session for a Skia render target.
pub trait ISkiaGpuRenderSession {
    /// The GPU context used by this session.
    fn gr_context(&self) -> Rc<dyn ISkiaGrContext>;

    /// The canvas surface that will be used to render.
    fn sk_surface(&self) -> Surface;

    /// The scaling factor.
    fn scale_factor(&self) -> f64;

    /// The origin of the surface.
    fn surface_origin(&self) -> SkiaSurfaceOrigin;

    /// Ends the session: flushes the drawing and presents the frame.
    fn dispose(&self);
}
