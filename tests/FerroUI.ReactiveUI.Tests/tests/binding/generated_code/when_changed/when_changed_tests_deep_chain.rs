//! `WhenChangedTests` (`ReactiveUI.Binding.GeneratedCode.Tests/WhenChanged/WhenChangedTests.DeepChain.cs`).
//!
//! A path through a nullable intermediate (`x => x.Child!.Name`) produces an `Option` here: `None`
//! while there is no child, where upstream produces nothing; the tests check the values present
//! and the whole sequence.

use crate::mocks::collect_items;
use crate::tests::binding::generated_code::test_models::scenarios::when_changed_scenarios;
use crate::tests::binding::generated_code::test_models::{Address, BigViewModel, HostTestFixture, TestViewModel};
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;

/// The minimum emission count after an initial value plus one change.
const MIN_EMISSIONS_AFTER_CHANGE: usize = 2;

/// The value of the first property.
const FIRST_PROPERTY_VALUE: &str = "Hello";

/// The initial city.
const DEEP_CHAIN_INITIAL_CITY: &str = "Seattle";

/// The first replacement city.
const DEEP_CHAIN_REPLACEMENT_CITY: &str = "Portland";

/// The second replacement city.
const DEEP_CHAIN_SECOND_REPLACEMENT_CITY: &str = "Eugene";

/// The initial age.
const INITIAL_AGE: i32 = 25;

/// The replacement age.
const REPLACEMENT_AGE: i32 = 30;

/// The replacement name.
const REPLACEMENT_NAME: &str = "Changed";

/// The value set after a subscription is disposed.
const AFTER_DISPOSAL_VALUE: &str = "AfterDisposal";

/// The initial name of the child.
const INITIAL_CHILD_NAME: &str = "Alice";

/// A view model named `name`.
fn named(name: &str) -> Rc<TestViewModel> {
    let vm = TestViewModel::new();
    vm.set_name(name.to_string());
    vm
}

/// An address in `city`.
fn address_in(city: &str) -> Rc<Address> {
    let address = Address::new();
    address.set_city(city.to_string());
    address
}

/// A host whose child is named `name`.
fn host_with_child_named(name: &str) -> Rc<HostTestFixture> {
    let host = HostTestFixture::new();
    host.set_child(Some(named(name)));
    host
}

#[test]
fn deep_chain_intermediate_object_replacement_re_subscribes() {
    let vm = BigViewModel::new();
    vm.address().set_city(DEEP_CHAIN_INITIAL_CITY.to_string());

    let values = collect_items(&when_changed_scenarios::deep_chain_address_city(&vm));

    assert_eq!(values.borrow()[0], DEEP_CHAIN_INITIAL_CITY);

    // Replace the entire Address object
    let new_address = address_in(DEEP_CHAIN_REPLACEMENT_CITY);
    vm.set_address(new_address.clone());

    assert!(values.borrow().contains(&DEEP_CHAIN_REPLACEMENT_CITY.to_string()));

    // Change the new address's City
    new_address.set_city(DEEP_CHAIN_SECOND_REPLACEMENT_CITY.to_string());

    assert!(values.borrow().contains(&DEEP_CHAIN_SECOND_REPLACEMENT_CITY.to_string()));
}

#[test]
fn deep_chain_intermediate_object_replacement_unsubscribes_old() {
    let vm = BigViewModel::new();
    vm.address().set_street("First Ave".to_string());

    let values = collect_items(&when_changed_scenarios::deep_chain_address_street(&vm));

    let old_address = vm.address();

    // Replace the Address object
    let new_address = Address::new();
    new_address.set_street("Second Ave".to_string());
    vm.set_address(new_address);

    let count_after_replacement = values.borrow().len();

    // Change the old address; it should NOT emit
    old_address.set_street("Third Ave".to_string());

    assert_eq!(values.borrow().len(), count_after_replacement);
    assert_eq!(values.borrow().last().map(String::as_str), Some("Second Ave"));
}

#[test]
fn single_property_distinct_until_changed_filters_duplicates() {
    let vm = named("A");

    let values = collect_items(&when_changed_scenarios::single_property_name(&vm));

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], "A");

    // Setting to the same value: TestViewModel's setter checks inequality,
    // so PropertyChanged won't fire. This verifies no extra emission.
    vm.set_name("A".to_string());

    assert_eq!(values.borrow().len(), 1);
}

#[test]
fn deep_chain_distinct_until_changed_filters_duplicates() {
    let vm = BigViewModel::new();
    vm.address().set_city(DEEP_CHAIN_INITIAL_CITY.to_string());

    let values = collect_items(&when_changed_scenarios::deep_chain_address_city(&vm));

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], DEEP_CHAIN_INITIAL_CITY);

    // Replace Address with one that has the same City.
    vm.set_address(address_in(DEEP_CHAIN_INITIAL_CITY));

    // Deviation (DEVIATIONS.md, ReactiveUI): upstream's generated code drops the repeat (one
    // value); the port observes as upstream's run-time observation (`ExpressionChainSink`), whose
    // re-attached step reports the leaf of the new object even when it is equal, so the city
    // comes again once.
    assert_eq!(*values.borrow(), vec![DEEP_CHAIN_INITIAL_CITY.to_string(), DEEP_CHAIN_INITIAL_CITY.to_string()]);
}

#[test]
fn multi_property_with_deep_chain_emits_on_nested_change() {
    let vm = BigViewModel::new();
    vm.set_prop1(FIRST_PROPERTY_VALUE.to_string());
    vm.address().set_city(DEEP_CHAIN_INITIAL_CITY.to_string());

    let values = collect_items(&when_changed_scenarios::multi_property_with_deep_chain(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0].0, DEEP_CHAIN_INITIAL_CITY);
    assert_eq!(values.borrow()[0].1, FIRST_PROPERTY_VALUE);

    // Change the nested property
    vm.address().set_city(DEEP_CHAIN_REPLACEMENT_CITY.to_string());

    assert_eq!(values.borrow().last(), Some(&(DEEP_CHAIN_REPLACEMENT_CITY.to_string(), FIRST_PROPERTY_VALUE.to_string())));
}

#[test]
fn multi_property_with_deep_chain_emits_on_simple_change() {
    let vm = BigViewModel::new();
    vm.set_prop1(FIRST_PROPERTY_VALUE.to_string());
    vm.address().set_city(DEEP_CHAIN_INITIAL_CITY.to_string());

    let values = collect_items(&when_changed_scenarios::multi_property_with_deep_chain(&vm));

    // Change the simple property
    vm.set_prop1("World".to_string());

    assert_eq!(values.borrow().last(), Some(&(DEEP_CHAIN_INITIAL_CITY.to_string(), "World".to_string())));
}

#[test]
fn multi_property_with_deep_chain_intermediate_replacement() {
    let vm = BigViewModel::new();
    vm.set_prop1(FIRST_PROPERTY_VALUE.to_string());
    vm.address().set_city(DEEP_CHAIN_INITIAL_CITY.to_string());

    let values = collect_items(&when_changed_scenarios::multi_property_with_deep_chain(&vm));

    // Replace the intermediate object
    vm.set_address(address_in(DEEP_CHAIN_REPLACEMENT_CITY));

    assert_eq!(values.borrow().last().map(|value| value.0.clone()), Some(DEEP_CHAIN_REPLACEMENT_CITY.to_string()));

    // Change the new object's property
    vm.address().set_city(DEEP_CHAIN_SECOND_REPLACEMENT_CITY.to_string());

    assert_eq!(values.borrow().last().map(|value| value.0.clone()), Some(DEEP_CHAIN_SECOND_REPLACEMENT_CITY.to_string()));
}

#[test]
fn multiple_subscriptions_receive_independent_emissions() {
    let vm = named("Initial");

    let observable = when_changed_scenarios::single_property_name(&vm);

    let values1 = collect_items(&observable);
    let values2 = collect_items(&observable);

    vm.set_name(REPLACEMENT_NAME.to_string());

    assert!(values1.borrow().len() >= MIN_EMISSIONS_AFTER_CHANGE);
    assert!(values2.borrow().len() >= MIN_EMISSIONS_AFTER_CHANGE);
    assert!(values1.borrow().contains(&REPLACEMENT_NAME.to_string()));
    assert!(values2.borrow().contains(&REPLACEMENT_NAME.to_string()));
}

#[test]
fn multiple_subscriptions_disposing_one_does_not_affect_other() {
    let vm = named("Initial");
    let values1 = Rc::new(RefCell::new(Vec::<String>::new()));

    let observable = when_changed_scenarios::single_property_name(&vm);

    let sink = values1.clone();
    let subscription1 = observable.subscribe_next(move |value| sink.borrow_mut().push(value));
    let values2 = collect_items(&observable);

    subscription1.dispose();

    vm.set_name(AFTER_DISPOSAL_VALUE.to_string());

    assert!(!values1.borrow().contains(&AFTER_DISPOSAL_VALUE.to_string()));
    assert!(values2.borrow().contains(&AFTER_DISPOSAL_VALUE.to_string()));
}

#[test]
fn deep_chain_disposal_stops_listening() {
    let vm = BigViewModel::new();
    vm.address().set_city(DEEP_CHAIN_INITIAL_CITY.to_string());
    let values = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = values.clone();
    let subscription = when_changed_scenarios::deep_chain_address_city(&vm).subscribe_next(move |value| sink.borrow_mut().push(value));

    subscription.dispose();

    vm.address().set_city(DEEP_CHAIN_REPLACEMENT_CITY.to_string());

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], DEEP_CHAIN_INITIAL_CITY);
}

#[test]
fn deep_chain_disposal_after_intermediate_replacement() {
    let vm = BigViewModel::new();
    vm.address().set_city(DEEP_CHAIN_INITIAL_CITY.to_string());
    let values = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = values.clone();
    let subscription = when_changed_scenarios::deep_chain_address_city(&vm).subscribe_next(move |value| sink.borrow_mut().push(value));

    vm.set_address(address_in(DEEP_CHAIN_REPLACEMENT_CITY));

    subscription.dispose();

    vm.address().set_city(DEEP_CHAIN_SECOND_REPLACEMENT_CITY.to_string());

    assert!(!values.borrow().contains(&DEEP_CHAIN_SECOND_REPLACEMENT_CITY.to_string()));
}

#[test]
fn single_property_int_type_emits_changes() {
    let vm = TestViewModel::new();
    vm.set_age(INITIAL_AGE);

    let values = collect_items(&when_changed_scenarios::single_property_age(&vm));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0], INITIAL_AGE);

    vm.set_age(REPLACEMENT_AGE);

    assert!(values.borrow().len() >= MIN_EMISSIONS_AFTER_CHANGE);
    assert!(values.borrow().contains(&REPLACEMENT_AGE));
}

#[test]
fn deep_chain_null_forgiving_emits_when_child_set() {
    let host = host_with_child_named(INITIAL_CHILD_NAME);

    let values = collect_items(&when_changed_scenarios::deep_chain_child_name(&host));

    assert!(!values.borrow().is_empty());
    assert_eq!(values.borrow()[0].as_deref(), Some(INITIAL_CHILD_NAME));

    host.child().expect("a child").set_name("Bob".to_string());

    assert!(values.borrow().contains(&Some("Bob".to_string())));
}

#[test]
fn deep_chain_null_forgiving_does_not_emit_when_child_null() {
    let host = HostTestFixture::new();

    let values = collect_items(&when_changed_scenarios::deep_chain_child_name(&host));

    // Upstream produces nothing while there is no child; the port produces `None`.
    assert_eq!(values.borrow().iter().flatten().count(), 0);
    assert_eq!(*values.borrow(), vec![None]);
}

#[test]
fn deep_chain_null_forgiving_re_subscribes_on_replacement() {
    let host = host_with_child_named(INITIAL_CHILD_NAME);

    let values = collect_items(&when_changed_scenarios::deep_chain_child_name(&host));

    // Replace the child entirely
    host.set_child(Some(named("Charlie")));

    assert!(values.borrow().contains(&Some("Charlie".to_string())));

    // Modify the new child
    host.child().expect("a child").set_name("Dave".to_string());

    assert!(values.borrow().contains(&Some("Dave".to_string())));
}

#[test]
fn deep_chain_null_forgiving_null_to_non_null() {
    let host = HostTestFixture::new();

    let values = collect_items(&when_changed_scenarios::deep_chain_child_name(&host));

    // Set child from null to a value
    host.set_child(Some(named("Eve")));

    assert!(values.borrow().contains(&Some("Eve".to_string())));

    // Verify further changes are tracked
    host.child().expect("a child").set_name("Frank".to_string());

    assert!(values.borrow().contains(&Some("Frank".to_string())));
}
