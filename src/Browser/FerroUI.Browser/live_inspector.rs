//! An opt-in inspector for a debug application running in a browser.

use crate::browser_top_level_impl::BrowserTopLevelImpl;
use ferroui_controls::Application;
pub use ferroui_inspect::Permissions;
use serde_json::{json, Value};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
export function registerLiveApp(elementId, dispatch, presented) {
    const root = document.getElementById(elementId);
    if (!root || Object.hasOwn(globalThis, "__ferrouiLiveApp")) return false;
    const liveApp = Object.freeze({
        version: 1,
        dispatch,
        ready() { return root.isConnected && presented(); },
        bounds() {
            if (!this.ready()) return null;
            const area = root.getBoundingClientRect();
            return {
                x: area.left + window.scrollX, y: area.top + window.scrollY,
                width: area.width, height: area.height,
            };
        },
    });
    Object.defineProperty(globalThis, "__ferrouiLiveApp", { configurable: true, value: liveApp });
    return true;
}

export function unregisterLiveApp(dispatch) {
    if (globalThis.__ferrouiLiveApp?.dispatch === dispatch) delete globalThis.__ferrouiLiveApp;
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = registerLiveApp)]
    fn register(element_id: &str, dispatch: &JsValue, presented: &JsValue) -> bool;
    #[wasm_bindgen(js_name = unregisterLiveApp)]
    fn unregister(dispatch: &JsValue);
}

/// Keep this handle alive for the lifetime of the inspected view.
pub struct LiveInspector {
    dispatch: Closure<dyn Fn(String) -> String>,
    _presented: Closure<dyn Fn() -> bool>,
}

impl Drop for LiveInspector {
    fn drop(&mut self) {
        unregister(self.dispatch.as_ref());
    }
}

/// Expose the live inspection bridge on the page that owns the view. Call on the UI thread
/// after platform setup. The host element supplies the bounds of the rendered browser frame.
pub fn install(element_id: &str, permissions: Permissions) -> Result<LiveInspector, &'static str> {
    if !cfg!(debug_assertions) {
        return Err("Live App requires a debug build");
    }
    ferroui_base::threading::Dispatcher::ui_thread().verify_access();
    let surface = Application::current()
        .and_then(|app| app.application_lifetime())
        .and_then(|lifetime| {
            lifetime
                .as_single_top_level_application_lifetime()?
                .top_level()
        })
        .and_then(|top_level| top_level.platform_impl())
        .and_then(|platform| {
            platform
                .as_any()
                .downcast_ref::<BrowserTopLevelImpl>()?
                .surface()
        })
        .ok_or("Start a FerroUI browser view before installing the inspector")?;
    let surface = Arc::downgrade(surface.shared());
    let presented = Closure::<dyn Fn() -> bool>::new(move || {
        surface
            .upgrade()
            .is_some_and(|surface| !surface.is_disposed() && surface.frames() > 0)
    });
    let dispatch = Closure::<dyn Fn(String) -> String>::new(move |request: String| {
        match inspect(&request, permissions) {
            Ok(value) => json!({"ok":true,"value":value}).to_string(),
            Err(error) => json!({"ok":false,"error":error}).to_string(),
        }
    });
    if !register(element_id, dispatch.as_ref(), presented.as_ref()) {
        return Err("Choose the view's host element and install one Live App inspector per page");
    }
    Ok(LiveInspector {
        dispatch,
        _presented: presented,
    })
}

fn inspect(request: &str, permissions: Permissions) -> Result<Value, String> {
    if request.len() > 256 * 1024 {
        return Err("The inspection request is too large".into());
    }
    let request: Value =
        serde_json::from_str(request).map_err(|_| "Invalid inspection request".to_owned())?;
    match request["method"].as_str() {
        Some("tools") => Ok(Value::Array(ferroui_inspect::tools())),
        Some("call") => {
            let name = request["tool"].as_str().ok_or("Choose an inspector tool")?;
            ferroui_inspect::execute(name, &request["arguments"], permissions)
        }
        _ => Err("Unknown inspection request".into()),
    }
}
