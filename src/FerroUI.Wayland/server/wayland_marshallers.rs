//! Marshallers used by the cross-thread proxies (the port of
//! `WaylandMarshallers.cs`).
//!
//! The reference generates its proxies with a source generator
//! (`[GenerateCrossThreadProxy]`): a proxy implements the interface of its
//! target and posts every call through a marshaller. The proxies of the port
//! are written by hand beside the interfaces. Those that go from the worker
//! to the UI thread hold their target as a [`UiThreadRef`] and post with the
//! marshaller of this file; those that go from the UI thread to the worker
//! hold the number of the worker's object and post with the marshaller of
//! the worker client.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::ThreadBound;
use std::rc::Rc;
use std::sync::Arc;

/// The marshaller to the UI thread: `Dispatcher.UIThread.Post`.
///
/// # Panics
/// The job panics on a thread other than the UI thread, which cannot happen: the dispatcher
/// runs it there.
pub fn ui_thread(action: impl FnOnce() + Send + 'static, priority: DispatcherPriority) {
    Dispatcher::ui_thread().post(action, priority);
}

/// An object of the UI thread as a handle any thread may hold: the worker
/// keeps it and posts calls to it.
///
/// The object is reached only on its thread ([`get`](Self::get)). The last
/// handle may be dropped on the worker; the object is then dropped by a job
/// of the dispatcher of its thread, because its destructor may only run
/// there.
pub struct UiThreadRef<T: ?Sized + 'static> {
    inner: Arc<Inner<T>>,
}

struct Inner<T: ?Sized + 'static> {
    target: Option<ThreadBound<Rc<T>>>,
    dispatcher: Arc<Dispatcher>,
}

impl<T: ?Sized + 'static> UiThreadRef<T> {
    /// Binds `target` to the calling thread, whose dispatcher `dispatcher` is.
    pub fn new(target: Rc<T>, dispatcher: Arc<Dispatcher>) -> Self {
        Self { inner: Arc::new(Inner { target: Some(ThreadBound::new(target)), dispatcher }) }
    }

    /// The object.
    ///
    /// # Panics
    /// On a thread other than the one the object belongs to.
    pub fn get(&self) -> &Rc<T> {
        self.inner.target.as_ref().expect("the target is there until the handle is dropped").get()
    }

    /// Whether both handles are of the same object.
    pub fn is_same(&self, other: &UiThreadRef<T>) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Posts a call of the object to its thread, at the default priority (the default of the
    /// generated proxies of the reference).
    pub fn post(&self, call: impl FnOnce(&T) + Send + 'static) {
        self.post_with_priority(call, DispatcherPriority::DEFAULT);
    }

    /// Posts a call of the object to its thread.
    pub fn post_with_priority(&self, call: impl FnOnce(&T) + Send + 'static, priority: DispatcherPriority) {
        let this = self.clone();
        self.inner.dispatcher.post(move || call(this.get()), priority);
    }
}

impl<T: ?Sized + 'static> Clone for UiThreadRef<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: ?Sized + 'static> Drop for Inner<T> {
    fn drop(&mut self) {
        let Some(target) = self.target.take() else {
            return;
        };
        if target.is_on_thread() {
            drop(target);
        } else {
            // The value is dropped where it belongs. `ThreadBound` is what makes the job
            // `Send`; the job runs on the thread of the value.
            self.dispatcher.post(move || drop(target), DispatcherPriority::BACKGROUND);
        }
    }
}
