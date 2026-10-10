// Not from the reference, which has no tests for this file. The handler
// of a drag runs here against a host that is a small tree of windows with
// properties, and that records what the handler asks of the connection.

use super::*;
use std::collections::HashMap;
use std::future::Future;
use std::task::{Context, Poll, Waker};

const ROOT: XID = 1;
const SOURCE: XID = 0x400001;
/// A window of the application, at x 0 to 99.
const OWN: XID = 0x400002;
/// A frame of a window manager at x 100 to 199, with the client inside.
const FRAME: XID = 0x500000;
const CLIENT: XID = 0x600001;
/// A window at x 200 to 299 that names a proxy.
const PROXIED: XID = 0x700001;
const PROXY: XID = 0x700002;
/// A window at x 300 to 399 that speaks version 4.
const OLD: XID = 0x800001;
/// A window at x 400 to 499 that does not take part.
const DEAF: XID = 0x900001;

const ATOMS: XdndSourceAtoms = XdndSourceAtoms {
    enter: 1,
    position: 2,
    status: 10,
    leave: 3,
    drop: 4,
    finished: 11,
    aware: 20,
    proxy: 21,
    atom: 30,
    window: 31,
    actions: XdndActions { copy: 101, move_: 102, link: 103 },
};
const COPY: c_long = 101;
const MOVE: c_long = 102;
const LINK: c_long = 103;

#[derive(Clone, Debug, PartialEq)]
enum Call {
    Publish(Vec<Atom>),
    Send(Atom, XID, [c_long; 5]),
    Raw(XID, RawDragEventType, PixelPoint, DragDropEffects),
    Ungrab,
    Cursor(DragDropEffects),
    RestartTimeout,
    StopTimeout,
    Activity(bool),
    SelectionRequest,
    Dispose,
}

struct FakeHost {
    calls: RefCell<Vec<Call>>,
    properties: RefCell<HashMap<(XID, Atom), c_ulong>>,
    formats: RefCell<Vec<Atom>>,
    /// What the handlers of a window of the application answer.
    raw_answer: Cell<DragDropEffects>,
    keysym: Cell<c_ulong>,
    source_is_own: Cell<bool>,
}

impl FakeHost {
    fn new() -> Rc<Self> {
        let mut properties = HashMap::new();
        properties.insert((CLIENT, ATOMS.aware), 5);
        properties.insert((PROXIED, ATOMS.proxy), PROXY as c_ulong);
        properties.insert((PROXY, ATOMS.proxy), PROXY as c_ulong);
        properties.insert((PROXY, ATOMS.aware), 5);
        properties.insert((OLD, ATOMS.aware), 4);
        Rc::new(Self {
            calls: RefCell::new(Vec::new()),
            properties: RefCell::new(properties),
            formats: RefCell::new(vec![201, 202]),
            raw_answer: Cell::new(DragDropEffects::COPY),
            keysym: Cell::new(0),
            source_is_own: Cell::new(true),
        })
    }

    fn take(&self) -> Vec<Call> {
        std::mem::take(&mut *self.calls.borrow_mut())
    }

    fn push(&self, call: Call) {
        self.calls.borrow_mut().push(call);
    }
}

impl IDragSourceHost for FakeHost {
    fn is_in_process_window(&self, window: XID) -> bool {
        window == OWN || (window == SOURCE && self.source_is_own.get())
    }

    fn get_window_property(&self, window: XID, property: Atom, type_: Atom) -> Option<c_ulong> {
        // A property is read with the type the specification gives it.
        let expected = if property == ATOMS.proxy { ATOMS.window } else { ATOMS.atom };
        assert_eq!(type_, expected);
        self.properties.borrow().get(&(window, property)).copied()
    }

    fn root_window(&self) -> XID {
        ROOT
    }

    fn child_at(&self, window: XID, root_position: PixelPoint) -> Option<XID> {
        if root_position.x < 0 {
            // Another screen: the server cannot tell.
            return None;
        }
        Some(match (window, root_position.x / 100) {
            (ROOT, 0) => OWN,
            (ROOT, 1) => FRAME,
            (FRAME, 1) => CLIENT,
            (ROOT, 2) => PROXIED,
            (ROOT, 3) => OLD,
            (ROOT, 4) => DEAF,
            _ => 0,
        })
    }

    fn format_atoms(&self) -> Vec<Atom> {
        self.formats.borrow().clone()
    }

    fn publish(&self, format_atoms: &[Atom]) {
        self.push(Call::Publish(format_atoms.to_vec()));
    }

    fn send_xdnd_message(&self, message_type: Atom, message_window: XID, data: [c_long; 5]) {
        self.push(Call::Send(message_type, message_window, data));
    }

    fn process_raw_drag_event(
        &self,
        target_window: XID,
        event_type: RawDragEventType,
        root_position: PixelPoint,
        _modifiers: RawInputModifiers,
        effective_allowed_effects: DragDropEffects,
    ) -> DragDropEffects {
        self.push(Call::Raw(target_window, event_type, root_position, effective_allowed_effects));
        self.raw_answer.get()
    }

    fn ungrab_pointer(&self) {
        self.push(Call::Ungrab);
    }

    fn set_grab_cursor(&self, effects: DragDropEffects) {
        self.push(Call::Cursor(effects));
    }

    fn restart_timeout(&self) {
        self.push(Call::RestartTimeout);
    }

    fn stop_timeout(&self) {
        self.push(Call::StopTimeout);
    }

    fn set_activity_restarts_timeout(&self, value: bool) {
        self.push(Call::Activity(value));
    }

    fn on_selection_request(&self, _evt: &XEvent) {
        self.push(Call::SelectionRequest);
    }

    fn lookup_keysym(&self, _key: &XKeyEvent) -> c_ulong {
        self.keysym.get()
    }

    fn atom_name(&self, _atom: Atom) -> Option<String> {
        None
    }

    fn dispose(&self) {
        self.push(Call::Dispose);
    }
}

const ALL: DragDropEffects = DragDropEffects::COPY.union(DragDropEffects::MOVE).union(DragDropEffects::LINK);

fn start(host: &Rc<FakeHost>, allowed: DragDropEffects, modifiers: KeyModifiers) -> Rc<Handler> {
    Handler::new(host.clone(), SOURCE, allowed, modifiers, ATOMS, None)
}

fn result(handler: &Handler) -> Option<DragDropEffects> {
    let mut completion = std::pin::pin!(handler.completion());
    match completion.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(effects) => Some(effects),
        Poll::Pending => None,
    }
}

fn at(x: i32) -> PixelPoint {
    PixelPoint::new(x, 40)
}

fn packed(x: i32) -> c_long {
    ((x as c_long) << 16) | 40
}

const NO_MODIFIERS: RawInputModifiers = RawInputModifiers::empty();
const SRC: c_long = SOURCE as c_long;

fn status(target: XID, accepted: bool, action: c_long) -> [c_long; 5] {
    [target as c_long, c_long::from(accepted), 0, 0, action]
}

fn finished(target: XID, accepted: bool, action: c_long) -> [c_long; 5] {
    [target as c_long, c_long::from(accepted), action, 0, 0]
}

#[test]
fn a_drag_from_a_window_that_is_not_ours_ends_at_once() {
    let host = FakeHost::new();
    host.source_is_own.set(false);
    let handler = start(&host, ALL, KeyModifiers::empty());
    assert_eq!(result(&handler), Some(DragDropEffects::NONE));
    // Nothing was published, grabbed or shown.
    assert!(host.take().is_empty());
    handler.dispose();
    assert_eq!(host.take(), [Call::Dispose]);
}

#[test]
fn a_drag_starts_with_the_cursor_of_its_effects_and_its_formats_published() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    assert_eq!(result(&handler), None);
    assert_eq!(host.take(), [Call::Cursor(ALL), Call::Publish(vec![201, 202])]);

    // The modifier keys that are down choose among the allowed effects.
    let host = FakeHost::new();
    start(&host, ALL, KeyModifiers::SHIFT);
    assert_eq!(host.take()[0], Call::Cursor(DragDropEffects::MOVE));
    let host = FakeHost::new();
    start(&host, DragDropEffects::COPY, KeyModifiers::SHIFT);
    // Nothing is left of the allowed effects: no change from "none", no cursor call.
    assert_eq!(host.take(), [Call::Publish(vec![201, 202])]);
}

#[test]
fn the_modifier_keys_choose_the_effect() {
    let all = get_allowed_effects_from_key_modifiers;
    assert_eq!(all(KeyModifiers::empty()), ALL);
    assert_eq!(all(KeyModifiers::CONTROL), DragDropEffects::COPY);
    assert_eq!(all(KeyModifiers::SHIFT), DragDropEffects::MOVE);
    assert_eq!(all(KeyModifiers::ALT), DragDropEffects::LINK);
    // Control before shift before alt.
    assert_eq!(all(KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT), DragDropEffects::COPY);
    assert_eq!(all(KeyModifiers::SHIFT | KeyModifiers::ALT), DragDropEffects::MOVE);
    assert_eq!(all(KeyModifiers::META), ALL);
}

#[test]
fn the_target_is_the_first_window_under_the_pointer_that_takes_part() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());

    let target = |x| handler.find_xdnd_target(at(x));
    // A window of the application: no protocol.
    assert_eq!(
        target(50),
        Some(XdndTargetInfo { version: 5, target_window: OWN, message_window: OWN, in_process: true })
    );
    // The client inside the frame of a window manager.
    assert_eq!(
        target(150),
        Some(XdndTargetInfo { version: 5, target_window: CLIENT, message_window: CLIENT, in_process: false })
    );
    // A window with a proxy: the messages go to the proxy.
    assert_eq!(
        target(250),
        Some(XdndTargetInfo { version: 5, target_window: PROXIED, message_window: PROXY, in_process: false })
    );
    assert_eq!(
        target(350),
        Some(XdndTargetInfo { version: 4, target_window: OLD, message_window: OLD, in_process: false })
    );
    // A window that does not take part, the bare root, and a point the server cannot place.
    assert_eq!(target(450), None);
    assert_eq!(target(550), None);
    assert_eq!(target(-5), None);

    // A proxy that does not name itself is no proxy: the window counts by itself.
    host.properties.borrow_mut().insert((PROXY, ATOMS.proxy), 12345);
    assert_eq!(target(250), None);
    host.properties.borrow_mut().insert((PROXIED, ATOMS.aware), 5);
    assert_eq!(
        target(250),
        Some(XdndTargetInfo { version: 5, target_window: PROXIED, message_window: PROXIED, in_process: false })
    );

    // Versions below 3 and values that are no version do not take part.
    for version in [0, 1, 2, 256, 100000] {
        host.properties.borrow_mut().insert((OLD, ATOMS.aware), version);
        assert_eq!(target(350), None, "version {version}");
    }
    host.properties.borrow_mut().insert((OLD, ATOMS.aware), 3);
    assert_eq!(target(350).map(|target| target.version), Some(3));
    // A target with a newer version is spoken to in ours.
    host.properties.borrow_mut().insert((OLD, ATOMS.aware), 9);
    assert_eq!(target(350).map(|target| target.version), Some(9));
    host.take();
    handler.on_pointer_moved(at(350), 1, NO_MODIFIERS);
    assert_eq!(host.take()[0], Call::Send(ATOMS.enter, OLD, [SRC, 5 << 24, 201, 202, 0]));
}

#[test]
fn positions_are_sent_one_at_a_time_and_the_drop_after_the_last_answer() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    host.take();

    // Over the target: enter, then the position with the action of the effects.
    handler.on_pointer_moved(at(150), 1000, NO_MODIFIERS);
    assert_eq!(
        host.take(),
        [
            Call::Send(ATOMS.enter, CLIENT, [SRC, 5 << 24, 201, 202, 0]),
            Call::RestartTimeout,
            Call::Send(ATOMS.position, CLIENT, [SRC, 0, packed(150), 1000, COPY]),
        ]
    );

    // More movement before the answer: nothing is sent, the last position is kept.
    handler.on_pointer_moved(at(160), 1001, NO_MODIFIERS);
    handler.on_pointer_moved(at(170), 1002, RawInputModifiers::SHIFT);
    assert!(host.take().is_empty());

    // The answer: the cursor of the accepted effect, and the kept position goes out.
    handler.on_xdnd_status(status(CLIENT, true, COPY));
    assert_eq!(
        host.take(),
        [
            Call::StopTimeout,
            Call::Cursor(DragDropEffects::COPY),
            Call::RestartTimeout,
            Call::Send(ATOMS.position, CLIENT, [SRC, 0, packed(170), 1002, MOVE]),
        ]
    );

    // Released before the answer to that: the drop waits.
    handler.on_pointer_released(at(170), 1003, NO_MODIFIERS);
    assert_eq!(host.take(), [Call::Ungrab]);
    assert_eq!(result(&handler), None);
    // Movement after the release is ignored.
    handler.on_pointer_moved(at(450), 1004, NO_MODIFIERS);
    assert!(host.take().is_empty());

    handler.on_xdnd_status(status(CLIENT, true, MOVE));
    assert_eq!(
        host.take(),
        [
            Call::StopTimeout,
            // The pointer is not grabbed any more: no cursor.
            Call::RestartTimeout,
            Call::Activity(true),
            Call::Send(ATOMS.drop, CLIENT, [SRC, 0, 1003, 0, 0]),
        ]
    );
    assert_eq!(result(&handler), None);

    // The target finished: its action is the result.
    handler.on_xdnd_finished(finished(CLIENT, true, MOVE));
    assert_eq!(host.take(), [Call::Activity(false), Call::StopTimeout]);
    assert_eq!(result(&handler), Some(DragDropEffects::MOVE));

    handler.dispose();
    assert_eq!(host.take(), [Call::Dispose]);
    // Disposed once.
    handler.dispose();
    assert!(host.take().is_empty());
}

#[test]
fn a_drop_on_a_target_that_answered_goes_out_at_once() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    handler.on_pointer_moved(at(150), 1, NO_MODIFIERS);
    handler.on_xdnd_status(status(CLIENT, true, LINK));
    host.take();

    handler.on_pointer_released(at(150), 2, NO_MODIFIERS);
    assert_eq!(
        host.take(),
        [
            Call::Ungrab,
            Call::RestartTimeout,
            Call::Activity(true),
            Call::Send(ATOMS.drop, CLIENT, [SRC, 0, 2, 0, 0])
        ]
    );
    // A target that finishes without accepting: no effect.
    handler.on_xdnd_finished(finished(CLIENT, false, COPY));
    assert_eq!(result(&handler), Some(DragDropEffects::NONE));
}

#[test]
fn a_release_over_a_target_that_refused_leaves_it() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    handler.on_pointer_moved(at(150), 1, NO_MODIFIERS);
    // Not accepted: the action of the message does not count.
    handler.on_xdnd_status(status(CLIENT, false, COPY));
    assert_eq!(host.take().last(), Some(&Call::Cursor(DragDropEffects::NONE)));

    handler.on_pointer_released(at(150), 2, NO_MODIFIERS);
    assert_eq!(host.take(), [Call::Ungrab, Call::Send(ATOMS.leave, CLIENT, [SRC, 0, 0, 0, 0])]);
    assert_eq!(result(&handler), Some(DragDropEffects::NONE));
}

#[test]
fn a_release_over_nothing_ends_the_drag_without_an_effect() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    handler.on_pointer_moved(at(450), 1, NO_MODIFIERS);
    host.take();
    handler.on_pointer_released(at(450), 2, NO_MODIFIERS);
    assert_eq!(host.take(), [Call::Ungrab]);
    assert_eq!(result(&handler), Some(DragDropEffects::NONE));
}

#[test]
fn moving_on_leaves_the_old_target_and_enters_the_new_one() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    handler.on_pointer_moved(at(150), 1, NO_MODIFIERS);
    handler.on_xdnd_status(status(CLIENT, true, COPY));
    host.take();

    // To the window with a proxy: every message goes to the proxy.
    handler.on_pointer_moved(at(250), 2, NO_MODIFIERS);
    assert_eq!(
        host.take(),
        [
            Call::Send(ATOMS.leave, CLIENT, [SRC, 0, 0, 0, 0]),
            // Back to the effects of the modifiers until the new target answers.
            Call::Cursor(ALL),
            Call::Send(ATOMS.enter, PROXY, [SRC, 5 << 24, 201, 202, 0]),
            Call::RestartTimeout,
            Call::Send(ATOMS.position, PROXY, [SRC, 0, packed(250), 2, COPY]),
        ]
    );
    // The answer names the target window, not the proxy.
    handler.on_xdnd_status(status(PROXY, true, COPY));
    assert!(host.take().is_empty());
    handler.on_xdnd_status(status(PROXIED, true, COPY));
    assert_eq!(host.take(), [Call::StopTimeout, Call::Cursor(DragDropEffects::COPY)]);

    // Off every target.
    handler.on_pointer_moved(at(450), 3, NO_MODIFIERS);
    assert_eq!(host.take(), [Call::Send(ATOMS.leave, PROXY, [SRC, 0, 0, 0, 0]), Call::Cursor(ALL)]);
    // An answer of the target that was left is ignored.
    handler.on_xdnd_status(status(PROXIED, true, MOVE));
    handler.on_xdnd_finished(finished(PROXIED, true, MOVE));
    assert!(host.take().is_empty());
    assert_eq!(result(&handler), None);
}

#[test]
fn more_than_three_formats_are_flagged_in_the_enter_message() {
    let host = FakeHost::new();
    *host.formats.borrow_mut() = vec![201, 202, 203, 204];
    let handler = start(&host, ALL, KeyModifiers::empty());
    host.take();
    handler.on_pointer_moved(at(150), 1, NO_MODIFIERS);
    assert_eq!(host.take()[0], Call::Send(ATOMS.enter, CLIENT, [SRC, (5 << 24) | 1, 201, 202, 203]));

    let host = FakeHost::new();
    *host.formats.borrow_mut() = vec![];
    let handler = start(&host, ALL, KeyModifiers::empty());
    host.take();
    handler.on_pointer_moved(at(150), 1, NO_MODIFIERS);
    assert_eq!(host.take()[0], Call::Send(ATOMS.enter, CLIENT, [SRC, 5 << 24, 0, 0, 0]));
}

#[test]
fn a_target_of_version_4_finishes_without_an_action() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    handler.on_pointer_moved(at(350), 1, NO_MODIFIERS);
    assert_eq!(host.take()[2], Call::Send(ATOMS.enter, OLD, [SRC, 4 << 24, 201, 202, 0]));
    handler.on_xdnd_status(status(OLD, true, LINK));
    handler.on_pointer_released(at(350), 2, NO_MODIFIERS);
    // The message of such a target has no result: the effect of its last answer stands.
    handler.on_xdnd_finished(finished(OLD, false, 0));
    assert_eq!(result(&handler), Some(DragDropEffects::LINK));
}

#[test]
fn the_answer_of_a_target_is_limited_to_the_allowed_effects() {
    let host = FakeHost::new();
    let handler = start(&host, DragDropEffects::COPY, KeyModifiers::empty());
    handler.on_pointer_moved(at(150), 1, NO_MODIFIERS);
    host.take();
    handler.on_xdnd_status(status(CLIENT, true, MOVE));
    // Move is not allowed: the cursor says "no drop", but the target allows a drop.
    assert_eq!(host.take(), [Call::StopTimeout, Call::Cursor(DragDropEffects::NONE)]);
    handler.on_pointer_released(at(150), 2, NO_MODIFIERS);
    handler.on_xdnd_finished(finished(CLIENT, true, MOVE));
    assert_eq!(result(&handler), Some(DragDropEffects::NONE));
}

#[test]
fn a_window_of_the_application_gets_drag_events_and_no_messages() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    host.take();

    host.raw_answer.set(DragDropEffects::MOVE);
    handler.on_pointer_moved(at(50), 1, NO_MODIFIERS);
    assert_eq!(
        host.take(),
        [
            Call::Raw(OWN, RawDragEventType::DragEnter, at(50), ALL),
            Call::Cursor(DragDropEffects::MOVE),
            Call::Raw(OWN, RawDragEventType::DragOver, at(50), ALL),
        ]
    );
    handler.on_pointer_moved(at(60), 2, RawInputModifiers::CONTROL);
    assert_eq!(host.take(), [Call::Raw(OWN, RawDragEventType::DragOver, at(60), DragDropEffects::COPY)]);

    handler.on_pointer_released(at(60), 3, NO_MODIFIERS);
    assert_eq!(host.take(), [Call::Ungrab, Call::Raw(OWN, RawDragEventType::Drop, at(60), ALL)]);
    assert_eq!(result(&handler), Some(DragDropEffects::MOVE));
}

#[test]
fn a_window_of_the_application_that_refuses_is_left_on_release() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    host.raw_answer.set(DragDropEffects::NONE);
    handler.on_pointer_moved(at(50), 1, NO_MODIFIERS);
    host.take();
    handler.on_pointer_released(at(50), 2, NO_MODIFIERS);
    assert_eq!(host.take(), [Call::Ungrab, Call::Raw(OWN, RawDragEventType::DragLeave, at(50), ALL)]);
    assert_eq!(result(&handler), Some(DragDropEffects::NONE));

    // Leaving such a window for another target raises the leave too.
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    handler.on_pointer_moved(at(50), 1, NO_MODIFIERS);
    host.take();
    handler.on_pointer_moved(at(150), 2, NO_MODIFIERS);
    assert_eq!(host.take()[0], Call::Raw(OWN, RawDragEventType::DragLeave, at(150), ALL));
}

#[test]
fn an_effect_the_handlers_choose_outside_the_allowed_ones_does_not_count() {
    let host = FakeHost::new();
    let handler = start(&host, DragDropEffects::COPY, KeyModifiers::empty());
    host.raw_answer.set(DragDropEffects::MOVE);
    handler.on_pointer_moved(at(50), 1, NO_MODIFIERS);
    handler.on_pointer_released(at(50), 2, NO_MODIFIERS);
    assert!(host.take().contains(&Call::Raw(OWN, RawDragEventType::DragLeave, at(50), DragDropEffects::COPY)));
    assert_eq!(result(&handler), Some(DragDropEffects::NONE));
}

#[test]
fn a_target_that_does_not_answer_times_out() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    handler.on_pointer_moved(at(150), 1, NO_MODIFIERS);
    assert!(host.take().contains(&Call::RestartTimeout));
    handler.on_timeout();
    assert_eq!(result(&handler), Some(DragDropEffects::NONE));
    // The handler is disposed by whoever awaited the drag; the pointer is released then.
    handler.dispose();
    assert_eq!(host.take(), [Call::Ungrab, Call::Dispose]);
}

fn client_message(window: XID, message_type: Atom, data: [c_long; 5]) -> XEvent {
    let mut evt = xlib::new_event();
    let message = xlib::client_message_event_mut(&mut evt);
    message.type_ = XEventName::ClientMessage as i32;
    message.window = window;
    message.message_type = message_type;
    message.format = 32;
    for (index, value) in data.into_iter().enumerate() {
        message.data.set_long(index, value);
    }
    evt
}

fn pointer_event(type_: XEventName, window: XID, x_root: i32, time: Time, state: u32) -> XEvent {
    let mut evt = xlib::new_event();
    if type_ == XEventName::MotionNotify {
        let motion = xlib::motion_event_mut(&mut evt);
        motion.type_ = type_ as i32;
        motion.window = window;
        motion.x_root = x_root;
        motion.y_root = 40;
        motion.time = time;
        motion.state = state;
    } else {
        let button = xlib::button_event_mut(&mut evt);
        button.type_ = type_ as i32;
        button.window = window;
        button.x_root = x_root;
        button.y_root = 40;
        button.time = time;
        button.state = state;
        button.button = 1;
    }
    evt
}

#[test]
fn the_hook_takes_the_pointer_while_it_is_grabbed_and_the_messages_of_the_source_window() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    host.take();

    // Pointer events of any window are the drag's while the pointer is grabbed.
    assert!(handler.try_handle_event(&pointer_event(XEventName::MotionNotify, CLIENT, 150, 77, 0)));
    assert_eq!(
        host.take().last(),
        Some(&Call::Send(ATOMS.position, CLIENT, [SRC, 0, packed(150), 77, COPY]))
    );
    assert!(handler.try_handle_event(&pointer_event(XEventName::ButtonPress, CLIENT, 150, 78, 0)));
    assert!(host.take().is_empty());

    // The answers arrive as messages to the source window; other messages there are not the drag's.
    assert!(handler.try_handle_event(&client_message(SOURCE, ATOMS.status, status(CLIENT, true, COPY))));
    assert_eq!(host.take(), [Call::StopTimeout, Call::Cursor(DragDropEffects::COPY)]);
    assert!(!handler.try_handle_event(&client_message(SOURCE, 999, [0; 5])));
    // The same message to another window is not looked at.
    assert!(!handler.try_handle_event(&client_message(OWN, ATOMS.status, status(CLIENT, false, 0))));
    assert!(host.take().is_empty());

    // The target asks for the data.
    let mut request = xlib::new_event();
    {
        let selection_request = xlib::selection_request_event_mut(&mut request);
        selection_request.type_ = XEventName::SelectionRequest as i32;
        selection_request.owner = SOURCE;
    }
    assert_eq!(xlib::event_window(&request), SOURCE);
    assert!(handler.try_handle_event(&request));
    assert_eq!(host.take(), [Call::SelectionRequest]);

    // The release with the control key down ends the grab; the drop goes out.
    assert!(handler.try_handle_event(&pointer_event(XEventName::ButtonRelease, CLIENT, 150, 79, 4)));
    assert_eq!(host.take().last(), Some(&Call::Send(ATOMS.drop, CLIENT, [SRC, 0, 79, 0, 0])));
    // After the release pointer events pass through again.
    assert!(!handler.try_handle_event(&pointer_event(XEventName::MotionNotify, CLIENT, 160, 80, 0)));
    assert!(handler.try_handle_event(&client_message(SOURCE, ATOMS.finished, finished(CLIENT, true, COPY))));
    assert_eq!(result(&handler), Some(DragDropEffects::COPY));
}

#[test]
fn a_modifier_key_changes_the_action_that_is_asked_for() {
    let host = FakeHost::new();
    let handler = start(&host, ALL, KeyModifiers::empty());
    handler.on_pointer_moved(at(150), 1, NO_MODIFIERS);
    handler.on_xdnd_status(status(CLIENT, true, COPY));
    host.take();

    let key = |type_: XEventName, state: u32, time: Time| {
        let mut evt = xlib::new_event();
        let key = xlib::key_event_mut(&mut evt);
        key.type_ = type_ as i32;
        key.window = CLIENT;
        key.x_root = 150;
        key.y_root = 40;
        key.state = state;
        key.time = time;
        evt
    };

    // Shift goes down: the state of the event does not have it yet.
    host.keysym.set(XK_SHIFT_L);
    assert!(handler.try_handle_event(&key(XEventName::KeyPress, 0, 5)));
    assert_eq!(
        host.take(),
        [Call::RestartTimeout, Call::Send(ATOMS.position, CLIENT, [SRC, 0, packed(150), 5, MOVE])]
    );
    handler.on_xdnd_status(status(CLIENT, true, MOVE));
    host.take();

    // Shift goes up: the state of the event still has it.
    assert!(handler.try_handle_event(&key(XEventName::KeyRelease, 1, 6)));
    assert_eq!(
        host.take(),
        [Call::RestartTimeout, Call::Send(ATOMS.position, CLIENT, [SRC, 0, packed(150), 6, COPY])]
    );
    handler.on_xdnd_status(status(CLIENT, true, COPY));
    host.take();

    // Control and alt.
    host.keysym.set(XK_CONTROL_R);
    handler.try_handle_event(&key(XEventName::KeyPress, 0, 7));
    assert_eq!(host.take()[1], Call::Send(ATOMS.position, CLIENT, [SRC, 0, packed(150), 7, COPY]));
    handler.on_xdnd_status(status(CLIENT, true, COPY));
    host.keysym.set(XK_ALT_L);
    host.take();
    handler.try_handle_event(&key(XEventName::KeyPress, 0, 8));
    assert_eq!(host.take()[1], Call::Send(ATOMS.position, CLIENT, [SRC, 0, packed(150), 8, LINK]));
    handler.on_xdnd_status(status(CLIENT, true, LINK));
    host.take();

    // Another key is taken by the drag and changes nothing.
    host.keysym.set(0x61);
    assert!(handler.try_handle_event(&key(XEventName::KeyPress, 0, 9)));
    assert!(host.take().is_empty());
}

#[test]
fn a_position_is_packed_into_one_item() {
    assert_eq!(position_to_message(PixelPoint::new(300, 250)), (300 << 16) | 250);
    assert_eq!(position_to_message(PixelPoint::new(0, 0)), 0);
}
