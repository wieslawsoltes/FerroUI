//! `IReactiveBinding<TView, TValue>`.

use crate::binding::interfaces::BindingDirection;
use crate::IViewFor;
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// A binding between a view and a view model property (`IReactiveBinding<TView, TValue>`):
/// disposing it disconnects the binding.
///
/// The bindings of [`PropertyBindingMixins`](crate::PropertyBindingMixins) return it as
/// `Rc<dyn IReactiveBinding<V, T>>`, which coerces to `Rc<dyn IDisposable>`.
pub trait IReactiveBinding<V: IViewFor + ?Sized, T>: IDisposable {
    /// The text of the view model property that is bound, or `None` when the binding carries none
    /// (`ViewModelExpression`). The bindings of the port carry none, as upstream's generated ones.
    fn view_model_expression(&self) -> Option<&str>;

    /// The view that is bound (`View`).
    fn view(&self) -> &Rc<V>;

    /// The text of the view property that is bound, or `None` when the binding carries none
    /// (`ViewExpression`).
    fn view_expression(&self) -> Option<&str>;

    /// The values the binding moves (`Changed`): the values written to the view for a one-way
    /// binding, the [`BindingChange`](crate::BindingChange)s for a two-way one.
    fn changed(&self) -> Rc<dyn IObservable<T>>;

    /// The direction of the binding (`Direction`).
    fn direction(&self) -> BindingDirection;
}
