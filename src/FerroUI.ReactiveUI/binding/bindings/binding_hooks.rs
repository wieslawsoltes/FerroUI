//! `BindingHooks`.

use crate::binding::interfaces::{BindingDirection, BindingHookChanges, IPropertyBindingHook};
use ferroui_splat::AppLocator;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// The resolved hooks, or `None` while none have been resolved since the last refresh.
    static HOOKS: RefCell<Option<Rc<[Rc<dyn IPropertyBindingHook>]>>> = const { RefCell::new(None) };
}

/// Lets a registered [`IPropertyBindingHook`] inspect, or refuse, a binding as it is created
/// (`BindingHooks`).
///
/// A hook is consulted once per binding, before anything is wired, and a hook that returns `false`
/// cancels the binding. Hooks are registered in `AppLocator`, so the set is only known at run time:
/// it is read on first use and kept until [`refresh`](Self::refresh). [`any`](Self::any) is the
/// guard a binding tests first, so that it builds the readers [`should_bind`](Self::should_bind)
/// takes only when a hook exists.
// Deviation (DEVIATIONS.md, ReactiveUI): the cached set is per thread, as the locator of the port
// is.
pub struct BindingHooks;

impl BindingHooks {
    /// Whether any hook was registered when the hooks were last read (`Any`).
    pub fn any() -> bool {
        !Self::resolve().is_empty()
    }

    /// Discards the cached hooks, so that the next use reads them from the locator again
    /// (`Refresh`).
    pub fn refresh() {
        let previous = HOOKS.with(|hooks| hooks.borrow_mut().take());
        drop(previous);
    }

    /// Asks the registered hooks in turn whether this binding may be created, stopping at the
    /// first refusal (`ShouldBind`): `true` when the binding may proceed, including when no hook
    /// is registered.
    ///
    /// `source` is the view model (`None` when there is none), `target` the view;
    /// `get_source_properties` and `get_target_properties` read the current values along each path.
    pub fn should_bind(
        source: Option<&Rc<dyn Any>>,
        target: &Rc<dyn Any>,
        get_source_properties: &dyn Fn() -> BindingHookChanges,
        get_target_properties: &dyn Fn() -> BindingHookChanges,
        direction: BindingDirection,
    ) -> bool {
        Self::resolve()
            .iter()
            .all(|hook| hook.execute_hook(source, target, get_source_properties, get_target_properties, direction))
    }

    /// The registered hooks, read once and kept (`Resolve`).
    fn resolve() -> Rc<[Rc<dyn IPropertyBindingHook>]> {
        if let Some(resolved) = HOOKS.with(|hooks| hooks.borrow().clone()) {
            return resolved;
        }
        // The locator is read outside the borrow: a factory may itself bind.
        let resolved: Rc<[Rc<dyn IPropertyBindingHook>]> = Rc::from(AppLocator::get_services::<dyn IPropertyBindingHook>());
        HOOKS.with(|hooks| hooks.borrow_mut().get_or_insert_with(|| resolved.clone()).clone())
    }
}
