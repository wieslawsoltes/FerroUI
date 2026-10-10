//! The EGL render surface of a surface of the worker (the port of
//! `WaylandEglWsiSurface.cs`).

use super::i_wayland_framebuffer_surface::{IWaylandSurfaceRenderTarget, WaylandRenderSurfaceTarget};
use super::wayland_framebuffer::target_state;
use crate::server::persistent::w_surface::WSurfaceId;
use crate::server::wayland_worker::with_worker_thread;
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetError, RenderTargetSceneInfo};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{PixelSize, RenderTargetNotReadyException};
use ferroui_opengl::egl::{
    EglContext, EglGlPlatformSurfaceBase, EglPlatformSurfaceRenderTarget, EglPlatformSurfaceRenderTargetBase, EglSurface,
};
use ferroui_opengl::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use ferroui_opengl::{IGlContext, OpenGlException};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use wayland_client::Proxy;
use wayland_egl::WlEglSurface;

/// EGL surface for a Wayland toplevel using the classic
/// `wl_egl_window` WSI path. The driver owns the swapchain,
/// buffer allocation and presentation. `eglSwapBuffers` is the
/// commit, which is why frame-callback / ack_configure / geometry
/// staging is delegated to `on_before_new_buffer_attached`
/// and invoked immediately before the swap.
///
/// The surface is a handle threads share: it names the worker's surface. Its
/// render target is an object of the worker thread.
#[derive(Clone)]
pub struct WaylandEglWsiSurface {
    surface: WSurfaceId,
}

impl WaylandEglWsiSurface {
    pub fn new(surface: WSurfaceId) -> Self {
        Self { surface }
    }
}

impl IPlatformRenderSurface for WaylandEglWsiSurface {
    fn is_ready(&self) -> bool {
        target_state(WaylandRenderSurfaceTarget::Surface(self.surface)).is_ready
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() {
            let this: Rc<dyn IGlPlatformSurface> = Rc::new(self.clone());
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EglGlPlatformSurfaceBase for WaylandEglWsiSurface {}

impl IGlPlatformSurface for WaylandEglWsiSurface {
    /// # Panics
    /// Panics when `context` is not an EGL context (the failing cast of the reference).
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        let Some(egl_context) = context.as_any().downcast_ref::<EglContext>() else {
            panic!("Unable to cast the context to type 'EglContext'.");
        };
        let render_target = Rc::new_cyclic(|this| RenderTarget {
            this: this.clone(),
            base: EglPlatformSurfaceRenderTargetBase::new(egl_context.this()),
            surface: self.surface,
            egl_window: RefCell::new(None),
            egl_surface: RefCell::new(None),
            current_size: Cell::new(PixelSize::default()),
            disposed: Cell::new(false),
        });
        let registered: Weak<dyn IWaylandSurfaceRenderTarget> = render_target.this.clone();
        with_worker_thread(|worker| {
            if let Some((_, surface)) = worker.state.framebuffer_surface(WaylandRenderSurfaceTarget::Surface(self.surface)) {
                surface.register_render_target(registered);
            }
        });
        render_target
    }
}

struct RenderTarget {
    this: Weak<RenderTarget>,
    base: EglPlatformSurfaceRenderTargetBase,
    surface: WSurfaceId,
    egl_window: RefCell<Option<WlEglSurface>>,
    egl_surface: RefCell<Option<Rc<EglSurface>>>,
    current_size: Cell<PixelSize>,
    disposed: Cell<bool>,
}

impl RenderTarget {
    fn surface_state(&self) -> PlatformRenderTargetState {
        target_state(WaylandRenderSurfaceTarget::Surface(self.surface))
    }

    fn ensure_window(&self, size: PixelSize) -> Result<Rc<EglSurface>, OpenGlException> {
        debug_assert!(size.width > 0 && size.height > 0);
        if let (Some(egl_surface), true) = (self.egl_surface.borrow().clone(), self.current_size.get() == size) {
            return Ok(egl_surface);
        }

        let existing = self.egl_surface.borrow().clone();
        let egl_surface = match existing {
            Some(egl_surface) => {
                if let Some(egl_window) = &*self.egl_window.borrow() {
                    egl_window.resize(size.width, size.height, 0, 0);
                }
                egl_surface
            }
            None => {
                let wl_surface = with_worker_thread(|worker| {
                    worker
                        .state
                        .framebuffer_surface(WaylandRenderSurfaceTarget::Surface(self.surface))
                        .and_then(|(_, surface)| surface.wl_surface().cloned())
                })
                .flatten()
                .ok_or_else(|| OpenGlException::new("The surface has no wl_surface"))?;
                let egl_window = WlEglSurface::new(wl_surface.id(), size.width, size.height)
                    .map_err(|_| OpenGlException::new("wl_egl_window_create failed"))?;
                // On failure the window is dropped, which destroys it.
                let egl_surface = self.base.context().display().create_window_surface(egl_window.ptr() as isize)?;
                *self.egl_window.borrow_mut() = Some(egl_window);
                *self.egl_surface.borrow_mut() = Some(egl_surface.clone());
                egl_surface
            }
        };

        self.current_size.set(size);
        Ok(egl_surface)
    }

    /// Releases the surface of EGL and the window of the driver, in that order.
    fn dispose_core(&self) {
        if let Some(egl_surface) = self.egl_surface.borrow_mut().take() {
            egl_surface.dispose();
        }
        // Dropping the window destroys it.
        self.egl_window.borrow_mut().take();
        self.current_size.set(PixelSize::default());
    }
}

impl IWaylandSurfaceRenderTarget for RenderTarget {
    fn dispose_from_surface(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.dispose_core();
    }
}

impl EglPlatformSurfaceRenderTarget for RenderTarget {
    fn base(&self) -> &EglPlatformSurfaceRenderTargetBase {
        &self.base
    }

    fn skip_waits(&self) -> bool {
        true
    }

    fn state(&self) -> PlatformRenderTargetState {
        if self.disposed.get() {
            return PlatformRenderTargetState::DISPOSED;
        }
        if self.is_corrupted() {
            return PlatformRenderTargetState::CORRUPTED;
        }
        self.surface_state()
    }

    /// # Panics
    /// Panics when the surface is not ready for a frame or the window surface cannot be made
    /// (the exceptions of the reference).
    fn begin_draw_core(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        match self.try_begin_draw_core(scene_info) {
            Ok(session) => session,
            Err(error) => panic!("{error}"),
        }
    }

    fn try_begin_draw_core(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        if self.disposed.get() || !self.surface_state().is_ready {
            return Err(RenderTargetError::NotReady(RenderTargetNotReadyException::default()));
        }

        let size = scene_info.size;
        let size = PixelSize::new(size.width.max(1), size.height.max(1));
        let egl_surface = match self.ensure_window(size) {
            Ok(egl_surface) => egl_surface,
            Err(error) => panic!("{error}"),
        };

        // on_before_new_buffer_attached must run *immediately* before the
        // implicit commit performed by eglSwapBuffers; the base class
        // invokes before_swap right before the swap call.
        //
        // SwapInterval(0) is set inside before_swap (so it runs with our
        // surface current); GTK re-asserts it every frame defensively
        // and we follow suit — some drivers reset it on resize.
        let display = self.base.context().display().clone();
        let surface = self.surface;
        let before_swap_scene_info = scene_info.clone();
        let before_swap: Rc<dyn Fn()> = Rc::new(move || {
            display.egl_interface().swap_interval(display.handle(), 0);
            with_worker_thread(|worker| {
                if let Some((globals, surface)) = worker.state.framebuffer_surface(WaylandRenderSurfaceTarget::Surface(surface)) {
                    surface.on_before_new_buffer_attached(globals, &before_swap_scene_info);
                }
            });
        });

        // The workaround of the reference for a driver quirk
        // (https://github.com/NVIDIA/egl-wayland2/issues/46: the viewport and the read and
        // draw buffers set to the back buffer for desktop OpenGL) is in `begin_draw` of the
        // base of the OpenGL crate.
        match self.base.begin_draw(&egl_surface, size, scene_info.scaling, None, false, Some(before_swap), true) {
            Ok(session) => Ok(session),
            Err(error) => panic!("{error}"),
        }
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        if let Some(this) = self.this.upgrade() {
            let this: Rc<dyn IWaylandSurfaceRenderTarget> = this;
            with_worker_thread(|worker| {
                if let Some((_, surface)) =
                    worker.state.framebuffer_surface(WaylandRenderSurfaceTarget::Surface(self.surface))
                {
                    surface.unregister_render_target(&this);
                }
            });
        }
        self.dispose_core();
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
