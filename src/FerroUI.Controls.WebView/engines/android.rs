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
    call_object, call_void, new_object, register_natives, run_native_method, string_of, JavaClass,
    JavaObject, JavaValue, NativeMethod,
};
use ferroui_android::AndroidViewControlHandle;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::platform::IPlatformHandle;
use jni_sys::{jclass, jint, jlong, jobject, JNIEnv};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Once;

const CLASS: &str = "org/ferroui/webview/WebViewEngine";
const POST_MESSAGE: &str = "function(text){ferrouiWebViewHost.postMessage(text);}";

/// What the Java class calls back into. Java holds one strong count, as a number, until `close`.
struct Owner {
    events: Rc<dyn IWebViewEngineEvents>,
    scripts: RefCell<HashMap<i64, ScriptCompletion>>,
    next_script: Cell<i64>,
}

/// # Safety
/// `raw` is 0 or the number the engine gave to Java, which Java clears in `close`.
unsafe fn owner(raw: jlong) -> Option<Rc<Owner>> {
    if raw == 0 {
        return None;
    }
    let pointer = raw as *const Owner;
    Rc::increment_strong_count(pointer);
    Some(Rc::from_raw(pointer))
}

/// # Safety
/// `text` is null or a valid reference to a string for this call.
unsafe fn text(value: jobject) -> String {
    JavaObject::from_raw(value)
        .map(|value| string_of(&value))
        .unwrap_or_default()
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

unsafe extern "system" fn native_navigation_starting(
    _env: *mut JNIEnv,
    _class: jclass,
    raw: jlong,
    id: jlong,
    uri: jobject,
    is_user_initiated: u8,
    is_redirected: u8,
) -> u8 {
    run_native_method(0, || {
        let Some(owner) = owner(raw) else { return 0 };
        let allowed = owner.events.navigation_starting(&WebViewNavigation {
            id: id as u64,
            uri: text(uri),
            is_user_initiated: is_user_initiated != 0,
            is_redirected: is_redirected != 0,
        });
        u8::from(allowed)
    })
}

unsafe extern "system" fn native_navigation_completed(
    _env: *mut JNIEnv,
    _class: jclass,
    raw: jlong,
    id: jlong,
    is_success: u8,
    error: jint,
) {
    run_native_method((), || {
        if let Some(owner) = owner(raw) {
            owner
                .events
                .navigation_completed(id as u64, is_success != 0, status_of(error));
        }
    });
}

unsafe extern "system" fn native_source_changed(
    _env: *mut JNIEnv,
    _class: jclass,
    raw: jlong,
    uri: jobject,
) {
    run_native_method((), || {
        if let Some(owner) = owner(raw) {
            owner.events.source_changed(&text(uri));
        }
    });
}

unsafe extern "system" fn native_history_changed(
    _env: *mut JNIEnv,
    _class: jclass,
    raw: jlong,
    can_go_back: u8,
    can_go_forward: u8,
) {
    run_native_method((), || {
        if let Some(owner) = owner(raw) {
            owner
                .events
                .history_changed(can_go_back != 0, can_go_forward != 0);
        }
    });
}

unsafe extern "system" fn native_title_changed(
    _env: *mut JNIEnv,
    _class: jclass,
    raw: jlong,
    title: jobject,
) {
    run_native_method((), || {
        if let Some(owner) = owner(raw) {
            owner.events.document_title_changed(&text(title));
        }
    });
}

unsafe extern "system" fn native_web_message(
    _env: *mut JNIEnv,
    _class: jclass,
    raw: jlong,
    source: jobject,
    message: jobject,
) {
    run_native_method((), || {
        if let Some(owner) = owner(raw) {
            owner
                .events
                .web_message_received(&text(source), &text(message));
        }
    });
}

unsafe extern "system" fn native_script_completed(
    _env: *mut JNIEnv,
    _class: jclass,
    raw: jlong,
    id: jlong,
    result: jobject,
) {
    run_native_method((), || {
        let Some(owner) = owner(raw) else { return };
        let completion = owner.scripts.borrow_mut().remove(&id);
        if let Some(completion) = completion {
            // The value of the result script is a string, which the system returns as JSON.
            completion(match json::parse_string(&text(result)) {
                Some(value) => bridge::parse_result(&value),
                None => Err(WebViewError::Engine("the script gave no result".to_owned())),
            });
        }
    });
}

fn register(class: &JavaClass) {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        let methods = [
            NativeMethod {
                name: c"nativeNavigationStarting",
                signature: c"(JJLjava/lang/String;ZZ)Z",
                function: native_navigation_starting as *mut c_void,
            },
            NativeMethod {
                name: c"nativeNavigationCompleted",
                signature: c"(JJZI)V",
                function: native_navigation_completed as *mut c_void,
            },
            NativeMethod {
                name: c"nativeSourceChanged",
                signature: c"(JLjava/lang/String;)V",
                function: native_source_changed as *mut c_void,
            },
            NativeMethod {
                name: c"nativeHistoryChanged",
                signature: c"(JZZ)V",
                function: native_history_changed as *mut c_void,
            },
            NativeMethod {
                name: c"nativeTitleChanged",
                signature: c"(JLjava/lang/String;)V",
                function: native_title_changed as *mut c_void,
            },
            NativeMethod {
                name: c"nativeWebMessage",
                signature: c"(JLjava/lang/String;Ljava/lang/String;)V",
                function: native_web_message as *mut c_void,
            },
            NativeMethod {
                name: c"nativeScriptCompleted",
                signature: c"(JJLjava/lang/String;)V",
                function: native_script_completed as *mut c_void,
            },
        ];
        // SAFETY: each function has the parameters of its signature after the environment and
        // the class, with `u8` for a Java `boolean`.
        unsafe { register_natives(class, &methods) };
    });
}

struct AndroidEngine {
    java: JavaObject,
    owner: Rc<Owner>,
    /// The strong count Java holds as a number; 0 once given back.
    java_owner: Cell<jlong>,
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
        let raw = self.java_owner.replace(0);
        if raw != 0 {
            // SAFETY: the count made for Java in `create`; Java no longer has the number.
            drop(unsafe { Rc::from_raw(raw as *const Owner) });
        }
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
        register(&class);
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
        let java_owner = Rc::into_raw(owner.clone()) as jlong;
        let script = bridge::document_script(POST_MESSAGE);
        let java = new_object(
            &class,
            "(Landroid/content/Context;JLjava/lang/String;)V",
            &[
                JavaValue::Object(Some(&context)),
                JavaValue::Long(java_owner),
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
            java_owner: Cell::new(java_owner),
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
