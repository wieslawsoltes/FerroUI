//! WKWebView in a real run loop, without a window or the network.
//!
//! Runs only with `FERROUI_WEBVIEW_ENGINE_TEST=1`: it starts WebKit's helper processes.

#[cfg(not(target_vendor = "apple"))]
fn main() {}

#[cfg(target_vendor = "apple")]
fn main() {
    if std::env::var_os("FERROUI_WEBVIEW_ENGINE_TEST").is_none() {
        println!("skipped: set FERROUI_WEBVIEW_ENGINE_TEST=1 to start a real engine");
        return;
    }
    apple::run();
    println!("web engine test passed");
}

#[cfg(target_vendor = "apple")]
mod apple {
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::platform::{IPlatformHandle, PlatformHandle};
    use ferroui_controls_webview::engines::default_factory;
    use ferroui_controls_webview::{
        CoreWebView2WebErrorStatus, IWebViewEngineEvents, WebViewError, WebViewNavigation,
    };
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopDefaultMode: *const c_void;
        fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source: u8) -> i32;
    }

    #[derive(Default)]
    struct Recorder {
        initialized: Cell<bool>,
        started: RefCell<Vec<String>>,
        completed: RefCell<Vec<bool>>,
        title: RefCell<String>,
        messages: RefCell<Vec<String>>,
    }

    impl IWebViewEngineEvents for Recorder {
        fn initialized(&self, result: Result<(), WebViewError>) {
            result.expect("the engine starts");
            self.initialized.set(true);
        }
        fn navigation_starting(&self, navigation: &WebViewNavigation) -> bool {
            self.started.borrow_mut().push(navigation.uri.clone());
            !navigation.uri.contains("blocked")
        }
        fn navigation_completed(
            &self,
            _id: u64,
            is_success: bool,
            _status: CoreWebView2WebErrorStatus,
        ) {
            self.completed.borrow_mut().push(is_success);
        }
        fn source_changed(&self, _uri: &str) {}
        fn history_changed(&self, _can_go_back: bool, _can_go_forward: bool) {}
        fn document_title_changed(&self, title: &str) {
            *self.title.borrow_mut() = title.to_owned();
        }
        fn web_message_received(&self, _source: &str, json: &str) {
            self.messages.borrow_mut().push(json.to_owned());
        }
    }

    fn pump_until(what: &str, done: impl Fn() -> bool) {
        let start = Instant::now();
        while !done() {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "timed out waiting for {what}"
            );
            // SAFETY: the default mode of the run loop of the main thread.
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.02, 0) };
            Dispatcher::ui_thread().run_jobs(None);
        }
    }

    const PAGE: &str = "<html><head><title>Engine page</title></head><body><script>\
        chrome.webview.addEventListener('message',function(e){chrome.webview.postMessage({echo:e.data});});\
        chrome.webview.postMessage('ready');\
        </script></body></html>";

    pub fn run() {
        let recorder = Rc::new(Recorder::default());
        let parent: Rc<dyn IPlatformHandle> = Rc::new(PlatformHandle::new(0, Some("NSView")));
        let engine = default_factory()
            .expect("this platform has an engine")
            .create(parent, recorder.clone())
            .expect("the engine is created");
        assert!(
            !recorder.initialized.get(),
            "initialized is reported after create returned"
        );
        assert_ne!(engine.native_control().handle(), 0);
        pump_until("initialized", || recorder.initialized.get());

        engine.navigate_to_string(PAGE);
        pump_until("the page", || recorder.completed.borrow().contains(&true));
        pump_until("the first message", || {
            recorder.messages.borrow().iter().any(|m| m == "\"ready\"")
        });
        pump_until("the title", || {
            recorder.title.borrow().as_str() == "Engine page"
        });

        let script = |text: &str| {
            let out = Rc::new(RefCell::new(None));
            let store = out.clone();
            engine.execute_script(
                text,
                Box::new(move |result| *store.borrow_mut() = Some(result)),
            );
            pump_until(text, || out.borrow().is_some());
            let value = out.borrow_mut().take().unwrap();
            value
        };
        assert_eq!(script("1 + 2"), Ok("3".to_owned()));
        assert_eq!(script("({a: 'b'})"), Ok("{\"a\":\"b\"}".to_owned()));
        assert_eq!(script("undefined"), Ok("null".to_owned()));
        assert_eq!(
            script("var engineGlobal = 5; engineGlobal"),
            Ok("5".to_owned())
        );
        assert_eq!(
            script("window.engineGlobal"),
            Ok("5".to_owned()),
            "scripts run in the global scope"
        );
        assert!(matches!(
            script("throw new Error('boom')"),
            Err(WebViewError::Engine(_))
        ));

        engine.post_web_message_as_json("{\"x\":1}");
        pump_until("the echo", || {
            recorder
                .messages
                .borrow()
                .iter()
                .any(|m| m == "{\"echo\":{\"x\":1}}")
        });

        let completed = recorder.completed.borrow().len();
        engine.navigate("https://blocked.invalid/");
        pump_until("the canceled navigation", || {
            recorder
                .started
                .borrow()
                .iter()
                .any(|uri| uri.contains("blocked"))
        });
        assert_eq!(
            script("document.title"),
            Ok("\"Engine page\"".to_owned()),
            "the page stays"
        );
        assert_eq!(recorder.completed.borrow().len(), completed);

        engine.close();
        engine.close();
    }
}
