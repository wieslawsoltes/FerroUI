//! The accessibility host of a view (`webapp/modules/ferroui/accessibility.ts`):
//! the elements that mirror the automation tree of the view in the page.
//!
//! Not in the original (see `browser_accessibility.rs`).

use super::JsObject;
use crate::browser_accessibility::BrowserAccessibility;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// Makes `host` the accessibility host of the view in `container`, whose
    /// top-level is `top_level_id`.
    #[wasm_bindgen(js_namespace = FerroAccessibility, js_name = attach)]
    pub fn attach(host: &JsObject, container: &JsObject, top_level_id: i32);

    /// Asks for one call of [`on_frame`] at the next animation frame.
    #[wasm_bindgen(js_namespace = FerroAccessibility, js_name = requestUpdate)]
    pub fn request_update(host: &JsObject);

    /// Applies a batch of changes (JSON, see `accessibility.ts`).
    #[wasm_bindgen(js_namespace = FerroAccessibility, js_name = apply)]
    pub fn apply(host: &JsObject, batch: &str);

    /// Removes every element of the host but the enable button.
    #[wasm_bindgen(js_namespace = FerroAccessibility, js_name = clear)]
    pub fn clear(host: &JsObject);

    /// Shows or removes the visually hidden button that builds the mirror.
    #[wasm_bindgen(js_namespace = FerroAccessibility, js_name = setEnableButton)]
    pub fn set_enable_button(host: &JsObject, shown: bool);
}

/// The animation frame a view asked for: it sends what changed.
#[wasm_bindgen(js_name = AccessibilityHelper_OnFrame)]
pub fn on_frame(top_level_id: i32) {
    if let Some(tree) = BrowserAccessibility::get(top_level_id) {
        tree.on_frame();
    }
}

/// An action of the page on an element of the view: `click` (a screen
/// reader activates the element `node_id`) or `enable` (the enable button).
#[wasm_bindgen(js_name = AccessibilityHelper_OnAction)]
pub fn on_action(top_level_id: i32, node_id: u32, action: &str) {
    if let Some(tree) = BrowserAccessibility::get(top_level_id) {
        tree.on_action(node_id, action);
    }
}
