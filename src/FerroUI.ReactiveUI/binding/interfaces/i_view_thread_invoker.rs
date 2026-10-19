//! `IViewThreadInvoker`.

use std::any::Any;
use std::rc::Rc;

/// Runs the write of a binding on the thread that owns the object it lands on, through the
/// dispatcher of its platform (`IViewThreadInvoker`).
///
/// An invoker is registered in `AppLocator` as `dyn IViewThreadInvoker` and found through
/// [`ViewThreadInvokers`](crate::ViewThreadInvokers). The objects are given as `Rc<dyn Any>`, so
/// that an invoker can downcast them to the types of its platform, or compare them by identity.
pub trait IViewThreadInvoker {
    /// Whether this invoker handles `target`, an object of its platform (`Claims`).
    fn claims(&self, target: &Rc<dyn Any>) -> bool;

    /// Whether the calling thread may write to `target`, an object this invoker claims, directly:
    /// on the owning thread, or when the object has no owning thread (`CheckAccess`).
    fn check_access(&self, target: &Rc<dyn Any>) -> bool;

    /// Queues `callback` on the thread that owns `target`, an object this invoker claims
    /// (`Post`).
    // Deviation (DEVIATIONS.md, ReactiveUI): the callback captures its state, where upstream
    // passes a state object to an `Action<object?>`.
    fn post(&self, target: &Rc<dyn Any>, callback: Box<dyn FnOnce()>);
}
