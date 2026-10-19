//! `WhenChangingExtendedScenarios` (`ReactiveUI.Binding.GeneratedCode.TestModels/Scenarios/WhenChangingExtendedScenarios.cs`).

#![allow(clippy::type_complexity)]

use crate::tests::binding::generated_code::test_models::BigViewModel;
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// `WhenChanging` of `Prop1` to `Prop5`, as a tuple (`FiveProperties`).
pub fn five_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String)>> {
    vm.when_changing((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
    ))
}

/// `WhenChanging` of `Prop1` to `Prop6`, as a tuple (`SixProperties`).
pub fn six_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32)>> {
    vm.when_changing((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
        prop!(BigViewModel => prop6),
    ))
}

/// `WhenChanging` of `Prop1` to `Prop7`, as a tuple (`SevenProperties`).
pub fn seven_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64)>> {
    vm.when_changing((
        prop!(BigViewModel => prop1),
        prop!(BigViewModel => prop2),
        prop!(BigViewModel => prop3),
        prop!(BigViewModel => prop4),
        prop!(BigViewModel => prop5),
        prop!(BigViewModel => prop6),
        prop!(BigViewModel => prop7),
    ))
}

/// `WhenChanging` of `Prop1` to `Prop8`, as a tuple (`EightProperties`).
pub fn eight_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool)>> {
    vm.when_changing((
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

/// `WhenChanging` of `Prop1` to `Prop9`, as a tuple (`NineProperties`).
pub fn nine_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String)>> {
    vm.when_changing((
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

/// `WhenChanging` of `Prop1` to `Prop10`, as a tuple (`TenProperties`).
pub fn ten_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32)>> {
    vm.when_changing((
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

/// `WhenChanging` of `Prop1` to `Prop11`, as a tuple (`ElevenProperties`).
pub fn eleven_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32, f64)>> {
    vm.when_changing((
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

/// `WhenChanging` of `Prop1` to `Prop12`, as a tuple (`TwelveProperties`).
pub fn twelve_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32, f64, bool)>> {
    vm.when_changing((
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

/// `WhenChanging` of `Prop1` to `Prop13`, as a tuple (`ThirteenProperties`).
pub fn thirteen_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32, f64, bool, String)>> {
    vm.when_changing((
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
        prop!(BigViewModel => prop13),
    ))
}

/// `WhenChanging` of `Prop1` to `Prop14`, as a tuple (`FourteenProperties`).
pub fn fourteen_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32, f64, bool, String, i32)>> {
    vm.when_changing((
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
        prop!(BigViewModel => prop13),
        prop!(BigViewModel => prop14),
    ))
}

/// `WhenChanging` of `Prop1` to `Prop15`, as a tuple (`FifteenProperties`).
pub fn fifteen_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32, f64, bool, String, i32, f64)>> {
    vm.when_changing((
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
        prop!(BigViewModel => prop13),
        prop!(BigViewModel => prop14),
        prop!(BigViewModel => prop15),
    ))
}

/// `WhenChanging` of `Prop1` to `Prop16`, as a tuple (`SixteenProperties`).
pub fn sixteen_properties(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<(String, i32, f64, bool, String, i32, f64, bool, String, i32, f64, bool, String, i32, f64, bool)>> {
    vm.when_changing((
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
        prop!(BigViewModel => prop13),
        prop!(BigViewModel => prop14),
        prop!(BigViewModel => prop15),
        prop!(BigViewModel => prop16),
    ))
}

// Not ported: WithSelector_FiveProperties to WithSelector_SixteenProperties - no test uses them.

/// `WhenChanging` of `Address.City` (`DeepChain_AddressCity`).
pub fn deep_chain_address_city(vm: &Rc<BigViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_changing(prop!(BigViewModel => address.city))
}
