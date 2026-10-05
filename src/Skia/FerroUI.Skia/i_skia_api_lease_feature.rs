use crate::gpu::ISkiaGrContext;
use ferroui_base::platform::IPlatformGraphicsContext;
use skia_safe::{Canvas, Surface};
use std::rc::Rc;

/// Gives direct access to the Skia objects a drawing context draws with.
///
/// Obtained from a drawing context with
/// `get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())`, which yields an
/// `Rc<dyn ISkiaApiLeaseFeature>`. Only drawing contexts that draw to a
/// surface provide it.
pub trait ISkiaApiLeaseFeature {
    /// Leases the Skia API. The drawing context cannot be used until the
    /// lease is disposed.
    fn lease(&self) -> Rc<dyn ISkiaApiLease>;
}

/// A lease of the Skia objects of a drawing context.
pub trait ISkiaApiLease {
    /// Runs `action` with the canvas of the drawing context.
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
