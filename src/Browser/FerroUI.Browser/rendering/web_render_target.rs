use super::{BrowserSoftwareRenderTarget, BrowserWebGlRenderTarget};
use crate::interop::{non_null, JsObject};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_base::PixelSize;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// A render target of the script side.
    pub type JsRenderTarget;

    #[wasm_bindgen(method, getter, js_name = renderTargetType)]
    fn render_target_type(this: &JsRenderTarget) -> String;

    #[wasm_bindgen(js_namespace = WebRenderTargetRegistry, js_name = getRenderTarget)]
    fn get_js_render_target(id: i32) -> JsObject;

    #[wasm_bindgen(js_namespace = WebRenderTarget, js_name = setSize)]
    fn set_js_size(target: &JsObject, w: i32, h: i32);

    #[wasm_bindgen(js_namespace = WebRenderTargetRegistry, js_name = initializeWorker)]
    fn initialize_js_worker();
}

/// Makes the worker of the calling thread take the canvases that are
/// transferred to it: installs the message handler of the registry of the
/// thread's script, which creates the render target of each canvas and
/// reports it
/// ([`on_render_target_registered`](crate::interop::canvas_helper::on_render_target_registered)).
///
/// Called once, by a thread other than the one of the page, before a canvas
/// is created with its id. The thread has to stay alive afterwards and
/// return to the event loop of its worker, where the messages arrive.
pub fn initialize_worker() {
    initialize_js_worker();
}

/// The size of the canvas in device pixels and its scaling, asked for at
/// the start of each frame.
pub type CanvasSize = Rc<dyn Fn() -> (PixelSize, f64)>;

/// What the two kinds of render targets of a canvas have in common.
pub trait BrowserRenderTarget {
    /// The graphics context the target renders with; `None` for a target
    /// that is rendered to in software.
    fn platform_graphics_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>>;

    /// The target as the render surface a render backend draws to.
    fn as_render_surface(&self) -> Arc<dyn IPlatformRenderSurface>;
}

/// The render target the script side created under `id`, when it exists.
///
/// The target is also the render surface of the canvas, which is handed out
/// in an `Arc`, so it is created in one.
///
/// # Panics
/// Panics when the target is of a kind the framework does not know.
pub fn get_render_target(id: i32, size_getter: CanvasSize) -> Option<Arc<dyn BrowserRenderTarget>> {
    let js = non_null(get_js_render_target(id))?;
    let type_ = JsCast::unchecked_ref::<JsRenderTarget>(&js).render_target_type();
    if type_ == "webgl" {
        return Some(BrowserWebGlRenderTarget::new(js, size_getter));
    }
    if type_ == "software" {
        return Some(BrowserSoftwareRenderTarget::new(js, size_getter));
    }
    panic!("{type_}");
}

/// Sets the size of the canvas behind a render target.
pub(crate) fn update_size(js: &JsObject, size: PixelSize) {
    set_js_size(js, size.width, size.height);
}
