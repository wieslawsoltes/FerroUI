//! `ExpressionChainNotificationTests` (`ReactiveUI.Binding.Tests/WhenAny/ExpressionChainNotificationTests.cs`).
//!
//! What an expression chain does with the signals the notifications of one step can produce: an
//! error, a completion, a change whose sender is gone, and one that arrives after the subscription
//! was disposed. A stub observer stands in for the real ones, so that each signal is delivered on
//! purpose. Upstream calls `SubscribeToExpressionChain` with an expression; the port observes a
//! property path. The state of the locator is per thread, so the registration of the stub does not
//! reach other tests.

use crate::tests::binding::test_models::{ObservedValueFixture, RecordingObserver};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive_testing::{new_error, TestException};
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::{prop, IDisposable, IObservable, IObserver};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The value the fixture starts out holding.
const INITIAL_VALUE: &str = "initial";

/// The changes the chain of the tests produces.
type Change = ObservedChange<Rc<ObservedValueFixture>, String>;

/// The observers of the notifications of one step.
type NotificationObservers = RefCell<Vec<Rc<dyn IObserver<ObservedChange<PropertyOwner, ()>>>>>;

#[test]
fn link_error_is_forwarded_downstream() {
    let context = arrange();

    context.notifier.push_error(new_error("link failed"));

    let error = context.recorder.error.borrow().clone();
    let error = error.expect("the error of the step reached the chain");
    assert!(error.is::<TestException>());
    assert_eq!(error.to_string(), "link failed");
}

#[test]
fn link_completion_leaves_the_chain_running() {
    let context = arrange();

    context.notifier.push_completed();

    assert!(context.recorder.error.borrow().is_none());
    assert!(!context.recorder.is_completed.get());
}

#[test]
fn notification_with_no_sender_yields_the_default_value() {
    let context = arrange();
    let before = context.recorder.values().len();

    // The port reads the value again from the root of the chain on every notification, whatever
    // the sender of the notification is, so the change carries the current value where upstream's
    // carries the default. The value is changed behind the stub first: a notification that repeats
    // the value read when the step was attached is dropped once.
    context.fixture.set_value("changed".to_string());
    context.notifier.push_change(PropertyOwner::default());

    let values = context.recorder.values();
    assert!(values.len() > before);
    assert_eq!(values.last().expect("a change was produced").value, "changed");
    assert!(context.recorder.error.borrow().is_none());
}

#[test]
fn notification_after_dispose_is_ignored() {
    let context = arrange();
    context.subscription.dispose();
    let after = context.recorder.values().len();

    let owner = PropertyOwner {
        object: Some(context.fixture.clone() as Rc<dyn Any>),
        notify: Some(context.fixture.clone() as Rc<dyn ferroui_system::component_model::INotifyPropertyChanged>),
    };
    context.notifier.push_change(owner);

    assert_eq!(context.recorder.values().len(), after);
    assert!(context.recorder.error.borrow().is_none());
}

/// Builds a chain over the fixture whose only step is fed by the stub observer.
fn arrange() -> ChainContext {
    let notifier = Rc::new(StubNotifier::default());

    register_observation_defaults(&*AppLocator::current_mutable());
    AppLocator::register_constant::<dyn ICreatesObservableForProperty>(notifier.clone() as Rc<dyn ICreatesObservableForProperty>);

    let fixture = ObservedValueFixture::new();
    fixture.set_value(INITIAL_VALUE.to_string());
    let recorder: Rc<RecordingObserver<Change>> = RecordingObserver::new();

    let subscription = fixture
        .subscribe_to_expression_chain(prop!(ObservedValueFixture => value), false, false, false)
        .subscribe(recorder.clone());

    ChainContext { notifier, fixture, recorder, subscription }
}

/// Stands in for a platform observer, handing back a sequence the test drives by hand. It claims
/// only [`ObservedValueFixture`], so that its registration cannot affect anything else.
#[derive(Default)]
struct StubNotifier {
    /// The subscribers of the chain to the notifications of the step.
    observers: Rc<NotificationObservers>,
}

/// An affinity no real observer bids, so that the stub is always chosen.
const WINNING_AFFINITY: i32 = 1000;

impl StubNotifier {
    /// Delivers a change notification with the given sender to the chain.
    fn push_change(&self, sender: PropertyOwner) {
        let change = ObservedChange::new(sender, "Value", ());
        let observers = self.observers.borrow().clone();
        for observer in observers {
            observer.on_next(change.clone());
        }
    }

    /// Faults the notifications of the step.
    fn push_error(&self, error: ferroui_system::ObservableError) {
        let observers = self.observers.borrow().clone();
        for observer in observers {
            observer.on_error(error.clone());
        }
    }

    /// Completes the notifications of the step.
    fn push_completed(&self) {
        let observers = self.observers.borrow().clone();
        for observer in observers {
            observer.on_completed();
        }
    }
}

impl ICreatesObservableForProperty for StubNotifier {
    fn get_affinity_for_object(&self, owner: &PropertyOwner, _property_name: &str, _before_changed: bool) -> i32 {
        match &owner.object {
            Some(object) if (**object).is::<ObservedValueFixture>() => WINNING_AFFINITY,
            _ => 0,
        }
    }

    fn get_notification_for_property(
        &self,
        _owner: &PropertyOwner,
        _property_name: &str,
        _before_changed: bool,
        _suppress_warnings: bool,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
        Rc::new(Stream { observers: self.observers.clone() })
    }
}

/// The notifications handed to one step of the chain. Ending the subscription deliberately keeps
/// the subscriber, so that a test can deliver a notification that was already in flight when the
/// chain was disposed.
struct Stream {
    observers: Rc<NotificationObservers>,
}

impl IObservable<ObservedChange<PropertyOwner, ()>> for Stream {
    fn subscribe(&self, observer: Rc<dyn IObserver<ObservedChange<PropertyOwner, ()>>>) -> Rc<dyn IDisposable> {
        self.observers.borrow_mut().push(observer);
        // Unhooks the step without silencing the stream.
        Disposable::empty()
    }
}

/// An arranged chain and everything needed to drive and inspect it.
struct ChainContext {
    /// The stub that feeds the only step of the chain.
    notifier: Rc<StubNotifier>,
    /// The observed object.
    fixture: Rc<ObservedValueFixture>,
    /// What the chain produced.
    recorder: Rc<RecordingObserver<Change>>,
    /// The subscription to the chain.
    subscription: Rc<dyn IDisposable>,
}

impl Drop for ChainContext {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}
