//! WKWebView on macOS and iOS. WebKit is loaded when the first web view is created, so an
//! application that links this crate and shows no web view does not pay for it at start-up.

use super::bridge;
use crate::engine::{
    CoreWebView2WebErrorStatus, IWebViewEngine, IWebViewEngineEvents, IWebViewEngineFactory,
    ScriptCompletion, WebViewError, WebViewNavigation,
};
use block2::{DynBlock, RcBlock};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::platform::{INativeControlHostDestroyableControlHandle, IPlatformHandle};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, NSObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void, CStr};
use std::rc::Rc;

const MESSAGE_HANDLER: &str = "ferrouiWebView";
const OBSERVED_KEYS: [&str; 4] = ["URL", "title", "canGoBack", "canGoForward"];
const WEBKIT_PATH: &CStr = c"/System/Library/Frameworks/WebKit.framework/WebKit";

#[cfg(target_os = "ios")]
const VIEW_DESCRIPTOR: &str = "UIView";
#[cfg(not(target_os = "ios"))]
const VIEW_DESCRIPTOR: &str = "NSView";

const UTF8: usize = 4;
const POLICY_CANCEL: isize = 0;
const POLICY_ALLOW: isize = 1;
const NAVIGATION_LINK: isize = 0;
const NAVIGATION_FORM: isize = 1;
const USER_SCRIPT_AT_START: isize = 0;
const KVO_NEW: usize = 1;

fn class(name: &CStr) -> Result<&'static AnyClass, WebViewError> {
    AnyClass::get(name).ok_or_else(|| {
        WebViewError::Unavailable(format!("{} is not available", name.to_string_lossy()))
    })
}

fn ns_string(text: &str) -> Retained<AnyObject> {
    let class =
        AnyClass::get(c"NSString").expect("Foundation is loaded with the Objective-C runtime");
    // SAFETY: the bytes are valid UTF-8 of the given length and are copied by the initializer.
    unsafe {
        let allocated: objc2::rc::Allocated<AnyObject> = msg_send![class, alloc];
        msg_send![allocated, initWithBytes: text.as_ptr().cast::<c_void>(), length: text.len(), encoding: UTF8]
    }
}

/// # Safety
/// `value` is null or an `NSString`.
unsafe fn string_of(value: *mut AnyObject) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let text: *const c_char = msg_send![value, UTF8String];
    (!text.is_null()).then(|| CStr::from_ptr(text).to_string_lossy().into_owned())
}

/// # Safety
/// `url` is null or an `NSURL`.
unsafe fn absolute_string(url: *mut AnyObject) -> Option<String> {
    if url.is_null() {
        return None;
    }
    let text: *mut AnyObject = msg_send![url, absoluteString];
    string_of(text)
}

fn status_of(code: isize) -> CoreWebView2WebErrorStatus {
    use CoreWebView2WebErrorStatus as Status;
    match code {
        -999 => Status::OperationCanceled,
        -1001 => Status::Timeout,
        -1003 => Status::HostNameNotResolved,
        -1004 => Status::ServerUnreachable,
        -1005 => Status::ConnectionAborted,
        -1007 => Status::RedirectFailed,
        -1009 => Status::Disconnected,
        -1206..=-1200 => Status::CertificateInvalid,
        _ => Status::Unknown,
    }
}

struct DelegateState {
    events: Rc<dyn IWebViewEngineEvents>,
    next_navigation: Cell<u64>,
    navigation: Cell<u64>,
    /// The main frame has a load that did not commit yet: a further decision is its redirect.
    is_provisional: Cell<bool>,
}

impl DelegateState {
    fn finish(&self, is_success: bool, status: CoreWebView2WebErrorStatus) {
        self.is_provisional.set(false);
        self.events
            .navigation_completed(self.navigation.get(), is_success, status);
    }

    /// # Safety
    /// `error` is null or an `NSError`.
    unsafe fn fail(&self, error: *mut AnyObject) {
        let code: isize = if error.is_null() {
            0
        } else {
            msg_send![error, code]
        };
        self.finish(false, status_of(code));
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FerroUIWebViewDelegate"]
    #[ivars = DelegateState]
    struct Delegate;

    impl Delegate {
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn decide_policy(&self, _web_view: *mut AnyObject, action: *mut AnyObject, handler: &DynBlock<dyn Fn(isize)>) {
            // SAFETY: WebKit passes a live WKNavigationAction.
            let allow = unsafe { self.decide(action) };
            handler.call((if allow { POLICY_ALLOW } else { POLICY_CANCEL },));
        }

        #[unsafe(method(webView:didStartProvisionalNavigation:))]
        fn did_start(&self, _web_view: *mut AnyObject, _navigation: *mut AnyObject) {
            self.ivars().is_provisional.set(true);
        }

        #[unsafe(method(webView:didCommitNavigation:))]
        fn did_commit(&self, _web_view: *mut AnyObject, _navigation: *mut AnyObject) {
            self.ivars().is_provisional.set(false);
        }

        #[unsafe(method(webView:didFinishNavigation:))]
        fn did_finish(&self, _web_view: *mut AnyObject, _navigation: *mut AnyObject) {
            self.ivars().finish(true, CoreWebView2WebErrorStatus::Unknown);
        }

        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn did_fail(&self, _web_view: *mut AnyObject, _navigation: *mut AnyObject, error: *mut AnyObject) {
            // SAFETY: WebKit passes an NSError.
            unsafe { self.ivars().fail(error) };
        }

        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn did_fail_provisional(&self, _web_view: *mut AnyObject, _navigation: *mut AnyObject, error: *mut AnyObject) {
            // SAFETY: WebKit passes an NSError.
            unsafe { self.ivars().fail(error) };
        }

        #[unsafe(method(userContentController:didReceiveScriptMessage:))]
        fn did_receive_message(&self, _controller: *mut AnyObject, message: *mut AnyObject) {
            // SAFETY: WebKit passes a live WKScriptMessage; the bridge posts strings only, and
            // `isKindOfClass:` rejects a page calling the handler directly with something else.
            unsafe {
                let body: *mut AnyObject = msg_send![message, body];
                let Some(string_class) = AnyClass::get(c"NSString") else { return };
                if body.is_null() || !msg_send![body, isKindOfClass: string_class] {
                    return;
                }
                let Some(json) = string_of(body) else { return };
                let frame: *mut AnyObject = msg_send![message, frameInfo];
                let request: *mut AnyObject = msg_send![frame, request];
                let url: *mut AnyObject = msg_send![request, URL];
                let source = absolute_string(url).unwrap_or_default();
                self.ivars().events.web_message_received(&source, &json);
            }
        }

        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn observe_value(
            &self,
            key_path: *mut AnyObject,
            object: *mut AnyObject,
            _change: *mut AnyObject,
            _context: *mut c_void,
        ) {
            // SAFETY: the delegate observes the keys of its WKWebView only.
            unsafe { self.observed(key_path, object) };
        }
    }
);

impl Delegate {
    fn new(mtm: MainThreadMarker, events: Rc<dyn IWebViewEngineEvents>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(DelegateState {
            events,
            next_navigation: Cell::new(1),
            navigation: Cell::new(0),
            is_provisional: Cell::new(false),
        });
        // SAFETY: `NSObject` is initialized with `init`.
        unsafe { msg_send![super(this), init] }
    }

    /// # Safety
    /// `action` is a live WKNavigationAction.
    unsafe fn decide(&self, action: *mut AnyObject) -> bool {
        let frame: *mut AnyObject = msg_send![action, targetFrame];
        // A request for a new window has no frame; it counts as a navigation of the page.
        if !frame.is_null() {
            let is_main: bool = msg_send![frame, isMainFrame];
            if !is_main {
                return true;
            }
        }
        let request: *mut AnyObject = msg_send![action, request];
        let url: *mut AnyObject = msg_send![request, URL];
        let uri = absolute_string(url).unwrap_or_default();
        let kind: isize = msg_send![action, navigationType];
        let state = self.ivars();
        let is_redirected = state.is_provisional.get();
        if !is_redirected {
            let id = state.next_navigation.get();
            state.next_navigation.set(id + 1);
            state.navigation.set(id);
        }
        state.events.navigation_starting(&WebViewNavigation {
            id: state.navigation.get(),
            uri,
            is_user_initiated: kind == NAVIGATION_LINK || kind == NAVIGATION_FORM,
            is_redirected,
        })
    }

    /// # Safety
    /// `key_path` is an `NSString` and `web_view` the observed WKWebView.
    unsafe fn observed(&self, key_path: *mut AnyObject, web_view: *mut AnyObject) {
        let events = &self.ivars().events;
        match string_of(key_path).as_deref() {
            Some("URL") => {
                let url: *mut AnyObject = msg_send![web_view, URL];
                if let Some(uri) = absolute_string(url) {
                    events.source_changed(&uri);
                }
            }
            Some("title") => {
                let title: *mut AnyObject = msg_send![web_view, title];
                events.document_title_changed(&string_of(title).unwrap_or_default());
            }
            Some("canGoBack" | "canGoForward") => {
                let can_go_back: bool = msg_send![web_view, canGoBack];
                let can_go_forward: bool = msg_send![web_view, canGoForward];
                events.history_changed(can_go_back, can_go_forward);
            }
            _ => {}
        }
    }
}

/// The view handed to the native control host.
struct ViewHandle {
    view: Retained<AnyObject>,
}

impl IPlatformHandle for ViewHandle {
    fn handle(&self) -> isize {
        Retained::as_ptr(&self.view) as isize
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(VIEW_DESCRIPTOR)
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

impl INativeControlHostDestroyableControlHandle for ViewHandle {
    fn destroy(&self) {
        // SAFETY: a view can be removed from its parent at any time, also when it has none.
        unsafe {
            let _: () = msg_send![&*self.view, removeFromSuperview];
        }
    }
}

struct AppleEngine {
    web_view: Retained<AnyObject>,
    controller: Retained<AnyObject>,
    delegate: Retained<Delegate>,
    is_closed: Rc<Cell<bool>>,
}

impl AppleEngine {
    fn evaluate(&self, script: &str, completion: Option<ScriptCompletion>) {
        let source = ns_string(script);
        match completion {
            Some(completion) => {
                let completion = RefCell::new(Some(completion));
                let block = RcBlock::new(move |result: *mut AnyObject, error: *mut AnyObject| {
                    let Some(completion) = completion.borrow_mut().take() else {
                        return;
                    };
                    // SAFETY: WebKit passes the value of the script, a string from the result
                    // script, or an NSError.
                    let outcome = unsafe {
                        if error.is_null() {
                            Ok(string_of(result).unwrap_or_else(|| "null".to_owned()))
                        } else {
                            let text: *mut AnyObject = msg_send![error, localizedDescription];
                            Err(WebViewError::Engine(string_of(text).unwrap_or_default()))
                        }
                    };
                    completion(outcome);
                });
                // SAFETY: WebKit copies the block and calls it on the main thread.
                unsafe {
                    let _: () = msg_send![&*self.web_view, evaluateJavaScript: &*source, completionHandler: &*block];
                }
            }
            None => {
                let none: Option<&DynBlock<dyn Fn(*mut AnyObject, *mut AnyObject)>> = None;
                // SAFETY: the completion handler of this method is nullable.
                unsafe {
                    let _: () = msg_send![&*self.web_view, evaluateJavaScript: &*source, completionHandler: none];
                }
            }
        }
    }
}

impl IWebViewEngine for AppleEngine {
    fn native_control(&self) -> Rc<dyn IPlatformHandle> {
        Rc::new(ViewHandle {
            view: self.web_view.clone(),
        })
    }

    fn close(&self) {
        if self.is_closed.replace(true) {
            return;
        }
        // SAFETY: each observer and handler was added once, in `create`; the delegate outlives
        // these calls.
        unsafe {
            for key in OBSERVED_KEYS {
                let key = ns_string(key);
                let _: () =
                    msg_send![&*self.web_view, removeObserver: &*self.delegate, forKeyPath: &*key];
            }
            let name = ns_string(MESSAGE_HANDLER);
            let _: () = msg_send![&*self.controller, removeScriptMessageHandlerForName: &*name];
            let no_delegate: Option<&AnyObject> = None;
            let _: () = msg_send![&*self.web_view, setNavigationDelegate: no_delegate];
            let _: () = msg_send![&*self.web_view, stopLoading];
            let _: () = msg_send![&*self.web_view, removeFromSuperview];
        }
    }

    fn navigate(&self, uri: &str) {
        let text = ns_string(uri);
        // SAFETY: the classes are loaded by `create` and the arguments are live objects.
        unsafe {
            let Some(url_class) = AnyClass::get(c"NSURL") else {
                return;
            };
            let url: Option<Retained<AnyObject>> = msg_send![url_class, URLWithString: &*text];
            let Some(url) = url else {
                self.delegate.ivars().events.navigation_completed(
                    0,
                    false,
                    CoreWebView2WebErrorStatus::Unknown,
                );
                return;
            };
            let Some(request_class) = AnyClass::get(c"NSURLRequest") else {
                return;
            };
            let request: Retained<AnyObject> = msg_send![request_class, requestWithURL: &*url];
            self.delegate.ivars().is_provisional.set(false);
            let _: Option<Retained<AnyObject>> = msg_send![&*self.web_view, loadRequest: &*request];
        }
    }

    fn navigate_to_string(&self, html: &str) {
        let text = ns_string(html);
        let base: Option<&AnyObject> = None;
        self.delegate.ivars().is_provisional.set(false);
        // SAFETY: the base address of this method is nullable.
        unsafe {
            let _: Option<Retained<AnyObject>> =
                msg_send![&*self.web_view, loadHTMLString: &*text, baseURL: base];
        }
    }

    fn reload(&self) {
        self.delegate.ivars().is_provisional.set(false);
        // SAFETY: a message without arguments to a live web view.
        unsafe {
            let _: Option<Retained<AnyObject>> = msg_send![&*self.web_view, reload];
        }
    }

    fn stop(&self) {
        // SAFETY: a message without arguments to a live web view.
        unsafe {
            let _: () = msg_send![&*self.web_view, stopLoading];
        }
    }

    fn go_back(&self) {
        self.delegate.ivars().is_provisional.set(false);
        // SAFETY: a message without arguments to a live web view.
        unsafe {
            let _: Option<Retained<AnyObject>> = msg_send![&*self.web_view, goBack];
        }
    }

    fn go_forward(&self) {
        self.delegate.ivars().is_provisional.set(false);
        // SAFETY: a message without arguments to a live web view.
        unsafe {
            let _: Option<Retained<AnyObject>> = msg_send![&*self.web_view, goForward];
        }
    }

    fn execute_script(&self, script: &str, completion: ScriptCompletion) {
        self.evaluate(&bridge::result_script(script), Some(completion));
    }

    fn post_web_message_as_json(&self, json: &str) {
        self.evaluate(&bridge::deliver_script(json), None);
    }
}

impl Drop for AppleEngine {
    fn drop(&mut self) {
        self.close();
    }
}

/// Creates WKWebView engines.
pub(super) struct AppleEngineFactory;

impl IWebViewEngineFactory for AppleEngineFactory {
    fn create(
        &self,
        _parent: Rc<dyn IPlatformHandle>,
        events: Rc<dyn IWebViewEngineEvents>,
    ) -> Result<Rc<dyn IWebViewEngine>, WebViewError> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            WebViewError::Unavailable("a web view is created on the main thread".to_owned())
        })?;
        // SAFETY: the path is a C string; the framework stays loaded, as with a linked one.
        if unsafe { libc::dlopen(WEBKIT_PATH.as_ptr(), libc::RTLD_LAZY) }.is_null() {
            return Err(WebViewError::Unavailable(
                "WebKit could not be loaded".to_owned(),
            ));
        }
        let web_view_class = class(c"WKWebView")?;
        let script_class = class(c"WKUserScript")?;

        let delegate = Delegate::new(mtm, events.clone());
        let is_closed = Rc::new(Cell::new(false));
        // SAFETY: the classes come from WebKit and every message matches its declaration there.
        let (web_view, controller) = unsafe {
            let web_view: Retained<AnyObject> = msg_send![web_view_class, new];
            let configuration: Retained<AnyObject> = msg_send![&*web_view, configuration];
            let controller: Retained<AnyObject> = msg_send![&*configuration, userContentController];

            let post = format!("function(text){{window.webkit.messageHandlers.{MESSAGE_HANDLER}.postMessage(text);}}");
            let source = ns_string(&bridge::document_script(&post));
            let allocated: objc2::rc::Allocated<AnyObject> = msg_send![script_class, alloc];
            let script: Retained<AnyObject> = msg_send![
                allocated,
                initWithSource: &*source,
                injectionTime: USER_SCRIPT_AT_START,
                forMainFrameOnly: false
            ];
            let _: () = msg_send![&*controller, addUserScript: &*script];
            let name = ns_string(MESSAGE_HANDLER);
            let handler: &AnyObject = delegate.as_ref();
            let _: () = msg_send![&*controller, addScriptMessageHandler: handler, name: &*name];
            let _: () = msg_send![&*web_view, setNavigationDelegate: handler];
            for key in OBSERVED_KEYS {
                let key = ns_string(key);
                let _: () = msg_send![
                    &*web_view,
                    addObserver: handler,
                    forKeyPath: &*key,
                    options: KVO_NEW,
                    context: std::ptr::null_mut::<c_void>()
                ];
            }
            (web_view, controller)
        };

        let engine = Rc::new(AppleEngine {
            web_view,
            controller,
            delegate,
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
