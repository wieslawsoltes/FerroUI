use super::JsObject;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// Creates the default native control: an element of the page (a
    /// `div`). `parent` is not used by the page.
    #[wasm_bindgen(js_namespace = NativeControlHost, js_name = createDefaultChild)]
    pub fn create_default_child(parent: Option<&JsObject>) -> JsObject;

    /// Creates the object that attaches a native control to the element of
    /// a top-level.
    #[wasm_bindgen(js_namespace = NativeControlHost, js_name = createAttachment)]
    pub fn create_attachment() -> JsObject;

    /// Makes `child` the native control of the attachment `element` and
    /// positions it absolutely.
    #[wasm_bindgen(js_namespace = NativeControlHost, js_name = initializeWithChildHandle)]
    pub fn initialize_with_child_handle(element: &JsObject, child: &JsObject);

    /// Moves the native control of the attachment into the element `host`,
    /// or removes it from its element when `host` is `None`.
    #[wasm_bindgen(js_namespace = NativeControlHost, js_name = attachTo)]
    pub fn attach_to(element: &JsObject, host: Option<&JsObject>);

    /// Shows the native control of the attachment in the given bounds, in
    /// CSS pixels of the view.
    #[wasm_bindgen(js_namespace = NativeControlHost, js_name = showInBounds)]
    pub fn show_in_bounds(element: &JsObject, x: f64, y: f64, width: f64, height: f64);

    /// Hides the native control of the attachment, keeping it at the given
    /// size.
    #[wasm_bindgen(js_namespace = NativeControlHost, js_name = hideWithSize)]
    pub fn hide_with_size(element: &JsObject, width: f64, height: f64);

    /// Forgets the native control of the attachment.
    #[wasm_bindgen(js_namespace = NativeControlHost, js_name = releaseChild)]
    pub fn release_child(element: &JsObject);
}
