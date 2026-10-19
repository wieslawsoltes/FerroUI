//! `WhenAnyValueTests` (`ReactiveUI.Binding.GeneratedCode.Tests/WhenAnyValue/WhenAnyValueTests.DeepChain.cs`).
//!
//! A path through a nullable intermediate (`x => x.Child!.Name`) produces an `Option` here: `None`
//! while a link is missing, where upstream produces nothing. The tests that compare traces with
//! upstream's compare the values that are present, and also check the whole sequence.
//!
//! Upstream compares its generated code with `RuntimeObservationFallback`, the run-time
//! observation; the port has only the run-time observation, so those tests compare
//! `when_any_value` with the expression chain it is built on (`subscribe_to_expression_chain`).

use crate::mocks::collect_items;
use crate::tests::binding::generated_code::test_models::scenarios::{when_any_value_extended_scenarios, when_any_value_scenarios};
use crate::tests::binding::generated_code::test_models::{Address, BigViewModel, HostTestFixture, TestViewModel, WhenAnyTestFixture};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The expected emission count after an initial value plus one change.
const EXPECTED_EMISSION_COUNT: usize = 2;

/// The initial value used by the disposal and multiple-subscription tests.
const INITIAL_VALUE: &str = "Initial";

/// The initial city of the deep-chain tests.
const INITIAL_CITY: &str = "Seattle";

/// The replacement city of the deep-chain tests.
const REPLACEMENT_CITY: &str = "Portland";

/// The number of changes of the rapid-change test.
const CHANGE_COUNT: usize = 100;

/// The expected emission count of the rapid-change test: the initial value plus the changes.
const EXPECTED_RAPID_EMISSION_COUNT: usize = 101;

/// The expected emission count of the twelve-property test: the initial value plus a change of
/// each property.
const EXPECTED_TWELVE_PROPERTY_COUNT: usize = 13;

/// The replacement name of the multiple-subscription test.
const REPLACEMENT_NAME: &str = "Changed";

/// The initial name of the child.
const INITIAL_CHILD_NAME: &str = "Alice";

/// The name of the replacement child.
const CHARLIE: &str = "Charlie";

/// A view model named `name`.
fn named(name: &str) -> Rc<TestViewModel> {
    let vm = TestViewModel::new();
    vm.set_name(name.to_string());
    vm
}

/// A view model whose child is named `name`.
fn with_child_named(name: &str) -> Rc<TestViewModel> {
    let vm = TestViewModel::new();
    vm.set_child(Some(named(name)));
    vm
}

/// The values present, joined with `|`, as upstream's traces are (upstream produces nothing
/// while an intermediate is missing).
fn trace(values: &[Option<String>]) -> String {
    values.iter().flatten().cloned().collect::<Vec<_>>().join("|")
}

#[test]
fn disposal_stops_listening() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1(INITIAL_VALUE.to_string());
    let values = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = values.clone();
    let subscription = when_any_value_scenarios::single_property(&fixture).subscribe_next(move |value| sink.borrow_mut().push(value));

    subscription.dispose();

    fixture.set_value1("AfterDisposal".to_string());

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], INITIAL_VALUE);
}

#[test]
fn two_properties_emits_on_either_change() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1("A".to_string());
    fixture.set_value2("B".to_string());

    let values = collect_items(&when_any_value_scenarios::two_properties(&fixture));

    assert!(!values.borrow().is_empty());

    fixture.set_value1("C".to_string());

    assert_eq!(values.borrow().last(), Some(&("C".to_string(), "B".to_string())));

    fixture.set_value2("D".to_string());

    assert_eq!(values.borrow().last(), Some(&("C".to_string(), "D".to_string())));
}

#[test]
fn deep_chain_emits_nested_property_value() {
    let vm = BigViewModel::new();
    vm.address().set_city(INITIAL_CITY.to_string());

    let values = collect_items(&when_any_value_scenarios::deep_chain_address_city(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], INITIAL_CITY);
}

#[test]
fn deep_chain_emits_on_nested_property_change() {
    let vm = BigViewModel::new();
    vm.address().set_city(INITIAL_CITY.to_string());

    let values = collect_items(&when_any_value_scenarios::deep_chain_address_city(&vm));

    vm.address().set_city(REPLACEMENT_CITY.to_string());

    assert!(values.borrow().len() >= EXPECTED_EMISSION_COUNT);
    assert!(values.borrow().contains(&REPLACEMENT_CITY.to_string()));
}

#[test]
fn deep_chain_intermediate_object_replacement() {
    let vm = BigViewModel::new();
    vm.address().set_city(INITIAL_CITY.to_string());

    let values = collect_items(&when_any_value_scenarios::deep_chain_address_city(&vm));

    let new_address = Address::new();
    new_address.set_city(REPLACEMENT_CITY.to_string());
    vm.set_address(new_address.clone());

    assert!(values.borrow().contains(&REPLACEMENT_CITY.to_string()));

    // Verify resubscription
    new_address.set_city("Eugene".to_string());

    assert!(values.borrow().contains(&"Eugene".to_string()));
}

#[test]
fn deep_chain_null_intermediate_matches_legacy_reactive_ui_semantics() {
    let generated_host = HostTestFixture::new();
    let fallback_host = HostTestFixture::new();

    let generated_values = collect_items(&when_any_value_scenarios::deep_chain_child_name(&generated_host));
    let fallback_chain: Rc<dyn IObservable<Option<String>>> = fallback_host
        .subscribe_to_expression_chain(prop!(HostTestFixture => child).then_optional_property("Name", |child: &Rc<TestViewModel>| child.name()), false, false, true)
        .select(|change: ObservedChange<Rc<HostTestFixture>, Option<String>>| change.value);
    let fallback_values = collect_items(&fallback_chain);

    generated_host.set_child(Some(named(INITIAL_CHILD_NAME)));
    fallback_host.set_child(Some(named(INITIAL_CHILD_NAME)));
    generated_host.child().expect("a child").set_name("Bob".to_string());
    fallback_host.child().expect("a child").set_name("Bob".to_string());

    generated_host.set_child(None);
    fallback_host.set_child(None);

    generated_host.set_child(Some(named(CHARLIE)));
    fallback_host.set_child(Some(named(CHARLIE)));

    let generated_trace = trace(&generated_values.borrow());
    let fallback_trace = trace(&fallback_values.borrow());
    assert_eq!(generated_trace, fallback_trace);
    assert_eq!(generated_trace, "Alice|Bob|Charlie");
    // The whole sequence: `None` while there is no child.
    assert_eq!(
        *generated_values.borrow(),
        vec![None, Some(INITIAL_CHILD_NAME.to_string()), Some("Bob".to_string()), None, Some(CHARLIE.to_string())]
    );
}

#[test]
fn deep_chain_three_links_detaches_orphaned_subtree_like_legacy_reactive_ui() {
    let generated_middle = with_child_named(INITIAL_CHILD_NAME);
    let fallback_middle = with_child_named(INITIAL_CHILD_NAME);
    let generated_host = HostTestFixture::new();
    generated_host.set_child(Some(generated_middle.clone()));
    let fallback_host = HostTestFixture::new();
    fallback_host.set_child(Some(fallback_middle.clone()));

    let generated_values = collect_items(&when_any_value_scenarios::deep_chain_grandchild_name(&generated_host));
    // The run-time form of the same path (`Child.Child.Name`).
    let fallback_path = when_any_value_scenarios::grandchild_name_path();
    assert_eq!(fallback_path.path(), "Child.Child.Name");
    let fallback_chain: Rc<dyn IObservable<Option<String>>> = fallback_host
        .subscribe_to_expression_chain(fallback_path, false, false, true)
        .select(|change: ObservedChange<Rc<HostTestFixture>, Option<String>>| change.value);
    let fallback_values = collect_items(&fallback_chain);

    generated_host.set_child(None);
    fallback_host.set_child(None);

    generated_middle.child().expect("a grandchild").set_name("Orphan mutation".to_string());
    fallback_middle.child().expect("a grandchild").set_name("Orphan mutation".to_string());

    generated_host.set_child(Some(with_child_named(CHARLIE)));
    fallback_host.set_child(Some(with_child_named(CHARLIE)));

    let generated_trace = trace(&generated_values.borrow());
    let fallback_trace = trace(&fallback_values.borrow());
    assert_eq!(generated_trace, fallback_trace);
    assert_eq!(generated_trace, "Alice|Charlie");
    // The whole sequence: `None` while the middle link is missing.
    assert_eq!(*generated_values.borrow(), vec![Some(INITIAL_CHILD_NAME.to_string()), None, Some(CHARLIE.to_string())]);
}

// Not ported: DeepChain_NullLeaf_StillEmits - a `String` property cannot hold null (upstream
// assigns `null!` to the non-nullable `Name`); a missing value of the port is the `None` of a
// missing intermediate, which the other deep-chain tests check.

#[test]
fn deep_chain_default_value_leaf_emits_only_when_intermediate_exists() {
    let host = HostTestFixture::new();

    let values = collect_items(&when_any_value_scenarios::deep_chain_child_age(&host));

    // Upstream produces nothing while there is no child; the port produces `None`.
    assert_eq!(values.borrow().iter().flatten().count(), 0);
    assert_eq!(*values.borrow(), vec![None]);

    let child = TestViewModel::new();
    child.set_age(0);
    host.set_child(Some(child));

    assert_eq!(values.borrow().iter().flatten().count(), 1);
    assert_eq!(values.borrow().iter().flatten().next(), Some(&0));
    assert_eq!(*values.borrow(), vec![None, Some(0)]);
}

#[test]
fn with_selector_emits_on_change() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1("Hello".to_string());
    fixture.set_value2("World".to_string());

    let values = collect_items(&when_any_value_scenarios::with_selector_two_properties(&fixture));

    assert_eq!(values.borrow()[0], "Hello_World");

    fixture.set_value1("Goodbye".to_string());

    assert_eq!(values.borrow().last().map(String::as_str), Some("Goodbye_World"));
}

#[test]
fn rapid_sequential_changes_all_captured() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1("A".to_string());

    let values = collect_items(&when_any_value_scenarios::single_property(&fixture));

    for i in 0..CHANGE_COUNT {
        fixture.set_value1(format!("Value_{i}"));
    }

    assert_eq!(values.borrow().len(), EXPECTED_RAPID_EMISSION_COUNT); // initial + 100 changes
    assert_eq!(values.borrow()[0], "A");
    assert_eq!(values.borrow().last().map(String::as_str), Some("Value_99"));
}

#[test]
fn multiple_subscriptions_receive_independent_emissions() {
    let fixture = WhenAnyTestFixture::new();
    fixture.set_value1(INITIAL_VALUE.to_string());

    let observable = when_any_value_scenarios::single_property(&fixture);

    let values1 = collect_items(&observable);
    let values2 = collect_items(&observable);

    fixture.set_value1(REPLACEMENT_NAME.to_string());

    assert!(values1.borrow().len() >= EXPECTED_EMISSION_COUNT);
    assert!(values2.borrow().len() >= EXPECTED_EMISSION_COUNT);
    assert!(values1.borrow().contains(&REPLACEMENT_NAME.to_string()));
    assert!(values2.borrow().contains(&REPLACEMENT_NAME.to_string()));
}

#[test]
fn twelve_properties_each_property_fires_independently() {
    let fixture = WhenAnyTestFixture::new();
    let count = Rc::new(Cell::new(0_usize));

    let counter = count.clone();
    let _subscription = when_any_value_extended_scenarios::twelve_properties_all_strings(&fixture).subscribe_next(move |_| counter.set(counter.get() + 1));

    // Initial emission
    assert_eq!(count.get(), 1);

    // Change each of the 12 properties; each should fire
    fixture.set_value1("a".to_string());
    fixture.set_value2("b".to_string());
    fixture.set_value3("c".to_string());
    fixture.set_value4("d".to_string());
    fixture.set_value5("e".to_string());
    fixture.set_value6("f".to_string());
    fixture.set_value7("g".to_string());
    fixture.set_value8("h".to_string());
    fixture.set_value9("i".to_string());
    fixture.set_value10("j".to_string());
    fixture.set_value11("k".to_string());
    fixture.set_value12("l".to_string());

    assert_eq!(count.get(), EXPECTED_TWELVE_PROPERTY_COUNT); // initial + 12 changes
}
