use super::egl_consts::EGL_CORE_NATIVE_ENGINE;
use super::{EglContext, EglDisplay, EglSurface, MakeCurrentError};
use crate::gl_consts::{GL_BACK, GL_FRAMEBUFFER};
use crate::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderingSession};
use crate::{GlProfileType, IGlContext};
use ferroui_base::platform::{IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use std::rc::Rc;

/// A surface that is rendered to through EGL.
///
/// The abstract class of the original declares nothing but the member of
/// [`IGlPlatformSurface`]; an EGL surface implements that contract and this marker.
pub trait EglGlPlatformSurfaceBase: IGlPlatformSurface {}

/// The message of the `RenderTargetCorruptedException` a render target of a lost context
/// throws from `BeginDraw`.
pub(crate) const RENDER_TARGET_CORRUPTED: &str = "The render target is corrupted: its graphics context is lost.";

/// What the render targets of EGL surfaces share: the context, and how a frame is begun
/// and presented on a surface of the context.
///
/// The abstract class of the original is this state plus the members a render target
/// overrides, which are the trait [`EglPlatformSurfaceRenderTarget`].
pub struct EglPlatformSurfaceRenderTargetBase {
    context: Rc<EglContext>,
}

impl EglPlatformSurfaceRenderTargetBase {
    pub fn new(context: Rc<EglContext>) -> Self {
        Self { context }
    }

    /// The context the render target renders with.
    pub fn context(&self) -> &Rc<EglContext> {
        &self.context
    }

    /// Begins a frame on `surface`: makes the context current with it and returns the
    /// session that presents the frame when it is disposed.
    ///
    /// `skip_waits` is the `SkipWaits` of the render target. `on_finish` runs after the
    /// frame is presented and the context is restored; `before_swap` runs before the
    /// buffers are swapped.
    #[allow(clippy::too_many_arguments)]
    pub fn begin_draw(
        &self,
        surface: &Rc<EglSurface>,
        size: PixelSize,
        scaling: f64,
        on_finish: Option<Rc<dyn Fn()>>,
        is_y_flipped: bool,
        before_swap: Option<Rc<dyn Fn()>>,
        skip_waits: bool,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, MakeCurrentError> {
        let restore_context = self.context.make_current_with_surface(Some(surface))?;
        // Nothing below fails: the session takes over the restoring of the context.
        let egli = self.context.display().egl_interface();

        if !skip_waits {
            egli.wait_client();
            egli.wait_gl();
            egli.wait_native(EGL_CORE_NATIVE_ENGINE);
        }

        let gl = self.context.gl_interface();
        gl.bind_framebuffer(GL_FRAMEBUFFER, 0);

        // Workaround for driver quirk https://github.com/NVIDIA/egl-wayland2/issues/46
        // This is NVIDIA-specific, but setting buffers to GL_BACK won't hurt for other drivers too
        if self.context.version().type_() == GlProfileType::OpenGL {
            gl.viewport(0, 0, size.width, size.height);
            if gl.is_read_buffer_available() {
                gl.read_buffer(GL_BACK);
            }
            if gl.is_write_buffer_available() {
                gl.write_buffer(GL_BACK);
            }
            if gl.is_draw_buffer_available() {
                gl.draw_buffer(GL_BACK);
            }
        }

        Ok(Rc::new(Session {
            context: self.context.clone(),
            gl_surface: surface.clone(),
            display: self.context.display().clone(),
            restore_context,
            on_finish,
            before_swap,
            skip_waits,
            size,
            scaling,
            is_y_flipped,
        }))
    }
}

/// The members a render target of an EGL surface overrides (the virtual and abstract
/// members of `EglPlatformSurfaceRenderTargetBase` in the original).
///
/// A render target implements `IGlPlatformSurfaceRenderTarget` and
/// `IPlatformRenderSurfaceRenderTarget` by forwarding to [`begin_draw`](Self::begin_draw),
/// [`dispose`](Self::dispose) and [`state`](Self::state) of this trait.
pub trait EglPlatformSurfaceRenderTarget {
    /// The shared state of the render target.
    fn base(&self) -> &EglPlatformSurfaceRenderTargetBase;

    /// Releases the render target. Does nothing by default.
    fn dispose(&self) {}

    /// Whether the waits for the client APIs and the native engine around a frame are
    /// left out.
    fn skip_waits(&self) -> bool {
        false
    }

    /// Begins a frame, the context not being lost.
    fn begin_draw_core(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession>;

    /// `BeginDraw(sceneInfo)`.
    ///
    /// # Panics
    /// Panics when the context is lost (the `RenderTargetCorruptedException` of the
    /// original).
    // Seam: `IGlPlatformSurfaceRenderTarget::begin_draw` returns a session, it cannot report
    // the `RenderTargetCorruptedException` the original throws here. The render loop asks
    // `state()` before it draws, which says the target is corrupted while the context is
    // lost.
    fn begin_draw(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        if IPlatformGraphicsContext::is_lost(&**self.base().context()) {
            panic!("{RENDER_TARGET_CORRUPTED}");
        }

        self.begin_draw_core(scene_info)
    }

    /// The state of the render target: corrupted when [`is_corrupted`](Self::is_corrupted),
    /// ready otherwise.
    fn state(&self) -> PlatformRenderTargetState {
        if self.is_corrupted() {
            PlatformRenderTargetState::CORRUPTED
        } else {
            PlatformRenderTargetState::READY
        }
    }

    /// Whether the render target can no longer be drawn to: its context is lost.
    fn is_corrupted(&self) -> bool {
        IPlatformGraphicsContext::is_lost(&**self.base().context())
    }
}

struct Session {
    context: Rc<EglContext>,
    gl_surface: Rc<EglSurface>,
    display: Rc<EglDisplay>,
    restore_context: Rc<dyn IDisposable>,
    on_finish: Option<Rc<dyn Fn()>>,
    before_swap: Option<Rc<dyn Fn()>>,
    skip_waits: bool,
    size: PixelSize,
    scaling: f64,
    is_y_flipped: bool,
}

impl IGlPlatformSurfaceRenderingSession for Session {
    fn context(&self) -> Rc<dyn IGlContext> {
        self.context.clone()
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn scaling(&self) -> f64 {
        self.scaling
    }

    fn is_y_flipped(&self) -> bool {
        self.is_y_flipped
    }

    fn dispose(&self) {
        self.context.gl_interface().flush();
        if !self.skip_waits {
            self.display.egl_interface().wait_gl();
        }
        if let Some(before_swap) = &self.before_swap {
            before_swap();
        }
        self.gl_surface.swap_buffers();
        if !self.skip_waits {
            self.display.egl_interface().wait_client();
            self.display.egl_interface().wait_gl();
            self.display.egl_interface().wait_native(EGL_CORE_NATIVE_ENGINE);
        }
        self.restore_context.dispose();
        if let Some(on_finish) = &self.on_finish {
            on_finish();
        }
    }
}
