use crate::surfaces::IGlPlatformSurfaceRenderTarget;
use crate::{GlInterface, GlVersion, OpenGlError};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;
use std::sync::Arc;

/// An OpenGL context.
///
/// A graphics context that is an OpenGL context announces it through its
/// optional features: `try_get_feature(TypeId::of::<dyn IGlContext>())`
/// returns an `Rc<dyn IGlContext>`.
pub trait IGlContext: IPlatformGraphicsContext {
    /// Borrowed EGLDisplay, valid only while this context is current on its
    /// owning thread. Native media importers must not store it across jobs.
    fn native_egl_display(&self) -> Option<isize> {
        None
    }
    fn shared_frame_retirement_queue(&self) -> Option<crate::SharedGlTextureRetirementQueue> {
        None
    }
    /// Available only when direct immutable frame sampling and asynchronous
    /// GPU completion are implemented for this context/profile.
    fn shared_frame_context_id(&self) -> Option<u64> {
        None
    }
    fn shared_frame_context_generation(&self) -> u64 {
        1
    }
    /// Starts a new compositor binding on a platform-owned shared context.
    /// Existing submitted work still retires on that native context, while
    /// queued imports from its previous binding become stale.
    fn begin_shared_frame_generation(&self) {}
    /// Retires a compositor binding on a platform-owned shared context. The
    /// platform may replace it on the next acquisition; existing native work
    /// must remain pollable until its consumers and resource owners retire.
    fn retire_shared_frame_generation(&self) {}
    fn schedule_frame_completion(
        &self,
        completion: ferroui_base::rendering::composition::frames::GpuCompletion,
    ) {
        completion.mark_indeterminate();
    }
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

    /// [`make_current`](Self::make_current) that fails, instead of
    /// panicking, when the context is lost or cannot be made current: for
    /// the callers that do without a broken context. The default asks
    /// `make_current`; the contexts whose making current can fail override
    /// it.
    fn try_make_current(&self) -> Result<Rc<dyn IDisposable>, OpenGlError> {
        Ok(self.make_current())
    }

    /// [`ensure_current`](IPlatformGraphicsContext::ensure_current) that
    /// fails, instead of panicking, when the context is lost or cannot be
    /// made current. The default asks `ensure_current`; the contexts whose
    /// making current can fail override it.
    fn try_ensure_current(&self) -> Result<Rc<dyn IDisposable>, OpenGlError> {
        Ok(self.ensure_current())
    }

    /// Whether the two contexts share their objects.
    fn is_shared_with(&self, context: &dyn IGlContext) -> bool;

    /// Whether [`create_shared_context`](Self::create_shared_context) is
    /// supported.
    fn can_create_shared_context(&self) -> bool;

    /// Creates a context that shares its objects with this one.
    fn create_shared_context(
        &self,
        preferred_versions: Option<&[GlVersion]>,
    ) -> Option<Rc<dyn IGlContext>>;
}

/// Lets a context render to surfaces that are not OpenGL surfaces
/// themselves.
///
/// A context announces the factory through its optional features:
/// `try_get_feature(TypeId::of::<dyn IGlPlatformSurfaceRenderTargetFactory>())`
/// returns an `Rc<dyn IGlPlatformSurfaceRenderTargetFactory>`.
pub trait IGlPlatformSurfaceRenderTargetFactory {
    /// Whether the context can render to the surface.
    fn can_render_to_surface(
        &self,
        context: &Rc<dyn IGlContext>,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> bool;

    /// Creates the render target of the surface.
    fn create_render_target(
        &self,
        context: &Rc<dyn IGlContext>,
        surface: &Arc<dyn IPlatformRenderSurface>,
    ) -> Rc<dyn IGlPlatformSurfaceRenderTarget>;
}
