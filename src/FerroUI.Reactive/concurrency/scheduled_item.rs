//! `ScheduledItem<TAbsolute>` (`ScheduledItem.cs`): a unit of work with a due time, which can be
//! cancelled before it runs.

use super::{IScheduler, ScheduledAction};
use crate::disposables::{ICancelable, SingleAssignmentDisposableValue};
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// Work scheduled for a due time (`ScheduledItem<TAbsolute>`). Disposing the item cancels it if it
/// has not run, and disposes what it started if it has.
pub struct ScheduledItem<T> {
    due_time: T,
    action: RefCell<Option<ScheduledAction>>,
    disposable: SingleAssignmentDisposableValue,
}

impl<T: Copy> ScheduledItem<T> {
    pub fn new(due_time: T, action: ScheduledAction) -> Rc<Self> {
        Rc::new(Self { due_time, action: RefCell::new(Some(action)), disposable: SingleAssignmentDisposableValue::new() })
    }

    /// The time the work is due.
    pub fn due_time(&self) -> T {
        self.due_time
    }

    /// Whether the work was cancelled.
    pub fn is_canceled(&self) -> bool {
        self.disposable.is_disposed()
    }

    /// Runs the work on `scheduler` unless it was cancelled or has run.
    pub fn invoke(&self, scheduler: &Rc<dyn IScheduler>) {
        if self.disposable.is_disposed() {
            return;
        }
        let action = self.action.borrow_mut().take();
        if let Some(action) = action {
            let started = action(scheduler);
            self.disposable.set_disposable(Some(started));
        }
    }

    /// Cancels the work (`Cancel`).
    pub fn cancel(&self) {
        self.dispose();
    }
}

impl<T> IDisposable for ScheduledItem<T> {
    fn dispose(&self) {
        self.action.borrow_mut().take();
        self.disposable.dispose();
    }
}

impl<T> ICancelable for ScheduledItem<T> {
    fn is_disposed(&self) -> bool {
        self.disposable.is_disposed()
    }
}
