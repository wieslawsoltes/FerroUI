use crate::gpu::ISkiaGrContext;
use ferroui_base::platform::IPlatformGraphicsContext;
use skia_safe::{Canvas, Surface};
use std::rc::Rc;

/// Gives direct access to the Skia objects a drawing context draws with.
///
/// Obtained from a drawing context with
/// `get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())`, which yields an
/// `Rc<dyn ISkiaApiLeaseFeature>`. Surface contexts and explicitly enabled
/// picture recordings provide it. A recording lease has no surface; see
/// [`crate::PictureRenderTarget::create_export_drawing_context`].
pub trait ISkiaApiLeaseFeature {
    /// Leases the Skia API. The drawing context cannot be used until the
    /// lease is disposed. Each lease observes the current transform and opacity,
    /// even when the feature was obtained before they changed. The feature and
    /// its leases cannot be used after the drawing context is disposed or dropped.
    fn lease(&self) -> Rc<dyn ISkiaApiLease>;
}

/// A lease of the Skia objects of a drawing context.
pub trait ISkiaApiLease {
    /// Runs `action` with the canvas of the drawing context. A picture recording
    /// supplies an isolated canvas for each call; its commands are replayed
    /// through the parent recording's clip and layers in their original order.
    fn with_sk_canvas(&self, action: &mut dyn FnMut(&Canvas));

    /// The GPU context the drawing context draws with, if any.
    ///
    /// It is accessible during a platform graphics API lease as well, since
    /// one might want to wrap native resources into Skia ones.
    fn gr_context(&self) -> Option<Rc<dyn ISkiaGrContext>>;

    /// The surface the drawing context draws to.
    fn sk_surface(&self) -> Option<Surface>;

    /// The opacity the drawing context currently multiplies its drawing
    /// with.
    fn current_opacity(&self) -> f64;

    /// Leases the platform graphics API underneath Skia, when the drawing
    /// context draws with a GPU.
    fn try_lease_platform_graphics_api(&self) -> Option<Rc<dyn ISkiaPlatformGraphicsApiLease>>;

    /// Ends the lease, restoring the transform the canvas had when the lease
    /// started.
    fn dispose(&self);
}

/// A lease of the platform graphics API underneath Skia.
pub trait ISkiaPlatformGraphicsApiLease {
    /// The platform graphics context.
    fn context(&self) -> Rc<dyn IPlatformGraphicsContext>;

    /// Ends the lease.
    fn dispose(&self);
}
