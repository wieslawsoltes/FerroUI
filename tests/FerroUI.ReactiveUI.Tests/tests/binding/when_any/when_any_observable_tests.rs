//! `WhenAnyObservableTests` (`ReactiveUI.Binding.Tests/WhenAny/WhenAnyObservableTests.cs`).
//!
//! Upstream calls `WhenAnyObservableUnsafe`, the run-time form; the port has one form,
//! `when_any_observable`, over property paths (a tuple of paths is merged). Upstream's
//! `EnsureInitialized` builds the application with the core services; the port observes with the
//! default observers while none is registered, so the tests need no setup.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::TestWhenAnyObsViewModel;
use ferroui_reactive::subjects::Subject;
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable, IObserver};
use std::rc::Rc;

/// A sample value emitted by a single observable.
const SAMPLE_VALUE: i32 = 42;

/// The second value emitted when merging two observables.
const SECOND_VALUE: i32 = 2;

/// The expected number of merged emissions from two observables.
const EXPECTED_TWO_EMISSIONS: usize = 2;

/// The first value emitted in the three-observable combining test.
const FIRST_COMBINE_VALUE: i32 = 10;

/// The second value emitted in the three-observable combining test.
const SECOND_COMBINE_VALUE: i32 = 20;

/// The third value emitted in the three-observable combining test.
const THIRD_COMBINE_VALUE: i32 = 30;

/// The expected number of emissions when combining three observables.
const EXPECTED_THREE_EMISSIONS: usize = 3;

/// A value emitted by a late-assigned observable.
const LATE_ASSIGNED_VALUE: i32 = 99;

/// The subject as the value of an observable property.
fn held(subject: &Rc<Subject<i32>>) -> Option<Rc<dyn IObservable<i32>>> {
    Some(subject.clone() as Rc<dyn IObservable<i32>>)
}

#[test]
fn null_observables_do_not_cause_exceptions() {
    let vm = TestWhenAnyObsViewModel::new();

    // `Command1` is null; this does not panic.
    let values = collect_items(&vm.when_any_observable(prop!(TestWhenAnyObsViewModel => command1)));

    // Set it to a real observable.
    let subject = Subject::<i32>::new();
    vm.set_command1(held(&subject));

    subject.on_next(SAMPLE_VALUE);

    assert!(values.borrow().len() >= 1);
    assert_eq!(values.borrow()[0], SAMPLE_VALUE);
}

#[test]
fn smoke_test_merging() {
    let subject1 = Subject::<i32>::new();
    let subject2 = Subject::<i32>::new();
    let vm = TestWhenAnyObsViewModel::new();
    vm.set_command1(held(&subject1));
    vm.set_command2(held(&subject2));

    let values = collect_items(&vm.when_any_observable((prop!(TestWhenAnyObsViewModel => command1), prop!(TestWhenAnyObsViewModel => command2))));

    subject1.on_next(1);
    subject2.on_next(SECOND_VALUE);

    assert_eq!(values.borrow().len(), EXPECTED_TWO_EMISSIONS);
    assert!(values.borrow().contains(&1));
    assert!(values.borrow().contains(&SECOND_VALUE));
}

#[test]
fn smoke_test_combining() {
    let subject1 = Subject::<i32>::new();
    let subject2 = Subject::<i32>::new();
    let subject3 = Subject::<i32>::new();
    let vm = TestWhenAnyObsViewModel::new();
    vm.set_command1(held(&subject1));
    vm.set_command2(held(&subject2));
    vm.set_command3(held(&subject3));

    let values = collect_items(&vm.when_any_observable((
        prop!(TestWhenAnyObsViewModel => command1),
        prop!(TestWhenAnyObsViewModel => command2),
        prop!(TestWhenAnyObsViewModel => command3),
    )));

    subject1.on_next(FIRST_COMBINE_VALUE);
    subject2.on_next(SECOND_COMBINE_VALUE);
    subject3.on_next(THIRD_COMBINE_VALUE);

    assert_eq!(values.borrow().len(), EXPECTED_THREE_EMISSIONS);
}

#[test]
fn null_object_updates_when_not_null_anymore() {
    let vm = TestWhenAnyObsViewModel::new();

    let values = collect_items(&vm.when_any_observable(prop!(TestWhenAnyObsViewModel => changes)));

    // Initially null: nothing is produced.
    assert_eq!(values.borrow().len(), 0);

    // Now set a real observable.
    let subject = Subject::<i32>::new();
    vm.set_changes(held(&subject));

    subject.on_next(LATE_ASSIGNED_VALUE);

    assert_eq!(values.borrow().len(), 1);
    assert_eq!(values.borrow()[0], LATE_ASSIGNED_VALUE);
}
