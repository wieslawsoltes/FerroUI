//! `ContextDisposable` (`Disposables/ContextDisposable.cs`).
//!
//! Deviation (DEVIATIONS.md, Reactive): the disposable it holds is not `Send`, so its disposal is
//! posted with `post_local` ([`Schedule::schedule_local`]): the disposable is disposed on the thread
//! of the context, from that thread, where upstream may dispose it from any thread.
//!
//! The context is a schedule of `ferroui-system` ([`Schedule`]).

use super::ICancelable;
use crate::concurrency::{post_with_start_complete, PostTarget};
use ferroui_system::task::Schedule;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// A disposable whose disposal is posted to a context, a [`Schedule`] (`ContextDisposable`).
pub struct ContextDisposable {
    target: PostTarget,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl ContextDisposable {
    /// Disposes `disposable` on `schedule` when it is disposed
    /// (`ContextDisposable(SynchronizationContext, IDisposable)`).
    pub fn with_schedule(schedule: Arc<dyn Schedule>, disposable: Rc<dyn IDisposable>) -> Rc<Self> {
        Self::with_target(PostTarget::Schedule(schedule), disposable)
    }

    pub(crate) fn with_target(target: PostTarget, disposable: Rc<dyn IDisposable>) -> Rc<Self> {
        Rc::new(Self { target, disposable: RefCell::new(Some(disposable)), disposed: Cell::new(false) })
    }

    /// The schedule the disposal is posted to (`Context`).
    pub fn schedule(&self) -> &Arc<dyn Schedule> {
        let PostTarget::Schedule(schedule) = &self.target;
        schedule
    }
}

impl IDisposable for ContextDisposable {
    /// Posts the disposal of the disposable to the context, the first time.
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let old = self.disposable.borrow_mut().take();
        if let Some(old) = old {
            post_with_start_complete(&self.target, move || old.dispose());
        }
    }
}

impl ICancelable for ContextDisposable {
    fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}
