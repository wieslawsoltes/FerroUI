//! `ExpressionChainTests` (`ReactiveUI.Binding.Tests/WhenAny/ExpressionChainTests.cs`).
//!
//! Upstream calls `SubscribeToExpressionChain` with the body of an expression; the port observes a
//! property path with `subscribe_to_expression_chain`. Upstream's `EnsureInitialized` builds the
//! application with the core services; the port observes with the default observers while none is
//! registered, so the tests need no setup.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::{HostTestFixture, PocoChainRoot, PocoModel, TestFixture};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::prop;
use std::rc::Rc;

/// The expected number of emitted values when two notifications are produced.
const EXPECTED_TWO_EMISSIONS: usize = 2;

/// The value a fixture starts out holding.
const START_VALUE: &str = "Start";

/// The value a before-change fixture starts out holding.
const INITIAL_VALUE: &str = "Before";

/// The value the first change moves to, which the second change then leaves behind.
const REPLACEMENT_VALUE: &str = "After";

#[test]
fn basic_usage_notifies_on_change() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string(START_VALUE.to_string());

    let values = collect_items(
        &fixture
            .subscribe_to_expression_chain(prop!(TestFixture => is_not_null_string), false, false, true)
            .select(|change: ObservedChange<Rc<TestFixture>, String>| change.value),
    );

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], START_VALUE);

    fixture.set_is_not_null_string("End".to_string());

    assert!(values.borrow().len() >= EXPECTED_TWO_EMISSIONS);
    assert_eq!(values.borrow()[1], "End");
}

/// A before-change chain reports the value a property is about to leave behind. The first change
/// leaves the value the initial emission already carried, and a repeat that immediately follows
/// the initial read is dropped. The second change leaves a value nobody has seen, and is reported.
#[test]
fn with_before_change_reports_the_value_each_change_leaves_behind() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string(INITIAL_VALUE.to_string());

    let values = collect_items(
        &fixture
            .subscribe_to_expression_chain(prop!(TestFixture => is_not_null_string), true, false, false)
            .select(|change: ObservedChange<Rc<TestFixture>, String>| change.value),
    );

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], INITIAL_VALUE);

    fixture.set_is_not_null_string(REPLACEMENT_VALUE.to_string());
    fixture.set_is_not_null_string("Third".to_string());

    assert!(values.borrow().len() >= EXPECTED_TWO_EMISSIONS);
    assert_eq!(values.borrow()[1], REPLACEMENT_VALUE);
}

#[test]
fn with_skip_initial_skips_first_emission() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string("Initial".to_string());

    // `SubscribeToExpressionChain(expression)`: after the change, skipping the initial value,
    // dropping repeats.
    let values = collect_items(
        &fixture
            .subscribe_to_expression_chain(prop!(TestFixture => is_not_null_string), false, true, true)
            .select(|change: ObservedChange<Rc<TestFixture>, String>| change.value),
    );

    // Should NOT have emitted the initial value.
    assert_eq!(values.borrow().len(), 0);

    fixture.set_is_not_null_string("Changed".to_string());

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], "Changed");
}

#[test]
fn with_is_distinct_deduplicates_same_values() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string("Same".to_string());

    let values = collect_items(
        &fixture
            .subscribe_to_expression_chain(prop!(TestFixture => is_not_null_string), false, true, true)
            .select(|change: ObservedChange<Rc<TestFixture>, String>| change.value),
    );

    fixture.set_is_not_null_string("A".to_string());
    fixture.set_is_not_null_string("B".to_string());
    fixture.set_is_not_null_string("B".to_string()); // Same value: the setter short-circuits.

    assert_eq!(values.borrow().len(), EXPECTED_TWO_EMISSIONS);
    assert_eq!(values.borrow()[0], "A");
    assert_eq!(values.borrow()[1], "B");
}

/// A step with no change notification produces once on subscription. The chain reads the same
/// value straight after, so it reports the initial value once, and the notifying leaf behind it
/// still reports its changes.
#[test]
fn poco_link_before_notifying_leaf_emits_the_initial_value_once() {
    let root = PocoChainRoot::new();
    root.leaf().set_is_not_null_string(START_VALUE.to_string());

    // `x => x.Leaf.IsNotNullString`: `Leaf` is a plain value, `IsNotNullString` notifies.
    let path = PropertyPath::<Rc<PocoChainRoot>, Rc<TestFixture>>::value("Leaf", |root: &Rc<PocoChainRoot>| root.leaf())
        .then_property("IsNotNullString", |leaf: &Rc<TestFixture>| leaf.is_not_null_string());
    let values = collect_items(
        &root
            .subscribe_to_expression_chain(path, false, false, false)
            .select(|change: ObservedChange<Rc<PocoChainRoot>, String>| change.value),
    );
    let initial = values.borrow().clone();

    root.leaf().set_is_not_null_string(REPLACEMENT_VALUE.to_string());

    assert_eq!(initial, vec![START_VALUE.to_string()]);
    assert_eq!(*values.borrow(), vec![START_VALUE.to_string(), REPLACEMENT_VALUE.to_string()]);
}

/// A leaf with no change notification reports its value once, and skipping the initial value
/// leaves nothing.
#[test]
fn poco_leaf_reports_the_initial_value_at_most_once() {
    for (skip_initial, expected_count) in [(false, 1usize), (true, 0usize)] {
        let fixture = PocoModel::new();
        fixture.set_value(START_VALUE.to_string());

        let path = PropertyPath::<Rc<PocoModel>, String>::value("Value", |model: &Rc<PocoModel>| model.value());
        let values = collect_items(
            &fixture
                .subscribe_to_expression_chain_with(path, false, skip_initial, false, true)
                .select(|change: ObservedChange<Rc<PocoModel>, String>| change.value),
        );

        assert_eq!(values.borrow().len(), expected_count, "skip_initial = {skip_initial}");
    }
}

#[test]
fn null_in_chain_propagates_correctly() {
    let fixture = HostTestFixture::new();

    // `x => x.Child!.IsNotNullString`; `Child` is absent at first. The port produces `None` while
    // there is no child, where upstream produces nothing.
    let path = prop!(HostTestFixture => child).then_optional_property("IsNotNullString", |child: &Rc<TestFixture>| child.is_not_null_string());
    let values = collect_items(&fixture.subscribe_to_expression_chain(path, false, false, false));

    // Set the child to something.
    let child = TestFixture::new();
    child.set_is_not_null_string("Hello".to_string());
    fixture.set_child(Some(child));

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow().last().map(|change| change.value.clone()), Some(Some("Hello".to_string())));
}

// Not ported: IdentityExpression_HasNoLinksAndNeverEmits - expression trees (`x => x` has no member
// access; a property path always has at least one step).
// Not ported: LeafValueOfAnotherType_SignalsInvalidCast - expression trees (the type of the value of
// a property path is checked at compile time).
// Not ported: NullRoot_ProducesNoValues - argument validation (the root is an `Rc`, which cannot be
// null).
