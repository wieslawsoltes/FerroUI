use super::completion_helper::{await_promise, PromiseOutcome};
use super::JsObject;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// Subscribes to the changes of the screens; they are reported through
    /// `DomHelper_ScreensChanged`.
    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = subscribeOnChanged)]
    pub fn subscribe_on_changed(global_this: &JsObject);

    // Answers with a promise, which nobody waits for: the details are requested again when a
    // previous session was given the permission.
    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = checkPermissions)]
    fn check_permissions_raw(global_this: &JsObject) -> JsObject;

    /// The screen objects of the page: the screen of the window, or every
    /// screen once the details have been granted.
    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = getAllScreens)]
    pub fn get_all_screens(global_this: &JsObject) -> Vec<JsObject>;

    // Answers with a promise of whether the details of all screens are available.
    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = requestDetailedScreens)]
    fn request_detailed_screens_raw(global_this: &JsObject) -> JsObject;

    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = getDisplayName)]
    pub fn get_display_name(screen: &JsObject) -> Option<String>;

    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = getScaling)]
    pub fn get_scaling(screen: &JsObject) -> f64;

    /// The bounds of a screen: x, y, width, height.
    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = getBounds)]
    pub fn get_bounds(screen: &JsObject) -> Vec<f64>;

    /// The working area of a screen: x, y, width, height.
    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = getWorkingArea)]
    pub fn get_working_area(screen: &JsObject) -> Vec<f64>;

    /// Whether the window is on the screen.
    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = isCurrent)]
    pub fn is_current(screen: &JsObject) -> bool;

    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = isPrimary)]
    pub fn is_primary(screen: &JsObject) -> bool;

    /// The orientation of a screen as the value of `ScreenOrientation`.
    #[wasm_bindgen(js_namespace = ScreenHelper, js_name = getCurrentOrientation)]
    pub fn get_current_orientation(screen: &JsObject) -> i32;
}

/// Requests the details of the screens if a previous session was given the
/// permission. Nothing waits for the answer.
pub fn check_permissions(global_this: &JsObject) {
    drop(check_permissions_raw(global_this));
}

/// Asks for the permission to read the details of all screens; resolves to
/// whether they are available.
pub async fn request_detailed_screens(global_this: &JsObject) -> PromiseOutcome<bool> {
    let result = await_promise(&request_detailed_screens_raw(global_this)).await?;
    Ok(result.as_bool().unwrap_or(false))
}
