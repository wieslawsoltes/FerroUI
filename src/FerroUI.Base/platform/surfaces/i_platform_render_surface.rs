use crate::platform::PlatformRenderTargetState;
use std::any::Any;

/// A surface that a render backend can create a render target for.
///
/// Backends recognise the surface kinds they support by downcasting through
/// [`as_any`](Self::as_any) or the typed accessors.
pub trait IPlatformRenderSurface {
    /// Whether the surface is ready to be rendered to.
    fn is_ready(&self) -> bool {
        true
    }

    /// The surface as a framebuffer surface, if it is one.
    fn as_framebuffer_surface(&self) -> Option<&dyn super::IFramebufferPlatformSurface> {
        None
    }

    /// The surface viewed as a surface kind defined outside this crate: for
    /// `TypeId::of::<dyn IFoo>()` the result holds an `Rc<dyn IFoo>`.
    fn try_get_surface_kind(&self, _kind: std::any::TypeId) -> Option<std::rc::Rc<dyn Any>> {
        None
    }

    /// Lets the backend recover the concrete surface type.
    fn as_any(&self) -> &dyn Any;
}

/// A render target created for a platform render surface.
pub trait IPlatformRenderSurfaceRenderTarget {
    /// The state of the platform render target.
    fn state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::READY
    }
}
