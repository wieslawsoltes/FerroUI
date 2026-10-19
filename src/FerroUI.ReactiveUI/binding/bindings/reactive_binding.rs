//! `ReactiveBinding<TView, TValue>`.

use crate::binding::interfaces::{BindingDirection, IReactiveBinding};
use crate::IViewFor;
use ferroui_system::{IDisposable, IObservable};
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

/// The binding the view-first bindings return (`ReactiveBinding<TView, TValue>`): the view, the
/// values the binding moves and its direction, and the subscription that disposing it ends.
pub struct ReactiveBinding<V: IViewFor + ?Sized, T> {
    view: Rc<V>,
    changed: Rc<dyn IObservable<T>>,
    direction: BindingDirection,
    subscription: Rc<dyn IDisposable>,
    disposed: Cell<bool>,
}

impl<V: IViewFor + ?Sized, T> ReactiveBinding<V, T> {
    /// The binding of `view`, whose values are `changed`, in `direction`; disposing it disposes
    /// `subscription` (`ReactiveBinding(TView, IObservable<TValue>, BindingDirection, IDisposable)`).
    pub fn new(view: Rc<V>, changed: Rc<dyn IObservable<T>>, direction: BindingDirection, subscription: Rc<dyn IDisposable>) -> Rc<Self> {
        Rc::new(Self { view, changed, direction, subscription, disposed: Cell::new(false) })
    }
}

impl<V: IViewFor + ?Sized, T> IDisposable for ReactiveBinding<V, T> {
    /// Disposes the subscription on the first call; later calls do nothing.
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.subscription.dispose();
    }
}

impl<V: IViewFor + ?Sized, T> IReactiveBinding<V, T> for ReactiveBinding<V, T> {
    /// `None`: the binding carries no expression, as upstream's generated bindings.
    fn view_model_expression(&self) -> Option<&str> {
        None
    }

    fn view(&self) -> &Rc<V> {
        &self.view
    }

    /// `None`: the binding carries no expression, as upstream's generated bindings.
    fn view_expression(&self) -> Option<&str> {
        None
    }

    fn changed(&self) -> Rc<dyn IObservable<T>> {
        self.changed.clone()
    }

    fn direction(&self) -> BindingDirection {
        self.direction
    }
}

impl<V: IViewFor + ?Sized, T> fmt::Debug for ReactiveBinding<V, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReactiveBinding: {:?} binding", self.direction)
    }
}
