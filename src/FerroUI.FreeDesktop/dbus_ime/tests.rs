//! The input methods against services that answer as IBus and Fcitx do.
//!
//! Not from the reference, which has no tests of these classes.
//!
//! The services are doubles on a second connection (`test_support.rs`:
//! without a bus by default, on a session bus when the environment asks).

use super::fcitx::{FcitxCapabilityFlags, FcitxKeyState, FcitxX11TextInputMethod};
use super::ibus::{IBusCapability, IBusModifierMask, IBusX11TextInputMethod};
use super::DBusTextInputMethodBase;
use crate::ix11_input_method::{IX11InputMethodControl, X11InputMethodForwardedKey};
use crate::test_support::{emit_from_service, log, pump_until, scope, Log, ServiceBuilder, TestConnections};
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType};
use ferroui_base::input::text_input::{
    ITextInputMethodImpl, TextInputContentType, TextInputMethodClient, TextInputMethodClientEvents, TextInputOptions,
    TextSelection,
};
use ferroui_base::input::{
    FocusManager, IInputDevice, IInputRoot, InputElement, InputScopeNameValue, Key, KeyDeviceType, KeyModifiers,
    PhysicalKey, RawInputModifiers,
};
use ferroui_base::{PixelPoint, Rect, Ref, Visual};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, Structure, Value};

const IBUS_NAME: &str = "org.freedesktop.portal.IBus";
const IBUS_CONTEXT_PATH: &str = "/org/freedesktop/IBus/InputContext_1";
const FCITX4_NAME: &str = "org.fcitx.Fcitx";
const FCITX4_CONTEXT_PATH: &str = "/inputcontext_7";
const FCITX5_NAME: &str = "org.freedesktop.portal.Fcitx";
const FCITX5_CONTEXT_PATH: &str = "/org/freedesktop/portal/inputcontext/3";

/// The key symbols the doubles treat specially.
const KEY_A: u32 = 0x61;
const KEY_FAILS: u32 = 0x7a;

fn ibus_text(text: &str) -> Value<'static> {
    let attachments: HashMap<String, Value<'static>> = HashMap::new();
    Value::Structure(Structure::from((
        "IBusText".to_string(),
        attachments,
        text.to_string(),
        Value::from(0u32),
    )))
}

// ---------------------------------------------------------------------
// IBus.

/// A call of a double that a test holds until it releases it.
#[derive(Default)]
struct DeferredCall {
    state: Mutex<(bool, Option<Waker>)>,
}

impl DeferredCall {
    fn hold(&self) {
        self.state.lock().unwrap().0 = true;
    }

    fn release(&self) {
        let waker = {
            let mut state = self.state.lock().unwrap();
            state.0 = false;
            state.1.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    async fn wait(&self) {
        futures_util::future::poll_fn(|cx| {
            let mut state = self.state.lock().unwrap();
            if state.0 {
                state.1 = Some(cx.waker().clone());
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .await;
    }
}

/// What a test asks of the IBus double: to hold the creation of a context or the reply to
/// `ContentType`, to refuse the next `ContentType`, to commit text while setting it.
#[derive(Default)]
struct IBusCalls {
    create: DeferredCall,
    content_type: DeferredCall,
    refuse_next_content_type: AtomicBool,
    commit_in_next_content_type: AtomicBool,
}

struct IBusPortal {
    log: Log,
    calls: Arc<IBusCalls>,
}

#[zbus::interface(name = "org.freedesktop.IBus.Portal")]
impl IBusPortal {
    async fn create_input_context(&self, client_name: &str) -> OwnedObjectPath {
        log(&self.log, format!("CreateInputContext({})", if client_name.is_empty() { "empty" } else { "name" }));
        self.calls.create.wait().await;
        ObjectPath::try_from(IBUS_CONTEXT_PATH).unwrap().into()
    }
}

struct IBusContext {
    log: Log,
    calls: Arc<IBusCalls>,
}

#[zbus::interface(name = "org.freedesktop.IBus.InputContext")]
impl IBusContext {
    async fn process_key_event(
        &self,
        keyval: u32,
        keycode: u32,
        state: u32,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> zbus::fdo::Result<bool> {
        log(&self.log, format!("ProcessKeyEvent({keyval:#x}, {keycode}, {state:#x})"));
        if keyval == KEY_FAILS {
            return Err(zbus::fdo::Error::Failed("the engine is gone".to_string()));
        }
        if keyval == KEY_A && state & IBusModifierMask::RELEASE_MASK.bits() == 0 {
            // The key is consumed and becomes committed text.
            emit_from_service(connection, IBUS_CONTEXT_PATH, "org.freedesktop.IBus.InputContext", "CommitText", &(ibus_text("あ"),)).await;
            return Ok(true);
        }
        Ok(false)
    }

    fn set_cursor_location(&self, x: i32, y: i32, w: i32, h: i32) {
        log(&self.log, format!("SetCursorLocation({x}, {y}, {w}, {h})"));
    }

    fn focus_in(&self) {
        log(&self.log, "FocusIn");
    }

    fn focus_out(&self) {
        log(&self.log, "FocusOut");
    }

    async fn reset(&self, #[zbus(connection)] connection: &zbus::Connection) {
        log(&self.log, "Reset");
        // What IBus does: it commits the pre-edit while it is being reset.
        emit_from_service(connection, IBUS_CONTEXT_PATH, "org.freedesktop.IBus.InputContext", "CommitText", &(ibus_text("left over"),)).await;
    }

    fn set_capabilities(&self, caps: u32) {
        log(&self.log, format!("SetCapabilities({caps})"));
    }

    // A write-only (uu) property, as IBus has it: a call of a method instead would not reach it.
    #[zbus(property)]
    async fn set_content_type(
        &self,
        value: (u32, u32),
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> zbus::fdo::Result<()> {
        log(&self.log, format!("SetContentType({}, {})", value.0, value.1));
        let refuse = self.calls.refuse_next_content_type.swap(false, Ordering::SeqCst);
        if self.calls.commit_in_next_content_type.swap(false, Ordering::SeqCst) {
            emit_from_service(connection, IBUS_CONTEXT_PATH, "org.freedesktop.IBus.InputContext", "CommitText", &(ibus_text("while setting"),)).await;
        }
        self.calls.content_type.wait().await;
        if refuse {
            Err(zbus::fdo::Error::UnknownProperty("ContentType".to_string()))
        } else {
            Ok(())
        }
    }
}

struct IBusService {
    log: Log,
}

#[zbus::interface(name = "org.freedesktop.IBus.Service")]
impl IBusService {
    fn destroy(&self) {
        log(&self.log, "Destroy");
    }
}

// ---------------------------------------------------------------------
// Fcitx 4 and 5.

struct Fcitx4Method {
    log: Log,
}

#[zbus::interface(name = "org.fcitx.Fcitx.InputMethod")]
impl Fcitx4Method {
    #[zbus(name = "CreateICv3")]
    fn create_icv3(&self, appname: &str, pid: i32) -> (i32, bool, u32, u32, u32, u32) {
        let own_pid = pid == std::process::id() as i32;
        log(&self.log, format!("CreateICv3(name: {}, own pid: {own_pid})", !appname.is_empty()));
        (7, true, 0, 0, 0, 0)
    }
}

struct Fcitx4Context {
    log: Log,
}

#[zbus::interface(name = "org.fcitx.Fcitx.InputContext")]
impl Fcitx4Context {
    fn focus_in(&self) {
        log(&self.log, "FocusIn");
    }

    fn focus_out(&self) {
        log(&self.log, "FocusOut");
    }

    fn reset(&self) {
        log(&self.log, "Reset");
    }

    fn set_cursor_rect(&self, x: i32, y: i32, w: i32, h: i32) {
        log(&self.log, format!("SetCursorRect({x}, {y}, {w}, {h})"));
    }

    fn set_capacity(&self, caps: u32) {
        log(&self.log, format!("SetCapacity({caps})"));
    }

    #[zbus(name = "DestroyIC")]
    fn destroy_ic(&self) {
        log(&self.log, "DestroyIC");
    }

    async fn process_key_event(
        &self,
        keyval: u32,
        keycode: u32,
        state: u32,
        type_: i32,
        time: u32,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> i32 {
        log(&self.log, format!("ProcessKeyEvent({keyval:#x}, {keycode}, {state}, {type_}, {time})"));
        if keyval == KEY_A && type_ == 0 {
            emit_from_service(connection, FCITX4_CONTEXT_PATH, "org.fcitx.Fcitx.InputContext", "CommitString", &("四",)).await;
            return 1;
        }
        0
    }
}

struct Fcitx5Method {
    log: Log,
}

#[zbus::interface(name = "org.fcitx.Fcitx.InputMethod1")]
impl Fcitx5Method {
    fn create_input_context(&self, arg0: Vec<(String, String)>) -> (OwnedObjectPath, Vec<u8>) {
        let keys: Vec<&str> = arg0.iter().map(|(key, _)| key.as_str()).collect();
        log(&self.log, format!("CreateInputContext({})", keys.join(", ")));
        (ObjectPath::try_from(FCITX5_CONTEXT_PATH).unwrap().into(), vec![1, 2, 3])
    }
}

struct Fcitx5Context {
    log: Log,
}

#[zbus::interface(name = "org.fcitx.Fcitx.InputContext1")]
impl Fcitx5Context {
    fn focus_in(&self) {
        log(&self.log, "FocusIn");
    }

    fn focus_out(&self) {
        log(&self.log, "FocusOut");
    }

    fn reset(&self) {
        log(&self.log, "Reset");
    }

    fn set_cursor_rect(&self, x: i32, y: i32, w: i32, h: i32) {
        log(&self.log, format!("SetCursorRect({x}, {y}, {w}, {h})"));
    }

    fn set_capability(&self, caps: u64) {
        log(&self.log, format!("SetCapability({caps})"));
    }

    #[zbus(name = "DestroyIC")]
    fn destroy_ic(&self) {
        log(&self.log, "DestroyIC");
    }

    async fn process_key_event(
        &self,
        keyval: u32,
        keycode: u32,
        state: u32,
        type_: bool,
        time: u32,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> bool {
        log(&self.log, format!("ProcessKeyEvent({keyval:#x}, {keycode}, {state}, {type_}, {time})"));
        if keyval == KEY_A && !type_ {
            emit_from_service(connection, FCITX5_CONTEXT_PATH, "org.fcitx.Fcitx.InputContext1", "CommitString", &("五",)).await;
            return true;
        }
        false
    }
}

// ---------------------------------------------------------------------
// The two connections.

struct TestBus {
    connections: TestConnections,
    log: Log,
    ibus_calls: Arc<IBusCalls>,
}

impl std::ops::Deref for TestBus {
    type Target = TestConnections;

    fn deref(&self) -> &TestConnections {
        &self.connections
    }
}

impl TestBus {
    fn new() -> TestBus {
        let log: Log = Arc::new(Mutex::new(Vec::new()));
        let ibus_calls = Arc::new(IBusCalls::default());

        let serve = {
            let log = log.clone();
            let calls = ibus_calls.clone();
            move |builder: ServiceBuilder| {
                builder
                    .serve_at("/org/freedesktop/IBus", IBusPortal { log: log.clone(), calls: calls.clone() })?
                    .serve_at(IBUS_CONTEXT_PATH, IBusContext { log: log.clone(), calls })?
                    .serve_at(IBUS_CONTEXT_PATH, IBusService { log: log.clone() })?
                    .serve_at("/inputmethod", Fcitx4Method { log: log.clone() })?
                    .serve_at(FCITX4_CONTEXT_PATH, Fcitx4Context { log: log.clone() })?
                    .serve_at("/inputmethod", Fcitx5Method { log: log.clone() })?
                    .serve_at(FCITX5_CONTEXT_PATH, Fcitx5Context { log: log.clone() })
            }
        };

        TestBus { connections: TestConnections::new("/org/freedesktop/IBus", serve), log, ibus_calls }
    }

    fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.lock().unwrap())
    }

    /// Runs the dispatcher until the services were called with `entries`,
    /// in that order and nothing else, and forgets those calls.
    fn expect_calls(&self, entries: &[&str]) {
        crate::test_support::expect_calls(&self.connections, &self.log, entries);
    }
}

// ---------------------------------------------------------------------
// The client of the input method, and key events.

#[derive(Default)]
struct TestClient {
    events: TextInputMethodClientEvents,
    supports_preedit: bool,
    preedit: RefCell<Vec<(Option<String>, Option<i32>)>>,
}

impl TestClient {
    fn new(supports_preedit: bool) -> Rc<TestClient> {
        Rc::new(TestClient { supports_preedit, ..Default::default() })
    }

    fn take_preedit(&self) -> Vec<(Option<String>, Option<i32>)> {
        std::mem::take(&mut *self.preedit.borrow_mut())
    }
}

impl TextInputMethodClient for TestClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }

    fn text_view_visual(&self) -> Option<Ref<Visual>> {
        None
    }

    fn supports_preedit(&self) -> bool {
        self.supports_preedit
    }

    fn supports_surrounding_text(&self) -> bool {
        false
    }

    fn surrounding_text(&self) -> String {
        String::new()
    }

    fn cursor_rectangle(&self) -> Rect {
        Rect::default()
    }

    fn selection(&self) -> TextSelection {
        TextSelection::default()
    }

    fn set_selection(&self, _value: TextSelection) {}

    fn set_preedit_text_with_cursor(&self, preedit_text: Option<&str>, cursor_pos: Option<i32>) {
        self.preedit.borrow_mut().push((preedit_text.map(str::to_string), cursor_pos));
    }
}

struct TestDevice;

impl IInputDevice for TestDevice {
    fn process_raw_event(&self, _ev: &dyn IRawInputEventArgs) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct TestRoot;

impl IInputRoot for TestRoot {
    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        None
    }

    fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        None
    }

    fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}

    fn cursor_element(&self) -> Option<Ref<InputElement>> {
        None
    }

    fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}

    fn root_element(&self) -> Ref<InputElement> {
        unreachable!("the input methods do not ask for the root element")
    }

    fn focus_root(&self) -> Ref<InputElement> {
        unreachable!("the input methods do not ask for the focus root")
    }

    fn pointer_over_invalidated(&self) {}
}

fn key(type_: RawKeyEventType, modifiers: RawInputModifiers, timestamp: u64) -> Rc<dyn IRawInputEventArgs> {
    Rc::new(RawKeyEventArgs::new(
        Rc::new(TestDevice),
        timestamp,
        Rc::new(TestRoot),
        type_,
        Key::A,
        modifiers,
        PhysicalKey::A,
        None,
        KeyDeviceType::Keyboard,
    ))
}

/// Offers a key to the input method and waits for its answer.
fn handle(
    im: &Rc<DBusTextInputMethodBase>,
    args: Rc<dyn IRawInputEventArgs>,
    key_val: u32,
    key_code: i32,
) -> bool {
    let answer = Rc::new(RefCell::new(None));
    let handled = im.handle_event_async(args, key_val as i32, key_code);
    {
        let answer = answer.clone();
        drop(ferroui_base::threading::Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            *answer.borrow_mut() = Some(handled.await);
        }));
    }
    pump_until(|| answer.borrow().is_some());
    let answer = answer.borrow_mut().take().unwrap();
    answer
}

struct Recorded {
    commits: Rc<RefCell<Vec<String>>>,
    forwarded: Rc<RefCell<Vec<X11InputMethodForwardedKey>>>,
}

fn record(im: &Rc<DBusTextInputMethodBase>) -> Recorded {
    let recorded = Recorded { commits: Rc::default(), forwarded: Rc::default() };
    {
        let commits = recorded.commits.clone();
        im.commit().subscribe(move |text| commits.borrow_mut().push(text));
    }
    {
        let forwarded = recorded.forwarded.clone();
        im.forward_key().subscribe(move |key| forwarded.borrow_mut().push(key));
    }
    recorded
}

/// Runs the dispatcher until the calls the input method queued returned:
/// the queue runs them in order, so a call queued behind them ends after
/// them.
fn finish_queued_calls(im: &Rc<DBusTextInputMethodBase>) {
    let done = Rc::new(Cell::new(false));
    let flag = done.clone();
    im.enqueue(move || async move {
        flag.set(true);
        Ok(())
    });
    pump_until(|| done.get());
}

fn caps(caps: IBusCapability) -> String {
    format!("SetCapabilities({})", caps.bits())
}

// ---------------------------------------------------------------------
// IBus.

#[test]
fn ibus_connects_when_its_portal_is_on_the_bus_and_follows_focus() {
    let _scope = scope();
    let bus = TestBus::new();
    bus.start(IBUS_NAME);

    let im = IBusX11TextInputMethod::new(bus.client.clone());
    assert!(!im.is_connected());
    assert!(!im.is_enabled());
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    assert!(im.is_connected());
    // Connected, and not enabled before the window and a client are there.
    assert!(!im.is_enabled());

    // An active window without a client is not focus. The first report
    // is made whatever it says: the input method is told "no focus".
    im.set_window_active(true);
    bus.expect_calls(&["FocusOut"]);
    assert!(!im.is_enabled());

    let client = TestClient::new(true);
    im.set_client(Some(client.clone()));
    bus.expect_calls(&["FocusIn", &caps(IBusCapability::CAP_FOCUS | IBusCapability::CAP_PREEDIT_TEXT)]);
    assert!(im.is_enabled());

    // The same state again is not reported again.
    im.set_window_active(true);
    bus.settle();
    assert!(bus.take_log().is_empty());

    im.set_window_active(false);
    bus.expect_calls(&["FocusOut"]);
    assert!(!im.is_enabled());

    im.set_window_active(true);
    bus.expect_calls(&["FocusIn"]);
    // Without the client: inactive, and the capabilities of no client.
    im.set_client(None);
    bus.expect_calls(&["FocusOut", &caps(IBusCapability::CAP_FOCUS)]);
    assert!(!im.is_enabled());
}

#[test]
fn ibus_connects_when_its_portal_comes_later_and_again_after_it_crashed() {
    let _scope = scope();
    let bus = TestBus::new();

    let im = IBusX11TextInputMethod::new(bus.client.clone());
    bus.settle();
    assert!(!im.is_connected());
    assert!(bus.take_log().is_empty());

    // What is reported while there is no input method is not sent later
    // by itself; the state is kept.
    im.set_window_active(true);
    im.set_client(Some(TestClient::new(false)));
    im.set_cursor_rect(Rect::new(1.0, 2.0, 3.0, 4.0));
    bus.settle();
    assert!(bus.take_log().is_empty());

    bus.start(IBUS_NAME);
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    assert!(im.is_connected());
    // The state is reported with the next change.
    im.set_window_active(true);
    bus.expect_calls(&["FocusIn"]);
    assert!(im.is_enabled());

    // The service is gone: the input method is disconnected and what it
    // last reported is forgotten.
    bus.stop(IBUS_NAME);
    pump_until(|| !im.is_connected());
    assert!(!im.is_enabled());
    bus.settle();
    assert!(bus.take_log().is_empty());

    bus.start(IBUS_NAME);
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    im.set_window_active(true);
    im.set_cursor_rect(Rect::new(1.0, 2.0, 3.0, 4.0));
    bus.expect_calls(&["FocusIn", "SetCursorLocation(1, 2, 3, 4)"]);
}

#[test]
fn ibus_takes_keys_commits_text_and_forwards_keys() {
    let _scope = scope();
    let bus = TestBus::new();
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    let recorded = record(&im);
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    im.set_window_active(true);
    im.set_client(Some(TestClient::new(false)));
    bus.expect_calls(&["FocusIn", &caps(IBusCapability::CAP_FOCUS)]);

    // A key the engine consumes: it commits text instead.
    assert!(handle(&im, key(RawKeyEventType::KeyDown, RawInputModifiers::empty(), 100), KEY_A, 38));
    bus.expect_calls(&["ProcessKeyEvent(0x61, 38, 0x0)"]);
    assert_eq!(*recorded.commits.borrow(), ["あ"]);

    // Its release, and a key with modifiers, are not consumed.
    assert!(!handle(&im, key(RawKeyEventType::KeyUp, RawInputModifiers::empty(), 101), KEY_A, 38));
    assert!(!handle(
        &im,
        key(RawKeyEventType::KeyDown, RawInputModifiers::CONTROL | RawInputModifiers::SHIFT, 102),
        0x62,
        56
    ));
    bus.expect_calls(&["ProcessKeyEvent(0x61, 38, 0x40000000)", "ProcessKeyEvent(0x62, 56, 0x5)"]);
    assert_eq!(recorded.commits.borrow().len(), 1);

    // A key the engine hands back.
    bus.emit(
        IBUS_CONTEXT_PATH,
        "org.freedesktop.IBus.InputContext",
        "ForwardKeyEvent",
        &(0xff0du32, 36u32, (IBusModifierMask::RELEASE_MASK | IBusModifierMask::MOD1_MASK).bits()),
    );
    bus.settle();
    assert_eq!(
        *recorded.forwarded.borrow(),
        [X11InputMethodForwardedKey {
            key_val: 0xff0d,
            modifiers: KeyModifiers::ALT,
            type_: RawKeyEventType::KeyUp,
            with_text: false
        }]
    );

    // Text that is committed while the context is reset is dropped; text
    // after it is not.
    im.reset();
    bus.expect_calls(&["Reset"]);
    finish_queued_calls(&im);
    bus.settle();
    assert_eq!(recorded.commits.borrow().len(), 1);
    bus.emit(IBUS_CONTEXT_PATH, "org.freedesktop.IBus.InputContext", "CommitText", &(ibus_text("後"),));
    bus.settle();
    assert_eq!(*recorded.commits.borrow(), ["あ", "後"]);
}

#[test]
fn ibus_reports_the_cursor_in_screen_pixels_when_it_changes() {
    let _scope = scope();
    let bus = TestBus::new();
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());

    im.set_cursor_rect(Rect::new(10.0, 20.0, 1.0, 16.0));
    bus.expect_calls(&["SetCursorLocation(10, 20, 1, 16)"]);

    // The window moved and is scaled: the rectangle is scaled, then moved.
    im.update_window_info(PixelPoint::new(100, 200), 2.0);
    bus.expect_calls(&["SetCursorLocation(120, 240, 2, 32)"]);

    // Nothing changed: nothing is sent.
    im.update_window_info(PixelPoint::new(100, 200), 2.0);
    im.set_cursor_rect(Rect::new(10.0, 20.0, 1.0, 16.0));
    bus.settle();
    assert!(bus.take_log().is_empty());

    // After a reset the rectangle is reported again.
    ITextInputMethodImpl::reset(&*im);
    im.set_cursor_rect(Rect::new(10.0, 20.0, 1.0, 16.0));
    bus.expect_calls(&["Reset", "SetCursorLocation(120, 240, 2, 32)"]);
}

#[test]
fn ibus_shows_and_hides_the_preedit_of_a_client_that_supports_it() {
    let _scope = scope();
    let bus = TestBus::new();
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    let client = TestClient::new(true);
    im.set_client(Some(client.clone()));
    bus.expect_calls(&["FocusOut", &caps(IBusCapability::CAP_FOCUS | IBusCapability::CAP_PREEDIT_TEXT)]);

    let interface = "org.freedesktop.IBus.InputContext";
    // The cursor counts characters; the client gets code units.
    bus.emit(IBUS_CONTEXT_PATH, interface, "UpdatePreeditText", &(ibus_text("\u{1F600}に"), 1u32, true));
    bus.settle();
    assert_eq!(client.take_preedit(), [(Some("\u{1F600}に".to_string()), Some(2))]);

    // The same text again is not reported.
    bus.emit(IBUS_CONTEXT_PATH, interface, "UpdatePreeditText", &(ibus_text("\u{1F600}に"), 2u32, true));
    bus.emit(IBUS_CONTEXT_PATH, interface, "ShowPreeditText", &());
    bus.settle();
    assert!(client.take_preedit().is_empty());

    bus.emit(IBUS_CONTEXT_PATH, interface, "HidePreeditText", &());
    bus.settle();
    assert_eq!(client.take_preedit(), [(Some(String::new()), Some(0))]);
    // Hidden twice: once.
    bus.emit(IBUS_CONTEXT_PATH, interface, "HidePreeditText", &());
    bus.settle();
    assert!(client.take_preedit().is_empty());

    // A client without pre-edit support is left alone.
    let plain = TestClient::new(false);
    im.set_client(Some(plain.clone()));
    bus.emit(IBUS_CONTEXT_PATH, interface, "UpdatePreeditText", &(ibus_text("x"), 1u32, true));
    bus.settle();
    assert!(plain.take_preedit().is_empty());
}

#[test]
fn ibus_destroys_its_context_when_disposed_and_when_a_call_fails() {
    let _scope = scope();
    let bus = TestBus::new();
    bus.start(IBUS_NAME);

    let im = IBusX11TextInputMethod::new(bus.client.clone());
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    im.dispose();
    bus.expect_calls(&["Destroy"]);
    assert!(!im.is_connected());
    // Disposed: signals are not listened to any more.
    let recorded = record(&im);
    bus.emit(IBUS_CONTEXT_PATH, "org.freedesktop.IBus.InputContext", "CommitText", &(ibus_text("late"),));
    bus.settle();
    assert!(recorded.commits.borrow().is_empty());

    // A key whose call fails is not consumed, and the input method lets
    // go of its context.
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    assert!(!handle(&im, key(RawKeyEventType::KeyDown, RawInputModifiers::empty(), 1), KEY_FAILS, 52));
    bus.expect_calls(&["ProcessKeyEvent(0x7a, 52, 0x0)", "Destroy"]);
    assert!(!im.is_connected());
    assert!(!handle(&im, key(RawKeyEventType::KeyDown, RawInputModifiers::empty(), 2), KEY_A, 38));
    bus.settle();
    assert!(bus.take_log().is_empty());
}

#[test]
fn ibus_drops_the_signals_of_a_context_whose_call_failed() {
    // Not upstream, whose subscriptions live until the owner of the name changes: the context is
    // retired before it is destroyed (DEVIATIONS.md, X11 platform).
    let _scope = scope();
    let bus = TestBus::new();
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    let recorded = record(&im);
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    assert!(!handle(&im, key(RawKeyEventType::KeyDown, RawInputModifiers::empty(), 1), KEY_FAILS, 52));
    bus.expect_calls(&["ProcessKeyEvent(0x7a, 52, 0x0)", "Destroy"]);
    assert!(!im.is_connected());
    bus.emit(IBUS_CONTEXT_PATH, "org.freedesktop.IBus.InputContext", "CommitText", &(ibus_text("late"),));
    bus.settle();
    assert!(recorded.commits.borrow().is_empty());
}

#[test]
fn ibus_dispose_during_connect_destroys_the_late_context_without_publishing_it() {
    let _scope = scope();
    let bus = TestBus::new();
    bus.ibus_calls.create.hold();
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    let recorded = record(&im);
    pump_until(|| !bus.log.lock().unwrap().is_empty());
    assert_eq!(bus.take_log(), ["CreateInputContext(name)"]);
    im.dispose();
    assert!(!im.is_connected());
    bus.ibus_calls.create.release();
    bus.expect_calls(&["Destroy"]);
    assert!(!im.is_connected());
    bus.emit(IBUS_CONTEXT_PATH, "org.freedesktop.IBus.InputContext", "CommitText", &(ibus_text("late"),));
    bus.settle();
    assert!(recorded.commits.borrow().is_empty());
    assert!(bus.take_log().is_empty());
}

#[test]
fn ibus_owner_loss_during_connect_retires_the_attempt_and_connects_to_the_next_owner() {
    let _scope = scope();
    let bus = TestBus::new();
    bus.ibus_calls.create.hold();
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    pump_until(|| !bus.log.lock().unwrap().is_empty());
    assert_eq!(bus.take_log(), ["CreateInputContext(name)"]);
    bus.stop(IBUS_NAME);
    bus.settle();
    bus.start(IBUS_NAME);
    bus.settle();
    assert!(!im.is_connected());
    bus.ibus_calls.create.release();
    bus.expect_calls(&["Destroy", "CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    assert!(im.is_connected());
}

#[test]
fn ibus_follows_a_portal_whose_owner_is_replaced_directly() {
    // Not upstream, which reacts only to the name losing its owner and keeps a context of the
    // old owner (DEVIATIONS.md, X11 platform).
    const NEW_OWNER: &str = ":1.43";
    let _scope = scope();
    let bus = TestBus::new();
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    let recorded = record(&im);
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    if !bus.replace_owner(IBUS_NAME, NEW_OWNER) {
        // A bus gives every connection its own unique name: only its double can do this.
        return;
    }
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    assert!(im.is_connected());
    let interface = "org.freedesktop.IBus.InputContext";
    bus.emit(IBUS_CONTEXT_PATH, interface, "CommitText", &(ibus_text("old owner"),));
    bus.emit_as(NEW_OWNER, IBUS_CONTEXT_PATH, interface, "CommitText", &(ibus_text("new owner"),));
    bus.settle();
    assert_eq!(*recorded.commits.borrow(), ["new owner"]);
}

fn assert_pending_ibus_signal_fence(dispose: bool) {
    use crate::signal_watch::watch_stream;
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::threading::Dispatcher;

    struct TrackedSubscription {
        inner: Rc<dyn IDisposable>,
        cleaned_up: Rc<Cell<bool>>,
    }

    impl IDisposable for TrackedSubscription {
        fn dispose(&self) {
            self.cleaned_up.set(true);
            self.inner.dispose();
        }
    }

    let _scope = scope();
    let bus = TestBus::new();
    bus.ibus_calls.create.hold();
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    let recorded = record(&im);
    let client = TestClient::new(true);
    im.set_client(Some(client.clone()));
    pump_until(|| !bus.log.lock().unwrap().is_empty());
    assert_eq!(bus.take_log(), ["CreateInputContext(name)"]);

    // The guard of the pending connect. A callback of a subscription can be queued while the
    // subscription is not registered yet: the handle is kept unregistered on purpose.
    let signals = im.signal_guard();
    let deliveries = Rc::new(Cell::new(0));
    let mut guarded = signals.handler({
        let deliveries = deliveries.clone();
        let weak = Rc::downgrade(&im);
        move |()| {
            deliveries.set(deliveries.get() + 1);
            let im = weak.upgrade().unwrap();
            im.fire_commit("queued text".to_string());
            im.fire_forward(super::ibus::ibus_x11_text_input_method::forwarded_key(KEY_A, 0));
            im.client().unwrap().set_preedit_text_with_cursor(Some("queued preedit"), Some(1));
        }
    });
    // The context being created gets its signals: the guard asks for nothing more.
    guarded(());
    assert_eq!(deliveries.get(), 1);
    assert_eq!(std::mem::take(&mut *recorded.commits.borrow_mut()), ["queued text"]);
    assert_eq!(recorded.forwarded.borrow_mut().drain(..).count(), 1);
    assert_eq!(client.take_preedit(), [(Some("queued preedit".to_string()), Some(1))]);

    let release = Arc::new(DeferredCall::default());
    release.hold();
    let stream = futures_util::stream::once({
        let release = release.clone();
        async move {
            release.wait().await;
        }
    });
    let attempted = Rc::new(Cell::new(false));
    let subscription = watch_stream(stream, {
        let attempted = attempted.clone();
        move |item| {
            attempted.set(true);
            guarded(item);
        }
    });
    let cleaned_up = Rc::new(Cell::new(false));
    let subscription = Rc::new(TrackedSubscription { inner: subscription, cleaned_up: cleaned_up.clone() });
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!attempted.get());

    if dispose {
        im.dispose();
    } else {
        bus.stop(IBUS_NAME);
        bus.settle();
    }
    release.release();
    pump_until(|| attempted.get());
    assert!(!cleaned_up.get(), "the queued callback ran before the subscription was cleaned up");
    assert_eq!(deliveries.get(), 1, "a callback of a retired context reaches nobody");
    assert!(recorded.commits.borrow().is_empty());
    assert!(recorded.forwarded.borrow().is_empty());
    assert!(client.take_preedit().is_empty());

    signals.register(subscription);
    assert!(cleaned_up.get(), "a late registration is disposed at once");
    if dispose {
        // What is delivered directly after the disposal reaches nobody either.
        im.fire_commit("late direct text".to_string());
        im.fire_forward(super::ibus::ibus_x11_text_input_method::forwarded_key(KEY_A, 0));
        assert!(recorded.commits.borrow().is_empty());
        assert!(recorded.forwarded.borrow().is_empty());
    }

    bus.ibus_calls.create.release();
    bus.expect_calls(&["Destroy"]);
    assert!(!im.is_connected());
    if !dispose {
        bus.start(IBUS_NAME);
        bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
        bus.emit(IBUS_CONTEXT_PATH, "org.freedesktop.IBus.InputContext", "CommitText", &(ibus_text("new context"),));
        bus.settle();
        assert_eq!(*recorded.commits.borrow(), ["new context"]);
        im.dispose();
        bus.expect_calls(&["Destroy"]);
    }
}

#[test]
fn ibus_dispose_fences_queued_signals_before_their_subscriptions_are_cleaned_up() {
    assert_pending_ibus_signal_fence(true);
}

#[test]
fn ibus_owner_loss_fences_queued_signals_before_their_subscriptions_are_cleaned_up() {
    assert_pending_ibus_signal_fence(false);
}

// ---------------------------------------------------------------------
// The IBus content type: for a client of the exact-edit connection only (decision D5 of
// `docs/porting/winui-rich-text.md`; upstream's `SetOptions` is a no-op for IBus).

#[test]
fn ibus_never_sets_the_content_type_for_a_legacy_client() {
    let _scope = scope();
    let bus = TestBus::new();
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    im.set_window_active(true);
    bus.expect_calls(&["FocusOut"]);
    let options = TextInputOptions {
        content_type: TextInputContentType::Password,
        is_sensitive: true,
        show_suggestions: Some(false),
        ..Default::default()
    };
    im.set_options(&options);
    im.set_client(Some(TestClient::new(true)));
    bus.expect_calls(&["FocusIn", &caps(IBusCapability::CAP_FOCUS | IBusCapability::CAP_PREEDIT_TEXT)]);
    im.set_options(&TextInputOptions { content_type: TextInputContentType::Email, ..options });
    im.set_client(None);
    bus.expect_calls(&["FocusOut", &caps(IBusCapability::CAP_FOCUS)]);
}

/// The input method under test as the feature of every window of a unit-test application, so
/// that the input method manager binds the focused element to it.
type InputMethodSlot = Rc<RefCell<Option<Rc<DBusTextInputMethodBase>>>>;

fn application_with_input_method() -> (ferroui_controls::testing::UnitTestApplicationScope, InputMethodSlot) {
    use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
    let slot = InputMethodSlot::default();
    let method = slot.clone();
    let app = UnitTestApplication::start(TestServices::focusable_window().with_windowing_platform(
        MockWindowingPlatform::with_window_impl(move || {
            let window = MockWindowingPlatform::create_window_mock();
            if let Some(im) = method.borrow().clone() {
                window.setup_feature::<dyn ITextInputMethodImpl>(im);
            }
            window
        }),
    ));
    (app, slot)
}

/// A rich edit box (a client of the exact-edit connection) and a focusable border in a window.
struct RichWindow {
    window: Ref<ferroui_controls::Window>,
    rich: Ref<ferroui_controls::RichEditBox>,
    outside: Ref<ferroui_controls::Border>,
    _text: ferroui_base::media::text_formatting::testing::TextTestScope,
}

impl RichWindow {
    fn show(slot: &InputMethodSlot, im: &Rc<DBusTextInputMethodBase>, scopes: &[InputScopeNameValue]) -> RichWindow {
        use ferroui_controls::{Border, Control, RichEditBox, StackPanel, Window};
        *slot.borrow_mut() = Some(im.clone());
        let text = ferroui_base::media::text_formatting::testing::TextTestScope::new();
        let rich = RichEditBox::new();
        rich.set_height(40.0);
        TextInputOptions::set_content_type(&rich, TextInputContentType::Email);
        TextInputOptions::set_input_scope_names(&rich, Some(Rc::from(scopes)));
        let outside = Border::new();
        outside.set_focusable(true);
        outside.set_height(20.0);
        let panel = StackPanel::new();
        panel.children().add(rich.clone());
        panel.children().add(outside.clone());
        let window = Window::new();
        window.set_width(200.0);
        window.set_height(100.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        RichWindow { window, rich, outside, _text: text }
    }

    fn set_scopes(&self, scopes: Option<&[InputScopeNameValue]>) {
        TextInputOptions::set_input_scope_names(&self.rich, scopes.map(Rc::from));
    }
}

impl Drop for RichWindow {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn content_types(log: &[String]) -> Vec<String> {
    log.iter().filter(|call| call.starts_with("SetContentType(")).cloned().collect()
}

/// Runs the dispatcher until the context was given `entries` as its `ContentType`, in that
/// order, and forgets the calls so far: what else the window reports (focus, the cursor) is not
/// what these tests are about.
fn expect_content_types(bus: &TestBus, entries: &[&str]) {
    pump_until(|| content_types(&bus.log.lock().unwrap()).len() >= entries.len());
    bus.settle();
    let log = bus.take_log();
    assert_eq!(content_types(&log), entries, "all calls: {log:?}");
}

/// The hints of the options of a rich edit box: text prediction is on (`SPELLCHECK`,
/// `WORD_COMPLETION`).
const RICH_HINTS: u32 = 5;

fn ibus_connected(bus: &TestBus) -> Rc<DBusTextInputMethodBase> {
    bus.start(IBUS_NAME);
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    bus.expect_calls(&["CreateInputContext(name)", &caps(IBusCapability::CAP_FOCUS)]);
    pump_until(|| im.is_connected());
    im
}

#[test]
fn ibus_gives_a_connection_client_its_input_scope_and_any_other_client_the_default() {
    use InputScopeNameValue as Scope;
    let (_app, slot) = application_with_input_method();
    let bus = TestBus::new();
    let im = ibus_connected(&bus);
    let f = RichWindow::show(&slot, &im, &[Scope::Digits, Scope::TelephoneNumber]);
    assert!(f.rich.focus());
    // The first scope, before the second and the content type (Email).
    expect_content_types(&bus, &[&format!("SetContentType(2, {RICH_HINTS})")]);
    f.set_scopes(Some(&[Scope::TelephoneNumber]));
    expect_content_types(&bus, &[&format!("SetContentType(4, {RICH_HINTS})")]);
    f.set_scopes(Some(&[Scope::Default]));
    expect_content_types(&bus, &[&format!("SetContentType(0, {RICH_HINTS})")]);
    // No scope: the content type.
    f.set_scopes(Some(&[]));
    expect_content_types(&bus, &[&format!("SetContentType(6, {RICH_HINTS})")]);
    f.set_scopes(None);
    bus.settle();
    assert!(content_types(&bus.take_log()).is_empty());

    // The focus leaves the rich edit box: the context goes back to its default, and gets the
    // content type again with the focus.
    assert!(f.outside.focus());
    expect_content_types(&bus, &["SetContentType(0, 0)"]);
    assert!(f.rich.focus());
    expect_content_types(&bus, &[&format!("SetContentType(6, {RICH_HINTS})")]);

    // A legacy client: the default again, and nothing of its options.
    im.set_options(&TextInputOptions { content_type: TextInputContentType::Url, ..Default::default() });
    im.set_client(Some(TestClient::new(true)));
    expect_content_types(&bus, &["SetContentType(0, 0)"]);
    im.set_options(&TextInputOptions { content_type: TextInputContentType::Number, ..Default::default() });
    bus.settle();
    assert!(content_types(&bus.take_log()).is_empty());
}

#[test]
fn ibus_keeps_the_input_method_when_the_content_type_is_refused() {
    use InputScopeNameValue as Scope;
    let (_app, slot) = application_with_input_method();
    let bus = TestBus::new();
    let im = ibus_connected(&bus);
    let recorded = record(&im);
    bus.ibus_calls.refuse_next_content_type.store(true, Ordering::SeqCst);
    let f = RichWindow::show(&slot, &im, &[Scope::Digits]);
    assert!(f.rich.focus());
    expect_content_types(&bus, &[&format!("SetContentType(2, {RICH_HINTS})")]);
    assert!(im.is_connected());
    // The context goes on taking keys and committing text.
    assert!(handle(&im, key(RawKeyEventType::KeyDown, RawInputModifiers::empty(), 1), KEY_A, 38));
    bus.settle();
    assert_eq!(*recorded.commits.borrow(), ["あ"]);
    assert!(im.is_connected());
    // The property is not set on that context again.
    f.set_scopes(Some(&[Scope::TelephoneNumber]));
    bus.settle();
    assert!(content_types(&bus.take_log()).is_empty());
    assert!(im.is_connected());

    // A new context is given it again.
    bus.stop(IBUS_NAME);
    pump_until(|| !im.is_connected());
    bus.start(IBUS_NAME);
    expect_content_types(&bus, &[&format!("SetContentType(4, {RICH_HINTS})")]);
    assert!(im.is_connected());
}

#[test]
fn ibus_gives_a_context_created_late_the_latest_content_type() {
    use InputScopeNameValue as Scope;
    let (_app, slot) = application_with_input_method();
    let bus = TestBus::new();
    let im = IBusX11TextInputMethod::new(bus.client.clone());
    let f = RichWindow::show(&slot, &im, &[Scope::Digits]);
    assert!(f.rich.focus());
    bus.settle();
    assert!(bus.take_log().is_empty());
    bus.ibus_calls.create.hold();
    bus.start(IBUS_NAME);
    pump_until(|| !bus.log.lock().unwrap().is_empty());
    assert_eq!(bus.take_log(), ["CreateInputContext(name)"]);
    f.set_scopes(Some(&[Scope::TelephoneNumber, Scope::EmailSmtpAddress]));
    bus.ibus_calls.create.release();
    expect_content_types(&bus, &[&format!("SetContentType(4, {RICH_HINTS})")]);
    assert!(im.is_connected());
}

#[test]
fn ibus_coalesces_content_types_queued_behind_a_pending_one() {
    use InputScopeNameValue as Scope;
    let (_app, slot) = application_with_input_method();
    let bus = TestBus::new();
    let im = ibus_connected(&bus);
    bus.ibus_calls.content_type.hold();
    let f = RichWindow::show(&slot, &im, &[Scope::Digits]);
    assert!(f.rich.focus());
    pump_until(|| !content_types(&bus.log.lock().unwrap()).is_empty());
    f.set_scopes(Some(&[Scope::EmailSmtpAddress]));
    f.set_scopes(Some(&[Scope::TelephoneNumber]));
    bus.ibus_calls.content_type.release();
    expect_content_types(&bus, &[&format!("SetContentType(2, {RICH_HINTS})"), &format!("SetContentType(4, {RICH_HINTS})")]);
    // Another scope with the same value is not sent.
    f.set_scopes(Some(&[Scope::TelephoneAreaCode, Scope::Digits]));
    bus.settle();
    assert!(content_types(&bus.take_log()).is_empty());
}

#[test]
fn ibus_applies_a_content_type_changed_by_a_signal_while_the_previous_one_waits() {
    use InputScopeNameValue as Scope;
    let (_app, slot) = application_with_input_method();
    let bus = TestBus::new();
    let im = ibus_connected(&bus);
    let f = Rc::new(RichWindow::show(&slot, &im, &[Scope::Digits]));
    let reentered = Rc::new(Cell::new(false));
    {
        let (f, reentered) = (Rc::downgrade(&f), reentered.clone());
        im.commit().subscribe(move |_| {
            if let Some(f) = f.upgrade() {
                f.set_scopes(Some(&[Scope::EmailSmtpAddress]));
            }
            reentered.set(true);
        });
    }
    bus.ibus_calls.content_type.hold();
    bus.ibus_calls.commit_in_next_content_type.store(true, Ordering::SeqCst);
    assert!(f.rich.focus());
    pump_until(|| reentered.get());
    bus.ibus_calls.content_type.release();
    expect_content_types(&bus, &[&format!("SetContentType(2, {RICH_HINTS})"), &format!("SetContentType(6, {RICH_HINTS})")]);
    assert!(im.is_connected());
}

#[test]
fn ibus_ignores_the_failed_content_type_reply_of_a_retired_context() {
    use InputScopeNameValue as Scope;
    let (_app, slot) = application_with_input_method();
    let bus = TestBus::new();
    let im = ibus_connected(&bus);
    bus.ibus_calls.content_type.hold();
    bus.ibus_calls.refuse_next_content_type.store(true, Ordering::SeqCst);
    let f = RichWindow::show(&slot, &im, &[Scope::Digits]);
    assert!(f.rich.focus());
    pump_until(|| !content_types(&bus.log.lock().unwrap()).is_empty());
    bus.stop(IBUS_NAME);
    pump_until(|| !im.is_connected());
    f.set_scopes(Some(&[Scope::TelephoneNumber]));
    bus.start(IBUS_NAME);
    pump_until(|| im.is_connected());
    bus.ibus_calls.content_type.release();
    // The refusal belongs to the old context; the new one gets the latest value.
    expect_content_types(&bus, &[&format!("SetContentType(2, {RICH_HINTS})"), &format!("SetContentType(4, {RICH_HINTS})")]);
    assert!(im.is_connected());
    f.set_scopes(Some(&[Scope::TelephoneLocalNumber]));
    bus.settle();
    assert!(content_types(&bus.take_log()).is_empty());
}

#[test]
fn fcitx_gives_a_connection_client_the_capabilities_of_its_input_scope() {
    use InputScopeNameValue as Scope;
    let (_app, slot) = application_with_input_method();
    let bus = TestBus::new();
    bus.start(FCITX5_NAME);
    let im = FcitxX11TextInputMethod::new(bus.client.clone());
    bus.expect_calls(&["CreateInputContext(appName)"]);
    pump_until(|| im.is_connected());
    let capabilities = |log: &[String]| -> Vec<String> {
        log.iter().filter(|call| call.starts_with("SetCapability(")).cloned().collect()
    };
    let expect_capability = |flags: FcitxCapabilityFlags| {
        let flags = flags | FcitxCapabilityFlags::CAPACITY_PREEDIT | FcitxCapabilityFlags::CAPACITY_NOAUTOUPPERCASE;
        pump_until(|| capabilities(&bus.log.lock().unwrap()).last() == Some(&format!("SetCapability({})", flags.bits())));
        bus.settle();
        bus.take_log();
    };
    let f = RichWindow::show(&slot, &im, &[Scope::Digits, Scope::TelephoneNumber]);
    assert!(f.rich.focus());
    expect_capability(FcitxCapabilityFlags::CAPACITY_DIGIT);
    f.set_scopes(Some(&[Scope::TelephoneNumber]));
    expect_capability(FcitxCapabilityFlags::CAPACITY_DIALABLE);
    // Without a scope, the content type (Email) as upstream.
    f.set_scopes(None);
    expect_capability(FcitxCapabilityFlags::CAPACITY_EMAIL);
    // A legacy client is given the flags of its content type only.
    f.set_scopes(Some(&[Scope::Digits]));
    expect_capability(FcitxCapabilityFlags::CAPACITY_DIGIT);
    im.set_options(&TextInputOptions { content_type: TextInputContentType::Url, ..Default::default() });
    im.set_client(Some(TestClient::new(true)));
    expect_capability(FcitxCapabilityFlags::CAPACITY_URL);
}

// ---------------------------------------------------------------------
// Fcitx.

fn fcitx_round(name: &str, context_path: &str, interface: &str, modern: bool) {
    let bus = TestBus::new();
    bus.start(name);
    let im = FcitxX11TextInputMethod::new(bus.client.clone());
    let recorded = record(&im);
    let client = TestClient::new(true);

    let set_capacity = |flags: FcitxCapabilityFlags| {
        if modern {
            format!("SetCapability({})", flags.bits())
        } else {
            format!("SetCapacity({})", flags.bits())
        }
    };
    let create =
        if modern { "CreateInputContext(appName)" } else { "CreateICv3(name: true, own pid: true)" }.to_string();
    bus.expect_calls(&[&create]);
    pump_until(|| im.is_connected());
    im.set_window_active(true);
    im.set_client(Some(client.clone()));
    bus.expect_calls(&["FocusIn", &set_capacity(FcitxCapabilityFlags::CAPACITY_PREEDIT)]);
    assert!(im.is_enabled());

    // The options of the text input are added to the capabilities, and
    // the same flags are not sent twice.
    let mut options = TextInputOptions::default_options();
    options.auto_capitalization = true;
    im.set_options(&options);
    bus.settle();
    assert!(bus.take_log().is_empty());
    options.content_type = TextInputContentType::Email;
    im.set_options(&options);
    bus.expect_calls(&[&set_capacity(
        FcitxCapabilityFlags::CAPACITY_PREEDIT | FcitxCapabilityFlags::CAPACITY_EMAIL,
    )]);

    // Keys: the press of "a" is consumed and committed; the type of the
    // event is a number for version 4 and a flag for version 5.
    assert!(handle(&im, key(RawKeyEventType::KeyDown, RawInputModifiers::empty(), 5000), KEY_A, 38));
    assert!(!handle(&im, key(RawKeyEventType::KeyUp, RawInputModifiers::CONTROL, 5001), KEY_A, 38));
    let (press, release) = if modern { ("false", "true") } else { ("0", "1") };
    bus.expect_calls(&[
        &format!("ProcessKeyEvent(0x61, 38, 0, {press}, 5000)"),
        &format!("ProcessKeyEvent(0x61, 38, {}, {release}, 5001)", FcitxKeyState::CTRL.bits()),
    ]);
    assert_eq!(*recorded.commits.borrow(), [if modern { "五" } else { "四" }]);

    // The cursor rectangle is never empty for Fcitx.
    im.set_cursor_rect(Rect::new(4.0, 5.0, 0.0, 0.0));
    bus.expect_calls(&["SetCursorRect(4, 5, 1, 1)"]);

    // A formatted pre-edit: the parts joined, the cursor from bytes.
    bus.emit(context_path, interface, "UpdateFormattedPreedit", &(vec![("に", 8i32), ("ほ", 0i32)], 3i32));
    bus.settle();
    assert_eq!(client.take_preedit(), [(Some("にほ".to_string()), Some(1))]);
    bus.emit(context_path, interface, "UpdateFormattedPreedit", &(Vec::<(&str, i32)>::new(), 0i32));
    bus.settle();
    assert_eq!(client.take_preedit(), [(None, None)]);

    // A forwarded key.
    if modern {
        bus.emit(context_path, interface, "ForwardKey", &(0xff08u32, FcitxKeyState::SHIFT.bits(), false));
    } else {
        bus.emit(context_path, interface, "ForwardKey", &(0xff08u32, FcitxKeyState::SHIFT.bits(), 0i32));
    }
    bus.settle();
    assert_eq!(
        *recorded.forwarded.borrow(),
        [X11InputMethodForwardedKey {
            key_val: 0xff08,
            modifiers: KeyModifiers::SHIFT,
            type_: RawKeyEventType::KeyDown,
            with_text: true
        }]
    );

    // A reset forgets what was reported: the flags are sent again with
    // the next change of the options.
    ITextInputMethodImpl::reset(&*im);
    im.set_options(&options);
    bus.expect_calls(&[
        "Reset",
        &set_capacity(FcitxCapabilityFlags::CAPACITY_PREEDIT | FcitxCapabilityFlags::CAPACITY_EMAIL),
    ]);

    im.dispose();
    bus.expect_calls(&["DestroyIC"]);
    assert!(!im.is_connected());
}

#[test]
fn fcitx_4_is_reached_by_its_own_name() {
    let _scope = scope();
    fcitx_round(FCITX4_NAME, FCITX4_CONTEXT_PATH, "org.fcitx.Fcitx.InputContext", false);
}

#[test]
fn fcitx_5_is_reached_by_its_portal_name() {
    let _scope = scope();
    fcitx_round(FCITX5_NAME, FCITX5_CONTEXT_PATH, "org.fcitx.Fcitx.InputContext1", true);
}

#[test]
fn fcitx_takes_the_name_that_is_online() {
    let _scope = scope();
    let bus = TestBus::new();
    let im = FcitxX11TextInputMethod::new(bus.client.clone());
    bus.settle();
    assert!(!im.is_connected());

    // Version 5 appears first; version 4 appearing later changes nothing.
    bus.start(FCITX5_NAME);
    bus.expect_calls(&["CreateInputContext(appName)"]);
    pump_until(|| im.is_connected());
    bus.start(FCITX4_NAME);
    bus.settle();
    assert!(bus.take_log().is_empty());

    // The other service going away does not disconnect.
    bus.stop(FCITX4_NAME);
    bus.settle();
    assert!(im.is_connected());
    bus.stop(FCITX5_NAME);
    pump_until(|| !im.is_connected());
}
