use super::egl_gl_platform_surface_base::RENDER_TARGET_CORRUPTED;
use super::{EglContext, MakeCurrentError};
use crate::gl_consts::{
    GL_CLAMP_TO_EDGE, GL_COLOR_ATTACHMENT0, GL_FRAMEBUFFER, GL_FRAMEBUFFER_COMPLETE, GL_LINEAR, GL_TEXTURE_2D,
    GL_TEXTURE_MAG_FILTER, GL_TEXTURE_MIN_FILTER, GL_TEXTURE_WRAP_S, GL_TEXTURE_WRAP_T,
};
use crate::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderingSession};
use crate::{GlInterface, IGlContext, OpenGlException};
use ferroui_base::platform::{IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use std::cell::Cell;
use std::ffi::c_void;
use std::rc::Rc;

/// Base class for GL platform surfaces that render to an `EGLImage` via an FBO rather than
/// to an `EGLSurface`. This is used when the compositor provides the render target as a
/// dmabuf-backed `EGLImage` (e.g. Wayland subsurface mode with linux-dmabuf).
///
/// The abstract class of the original declares nothing but the member of
/// [`IGlPlatformSurface`]; such a surface implements that contract and this marker.
pub trait EglGlPlatformImageSurfaceBase: IGlPlatformSurface {}

/// Why a frame could not be begun on an `EGLImage`.
#[derive(Clone, Debug)]
pub enum BeginImageDrawError {
    /// The context could not be made current.
    MakeCurrent(MakeCurrentError),
    /// The framebuffer of the image is incomplete.
    OpenGl(OpenGlException),
}

impl std::fmt::Display for BeginImageDrawError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MakeCurrent(error) => write!(f, "{error}"),
            Self::OpenGl(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for BeginImageDrawError {}

/// Render target base that manages an FBO backed by an `EGLImage`. Subclasses provide the
/// `EGLImage` for each frame and handle presentation.
///
/// The abstract class of the original is this state plus the members a render target
/// overrides, which are the trait [`EglPlatformImageSurfaceRenderTarget`].
pub struct EglPlatformImageSurfaceRenderTargetBase {
    context: Rc<EglContext>,
    fbo: Cell<i32>,
    texture: Cell<i32>,
    fbo_size: Cell<PixelSize>,
}

impl EglPlatformImageSurfaceRenderTargetBase {
    pub fn new(context: Rc<EglContext>) -> Self {
        Self { context, fbo: Cell::new(0), texture: Cell::new(0), fbo_size: Cell::new(PixelSize::default()) }
    }

    /// The context the render target renders with.
    pub fn context(&self) -> &Rc<EglContext> {
        &self.context
    }

    /// `Dispose()` of the base class: destroys the framebuffer and its texture.
    pub fn dispose(&self) {
        self.destroy_fbo();
    }

    /// Begins a draw session targeting the given `EGLImage`. The image is attached to an
    /// FBO as a color attachment.
    ///
    /// `on_finish_draw` is called after rendering is flushed, to present the buffer.
    ///
    /// # Safety
    /// `egl_image` must be a valid `EGLImageKHR` of the display of the context, alive
    /// until the returned session is disposed.
    pub unsafe fn begin_draw(
        &self,
        egl_image: isize,
        size: PixelSize,
        scaling: f64,
        on_finish_draw: Rc<dyn Fn()>,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, BeginImageDrawError> {
        let restore_context =
            self.context.make_current_with_surface(None).map_err(BeginImageDrawError::MakeCurrent)?;

        let gl = self.context.gl_interface();

        self.ensure_fbo(&gl, size);

        gl.bind_framebuffer(GL_FRAMEBUFFER, self.fbo.get());
        gl.bind_texture(GL_TEXTURE_2D, self.texture.get());
        // SAFETY: the caller guarantees that the image is valid for the frame; the texture
        // bound above is the target.
        unsafe { gl.egl_image_target_texture_2d_oes(GL_TEXTURE_2D, egl_image as *mut c_void) };
        gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, self.texture.get(), 0);

        let status = gl.check_framebuffer_status(GL_FRAMEBUFFER);
        if status != GL_FRAMEBUFFER_COMPLETE {
            // The `finally` of the original: the context is restored when the frame does
            // not begin.
            restore_context.dispose();
            return Err(BeginImageDrawError::OpenGl(OpenGlException::new(format!(
                "Framebuffer incomplete: 0x{status:X}"
            ))));
        }

        gl.viewport(0, 0, size.width, size.height);

        Ok(Rc::new(Session { context: self.context.clone(), size, scaling, restore_context, on_finish_draw }))
    }

    fn ensure_fbo(&self, gl: &GlInterface, size: PixelSize) {
        if self.fbo.get() != 0 && self.fbo_size.get() == size {
            return;
        }

        self.destroy_fbo_core(gl);

        self.fbo.set(gl.gen_framebuffer());
        self.texture.set(gl.gen_texture());
        self.fbo_size.set(size);

        gl.bind_texture(GL_TEXTURE_2D, self.texture.get());
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    }

    fn destroy_fbo(&self) {
        if self.fbo.get() == 0 {
            return;
        }

        // Context may already be lost
        // (The original swallows every exception here; making a disposed context current
        // fails with a panic, so a disposed context is left alone.)
        if self.context.is_disposed() {
            return;
        }
        if let Ok(current) = self.context.make_current_with_surface(None) {
            self.destroy_fbo_core(&self.context.gl_interface());
            current.dispose();
        }
    }

    fn destroy_fbo_core(&self, gl: &GlInterface) {
        if self.fbo.get() != 0 {
            gl.delete_framebuffer(self.fbo.get());
            self.fbo.set(0);
        }

        if self.texture.get() != 0 {
            gl.delete_texture(self.texture.get());
            self.texture.set(0);
        }

        self.fbo_size.set(PixelSize::default());
    }
}

/// The members a render target of an image surface overrides (the virtual and abstract
/// members of `EglPlatformImageSurfaceRenderTargetBase` in the original).
///
/// A render target implements `IGlPlatformSurfaceRenderTarget` and
/// `IPlatformRenderSurfaceRenderTarget` by forwarding to [`begin_draw`](Self::begin_draw),
/// [`dispose`](Self::dispose) and [`state`](Self::state) of this trait.
pub trait EglPlatformImageSurfaceRenderTarget {
    /// The shared state of the render target.
    fn base(&self) -> &EglPlatformImageSurfaceRenderTargetBase;

    /// Releases the render target: destroys the framebuffer by default.
    fn dispose(&self) {
        self.base().dispose();
    }

    /// Begins a frame, the context not being lost.
    fn begin_draw_core(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession>;

    /// `BeginDraw(sceneInfo)`.
    ///
    /// # Panics
    /// Panics when the context is lost (the `RenderTargetCorruptedException` of the
    /// original).
    // Seam: see `EglPlatformSurfaceRenderTarget::begin_draw`.
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
    size: PixelSize,
    scaling: f64,
    restore_context: Rc<dyn IDisposable>,
    on_finish_draw: Rc<dyn Fn()>,
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
        true
    }

    fn dispose(&self) {
        self.context.gl_interface().flush();
        self.restore_context.dispose();
        (self.on_finish_draw)();
    }
}
