//! `...ObjectObservableForPropertyTests` (the tests of ReactiveUI.Binding's integration).
//!
//! Upstream passes the type of the object to the affinity; the port passes the object, as the
//! owner of the property ([`ferro_property_owner`]).

use super::binding_support::{start_app, ListObserver, TestObject};
use crate::{ferro_property_owner, FerroObjectObservableForProperty};
use ferroui_reactiveui::{ICreatesObservableForProperty, ObservedChange};
use ferroui_system::component_model::PropertyOwner;
use std::any::Any;
use std::rc::Rc;

/// The affinity of a dependency property of WPF, which a registered property shares.
const REGISTERED_PROPERTY_AFFINITY: i32 = 4;

/// The value the tests write to the observed property.
const NEW_TEXT: &str = "first";

/// The owner of a value that is not a FerroUI object.
fn plain_owner() -> PropertyOwner {
    PropertyOwner { object: Some(Rc::new(String::new()) as Rc<dyn Any>), notify: None }
}

#[test]
fn get_affinity_for_object_with_a_non_ferro_type_is_zero() {
    assert_eq!(FerroObjectObservableForProperty::new().get_affinity_for_object(&plain_owner(), "Length", false), 0);
}

#[test]
fn get_affinity_for_object_with_a_registered_property_is_the_dependency_property_affinity() {
    let _app = start_app();
    let owner = ferro_property_owner(&TestObject::new());

    assert_eq!(FerroObjectObservableForProperty::new().get_affinity_for_object(&owner, "Text", false), REGISTERED_PROPERTY_AFFINITY);
}

#[test]
fn get_affinity_for_object_with_a_plain_property_is_zero() {
    let _app = start_app();
    let owner = ferro_property_owner(&TestObject::new());

    assert_eq!(FerroObjectObservableForProperty::new().get_affinity_for_object(&owner, "Plain", true), 0);
}

#[test]
#[should_panic(expected = "ArgumentNullException")]
fn get_notification_for_property_with_a_null_sender_throws() {
    let _ = FerroObjectObservableForProperty::new().get_notification_for_property(&PropertyOwner::default(), "Text", false, false);
}

#[test]
#[should_panic(expected = "The sender is not a FerroObject")]
fn get_notification_for_property_with_a_non_ferro_sender_throws() {
    let _ = FerroObjectObservableForProperty::new().get_notification_for_property(&plain_owner(), "Text", false, false);
}

/// `GetNotificationForProperty_WithAnUnregisteredProperty_Throws(false)`.
#[test]
#[should_panic(expected = "MissingMemberException")]
fn get_notification_for_property_with_an_unregistered_property_throws_with_the_message() {
    let _app = start_app();
    let owner = ferro_property_owner(&TestObject::new());

    let _ = FerroObjectObservableForProperty::new().get_notification_for_property(&owner, "Plain", false, false);
}

/// `GetNotificationForProperty_WithAnUnregisteredProperty_Throws(true)`.
#[test]
#[should_panic(expected = "MissingMemberException")]
fn get_notification_for_property_with_an_unregistered_property_throws_without_the_message() {
    let _app = start_app();
    let owner = ferro_property_owner(&TestObject::new());

    let _ = FerroObjectObservableForProperty::new().get_notification_for_property(&owner, "Plain", false, true);
}

#[test]
fn get_notification_for_property_raises_only_the_observed_propertys_changes() {
    let _app = start_app();
    let target = TestObject::new();
    let owner = ferro_property_owner(&target);
    let observer = ListObserver::<ObservedChange<PropertyOwner, ()>>::new();

    let subscription = FerroObjectObservableForProperty::new().get_notification_for_property(&owner, "Text", false, false).subscribe(observer.clone());
    target.set_count(1);
    target.set_text(Some(NEW_TEXT.to_string()));
    subscription.dispose();

    let values = observer.values();
    assert_eq!(values.len(), 1);
    // The change carries no value: the reader reads it from the object.
    assert_eq!(target.text(), Some(NEW_TEXT.to_string()));
    assert!(values[0].sender.same_object(&owner));
    assert_eq!(values[0].expression, "Text");
}

#[test]
fn get_notification_for_property_after_disposal_raises_nothing() {
    let _app = start_app();
    let target = TestObject::new();
    let owner = ferro_property_owner(&target);
    let observer = ListObserver::<ObservedChange<PropertyOwner, ()>>::new();

    FerroObjectObservableForProperty::new().get_notification_for_property(&owner, "Text", true, false).subscribe(observer.clone()).dispose();
    target.set_text(Some(NEW_TEXT.to_string()));

    assert!(observer.values().is_empty());
}
