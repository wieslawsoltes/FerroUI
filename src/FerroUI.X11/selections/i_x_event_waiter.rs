//! Waiting for an event of the connection (the port of `IXEventWaiter.cs`).

use crate::xlib::XEvent;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::reactive::IDisposable;
use std::time::Duration;

/// The condition an awaited event has to meet.
pub type XEventPredicate = Box<dyn Fn(&XEvent) -> bool>;

/// Something that sees the events of a window and lets a transfer wait for
/// one of them.
pub trait IXEventWaiter: IDisposable {
    /// Waits for the next event `predicate` accepts. The future resolves to
    /// the event, or to `None` when none came within `timeout` or the
    /// waiter was disposed.
    ///
    /// The waiter listens from the moment this is called, not from the
    /// moment the future is first polled (a task of the reference runs up
    /// to its first incomplete await when it is created): an event that
    /// arrives between the call and the first poll is not lost.
    fn wait_for_event_async(&self, predicate: XEventPredicate, timeout: Duration) -> LocalBoxFuture<Option<XEvent>>;
}
