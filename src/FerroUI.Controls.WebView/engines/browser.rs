//! An iframe in the browser; see `browser.js` for what the browser allows it to do.
//!
//! The frame is sandboxed from the application. A document given as text has the full page
//! interface. For a page from an address the browser reports only that a load ended: not its
//! errors, its navigations inside the frame, its title or its history, and scripts and messages
//! are refused.

use super::bridge;
use crate::engine::{
    CoreWebView2WebErrorStatus, IWebViewEngine, IWebViewEngineEvents, IWebViewEngineFactory, ScriptCompletion,
    WebViewError, WebViewNavigation,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_browser::JsObjectControlHandle;
use ferroui_controls::platform::IPlatformHandle;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

const POST_MESSAGE: &str =
    "function(text){parent.postMessage({ferrouiWebView:true,type:'message',text:text},'*');}";
const TEXT_DOCUMENT: &str = "about:srcdoc";

#[wasm_bindgen(module = "/engines/browser.js")]
extern "C" {
    type WebFrame;
    #[wasm_bindgen(constructor)]
    fn new(
        bridge_script: &str,
        on_load: &Closure<dyn FnMut()>,
        on_message: &Closure<dyn FnMut(String)>,
        on_result: &Closure<dyn FnMut(f64, String)>,
        on_title: &Closure<dyn FnMut(String)>,
    ) -> WebFrame;
    #[wasm_bindgen(method, getter)]
    fn element(this: &WebFrame) -> JsValue;
    #[wasm_bindgen(method)]
    fn navigate(this: &WebFrame, uri: &str);
    #[wasm_bindgen(method, js_name = navigateToString)]
    fn navigate_to_string(this: &WebFrame, html: &str);
    #[wasm_bindgen(method)]
    fn reload(this: &WebFrame);
    #[wasm_bindgen(method, js_name = executeScript)]
    fn execute_script(this: &WebFrame, id: f64, script: &str);
    #[wasm_bindgen(method, js_name = postMessage)]
    fn post_message(this: &WebFrame, json: &str);
    #[wasm_bindgen(method)]
    fn close(this: &WebFrame);
    #[wasm_bindgen(js_name = frameBridge)]
    fn frame_bridge() -> String;
}

struct State {
    events: Rc<dyn IWebViewEngineEvents>,
    scripts: RefCell<HashMap<u64, ScriptCompletion>>,
    next_script: Cell<u64>,
    next_navigation: Cell<u64>,
    /// The navigation whose load event is awaited; 0 for none.
    navigation: Cell<u64>,
    source: RefCell<String>,
}

impl State {
    /// Asks whether a navigation may start and makes it the current one.
    fn announce(&self, uri: &str) -> bool {
        let id = self.next_navigation.get();
        self.next_navigation.set(id + 1);
        let allowed = self.events.navigation_starting(&WebViewNavigation {
            id,
            uri: uri.to_owned(),
            is_user_initiated: false,
            is_redirected: false,
        });
        if allowed {
            self.navigation.set(id);
            *self.source.borrow_mut() = uri.to_owned();
        }
        allowed
    }
}

struct BrowserEngine {
    frame: WebFrame,
    state: Rc<State>,
    control: Rc<JsObjectControlHandle>,
    is_closed: Rc<Cell<bool>>,
    document_script: String,
    _on_load: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(String)>,
    _on_result: Closure<dyn FnMut(f64, String)>,
    _on_title: Closure<dyn FnMut(String)>,
}

impl IWebViewEngine for BrowserEngine {
    fn native_control(&self) -> Rc<dyn IPlatformHandle> {
        self.control.clone()
    }

    fn close(&self) {
        if self.is_closed.replace(true) {
            return;
        }
        self.frame.close();
        let scripts = std::mem::take(&mut *self.state.scripts.borrow_mut());
        for (_, completion) in scripts {
            completion(Err(WebViewError::Closed));
        }
    }

    fn navigate(&self, uri: &str) {
        if self.state.announce(uri) && !self.is_closed.get() {
            self.frame.navigate(uri);
            self.state.events.source_changed(uri);
        }
    }

    fn navigate_to_string(&self, html: &str) {
        if self.state.announce(TEXT_DOCUMENT) && !self.is_closed.get() {
            self.frame.navigate_to_string(&bridge::with_document_script(html, &self.document_script));
            self.state.events.source_changed(TEXT_DOCUMENT);
        }
    }

    fn reload(&self) {
        let source = self.state.source.borrow().clone();
        if !source.is_empty() && self.state.announce(&source) && !self.is_closed.get() {
            self.frame.reload();
        }
    }

    /// A frame cannot be stopped from outside its origin.
    fn stop(&self) {}

    /// The history of a frame of another origin cannot be read or moved.
    fn go_back(&self) {}

    /// The history of a frame of another origin cannot be read or moved.
    fn go_forward(&self) {}

    fn execute_script(&self, script: &str, completion: ScriptCompletion) {
        if self.is_closed.get() {
            completion(Err(WebViewError::Closed));
            return;
        }
        let id = self.state.next_script.get();
        self.state.next_script.set(id + 1);
        self.state.scripts.borrow_mut().insert(id, completion);
        self.frame.execute_script(id as f64, &bridge::result_script(script));
    }

    fn post_web_message_as_json(&self, json: &str) {
        if !self.is_closed.get() {
            self.frame.post_message(json);
        }
    }
}

impl Drop for BrowserEngine {
    fn drop(&mut self) {
        self.close();
    }
}

/// Creates engines on an iframe.
pub(super) struct BrowserEngineFactory;

impl IWebViewEngineFactory for BrowserEngineFactory {
    fn create(
        &self,
        _parent: Rc<dyn IPlatformHandle>,
        events: Rc<dyn IWebViewEngineEvents>,
    ) -> Result<Rc<dyn IWebViewEngine>, WebViewError> {
        let state = Rc::new(State {
            events: events.clone(),
            scripts: RefCell::new(HashMap::new()),
            next_script: Cell::new(1),
            next_navigation: Cell::new(1),
            navigation: Cell::new(0),
            source: RefCell::new(String::new()),
        });
        let document_script = format!("{}{}", bridge::document_script(POST_MESSAGE), frame_bridge());

        let load_state = state.clone();
        let on_load = Closure::<dyn FnMut()>::new(move || {
            let id = load_state.navigation.replace(0);
            if id != 0 {
                // The browser does not say whether the load of a frame failed.
                load_state.events.navigation_completed(id, true, CoreWebView2WebErrorStatus::Unknown);
            }
        });
        let message_state = state.clone();
        let on_message = Closure::<dyn FnMut(String)>::new(move |json: String| {
            let source = message_state.source.borrow().clone();
            message_state.events.web_message_received(&source, &json);
        });
        let result_state = state.clone();
        let on_result = Closure::<dyn FnMut(f64, String)>::new(move |id: f64, text: String| {
            let completion = result_state.scripts.borrow_mut().remove(&(id as u64));
            if let Some(completion) = completion {
                completion(bridge::parse_result(&text));
            }
        });
        let title_state = state.clone();
        let on_title = Closure::<dyn FnMut(String)>::new(move |title: String| {
            title_state.events.document_title_changed(&title);
        });

        let frame = WebFrame::new(&document_script, &on_load, &on_message, &on_result, &on_title);
        let control = Rc::new(JsObjectControlHandle::new(frame.element()));
        let is_closed = Rc::new(Cell::new(false));
        let engine = Rc::new(BrowserEngine {
            frame,
            state,
            control,
            is_closed: is_closed.clone(),
            document_script,
            _on_load: on_load,
            _on_message: on_message,
            _on_result: on_result,
            _on_title: on_title,
        });
        Dispatcher::ui_thread().post_local(
            move || {
                if !is_closed.get() {
                    events.initialized(Ok(()));
                }
            },
            DispatcherPriority::NORMAL,
        );
        Ok(engine)
    }
}
