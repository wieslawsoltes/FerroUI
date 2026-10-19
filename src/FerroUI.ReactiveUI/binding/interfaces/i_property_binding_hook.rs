//! `IPropertyBindingHook`.

use crate::binding::interfaces::{BindingDirection, ObservedChange};
use std::any::Any;
use std::rc::Rc;

/// The current values along a bound property path, as a hook reads them: one change per step,
/// with the object that owns the property, the name of the property and its value
/// (`IObservedChange<object, object>[]`).
pub type BindingHookChanges = Vec<ObservedChange<Rc<dyn Any>, Rc<dyn Any>>>;

/// Intercepts the bindings as they are created, to run an action or to cancel the binding
/// (`IPropertyBindingHook`).
///
/// A hook is registered in `AppLocator` as `dyn IPropertyBindingHook` and consulted through
/// [`BindingHooks`](crate::BindingHooks).
pub trait IPropertyBindingHook {
    /// Called as a binding is set up, before it is wired; returns `true` to let the binding
    /// proceed and `false` to cancel it (`ExecuteHook`).
    ///
    /// `source` is the view model (`None` when there is none), `target` the view (not the
    /// control); `get_current_view_model_properties` and `get_current_view_properties` read the
    /// current values along each path when called.
    fn execute_hook(
        &self,
        source: Option<&Rc<dyn Any>>,
        target: &Rc<dyn Any>,
        get_current_view_model_properties: &dyn Fn() -> BindingHookChanges,
        get_current_view_properties: &dyn Fn() -> BindingHookChanges,
        direction: BindingDirection,
    ) -> bool;
}
