//! `WhenPropertyChangedFixture.UnitTests` (`DynamicData.Tests/Binding/WhenPropertyChangedFixture.UnitTests.cs`).
//!
//! The generator of the class (`_randomizer`) is
//! [`unit_tests_randomizer`](super::when_property_changed_fixture::unit_tests_randomizer).

use super::when_property_changed_fixture::{ChildModel, IHasAge};
use crate::utilities::ObservableExtensions;
use ferroui_dynamic_data::binding::NotifyPropertyChangedEx;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// `ObserveAge<T>(T source) where T : IHasAge`: the age through the interface.
fn observe_age<T: IHasAge + 'static>(source: &Rc<T>) -> Rc<dyn IObservable<i32>> {
    source.when_value_changed(prop!(T => age))
}

// https://github.com/reactivemarbles/DynamicData/issues/1149
#[test]
fn expression_contains_implicit_interface_cast() {
    let child = ChildModel::new(10);

    let (subscription, results) = observe_age(&child).record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert_eq!(results.recorded_values().len(), 1, "the initial value of the observed expression should have been published");
    assert_eq!(results.recorded_values()[0], child.age(), "the initial value of the observed expression should have been published");

    child.set_age(child.age() + 1);

    assert!(results.error().is_none(), "no errors should have occurred");
    assert_eq!(results.recorded_values().len() - 1, 1, "the value of the observed expression changed once");
    assert_eq!(results.recorded_values()[1], child.age(), "the correct value should have been published");

    subscription.dispose();
}
