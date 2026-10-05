use super::promise_helper::{JsError, JsTask};
use wasm_bindgen::prelude::*;

/// The name the main script module is imported by, relative to the script
/// of the WebAssembly module: the site places both files side by side.
pub const MAIN_MODULE_NAME: &str = "./ferroui.js";

/// The name the storage bundle is imported by, relative to the main script
/// module.
pub const STORAGE_MODULE_NAME: &str = "./storage.js";

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// Whether the page runs on a mobile device.
    #[wasm_bindgen(js_namespace = Caniuse, js_name = isMobile)]
    pub fn is_mobile() -> bool;

    /// Whether the page runs on a television.
    #[wasm_bindgen(js_namespace = Caniuse, js_name = isTv)]
    pub fn is_tv() -> bool;

    #[wasm_bindgen(js_name = importStorage)]
    fn import_storage_raw() -> super::JsObject;
}

/// Imports the storage bundle unless it is already imported. The functions
/// of [`storage_helper`](super::storage_helper) that live in that bundle
/// may only be called once this has completed.
pub async fn import_storage() -> Result<(), JsError> {
    JsTask::new(import_storage_raw()).await.map(|_| ())
}
