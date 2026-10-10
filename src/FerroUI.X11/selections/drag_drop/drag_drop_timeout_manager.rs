//! The timeout of the steps of a drag that wait for the other client (the
//! port of `DragDropTimeoutManager.cs`).
//!
//! The reference uses a timer of the thread pool and completes the drag
//! from its thread. Here the timer is one of the UI dispatcher: the
//! timeout handler runs on the UI thread, like everything else of a drag.

use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

pub struct DragDropTimeoutManager {
    timer: Rc<DispatcherTimer>,
    disposed: Cell<bool>,
}

impl DragDropTimeoutManager {
    /// A timeout that is not running; `on_timeout` is called once per
    /// [`restart`](Self::restart) that is not followed by another restart
    /// or a [`stop`](Self::stop) within `timeout`.
    pub fn new(timeout: Duration, on_timeout: impl Fn() + 'static) -> Self {
        let timer = DispatcherTimer::with_callback(timeout, DispatcherPriority::DEFAULT, move |timer| {
            timer.stop();
            on_timeout();
        });
        timer.stop();
        Self { timer, disposed: Cell::new(false) }
    }

    pub fn stop(&self) {
        self.timer.stop();
    }

    pub fn restart(&self) {
        if self.disposed.get() {
            return;
        }
        self.timer.stop();
        self.timer.start();
    }

    pub fn dispose(&self) {
        self.disposed.set(true);
        self.timer.stop();
    }
}

impl Drop for DragDropTimeoutManager {
    fn drop(&mut self) {
        self.timer.stop();
    }
}
