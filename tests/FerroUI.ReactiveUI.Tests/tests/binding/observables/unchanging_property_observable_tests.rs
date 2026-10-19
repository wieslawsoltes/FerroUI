//! `UnchangingPropertyObservableTests`
//! (`ReactiveUI.Binding.Tests/Observables/UnchangingPropertyObservableTests.cs`).

use crate::tests::binding::test_models::RecordingObserver;
use ferroui_reactiveui::*;
use ferroui_system::{IObservable, IObserver};
use std::rc::Rc;

const OBSERVED_VALUE: &str = "only";

fn expected_emissions() -> Vec<String> {
    vec![OBSERVED_VALUE.to_string()]
}

#[test]
fn subscribe_emits_the_value_it_was_built_with() {
    let observer = RecordingObserver::<String>::new();
    let observable = UnchangingPropertyObservable::new(OBSERVED_VALUE.to_string());

    let subscription = observable.subscribe(observer.clone() as Rc<dyn IObserver<String>>);

    assert_eq!(observer.values(), expected_emissions());
    subscription.dispose();
}

#[test]
fn subscribe_does_not_complete() {
    let observer = RecordingObserver::<String>::new();
    let observable = UnchangingPropertyObservable::new(OBSERVED_VALUE.to_string());

    let subscription = observable.subscribe(observer.clone() as Rc<dyn IObserver<String>>);

    assert!(!observer.is_completed.get());
    subscription.dispose();
}

#[test]
fn subscribe_second_subscriber_also_receives_the_value() {
    let first = RecordingObserver::<String>::new();
    let second = RecordingObserver::<String>::new();
    let observable = UnchangingPropertyObservable::new(OBSERVED_VALUE.to_string());

    let first_subscription = observable.subscribe(first.clone() as Rc<dyn IObserver<String>>);
    let second_subscription = observable.subscribe(second.clone() as Rc<dyn IObserver<String>>);

    assert_eq!(first.values(), expected_emissions());
    assert_eq!(second.values(), expected_emissions());
    second_subscription.dispose();
    first_subscription.dispose();
}

#[test]
fn dispose_called_again_does_not_throw() {
    let observable = UnchangingPropertyObservable::new(1);
    let subscription = observable.subscribe(RecordingObserver::<i32>::new() as Rc<dyn IObserver<i32>>);

    subscription.dispose();
    subscription.dispose();
}

// Not ported: Subscribe_NullObserver_Throws - argument validation
