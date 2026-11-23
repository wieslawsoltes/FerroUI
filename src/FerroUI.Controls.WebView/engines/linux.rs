//! WebKitGTK on Linux, on X11.
//!
//! The libraries are opened when the first web view is created; without them the control reports
//! `Unavailable`. GTK belongs to its own thread: the widgets live there, commands go there in
//! order, and what WebKit reports comes back to the UI thread as [`Report`] values addressed by
//! the number of the engine, so a report for a closed engine reaches nothing.

use super::bridge;
use crate::engine::{
    CoreWebView2WebErrorStatus, IWebViewEngine, IWebViewEngineEvents, IWebViewEngineFactory,
    ScriptCompletion, WebViewError, WebViewNavigation,
};
use ferroui_base::native_functions;
use ferroui_base::platform::native_library::{NativeLibrary, NativeLibraryError};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::platform::{INativeControlHostDestroyableControlHandle, IPlatformHandle};
use ferroui_x11::interop::glib::Glib;
use ferroui_x11::interop::GtkInteropHelper;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void, CStr, CString};
use std::future::Future;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;

const MESSAGE_HANDLER: &CStr = c"ferrouiWebView";
const MESSAGE_SIGNAL: &CStr = c"script-message-received::ferrouiWebView";
const POST_MESSAGE: &str =
    "function(text){window.webkit.messageHandlers.ferrouiWebView.postMessage(text);}";

const GTK_WINDOW_TOPLEVEL: c_int = 0;
const INJECT_ALL_FRAMES: c_int = 0;
const INJECT_AT_DOCUMENT_START: c_int = 0;
const POLICY_NAVIGATION_ACTION: c_int = 0;
const NAVIGATION_TYPE_LINK: c_int = 0;
const NAVIGATION_TYPE_FORM: c_int = 1;
const LOAD_COMMITTED: c_int = 2;
const LOAD_FINISHED: c_int = 3;
const NETWORK_ERROR_CANCELLED: c_int = 302;

native_functions! {
    /// The functions, as C declares them (a `gboolean` is an `int`).
    pub(crate) struct WebKitApi(webkit, javascript, gtk, gdk, gobject, glib) {
        gtk::gtk_window_new: unsafe extern "C" fn(c_int) -> *mut c_void;
        gtk::gtk_window_set_decorated: unsafe extern "C" fn(*mut c_void, c_int);
        gtk::gtk_container_add: unsafe extern "C" fn(*mut c_void, *mut c_void);
        gtk::gtk_widget_show_all: unsafe extern "C" fn(*mut c_void);
        gtk::gtk_widget_realize: unsafe extern "C" fn(*mut c_void);
        gtk::gtk_widget_get_window: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        gtk::gtk_widget_destroy: unsafe extern "C" fn(*mut c_void);
        gdk::gdk_x11_window_get_xid: unsafe extern "C" fn(*mut c_void) -> c_ulong;
        gobject::g_signal_connect_data:
            unsafe extern "C" fn(*mut c_void, *const c_char, *const c_void, *mut c_void, *const c_void, c_int) -> c_ulong;
        gobject::g_signal_handler_disconnect: unsafe extern "C" fn(*mut c_void, c_ulong);
        gobject::g_object_ref: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        gobject::g_object_unref: unsafe extern "C" fn(*mut c_void);
        glib::g_free: unsafe extern "C" fn(*mut c_void);
        glib::g_error_free: unsafe extern "C" fn(*mut GError);
        javascript::jsc_value_to_string: unsafe extern "C" fn(*mut c_void) -> *mut c_char;
        webkit::webkit_user_content_manager_new: unsafe extern "C" fn() -> *mut c_void;
        webkit::webkit_user_content_manager_add_script: unsafe extern "C" fn(*mut c_void, *mut c_void);
        webkit::webkit_user_content_manager_register_script_message_handler:
            unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;
        webkit::webkit_user_content_manager_unregister_script_message_handler:
            unsafe extern "C" fn(*mut c_void, *const c_char);
        webkit::webkit_user_script_new:
            unsafe extern "C" fn(*const c_char, c_int, c_int, *const *const c_char, *const *const c_char) -> *mut c_void;
        webkit::webkit_user_script_unref: unsafe extern "C" fn(*mut c_void);
        webkit::webkit_web_view_new_with_user_content_manager: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        webkit::webkit_web_view_load_uri: unsafe extern "C" fn(*mut c_void, *const c_char);
        webkit::webkit_web_view_load_html: unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char);
        webkit::webkit_web_view_reload: unsafe extern "C" fn(*mut c_void);
        webkit::webkit_web_view_stop_loading: unsafe extern "C" fn(*mut c_void);
        webkit::webkit_web_view_go_back: unsafe extern "C" fn(*mut c_void);
        webkit::webkit_web_view_go_forward: unsafe extern "C" fn(*mut c_void);
        webkit::webkit_web_view_can_go_back: unsafe extern "C" fn(*mut c_void) -> c_int;
        webkit::webkit_web_view_can_go_forward: unsafe extern "C" fn(*mut c_void) -> c_int;
        webkit::webkit_web_view_get_uri: unsafe extern "C" fn(*mut c_void) -> *const c_char;
        webkit::webkit_web_view_get_title: unsafe extern "C" fn(*mut c_void) -> *const c_char;
        webkit::webkit_web_view_run_javascript:
            unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_void, *const c_void, *mut c_void);
        webkit::webkit_web_view_run_javascript_finish:
            unsafe extern "C" fn(*mut c_void, *mut c_void, *mut *mut GError) -> *mut c_void;
        webkit::webkit_javascript_result_get_js_value: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        webkit::webkit_javascript_result_unref: unsafe extern "C" fn(*mut c_void);
        webkit::webkit_navigation_policy_decision_get_navigation_action:
            unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        webkit::webkit_navigation_action_get_request: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        webkit::webkit_navigation_action_get_navigation_type: unsafe extern "C" fn(*mut c_void) -> c_int;
        webkit::webkit_navigation_action_is_redirect: unsafe extern "C" fn(*mut c_void) -> c_int;
        webkit::webkit_uri_request_get_uri: unsafe extern "C" fn(*mut c_void) -> *const c_char;
        webkit::webkit_policy_decision_use: unsafe extern "C" fn(*mut c_void);
        webkit::webkit_policy_decision_ignore: unsafe extern "C" fn(*mut c_void);
        webkit::webkit_network_error_quark: unsafe extern "C" fn() -> c_uint;
    }
}

/// `GError` of GLib.
#[repr(C)]
pub(crate) struct GError {
    domain: c_uint,
    code: c_int,
    message: *mut c_char,
}

/// The libraries, opened at the first web view. The two WebKit API versions in distributions
/// differ in their HTTP library only, which this engine does not touch.
fn api() -> Result<&'static WebKitApi, WebViewError> {
    static API: OnceLock<Result<WebKitApi, NativeLibraryError>> = OnceLock::new();
    API.get_or_init(|| {
        let (webkit, javascript) = match NativeLibrary::open(c"libwebkit2gtk-4.1.so.0") {
            Ok(webkit) => (
                webkit,
                NativeLibrary::open(c"libjavascriptcoregtk-4.1.so.0")?,
            ),
            Err(_) => (
                NativeLibrary::open(c"libwebkit2gtk-4.0.so.37")?,
                NativeLibrary::open(c"libjavascriptcoregtk-4.0.so.18")?,
            ),
        };
        let gtk = NativeLibrary::open(c"libgtk-3.so.0")?;
        let gdk = NativeLibrary::open(c"libgdk-3.so.0")?;
        let gobject = NativeLibrary::open(c"libgobject-2.0.so.0")?;
        let glib = NativeLibrary::open(c"libglib-2.0.so.0")?;
        WebKitApi::load(&webkit, &javascript, &gtk, &gdk, &gobject, &glib)
    })
    .as_ref()
    .map_err(|error| WebViewError::Unavailable(format!("WebKitGTK could not be loaded: {error}")))
}

/// Text for C: an interior NUL ends it, as it would in the page.
fn c_text(text: &str) -> CString {
    let end = text.find('\0').unwrap_or(text.len());
    CString::new(&text[..end]).expect("the text was cut at its first NUL")
}

/// # Safety
/// `text` is null or a terminated string.
unsafe fn text_of(text: *const c_char) -> String {
    if text.is_null() {
        String::new()
    } else {
        CStr::from_ptr(text).to_string_lossy().into_owned()
    }
}

/// What WebKit reported, on its way to the UI thread.
enum Report {
    Starting {
        decision: usize,
        uri: String,
        is_user_initiated: bool,
        is_redirected: bool,
    },
    Finished,
    Failed(CoreWebView2WebErrorStatus),
    Source(String),
    History(bool, bool),
    Title(String),
    Message {
        source: String,
        json: String,
    },
    Script {
        id: u64,
        result: Result<String, String>,
    },
}

ferroui_base::ferro_thread_local! {
    /// The open engines of the UI thread.
    static ENGINES: RefCell<BTreeMap<u64, Weak<EngineState>>> = const { RefCell::new(BTreeMap::new()) };
}

static NEXT_ENGINE: AtomicU64 = AtomicU64::new(1);

/// Called on the thread of GTK.
fn report(engine: u64, report: Report) {
    Dispatcher::ui_thread().post(move || deliver(engine, report), DispatcherPriority::NORMAL);
}

/// Runs an action on the thread of GTK, after those given before it.
fn on_gtk(action: impl FnOnce(&'static WebKitApi) + Send + 'static) {
    let (Ok(api), Ok(glib)) = (api(), Glib::try_get()) else {
        return;
    };
    glib.g_idle_add_once(Box::new(move || action(api)));
}

/// Answers a navigation WebKit is holding, and lets go of it.
fn decide(decision: usize, allow: bool) {
    on_gtk(move |api| {
        let decision = decision as *mut c_void;
        // SAFETY: the reference taken in `on_decide_policy`, given back once here.
        unsafe {
            if allow {
                (api.webkit_policy_decision_use)(decision);
            } else {
                (api.webkit_policy_decision_ignore)(decision);
            }
            (api.g_object_unref)(decision);
        }
    });
}

/// On the UI thread.
fn deliver(engine: u64, report: Report) {
    let state = ENGINES.with(|engines| engines.borrow().get(&engine).and_then(Weak::upgrade));
    let Some(state) = state else {
        if let Report::Starting { decision, .. } = report {
            decide(decision, false);
        }
        return;
    };
    match report {
        Report::Starting {
            decision,
            uri,
            is_user_initiated,
            is_redirected,
        } => {
            let id = if is_redirected {
                state.navigation.get()
            } else {
                state.navigation.get() + 1
            };
            let allow = state.events.navigation_starting(&WebViewNavigation {
                id,
                uri,
                is_user_initiated,
                is_redirected,
            });
            if allow {
                state.navigation.set(id);
                state.is_navigating.set(true);
            }
            decide(decision, allow);
        }
        Report::Finished => state.complete(true, CoreWebView2WebErrorStatus::Unknown),
        Report::Failed(status) => state.complete(false, status),
        Report::Source(uri) => state.events.source_changed(&uri),
        Report::History(back, forward) => state.events.history_changed(back, forward),
        Report::Title(title) => state.events.document_title_changed(&title),
        Report::Message { source, json } => state.events.web_message_received(&source, &json),
        Report::Script { id, result } => {
            let completion = state.scripts.borrow_mut().remove(&id);
            if let Some(completion) = completion {
                completion(match result {
                    Ok(text) => bridge::parse_result(&text),
                    Err(message) => Err(WebViewError::Engine(message)),
                });
            }
        }
    }
}

/// The address of the engine, as the user data of a signal.
fn engine_of(data: *mut c_void) -> u64 {
    data as usize as u64
}

fn report_history(api: &WebKitApi, web_view: *mut c_void, engine: u64) {
    // SAFETY: a live web view, on the thread of GTK.
    let (back, forward) = unsafe {
        (
            (api.webkit_web_view_can_go_back)(web_view) != 0,
            (api.webkit_web_view_can_go_forward)(web_view) != 0,
        )
    };
    report(engine, Report::History(back, forward));
}

unsafe extern "C" fn on_decide_policy(
    _web_view: *mut c_void,
    decision: *mut c_void,
    kind: c_int,
    data: *mut c_void,
) -> c_int {
    let Ok(api) = api() else { return 0 };
    if kind != POLICY_NAVIGATION_ACTION {
        return 0;
    }
    let action = (api.webkit_navigation_policy_decision_get_navigation_action)(decision);
    let request = (api.webkit_navigation_action_get_request)(action);
    let uri = text_of((api.webkit_uri_request_get_uri)(request));
    let navigation_type = (api.webkit_navigation_action_get_navigation_type)(action);
    let is_redirected = (api.webkit_navigation_action_is_redirect)(action) != 0;
    // Held until the UI thread has asked its handlers; `decide` gives the reference back.
    (api.g_object_ref)(decision);
    report(
        engine_of(data),
        Report::Starting {
            decision: decision as usize,
            uri,
            is_user_initiated: navigation_type == NAVIGATION_TYPE_LINK
                || navigation_type == NAVIGATION_TYPE_FORM,
            is_redirected,
        },
    );
    1
}

unsafe extern "C" fn on_load_changed(web_view: *mut c_void, event: c_int, data: *mut c_void) {
    let Ok(api) = api() else { return };
    if event == LOAD_COMMITTED || event == LOAD_FINISHED {
        report_history(api, web_view, engine_of(data));
    }
    if event == LOAD_FINISHED {
        report(engine_of(data), Report::Finished);
    }
}

unsafe extern "C" fn on_load_failed(
    _web_view: *mut c_void,
    _event: c_int,
    _uri: *const c_char,
    error: *mut GError,
    data: *mut c_void,
) -> c_int {
    let Ok(api) = api() else { return 0 };
    let is_canceled = !error.is_null()
        && (*error).domain == (api.webkit_network_error_quark)()
        && (*error).code == NETWORK_ERROR_CANCELLED;
    let status = if is_canceled {
        CoreWebView2WebErrorStatus::OperationCanceled
    } else {
        CoreWebView2WebErrorStatus::Unknown
    };
    report(engine_of(data), Report::Failed(status));
    0
}

unsafe extern "C" fn on_tls_failed(
    _web_view: *mut c_void,
    _uri: *const c_char,
    _certificate: *mut c_void,
    _errors: c_int,
    data: *mut c_void,
) -> c_int {
    report(
        engine_of(data),
        Report::Failed(CoreWebView2WebErrorStatus::CertificateInvalid),
    );
    // Not handled: WebKit refuses the page.
    0
}

unsafe extern "C" fn on_uri(web_view: *mut c_void, _property: *mut c_void, data: *mut c_void) {
    let Ok(api) = api() else { return };
    report(
        engine_of(data),
        Report::Source(text_of((api.webkit_web_view_get_uri)(web_view))),
    );
    report_history(api, web_view, engine_of(data));
}

unsafe extern "C" fn on_title(web_view: *mut c_void, _property: *mut c_void, data: *mut c_void) {
    let Ok(api) = api() else { return };
    report(
        engine_of(data),
        Report::Title(text_of((api.webkit_web_view_get_title)(web_view))),
    );
}

/// The address of the page that posted a message is that of the view it is in.
struct MessageData {
    engine: u64,
    web_view: usize,
}

unsafe extern "C" fn on_message(_manager: *mut c_void, result: *mut c_void, data: *mut c_void) {
    let Ok(api) = api() else { return };
    let data = &*(data as *const MessageData);
    let value = (api.webkit_javascript_result_get_js_value)(result);
    let text = (api.jsc_value_to_string)(value);
    let json = text_of(text);
    (api.g_free)(text.cast());
    let source = text_of((api.webkit_web_view_get_uri)(data.web_view as *mut c_void));
    report(data.engine, Report::Message { source, json });
}

unsafe extern "C" fn free_message_data(data: *mut c_void, _closure: *mut c_void) {
    drop(Box::from_raw(data as *mut MessageData));
}

struct ScriptRequest {
    engine: u64,
    id: u64,
}

unsafe extern "C" fn on_script_finished(source: *mut c_void, task: *mut c_void, data: *mut c_void) {
    let request = Box::from_raw(data as *mut ScriptRequest);
    let Ok(api) = api() else { return };
    let mut error = std::ptr::null_mut::<GError>();
    let result = (api.webkit_web_view_run_javascript_finish)(source, task, &mut error);
    let outcome = if result.is_null() {
        let message = if error.is_null() {
            String::new()
        } else {
            text_of((*error).message)
        };
        if !error.is_null() {
            (api.g_error_free)(error);
        }
        Err(message)
    } else {
        let value = (api.webkit_javascript_result_get_js_value)(result);
        let text = (api.jsc_value_to_string)(value);
        let outcome = text_of(text);
        (api.g_free)(text.cast());
        (api.webkit_javascript_result_unref)(result);
        Ok(outcome)
    };
    report(
        request.engine,
        Report::Script {
            id: request.id,
            result: outcome,
        },
    );
}

/// The widgets, as numbers: they are only touched on the thread of GTK.
#[derive(Clone, Copy)]
struct Widgets {
    window: usize,
    web_view: usize,
    manager: usize,
    message_signal: c_ulong,
}

/// Makes the widgets on the thread of GTK; returns them and the window on the X server.
fn create_widgets(api: &WebKitApi, engine: u64) -> Option<(Widgets, c_ulong)> {
    let script = c_text(&bridge::document_script(POST_MESSAGE));
    let data = engine as usize as *mut c_void;
    // SAFETY: on the thread of GTK, which is initialized; each object is used as its type, the
    // callbacks have the parameters of their signals, and the texts live for the calls.
    unsafe {
        let manager = (api.webkit_user_content_manager_new)();
        let user_script = (api.webkit_user_script_new)(
            script.as_ptr(),
            INJECT_ALL_FRAMES,
            INJECT_AT_DOCUMENT_START,
            std::ptr::null(),
            std::ptr::null(),
        );
        (api.webkit_user_content_manager_add_script)(manager, user_script);
        (api.webkit_user_script_unref)(user_script);
        (api.webkit_user_content_manager_register_script_message_handler)(
            manager,
            MESSAGE_HANDLER.as_ptr(),
        );
        let web_view = (api.webkit_web_view_new_with_user_content_manager)(manager);

        let connect = |object: *mut c_void, signal: &CStr, callback: *const c_void| {
            (api.g_signal_connect_data)(
                object,
                signal.as_ptr(),
                callback,
                data,
                std::ptr::null(),
                0,
            )
        };
        connect(
            web_view,
            c"decide-policy",
            on_decide_policy as *const c_void,
        );
        connect(web_view, c"load-changed", on_load_changed as *const c_void);
        connect(web_view, c"load-failed", on_load_failed as *const c_void);
        connect(
            web_view,
            c"load-failed-with-tls-errors",
            on_tls_failed as *const c_void,
        );
        connect(web_view, c"notify::uri", on_uri as *const c_void);
        connect(web_view, c"notify::title", on_title as *const c_void);
        let message_data = Box::into_raw(Box::new(MessageData {
            engine,
            web_view: web_view as usize,
        }));
        let message_signal = (api.g_signal_connect_data)(
            manager,
            MESSAGE_SIGNAL.as_ptr(),
            on_message as *const c_void,
            message_data.cast(),
            free_message_data as *const c_void,
            0,
        );

        let window = (api.gtk_window_new)(GTK_WINDOW_TOPLEVEL);
        (api.gtk_window_set_decorated)(window, 0);
        (api.gtk_container_add)(window, web_view);
        (api.gtk_widget_realize)(window);
        (api.gtk_widget_show_all)(window);
        let gdk_window = (api.gtk_widget_get_window)(window);
        if gdk_window.is_null() {
            (api.g_signal_handler_disconnect)(manager, message_signal);
            (api.webkit_user_content_manager_unregister_script_message_handler)(
                manager,
                MESSAGE_HANDLER.as_ptr(),
            );
            (api.gtk_widget_destroy)(window);
            (api.g_object_unref)(manager);
            return None;
        }
        let xid = (api.gdk_x11_window_get_xid)(gdk_window);
        let widgets = Widgets {
            window: window as usize,
            web_view: web_view as usize,
            manager: manager as usize,
            message_signal,
        };
        Some((widgets, xid))
    }
}

struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Waits for the thread of GTK. Only used to make the widgets: the handle of the native control
/// has to exist when the host asks for it, and that thread never waits for the UI thread.
fn wait<T>(future: impl Future<Output = T>) -> T {
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

struct EngineState {
    events: Rc<dyn IWebViewEngineEvents>,
    scripts: RefCell<HashMap<u64, ScriptCompletion>>,
    next_script: Cell<u64>,
    navigation: Cell<u64>,
    is_navigating: Cell<bool>,
}

impl EngineState {
    fn complete(&self, is_success: bool, status: CoreWebView2WebErrorStatus) {
        // WebKit reports a failed load as failed and then as finished.
        if self.is_navigating.replace(false) {
            self.events
                .navigation_completed(self.navigation.get(), is_success, status);
        }
    }
}

struct WindowHandle {
    window: c_ulong,
}

impl IPlatformHandle for WindowHandle {
    fn handle(&self) -> isize {
        self.window as isize
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some("XID")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_native_control_host_destroyable_control_handle(
        &self,
    ) -> Option<&dyn INativeControlHostDestroyableControlHandle> {
        Some(self)
    }
}

impl INativeControlHostDestroyableControlHandle for WindowHandle {
    /// The engine destroys the widgets in `close`, on their thread.
    fn destroy(&self) {}
}

struct LinuxEngine {
    id: u64,
    state: Rc<EngineState>,
    widgets: Widgets,
    control: Rc<WindowHandle>,
    is_closed: Rc<Cell<bool>>,
}

impl LinuxEngine {
    fn on_view(&self, action: impl FnOnce(&'static WebKitApi, *mut c_void) + Send + 'static) {
        if self.is_closed.get() {
            return;
        }
        let web_view = self.widgets.web_view;
        on_gtk(move |api| action(api, web_view as *mut c_void));
    }

    /// Runs a script; with a request its result comes back to the UI thread.
    fn evaluate(&self, script: String, request: Option<ScriptRequest>) {
        self.on_view(move |api, web_view| {
            let source = c_text(&script);
            let (callback, data): (*const c_void, *mut c_void) = match request {
                Some(request) => (
                    on_script_finished as *const c_void,
                    Box::into_raw(Box::new(request)).cast(),
                ),
                None => (std::ptr::null(), std::ptr::null_mut()),
            };
            // SAFETY: a live web view; the callback takes the request back, once.
            unsafe {
                (api.webkit_web_view_run_javascript)(
                    web_view,
                    source.as_ptr(),
                    std::ptr::null_mut(),
                    callback,
                    data,
                );
            }
        });
    }
}

impl IWebViewEngine for LinuxEngine {
    fn native_control(&self) -> Rc<dyn IPlatformHandle> {
        self.control.clone()
    }

    fn close(&self) {
        if self.is_closed.replace(true) {
            return;
        }
        ENGINES.with(|engines| engines.borrow_mut().remove(&self.id));
        let widgets = self.widgets;
        on_gtk(move |api| {
            // SAFETY: the widgets of this engine, destroyed once: `is_closed` guards this call.
            unsafe {
                let manager = widgets.manager as *mut c_void;
                (api.g_signal_handler_disconnect)(manager, widgets.message_signal);
                (api.webkit_user_content_manager_unregister_script_message_handler)(
                    manager,
                    MESSAGE_HANDLER.as_ptr(),
                );
                (api.webkit_web_view_stop_loading)(widgets.web_view as *mut c_void);
                (api.gtk_widget_destroy)(widgets.window as *mut c_void);
                (api.g_object_unref)(manager);
            }
        });
        let scripts = std::mem::take(&mut *self.state.scripts.borrow_mut());
        for (_, completion) in scripts {
            completion(Err(WebViewError::Closed));
        }
    }

    fn navigate(&self, uri: &str) {
        let uri = c_text(uri);
        // SAFETY: a live web view, on its thread.
        self.on_view(move |api, web_view| unsafe {
            (api.webkit_web_view_load_uri)(web_view, uri.as_ptr())
        });
    }

    fn navigate_to_string(&self, html: &str) {
        let html = c_text(html);
        // SAFETY: a live web view, on its thread; the base address is nullable.
        self.on_view(move |api, web_view| unsafe {
            (api.webkit_web_view_load_html)(web_view, html.as_ptr(), std::ptr::null());
        });
    }

    fn reload(&self) {
        // SAFETY: a live web view, on its thread.
        self.on_view(|api, web_view| unsafe { (api.webkit_web_view_reload)(web_view) });
    }

    fn stop(&self) {
        // SAFETY: a live web view, on its thread.
        self.on_view(|api, web_view| unsafe { (api.webkit_web_view_stop_loading)(web_view) });
    }

    fn go_back(&self) {
        // SAFETY: a live web view, on its thread.
        self.on_view(|api, web_view| unsafe { (api.webkit_web_view_go_back)(web_view) });
    }

    fn go_forward(&self) {
        // SAFETY: a live web view, on its thread.
        self.on_view(|api, web_view| unsafe { (api.webkit_web_view_go_forward)(web_view) });
    }

    fn execute_script(&self, script: &str, completion: ScriptCompletion) {
        if self.is_closed.get() {
            completion(Err(WebViewError::Closed));
            return;
        }
        let id = self.state.next_script.get();
        self.state.next_script.set(id + 1);
        self.state.scripts.borrow_mut().insert(id, completion);
        self.evaluate(
            bridge::result_script(script),
            Some(ScriptRequest {
                engine: self.id,
                id,
            }),
        );
    }

    fn post_web_message_as_json(&self, json: &str) {
        self.evaluate(bridge::deliver_script(json), None);
    }
}

impl Drop for LinuxEngine {
    fn drop(&mut self) {
        self.close();
    }
}

/// Creates WebKitGTK engines.
pub(super) struct LinuxEngineFactory;

impl IWebViewEngineFactory for LinuxEngineFactory {
    fn create(
        &self,
        parent: Rc<dyn IPlatformHandle>,
        events: Rc<dyn IWebViewEngineEvents>,
    ) -> Result<Rc<dyn IWebViewEngine>, WebViewError> {
        if parent.handle_descriptor() != Some("XID") {
            return Err(WebViewError::Unavailable(
                "WebKitGTK is embedded on X11 only".to_owned(),
            ));
        }
        let api = api()?;
        let id = NEXT_ENGINE.fetch_add(1, Ordering::Relaxed);
        let created = wait(GtkInteropHelper::run_on_glib_thread(move || {
            create_widgets(api, id)
        }))
        .map_err(WebViewError::Unavailable)?;
        let (widgets, xid) = created.ok_or_else(|| {
            WebViewError::Engine("the window of the web view was not created".to_owned())
        })?;

        let state = Rc::new(EngineState {
            events: events.clone(),
            scripts: RefCell::new(HashMap::new()),
            next_script: Cell::new(1),
            navigation: Cell::new(0),
            is_navigating: Cell::new(false),
        });
        ENGINES.with(|engines| engines.borrow_mut().insert(id, Rc::downgrade(&state)));
        let is_closed = Rc::new(Cell::new(false));
        let engine = Rc::new(LinuxEngine {
            id,
            state,
            widgets,
            control: Rc::new(WindowHandle { window: xid }),
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
