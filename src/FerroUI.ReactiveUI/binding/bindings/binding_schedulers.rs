//! `BindingSchedulers`.

use super::ViewThreadInvokers;
use crate::binding::interfaces::IViewThreadInvoker;
use crate::binding::observables::ViewThreadObservable;
use ferroui_reactive::concurrency::{IScheduler, SynchronizationContextScheduler};
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::IObservable;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

ferroui_system::ferro_thread_local! {
    /// The scheduler a write from another thread is delivered on, or `None` for the dispatcher of
    /// the view (`MainThread`).
    static MAIN_THREAD: RefCell<Option<Rc<dyn IScheduler>>> = const { RefCell::new(None) };
}

/// Where a binding delivers its writes to the view (`BindingSchedulers`).
///
/// A write is delivered at once on the thread that owns the object it lands on, and otherwise
/// through the [`IViewThreadInvoker`] that claims the object, or the scheduler the host set
/// ([`set_main_thread`](Self::set_main_thread)).
// Deviation (DEVIATIONS.md, ReactiveUI): the main-thread scheduler is per thread, as the schedulers
// of `RxSchedulers` are.
pub struct BindingSchedulers;

impl BindingSchedulers {
    /// The scheduler a write that waits for the owning thread is delivered on, or `None` to use the
    /// view's own dispatcher (the getter of `MainThread`).
    pub fn main_thread() -> Option<Rc<dyn IScheduler>> {
        MAIN_THREAD.with(|main_thread| main_thread.borrow().clone())
    }

    /// Sets the scheduler a write that waits for the owning thread is delivered on, or `None` to
    /// use the view's own dispatcher (the setter of `MainThread`).
    pub fn set_main_thread(scheduler: Option<Rc<dyn IScheduler>>) {
        let previous = MAIN_THREAD.with(|main_thread| main_thread.replace(scheduler));
        drop(previous);
    }

    /// Delivers the writes that wait for the owning thread through `context`, or through the
    /// view's own dispatcher when `context` is `None` (`UseSynchronizationContext`).
    pub fn use_synchronization_context(context: Option<Arc<dyn SynchronizationContext>>) {
        Self::set_main_thread(context.map(|context| SynchronizationContextScheduler::new(context) as Rc<dyn IScheduler>));
    }

    /// `source` routed onto the thread that owns `target`, the object the write lands on; the
    /// source itself when `target` is `None` or no registered invoker claims it
    /// (`ObserveOnViewThread<T>(IObservable<T>, object?)`).
    pub fn observe_on_view_thread<T: 'static>(source: Rc<dyn IObservable<T>>, target: Option<Rc<dyn Any>>) -> Rc<dyn IObservable<T>> {
        let invoker = ViewThreadInvokers::for_target(target.as_ref());
        Self::route(source, target, invoker)
    }

    /// [`observe_on_view_thread`](Self::observe_on_view_thread), with `fallback`, the invoker of
    /// the platform of `target`, used when no registered invoker claims it; the source itself when
    /// `target` is `None` (`ObserveOnViewThread<T>(IObservable<T>, object?, IViewThreadInvoker)`).
    pub fn observe_on_view_thread_with_fallback<T: 'static>(
        source: Rc<dyn IObservable<T>>,
        target: Option<Rc<dyn Any>>,
        fallback: Rc<dyn IViewThreadInvoker>,
    ) -> Rc<dyn IObservable<T>> {
        if target.is_none() {
            return source;
        }
        let invoker = ViewThreadInvokers::for_target(target.as_ref()).unwrap_or(fallback);
        Self::route(source, target, Some(invoker))
    }

    /// `source` observed on `scheduler`, a newer value replacing one that has not been delivered
    /// (`ObserveOnSequencer<T>`).
    pub fn observe_on_sequencer<T: 'static>(source: Rc<dyn IObservable<T>>, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        Rc::new(ViewThreadObservable::on_sequencer(source, scheduler))
    }

    /// `source` wrapped in the stage that writes on the thread of `invoker`, or `source` itself
    /// when no invoker owns the object (`Route`).
    fn route<T: 'static>(source: Rc<dyn IObservable<T>>, target: Option<Rc<dyn Any>>, invoker: Option<Rc<dyn IViewThreadInvoker>>) -> Rc<dyn IObservable<T>> {
        match (target, invoker) {
            (Some(target), Some(invoker)) => Rc::new(ViewThreadObservable::new(source, target, invoker)),
            _ => source,
        }
    }
}
