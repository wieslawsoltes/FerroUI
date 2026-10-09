use super::web_render_target::{get_render_target, BrowserRenderTarget};
use super::BrowserSurfaceShared;
use ferroui_base::platform::surfaces::{IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface};
use ferroui_opengl::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget};
use ferroui_opengl::IGlContext;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::Arc;

/// The render surface of a canvas, as it is handed to the compositor.
///
/// Upstream hands out the render target itself (`BrowserWebGlRenderTarget`
/// or `BrowserSoftwareRenderTarget`), an object of the thread that draws.
/// A render surface of this port is shared between the thread of the user
/// interface and the thread that renders, so this object holds only what
/// the two share ([`BrowserSurfaceShared`]) and resolves the render target
/// on the thread that asks, in the table of that thread
/// ([`get_render_target`]):
///
/// - whether the surface is ready is read from the shared state, by either
///   thread;
/// - what kind of surface it is (an OpenGL surface, a framebuffer surface)
///   is the kind of the target of the calling thread. A thread that has no
///   target of the canvas (the thread of the page, when the canvas was
///   transferred to a worker) is told that the surface is of neither kind;
/// - the render targets it creates belong to the thread that asks for them.
///
/// Without a render thread the one thread of the page has the target, and
/// the surface answers as upstream's render target does.
pub struct BrowserRenderSurface {
    shared: Arc<BrowserSurfaceShared>,
}

// Not from upstream: the render surface contract requires a surface to be
// shared between threads.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<BrowserRenderSurface>();
};

impl BrowserRenderSurface {
    /// Creates the render surface of the canvas `shared` describes.
    pub fn new(shared: Arc<BrowserSurfaceShared>) -> Arc<Self> {
        Arc::new(Self { shared })
    }

    /// What the two threads share about the canvas.
    pub fn shared(&self) -> &Arc<BrowserSurfaceShared> {
        &self.shared
    }

    /// The render target of the canvas on the calling thread; `None` while
    /// no thread has reported the target, on a thread that does not have
    /// it, and once the view is disposed (the target has left the table of
    /// its thread then, and must not be wrapped a second time).
    fn target(&self) -> Option<BrowserRenderTarget> {
        if !self.shared.has_target() || self.shared.is_disposed() {
            return None;
        }
        get_render_target(self.shared.target_id())
    }
}

impl IPlatformRenderSurface for BrowserRenderSurface {
    fn is_ready(&self) -> bool {
        self.shared.has_target() && !self.shared.is_disposed()
    }

    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        match self.target() {
            Some(BrowserRenderTarget::Software(_)) => Some(self as &dyn IFramebufferPlatformSurface),
            _ => None,
        }
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() {
            if let Some(BrowserRenderTarget::WebGl(_)) = self.target() {
                // The surface kind is handed out in an `Rc` and the surface
                // lives in an `Arc`: the answer is a second surface over the
                // same shared state, which is all a surface is.
                let this: Rc<dyn IGlPlatformSurface> = Rc::new(Self { shared: self.shared.clone() });
                return Some(Rc::new(this));
            }
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for BrowserRenderSurface {
    /// # Panics
    /// Panics on a thread that has no software render target of the canvas.
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        match self.target() {
            Some(BrowserRenderTarget::Software(target)) => target.create_framebuffer_render_target(self.shared.clone()),
            _ => panic!("the thread has no software render target of the canvas"),
        }
    }
}

impl IGlPlatformSurface for BrowserRenderSurface {
    /// # Panics
    /// Panics on a thread that has no WebGL render target of the canvas.
    fn create_gl_render_target(&self, _context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        match self.target() {
            Some(BrowserRenderTarget::WebGl(target)) => target.create_gl_render_target(self.shared.clone()),
            _ => panic!("the thread has no WebGL render target of the canvas"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::canvas_helper::{RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL};
    use crate::interop::JsObject;
    use crate::rendering::web_render_target::set_script_render_targets_for_unit_tests;
    use crate::rendering::BrowserSoftwareRenderTarget;
    use ferroui_opengl::surfaces::try_get_gl_surface;

    /// A script that has a software render target under every id.
    fn software_script(_id: i32) -> Option<BrowserRenderTarget> {
        Some(BrowserRenderTarget::Software(BrowserSoftwareRenderTarget::new(JsObject::NULL)))
    }

    /// A script without render targets: the thread of the page, for a canvas
    /// it transferred.
    fn empty_script(_id: i32) -> Option<BrowserRenderTarget> {
        None
    }

    fn surface(target_id: i32) -> Arc<BrowserRenderSurface> {
        let shared = BrowserSurfaceShared::new();
        shared.set_target_id(target_id);
        BrowserRenderSurface::new(shared)
    }

    #[test]
    fn a_surface_without_a_reported_target_is_not_ready_and_of_no_kind() {
        // The script would have a target; nobody has reported one.
        set_script_render_targets_for_unit_tests(software_script);
        let surface = surface(1);

        assert!(!surface.is_ready());
        assert!(surface.as_framebuffer_surface().is_none());
        assert!(try_get_gl_surface(&*surface).is_none());
    }

    #[test]
    fn the_thread_that_has_the_target_sees_a_framebuffer_surface() {
        set_script_render_targets_for_unit_tests(software_script);
        let surface = surface(2);
        surface.shared().set_target_kind(RENDER_TARGET_KIND_SOFTWARE);

        assert!(surface.is_ready());
        assert!(surface.as_framebuffer_surface().is_some());
        assert!(try_get_gl_surface(&*surface).is_none());
        assert!(surface.try_get_surface_kind(TypeId::of::<dyn IPlatformRenderSurface>()).is_none());
    }

    #[test]
    fn a_thread_without_the_target_reads_that_it_is_ready_and_sees_no_kind() {
        set_script_render_targets_for_unit_tests(software_script);
        let surface = surface(3);
        surface.shared().set_target_kind(RENDER_TARGET_KIND_SOFTWARE);
        assert!(surface.as_framebuffer_surface().is_some());

        // The surface crosses to another thread as it is; that thread has a
        // table and a script of its own.
        let seen = std::thread::spawn({
            let surface = surface.clone();
            move || {
                set_script_render_targets_for_unit_tests(empty_script);
                (
                    surface.is_ready(),
                    surface.as_framebuffer_surface().is_some(),
                    try_get_gl_surface(&*surface).is_some(),
                )
            }
        })
        .join()
        .unwrap();

        assert_eq!((true, false, false), seen);
    }

    #[test]
    fn the_kind_is_the_one_of_the_target_of_the_thread() {
        // The shared state says WebGL; the target of this thread is a
        // software one, so the surface is not handed out as an OpenGL one.
        set_script_render_targets_for_unit_tests(software_script);
        let surface = surface(4);
        surface.shared().set_target_kind(RENDER_TARGET_KIND_WEB_GL);

        assert!(try_get_gl_surface(&*surface).is_none());
        assert!(surface.as_framebuffer_surface().is_some());
    }

    #[test]
    fn a_disposed_surface_is_not_ready() {
        let surface = surface(5);
        surface.shared().set_target_kind(RENDER_TARGET_KIND_WEB_GL);
        assert!(surface.is_ready());

        surface.shared().dispose();
        assert!(!surface.is_ready());
    }

    #[test]
    #[should_panic(expected = "the thread has no software render target of the canvas")]
    fn a_thread_without_the_target_cannot_create_a_render_target() {
        set_script_render_targets_for_unit_tests(empty_script);
        let surface = surface(6);
        surface.shared().set_target_kind(RENDER_TARGET_KIND_SOFTWARE);

        surface.create_framebuffer_render_target();
    }

    #[test]
    fn the_thread_that_has_the_target_creates_a_render_target() {
        set_script_render_targets_for_unit_tests(software_script);
        let surface = surface(7);
        surface.shared().set_target_kind(RENDER_TARGET_KIND_SOFTWARE);

        let render_target =
            surface.as_framebuffer_surface().expect("a framebuffer surface").create_framebuffer_render_target();
        // Nothing was locked: there is nothing to release, and no call into
        // the page.
        render_target.dispose();
    }
}
