//! `WhenAnyValueExtendedScenarios` (`ReactiveUI.Binding.GeneratedCode.TestModels/Scenarios/WhenAnyValueExtendedScenarios.cs`).

#![allow(clippy::type_complexity)]

use crate::tests::binding::generated_code::test_models::{BigViewModel, WhenAnyTestFixture};
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// `WhenAnyValue` of `Prop1` to `Prop4`, as a tuple (`FourProperties`).
pub fn four_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool)>> {
    vm.when_any_value((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
    ))
}

/// `WhenAnyValue` of `Prop1` to `Prop5`, as a tuple (`FiveProperties`).
pub fn five_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String)>> {
    vm.when_any_value((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
    ))
}

/// `WhenAnyValue` of `Prop1` to `Prop6`, as a tuple (`SixProperties`).
pub fn six_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32)>> {
    vm.when_any_value((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
        prop!(BigViewModel => prop6),
    ))
}

/// `WhenAnyValue` of `Prop1` to `Prop7`, as a tuple (`SevenProperties`).
pub fn seven_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64)>> {
    vm.when_any_value((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
        prop!(BigViewModel => prop6),
        prop!(BigViewModel => prop7),
    ))
}

/// `WhenAnyValue` of `Prop1` to `Prop8`, as a tuple (`EightProperties`).
pub fn eight_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool)>> {
    vm.when_any_value((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
        prop!(BigViewModel => prop6),
        prop!(BigViewModel => prop7),
        prop!(BigViewModel => prop8),
    ))
}

/// `WhenAnyValue` of `Prop1` to `Prop9`, as a tuple (`NineProperties`).
pub fn nine_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String)>> {
    vm.when_any_value((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
        prop!(BigViewModel => prop6),
        prop!(BigViewModel => prop7),
        prop!(BigViewModel => prop8),
        prop!(BigViewModel => prop9),
    ))
}

/// `WhenAnyValue` of `Prop1` to `Prop10`, as a tuple (`TenProperties`).
pub fn ten_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32)>> {
    vm.when_any_value((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
        prop!(BigViewModel => prop6),
        prop!(BigViewModel => prop7),
        prop!(BigViewModel => prop8),
        prop!(BigViewModel => prop9),
        prop!(BigViewModel => prop10),
    ))
}

/// `WhenAnyValue` of `Prop1` to `Prop11`, as a tuple (`ElevenProperties`).
pub fn eleven_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32, f64)>> {
    vm.when_any_value((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
        prop!(BigViewModel => prop6),
        prop!(BigViewModel => prop7),
        prop!(BigViewModel => prop8),
        prop!(BigViewModel => prop9),
        prop!(BigViewModel => prop10),
        prop!(BigViewModel => prop11),
    ))
}

/// `WhenAnyValue` of `Prop1` to `Prop12`, as a tuple (`TwelveProperties`).
pub fn twelve_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32, f64, bool)>> {
    vm.when_any_value((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
        prop!(BigViewModel => prop6),
        prop!(BigViewModel => prop7),
        prop!(BigViewModel => prop8),
        prop!(BigViewModel => prop9),
        prop!(BigViewModel => prop10),
        prop!(BigViewModel => prop11),
        prop!(BigViewModel => prop12),
    ))
}

// Not ported: ThirteenProperties, FourteenProperties, FifteenProperties, SixteenProperties - needs
// arity 13 to 16 (the tuple traits stop at 12).
// Not ported: WithSelector_FourProperties to WithSelector_FifteenProperties - no test uses them.
// Not ported: WithSelector_SixteenProperties - needs arity 13 to 16 (the tuple traits stop at 12).

/// `WhenAnyValue` of `Address.City` (`DeepChain_AddressCity`).
pub fn deep_chain_address_city(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_any_value(prop!(BigViewModel => address.city))
}

/// `WhenAnyValue` of the twelve properties of a [`WhenAnyTestFixture`], as a tuple
/// (`TwelveProperties_AllStrings`).
pub fn twelve_properties_all_strings(
    fixture: &Rc<WhenAnyTestFixture>,
) -> Rc<dyn IObservable<(String, String, String, String, String, String, String, String, String, String, String, String)>> {
    fixture.when_any_value((
        prop!(WhenAnyTestFixture => value1),
        prop!(WhenAnyTestFixture => value2),
        prop!(WhenAnyTestFixture => value3),
        prop!(WhenAnyTestFixture => value4),
        prop!(WhenAnyTestFixture => value5),
        prop!(WhenAnyTestFixture => value6),
        prop!(WhenAnyTestFixture => value7),
        prop!(WhenAnyTestFixture => value8),
        prop!(WhenAnyTestFixture => value9),
        prop!(WhenAnyTestFixture => value10),
        prop!(WhenAnyTestFixture => value11),
        prop!(WhenAnyTestFixture => value12),
    ))
}
