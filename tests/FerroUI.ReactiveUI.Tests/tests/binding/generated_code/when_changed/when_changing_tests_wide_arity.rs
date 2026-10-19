//! `WhenChangingTests` (`ReactiveUI.Binding.GeneratedCode.Tests/WhenChanged/WhenChangingTests.WideArity.cs`).
//!
//! `WhenChanging` produces the values before each change; these tests check the initial values.

use crate::tests::binding::generated_code::test_models::scenarios::when_changing_extended_scenarios;
use crate::tests::binding::generated_code::test_models::BigViewModel;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::cell::Cell;
use std::rc::Rc;

/// Whether `source` produces a value on subscription (upstream's `received` flag).
fn emits_on_subscribe<T: 'static>(source: &Rc<dyn IObservable<T>>) -> bool {
    let received = Rc::new(Cell::new(false));
    let flag = received.clone();
    let _subscription = source.subscribe_next(move |_| flag.set(true));
    received.get()
}

#[test]
fn five_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::five_properties(&vm)));
}

#[test]
fn six_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::six_properties(&vm)));
}

#[test]
fn seven_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::seven_properties(&vm)));
}

#[test]
fn eight_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::eight_properties(&vm)));
}

#[test]
fn nine_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::nine_properties(&vm)));
}

#[test]
fn ten_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::ten_properties(&vm)));
}

#[test]
fn eleven_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::eleven_properties(&vm)));
}

#[test]
fn twelve_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::twelve_properties(&vm)));
}

#[test]
fn thirteen_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::thirteen_properties(&vm)));
}

#[test]
fn fourteen_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::fourteen_properties(&vm)));
}

#[test]
fn fifteen_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::fifteen_properties(&vm)));
}

#[test]
fn sixteen_properties_emits_initial_values() {
    let vm = BigViewModel::create_populated();

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::sixteen_properties(&vm)));
}

#[test]
fn deep_chain_address_city() {
    let vm = BigViewModel::new();
    vm.address().set_city("Seattle".to_string());

    assert!(emits_on_subscribe(&when_changing_extended_scenarios::deep_chain_address_city(&vm)));
}
