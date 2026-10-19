//! `ContextDisposable` (`Disposables/ContextDisposable.cs`).
//!
//! Deviation (DEVIATIONS.md, Reactive): the disposable it holds is not `Send`, so its disposal is
//! posted with [`SynchronizationContext::post_local`]: the disposable is disposed on the thread of
//! the context, from that thread, where upstream may dispose it from any thread.

use super::ICancelable;
use crate::concurrency::post_with_start_complete;
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// A disposable whose disposal is posted to a [`SynchronizationContext`] (`ContextDisposable`).
pub struct ContextDisposable {
    context: Arc<dyn SynchronizationContext>,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl ContextDisposable {
    /// Disposes `disposable` on `context` when it is disposed
    /// (`ContextDisposable(SynchronizationContext, IDisposable)`).
    pub fn new(context: Arc<dyn SynchronizationContext>, disposable: Rc<dyn IDisposable>) -> Rc<Self> {
        Rc::new(Self { context, disposable: RefCell::new(Some(disposable)), disposed: Cell::new(false) })
    }

    /// The context the disposal is posted to (`Context`).
    pub fn context(&self) -> &Arc<dyn SynchronizationContext> {
        &self.context
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
            post_with_start_complete(&self.context, move || old.dispose());
        }
    }
}

impl ICancelable for ContextDisposable {
    fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}
