//! `AppliedChangeObservableTests` (`ReactiveUI.Binding.Tests/Observables/AppliedChangeObservableTests.cs`).

use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

const FIRST_VALUE: &str = "first";

const SECOND_VALUE: &str = "second";

/// Records the text of each change it receives (`RecordingObserver`).
#[derive(Default)]
struct RecordingObserver {
    received: RefCell<Vec<String>>,
}

impl RecordingObserver {
    fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    fn received(&self) -> Vec<String> {
        self.received.borrow().clone()
    }
}

impl IObserver<BindingChange> for RecordingObserver {
    fn on_next(&self, value: BindingChange) {
        self.received.borrow_mut().push(value.value_as::<String>().cloned().unwrap_or_default());
    }
}

/// Runs an action on each change (`DelegateObserver`).
struct DelegateObserver(Box<dyn Fn()>);

impl IObserver<BindingChange> for DelegateObserver {
    fn on_next(&self, _value: BindingChange) {
        (self.0)();
    }
}

fn change(value: &str, from_view_model: bool) -> BindingChange {
    BindingChange::new(value.to_string(), from_view_model)
}

#[test]
fn has_observers_tracks_subscription_lifetime() {
    let changes = AppliedChangeObservable::new();
    assert!(!changes.has_observers());
    let subscription = changes.subscribe(RecordingObserver::new());
    assert!(changes.has_observers());
    subscription.dispose();
    assert!(!changes.has_observers());
}

#[test]
fn on_next_with_no_observers_reports_to_nobody_and_does_not_throw() {
    let changes = AppliedChangeObservable::new();

    changes.on_next(change(FIRST_VALUE, true));
}

#[test]
fn on_next_with_two_observers_reports_to_both() {
    let changes = AppliedChangeObservable::new();
    let first = RecordingObserver::new();
    let second = RecordingObserver::new();

    let first_subscription = changes.subscribe(first.clone());
    let second_subscription = changes.subscribe(second.clone());
    changes.on_next(change(FIRST_VALUE, true));
    second_subscription.dispose();
    first_subscription.dispose();

    assert_eq!(first.received(), [FIRST_VALUE]);
    assert_eq!(second.received(), [FIRST_VALUE]);
}

#[test]
fn dispose_of_one_subscription_leaves_the_others_reporting() {
    let changes = AppliedChangeObservable::new();
    let dropped = RecordingObserver::new();
    let kept = RecordingObserver::new();

    let dropped_subscription = changes.subscribe(dropped.clone());
    let kept_subscription = changes.subscribe(kept.clone());

    changes.on_next(change(FIRST_VALUE, true));
    dropped_subscription.dispose();
    changes.on_next(change(SECOND_VALUE, false));
    kept_subscription.dispose();

    assert_eq!(dropped.received(), [FIRST_VALUE]);
    assert_eq!(kept.received(), [FIRST_VALUE, SECOND_VALUE]);
}

#[test]
fn dispose_called_twice_drops_one_place() {
    let changes = AppliedChangeObservable::new();
    let kept = RecordingObserver::new();

    let dropped_subscription = changes.subscribe(RecordingObserver::new());
    let kept_subscription = changes.subscribe(kept.clone());

    dropped_subscription.dispose();
    dropped_subscription.dispose();
    changes.on_next(change(FIRST_VALUE, true));
    kept_subscription.dispose();

    assert_eq!(kept.received(), [FIRST_VALUE]);
}

#[test]
fn subscribe_during_delivery_joins_the_next_change() {
    let changes = AppliedChangeObservable::new();
    let joining = RecordingObserver::new();
    let subscriptions: Rc<RefCell<Vec<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(Vec::new()));

    let (source, observer, added) = (changes.clone(), joining.clone(), subscriptions.clone());
    let first = changes.subscribe(Rc::new(DelegateObserver(Box::new(move || {
        let subscription = source.subscribe(observer.clone());
        added.borrow_mut().push(subscription);
    }))));

    changes.on_next(change(FIRST_VALUE, true));
    changes.on_next(change(SECOND_VALUE, false));

    let taken: Vec<Rc<dyn IDisposable>> = subscriptions.borrow_mut().drain(..).collect();
    for subscription in taken {
        subscription.dispose();
    }
    first.dispose();

    assert_eq!(joining.received(), [SECOND_VALUE]);
}

#[test]
fn remove_of_an_observer_that_never_subscribed_leaves_the_others_reporting() {
    let changes = AppliedChangeObservable::new();
    let kept = RecordingObserver::new();

    let kept_subscription = changes.subscribe(kept.clone());

    let stranger: Rc<dyn IObserver<BindingChange>> = RecordingObserver::new();
    changes.remove(&stranger);
    changes.on_next(change(FIRST_VALUE, true));
    kept_subscription.dispose();

    assert_eq!(kept.received(), [FIRST_VALUE]);
}

// Not ported: Subscribe_WithNoObserver_ThrowsArgumentNullException - argument validation
