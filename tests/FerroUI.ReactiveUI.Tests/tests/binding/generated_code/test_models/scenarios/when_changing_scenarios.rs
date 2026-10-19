//! `WhenChangingScenarios` (`ReactiveUI.Binding.GeneratedCode.TestModels/Scenarios/WhenChangingScenarios.cs`).

use crate::tests::binding::generated_code::test_models::{BigViewModel, TestViewModel};
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// `WhenChanging` of `Name` (`SingleProperty_Name`).
pub fn single_property_name(vm: &Rc<TestViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_changing(prop!(TestViewModel => name))
}

// Not ported: SingleProperty_Prop1 - no test uses it.

/// `WhenChanging` of `Prop1` and `Prop2`, as a tuple (`TwoProperties`).
pub fn two_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32)>> {
    vm.when_changing((prop!(BigViewModel => prop1), prop!(BigViewModel => prop2)))
}

/// `WhenChanging` of `Prop1` to `Prop3`, as a tuple (`ThreeProperties`).
pub fn three_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64)>> {
    vm.when_changing((prop!(BigViewModel => prop1), prop!(BigViewModel => prop2), prop!(BigViewModel => prop3)))
}

/// `WhenChanging` of `Prop1` to `Prop4`, as a tuple (`FourProperties`).
pub fn four_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool)>> {
    vm.when_changing((prop!(BigViewModel => prop1), prop!(BigViewModel => prop2), prop!(BigViewModel => prop3), prop!(BigViewModel => prop4)))
}

/// `WhenChanging` of `Address.City` (`DeepChain_AddressCity`).
pub fn deep_chain_address_city(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_changing(prop!(BigViewModel => address.city))
}
