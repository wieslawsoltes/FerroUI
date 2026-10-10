//! Providing a selection to the other clients (the port of
//! `SelectionDataProvider.cs`).

use crate::selections::clipboard::event_stream_window::EventStreamWindow;
use crate::selections::data_format_helper::{self, MIME_TYPE_PNG_FORMAT};
use crate::selections::i_x_event_waiter::IXEventWaiter;
use crate::selections::selection_helper::{self, ISelectionConnection, XlibSelectionConnection};
use crate::selections::uri_list_helper;
use crate::x11_enums::XEventMask;
use crate::x11_info::X11Info;
use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::{EventMask, XEventName};
use crate::xlib::{self, Atom, XDisplay, XEvent, XID};
use ferroui_base::input::platform::ClipboardError;
use ferroui_base::input::{
    AsyncDataTransferExtensions, AsyncDataTransferItemExtensions, DataFormat, IAsyncDataTransfer, LocalBoxFuture,
};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::imaging::PngBitmapEncoderOptions;
use ferroui_base::threading::{Dispatcher, DispatcherFrame};
use std::any::Any;
use std::cell::RefCell;
use std::ffi::{c_long, c_ulong};
use std::fmt::Display;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

/// The state of the property that notifies a deletion (`PropertyDelete`).
const PROPERTY_DELETE: i32 = 1;

/// The most bytes of data that are written to a property at once: more is
/// sent incrementally.
///
/// The sizes are those of `XExtendedMaxRequestSize` and `XMaxRequestSize`,
/// which count units of four bytes; the reference takes them for bytes,
/// which keeps the properties well below what a request may carry.
pub fn maximum_property_size(extended_max_request_size: c_long, max_request_size: c_long) -> i32 {
    let mut max_request_size_ = i64::from(extended_max_request_size);
    if max_request_size_ == 0 {
        max_request_size_ = i64::from(max_request_size);
    }

    0x100000_i64.min(max_request_size_ - 0x100) as i32
}

/// The length of the buffer the shared array pool of the reference's base
/// library rents for a minimum length: the next power of two, and sixteen
/// at least. The parts of an incremental transfer have this length, which
/// can be more than the maximum property size.
pub fn rented_buffer_length(minimum_length: i64) -> usize {
    if minimum_length <= 0 {
        // The pool has an empty array for a length of zero (and throws for
        // a negative one, which the maximum property size of a server
        // never gives).
        return 0;
    }

    (minimum_length as usize).next_power_of_two().max(16)
}

/// The answer to a `TARGETS` request: the two targets of the protocol and
/// the targets of the data.
pub fn targets_answer(targets_atom: Atom, multiple_atom: Atom, format_atoms: &[Atom]) -> Vec<Atom> {
    let mut atom_values = Vec::with_capacity(format_atoms.len() + 2);
    atom_values.push(targets_atom);
    atom_values.push(multiple_atom);
    atom_values.extend_from_slice(format_atoms);
    atom_values
}

/// Answers a `MULTIPLE` request: `pairs` holds a target and a property for
/// every conversion that is asked for; `convert` makes the conversion and
/// returns the property, or zero when it refuses, which replaces the
/// property of the pair.
///
/// A last item without its pair is left as it is (the reference reads
/// past the end of the property for it).
pub fn convert_multiple(pairs: &mut [c_ulong], mut convert: impl FnMut(Atom, Atom) -> Atom) {
    for pair in pairs.chunks_exact_mut(2) {
        let sub_target = pair[0];
        let sub_prop = pair[1];
        let converted = convert(sub_target, sub_prop);
        pair[1] = converted;
    }
}

/// The sending side of an incremental transfer (`INCR`): the announcement
/// of the size, the parts, and the empty part that ends it. The requestor
/// asks for each part by deleting the property.
pub struct IncrSender {
    window: XID,
    property: Atom,
    target: Atom,
    data: Vec<u8>,
    position: usize,
    buffer_length: usize,
}

impl IncrSender {
    pub fn new(window: XID, property: Atom, target: Atom, data: Vec<u8>, maximum_property_size: i32) -> Self {
        let buffer_length = rented_buffer_length(i64::from(maximum_property_size).min(data.len() as i64));
        Self { window, property, target, data, position: 0, buffer_length }
    }

    /// The property the transfer goes through.
    pub fn property(&self) -> Atom {
        self.property
    }

    /// Asks for the property events of the window of the requestor and
    /// announces the transfer with the size of the data.
    pub fn begin(&self, connection: &dyn ISelectionConnection, incr_atom: Atom) {
        connection.select_input(self.window, XEventMask::PROPERTY_CHANGE_MASK.bits() as c_long);

        let size = self.data.len() as c_ulong;
        connection.change_property_longs(self.window, self.property, incr_atom, &[size]);
    }

    /// Writes the next part. Returns `false`, and writes nothing, when
    /// there is no data left.
    pub fn send_next_part(&mut self, connection: &dyn ISelectionConnection) -> bool {
        let read = self.buffer_length.min(self.data.len() - self.position);
        if read == 0 {
            return false;
        }

        let part = &self.data[self.position..self.position + read];
        connection.change_property_bytes(self.window, self.property, self.target, part);
        self.position += read;
        true
    }

    /// Writes the empty part that ends the transfer.
    pub fn finish(&self, connection: &dyn ISelectionConnection) {
        connection.change_property_bytes(self.window, self.property, self.target, &[]);
    }
}

/// Runs an incremental transfer: announces it, and writes a part every
/// time `events` sees the requestor delete the property, until there is no
/// data left or the requestor stops asking. `events` is disposed at the
/// end.
///
/// The announcement is made, and the wait for the first deletion starts,
/// before this returns: the caller notifies the requestor afterwards, and
/// what the requestor does then must not be missed.
pub fn send_incr_data_async(
    mut sender: IncrSender,
    connection: Rc<dyn ISelectionConnection>,
    events: Rc<dyn IXEventWaiter>,
    incr_atom: Atom,
    on_activity: Rc<dyn Fn()>,
) -> LocalBoxFuture<()> {
    sender.begin(&*connection, incr_atom);

    let property = sender.property();
    let wait_for_deletion = move |events: &Rc<dyn IXEventWaiter>| {
        events.wait_for_event_async(
            Box::new(move |x: &XEvent| {
                xlib::event_type(x) == XEventName::PropertyNotify as i32 && {
                    let property_event = xlib::property_event(x);
                    property_event.atom == property && property_event.state == PROPERTY_DELETE
                }
            }),
            selection_helper::TIMEOUT,
        )
    };
    let mut first_wait = Some(wait_for_deletion(&events));

    Box::pin(async move {
        let mut got_timeout = false;

        loop {
            let evt = match first_wait.take() {
                Some(first_wait) => first_wait.await,
                None => wait_for_deletion(&events).await,
            };

            if evt.is_none() {
                got_timeout = true;
                break;
            }

            if !sender.send_next_part(&*connection) {
                break;
            }

            on_activity();
        }

        // Finish the transfer
        sender.finish(&*connection);

        if !got_timeout {
            on_activity();
        }

        events.dispose();
    })
}

/// Stops the nested dispatcher frame that waits for a future.
struct FrameWaker(Arc<DispatcherFrame>);

impl Wake for FrameWaker {
    fn wake(self: Arc<Self>) {
        self.0.set_continue(false);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.set_continue(false);
    }
}

/// The result of a future, for the request of another client that has to
/// be answered now (the reference blocks on the task). The data transfers
/// an application puts on the clipboard have their values at hand, so the
/// future is ready; one that is not is waited for in a nested frame of the
/// dispatcher, as the synchronous wrappers of the base library do.
pub(crate) fn get_result<T>(mut future: LocalBoxFuture<T>) -> T {
    if let Poll::Ready(value) = future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        return value;
    }

    let dispatcher = Dispatcher::ui_thread();
    loop {
        let frame = DispatcherFrame::new();
        let waker = Waker::from(Arc::new(FrameWaker(frame.clone())));
        if let Poll::Ready(value) = future.as_mut().poll(&mut Context::from_waker(&waker)) {
            return value;
        }
        dispatcher.push_frame(&frame);
    }
}

/// Whether a data transfer has a format. A data transfer that fails to
/// tell its formats has none.
fn contains(data_transfer: &dyn IAsyncDataTransfer, format: &DataFormat) -> bool {
    data_transfer.try_formats().is_ok_and(|formats| formats.iter().any(|candidate| candidate == format))
}

/// The value of the first item of a data transfer that has a format, as
/// the item stores it.
fn try_get_raw(
    data_transfer: &dyn IAsyncDataTransfer,
    format: &DataFormat,
) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
    for item in data_transfer.try_items()?.iter() {
        if item.try_contains(format)? {
            return get_result(item.try_get_raw_async(format));
        }
    }

    Ok(None)
}

/// The bytes of a value of an application or platform format: a string as
/// UTF-8, bytes as they are.
fn value_to_bytes(value: &dyn Any) -> Option<Vec<u8>> {
    if let Some(string_value) = value.downcast_ref::<String>() {
        return Some(string_value.as_bytes().to_vec());
    }

    if let Some(bytes) = value.downcast_ref::<Rc<[u8]>>() {
        return Some(bytes.to_vec());
    }

    value.downcast_ref::<Vec<u8>>().cloned()
}

/// Provides an X11 selection (clipboard/drag-and-drop).
///
/// The reference class is the abstract base of the providers; here it is
/// the part they hold: what the reference overrides (`OnActivity`) is a
/// handler the owner sets, and `Dispose` is the owner's.
pub struct SelectionDataProvider {
    selection: Atom,
    maximum_property_size: i32,

    platform: Weak<FerroX11Platform>,
    info: Rc<X11Info>,
    display: XDisplay,
    connection: Rc<dyn ISelectionConnection>,

    data_transfer: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
    on_activity: RefCell<Option<Rc<dyn Fn()>>>,
}

impl SelectionDataProvider {
    pub fn new(platform: &Rc<FerroX11Platform>, selection: Atom) -> Self {
        let display = platform.display();

        Self {
            selection,
            maximum_property_size: maximum_property_size(
                xlib::x_extended_max_request_size(display),
                xlib::x_max_request_size(display),
            ),
            platform: Rc::downgrade(platform),
            info: platform.info().clone(),
            display,
            connection: XlibSelectionConnection::new(display),
            data_transfer: RefCell::new(None),
            on_activity: RefCell::new(None),
        }
    }

    /// The platform, while it lives.
    pub fn platform(&self) -> Option<Rc<FerroX11Platform>> {
        self.platform.upgrade()
    }

    pub fn info(&self) -> &Rc<X11Info> {
        &self.info
    }

    pub fn display(&self) -> XDisplay {
        self.display
    }

    /// The calls of the protocol on the connection of the platform.
    pub fn connection(&self) -> &Rc<dyn ISelectionConnection> {
        &self.connection
    }

    /// The data the selection provides.
    pub fn data_transfer(&self) -> Option<Rc<dyn IAsyncDataTransfer>> {
        self.data_transfer.borrow().clone()
    }

    pub fn set_data_transfer(&self, value: Option<Rc<dyn IAsyncDataTransfer>>) {
        let previous = self.data_transfer.replace(value);
        drop(previous);
    }

    pub fn get_owner(&self) -> XID {
        xlib::x_get_selection_owner(self.display, self.selection)
    }

    pub fn set_owner(&self, owner: XID) {
        xlib::x_set_selection_owner(self.display, self.selection, owner, 0);
    }

    /// Answers the request of another client for the selection. `evt` is
    /// the `SelectionRequest` event.
    pub fn on_selection_request(&self, evt: &XEvent) {
        let request = *xlib::selection_request_event(evt);

        let mut response = xlib::new_event();
        {
            let selection_event = xlib::selection_event_mut(&mut response);
            selection_event.type_ = XEventName::SelectionNotify as i32;
            selection_event.send_event = 1;
            selection_event.display = self.display.as_ptr().cast();
            selection_event.selection = request.selection;
            selection_event.target = request.target;
            selection_event.requestor = request.requestor;
            selection_event.time = request.time;
            selection_event.property = 0;
        }

        if request.selection == self.selection {
            let property = self.write_target_to_property(request.target, request.requestor, request.property);
            xlib::selection_event_mut(&mut response).property = property;
        }

        xlib::x_send_event(
            self.display,
            request.requestor,
            false,
            EventMask::NO_EVENT_MASK.bits() as c_long,
            &mut response,
        );
        self.on_activity();
    }

    fn write_target_to_property(&self, target: Atom, window: XID, property: Atom) -> Atom {
        let atoms = self.info.atoms();

        if target == atoms.TARGETS {
            let atom_values = self.convert_data_transfer(self.data_transfer().as_deref());
            self.connection.change_property_longs(window, property, atoms.ATOM, &atom_values);
            return property;
        }

        if target == atoms.SAVE_TARGETS {
            return property;
        }

        if let Some(mut data_format) = data_format_helper::to_data_format(target, atoms) {
            let Some(data_transfer) = self.data_transfer() else {
                return 0;
            };

            // Our default bitmap format is image/png
            if data_format.identifier() == MIME_TYPE_PNG_FORMAT && contains(&*data_transfer, &DataFormat::bitmap()) {
                data_format = DataFormat::bitmap().into();
            }

            if !contains(&*data_transfer, &data_format) {
                return 0;
            }

            let Some(bytes) = self.try_get_data_as_bytes(&*data_transfer, &data_format, target) else {
                return 0;
            };

            self.send_data_to_client(window, property, target, bytes);
            return property;
        }

        if target == atoms.MULTIPLE {
            let prop =
                self.connection.get_window_property(window, property, 0, c_long::from(i32::MAX), false, atoms.ATOM_PAIR);

            if prop.nitems == 0 {
                return 0;
            }

            if prop.actual_format == 32 {
                let mut data = prop.longs();
                convert_multiple(&mut data, |sub_target, sub_prop| {
                    self.write_target_to_property(sub_target, window, sub_prop)
                });

                self.connection.change_property_longs(window, property, atoms.ATOM_PAIR, &data);
            }

            return property;
        }

        0
    }

    /// The bytes a target of the selection is answered with.
    ///
    /// The reference blocks on the tasks of the data transfer and lets
    /// their exceptions escape the handler of the event; here a data
    /// transfer that fails is logged and the request is refused.
    ///
    /// The reference decides between text and bytes for the application
    /// and platform formats by the type of the format, which a format of
    /// the port does not carry: the value the data transfer stores decides
    /// (a string is sent as UTF-8, bytes as they are).
    fn try_get_data_as_bytes(
        &self,
        data_transfer: &dyn IAsyncDataTransfer,
        format: &DataFormat,
        target_format_atom: Atom,
    ) -> Option<Vec<u8>> {
        match self.try_get_data_as_bytes_core(data_transfer, format, target_format_atom) {
            Ok(bytes) => bytes,
            Err(error) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::X11_PLATFORM) {
                    logger.log_with_values(
                        None,
                        "Unable to get the data of the format {Format}: {Error}",
                        &[format as &dyn Display, &error as &dyn Display],
                    );
                }
                None
            }
        }
    }

    fn try_get_data_as_bytes_core(
        &self,
        data_transfer: &dyn IAsyncDataTransfer,
        format: &DataFormat,
        target_format_atom: Atom,
    ) -> Result<Option<Vec<u8>>, ClipboardError> {
        if DataFormat::text() == *format {
            let text = get_result(data_transfer.try_get_value_async(&DataFormat::text()))?;

            return Ok(data_format_helper::try_get_string_encoding(target_format_atom, self.info.atoms())
                .map(|encoding| encoding.get_bytes(text.as_deref().unwrap_or_default())));
        }

        if DataFormat::bitmap() == *format {
            let Some(bitmap) = get_result(data_transfer.try_get_value_async(&DataFormat::bitmap()))? else {
                return Ok(None);
            };

            let mut stream = Vec::new();
            bitmap
                .save(&mut stream, &PngBitmapEncoderOptions::DEFAULT.into())
                .map_err(|error| ClipboardError::other(format!("Unable to save the bitmap: {error}")))?;

            return Ok(Some(stream));
        }

        if DataFormat::file() == *format {
            let Some(files) = get_result(data_transfer.try_get_values_async(&DataFormat::file()))? else {
                return Ok(None);
            };

            return Ok(Some(uri_list_helper::file_uri_list_to_utf8_bytes(&files)));
        }

        let Some(value) = try_get_raw(data_transfer, format)? else {
            return Ok(None);
        };

        if let Some(bytes) = value_to_bytes(&*value) {
            return Ok(Some(bytes));
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::X11_PLATFORM) {
            logger.log_with_values(None, "Unsupported data format {Format}", &[format as &dyn Display]);
        }

        Ok(None)
    }

    fn send_incr_data_to_client(&self, window: XID, property: Atom, target: Atom, data: Vec<u8>) {
        let Some(platform) = self.platform() else {
            return;
        };

        let events = EventStreamWindow::new(&platform, Some(window));
        let on_activity = self.on_activity.borrow().clone();
        let on_activity: Rc<dyn Fn()> = on_activity.unwrap_or_else(|| Rc::new(|| {}));

        let transfer = send_incr_data_async(
            IncrSender::new(window, property, target, data, self.maximum_property_size),
            self.connection.clone(),
            events,
            self.info.atoms().INCR,
            on_activity,
        );

        // The transfer goes on by itself, as the task the reference does
        // not await.
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || transfer));
    }

    fn send_data_to_client(&self, window: XID, property: Atom, target: Atom, bytes: Vec<u8>) {
        if (bytes.len() as i64) < i64::from(self.maximum_property_size) {
            self.connection.change_property_bytes(window, property, target, &bytes);
            self.on_activity();
            return;
        }

        self.send_incr_data_to_client(window, property, target, bytes);
    }

    /// The targets a data transfer is offered as.
    pub fn convert_data_transfer(&self, data_transfer: Option<&dyn IAsyncDataTransfer>) -> Vec<Atom> {
        let atoms = self.info.atoms();

        let formats = data_transfer.and_then(|data_transfer| data_transfer.try_formats().ok());
        let format_atoms = data_format_helper::formats_to_atoms(formats.as_deref().unwrap_or_default(), atoms);
        targets_answer(atoms.TARGETS, atoms.MULTIPLE, &format_atoms)
    }

    /// Sets what is called after something was sent to another client
    /// (the override of `OnActivity` of the reference).
    pub fn set_on_activity(&self, handler: Option<Rc<dyn Fn()>>) {
        let previous = self.on_activity.replace(handler);
        drop(previous);
    }

    fn on_activity(&self) {
        let handler = self.on_activity.borrow().clone();
        if let Some(handler) = handler {
            handler();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use crate::selections::selection_helper::test_support::{run_ready, Call, MockConnection, MockEventWaiter};
    use std::cell::Cell;

    const WINDOW: XID = 900;
    const PROPERTY: Atom = 301;
    const TARGET: Atom = 302;
    const INCR: Atom = 303;

    fn property_notify(atom: Atom, state: i32) -> Option<XEvent> {
        let mut event = xlib::new_event();
        let property_event = xlib::property_event_mut(&mut event);
        property_event.type_ = XEventName::PropertyNotify as i32;
        property_event.window = WINDOW;
        property_event.atom = atom;
        property_event.state = state;
        Some(event)
    }

    fn deletion() -> Option<XEvent> {
        property_notify(PROPERTY, PROPERTY_DELETE)
    }

    fn part(data: &[u8]) -> Call {
        Call::ChangePropertyBytes { window: WINDOW, property: PROPERTY, type_: TARGET, data: data.to_vec() }
    }

    fn counter() -> (Rc<Cell<usize>>, Rc<dyn Fn()>) {
        let count = Rc::new(Cell::new(0));
        let handler: Rc<dyn Fn()> = Rc::new({
            let count = count.clone();
            move || count.set(count.get() + 1)
        });
        (count, handler)
    }

    #[test]
    fn the_maximum_property_size_is_a_megabyte_at_most() {
        // With the big requests extension.
        assert_eq!(maximum_property_size(4_194_303, 65_535), 0x100000);
        // Without it: the size of a request less a margin.
        assert_eq!(maximum_property_size(0, 65_535), 65_535 - 0x100);
        assert_eq!(maximum_property_size(0, 4_096), 4_096 - 0x100);
        assert_eq!(maximum_property_size(0x100100, 65_535), 0x100000);
        assert_eq!(maximum_property_size(0x1000ff, 65_535), 0xfffff);
    }

    #[test]
    fn a_rented_buffer_has_the_length_of_the_next_power_of_two() {
        assert_eq!(rented_buffer_length(0), 0);
        assert_eq!(rented_buffer_length(-5), 0);
        assert_eq!(rented_buffer_length(1), 16);
        assert_eq!(rented_buffer_length(16), 16);
        assert_eq!(rented_buffer_length(17), 32);
        assert_eq!(rented_buffer_length(65_279), 65_536);
        assert_eq!(rented_buffer_length(0x100000), 0x100000);
    }

    #[test]
    fn the_targets_start_with_the_targets_of_the_protocol() {
        assert_eq!(targets_answer(1, 2, &[10, 11]), [1, 2, 10, 11]);
        assert_eq!(targets_answer(1, 2, &[]), [1, 2]);
    }

    #[test]
    fn a_multiple_request_gets_the_property_of_every_conversion_or_zero() {
        let mut pairs: Vec<c_ulong> = vec![10, 100, 11, 101, 12, 102];
        let mut asked = Vec::new();
        convert_multiple(&mut pairs, |target, property| {
            asked.push((target, property));
            // The second target is refused.
            if target == 11 {
                0
            } else {
                property
            }
        });
        assert_eq!(asked, [(10, 100), (11, 101), (12, 102)]);
        assert_eq!(pairs, [10, 100, 11, 0, 12, 102]);

        // A target without a property is not converted.
        let mut pairs: Vec<c_ulong> = vec![10, 100, 11];
        convert_multiple(&mut pairs, |_, _| 7);
        assert_eq!(pairs, [10, 7, 11]);
    }

    #[test]
    fn values_are_sent_as_their_bytes() {
        assert_eq!(value_to_bytes(&String::from("h\u{e9}")), Some("h\u{e9}".as_bytes().to_vec()));
        let bytes: Rc<[u8]> = Rc::from(vec![1u8, 2, 3]);
        assert_eq!(value_to_bytes(&bytes), Some(vec![1, 2, 3]));
        assert_eq!(value_to_bytes(&vec![4u8, 5]), Some(vec![4, 5]));
        assert_eq!(value_to_bytes(&42i32), None);
    }

    #[test]
    fn an_incremental_transfer_is_cut_into_parts_and_ended_by_an_empty_one() {
        let data: Vec<u8> = (0..40).collect();
        let connection = MockConnection::new([]);
        let mut sender = IncrSender::new(WINDOW, PROPERTY, TARGET, data.clone(), 16);

        sender.begin(&*connection, INCR);
        assert!(sender.send_next_part(&*connection));
        assert!(sender.send_next_part(&*connection));
        assert!(sender.send_next_part(&*connection));
        assert!(!sender.send_next_part(&*connection));
        assert!(!sender.send_next_part(&*connection));
        sender.finish(&*connection);

        assert_eq!(
            connection.calls(),
            [
                Call::SelectInput { window: WINDOW, mask: 0x400000 },
                Call::ChangePropertyLongs { window: WINDOW, property: PROPERTY, type_: INCR, data: vec![40] },
                part(&data[..16]),
                part(&data[16..32]),
                part(&data[32..]),
                part(&[]),
            ]
        );
    }

    #[test]
    fn the_parts_have_the_length_of_the_rented_buffer() {
        let data = vec![7u8; 100];
        let connection = MockConnection::new([]);
        // Twenty bytes are asked from the pool, which has thirty-two.
        let mut sender = IncrSender::new(WINDOW, PROPERTY, TARGET, data, 20);
        while sender.send_next_part(&*connection) {}
        let lengths: Vec<usize> = connection
            .calls()
            .iter()
            .map(|call| match call {
                Call::ChangePropertyBytes { data, .. } => data.len(),
                other => panic!("unexpected call {other:?}"),
            })
            .collect();
        assert_eq!(lengths, [32, 32, 32, 4]);
    }

    #[test]
    fn a_part_is_sent_for_every_deletion_of_the_property() {
        let data: Vec<u8> = (0..40).collect();
        let connection = MockConnection::new([]);
        // The requestor deletes the announcement and then every part.
        let waiter = MockEventWaiter::new([deletion(), deletion(), deletion(), deletion()]);
        let (activity, on_activity) = counter();

        let transfer = send_incr_data_async(
            IncrSender::new(WINDOW, PROPERTY, TARGET, data.clone(), 16),
            connection.clone(),
            waiter.clone(),
            INCR,
            on_activity,
        );

        // The announcement is made and the first wait has started before
        // the transfer is first polled.
        assert_eq!(connection.calls().len(), 2);
        assert_eq!(waiter.waits.get(), 1);
        assert!(!waiter.disposed.get());

        run_ready(transfer);

        assert_eq!(
            connection.calls()[2..],
            [part(&data[..16]), part(&data[16..32]), part(&data[32..]), part(&[])]
        );
        assert_eq!(waiter.waits.get(), 4);
        assert_eq!(waiter.rejected.get(), 0);
        // Once for every part and once for the end.
        assert_eq!(activity.get(), 4);
        assert!(waiter.disposed.get());
    }

    #[test]
    fn a_transfer_whose_requestor_stops_asking_is_ended() {
        let data: Vec<u8> = (0..40).collect();
        let connection = MockConnection::new([]);
        let waiter = MockEventWaiter::new([deletion(), None]);
        let (activity, on_activity) = counter();

        run_ready(send_incr_data_async(
            IncrSender::new(WINDOW, PROPERTY, TARGET, data.clone(), 16),
            connection.clone(),
            waiter.clone(),
            INCR,
            on_activity,
        ));

        assert_eq!(connection.calls()[2..], [part(&data[..16]), part(&[])]);
        // The end of a transfer that timed out is no activity.
        assert_eq!(activity.get(), 1);
        assert!(waiter.disposed.get());
    }

    #[test]
    fn only_the_deletion_of_the_property_of_the_transfer_asks_for_a_part() {
        for event in [
            // `PropertyNewValue`: the part that was just written.
            property_notify(PROPERTY, 0),
            property_notify(PROPERTY + 1, PROPERTY_DELETE),
        ] {
            let connection = MockConnection::new([]);
            let waiter = MockEventWaiter::new([event]);
            let (_, on_activity) = counter();
            run_ready(send_incr_data_async(
                IncrSender::new(WINDOW, PROPERTY, TARGET, vec![1u8; 40], 16),
                connection.clone(),
                waiter.clone(),
                INCR,
                on_activity,
            ));
            assert_eq!(waiter.rejected.get(), 1);
            assert_eq!(connection.calls()[2..], [part(&[])]);
        }
    }
}
