//! What the selection transfers share (the port of `SelectionHelper.cs`).
//!
//! The reference file only holds the timeout. The port adds what the
//! transfers of the reference get from elsewhere:
//!
//! - [`ISelectionConnection`]: the calls of the protocol a transfer makes
//!   between two waits, so that a transfer can run against a connection
//!   that is not a server (the tests);
//! - [`TaskCompletionSource`]: the completion a waiting transfer awaits
//!   (the class of that name of the reference's base library);
//! - [`create_event_window`]: `XLib.CreateEventWindow` of the reference,
//!   which needs the platform and so does not live with the Xlib calls.

use crate::dispatching::x11_event_dispatcher::EventHandler;
use crate::x11_platform::FerroX11Platform;
use crate::x11_window_info::X11WindowInfo;
use crate::xlib::{self, Atom, PropertyMode, WindowProperty, XDisplay, XID};
use std::cell::RefCell;
use std::ffi::{c_long, c_ulong};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// How long a transfer waits for the other client.
pub const TIMEOUT: Duration = Duration::from_secs(5);

/// The calls of the protocol the selection transfers make on a connection.
pub trait ISelectionConnection {
    /// `XConvertSelection`.
    fn convert_selection(&self, selection: Atom, target: Atom, property: Atom, requestor: XID, time: xlib::Time);

    /// `XGetWindowProperty`, with the offset and the length in units of
    /// four bytes.
    fn get_window_property(
        &self,
        window: XID,
        property: Atom,
        long_offset: c_long,
        long_length: c_long,
        delete: bool,
        req_type: Atom,
    ) -> WindowProperty;

    /// `XChangeProperty` replacing the property with data of format 8.
    fn change_property_bytes(&self, window: XID, property: Atom, type_: Atom, data: &[u8]);

    /// `XChangeProperty` replacing the property with data of format 32.
    fn change_property_longs(&self, window: XID, property: Atom, type_: Atom, data: &[c_ulong]);

    /// `XSelectInput`.
    fn select_input(&self, window: XID, mask: c_long);

    /// `XFlush`.
    fn flush(&self);
}

/// The connection to the server.
pub struct XlibSelectionConnection {
    display: XDisplay,
}

impl XlibSelectionConnection {
    pub fn new(display: XDisplay) -> Rc<Self> {
        Rc::new(Self { display })
    }
}

impl ISelectionConnection for XlibSelectionConnection {
    fn convert_selection(&self, selection: Atom, target: Atom, property: Atom, requestor: XID, time: xlib::Time) {
        xlib::x_convert_selection(self.display, selection, target, property, requestor, time);
    }

    fn get_window_property(
        &self,
        window: XID,
        property: Atom,
        long_offset: c_long,
        long_length: c_long,
        delete: bool,
        req_type: Atom,
    ) -> WindowProperty {
        xlib::x_get_window_property(self.display, window, property, long_offset, long_length, delete, req_type)
    }

    fn change_property_bytes(&self, window: XID, property: Atom, type_: Atom, data: &[u8]) {
        xlib::x_change_property_bytes(self.display, window, property, type_, PropertyMode::Replace, data);
    }

    fn change_property_longs(&self, window: XID, property: Atom, type_: Atom, data: &[c_ulong]) {
        xlib::x_change_property_longs(self.display, window, property, type_, PropertyMode::Replace, data);
    }

    fn select_input(&self, window: XID, mask: c_long) {
        xlib::x_select_input(self.display, window, mask);
    }

    fn flush(&self) {
        xlib::x_flush(self.display);
    }
}

/// Creates a window that is never mapped and only receives events, which
/// go to `handler` (`XLib.CreateEventWindow`).
pub fn create_event_window(platform: &FerroX11Platform, handler: EventHandler) -> XID {
    let win =
        xlib::x_create_simple_window(platform.display(), platform.info().default_root_window(), 0, 0, 1, 1, 0, 0, 0);
    platform.set_window(win, X11WindowInfo::new(handler, None));
    win
}

struct CompletionState<T> {
    result: Option<T>,
    wakers: Vec<Waker>,
}

/// A result that is set once and that any number of futures wait for.
pub struct TaskCompletionSource<T> {
    state: RefCell<CompletionState<T>>,
}

impl<T: Clone> TaskCompletionSource<T> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { state: RefCell::new(CompletionState { result: None, wakers: Vec::new() }) })
    }

    /// Whether the result was set.
    pub fn is_completed(&self) -> bool {
        self.state.borrow().result.is_some()
    }

    /// Sets the result and wakes whoever waits for it. Returns `false`
    /// when the result was already set.
    pub fn try_set_result(&self, result: T) -> bool {
        let wakers = {
            let mut state = self.state.borrow_mut();
            if state.result.is_some() {
                return false;
            }
            state.result = Some(result);
            std::mem::take(&mut state.wakers)
        };
        // The wakers run with the state released: a waker may poll.
        for waker in wakers {
            waker.wake();
        }
        true
    }

    /// The future that resolves to the result.
    pub fn task(self: &Rc<Self>) -> TaskCompletionFuture<T> {
        TaskCompletionFuture { source: self.clone() }
    }
}

/// The future of a [`TaskCompletionSource`].
pub struct TaskCompletionFuture<T> {
    source: Rc<TaskCompletionSource<T>>,
}

impl<T: Clone> Future for TaskCompletionFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.source.state.borrow_mut();
        if let Some(result) = &state.result {
            return Poll::Ready(result.clone());
        }
        if !state.wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
            state.wakers.push(cx.waker().clone());
        }
        Poll::Pending
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    // Not from the reference: what the tests of the selection transfers
    // share.
    use super::*;
    use crate::selections::i_x_event_waiter::{IXEventWaiter, XEventPredicate};
    use crate::xlib::XEvent;
    use ferroui_base::input::LocalBoxFuture;
    use ferroui_base::reactive::IDisposable;
    use std::cell::Cell;
    use std::collections::VecDeque;

    /// A call a transfer made on the connection.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) enum Call {
        ConvertSelection { selection: Atom, target: Atom, property: Atom, requestor: XID, time: xlib::Time },
        GetWindowProperty { window: XID, property: Atom, delete: bool, req_type: Atom },
        ChangePropertyBytes { window: XID, property: Atom, type_: Atom, data: Vec<u8> },
        ChangePropertyLongs { window: XID, property: Atom, type_: Atom, data: Vec<c_ulong> },
        SelectInput { window: XID, mask: c_long },
        Flush,
    }

    /// A connection that records the calls and answers the property reads
    /// from a list.
    #[derive(Default)]
    pub(crate) struct MockConnection {
        pub(crate) calls: RefCell<Vec<Call>>,
        pub(crate) properties: RefCell<VecDeque<WindowProperty>>,
    }

    impl MockConnection {
        pub(crate) fn new(properties: impl IntoIterator<Item = WindowProperty>) -> Rc<Self> {
            Rc::new(Self { calls: RefCell::new(Vec::new()), properties: RefCell::new(properties.into_iter().collect()) })
        }

        pub(crate) fn calls(&self) -> Vec<Call> {
            self.calls.borrow().clone()
        }
    }

    impl ISelectionConnection for MockConnection {
        fn convert_selection(&self, selection: Atom, target: Atom, property: Atom, requestor: XID, time: xlib::Time) {
            self.calls.borrow_mut().push(Call::ConvertSelection { selection, target, property, requestor, time });
        }

        fn get_window_property(
            &self,
            window: XID,
            property: Atom,
            _long_offset: c_long,
            _long_length: c_long,
            delete: bool,
            req_type: Atom,
        ) -> WindowProperty {
            self.calls.borrow_mut().push(Call::GetWindowProperty { window, property, delete, req_type });
            self.properties.borrow_mut().pop_front().unwrap_or_default()
        }

        fn change_property_bytes(&self, window: XID, property: Atom, type_: Atom, data: &[u8]) {
            self.calls.borrow_mut().push(Call::ChangePropertyBytes { window, property, type_, data: data.to_vec() });
        }

        fn change_property_longs(&self, window: XID, property: Atom, type_: Atom, data: &[c_ulong]) {
            self.calls.borrow_mut().push(Call::ChangePropertyLongs { window, property, type_, data: data.to_vec() });
        }

        fn select_input(&self, window: XID, mask: c_long) {
            self.calls.borrow_mut().push(Call::SelectInput { window, mask });
        }

        fn flush(&self) {
            self.calls.borrow_mut().push(Call::Flush);
        }
    }

    /// A property of format 8.
    pub(crate) fn bytes_property(actual_type: Atom, data: &[u8]) -> WindowProperty {
        WindowProperty {
            status: 0,
            actual_type,
            actual_format: 8,
            nitems: data.len() as c_ulong,
            bytes_after: 0,
            data: data.to_vec(),
        }
    }

    /// A property of format 32, as Xlib returns it (an array of C `long`).
    pub(crate) fn longs_property(actual_type: Atom, items: &[c_ulong]) -> WindowProperty {
        WindowProperty {
            status: 0,
            actual_type,
            actual_format: 32,
            nitems: items.len() as c_ulong,
            bytes_after: 0,
            data: items.iter().flat_map(|item| item.to_ne_bytes()).collect(),
        }
    }

    /// A waiter whose events are given beforehand: each wait takes the
    /// next one, which is delivered when the predicate accepts it; `None`
    /// (and the end of the list) is a wait that times out.
    pub(crate) struct MockEventWaiter {
        pub(crate) events: RefCell<VecDeque<Option<XEvent>>>,
        pub(crate) waits: Cell<usize>,
        pub(crate) rejected: Cell<usize>,
        pub(crate) disposed: Cell<bool>,
    }

    impl MockEventWaiter {
        pub(crate) fn new(events: impl IntoIterator<Item = Option<XEvent>>) -> Rc<Self> {
            Rc::new(Self {
                events: RefCell::new(events.into_iter().collect()),
                waits: Cell::new(0),
                rejected: Cell::new(0),
                disposed: Cell::new(false),
            })
        }
    }

    impl IDisposable for MockEventWaiter {
        fn dispose(&self) {
            self.disposed.set(true);
        }
    }

    impl IXEventWaiter for MockEventWaiter {
        fn wait_for_event_async(
            &self,
            predicate: XEventPredicate,
            _timeout: Duration,
        ) -> LocalBoxFuture<Option<XEvent>> {
            self.waits.set(self.waits.get() + 1);
            let event = self.events.borrow_mut().pop_front().flatten();
            let event = match event {
                Some(event) if predicate(&event) => Some(event),
                Some(_) => {
                    self.rejected.set(self.rejected.get() + 1);
                    None
                }
                None => None,
            };
            Box::pin(std::future::ready(event))
        }
    }

    /// Runs a future whose waits are all answered already.
    pub(crate) fn run_ready<T>(future: impl Future<Output = T>) -> T {
        let mut future = Box::pin(future);
        match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the future waits for something the test did not provide"),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::Wake;

    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn the_timeout_is_five_seconds() {
        assert_eq!(TIMEOUT, Duration::from_secs(5));
    }

    #[test]
    fn a_completion_wakes_its_waiters_once_and_keeps_its_result() {
        let source = TaskCompletionSource::<u32>::new();
        let counter = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let waker = Waker::from(counter.clone());
        let mut context = Context::from_waker(&waker);

        let mut first = Box::pin(source.task());
        let mut second = Box::pin(source.task());
        assert!(first.as_mut().poll(&mut context).is_pending());
        assert!(first.as_mut().poll(&mut context).is_pending());
        assert!(second.as_mut().poll(&mut context).is_pending());
        assert!(!source.is_completed());

        assert!(source.try_set_result(7));
        assert!(source.is_completed());
        // Both futures registered the same waker: it is woken once.
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        assert!(!source.try_set_result(8));

        assert_eq!(first.as_mut().poll(&mut context), Poll::Ready(7));
        assert_eq!(second.as_mut().poll(&mut context), Poll::Ready(7));
        assert_eq!(test_support::run_ready(source.task()), 7);
    }
}
