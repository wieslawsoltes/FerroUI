use super::JsObject;
use crate::browser_top_level_impl::BrowserTopLevelImpl;
use wasm_bindgen::prelude::*;

/// What the script side knows about a WebGL context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlInfo {
    pub context_id: i32,
    pub fbo_id: u32,
    pub stencils: i32,
    pub samples: i32,
    pub depth: i32,
}

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// A canvas and the render target created for it.
    pub type CanvasSurface;

    #[wasm_bindgen(method, getter, js_name = targetId)]
    pub fn target_id(this: &CanvasSurface) -> i32;

    #[wasm_bindgen(method, getter)]
    pub fn width(this: &CanvasSurface) -> f64;

    #[wasm_bindgen(method, getter)]
    pub fn height(this: &CanvasSurface) -> f64;

    #[wasm_bindgen(method, getter)]
    pub fn scaling(this: &CanvasSurface) -> f64;

    /// Creates a canvas in `container` and a render target of the first of
    /// `modes` the browser supports.
    #[wasm_bindgen(static_method_of = CanvasSurface, js_name = create)]
    pub fn create_render_target_surface(container: &JsObject, modes: &[i32], top_level_id: i32) -> CanvasSurface;

    #[wasm_bindgen(static_method_of = CanvasSurface, js_name = destroy)]
    pub fn destroy(canvas_surface: &CanvasSurface);
}

/// The canvas of a top-level changed its size or its scaling.
#[wasm_bindgen(js_name = CanvasHelper_OnSizeChanged)]
pub fn on_size_changed(top_level_id: i32, width: f64, height: f64, dpr: f64) {
    if let Some(surface) = BrowserTopLevelImpl::try_get_top_level(top_level_id).and_then(|top_level| top_level.surface())
    {
        surface.on_size_changed(width, height, dpr);
    }
}
