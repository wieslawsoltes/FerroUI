//! Reading a selection from a window (the port of
//! `SelectionReadSession.cs`).

use crate::selections::i_x_event_waiter::IXEventWaiter;
use crate::selections::selection_helper::{self, ISelectionConnection};
use crate::x11_atoms::X11Atoms;
use crate::x11_structs::XEventName;
use crate::xlib::{self, Atom, Time, WindowProperty, XEvent, XID};
use std::cell::Cell;
use std::io::Cursor;
use std::rc::Rc;

/// The state of the property that notifies a new value (`PropertyNewValue`).
const PROPERTY_NEW_VALUE: i32 = 0;

/// The bytes of a property the reference copies: the number of items times
/// the bytes of the format.
///
/// For the format 32 that is half of what Xlib returns where a C `long`
/// has eight bytes (Xlib returns such items as `long`); the reference
/// copies that much, and so does the port. The targets, which have this
/// format, are not read through this function.
pub fn property_bytes(property: &WindowProperty) -> &[u8] {
    let length = (property.nitems as usize).saturating_mul((property.actual_format / 8).max(0) as usize);
    &property.data[..length.min(property.data.len())]
}

/// The data of a selection target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetDataResult {
    data: Vec<u8>,
    actual_type_atom: Atom,
}

impl GetDataResult {
    pub fn new(data: Vec<u8>, actual_type_atom: Atom) -> Self {
        Self { data, actual_type_atom }
    }

    /// The type of the property the data came in.
    pub fn type_atom(&self) -> Atom {
        self.actual_type_atom
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    pub fn as_stream(&self) -> Cursor<&[u8]> {
        Cursor::new(&self.data)
    }
}

/// The receiving side of an incremental transfer (`INCR`): the parts the
/// owner of the selection writes one after the other, up to an empty one.
#[derive(Debug, Default)]
pub struct IncrReader {
    data: Vec<u8>,
    actual_type_atom: Atom,
}

impl IncrReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the next part. Returns `false` when the part is the empty one
    /// that ends the transfer.
    pub fn append(&mut self, part: &WindowProperty) -> bool {
        if self.actual_type_atom == 0 {
            self.actual_type_atom = part.actual_type;
        }
        if part.nitems == 0 {
            return false;
        }

        self.data.extend_from_slice(property_bytes(part));
        true
    }

    /// The data of the parts, with the type of the first one.
    pub fn finish(self) -> GetDataResult {
        GetDataResult::new(self.data, self.actual_type_atom)
    }
}

/// A session used to read a X11 selection (Clipboard/Drag-and-Drop) from a
/// given window.
pub struct SelectionReadSession {
    connection: Rc<dyn ISelectionConnection>,
    window: XID,
    selection: Atom,
    event_waiter: Rc<dyn IXEventWaiter>,
    targets_atom: Atom,
    incr_atom: Atom,
    disposed: Cell<bool>,
}

impl SelectionReadSession {
    /// The reference takes the connection as its display; the port takes
    /// the calls a session makes on it, and keeps the two atoms it needs.
    pub fn new(
        connection: Rc<dyn ISelectionConnection>,
        window: XID,
        selection: Atom,
        event_waiter: Rc<dyn IXEventWaiter>,
        atoms: &X11Atoms,
    ) -> Self {
        Self {
            connection,
            window,
            selection,
            event_waiter,
            targets_atom: atoms.TARGETS,
            incr_atom: atoms.INCR,
            disposed: Cell::new(false),
        }
    }

    /// Releases the event waiter. Whoever creates a session disposes it
    /// when the read ends (the reference does with a `using` statement).
    pub fn dispose(&self) {
        if !self.disposed.replace(true) {
            self.event_waiter.dispose();
        }
    }

    async fn wait_for_selection_notify_and_get_property(&self, property: Atom) -> Option<WindowProperty> {
        let window = self.window;
        let selection = self.selection;
        let ev = self
            .event_waiter
            .wait_for_event_async(
                Box::new(move |ev: &XEvent| {
                    xlib::event_type(ev) == XEventName::SelectionNotify as i32 && {
                        let selection_event = xlib::selection_event(ev);
                        selection_event.requestor == window
                            && selection_event.selection == selection
                            && selection_event.property == property
                    }
                }),
                selection_helper::TIMEOUT,
            )
            .await;

        ev?;

        Some(self.read_property(property))
    }

    fn read_property(&self, property: Atom) -> WindowProperty {
        self.connection.get_window_property(self.window, property, 0, 0x7fffffff, true, xlib::ANY_PROPERTY_TYPE)
    }

    async fn convert_selection_and_get_property(
        &self,
        target: Atom,
        property: Atom,
        timestamp: Time,
    ) -> Option<WindowProperty> {
        self.connection.convert_selection(self.selection, target, property, self.window, timestamp);
        self.wait_for_selection_notify_and_get_property(property).await
    }

    /// Asks the owner of the selection for its targets. `None` when it
    /// does not answer in time, or answers with something that is not a
    /// list of atoms.
    pub async fn send_format_request(&self, _targets_atom: Atom) -> Option<Vec<Atom>> {
        let res = self.convert_selection_and_get_property(self.targets_atom, self.targets_atom, 0).await?;

        if res.nitems == 0 {
            return None;
        }
        if res.actual_format != 32 {
            return None;
        }

        Some(res.longs())
    }

    async fn read_incr(&self, property: Atom) -> Option<GetDataResult> {
        self.connection.flush();
        let mut reader = IncrReader::new();
        let window = self.window;
        loop {
            let ev = self
                .event_waiter
                .wait_for_event_async(
                    Box::new(move |x: &XEvent| {
                        xlib::event_type(x) == XEventName::PropertyNotify as i32 && {
                            let property_event = xlib::property_event(x);
                            property_event.state == PROPERTY_NEW_VALUE
                                && property_event.window == window
                                && property_event.atom == property
                        }
                    }),
                    selection_helper::TIMEOUT,
                )
                .await;

            ev?;

            let part = self.read_property(property);

            if !reader.append(&part) {
                break;
            }
        }

        Some(reader.finish())
    }

    /// Asks the owner of the selection for the data of a target. `None`
    /// when it does not answer in time or has no data for the target.
    pub async fn send_data_request(&self, format: Atom, timestamp: Time) -> Option<GetDataResult> {
        let res = self.convert_selection_and_get_property(format, format, timestamp).await?;

        if res.nitems == 0 {
            return None;
        }
        if res.actual_type == self.incr_atom {
            self.read_incr(format).await
        } else {
            Some(GetDataResult::new(property_bytes(&res).to_vec(), res.actual_type))
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use crate::selections::selection_helper::test_support::{
        bytes_property, longs_property, run_ready, Call, MockConnection, MockEventWaiter,
    };
    use std::ffi::c_ulong;

    const WINDOW: XID = 77;
    const SELECTION: Atom = 500;

    fn atoms() -> X11Atoms {
        let mut next = 100;
        X11Atoms::with_interned(|_| {
            next += 1;
            next
        })
    }

    fn selection_notify(requestor: XID, selection: Atom, property: Atom) -> Option<XEvent> {
        let mut event = xlib::new_event();
        let selection_event = xlib::selection_event_mut(&mut event);
        selection_event.type_ = XEventName::SelectionNotify as i32;
        selection_event.requestor = requestor;
        selection_event.selection = selection;
        selection_event.property = property;
        Some(event)
    }

    fn property_notify(window: XID, atom: Atom, state: i32) -> Option<XEvent> {
        let mut event = xlib::new_event();
        let property_event = xlib::property_event_mut(&mut event);
        property_event.type_ = XEventName::PropertyNotify as i32;
        property_event.window = window;
        property_event.atom = atom;
        property_event.state = state;
        Some(event)
    }

    fn session(
        connection: &Rc<MockConnection>,
        waiter: &Rc<MockEventWaiter>,
        atoms: &X11Atoms,
    ) -> SelectionReadSession {
        SelectionReadSession::new(connection.clone(), WINDOW, SELECTION, waiter.clone(), atoms)
    }

    fn read(property: Atom) -> Call {
        Call::GetWindowProperty { window: WINDOW, property, delete: true, req_type: xlib::ANY_PROPERTY_TYPE }
    }

    #[test]
    fn the_copied_bytes_are_the_items_times_the_bytes_of_the_format() {
        assert_eq!(property_bytes(&bytes_property(1, b"abc")), b"abc");
        // Half of the items of the format 32 where a long has eight bytes.
        let longs = longs_property(1, &[1, 2]);
        assert_eq!(property_bytes(&longs).len(), 8.min(longs.data.len()));
        // A property that does not exist.
        assert!(property_bytes(&WindowProperty::default()).is_empty());
        // Never more than there is.
        let mut short = bytes_property(1, b"ab");
        short.nitems = 10;
        assert_eq!(property_bytes(&short), b"ab");
    }

    #[test]
    fn the_targets_are_the_atoms_of_the_answer() {
        let atoms = atoms();
        let targets: [c_ulong; 3] = [atoms.TARGETS, atoms.UTF8_STRING, atoms.STRING];
        let connection = MockConnection::new([longs_property(atoms.ATOM, &targets)]);
        let waiter = MockEventWaiter::new([selection_notify(WINDOW, SELECTION, atoms.TARGETS)]);
        let session = session(&connection, &waiter, &atoms);

        assert_eq!(run_ready(session.send_format_request(0)), Some(targets.to_vec()));
        assert_eq!(
            connection.calls(),
            [
                Call::ConvertSelection {
                    selection: SELECTION,
                    target: atoms.TARGETS,
                    property: atoms.TARGETS,
                    requestor: WINDOW,
                    time: 0
                },
                read(atoms.TARGETS),
            ]
        );
        assert_eq!(waiter.rejected.get(), 0);
    }

    #[test]
    fn there_are_no_targets_without_an_answer_or_with_another_format() {
        let atoms = atoms();

        // The owner does not answer: the property is not read.
        let connection = MockConnection::new([]);
        let waiter = MockEventWaiter::new([None]);
        assert_eq!(run_ready(session(&connection, &waiter, &atoms).send_format_request(0)), None);
        assert_eq!(connection.calls().len(), 1);

        // The owner refuses: the property does not exist.
        let connection = MockConnection::new([WindowProperty::default()]);
        let waiter = MockEventWaiter::new([selection_notify(WINDOW, SELECTION, atoms.TARGETS)]);
        assert_eq!(run_ready(session(&connection, &waiter, &atoms).send_format_request(0)), None);

        // The answer is not a list of atoms.
        let connection = MockConnection::new([bytes_property(atoms.STRING, b"text")]);
        let waiter = MockEventWaiter::new([selection_notify(WINDOW, SELECTION, atoms.TARGETS)]);
        assert_eq!(run_ready(session(&connection, &waiter, &atoms).send_format_request(0)), None);
    }

    #[test]
    fn a_notification_of_another_request_is_not_the_answer() {
        let atoms = atoms();
        for event in [
            selection_notify(WINDOW + 1, SELECTION, atoms.UTF8_STRING),
            selection_notify(WINDOW, SELECTION + 1, atoms.UTF8_STRING),
            // A refusal: the property of the notification is zero.
            selection_notify(WINDOW, SELECTION, 0),
            property_notify(WINDOW, atoms.UTF8_STRING, PROPERTY_NEW_VALUE),
        ] {
            let connection = MockConnection::new([bytes_property(atoms.UTF8_STRING, b"text")]);
            let waiter = MockEventWaiter::new([event]);
            assert_eq!(run_ready(session(&connection, &waiter, &atoms).send_data_request(atoms.UTF8_STRING, 0)), None);
            assert_eq!(waiter.rejected.get(), 1);
        }
    }

    #[test]
    fn data_that_fits_a_property_is_read_at_once() {
        let atoms = atoms();
        let connection = MockConnection::new([bytes_property(atoms.UTF8_STRING, b"hello")]);
        let waiter = MockEventWaiter::new([selection_notify(WINDOW, SELECTION, atoms.UTF8_STRING)]);
        let session = session(&connection, &waiter, &atoms);

        let result = run_ready(session.send_data_request(atoms.UTF8_STRING, 12345)).unwrap();
        assert_eq!(result.type_atom(), atoms.UTF8_STRING);
        assert_eq!(result.as_bytes(), b"hello");
        assert_eq!(
            connection.calls(),
            [
                Call::ConvertSelection {
                    selection: SELECTION,
                    target: atoms.UTF8_STRING,
                    property: atoms.UTF8_STRING,
                    requestor: WINDOW,
                    time: 12345
                },
                read(atoms.UTF8_STRING),
            ]
        );
        assert_eq!(waiter.waits.get(), 1);
    }

    #[test]
    fn an_incremental_transfer_is_read_part_by_part_up_to_the_empty_part() {
        let atoms = atoms();
        let target = atoms.UTF8_STRING;
        let connection = MockConnection::new([
            // The announcement, with a lower bound of the size.
            longs_property(atoms.INCR, &[11]),
            bytes_property(target, b"hello "),
            bytes_property(target, b"world"),
            bytes_property(target, b""),
        ]);
        let waiter = MockEventWaiter::new([
            selection_notify(WINDOW, SELECTION, target),
            property_notify(WINDOW, target, PROPERTY_NEW_VALUE),
            property_notify(WINDOW, target, PROPERTY_NEW_VALUE),
            property_notify(WINDOW, target, PROPERTY_NEW_VALUE),
        ]);
        let session = session(&connection, &waiter, &atoms);

        let result = run_ready(session.send_data_request(target, 0)).unwrap();
        assert_eq!(result.type_atom(), target);
        assert_eq!(result.as_bytes(), b"hello world");
        let mut stream = result.as_stream();
        let mut text = String::new();
        std::io::Read::read_to_string(&mut stream, &mut text).unwrap();
        assert_eq!(text, "hello world");

        // Every read deletes the property, which asks for the next part;
        // the deletion of the announcement is flushed before the wait.
        assert_eq!(
            connection.calls()[1..],
            [read(target), Call::Flush, read(target), read(target), read(target)]
        );
        assert_eq!(waiter.waits.get(), 4);
        assert_eq!(waiter.rejected.get(), 0);
    }

    #[test]
    fn an_incremental_transfer_that_stops_has_no_data() {
        let atoms = atoms();
        let target = atoms.UTF8_STRING;
        let connection =
            MockConnection::new([longs_property(atoms.INCR, &[11]), bytes_property(target, b"hello ")]);
        let waiter = MockEventWaiter::new([
            selection_notify(WINDOW, SELECTION, target),
            property_notify(WINDOW, target, PROPERTY_NEW_VALUE),
            // The owner went away: the wait for the next part times out.
            None,
        ]);
        assert_eq!(run_ready(session(&connection, &waiter, &atoms).send_data_request(target, 0)), None);
    }

    #[test]
    fn the_deletion_of_a_part_is_not_a_new_part() {
        let atoms = atoms();
        let target = atoms.UTF8_STRING;
        let connection = MockConnection::new([longs_property(atoms.INCR, &[11])]);
        let waiter = MockEventWaiter::new([
            selection_notify(WINDOW, SELECTION, target),
            // `PropertyDelete`.
            property_notify(WINDOW, target, 1),
        ]);
        assert_eq!(run_ready(session(&connection, &waiter, &atoms).send_data_request(target, 0)), None);
        assert_eq!(waiter.rejected.get(), 1);
    }

    #[test]
    fn the_parts_of_a_transfer_have_the_type_of_the_first() {
        let mut reader = IncrReader::new();
        assert!(reader.append(&bytes_property(5, b"ab")));
        assert!(reader.append(&bytes_property(6, b"cd")));
        assert!(!reader.append(&bytes_property(6, b"")));
        assert_eq!(reader.finish(), GetDataResult::new(b"abcd".to_vec(), 5));

        // A transfer of nothing: the type is the one of the empty part.
        let mut reader = IncrReader::new();
        assert!(!reader.append(&bytes_property(6, b"")));
        assert_eq!(reader.finish(), GetDataResult::new(Vec::new(), 6));
    }

    #[test]
    fn the_session_releases_its_waiter_once() {
        let atoms = atoms();
        let connection = MockConnection::new([]);
        let waiter = MockEventWaiter::new([]);
        let session = session(&connection, &waiter, &atoms);
        assert!(!waiter.disposed.get());
        session.dispose();
        assert!(waiter.disposed.get());
        waiter.disposed.set(false);
        session.dispose();
        assert!(!waiter.disposed.get());
    }
}
