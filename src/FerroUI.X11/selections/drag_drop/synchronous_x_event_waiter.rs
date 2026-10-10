//! Waiting for an event of the connection without leaving the call (the
//! port of `SynchronousXEventWaiter.cs`).

use crate::selections::i_x_event_waiter::{IXEventWaiter, XEventPredicate};
use crate::x11_structs::XEventName;
use crate::xlib::{self, XDisplay, XEvent};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::reactive::IDisposable;
use std::time::{Duration, Instant};

/// An implementation of [`IXEventWaiter`] that waits for an event synchronously by using its own event loop.
/// Unprocessed events are put back onto the main queue after the wait.
pub struct SynchronousXEventWaiter {
    display: XDisplay,
}

/// Waits until the descriptor can be read; false when it could not within
/// `timeout` (or the wait failed).
fn poll_readable(fd: i32, timeout: Duration) -> bool {
    let mut pollfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    let milliseconds = timeout.as_millis().min(i32::MAX as u128) as i32;
    // SAFETY: `pollfd` is one valid entry, and the count says so.
    unsafe { libc::poll(&mut pollfd, 1, milliseconds) == 1 }
}

impl SynchronousXEventWaiter {
    pub fn new(display: XDisplay) -> Self {
        Self { display }
    }

    /// Takes events off the queue of the connection until `predicate`
    /// accepts one, which is returned; `None` when none came within
    /// `timeout`. The events that were taken and not accepted are put back
    /// in the order they had.
    pub fn wait_for_event(&self, predicate: &dyn Fn(&XEvent) -> bool, timeout: Duration) -> Option<XEvent> {
        let display = self.display;
        let starting_timestamp = Instant::now();
        let check_timeout = || starting_timestamp.elapsed() < timeout;

        // Puts the stashed events back when the wait ends, however it ends
        // (the `finally` of the reference).
        struct Stash {
            display: XDisplay,
            events: Vec<XEvent>,
        }
        impl Drop for Stash {
            fn drop(&mut self) {
                // `XPutBackEvent` pushes onto the head of the queue, so the
                // last event taken goes back first: the queue then has the
                // events in the order they arrived in. (The reference puts
                // them back first to last, which reverses them:
                // docs/porting/DEVIATIONS.md.)
                for evt in self.events.iter_mut().rev() {
                    let is_generic = xlib::event_type(evt) == XEventName::GenericEvent as i32;
                    let has_data = is_generic && !xlib::generic_event_cookie(evt).data.is_null();
                    xlib::x_put_back_event(self.display, evt);
                    if has_data {
                        xlib::x_free_event_data(self.display, evt);
                    }
                }
            }
        }
        let mut stashed_events = Stash { display, events: Vec::new() };
        let mut fd = 0;

        loop {
            if !check_timeout() {
                return None;
            }

            // First, wait until there's some event in the queue
            while xlib::x_pending(display) == 0 {
                if fd == 0 {
                    fd = xlib::x_connection_number(display);
                }

                if !check_timeout() {
                    return None;
                }

                let remaining_timeout = timeout.saturating_sub(starting_timestamp.elapsed());
                if !poll_readable(fd, remaining_timeout) {
                    return None;
                }
            }

            if !check_timeout() {
                return None;
            }

            // Second, check if there's an event we're interested in.
            // Use XNextEvent and stash the unrelated events, then put them back onto the queue.
            // We can't use XIfEvent because it could block indefinitely.
            // We can't use XCheckIfEvent either: by leaving the event in the queue, the next poll()
            // returns immediately, eating CPU, and we don't want to add arbitrary thread sleeps.
            let mut evt = xlib::x_next_event(display);

            if xlib::event_type(&evt) == XEventName::GenericEvent as i32 {
                xlib::x_get_event_data(display, &mut evt);
            }

            if predicate(&evt) {
                return Some(evt);
            }

            stashed_events.events.push(evt);
        }
    }
}

impl IXEventWaiter for SynchronousXEventWaiter {
    fn wait_for_event_async(&self, predicate: XEventPredicate, timeout: Duration) -> LocalBoxFuture<Option<XEvent>> {
        Box::pin(std::future::ready(self.wait_for_event(&*predicate, timeout)))
    }
}

impl IDisposable for SynchronousXEventWaiter {
    fn dispose(&self) {}
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. The wait
    // itself needs a server: the smoke mode of the example reads a drop
    // through it.
    use super::*;
    use std::os::unix::net::UnixStream;

    #[test]
    fn a_descriptor_is_readable_when_something_was_written_and_not_before() {
        use std::io::Write;
        use std::os::fd::AsRawFd;

        let (reader, mut writer) = UnixStream::pair().unwrap();
        let started = Instant::now();
        assert!(!poll_readable(reader.as_raw_fd(), Duration::from_millis(30)));
        assert!(started.elapsed() >= Duration::from_millis(25));

        writer.write_all(b"x").unwrap();
        assert!(poll_readable(reader.as_raw_fd(), Duration::from_secs(5)));
    }
}
