//! `ReactiveWindowUserControlBindingTests` (`ReactiveWindowUserControlBindingTests.cs`): the
//! synchronization of the view model and the data context of the reactive user control and window,
//! typed and untyped.

use super::view_support::{same, start};
use crate::{view_model_of_value, ReactiveUserControl, ReactiveUserControlBase, ReactiveWindow, ReactiveWindowBase};
use ferroui_base::BoxedValue;
use ferroui_reactiveui::{reactive_object, IViewFor, ReactiveObject};
use std::any::Any;
use std::rc::Rc;

/// A test view model (`VM`).
struct Vm {
    reactive: ReactiveObject,
}
reactive_object!(Vm);

impl PartialEq for Vm {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Vm {
    fn new() -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new() })
    }
}

/// An object that is not a view model of the typed views (`new object()`).
#[derive(PartialEq)]
struct Plain(u8);

fn plain() -> Rc<dyn Any> {
    Rc::new(Plain(0))
}

/// A test reactive user control for `Vm` (`C : ReactiveUserControl<VM>`).
type C = ReactiveUserControl<Vm>;

/// A test reactive window for `Vm` (`W : ReactiveWindow<VM>`).
type W = ReactiveWindow<Vm>;

/// Whether the untyped value `value` holds the object `expected` (`IsSameReferenceAs`).
fn holds(value: Option<BoxedValue>, expected: &Rc<dyn Any>) -> bool {
    value.is_some_and(|value| same(&view_model_of_value(&value), expected))
}

#[test]
fn reactive_user_control_data_context_and_view_model_sync() {
    let _app = start();
    let c = C::new();
    let vm = Vm::new();

    c.set_data_context(Some(vm.clone() as BoxedValue));
    assert!(c.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));

    let vm2 = Vm::new();
    c.set_view_model(Some(vm2.clone()));
    assert!(c.data_context().is_some_and(|data_context| same(&data_context, &vm2)));
}

#[test]
fn reactive_user_control_view_model_set_to_null_syncs_data_context() {
    let _app = start();
    let c = C::new();
    let vm = Vm::new();
    c.set_data_context(Some(vm.clone() as BoxedValue));
    assert!(c.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));

    c.set_view_model(None);
    assert!(c.data_context().is_none());
}

#[test]
fn reactive_user_control_i_view_for_view_model_delegates_to_typed_view_model() {
    let _app = start();
    let c = C::new();
    let vm = Vm::new();
    let view: Rc<dyn IViewFor> = c.clone().into_view_for();

    view.set_view_model(Some(vm.clone() as Rc<dyn Any>));

    assert!(view.view_model().is_some_and(|view_model| same(&view_model, &vm)));
    assert!(c.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
}

#[test]
fn reactive_user_control_incompatible_data_context_does_not_replace_view_model() {
    let _app = start();
    let c = C::new();
    let vm = Vm::new();
    c.set_data_context(Some(vm.clone() as BoxedValue));

    c.set_data_context(Some(Rc::new(Plain(1)) as BoxedValue));

    assert!(c.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
}

#[test]
fn reactive_window_data_context_and_view_model_sync() {
    let _app = start();
    let window = W::new();
    let vm = Vm::new();

    window.set_data_context(Some(vm.clone() as BoxedValue));
    assert!(window.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));

    let vm2 = Vm::new();
    window.set_view_model(Some(vm2.clone()));
    assert!(window.data_context().is_some_and(|data_context| same(&data_context, &vm2)));
}

#[test]
fn reactive_window_view_model_set_to_null_syncs_data_context() {
    let _app = start();
    let window = W::new();
    let vm = Vm::new();
    window.set_data_context(Some(vm.clone() as BoxedValue));
    assert!(window.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));

    window.set_view_model(None);
    assert!(window.data_context().is_none());
}

#[test]
fn reactive_window_i_view_for_view_model_delegates_to_typed_view_model() {
    let _app = start();
    let window = W::new();
    let vm = Vm::new();
    let view: Rc<dyn IViewFor> = window.clone().into_view_for();

    view.set_view_model(Some(vm.clone() as Rc<dyn Any>));

    assert!(view.view_model().is_some_and(|view_model| same(&view_model, &vm)));
    assert!(window.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
}

#[test]
fn reactive_window_incompatible_data_context_does_not_replace_view_model() {
    let _app = start();
    let window = W::new();
    let vm = Vm::new();
    window.set_data_context(Some(vm.clone() as BoxedValue));

    window.set_data_context(Some(Rc::new(Plain(1)) as BoxedValue));

    assert!(window.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
}

#[test]
fn reactive_user_control_base_data_context_and_view_model_sync() {
    let _app = start();
    let control = ReactiveUserControlBase::new();
    let vm = Rc::new(Plain(0));

    control.set_data_context(Some(vm.clone() as BoxedValue));

    let view = ReactiveUserControlBase::view_for(&control);
    assert!(view.view_model().is_some_and(|view_model| same(&view_model, &vm)));
}

#[test]
fn reactive_window_base_data_context_and_view_model_sync() {
    let _app = start();
    let window = ReactiveWindowBase::new();
    let vm = Rc::new(Plain(0));

    window.set_data_context(Some(vm.clone() as BoxedValue));

    let view = ReactiveWindowBase::view_for(&window);
    assert!(view.view_model().is_some_and(|view_model| same(&view_model, &vm)));
}

#[test]
fn reactive_user_control_base_i_view_for_replaces_and_clears_data_context() {
    let _app = start();
    let control = ReactiveUserControlBase::new();
    let view = ReactiveUserControlBase::view_for(&control);
    let first = plain();
    let second = plain();

    view.set_view_model(Some(first.clone()));
    assert!(holds(control.data_context(), &first));
    view.set_view_model(Some(second.clone()));
    assert!(holds(control.data_context(), &second));
    control.set_data_context(None);
    assert!(view.view_model().is_none());
}

#[test]
fn reactive_window_base_i_view_for_replaces_and_clears_data_context() {
    let _app = start();
    let window = ReactiveWindowBase::new();
    let view = ReactiveWindowBase::view_for(&window);
    let first = plain();
    let second = plain();

    view.set_view_model(Some(first.clone()));
    assert!(holds(window.data_context(), &first));
    view.set_view_model(Some(second.clone()));
    assert!(holds(window.data_context(), &second));
    window.set_data_context(None);
    assert!(view.view_model().is_none());
}
