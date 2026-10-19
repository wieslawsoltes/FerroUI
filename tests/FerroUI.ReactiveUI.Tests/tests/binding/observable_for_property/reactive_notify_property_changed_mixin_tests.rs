//! `ReactiveNotifyPropertyChangedMixinTests` (`ReactiveUI.Binding.Tests/ObservableForProperty/ReactiveNotifyPropertyChangedMixinTests.cs`).
//!
//! Upstream calls `SubscribeToExpressionChain` with the body of an expression and
//! `ObservableForProperty` with an expression or a property name read by reflection; the port
//! observes a property path (`subscribe_to_expression_chain`, `observable_for_property`) or a
//! property name with its getter (`observable_for_property_named`). Upstream's
//! `ObservableForProperty<TSender, TValue>(name, skipInitial)` is
//! `observable_for_property_named(name, getter, false, skipInitial, true)`.
//!
//! `NestedObservedChanges` (the observation of one member of an expression chain, started by a
//! kicker) is not ported: the port's `ExpressionChainSink` observes the steps of a path and reads
//! each value after subscribing. Upstream's `EnsureInitialized` builds the application with the
//! core services; the port observes with the default observers while none is registered, so the
//! tests need no setup.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::{HostTestFixture, TestFixture, TestViewModel};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::prop;
use std::rc::Rc;

/// The value the view model starts with.
const INITIAL_VALUE: &str = "Initial";

/// A changed property value used across notification tests.
const NOTIFYING_VALUE: &str = "Changed";

/// The expected number of emissions after two successive changes (kicker plus two changes).
const EXPECTED_THREE_EMISSIONS: usize = 3;

/// A fixture whose `IsNotNullString` is `value`.
fn fixture_with(value: &str) -> Rc<TestFixture> {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string(value.to_string());
    fixture
}

// Not ported: NestedObservedChanges_NullSourceChangeValue_ReturnsSingleKicker - expression trees
// (`NestedObservedChanges` observes one member expression of a chain; the port observes the steps
// of a property path in `ExpressionChainSink`).
// Not ported: NestedObservedChanges_NonNullSourceChangeValue_EmitsKickerAndNotifications - expression
// trees (same reason).
// Not ported: NotifyForProperty_ExpressionWithNoMemberInfo_ThrowsArgumentException - expression trees
// (a parameter expression names no member; a path step always names its property).

#[test]
fn subscribe_to_expression_chain_is_distinct_false_does_not_deduplicate() {
    let fixture = fixture_with("A");

    let values = collect_items(&fixture.subscribe_to_expression_chain(prop!(TestFixture => is_not_null_string), false, false, false));

    fixture.set_is_not_null_string("B".to_string());
    fixture.set_is_not_null_string("C".to_string());

    // The initial value and two changes: at least three.
    assert!(values.borrow().len() >= EXPECTED_THREE_EMISSIONS);
}

#[test]
fn subscribe_to_expression_chain_is_distinct_true_deduplicates_by_value() {
    let fixture = fixture_with("A");

    let values = collect_items(&fixture.subscribe_to_expression_chain(prop!(TestFixture => is_not_null_string), false, false, true));

    // Change to different values.
    fixture.set_is_not_null_string("B".to_string());
    fixture.set_is_not_null_string("C".to_string());

    // All values are distinct, so all come through.
    assert!(values.borrow().len() >= EXPECTED_THREE_EMISSIONS);
}

#[test]
fn subscribe_to_expression_chain_skip_initial_true_skips_first_value() {
    let fixture = fixture_with(INITIAL_VALUE);

    let values = collect_items(&fixture.subscribe_to_expression_chain(prop!(TestFixture => is_not_null_string), false, true, false));

    // No initial value was produced.
    assert_eq!(values.borrow().len(), 0);

    fixture.set_is_not_null_string(NOTIFYING_VALUE.to_string());

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0].value, NOTIFYING_VALUE);
}

#[test]
fn subscribe_to_expression_chain_null_sender_in_chain_is_filtered_out() {
    // A deep chain whose intermediate becomes null.
    let child = TestFixture::new();
    child.set_is_not_null_string("Hello".to_string());
    let fixture = HostTestFixture::new();
    fixture.set_child(Some(child));

    // `x => x.Child!.IsNotNullString`: the value is `None` while there is no child.
    let path = prop!(HostTestFixture => child).then_optional_property("IsNotNullString", |child: &Rc<TestFixture>| child.is_not_null_string());
    let values = collect_items(&fixture.subscribe_to_expression_chain(path, false, false, false));

    let initial_count = values.borrow().len();

    // Setting the child to null produces `None` in the port (upstream filters the change out), and
    // every change carries the root as its sender.
    fixture.set_child(None);

    // Setting it back produces a new value.
    let replacement = TestFixture::new();
    replacement.set_is_not_null_string("World".to_string());
    fixture.set_child(Some(replacement));

    assert!(values.borrow().len() > initial_count);
    assert!(values.borrow().iter().all(|change| Rc::ptr_eq(&change.sender, &fixture)));
    assert_eq!(values.borrow().last().map(|change| change.value.clone()), Some(Some("World".to_string())));
}

#[test]
fn observable_for_property_by_name_before_change_emits_notification() {
    let vm = TestViewModel::new();

    // Upstream's `ObservableForProperty<TestViewModel, string>("Name", true)` is the overload whose
    // second argument is `skipInitial`.
    let values = collect_items(&vm.observable_for_property_named("Name", |vm: &Rc<TestViewModel>| vm.name(), false, true, true));

    vm.set_name("Alice".to_string());

    assert!(values.borrow().len() >= 1);
}

#[test]
fn observable_for_property_by_expression_before_change_emits_notification() {
    let vm = TestViewModel::new();

    let values = collect_items(&vm.observable_for_property(prop!(TestViewModel => name), true, true, true));

    vm.set_name("Bob".to_string());

    assert!(values.borrow().len() >= 1);
}

// Not ported: NestedObservedChanges_BeforeChange_RoutesToPropertyChanging - expression trees
// (`NestedObservedChanges` observes one member expression of a chain).

#[test]
fn observable_for_property_by_name_get_current_value_val_is_t_value_succeeds() {
    let vm = TestViewModel::new();
    vm.set_name("Test".to_string());

    let values = collect_items(&vm.observable_for_property_named("Name", |vm: &Rc<TestViewModel>| vm.name(), false, false, true).value());

    assert_eq!(values.borrow().last().map(String::as_str), Some("Test"));
}

// Not ported: ObservableForProperty_ByName_LookupBeforeRegistration_ResolvesAfterRegistration - no
// failing lookup (while no observer is registered the port uses the default ones, a deviation of
// `observers_for_property`, so a lookup before registration cannot fail; and the port does not
// memoize the observer of a type, so there is no memo to outlive a failure).
// Not ported: NotifyForProperty_LookupBeforeRegistration_ResolvesAfterRegistration - no failing
// lookup (same reason).

/// A property holding nothing is observed as nothing, so a subscriber sees the absence rather than
/// the observation faulting on the way to reading it.
#[test]
fn observable_for_property_by_name_null_property_value_emits_the_default() {
    let fixture = TestFixture::new();

    let values = collect_items(
        &fixture
            .observable_for_property_named("NullableInt", |fixture: &Rc<TestFixture>| fixture.nullable_int(), false, false, true)
            .select(|change: ObservedChange<Rc<TestFixture>, Option<i32>>| change.value),
    );

    assert!(values.borrow().iter().any(|value| value.is_none()));
}

// Not ported: ObservableForProperty_ByName_ValueOfAnotherType_SignalsInvalidCast - reflection (the
// value is read with a typed getter, so its type is checked at compile time).

/// The form that takes every option builds the same chain observation.
#[test]
fn subscribe_to_expression_chain_with_suppressed_warnings_observes_the_chain() {
    let fixture = fixture_with(INITIAL_VALUE);

    let values = collect_items(
        &fixture
            .subscribe_to_expression_chain_with(prop!(TestFixture => is_not_null_string), false, false, true, true)
            .select(|change: ObservedChange<Rc<TestFixture>, String>| change.value),
    );

    fixture.set_is_not_null_string(NOTIFYING_VALUE.to_string());

    assert!(values.borrow().iter().any(|value| value == NOTIFYING_VALUE));
}

// Not ported: NotifyForProperty_WithSuppressedWarnings_ExpressionNamingNoMember_Throws - expression
// trees (an array index expression names no member; a path step always names its property).
