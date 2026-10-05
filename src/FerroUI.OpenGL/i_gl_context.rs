use crate::surfaces::IGlPlatformSurfaceRenderTarget;
use crate::{GlInterface, GlVersion};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// An OpenGL context.
///
/// A graphics context that is an OpenGL context announces it through its
/// optional features: `try_get_feature(TypeId::of::<dyn IGlContext>())`
/// returns an `Rc<dyn IGlContext>`.
pub trait IGlContext: IPlatformGraphicsContext {
    /// The version of the API the context implements.
    fn version(&self) -> GlVersion;

    /// The entry points of the context.
    fn gl_interface(&self) -> Rc<GlInterface>;

    /// The number of samples of the default framebuffer.
    fn sample_count(&self) -> i32;

    /// The number of stencil bits of the default framebuffer.
    fn stencil_size(&self) -> i32;

    /// Makes the context current. Disposing the result restores the
    /// previously current context.
    fn make_current(&self) -> Rc<dyn IDisposable>;

    /// Whether the two contexts share their objects.
    fn is_shared_with(&self, context: &dyn IGlContext) -> bool;

    /// Whether [`create_shared_context`](Self::create_shared_context) is
    /// supported.
    fn can_create_shared_context(&self) -> bool;

    /// Creates a context that shares its objects with this one.
    fn create_shared_context(&self, preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>>;
}

/// Lets a context render to surfaces that are not OpenGL surfaces
/// themselves.
///
/// A context announces the factory through its optional features:
/// `try_get_feature(TypeId::of::<dyn IGlPlatformSurfaceRenderTargetFactory>())`
/// returns an `Rc<dyn IGlPlatformSurfaceRenderTargetFactory>`.
pub trait IGlPlatformSurfaceRenderTargetFactory {
    /// Whether the context can render to the surface.
    fn can_render_to_surface(&self, context: &Rc<dyn IGlContext>, surface: &Rc<dyn IPlatformRenderSurface>) -> bool;

    /// Creates the render target of the surface.
    fn create_render_target(
        &self,
        context: &Rc<dyn IGlContext>,
        surface: &Rc<dyn IPlatformRenderSurface>,
    ) -> Rc<dyn IGlPlatformSurfaceRenderTarget>;
}
