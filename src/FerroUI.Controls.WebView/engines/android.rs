//! `android.webkit.WebView`, through the Java class of this crate (`packaging.gradle`).
//!
//! A page gets `window.chrome.webview` from a script run when its load starts. For a document
//! given as text the script is its first element; for a page from an address the system
//! offers no earlier point, so an inline script of that page can run before it.

use super::bridge;
use crate::engine::{
    CoreWebView2WebErrorStatus, IWebViewEngine, IWebViewEngineEvents, IWebViewEngineFactory,
    ScriptCompletion, WebViewError, WebViewNavigation,
};
use crate::json;
use ferroui_android::interop::java::{
    call_object, call_void, new_object, JavaClass, JavaObject, JavaValue,
};
use ferroui_android::interop::java_callback::{JavaCallback, JavaCallbackArgs};
use ferroui_android::AndroidViewControlHandle;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::platform::IPlatformHandle;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

const CLASS: &str = "org/ferroui/webview/WebViewEngine";
const POST_MESSAGE: &str = "function(text){ferrouiWebViewHost.postMessage(text);}";

// The kinds of call from the Java class, with the same numbers there.
const NAVIGATION_STARTING: i32 = 0;
const NAVIGATION_COMPLETED: i32 = 1;
const SOURCE_CHANGED: i32 = 2;
const HISTORY_CHANGED: i32 = 3;
const TITLE_CHANGED: i32 = 4;
const WEB_MESSAGE: i32 = 5;
const SCRIPT_COMPLETED: i32 = 6;

/// What the Java class calls back into.
struct Owner {
    events: Rc<dyn IWebViewEngineEvents>,
    scripts: RefCell<HashMap<i64, ScriptCompletion>>,
    next_script: Cell<i64>,
}

impl Owner {
    fn called(&self, args: JavaCallbackArgs) -> bool {
        let first_text = args.first_text.unwrap_or_default();
        match args.kind {
            NAVIGATION_STARTING => {
                return self.events.navigation_starting(&WebViewNavigation {
                    id: args.number as u64,
                    uri: first_text,
                    is_user_initiated: args.first,
                    is_redirected: args.second,
                });
            }
            NAVIGATION_COMPLETED => {
                self.events.navigation_completed(args.number as u64, args.first, status_of(args.value));
            }
            SOURCE_CHANGED => self.events.source_changed(&first_text),
            HISTORY_CHANGED => self.events.history_changed(args.first, args.second),
            TITLE_CHANGED => self.events.document_title_changed(&first_text),
            WEB_MESSAGE => {
                self.events.web_message_received(&first_text, &args.second_text.unwrap_or_default());
            }
            SCRIPT_COMPLETED => {
                let completion = self.scripts.borrow_mut().remove(&args.number);
                if let Some(completion) = completion {
                    // The value of the result script is a string, which the system returns as JSON.
                    completion(match json::parse_string(&first_text) {
                        Some(value) => bridge::parse_result(&value),
                        None => Err(WebViewError::Engine("the script gave no result".to_owned())),
                    });
                }
            }
            _ => {}
        }
        false
    }
}

fn status_of(error: i32) -> CoreWebView2WebErrorStatus {
    use CoreWebView2WebErrorStatus as Status;
    // The `ERROR_` constants of `WebViewClient`.
    match error {
        -2 => Status::HostNameNotResolved,
        -6 => Status::ServerUnreachable,
        -8 => Status::Timeout,
        -9 => Status::RedirectFailed,
        -11 => Status::CertificateInvalid,
        _ => Status::Unknown,
    }
}

struct AndroidEngine {
    java: JavaObject,
    owner: Rc<Owner>,
    /// Ends the calls from Java when it is dropped.
    callback: RefCell<Option<JavaCallback>>,
    control: Rc<AndroidViewControlHandle>,
    is_closed: Rc<Cell<bool>>,
}

impl IWebViewEngine for AndroidEngine {
    fn native_control(&self) -> Rc<dyn IPlatformHandle> {
        self.control.clone()
    }

    fn close(&self) {
        if self.is_closed.replace(true) {
            return;
        }
        call_void(&self.java, "close", "()V", &[]);
        let callback = self.callback.borrow_mut().take();
        drop(callback);
        let scripts = std::mem::take(&mut *self.owner.scripts.borrow_mut());
        for (_, completion) in scripts {
            completion(Err(WebViewError::Closed));
        }
    }

    fn navigate(&self, uri: &str) {
        call_void(
            &self.java,
            "navigate",
            "(Ljava/lang/String;)V",
            &[JavaValue::String(uri)],
        );
    }

    fn navigate_to_string(&self, html: &str) {
        let script = bridge::document_script(POST_MESSAGE);
        let document = bridge::with_document_script(html, &script);
        call_void(
            &self.java,
            "navigateToString",
            "(Ljava/lang/String;)V",
            &[JavaValue::String(&document)],
        );
    }

    fn reload(&self) {
        call_void(&self.java, "reload", "()V", &[]);
    }

    fn stop(&self) {
        call_void(&self.java, "stop", "()V", &[]);
    }

    fn go_back(&self) {
        call_void(&self.java, "goBack", "()V", &[]);
    }

    fn go_forward(&self) {
        call_void(&self.java, "goForward", "()V", &[]);
    }

    fn execute_script(&self, script: &str, completion: ScriptCompletion) {
        if self.is_closed.get() {
            completion(Err(WebViewError::Closed));
            return;
        }
        let id = self.owner.next_script.get();
        self.owner.next_script.set(id + 1);
        self.owner.scripts.borrow_mut().insert(id, completion);
        let source = bridge::result_script(script);
        call_void(
            &self.java,
            "executeScript",
            "(JLjava/lang/String;)V",
            &[JavaValue::Long(id), JavaValue::String(&source)],
        );
    }

    fn post_web_message_as_json(&self, json: &str) {
        let script = bridge::deliver_script(json);
        call_void(
            &self.java,
            "evaluate",
            "(Ljava/lang/String;)V",
            &[JavaValue::String(&script)],
        );
    }
}

impl Drop for AndroidEngine {
    fn drop(&mut self) {
        self.close();
    }
}

/// Creates engines on the system web view.
pub(super) struct AndroidEngineFactory;

impl IWebViewEngineFactory for AndroidEngineFactory {
    fn create(
        &self,
        parent: Rc<dyn IPlatformHandle>,
        events: Rc<dyn IWebViewEngineEvents>,
    ) -> Result<Rc<dyn IWebViewEngine>, WebViewError> {
        let class = JavaClass::try_find(CLASS).ok_or_else(|| {
            WebViewError::Unavailable(
                "the application does not apply the packaging.gradle of this crate".to_owned(),
            )
        })?;
        let parent_view = parent
            .as_any()
            .downcast_ref::<AndroidViewControlHandle>()
            .and_then(AndroidViewControlHandle::view)
            .ok_or_else(|| WebViewError::Unavailable("the parent is not a view".to_owned()))?;
        let context = call_object(
            &parent_view,
            "getContext",
            "()Landroid/content/Context;",
            &[],
        )
        .ok_or_else(|| WebViewError::Unavailable("the parent view has no context".to_owned()))?;

        let owner = Rc::new(Owner {
            events: events.clone(),
            scripts: RefCell::new(HashMap::new()),
            next_script: Cell::new(1),
        });
        let called: Weak<Owner> = Rc::downgrade(&owner);
        let callback = JavaCallback::new(move |args| called.upgrade().is_some_and(|owner| owner.called(args)));
        let script = bridge::document_script(POST_MESSAGE);
        let java = new_object(
            &class,
            "(Landroid/content/Context;JLjava/lang/String;)V",
            &[
                JavaValue::Object(Some(&context)),
                JavaValue::Long(callback.handle()),
                JavaValue::String(&script),
            ],
        )
        .to_global();
        let view = call_object(&java, "view", "()Landroid/view/View;", &[])
            .ok_or_else(|| WebViewError::Engine("the web view has no view".to_owned()))?
            .to_global();

        let is_closed = Rc::new(Cell::new(false));
        let engine = Rc::new(AndroidEngine {
            java,
            owner,
            callback: RefCell::new(Some(callback)),
            control: Rc::new(AndroidViewControlHandle::new(view)),
            is_closed: is_closed.clone(),
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
