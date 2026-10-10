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
    FocusManager, IInputDevice, IInputRoot, InputElement, Key, KeyDeviceType, KeyModifiers, PhysicalKey,
    RawInputModifiers,
};
use ferroui_base::{PixelPoint, Rect, Ref, Visual};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
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

struct IBusPortal {
    log: Log,
}

#[zbus::interface(name = "org.freedesktop.IBus.Portal")]
impl IBusPortal {
    fn create_input_context(&self, client_name: &str) -> OwnedObjectPath {
        log(&self.log, format!("CreateInputContext({})", if client_name.is_empty() { "empty" } else { "name" }));
        ObjectPath::try_from(IBUS_CONTEXT_PATH).unwrap().into()
    }
}

struct IBusContext {
    log: Log,
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

        let serve = {
            let log = log.clone();
            move |builder: ServiceBuilder| {
                builder
                    .serve_at("/org/freedesktop/IBus", IBusPortal { log: log.clone() })?
                    .serve_at(IBUS_CONTEXT_PATH, IBusContext { log: log.clone() })?
                    .serve_at(IBUS_CONTEXT_PATH, IBusService { log: log.clone() })?
                    .serve_at("/inputmethod", Fcitx4Method { log: log.clone() })?
                    .serve_at(FCITX4_CONTEXT_PATH, Fcitx4Context { log: log.clone() })?
                    .serve_at("/inputmethod", Fcitx5Method { log: log.clone() })?
                    .serve_at(FCITX5_CONTEXT_PATH, Fcitx5Context { log: log.clone() })
            }
        };

        TestBus { connections: TestConnections::new("/org/freedesktop/IBus", serve), log }
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

    fn text_view_visual(&self) -> Ref<Visual> {
        unreachable!("the input methods over D-Bus do not ask for the visual")
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
