//! The surface factory of a composition mode: a window asks it for the
//! surface it is rendered through.

use crate::i_blur_host::ICompositionEffectsSurface;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
use std::sync::Arc;

/// The surface a factory created for a window.
///
/// The reference returns one object and asks it what else it is (a surface
/// with blur effects, a disposable object) by casting; here the factory
/// says so.
pub(crate) struct WindowsSurface {
    /// The surface the window is rendered through.
    pub surface: Arc<dyn IPlatformRenderSurface>,
    /// The surface as a surface with blur effects, when it is one.
    pub effects: Option<Arc<dyn ICompositionEffectsSurface>>,
    /// Releases what the surface holds of the window; the window calls it
    /// when it is destroyed.
    pub dispose: Option<Arc<dyn Fn() + Send + Sync>>,
}

/// Creates the surfaces of windows: the composition mode the graphics
/// manager registered.
pub(crate) trait IWindowsSurfaceFactory {
    /// Whether a window of the factory is created without a redirection
    /// bitmap (`WS_EX_NOREDIRECTIONBITMAP`).
    fn requires_no_redirection_bitmap(&self) -> bool;

    fn create_surface(&self, info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>) -> WindowsSurface;
}
