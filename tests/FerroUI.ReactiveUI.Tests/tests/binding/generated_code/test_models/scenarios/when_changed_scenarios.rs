//! `WhenChangedScenarios` (`ReactiveUI.Binding.GeneratedCode.TestModels/Scenarios/WhenChangedScenarios.cs`).

use crate::tests::binding::generated_code::test_models::{BigViewModel, HostTestFixture, TestViewModel};
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// `WhenChanged` of `Name` (`SingleProperty_Name`).
pub fn single_property_name(vm: &Rc<TestViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_changed(prop!(TestViewModel => name))
}

/// `WhenChanged` of `Age` (`SingleProperty_Age`).
pub fn single_property_age(vm: &Rc<TestViewModel>) -> Rc<dyn IObservable<i32>> {
    vm.when_changed(prop!(TestViewModel => age))
}

/// `WhenChanged` of `Prop1` and `Prop2`, as a tuple (`TwoProperties`).
pub fn two_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32)>> {
    vm.when_changed((prop!(BigViewModel => prop1), prop!(BigViewModel => prop2)))
}

/// `WhenChanged` of `Prop1` to `Prop3`, as a tuple (`ThreeProperties`).
pub fn three_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64)>> {
    vm.when_changed((prop!(BigViewModel => prop1), prop!(BigViewModel => prop2), prop!(BigViewModel => prop3)))
}

/// `WhenChanged` of `Prop1` to `Prop4`, as a tuple (`FourProperties`).
pub fn four_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool)>> {
    vm.when_changed((prop!(BigViewModel => prop1), prop!(BigViewModel => prop2), prop!(BigViewModel => prop3), prop!(BigViewModel => prop4)))
}

/// `WhenChanged` of `Prop1` and `Prop2` converted by a selector (`WithSelector_TwoProperties`).
pub fn with_selector_two_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_changed_select((prop!(BigViewModel => prop1), prop!(BigViewModel => prop2)), |(p1, p2)| format!("{p1}_{p2}"))
}

/// `WhenChanged` of `Address.City` (`DeepChain_AddressCity`).
pub fn deep_chain_address_city(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_changed(prop!(BigViewModel => address.city))
}

/// `WhenChanged` of `Address.Street` (`DeepChain_AddressStreet`).
pub fn deep_chain_address_street(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_changed(prop!(BigViewModel => address.street))
}

/// `WhenChanged` of `Child!.Name` (`DeepChain_ChildName`): `None` while there is no child, where
/// upstream produces nothing.
pub fn deep_chain_child_name(host: &Rc<HostTestFixture>) -> Rc<dyn IObservable<Option<String>>> {
    host.when_changed(prop!(HostTestFixture => child).then_optional_property("Name", |child: &Rc<TestViewModel>| child.name()))
}

/// `WhenChanged` of `Address.City` and `Prop1`, as a tuple (`MultiProperty_WithDeepChain`).
pub fn multi_property_with_deep_chain(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, String)>> {
    vm.when_changed((prop!(BigViewModel => address.city), prop!(BigViewModel => prop1)))
}
