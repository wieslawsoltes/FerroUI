use super::JsObject;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// Gives the keyboard focus of the page to the element.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = focusElement)]
    pub fn focus_element(html_element: &JsObject);

    /// Sets the CSS cursor of the element; `"default"` removes it.
    #[wasm_bindgen(js_namespace = InputHelper, js_name = setCursor)]
    pub fn set_cursor(html_element: &JsObject, kind: &str);
}
