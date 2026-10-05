use super::IGlPlatformSurfaceRenderTarget;
use crate::IGlContext;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use std::any::TypeId;
use std::rc::Rc;

/// A surface that is rendered to with OpenGL.
///
/// A render surface announces that it is an OpenGL surface through
/// `try_get_surface_kind(TypeId::of::<dyn IGlPlatformSurface>())`, which
/// returns an `Rc<dyn IGlPlatformSurface>`; see [`try_get_gl_surface`].
pub trait IGlPlatformSurface: IPlatformRenderSurface {
    /// Creates the OpenGL render target of the surface for `context`.
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget>;
}

/// The render surface viewed as an OpenGL surface, when it is one.
pub fn try_get_gl_surface(surface: &dyn IPlatformRenderSurface) -> Option<Rc<dyn IGlPlatformSurface>> {
    surface
        .try_get_surface_kind(TypeId::of::<dyn IGlPlatformSurface>())?
        .downcast_ref::<Rc<dyn IGlPlatformSurface>>()
        .cloned()
}
