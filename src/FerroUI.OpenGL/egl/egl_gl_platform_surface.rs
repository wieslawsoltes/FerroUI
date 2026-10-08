use super::egl_gl_platform_surface_base::{
    EglGlPlatformSurfaceBase, EglPlatformSurfaceRenderTarget, EglPlatformSurfaceRenderTargetBase,
};
use super::{EglContext, EglSurface};
use crate::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use crate::IGlContext;
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The window an [`EglGlPlatformSurface`] renders to.
pub trait IEglWindowGlPlatformSurfaceInfo {
    /// The native handle of the window.
    fn handle(&self) -> isize;

    /// The size of the window in device pixels.
    fn size(&self) -> PixelSize;

    /// The scaling from logical units to device pixels.
    fn scaling(&self) -> f64;

    /// `this as IEglWindowGlPlatformSurfaceInfoWithWaitPolicy`: the wait policy of the
    /// window, when it states one.
    fn as_info_with_wait_policy(&self) -> Option<&dyn IEglWindowGlPlatformSurfaceInfoWithWaitPolicy> {
        None
    }
}

/// A window that states whether the waits around a frame are left out.
pub trait IEglWindowGlPlatformSurfaceInfoWithWaitPolicy: IEglWindowGlPlatformSurfaceInfo {
    fn skip_waits(&self) -> bool;
}

/// A window that is rendered to through an EGL window surface.
pub struct EglGlPlatformSurface {
    this: Weak<EglGlPlatformSurface>,
    info: Rc<dyn IEglWindowGlPlatformSurfaceInfo>,
}

impl EglGlPlatformSurface {
    pub fn new(info: Rc<dyn IEglWindowGlPlatformSurfaceInfo>) -> Rc<EglGlPlatformSurface> {
        Rc::new_cyclic(|this| Self { this: this.clone(), info })
    }
}

impl IPlatformRenderSurface for EglGlPlatformSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() {
            let this: Rc<dyn IGlPlatformSurface> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EglGlPlatformSurfaceBase for EglGlPlatformSurface {}

impl IGlPlatformSurface for EglGlPlatformSurface {
    /// # Panics
    /// Panics when `context` is not an EGL context (the failing cast of the original) and
    /// when the window surface cannot be created (the exception of the original).
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        let Some(egl_context) = context.as_any().downcast_ref::<EglContext>() else {
            panic!("Unable to cast the context to type 'EglContext'.");
        };
        let egl_context = egl_context.this();
        let gl_surface = match egl_context.display().create_window_surface(self.info.handle()) {
            Ok(gl_surface) => gl_surface,
            Err(error) => panic!("{error}"),
        };
        Rc::new(RenderTarget::new(gl_surface, egl_context, self.info.clone()))
    }
}

struct RenderTarget {
    base: EglPlatformSurfaceRenderTargetBase,
    gl_surface: RefCell<Option<Rc<EglSurface>>>,
    info: Rc<dyn IEglWindowGlPlatformSurfaceInfo>,
    current_size: Cell<PixelSize>,
    handle: Cell<isize>,
    skip_waits: bool,
}

impl RenderTarget {
    fn new(gl_surface: Rc<EglSurface>, context: Rc<EglContext>, info: Rc<dyn IEglWindowGlPlatformSurfaceInfo>) -> Self {
        let current_size = info.size();
        let handle = info.handle();
        let skip_waits = info.as_info_with_wait_policy().is_some_and(|info| info.skip_waits());
        Self {
            base: EglPlatformSurfaceRenderTargetBase::new(context),
            gl_surface: RefCell::new(Some(gl_surface)),
            info,
            current_size: Cell::new(current_size),
            handle: Cell::new(handle),
            skip_waits,
        }
    }
}

impl EglPlatformSurfaceRenderTarget for RenderTarget {
    fn base(&self) -> &EglPlatformSurfaceRenderTargetBase {
        &self.base
    }

    fn skip_waits(&self) -> bool {
        self.skip_waits
    }

    fn dispose(&self) {
        let gl_surface = self.gl_surface.borrow().clone();
        if let Some(gl_surface) = gl_surface {
            gl_surface.dispose();
        }
    }

    /// # Panics
    /// Panics when the window surface cannot be created or the context cannot be made
    /// current with it (the exceptions of the original).
    fn begin_draw_core(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        // TODO: use expectedPixelSize
        let handle = self.info.handle();
        let size = self.info.size();
        let existing = self.gl_surface.borrow().clone();
        let gl_surface = match existing {
            Some(gl_surface) if size == self.current_size.get() && self.handle.get() == handle => gl_surface,
            existing => {
                if let Some(old) = existing {
                    old.dispose();
                }
                *self.gl_surface.borrow_mut() = None;
                let gl_surface = match self.base.context().display().create_window_surface(handle) {
                    Ok(gl_surface) => gl_surface,
                    Err(error) => panic!("{error}"),
                };
                *self.gl_surface.borrow_mut() = Some(gl_surface.clone());
                self.current_size.set(size);
                self.handle.set(handle);
                gl_surface
            }
        };

        match self.base.begin_draw(&gl_surface, size, self.info.scaling(), None, false, None, self.skip_waits) {
            Ok(session) => session,
            Err(error) => panic!("{error}"),
        }
    }
}

impl IPlatformRenderSurfaceRenderTarget for RenderTarget {
    fn state(&self) -> PlatformRenderTargetState {
        EglPlatformSurfaceRenderTarget::state(self)
    }
}

impl IGlPlatformSurfaceRenderTarget for RenderTarget {
    fn begin_draw(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        EglPlatformSurfaceRenderTarget::begin_draw(self, scene_info)
    }

    fn dispose(&self) {
        EglPlatformSurfaceRenderTarget::dispose(self)
    }
}
