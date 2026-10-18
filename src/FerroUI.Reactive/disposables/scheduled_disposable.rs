//! `ScheduledDisposable`.

use super::{Disposable, ICancelable};
use crate::concurrency::{IScheduler, SchedulerExt};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A disposable whose disposal runs on a scheduler (`ScheduledDisposable`).
pub struct ScheduledDisposable {
    scheduler: Rc<dyn IScheduler>,
    disposable: Rc<RefCell<Option<Rc<dyn IDisposable>>>>,
    disposed: Cell<bool>,
}

impl ScheduledDisposable {
    /// Disposes `disposable` on `scheduler` when it is disposed.
    pub fn new(scheduler: Rc<dyn IScheduler>, disposable: Rc<dyn IDisposable>) -> Rc<Self> {
        Rc::new(Self { scheduler, disposable: Rc::new(RefCell::new(Some(disposable))), disposed: Cell::new(false) })
    }

    /// The scheduler the disposal runs on.
    pub fn scheduler(&self) -> &Rc<dyn IScheduler> {
        &self.scheduler
    }

    /// The disposable that is disposed (an empty one once it has been).
    pub fn disposable(&self) -> Rc<dyn IDisposable> {
        self.disposable.borrow().clone().unwrap_or_else(Disposable::empty)
    }
}

impl IDisposable for ScheduledDisposable {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let disposable = self.disposable.clone();
        self.scheduler.schedule_action(move || {
            let inner = disposable.borrow_mut().take();
            if let Some(inner) = inner {
                inner.dispose();
            }
        });
    }
}

impl ICancelable for ScheduledDisposable {
    fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}
