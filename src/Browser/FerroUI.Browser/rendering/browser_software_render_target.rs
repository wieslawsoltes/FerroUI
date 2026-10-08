use super::web_render_target::update_size;
use super::{BrowserSurfaceShared, RenderStatistics};
use crate::interop::canvas_helper::RENDER_TARGET_KIND_SOFTWARE;
use crate::interop::{thread_proxy, JsObject};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, IFramebufferRenderTarget, IPlatformRenderSurfaceRenderTarget,
};
use ferroui_base::platform::{
    AlphaFormat, ILockedFramebuffer, PixelFormats, RenderTargetSceneInfo, RetainedFramebuffer,
};
use ferroui_base::Vector;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(js_namespace = SoftwareRenderTarget, js_name = staticPutPixelData)]
    fn put_pixel_data(js: &JsObject, address: u32, size: i32, width: i32, height: i32);
}

/// A canvas that is rendered to in memory, as the thread that draws to it
/// holds it: each frame is drawn into a retained framebuffer and copied to
/// the 2D context of the canvas through the object of the script side of
/// that thread.
///
/// Upstream's class is also the render surface of the canvas. Here the
/// surface is [`BrowserRenderSurface`](super::BrowserRenderSurface), which
/// both threads hold and which holds nothing of a thread; this object stays
/// in the table of the thread that renders
/// ([`get_render_target`](super::get_render_target)) and is what the surface
/// resolves there.
pub struct BrowserSoftwareRenderTarget {
    js: JsObject,
}

impl BrowserSoftwareRenderTarget {
    /// Wraps the software render target of the script side.
    ///
    /// The thread that calls this is the one the target belongs to.
    pub fn new(js: JsObject) -> Rc<Self> {
        Rc::new(Self { js })
    }

    /// Creates the framebuffer render target of the canvas. `shared` is
    /// where each frame reads the size the canvas has to have.
    pub fn create_framebuffer_render_target(
        self: &Rc<Self>,
        shared: Arc<BrowserSurfaceShared>,
    ) -> Rc<dyn IFramebufferRenderTarget> {
        Rc::new(FramebufferRenderTarget { parent: self.clone(), shared, fb: RefCell::new(None) })
    }

    fn blit(&self, fb: &RetainedFramebuffer) {
        let size = fb.size();
        put_pixel_data(&self.js, fb.address() as usize as u32, size.width * size.height * 4, size.width, size.height);
        // Not from upstream: where frames are drawn is otherwise invisible
        // to the thread of the page.
        RenderStatistics::frame_presented(
            thread_proxy::current_thread(),
            RENDER_TARGET_KIND_SOFTWARE,
            0,
            size.width,
            size.height,
        );
    }
}

struct FramebufferRenderTarget {
    parent: Rc<BrowserSoftwareRenderTarget>,
    shared: Arc<BrowserSurfaceShared>,
    fb: RefCell<Option<Rc<RetainedFramebuffer>>>,
}

impl IPlatformRenderSurfaceRenderTarget for FramebufferRenderTarget {}

impl IFramebufferRenderTarget for FramebufferRenderTarget {
    fn lock(&self, _scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        let properties = FramebufferLockProperties::default();
        // The size as the thread of the user interface last wrote it. The
        // canvas is given that size here, by the thread that draws to it:
        // for a canvas that was transferred to a worker no other can.
        let (size, scaling) = self.shared.size();
        update_size(&self.parent.js, size);

        let mut fb = self.fb.borrow_mut();
        if fb.as_ref().is_none_or(|fb| fb.size() != size) {
            if let Some(old) = fb.take() {
                old.dispose();
            }
            *fb = Some(RetainedFramebuffer::new(size, PixelFormats::RGBA8888, AlphaFormat::Premul));
        }

        let blit_target = Rc::downgrade(&self.parent);
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
