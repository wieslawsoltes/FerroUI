//! `ContextDisposable` (`Disposables/ContextDisposable.cs`).
//!
//! Deviation (DEVIATIONS.md, Reactive): the disposable it holds is not `Send`, so its disposal is
//! posted with `post_local` ([`Schedule::schedule_local`]): the disposable is disposed on the thread
//! of the context, from that thread, where upstream may dispose it from any thread.
//!
//! The context is a schedule of `ferroui-system` ([`Schedule`]); the forms that take a
//! `SynchronizationContext` are deprecated since 0.5.0 (step A5 of phase C4 of #423).
#![allow(deprecated)]

use super::ICancelable;
use crate::concurrency::{post_with_start_complete, PostTarget};
use ferroui_system::task::Schedule;
use ferroui_system::threading::SynchronizationContext;
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

    /// Disposes `disposable` on `context` when it is disposed
    /// (`ContextDisposable(SynchronizationContext, IDisposable)`).
    #[deprecated(since = "0.5.0", note = "use `ContextDisposable::with_schedule`")]
    pub fn new(context: Arc<dyn SynchronizationContext>, disposable: Rc<dyn IDisposable>) -> Rc<Self> {
        Self::with_target(PostTarget::Context(context), disposable)
    }

    pub(crate) fn with_target(target: PostTarget, disposable: Rc<dyn IDisposable>) -> Rc<Self> {
        Rc::new(Self { target, disposable: RefCell::new(Some(disposable)), disposed: Cell::new(false) })
    }

    /// The schedule the disposal is posted to (`Context`), `None` for a deprecated context.
    pub fn schedule(&self) -> Option<&Arc<dyn Schedule>> {
        match &self.target {
            PostTarget::Schedule(schedule) => Some(schedule),
            PostTarget::Context(_) => None,
        }
    }

    /// The context the disposal is posted to (`Context`).
    ///
    /// # Panics
    /// When the disposable was made with a schedule ([`with_schedule`](Self::with_schedule)).
    #[deprecated(since = "0.5.0", note = "use `ContextDisposable::schedule`")]
    pub fn context(&self) -> &Arc<dyn SynchronizationContext> {
        match &self.target {
            PostTarget::Context(context) => context,
            PostTarget::Schedule(_) => panic!("the disposable posts to a schedule, not to a SynchronizationContext"),
        }
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
