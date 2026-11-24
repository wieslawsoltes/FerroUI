use super::ISkiaGrContext;
use ferroui_base::platform::IPartialPresentationFeature;
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

    /// The partial presentation of the frame, when the render target presents partially
    /// (upstream `ISkiaGpuRenderSessionWithPartialPresentation`, 070c3be990): what the
    /// drawing context answers for `IPartialPresentationFeature`. `None` by default.
    fn partial_presentation(&self) -> Option<Rc<dyn IPartialPresentationFeature>> {
        None
    }

    /// Ends the session: flushes the drawing and presents the frame.
    fn dispose(&self);
}
