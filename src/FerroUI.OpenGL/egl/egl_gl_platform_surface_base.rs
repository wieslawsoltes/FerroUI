use super::egl_consts::EGL_CORE_NATIVE_ENGINE;
use super::egl_partial_presentation::{EglPartialPresentation, EglPartialSession};
use super::{EglContext, EglDisplay, EglSurface, MakeCurrentError};
use crate::gl_consts::{GL_BACK, GL_FRAMEBUFFER};
use crate::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderingSession};
use crate::{GlProfileType, IGlContext};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::{
    IPartialPresentationFeature, IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetError,
    RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{PixelSize, RenderTargetCorruptedError};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

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
    /// `UsePartialPresentation` of the render target (upstream 070c3be990): the frames are
    /// presented with the region that changed, where the display has buffer age.
    use_partial_presentation: bool,
    partial_presentation: RefCell<Option<Rc<EglPartialPresentation>>>,
    /// The surface the partial presentation knows: EGL can give the handle of a destroyed
    /// surface to a new one, so the surface is compared, not its handle.
    partial_presentation_surface: RefCell<Weak<EglSurface>>,
    /// Set when a declared region or a swap failed: the render target is made again.
    partial_presentation_corrupted: Rc<Cell<bool>>,
    /// `OnSwapBuffersFailed` of the render target: a swap of a partial frame failed.
    on_swap_buffers_failed: Option<Rc<dyn Fn()>>,
}

impl EglPlatformSurfaceRenderTargetBase {
    pub fn new(context: Rc<EglContext>) -> Self {
        Self {
            context,
            use_partial_presentation: false,
            partial_presentation: RefCell::new(None),
            partial_presentation_surface: RefCell::new(Weak::new()),
            partial_presentation_corrupted: Rc::new(Cell::new(false)),
            on_swap_buffers_failed: None,
        }
    }

    /// The base of a render target that presents partially when `enabled` and the display
    /// has buffer age (the `UsePartialPresentation` override of upstream).
    pub fn with_partial_presentation(mut self, enabled: bool) -> Self {
        self.use_partial_presentation = enabled;
        self
    }

    /// Calls `on_swap_buffers_failed` when the swap of a partial frame fails (the
    /// `OnSwapBuffersFailed` override of upstream: a Wayland surface drops the frame callback
    /// it staged).
    pub fn with_on_swap_buffers_failed(mut self, on_swap_buffers_failed: Rc<dyn Fn()>) -> Self {
        self.on_swap_buffers_failed = Some(on_swap_buffers_failed);
        self
    }

    /// Whether the frames are presented partially: asked for, and the display has buffer age.
    pub fn supports_partial_presentation(&self) -> bool {
        self.use_partial_presentation && self.context.display().partial_presentation_support().buffer_age
    }

    /// The properties partial presentation gives the render target: direct rendering needs a
    /// stencil buffer (the clip of several rectangles) and a driver that gives back buffers
    /// with contents. A render target adds its own (the rotation of a DRM output).
    pub fn properties(&self) -> RenderTargetProperties {
        let supports = self.supports_partial_presentation();
        RenderTargetProperties {
            supports_partial_presentation: supports,
            is_suitable_for_direct_rendering: supports
                && self.context.stencil_size() >= 8
                && !self.partial_presentation.borrow().as_ref().is_some_and(|partial| partial.prefer_retained_layer()),
            ..RenderTargetProperties::default()
        }
    }

    /// Whether a declared region or a swap of a partial frame failed.
    pub fn is_partial_presentation_corrupted(&self) -> bool {
        self.partial_presentation_corrupted.get()
    }

    /// Forgets what the partial presentation knows of the buffers: what the disposal of a
    /// render target calls.
    pub fn invalidate_partial_presentation(&self) {
        if let Some(partial) = self.partial_presentation.borrow().as_ref() {
            partial.invalidate();
        }
        *self.partial_presentation_surface.borrow_mut() = Weak::new();
    }

    /// The partial presentation of a frame on `surface`; `None` when the target does not
    /// present partially or the surface cannot be queried (the frame is then presented whole).
    fn begin_partial_frame(
        &self,
        surface: &Rc<EglSurface>,
        size: PixelSize,
        is_y_flipped: bool,
    ) -> Option<Rc<EglPartialSession>> {
        if !self.supports_partial_presentation() {
            return None;
        }
        let display = self.context.display();
        let partial = self
            .partial_presentation
            .borrow_mut()
            .get_or_insert_with(|| {
                EglPartialPresentation::new(
                    display.egl_interface().clone(),
                    display.handle(),
                    display.partial_presentation_support(),
                    self.partial_presentation_corrupted.clone(),
                )
            })
            .clone();
        let known =
            self.partial_presentation_surface.borrow().upgrade().is_some_and(|known| Rc::ptr_eq(&known, surface));
        if !known {
            partial.invalidate();
            *self.partial_presentation_surface.borrow_mut() = Rc::downgrade(surface);
        }
        match partial.begin_draw(surface.dangerous_get_handle(), size, is_y_flipped) {
            Ok(session) => Some(session),
            Err(error) => {
                partial.invalidate();
                if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::PLATFORM) {
                    log.log(None, &format!("Partial presentation is not used for the frame: {}", error.message()));
                }
                None
            }
        }
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

        let partial = self.begin_partial_frame(surface, size, is_y_flipped);
        Ok(Rc::new(Session {
            partial,
            on_swap_buffers_failed: self.on_swap_buffers_failed.clone(),
            disposed: Cell::new(false),
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

    /// `BeginDraw(sceneInfo)` with the exception of a lost context as an error: what
    /// `IGlPlatformSurfaceRenderTarget::try_begin_draw` of a render target forwards to.
    fn try_begin_draw(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        if IPlatformGraphicsContext::is_lost(&**self.base().context()) {
            return Err(RenderTargetCorruptedError::new().into());
        }

        self.try_begin_draw_core(scene_info)
    }

    /// Begins a frame, the context not being lost, or says that the target is not ready
    /// or is corrupted. The default begins the frame with
    /// [`begin_draw_core`](Self::begin_draw_core).
    fn try_begin_draw_core(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        Ok(self.begin_draw_core(scene_info))
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

    /// Whether the render target can no longer be drawn to: its context is lost, or a frame
    /// it presented partially failed.
    fn is_corrupted(&self) -> bool {
        IPlatformGraphicsContext::is_lost(&**self.base().context()) || self.base().is_partial_presentation_corrupted()
    }
}

struct Session {
    partial: Option<Rc<EglPartialSession>>,
    on_swap_buffers_failed: Option<Rc<dyn Fn()>>,
    disposed: Cell<bool>,
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

    fn partial_presentation(&self) -> Option<Rc<dyn IPartialPresentationFeature>> {
        let partial = self.partial.as_ref()?;
        partial.take();
        Some(partial.clone())
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let presented = self.present();
        if let Some(partial) = &self.partial {
            // A session that did not present ends without presenting.
            partial.cancel();
        }
        self.restore_context.dispose();
        if presented {
            if let Some(on_finish) = &self.on_finish {
                on_finish();
            }
        }
    }
}

impl Session {
    /// Presents the frame; `false` when it was not presented: a partial frame that did not
    /// succeed (the renderer took the feature and did not finish), or whose swap failed.
    fn present(&self) -> bool {
        let partial = self.partial.as_ref().filter(|partial| partial.is_taken());
        if partial.is_some_and(|partial| !partial.can_present()) {
            return false;
        }
        self.context.gl_interface().flush();
        if !self.skip_waits {
            self.display.egl_interface().wait_gl();
        }
        if let Some(before_swap) = &self.before_swap {
            before_swap();
        }
        match (partial, &self.partial) {
            (Some(partial), _) => {
                let presented = match partial.present() {
                    Ok(presented) => presented,
                    Err(error) => {
                        if let Some(log) = Logger::try_get(LogEventLevel::Error, LogArea::PLATFORM) {
                            log.log(None, error.message());
                        }
                        false
                    }
                };
                if !presented {
                    if let Some(on_swap_buffers_failed) = &self.on_swap_buffers_failed {
                        on_swap_buffers_failed();
                    }
                    return false;
                }
            }
            // The feature was not taken: the frame is presented whole.
            (None, Some(untaken)) => untaken.present_whole(|| self.gl_surface.swap_buffers()),
            (None, None) => self.gl_surface.swap_buffers(),
        }
        if !self.skip_waits {
            self.display.egl_interface().wait_client();
            self.display.egl_interface().wait_gl();
            self.display.egl_interface().wait_native(EGL_CORE_NATIVE_ENGINE);
        }
        true
    }
}
