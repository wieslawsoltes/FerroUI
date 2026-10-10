// Not from the reference, which has no tests for this file. The drop
// target runs here against a connection that records what it is asked to
// do, a window at a known place and a drag and drop device that records
// the drag events and answers with the effects a test chooses.

use super::*;
use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::{FocusManager, IInputDevice, IInputRoot, InputElement, KeyModifiers};
use ferroui_base::Ref;
use std::any::Any;
use std::cell::Cell;

const WINDOW: XID = 0x400001;
const SOURCE: XID = 0x600007;
const OTHER_SOURCE: XID = 0x700009;

const ENTER: Atom = 1;
const POSITION: Atom = 2;
const LEAVE: Atom = 3;
const DROP: Atom = 4;
const STATUS: Atom = 10;
const FINISHED: Atom = 11;
const COPY: Atom = 101;
const MOVE: Atom = 102;
const LINK: Atom = 103;
const TEXT: Atom = 201;
const URI_LIST: Atom = 202;
const HTML: Atom = 203;
const PNG: Atom = 204;

const ATOMS: XdndTargetAtoms = XdndTargetAtoms {
    status: STATUS,
    finished: FINISHED,
    actions: XdndActions { copy: COPY, move_: MOVE, link: LINK },
};

struct FakeItems {
    disposed: Cell<u32>,
}

impl IDragDropItemsSource for FakeItems {
    fn create_items(self: Rc<Self>) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        Ok(vec![PlatformDataTransferItem::create(&DataFormat::text(), "dragged".to_string())])
    }

    fn dispose(&self) {
        self.disposed.set(self.disposed.get() + 1);
    }
}

#[derive(Default)]
struct FakeConnection {
    aware: RefCell<Vec<(XID, u8)>>,
    type_list: RefCell<Option<Vec<Atom>>>,
    type_list_asked_of: RefCell<Vec<XID>>,
    readers: RefCell<Vec<(Vec<Atom>, XID, Rc<FakeItems>)>>,
    sent: RefCell<Vec<(XID, Atom, [c_long; 5])>>,
    modifiers: Cell<RawInputModifiers>,
}

impl IXdndTargetConnection for FakeConnection {
    fn set_xdnd_aware(&self, window: XID, version: u8) {
        self.aware.borrow_mut().push((window, version));
    }

    fn get_type_list(&self, source_window: XID) -> Option<Vec<Atom>> {
        self.type_list_asked_of.borrow_mut().push(source_window);
        self.type_list.borrow().clone()
    }

    fn create_reader(
        &self,
        format_atoms: &[Atom],
        target_window: XID,
    ) -> (Rc<dyn IDragDropItemsSource>, Vec<DataFormat>) {
        let items = Rc::new(FakeItems { disposed: Cell::new(0) });
        self.readers.borrow_mut().push((format_atoms.to_vec(), target_window, items.clone()));
        let data_formats = if format_atoms.contains(&TEXT) { vec![DataFormat::text().as_data_format().clone()] } else { vec![] };
        (items, data_formats)
    }

    fn send_client_message(&self, window: XID, message_type: Atom, data: [c_long; 5]) {
        self.sent.borrow_mut().push((window, message_type, data));
    }

    fn query_modifiers(&self, _window: XID) -> RawInputModifiers {
        self.modifiers.get()
    }
}

impl FakeConnection {
    fn take_sent(&self) -> Vec<(XID, Atom, [c_long; 5])> {
        std::mem::take(&mut *self.sent.borrow_mut())
    }

    fn last_reader(&self) -> (Vec<Atom>, XID, Rc<FakeItems>) {
        self.readers.borrow().last().cloned().expect("a reader was made")
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
        unreachable!("the drop target does not ask for the root element")
    }

    fn focus_root(&self) -> Ref<InputElement> {
        unreachable!("the drop target does not ask for the focus root")
    }

    fn pointer_over_invalidated(&self) {}
}

/// A window whose client area starts at (100, 50) of the screen, at
/// scaling 2.
struct FakeWindow {
    has_input_root: Cell<bool>,
}

impl IXdndWindow for FakeWindow {
    fn handle(&self) -> XID {
        WINDOW
    }

    fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.has_input_root.get().then(|| Rc::new(TestRoot) as Rc<dyn IInputRoot>)
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        Point::new(f64::from(point.x - 100) / 2.0, f64::from(point.y - 50) / 2.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Seen {
    type_: RawDragEventType,
    location: Point,
    effects: DragDropEffects,
    key_modifiers: KeyModifiers,
    formats: usize,
    text: Option<String>,
}

/// Records the drag events and answers each with the effects of `answer`.
struct RecordingDevice {
    seen: RefCell<Vec<Seen>>,
    answer: Cell<DragDropEffects>,
    read_text: Cell<bool>,
}

impl IInputDevice for RecordingDevice {
    fn process_raw_event(&self, ev: &dyn IRawInputEventArgs) {
        let drag = ev.query_args(std::any::TypeId::of::<RawDragEvent>()).and_then(|args| args.downcast_ref::<RawDragEvent>());
        let Some(drag) = drag else {
            panic!("the drop target raises drag events only");
        };
        let text = self.read_text.get().then(|| {
            use ferroui_base::input::DataTransferExtensions;
            drag.data_transfer().try_get_text()
        });
        self.seen.borrow_mut().push(Seen {
            type_: drag.type_(),
            location: drag.location(),
            effects: drag.effects(),
            key_modifiers: drag.key_modifiers(),
            formats: drag.data_transfer().formats().len(),
            text: text.flatten(),
        });
        drag.set_effects(self.answer.get());
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IDragDropDevice for RecordingDevice {}

impl RecordingDevice {
    fn take_seen(&self) -> Vec<Seen> {
        std::mem::take(&mut *self.seen.borrow_mut())
    }
}

struct Fixture {
    connection: Rc<FakeConnection>,
    device: Rc<RecordingDevice>,
    window: Rc<FakeWindow>,
    target: X11DropTarget,
}

fn fixture() -> Fixture {
    let connection = Rc::new(FakeConnection::default());
    let device = Rc::new(RecordingDevice {
        seen: RefCell::new(Vec::new()),
        answer: Cell::new(DragDropEffects::COPY),
        read_text: Cell::new(false),
    });
    let window = Rc::new(FakeWindow { has_input_root: Cell::new(true) });
    let weak: Weak<dyn IXdndWindow> = Rc::downgrade(&(window.clone() as Rc<dyn IXdndWindow>));
    let target = X11DropTarget::new(device.clone(), weak, WINDOW, connection.clone(), ATOMS);
    Fixture { connection, device, window, target }
}

fn message(message_type: Atom, data: [c_long; 5]) -> XClientMessageEvent {
    let mut evt = xlib::new_event();
    {
        let message = xlib::client_message_event_mut(&mut evt);
        message.type_ = XEventName::ClientMessage as i32;
        message.window = WINDOW;
        message.message_type = message_type;
        message.format = 32;
        for (index, value) in data.into_iter().enumerate() {
            message.data.set_long(index, value);
        }
    }
    *xlib::client_message_event(&evt)
}

fn enter(source: XID, version: c_long, extra_formats: bool, formats: [Atom; 3]) -> XClientMessageEvent {
    message(
        ENTER,
        [
            source as c_long,
            (version << 24) | c_long::from(extra_formats),
            formats[0] as c_long,
            formats[1] as c_long,
            formats[2] as c_long,
        ],
    )
}

fn position(source: XID, x: c_long, y: c_long, time: c_long, action: Atom) -> XClientMessageEvent {
    message(POSITION, [source as c_long, 0, (x << 16) | y, time, action as c_long])
}

#[test]
fn the_window_is_announced_with_the_version_of_the_protocol() {
    let f = fixture();
    assert_eq!(*f.connection.aware.borrow(), [(WINDOW, 5)]);
    assert!(f.connection.take_sent().is_empty());
}

#[test]
fn the_fields_of_an_enter_message() {
    let parsed = XdndEnter::parse([SOURCE as c_long, (5 << 24) | 1, TEXT as c_long, 0, HTML as c_long]);
    assert_eq!(
        parsed,
        XdndEnter { source_window: SOURCE, version: 5, has_extra_formats: true, formats: vec![TEXT, HTML] }
    );
    let parsed = XdndEnter::parse([SOURCE as c_long, 3 << 24, 0, 0, 0]);
    assert_eq!(parsed, XdndEnter { source_window: SOURCE, version: 3, has_extra_formats: false, formats: vec![] });
}

#[test]
fn a_position_is_two_halves_of_one_item() {
    assert_eq!(position_from_message((300 << 16) | 250), PixelPoint::new(300, 250));
    assert_eq!(position_from_message(0), PixelPoint::new(0, 0));
    assert_eq!(position_from_message((0xFFFF << 16) | 0xFFFF), PixelPoint::new(65535, 65535));
}

#[test]
fn the_first_position_enters_and_the_next_ones_move_over() {
    let f = fixture();
    f.target.on_xdnd_enter(&enter(SOURCE, 5, false, [TEXT, 0, 0]));
    // Entering alone raises nothing and answers nothing: the position is not known yet.
    assert!(f.device.take_seen().is_empty());
    assert!(f.connection.take_sent().is_empty());
    assert_eq!(f.connection.last_reader().0, [TEXT]);
    assert_eq!(f.connection.last_reader().1, WINDOW);

    f.connection.modifiers.set(RawInputModifiers::CONTROL | RawInputModifiers::LEFT_MOUSE_BUTTON);
    f.target.on_xdnd_position(&position(SOURCE, 300, 250, 1234, MOVE));
    assert_eq!(
        f.device.take_seen(),
        [Seen {
            type_: RawDragEventType::DragEnter,
            // The point of the screen in the coordinates of the window.
            location: Point::new(100.0, 100.0),
            // What the source asks for.
            effects: DragDropEffects::MOVE,
            key_modifiers: KeyModifiers::CONTROL,
            formats: 1,
            text: None,
        }]
    );
    // The answer of the handlers goes back as the status: accepted, with the action.
    assert_eq!(f.connection.take_sent(), [(SOURCE, STATUS, [WINDOW as c_long, 1, 0, 0, COPY as c_long])]);

    f.device.answer.set(DragDropEffects::NONE);
    f.connection.modifiers.set(RawInputModifiers::empty());
    f.target.on_xdnd_position(&position(SOURCE, 102, 54, 1240, COPY));
    let seen = f.device.take_seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].type_, RawDragEventType::DragOver);
    assert_eq!(seen[0].location, Point::new(1.0, 2.0));
    assert_eq!(seen[0].effects, DragDropEffects::COPY);
    assert_eq!(seen[0].key_modifiers, KeyModifiers::empty());
    // Refused: not accepted, no action.
    assert_eq!(f.connection.take_sent(), [(SOURCE, STATUS, [WINDOW as c_long, 0, 0, 0, 0])]);

    // Of several effects the action is the first of copy, move, link.
    f.device.answer.set(DragDropEffects::MOVE | DragDropEffects::LINK);
    f.target.on_xdnd_position(&position(SOURCE, 102, 54, 1250, 0));
    assert_eq!(f.device.take_seen()[0].effects, DragDropEffects::NONE);
    assert_eq!(f.connection.take_sent(), [(SOURCE, STATUS, [WINDOW as c_long, 1, 0, 0, MOVE as c_long])]);
}

#[test]
fn a_source_with_a_version_out_of_range_is_ignored() {
    for version in [0, 1, 2, 6, 255] {
        let f = fixture();
        f.target.on_xdnd_enter(&enter(SOURCE, version, false, [TEXT, 0, 0]));
        f.target.on_xdnd_position(&position(SOURCE, 300, 250, 1, COPY));
        assert!(f.device.take_seen().is_empty(), "version {version}");
        assert!(f.connection.take_sent().is_empty(), "version {version}");
        assert!(f.connection.readers.borrow().is_empty(), "version {version}");
    }
    for version in [3, 4, 5] {
        let f = fixture();
        f.target.on_xdnd_enter(&enter(SOURCE, version, false, [TEXT, 0, 0]));
        f.target.on_xdnd_position(&position(SOURCE, 300, 250, 1, COPY));
        assert_eq!(f.device.take_seen().len(), 1, "version {version}");
    }
}

#[test]
fn a_window_without_an_input_root_takes_no_drag() {
    let f = fixture();
    f.window.has_input_root.set(false);
    f.target.on_xdnd_enter(&enter(SOURCE, 5, false, [TEXT, 0, 0]));
    f.target.on_xdnd_position(&position(SOURCE, 300, 250, 1, COPY));
    assert!(f.device.take_seen().is_empty());
    assert!(f.connection.take_sent().is_empty());
}

#[test]
fn more_than_three_formats_are_read_from_the_source_window() {
    // The flag is set and the source lists its formats: the list counts, not the message.
    let f = fixture();
    *f.connection.type_list.borrow_mut() = Some(vec![TEXT, URI_LIST, 0, HTML, PNG, TEXT]);
    f.target.on_xdnd_enter(&enter(SOURCE, 5, true, [PNG, 0, 0]));
    assert_eq!(*f.connection.type_list_asked_of.borrow(), [SOURCE]);
    // Without the empty entries, and each format once.
    assert_eq!(f.connection.last_reader().0, [TEXT, URI_LIST, HTML, PNG]);

    // The flag is set and the property is not there: the formats of the message.
    let f = fixture();
    f.target.on_xdnd_enter(&enter(SOURCE, 5, true, [PNG, 0, HTML]));
    assert_eq!(f.connection.last_reader().0, [PNG, HTML]);

    // No flag: the property is not asked for.
    let f = fixture();
    *f.connection.type_list.borrow_mut() = Some(vec![TEXT]);
    f.target.on_xdnd_enter(&enter(SOURCE, 5, false, [PNG, PNG, HTML]));
    assert!(f.connection.type_list_asked_of.borrow().is_empty());
    assert_eq!(f.connection.last_reader().0, [PNG, HTML]);
}

#[test]
fn messages_of_another_source_are_ignored() {
    let f = fixture();
    f.target.on_xdnd_enter(&enter(SOURCE, 5, false, [TEXT, 0, 0]));
    f.target.on_xdnd_position(&position(OTHER_SOURCE, 300, 250, 1, COPY));
    f.target.on_xdnd_leave(&message(LEAVE, [OTHER_SOURCE as c_long, 0, 0, 0, 0]));
    f.target.on_xdnd_drop(&message(DROP, [OTHER_SOURCE as c_long, 0, 1, 0, 0]));
    assert!(f.device.take_seen().is_empty());
    assert!(f.connection.take_sent().is_empty());
    assert_eq!(f.connection.last_reader().2.disposed.get(), 0);

    // And so are messages before any drag entered.
    let f = fixture();
    f.target.on_xdnd_position(&position(SOURCE, 300, 250, 1, COPY));
    f.target.on_xdnd_drop(&message(DROP, [SOURCE as c_long, 0, 1, 0, 0]));
    assert!(f.device.take_seen().is_empty());
    assert!(f.connection.take_sent().is_empty());
}

#[test]
fn leaving_ends_the_drag_without_a_result() {
    let f = fixture();
    f.target.on_xdnd_enter(&enter(SOURCE, 5, false, [TEXT, 0, 0]));
    f.target.on_xdnd_position(&position(SOURCE, 300, 250, 1, COPY));
    f.device.take_seen();
    f.connection.take_sent();

    f.target.on_xdnd_leave(&message(LEAVE, [SOURCE as c_long, 0, 0, 0, 0]));
    let seen = f.device.take_seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].type_, RawDragEventType::DragLeave);
    assert_eq!(seen[0].location, Point::default());
    assert_eq!(seen[0].effects, DragDropEffects::NONE);
    // Nothing is sent back, and the data is released.
    assert!(f.connection.take_sent().is_empty());
    assert_eq!(f.connection.last_reader().2.disposed.get(), 1);

    // The drag is over: a late position does nothing.
    f.target.on_xdnd_position(&position(SOURCE, 300, 250, 2, COPY));
    assert!(f.device.take_seen().is_empty());
}

#[test]
fn a_drop_is_raised_where_the_pointer_last_was_and_its_result_goes_back_as_finished() {
    let f = fixture();
    f.device.read_text.set(true);
    f.target.on_xdnd_enter(&enter(SOURCE, 5, false, [TEXT, 0, 0]));
    f.device.answer.set(DragDropEffects::COPY | DragDropEffects::MOVE);
    f.target.on_xdnd_position(&position(SOURCE, 300, 250, 1, COPY));
    f.device.take_seen();
    f.connection.take_sent();

    f.connection.modifiers.set(RawInputModifiers::SHIFT);
    f.device.answer.set(DragDropEffects::MOVE);
    f.target.on_xdnd_drop(&message(DROP, [SOURCE as c_long, 0, 2, 0, 0]));
    assert_eq!(
        f.device.take_seen(),
        [Seen {
            type_: RawDragEventType::Drop,
            location: Point::new(100.0, 100.0),
            // What the handlers answered to the last position.
            effects: DragDropEffects::COPY | DragDropEffects::MOVE,
            key_modifiers: KeyModifiers::SHIFT,
            formats: 1,
            // The data can be read while the drop is handled.
            text: Some("dragged".to_string()),
        }]
    );
    // Accepted, with the action the drop handlers chose.
    assert_eq!(f.connection.take_sent(), [(SOURCE, FINISHED, [WINDOW as c_long, 1, MOVE as c_long, 0, 0])]);
    assert_eq!(f.connection.last_reader().2.disposed.get(), 1);

    // The drag is over.
    f.target.on_xdnd_drop(&message(DROP, [SOURCE as c_long, 0, 3, 0, 0]));
    assert!(f.device.take_seen().is_empty());
    assert!(f.connection.take_sent().is_empty());
}

#[test]
fn a_refused_drop_finishes_without_an_action() {
    let f = fixture();
    f.target.on_xdnd_enter(&enter(SOURCE, 5, false, [TEXT, 0, 0]));
    f.target.on_xdnd_position(&position(SOURCE, 300, 250, 1, COPY));
    f.connection.take_sent();

    f.device.answer.set(DragDropEffects::NONE);
    f.target.on_xdnd_drop(&message(DROP, [SOURCE as c_long, 0, 2, 0, 0]));
    assert_eq!(f.connection.take_sent(), [(SOURCE, FINISHED, [WINDOW as c_long, 0, 0, 0, 0])]);
}

#[test]
fn a_new_drag_replaces_one_that_never_ended() {
    let f = fixture();
    f.target.on_xdnd_enter(&enter(SOURCE, 5, false, [TEXT, 0, 0]));
    f.target.on_xdnd_position(&position(SOURCE, 300, 250, 1, COPY));
    let stale = f.connection.last_reader().2;
    f.device.take_seen();
    f.connection.take_sent();

    f.target.on_xdnd_enter(&enter(OTHER_SOURCE, 4, false, [TEXT, 0, 0]));
    // The stale drag is released without a message: it was not dropped.
    assert_eq!(stale.disposed.get(), 1);
    assert!(f.connection.take_sent().is_empty());

    // The new drag starts over: its first position enters.
    f.target.on_xdnd_position(&position(OTHER_SOURCE, 110, 60, 5, COPY));
    assert_eq!(f.device.take_seen()[0].type_, RawDragEventType::DragEnter);
    assert_eq!(f.connection.take_sent(), [(OTHER_SOURCE, STATUS, [WINDOW as c_long, 1, 0, 0, COPY as c_long])]);
    // The old source is no longer listened to.
    f.target.on_xdnd_position(&position(SOURCE, 300, 250, 6, COPY));
    assert!(f.device.take_seen().is_empty());
}
