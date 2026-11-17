use crate::{BrowserNavigationHistory, BrowserSystemNavigationManagerImpl};
use ferroui_base::{FerroLocator, LocatorExtensions};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// Makes the back navigation of the browser a back request of the
    /// framework: an entry is pushed onto the history of the page, and going
    /// back from it calls `NavigationHelper_OnBackRequested`. When the
    /// request is not handled the page goes back.
    ///
    /// The original passes the callback as a function; here the page calls
    /// the export.
    /// Has no effect if a [`BrowserNavigationHistory`] is active.
    #[wasm_bindgen(js_namespace = NavigationHelper, js_name = addBackHandler)]
    pub fn add_back_handler();

    #[wasm_bindgen(js_namespace = NavigationHelper, js_name = startHistory, catch)]
    pub(crate) fn start_history(initial_route: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(js_namespace = NavigationHelper, js_name = pushHistory, catch)]
    pub(crate) fn push_history(route: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(js_namespace = NavigationHelper, js_name = stopHistory)]
    pub(crate) fn stop_history();

    /// Opens a URI in a browsing context (`window.open`). Returns whether a
    /// context was opened.
    #[wasm_bindgen(js_namespace = NavigationHelper, js_name = openUri)]
    pub fn window_open(uri: &str, target: &str) -> bool;
}

/// The user asked to go back. Returns whether the request was handled.
// The original answers with a task; here the answer is synchronous.
#[wasm_bindgen(js_name = NavigationHelper_OnBackRequested)]
pub fn on_back_requested() -> bool {
    FerroLocator::current()
        .get_service::<BrowserSystemNavigationManagerImpl>()
        .is_some_and(|manager| manager.on_back_requested())
}

#[wasm_bindgen(js_name = NavigationHelper_OnHistoryTraversed)]
pub fn on_history_traversed(route: &str, delta: i32) {
    BrowserNavigationHistory::on_traversed(route, delta);
}
