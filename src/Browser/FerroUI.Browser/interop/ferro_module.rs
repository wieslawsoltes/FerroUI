use super::completion_helper::{await_promise, PromiseError};
use wasm_bindgen::prelude::*;

/// The name the main script module is imported by, relative to the script
/// of the WebAssembly module: the site places both files side by side.
pub const MAIN_MODULE_NAME: &str = "./ferroui.js";

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

    // Answers with a promise, which nobody waits for: a failed registration is reported by the
    // browser as an unhandled rejection, as in the original.
    #[wasm_bindgen(js_name = registerServiceWorker)]
    fn register_service_worker_raw(path: &str, scope: Option<&str>) -> super::JsObject;
}

/// The path of the service worker script.
///
/// `serviceWorker.register` resolves the path against the document, not the
/// script module, so it is relative to the page. The worker also has to sit
/// at the root of the site: it is scoped to its own directory, and the save
/// picker polyfill looks it up with `getRegistration()`, which matches
/// against the address of the document.
///
/// A module built with threads registers the worker with `?coi=1`, which
/// makes it also add the response headers of cross-origin isolation. It is
/// the address the host page of such a site registers when its host does not
/// send those headers (`ensureCrossOriginIsolated` of `ferroui-threads.js`).
/// A scope has one service worker, and a registration with another address
/// replaces the worker: with the plain address here the page would lose its
/// isolation, and with it the shared memory of the module, at its next load.
/// With the same address the second registration changes nothing.
pub fn resolve_service_worker_path() -> &'static str {
    if cfg!(target_feature = "atomics") {
        "./ferroui-sw.js?coi=1"
    } else {
        "./ferroui-sw.js"
    }
}

/// Registers the service worker at `path` with the browser, with `scope`
/// or the default scope (the directory of the script). Nothing happens in
/// a browser without service workers.
pub fn register_service_worker(path: &str, scope: Option<&str>) {
    drop(register_service_worker_raw(path, scope));
}

/// Imports the storage bundle unless it is already imported. The functions
/// of [`storage_helper`](super::storage_helper) that live in that bundle
/// may only be called once this has completed.
pub async fn import_storage() -> Result<(), PromiseError> {
    await_promise(&import_storage_raw()).await.map(|_| ())
}
