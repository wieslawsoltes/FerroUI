//! `POCOObservableForPropertyTests` (`ReactiveUI.Binding.Tests/ObservableForProperty/POCOObservableForPropertyTests.cs`).
//!
//! Upstream passes a `Type` to `GetAffinityForObject` and an object with an expression to
//! `GetNotificationForProperty`; the port passes the object as a `PropertyOwner` and the name of
//! the property.

use crate::tests::binding::test_models::{PocoModel, TestViewModel};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyOwner};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The name of the property the tests observe.
const VALUE_PROPERTY_NAME: &str = "Value";

/// The owner of the properties of an object without change notification.
fn poco_owner<M: 'static>(object: &Rc<M>) -> PropertyOwner {
    PropertyOwner { object: Some(object.clone() as Rc<dyn Any>), notify: None }
}

/// The owner of the properties of an object that raises `PropertyChanged`.
fn notifying_owner<M: INotifyPropertyChanged + 'static>(object: &Rc<M>) -> PropertyOwner {
    PropertyOwner { object: Some(object.clone() as Rc<dyn Any>), notify: Some(object.clone() as Rc<dyn INotifyPropertyChanged>) }
}

/// A model whose `Value` is `value`.
fn model_with(value: &str) -> Rc<PocoModel> {
    let model = PocoModel::new();
    model.set_value(value.to_string());
    model
}

#[test]
fn get_affinity_for_object_any_type_returns1() {
    let sut = POCOObservableForProperty::new();

    let affinity = sut.get_affinity_for_object_after_change(&poco_owner(&PocoModel::new()), VALUE_PROPERTY_NAME);

    assert_eq!(affinity, 1);
}

#[test]
fn get_affinity_for_object_inpc_type_still_returns1() {
    let sut = POCOObservableForProperty::new();

    let affinity = sut.get_affinity_for_object_after_change(&notifying_owner(&TestViewModel::new()), "Name");

    assert_eq!(affinity, 1);
}

#[test]
fn get_notification_for_property_emits_one_value() {
    let sut = POCOObservableForProperty::new();
    let model = model_with("Hello");

    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    let _subscription = sut
        .get_notification_for_property_after_change(&poco_owner(&model), VALUE_PROPERTY_NAME)
        .take(1)
        .subscribe_next(move |_| counter.set(counter.get() + 1));

    assert_eq!(count.get(), 1);
}

#[test]
fn get_notification_for_property_no_further_emissions() {
    let sut = POCOObservableForProperty::new();
    let model = model_with("Hello");

    let values: Rc<RefCell<Vec<ObservedChange<PropertyOwner, ()>>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    let _subscription = sut
        .get_notification_for_property_after_change(&poco_owner(&model), VALUE_PROPERTY_NAME)
        .take(1)
        .subscribe_next(move |change| sink.borrow_mut().push(change));

    model.set_value("World".to_string());

    // Only the initial value was produced.
    assert_eq!(values.borrow().len(), 1);
}
