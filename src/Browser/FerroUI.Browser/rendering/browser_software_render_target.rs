use super::web_render_target::{update_size, BrowserRenderTarget, CanvasSize};
use crate::interop::JsObject;
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
    IPlatformRenderSurfaceRenderTarget,
};
use ferroui_base::platform::{
    AlphaFormat, ILockedFramebuffer, IPlatformGraphicsContext, PixelFormats, RenderTargetSceneInfo,
    RetainedFramebuffer,
};
use ferroui_base::utilities::ThreadBound;
use ferroui_base::Vector;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(js_namespace = SoftwareRenderTarget, js_name = staticPutPixelData)]
    fn put_pixel_data(js: &JsObject, address: u32, size: i32, width: i32, height: i32);
}

/// A canvas that is rendered to in memory: each frame is drawn into a
/// retained framebuffer and copied to the 2D context of the canvas.
///
/// A render surface is shared between the thread of the user interface and
/// the thread that renders, so the object is `Send + Sync`. The browser runs
/// the compositor on its one thread, and everything the target holds belongs
/// to that thread (the object of the script side and the size of the
/// canvas): it is kept in a [`ThreadBound`], which only that thread can
/// open. A render worker (stage B2 of `docs/porting/render-thread.md`) is
/// where the target becomes an object that is really used across threads.
pub struct BrowserSoftwareRenderTarget {
    this: std::sync::Weak<BrowserSoftwareRenderTarget>,
    state: ThreadBound<SoftwareRenderTargetState>,
}

/// What a software render target holds, all of it bound to the thread of
/// the page.
struct SoftwareRenderTargetState {
    js: JsObject,
    size_getter: CanvasSize,
}

// Not from upstream: the render surface contract requires a surface to be
// shared between threads.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<BrowserSoftwareRenderTarget>();
};

impl BrowserSoftwareRenderTarget {
    /// Wraps the software render target of the script side.
    ///
    /// The thread that calls this is the one the target stays bound to.
    pub fn new(js: JsObject, size_getter: CanvasSize) -> Arc<Self> {
        let state = ThreadBound::new(SoftwareRenderTargetState { js, size_getter });
        Arc::new_cyclic(|this| Self { this: this.clone(), state })
    }

    fn blit(&self, fb: &RetainedFramebuffer) {
        let size = fb.size();
        put_pixel_data(&self.state.get().js, fb.address() as usize as u32, size.width * size.height * 4, size.width, size.height);
    }
}

impl BrowserRenderTarget for BrowserSoftwareRenderTarget {
    fn platform_graphics_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>> {
        None
    }

    fn as_render_surface(&self) -> Arc<dyn IPlatformRenderSurface> {
        self.this.upgrade().expect("the render target is alive")
    }
}

impl IPlatformRenderSurface for BrowserSoftwareRenderTarget {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for BrowserSoftwareRenderTarget {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        Rc::new(FramebufferRenderTarget {
            parent: self.this.upgrade().expect("the render target is alive"),
            fb: RefCell::new(None),
        })
    }
}

struct FramebufferRenderTarget {
    parent: Arc<BrowserSoftwareRenderTarget>,
    fb: RefCell<Option<Rc<RetainedFramebuffer>>>,
}

impl IPlatformRenderSurfaceRenderTarget for FramebufferRenderTarget {}

impl IFramebufferRenderTarget for FramebufferRenderTarget {
    fn lock(&self, _scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        let properties = FramebufferLockProperties::default();
        let state = self.parent.state.get();
        let (size, scaling) = (state.size_getter)();
        update_size(&state.js, size);

        let mut fb = self.fb.borrow_mut();
        if fb.as_ref().is_none_or(|fb| fb.size() != size) {
            if let Some(old) = fb.take() {
                old.dispose();
            }
            *fb = Some(RetainedFramebuffer::new(size, PixelFormats::RGBA8888, AlphaFormat::Premul));
        }

        let blit_target = Arc::downgrade(&self.parent);
        let locked = fb.as_ref().expect("the framebuffer was just created").lock(
            Vector::new(scaling * 96.0, scaling * 96.0),
            move |fb| {
                if let Some(parent) = blit_target.upgrade() {
                    parent.blit(fb);
                }
            },
        );
        (locked, properties)
    }

    fn dispose(&self) {
        if let Some(fb) = self.fb.borrow_mut().take() {
            fb.dispose();
        }
    }
}
