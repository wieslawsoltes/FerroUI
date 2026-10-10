//! The events of the connection that renders (the port of
//! `X11DeferredDisplayDispatcher.cs`).

use crate::xlib::{self, XDisplay, XEvent};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_ulong;
use std::rc::Rc;

/// The callbacks that wait for the completion of a shared memory segment,
/// apart from the connection: who waits for which segment.
pub(crate) struct ShmCompletionCallbacks<C> {
    callbacks: HashMap<c_ulong, C>,
}

impl<C> ShmCompletionCallbacks<C> {
    pub(crate) fn new() -> Self {
        Self { callbacks: HashMap::new() }
    }

    /// Registers the callback of a segment; a callback registered before
    /// for the same segment is replaced.
    pub(crate) fn register(&mut self, shm_seg: c_ulong, callback: C) {
        self.callbacks.insert(shm_seg, callback);
    }

    /// The callback of a segment, which no longer waits.
    pub(crate) fn take(&mut self, shm_seg: c_ulong) -> Option<C> {
        self.callbacks.remove(&shm_seg)
    }
}

thread_local! {
    /// The dispatchers of this thread, by connection.
    static DISPATCHERS: RefCell<Vec<(XDisplay, Rc<X11DeferredDisplayDispatcher>)>> = const { RefCell::new(Vec::new()) };
}

/// Drains events from the `DeferredDisplay` connection, which the rendering thread owns and pumps
/// itself (the main UI event loop never reads it). A single instance is shared by every window on the
/// platform. Currently it only routes MIT-SHM completion events, but the draining concept is not
/// SHM-specific.
///
/// All members are expected to be called from the rendering thread, which both submits work and drains the
/// resulting events off this connection, so completions can be awaited by blocking here regardless of which
/// thread renders.
///
/// The reference creates the instance on the platform, on the UI thread,
/// and hands it to every surface. Here it is an object of the thread that
/// renders ([`for_current_thread`](Self::for_current_thread)), which is
/// the one thread that uses it: its callbacks hold render targets, which
/// are objects of that thread.
pub struct X11DeferredDisplayDispatcher {
    deferred_display: XDisplay,
    x_shm_completion_type: i32,
    shm_completion_callbacks: RefCell<ShmCompletionCallbacks<Box<dyn FnOnce()>>>,
}

impl X11DeferredDisplayDispatcher {
    pub fn new(deferred_display: XDisplay) -> Self {
        // From https://www.x.org/releases/X11R7.5/doc/Xext/mit-shm.html
        // > int CompletionType = XShmGetEventBase (display) + ShmCompletion;
        const SHM_COMPLETION: i32 = 0;
        Self {
            deferred_display,
            x_shm_completion_type: xlib::x_shm_get_event_base(deferred_display) + SHM_COMPLETION,
            shm_completion_callbacks: RefCell::new(ShmCompletionCallbacks::new()),
        }
    }

    /// The dispatcher of the calling thread for a connection, created when
    /// first asked for (`FerroX11Platform.DeferredDisplayDispatcher` of
    /// the reference).
    pub fn for_current_thread(deferred_display: XDisplay) -> Rc<X11DeferredDisplayDispatcher> {
        DISPATCHERS.with(|dispatchers| {
            let mut dispatchers = dispatchers.borrow_mut();
            if let Some((_, dispatcher)) = dispatchers.iter().find(|(display, _)| *display == deferred_display) {
                return dispatcher.clone();
            }
            let dispatcher = Rc::new(X11DeferredDisplayDispatcher::new(deferred_display));
            dispatchers.push((deferred_display, dispatcher.clone()));
            dispatcher
        })
    }

    /// Registers a one-shot callback to be invoked when the server signals completion for the given shm
    /// segment. The callback is removed once fired.
    pub fn register_for_shm_completion(&self, shm_seg: c_ulong, callback: Box<dyn FnOnce()>) {
        self.shm_completion_callbacks.borrow_mut().register(shm_seg, callback);
    }

    /// Dispatches every event currently pending on the connection without blocking.
    pub fn drain_pending_events(&self) {
        while xlib::x_pending(self.deferred_display) != 0 {
            let ev = xlib::x_next_event(self.deferred_display);
            self.dispatch(&ev);
        }
    }

    /// Makes progress on the event queue, blocking for at most one event. If events are already pending
    /// they are all drained; otherwise this blocks until a single event arrives and then drains the rest.
    pub fn drain_events_blocking_at_most_once(&self) {
        if xlib::x_pending(self.deferred_display) != 0 {
            self.drain_pending_events();
        } else {
            let ev = xlib::x_next_event(self.deferred_display);
            self.dispatch(&ev);
            self.drain_pending_events();
        }
    }

    fn dispatch(&self, ev: &XEvent) {
        if xlib::event_type(ev) != self.x_shm_completion_type {
            // The DeferredDisplay connection only renders; any non-SHM event is unexpected but harmless.
            return;
        }

        let shm_seg = xlib::shm_completion_event_shmseg(ev);

        if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::X11_PLATFORM) {
            logger.log_with_values(None, "[X11DeferredDisplayDispatcher] XShmCompletion shmseg={ShmSeg}", &[&shm_seg]);
        }

        // The callback runs with the callbacks released: it may register again.
        let callback = self.shm_completion_callbacks.borrow_mut().take(shm_seg);
        if let Some(callback) = callback {
            callback();
        } else if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::X11_PLATFORM) {
            // Unexpected case, all submitted shm segments should be registered before submission.
            logger.log_with_values(
                None,
                "[X11DeferredDisplayDispatcher] No completion callback registered for shmseg={ShmSeg}",
                &[&shm_seg],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn a_completion_is_given_to_the_callback_of_its_segment_once() {
        let mut callbacks = ShmCompletionCallbacks::<u32>::new();
        callbacks.register(7, 70);
        callbacks.register(8, 80);
        assert_eq!(callbacks.take(8), Some(80));
        assert_eq!(callbacks.take(8), None);
        assert_eq!(callbacks.take(9), None);
        // A segment that is submitted again replaces what waited for it.
        callbacks.register(7, 71);
        assert_eq!(callbacks.take(7), Some(71));
        assert_eq!(callbacks.take(7), None);
    }
}
