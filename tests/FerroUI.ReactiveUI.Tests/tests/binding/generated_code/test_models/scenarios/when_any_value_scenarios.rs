//! `WhenAnyValueScenarios` (`ReactiveUI.Binding.GeneratedCode.TestModels/Scenarios/WhenAnyValueScenarios.cs`).
//!
//! Upstream's `x => x.Child!.Name` reads through a nullable intermediate with the null-forgiving
//! operator and produces nothing while the intermediate is null; here the path continues with
//! `then_optional_property`, so its value is an `Option` that is `None` while there is no child.

use crate::tests::binding::generated_code::test_models::{BigViewModel, HostTestFixture, TestViewModel, WhenAnyTestFixture};
use ferroui_reactiveui::*;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// `WhenAnyValue` of `Value1` (`SingleProperty`).
pub fn single_property(fixture: &Rc<WhenAnyTestFixture>) -> Rc<dyn IObservable<String>> {
    fixture.when_any_value(prop!(WhenAnyTestFixture => value1))
}

/// `WhenAnyValue` of `Value1` and `Value2`, as a tuple (`TwoProperties`).
pub fn two_properties(fixture: &Rc<WhenAnyTestFixture>) -> Rc<dyn IObservable<(String, String)>> {
    fixture.when_any_value((prop!(WhenAnyTestFixture => value1), prop!(WhenAnyTestFixture => value2)))
}

/// `WhenAnyValue` of `Value1`, `Value2` and `Value3`, as a tuple (`ThreeProperties`).
pub fn three_properties(fixture: &Rc<WhenAnyTestFixture>) -> Rc<dyn IObservable<(String, String, String)>> {
    fixture.when_any_value((prop!(WhenAnyTestFixture => value1), prop!(WhenAnyTestFixture => value2), prop!(WhenAnyTestFixture => value3)))
}

/// `WhenAnyValue` of `Value1` and `Value2` combined by a selector (`WithSelector_TwoProperties`).
pub fn with_selector_two_properties(fixture: &Rc<WhenAnyTestFixture>) -> Rc<dyn IObservable<String>> {
    fixture.when_any_value_select((prop!(WhenAnyTestFixture => value1), prop!(WhenAnyTestFixture => value2)), |(v1, v2)| format!("{v1}_{v2}"))
}

/// `WhenAnyValue` of `Address.City` (`DeepChain_AddressCity`).
pub fn deep_chain_address_city(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_any_value(prop!(BigViewModel => address.city))
}

/// `WhenAnyValue` of `Child!.Name` (`DeepChain_ChildName`): `None` while there is no child.
pub fn deep_chain_child_name(host: &Rc<HostTestFixture>) -> Rc<dyn IObservable<Option<String>>> {
    host.when_any_value(prop!(HostTestFixture => child).then_optional_property("Name", |child: &Rc<TestViewModel>| child.name()))
}

/// `WhenAnyValue` of `Child!.Child!.Name` (`DeepChain_GrandchildName`): `None` while a link is
/// missing. The second nullable link is `then_optional_property_and_then`.
pub fn deep_chain_grandchild_name(host: &Rc<HostTestFixture>) -> Rc<dyn IObservable<Option<String>>> {
    host.when_any_value(grandchild_name_path())
}

/// The path `Child!.Child!.Name` of a host (its text is `Child.Child.Name`).
pub fn grandchild_name_path() -> PropertyPath<Rc<HostTestFixture>, Option<String>> {
    prop!(HostTestFixture => child)
        .then_optional_property_and_then("Child", |child: &Rc<TestViewModel>| child.child())
        .then_optional_property("Name", |grandchild: &Rc<TestViewModel>| grandchild.name())
}

/// `WhenAnyValue` of `Child!.Age` (`DeepChain_ChildAge`): `None` while there is no child.
pub fn deep_chain_child_age(host: &Rc<HostTestFixture>) -> Rc<dyn IObservable<Option<i32>>> {
    host.when_any_value(prop!(HostTestFixture => child).then_optional_property("Age", |child: &Rc<TestViewModel>| child.age()))
}
