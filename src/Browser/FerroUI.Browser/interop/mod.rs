//! The boundary between the framework and the script module of the
//! platform.
//!
//! One file per group of functions, as the script module groups them. Each
//! holds the imports (functions of the script module, called from here) and
//! the exports (functions of the framework, called by the script module).
//! Objects of the page are opaque [`JsObject`] handles; nothing outside this
//! module and the render targets touches them except to pass them back.

pub mod canvas_helper;
pub mod completion_helper;
pub mod dom_helper;
pub mod ferro_module;
pub mod input_helper;
pub mod navigation_helper;
pub mod screen_helper;
pub mod timer_helper;

/// An object of the web page, held by the framework without looking inside.
pub type JsObject = wasm_bindgen::JsValue;

/// `value` unless it is `null` or `undefined`.
pub(crate) fn non_null(value: JsObject) -> Option<JsObject> {
    if value.is_null() || value.is_undefined() {
        None
    } else {
        Some(value)
    }
}
