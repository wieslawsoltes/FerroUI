use crate::browser_system_navigation_manager::BrowserSystemNavigationManagerImpl;
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
    #[wasm_bindgen(js_namespace = NavigationHelper, js_name = addBackHandler)]
    pub fn add_back_handler();

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
