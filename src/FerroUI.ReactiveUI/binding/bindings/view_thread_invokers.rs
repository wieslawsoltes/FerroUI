//! `ViewThreadInvokers`.

use crate::binding::interfaces::IViewThreadInvoker;
use ferroui_splat::AppLocator;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// The resolved invokers, or `None` while none have been resolved since the last refresh.
    static INVOKERS: RefCell<Option<Rc<[Rc<dyn IViewThreadInvoker>]>>> = const { RefCell::new(None) };
}

/// Finds the registered [`IViewThreadInvoker`] for the object a binding writes to
/// (`ViewThreadInvokers`).
///
/// The invokers are registered in `AppLocator` as `dyn IViewThreadInvoker`, read on first use and
/// kept until [`refresh`](Self::refresh).
// Deviation (DEVIATIONS.md, ReactiveUI): the cached set is per thread, as the locator of the port
// is.
pub struct ViewThreadInvokers;

impl ViewThreadInvokers {
    /// Reads the registered invokers again on the next use, for a host that registers one after
    /// its first binding (`Refresh`).
    pub fn refresh() {
        let previous = INVOKERS.with(|invokers| invokers.borrow_mut().take());
        drop(previous);
    }

    /// The first registered invoker that claims `target`, or `None` when `target` is `None` or
    /// nothing claims it (`ForTarget`).
    pub fn for_target(target: Option<&Rc<dyn Any>>) -> Option<Rc<dyn IViewThreadInvoker>> {
        let target = target?;
        Self::resolve().iter().find(|invoker| invoker.claims(target)).cloned()
    }

    /// The registered invokers, read once and kept (`Resolve`).
    fn resolve() -> Rc<[Rc<dyn IViewThreadInvoker>]> {
        if let Some(resolved) = INVOKERS.with(|invokers| invokers.borrow().clone()) {
            return resolved;
        }
        let resolved: Rc<[Rc<dyn IViewThreadInvoker>]> = Rc::from(AppLocator::get_services::<dyn IViewThreadInvoker>());
        INVOKERS.with(|invokers| invokers.borrow_mut().get_or_insert_with(|| resolved.clone()).clone())
    }
}
