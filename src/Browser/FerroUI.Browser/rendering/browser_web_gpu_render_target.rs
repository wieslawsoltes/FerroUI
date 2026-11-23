use super::web_render_target::update_size;
use crate::interop::JsObject;
use ferroui_base::PixelSize;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(js_namespace = WebGpuRenderTarget, js_name = selectorOf)]
    fn selector_of(js: &JsObject) -> String;

    #[wasm_bindgen(js_namespace = WebGpuRenderTarget, js_name = prefersRgba8)]
    fn prefers_rgba8() -> bool;
}

/// A canvas of the page that a backend draws to with the WebGPU of the page. The backend makes
/// the surface from the selector: this object only keeps the size of the canvas, as setting it
/// clears what the canvas shows.
pub struct BrowserWebGpuRenderTarget {
    js: JsObject,
    selector: String,
    prefers_rgba8: bool,
    size: Cell<PixelSize>,
}

impl BrowserWebGpuRenderTarget {
    /// Wraps the WebGPU render target of the script side.
    pub fn new(js: JsObject) -> Rc<Self> {
        let selector = selector_of(&js);
        Rc::new(Self { js, selector, prefers_rgba8: prefers_rgba8(), size: Cell::new(PixelSize::default()) })
    }

    /// The CSS selector of the canvas.
    pub fn selector(&self) -> &str {
        &self.selector
    }

    /// Whether the page prefers `RGBA8Unorm` for a canvas.
    pub fn prefers_rgba8(&self) -> bool {
        self.prefers_rgba8
    }

    /// Gives the canvas `size`, when it has another one.
    pub fn set_size(&self, size: PixelSize) {
        if self.size.replace(size) != size {
            update_size(&self.js, size);
        }
    }
}
