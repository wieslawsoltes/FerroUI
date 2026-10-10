use super::egl_gl_platform_surface_base::{
    EglGlPlatformSurfaceBase, EglPlatformSurfaceRenderTarget, EglPlatformSurfaceRenderTargetBase,
};
use super::{EglContext, EglSurface};
use crate::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use crate::IGlContext;
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetError, RenderTargetSceneInfo};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// The window an [`EglGlPlatformSurface`] renders to.
///
/// The members are read by the thread that renders a frame, which is the render thread or
/// the UI thread: when the render target is created and at the beginning of every frame.
/// An implementor is therefore not the window object of the UI thread itself but what the
/// window publishes of itself (the handle, the size and the scaling behind atomics or a
/// lock).
pub trait IEglWindowGlPlatformSurfaceInfo: Send + Sync {
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
///
/// The surface is shared between the threads; all it holds is the window, so it is the
/// same surface wherever it is used. The render target it creates belongs to the thread
/// that renders, with the context.
pub struct EglGlPlatformSurface {
    info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
}

impl EglGlPlatformSurface {
    pub fn new(info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>) -> Arc<EglGlPlatformSurface> {
        Arc::new(Self { info })
    }
}

impl IPlatformRenderSurface for EglGlPlatformSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() {
            // The view is a handle of the calling thread, the surface a handle shared
            // between the threads: the view is a surface of its own over the same window,
            // which is all the state of a surface.
            let this: Rc<dyn IGlPlatformSurface> = Rc::new(Self { info: self.info.clone() });
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
    info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
    current_size: Cell<PixelSize>,
    handle: Cell<isize>,
    skip_waits: bool,
}

impl RenderTarget {
    fn new(gl_surface: Rc<EglSurface>, context: Rc<EglContext>, info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>) -> Self {
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

    fn try_begin_draw(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        EglPlatformSurfaceRenderTarget::try_begin_draw(self, scene_info)
    }

    fn dispose(&self) {
        EglPlatformSurfaceRenderTarget::dispose(self)
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use crate::surfaces::try_get_gl_surface;
    use std::sync::atomic::{AtomicIsize, Ordering};

    struct WindowInfo {
        handle: AtomicIsize,
    }

    impl IEglWindowGlPlatformSurfaceInfo for WindowInfo {
        fn handle(&self) -> isize {
            self.handle.load(Ordering::Relaxed)
        }

        fn size(&self) -> PixelSize {
            PixelSize::new(1, 1)
        }

        fn scaling(&self) -> f64 {
            1.0
        }
    }

    #[test]
    fn the_surface_is_an_open_gl_surface_on_the_thread_that_renders() {
        let info = Arc::new(WindowInfo { handle: AtomicIsize::new(1) });
        let surface: Arc<dyn IPlatformRenderSurface> = EglGlPlatformSurface::new(info.clone());

        assert!(try_get_gl_surface(&*surface).is_some());
        assert!(surface.try_get_surface_kind(TypeId::of::<dyn IPlatformRenderSurface>()).is_none());

        // The view of another thread is over the same window.
        info.handle.store(2, Ordering::Relaxed);
        let handle = std::thread::spawn(move || {
            let view = try_get_gl_surface(&*surface).expect("the surface is an OpenGL surface");
            let view = view.as_any().downcast_ref::<EglGlPlatformSurface>().expect("the view is an EGL surface");
            view.info.handle()
        })
        .join()
        .unwrap();
        assert_eq!(2, handle);
    }
}
