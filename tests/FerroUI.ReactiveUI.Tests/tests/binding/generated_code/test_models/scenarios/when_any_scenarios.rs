//! `WhenAnyScenarios` (`ReactiveUI.Binding.GeneratedCode.TestModels/Scenarios/WhenAnyScenarios.cs`).

use crate::tests::binding::generated_code::test_models::TestViewModel;
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// `WhenAny` of `Name` with a selector that takes the value (`SingleProperty_Name`).
pub fn single_property_name(vm: &Rc<TestViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_any(prop!(TestViewModel => name), |change| change.value)
}

/// `WhenAny` of `Name` that produces the changes themselves (`SingleProperty_Name_ObservedChange`).
pub fn single_property_name_observed_change(vm: &Rc<TestViewModel>) -> Rc<dyn IObservable<ObservedChange<Rc<TestViewModel>, String>>> {
    vm.when_any(prop!(TestViewModel => name), |change| change)
}

/// `WhenAny` of `Name` and `Age` combined by a selector (`TwoProperties_NameAge`).
pub fn two_properties_name_age(vm: &Rc<TestViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_any((prop!(TestViewModel => name), prop!(TestViewModel => age)), |(name, age)| format!("{}_{}", name.value, age.value))
}
