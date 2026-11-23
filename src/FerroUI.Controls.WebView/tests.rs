//! The control against an engine that records what it is asked.

use crate::*;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroLocator, Rect, Ref, Size};
use ferroui_controls::platform::{
    INativeControlHostControlTopLevelAttachment, INativeControlHostDestroyableControlHandle,
    INativeControlHostImpl, IPlatformHandle,
};
use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use ferroui_controls::{Control, Window};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[derive(Default)]
struct Handle;

impl IPlatformHandle for Handle {
    fn handle(&self) -> isize {
        1
    }
    fn handle_descriptor(&self) -> Option<&str> {
        Some("test")
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

impl INativeControlHostDestroyableControlHandle for Handle {
    fn destroy(&self) {}
}

#[derive(Default)]
struct Attachment {
    attached_to: RefCell<Option<Rc<dyn INativeControlHostImpl>>>,
}

impl IDisposable for Attachment {
    fn dispose(&self) {
        *self.attached_to.borrow_mut() = None;
    }
}

impl INativeControlHostControlTopLevelAttachment for Attachment {
    fn attached_to(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
        self.attached_to.borrow().clone()
    }
    fn set_attached_to(&self, value: Option<Rc<dyn INativeControlHostImpl>>) {
        *self.attached_to.borrow_mut() = value;
    }
    fn is_compatible_with(&self, _host: &dyn INativeControlHostImpl) -> bool {
        true
    }
    fn hide_with_size(&self, _size: Size) {}
    fn show_in_bounds(&self, _rect: Rect) {}
}

struct Host {
    this: Weak<Host>,
}

impl Host {
    fn attachment(&self) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        let host: Rc<dyn INativeControlHostImpl> = self.this.upgrade().unwrap();
        Rc::new(Attachment {
            attached_to: RefCell::new(Some(host)),
        })
    }
}

impl INativeControlHostImpl for Host {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn create_default_child(
        &self,
        _parent: Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn INativeControlHostDestroyableControlHandle> {
        Rc::new(Handle)
    }
    fn create_new_attachment_with(
        &self,
        create: Rc<dyn Fn(Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>>,
    ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        create(Rc::new(Handle));
        self.attachment()
    }
    fn create_new_attachment(
        &self,
        _handle: Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        self.attachment()
    }
    fn is_compatible_with(&self, _handle: &dyn IPlatformHandle) -> bool {
        true
    }
}

type Log = Rc<RefCell<Vec<String>>>;

struct FakeEngine {
    log: Log,
    events: Rc<dyn IWebViewEngineEvents>,
    scripts: RefCell<Vec<ScriptCompletion>>,
    on_navigate: RefCell<Option<Box<dyn Fn()>>>,
}

impl IWebViewEngine for FakeEngine {
    fn native_control(&self) -> Rc<dyn IPlatformHandle> {
        Rc::new(Handle)
    }
    fn close(&self) {
        self.log.borrow_mut().push("close".into());
    }
    fn navigate(&self, uri: &str) {
        self.log.borrow_mut().push(format!("navigate {uri}"));
        if let Some(hook) = self.on_navigate.borrow().as_ref() {
            hook();
        }
    }
    fn navigate_to_string(&self, html: &str) {
        self.log.borrow_mut().push(format!("html {html}"));
    }
    fn reload(&self) {
        self.log.borrow_mut().push("reload".into());
    }
    fn stop(&self) {
        self.log.borrow_mut().push("stop".into());
    }
    fn go_back(&self) {
        self.log.borrow_mut().push("back".into());
    }
    fn go_forward(&self) {
        self.log.borrow_mut().push("forward".into());
    }
    fn execute_script(&self, script: &str, completion: ScriptCompletion) {
        self.log.borrow_mut().push(format!("script {script}"));
        self.scripts.borrow_mut().push(completion);
    }
    fn post_web_message_as_json(&self, json: &str) {
        self.log.borrow_mut().push(format!("post {json}"));
    }
}

#[derive(Default)]
struct Factory {
    log: Log,
    engines: RefCell<Vec<Rc<FakeEngine>>>,
    failure: RefCell<Option<WebViewError>>,
}

impl IWebViewEngineFactory for Factory {
    fn create(
        &self,
        _parent: Rc<dyn IPlatformHandle>,
        events: Rc<dyn IWebViewEngineEvents>,
    ) -> Result<Rc<dyn IWebViewEngine>, WebViewError> {
        if let Some(error) = self.failure.borrow().clone() {
            return Err(error);
        }
        let engine = Rc::new(FakeEngine {
            log: self.log.clone(),
            events,
            scripts: RefCell::default(),
            on_navigate: RefCell::default(),
        });
        self.engines.borrow_mut().push(engine.clone());
        Ok(engine)
    }
}

impl Factory {
    fn engine(&self) -> Rc<FakeEngine> {
        self.engines.borrow().last().unwrap().clone()
    }

    fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

struct Scene {
    _scope: Rc<dyn IDisposable>,
    _app: Box<dyn Any>,
    factory: Rc<Factory>,
    window: Ref<Window>,
}

fn scene() -> Scene {
    let app = UnitTestApplication::start(TestServices::styled_window());
    let scope = FerroLocator::enter_scope();
    let factory = Rc::new(Factory::default());
    FerroLocator::current_mutable()
        .bind::<dyn IWebViewEngineFactory>()
        .to_constant(factory.clone() as Rc<dyn IWebViewEngineFactory>);
    let window_impl = MockWindowingPlatform::create_window_mock();
    let host: Rc<Host> = Rc::new_cyclic(|this| Host { this: this.clone() });
    window_impl.setup_feature::<dyn INativeControlHostImpl>(host);
    let window = Window::with_impl(window_impl);
    window.set_width(200.0);
    window.set_height(100.0);
    window.show();
    Scene {
        _scope: scope,
        _app: Box::new(app),
        factory,
        window,
    }
}

fn attach(scene: &Scene, view: &Ref<WebView2>) {
    scene.window.set_content(Some(Control::boxed(view.clone())));
    scene.window.update_layout();
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
}

fn detach(scene: &Scene) {
    scene.window.set_content(None);
    scene.window.update_layout();
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn a_source_set_before_the_engine_is_shown_once_it_is_ready() {
    let scene = scene();
    let view = WebView2::new();
    view.set_source(Some("https://example.test/first".into()));
    view.set_source(Some("https://example.test/second".into()));
    attach(&scene, &view);
    assert!(
        scene.factory.take_log().is_empty(),
        "nothing is asked before the engine is ready"
    );
    assert!(view.core_web_view2().is_none());

    scene.factory.engine().events.initialized(Ok(()));
    assert_eq!(
        scene.factory.take_log(),
        ["navigate https://example.test/second"]
    );
    assert!(view.core_web_view2().is_some());
}

#[test]
fn commands_wait_for_the_engine_in_order_and_the_last_document_wins() {
    let scene = scene();
    let view = WebView2::new();
    attach(&scene, &view);
    view.navigate_to_string("<p>old</p>");
    view.post_web_message_as_string("say \"hi\"");
    view.reload();
    view.navigate_to_string("<p>new</p>");
    let script = view.execute_script_async("1 + 1");
    scene.factory.engine().events.initialized(Ok(()));
    assert_eq!(
        scene.factory.take_log(),
        [
            "post \"say \\\"hi\\\"\"",
            "reload",
            "html <p>new</p>",
            "script 1 + 1"
        ]
    );
    let completion = scene.factory.engine().scripts.borrow_mut().remove(0);
    completion(Ok("2".into()));
    assert_eq!(script.outcome().unwrap().unwrap(), Ok("2".to_owned()));
}

#[test]
fn ensure_answers_when_the_engine_is_ready_and_after() {
    let scene = scene();
    let view = WebView2::new();
    let raised = Rc::new(Cell::new(0));
    let count = raised.clone();
    view.core_web_view2_initialized().add(Rc::new(
        move |args: &CoreWebView2InitializedEventArgs| {
            assert_eq!(args.exception, None);
            count.set(count.get() + 1);
        },
    ));
    let task = view.ensure_core_web_view2_async();
    attach(&scene, &view);
    assert!(!task.is_finished());
    scene.factory.engine().events.initialized(Ok(()));
    let core = task.outcome().unwrap().unwrap().unwrap();
    assert_eq!(raised.get(), 1);
    let again = view
        .ensure_core_web_view2_async()
        .outcome()
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(Rc::ptr_eq(&core, &again));
}

#[test]
fn an_engine_that_cannot_start_answers_everything_that_waits() {
    let scene = scene();
    let error = WebViewError::Unavailable("no runtime".into());
    *scene.factory.failure.borrow_mut() = Some(error.clone());
    let view = WebView2::new();
    let ensure = view.ensure_core_web_view2_async();
    let script = view.execute_script_async("1");
    attach(&scene, &view);
    assert_eq!(ensure.outcome().unwrap().unwrap().err(), Some(error));
    assert_eq!(
        script.outcome().unwrap().unwrap(),
        Err(WebViewError::Closed)
    );
    assert!(
        view.native_control_handle().is_some(),
        "the control keeps a native child"
    );
}

#[test]
fn an_engine_that_fails_later_answers_too() {
    let scene = scene();
    let view = WebView2::new();
    let ensure = view.ensure_core_web_view2_async();
    attach(&scene, &view);
    let error = WebViewError::Engine("the runtime failed".into());
    scene
        .factory
        .engine()
        .events
        .initialized(Err(error.clone()));
    assert_eq!(ensure.outcome().unwrap().unwrap().err(), Some(error));
    assert!(view.core_web_view2().is_none());
}

#[test]
fn after_a_failed_start_nothing_waits_and_the_next_attachment_tries_again() {
    let scene = scene();
    let error = WebViewError::Unavailable("no runtime".into());
    *scene.factory.failure.borrow_mut() = Some(error.clone());
    let view = WebView2::new();
    view.set_source(Some("https://example.test/page".into()));
    attach(&scene, &view);
    assert_eq!(
        view.ensure_core_web_view2_async()
            .outcome()
            .unwrap()
            .unwrap()
            .err(),
        Some(error.clone())
    );
    assert_eq!(
        view.execute_script_async("1").outcome().unwrap().unwrap(),
        Err(error)
    );

    detach(&scene);
    *scene.factory.failure.borrow_mut() = None;
    let ensure = view.ensure_core_web_view2_async();
    attach(&scene, &view);
    scene.factory.engine().events.initialized(Ok(()));
    assert!(ensure.outcome().unwrap().unwrap().is_ok());
    assert_eq!(
        scene.factory.take_log(),
        ["navigate https://example.test/page"]
    );
}

#[test]
fn an_engine_can_call_back_into_the_control_while_commands_replay() {
    let scene = scene();
    let view = WebView2::new();
    view.set_source(Some("https://example.test/page".into()));
    view.stop();
    attach(&scene, &view);
    let weak = view.downgrade();
    *scene.factory.engine().on_navigate.borrow_mut() = Some(Box::new(move || {
        if let Some(view) = weak.upgrade() {
            view.reload();
        }
    }));
    scene.factory.engine().events.initialized(Ok(()));
    assert_eq!(
        scene.factory.take_log(),
        ["navigate https://example.test/page", "reload", "stop"]
    );
}

#[test]
fn a_handler_can_cancel_a_navigation() {
    let scene = scene();
    let view = WebView2::new();
    attach(&scene, &view);
    let events = scene.factory.engine().events.clone();
    events.initialized(Ok(()));
    view.navigation_starting()
        .add(Rc::new(|args: &NavigationStartingEventArgs| {
            args.set_cancel(args.uri.contains("blocked"));
        }));
    let navigation = |uri: &str| WebViewNavigation {
        id: 7,
        uri: uri.into(),
        is_user_initiated: true,
        is_redirected: false,
    };
    assert!(events.navigation_starting(&navigation("https://example.test/ok")));
    assert!(!events.navigation_starting(&navigation("https://example.test/blocked")));
}

#[test]
fn the_engine_reports_the_address_without_a_new_navigation() {
    let scene = scene();
    let view = WebView2::new();
    attach(&scene, &view);
    let events = scene.factory.engine().events.clone();
    events.initialized(Ok(()));
    scene.factory.take_log();
    events.source_changed("https://example.test/link");
    assert_eq!(view.source().as_deref(), Some("https://example.test/link"));
    assert_eq!(
        view.core_web_view2().unwrap().source(),
        "https://example.test/link"
    );
    assert!(scene.factory.take_log().is_empty());

    view.set_source(Some("https://example.test/typed".into()));
    assert_eq!(
        scene.factory.take_log(),
        ["navigate https://example.test/typed"]
    );
}

#[test]
fn the_history_and_the_title_follow_the_engine() {
    let scene = scene();
    let view = WebView2::new();
    attach(&scene, &view);
    let events = scene.factory.engine().events.clone();
    events.initialized(Ok(()));
    events.history_changed(true, false);
    assert!(view.can_go_back());
    assert!(!view.can_go_forward());
    events.document_title_changed("A page");
    assert_eq!(view.core_web_view2().unwrap().document_title(), "A page");
    view.go_back();
    view.go_forward();
    view.stop();
    assert_eq!(scene.factory.take_log(), ["back", "forward", "stop"]);
}

#[test]
fn messages_of_the_page_arrive_as_json_and_as_strings() {
    let scene = scene();
    let view = WebView2::new();
    attach(&scene, &view);
    let events = scene.factory.engine().events.clone();
    events.initialized(Ok(()));
    let received = Rc::new(RefCell::new(Vec::new()));
    let store = received.clone();
    view.web_message_received()
        .add(Rc::new(move |args: &WebMessageReceivedEventArgs| {
            store.borrow_mut().push((
                args.web_message_as_json.clone(),
                args.try_get_web_message_as_string(),
            ));
        }));
    events.web_message_received("https://example.test/", "\"a\\n\\u00e9\\ud83d\\ude00\"");
    events.web_message_received("https://example.test/", "{\"a\":1}");
    let received = received.borrow();
    assert_eq!(received[0].1.as_deref(), Some("a\n\u{e9}\u{1F600}"));
    assert_eq!(received[1].1, None);
}

#[test]
fn detaching_closes_the_engine_and_a_new_engine_shows_the_document_again() {
    let scene = scene();
    let view = WebView2::new();
    view.set_source(Some("https://example.test/page".into()));
    attach(&scene, &view);
    let first = scene.factory.engine();
    first.events.initialized(Ok(()));
    first.events.history_changed(true, true);
    let core = view.core_web_view2().unwrap();
    let script = view.execute_script_async("slow()");
    scene.factory.take_log();

    detach(&scene);
    assert_eq!(scene.factory.take_log(), ["close"]);
    assert_eq!(
        script.outcome().unwrap().unwrap(),
        Err(WebViewError::Closed)
    );
    assert!(view.core_web_view2().is_none());
    assert!(!view.can_go_back());
    core.navigate("https://example.test/late");
    assert_eq!(
        core.execute_script_async("late()")
            .outcome()
            .unwrap()
            .unwrap(),
        Err(WebViewError::Closed)
    );
    assert!(
        scene.factory.take_log().is_empty(),
        "a closed engine is asked nothing"
    );

    attach(&scene, &view);
    scene.factory.engine().events.initialized(Ok(()));
    assert_eq!(
        scene.factory.take_log(),
        ["navigate https://example.test/page"]
    );
    assert!(!Rc::ptr_eq(&core, &view.core_web_view2().unwrap()));
}

#[test]
fn an_engine_of_a_released_control_is_ignored() {
    let scene = scene();
    let events = {
        let view = WebView2::new();
        attach(&scene, &view);
        let events = scene.factory.engine().events.clone();
        detach(&scene);
        events
    };
    events.initialized(Ok(()));
    events.source_changed("https://example.test/");
    let navigation = WebViewNavigation {
        id: 1,
        uri: "x".into(),
        is_user_initiated: false,
        is_redirected: false,
    };
    assert!(
        !events.navigation_starting(&navigation),
        "with no control the navigation is refused"
    );
}

#[test]
fn json_strings_round_trip() {
    for text in [
        "",
        "plain",
        "\"\\/\n\r\t\u{8}\u{c}",
        "\u{1}\u{1f}",
        "\u{2028}\u{2029}",
        "\u{e9}\u{1F600}",
    ] {
        assert_eq!(
            crate::json::parse_string(&crate::json::quote(text)).as_deref(),
            Some(text)
        );
    }
    for bad in ["1", "{}", "\"", "\"\\x\"", "\"\\ud83d\"", "\"a\"b\""] {
        assert_eq!(crate::json::parse_string(bad), None, "{bad}");
    }
}
