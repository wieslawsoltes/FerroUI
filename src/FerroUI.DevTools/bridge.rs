//! JavaScript bridge for the browser platform. No network listener is created.
use crate::Browser;
use ferroui_base::Ref;
use ferroui_controls::TopLevel;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(
    inline_js = "export function installFerroDevtools(handle, poll) { const f = globalThis.ferroui ??= {}; const previous = f.devtools; const bridge = { handle: message => JSON.parse(handle(typeof message === 'string' ? message : JSON.stringify(message))), poll: () => JSON.parse(poll()) }; f.devtools = bridge; return () => { if (f.devtools === bridge) { if (previous === undefined) delete f.devtools; else f.devtools = previous; } }; }"
)]
extern "C" {
    #[wasm_bindgen(js_name = installFerroDevtools)]
    fn install(handle: &JsValue, poll: &JsValue) -> js_sys::Function;
}

/// Retain this handle while the bridge should be available at `window.ferroui.devtools`.
/// Use `Target.getTargets` and `Target.attachToTarget` to address individual windows.
pub struct BrowserBridge {
    cleanup: js_sys::Function,
    _handle: Closure<dyn FnMut(String) -> String>,
    _poll: Closure<dyn FnMut() -> String>,
}
impl BrowserBridge {
    pub fn install(root: &Ref<TopLevel>) -> Self {
        let mut browser = Browser::new();
        browser.register(root);
        let connection = browser.connect(None).expect("browser connection");
        let browser = Rc::new(RefCell::new(browser));
        let dispatch = browser.clone();
        // A panic in a command (in the application's handlers of the input, for example) answers
        // that command with an error; it does not reach the closure, which would abort the module.
        let handle = Closure::wrap_aborting(Box::new(move |message: String| {
            guarded(|| dispatch.borrow_mut().handle(connection, &message))
        }) as Box<dyn FnMut(String) -> String>);
        let poll = Closure::wrap_aborting(Box::new(move || {
            guarded(|| browser.borrow_mut().poll(connection))
        }) as Box<dyn FnMut() -> String>);
        let cleanup = install(handle.as_ref(), poll.as_ref());
        Self {
            cleanup,
            _handle: handle,
            _poll: poll,
        }
    }
}
fn guarded(f: impl FnOnce() -> Vec<serde_json::Value>) -> String {
    let messages = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| {
        vec![
            serde_json::json!({"id":null,"error":{"code":-32603,"message":"The command panicked"}}),
        ]
    });
    serde_json::to_string(&messages).unwrap_or_else(|_| "[]".into())
}
impl Drop for BrowserBridge {
    fn drop(&mut self) {
        let _ = self.cleanup.call0(&JsValue::NULL);
    }
}
