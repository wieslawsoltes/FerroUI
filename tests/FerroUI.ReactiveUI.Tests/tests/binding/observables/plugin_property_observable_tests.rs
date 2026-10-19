//! `PluginPropertyObservableTests`
//! (`ReactiveUI.Binding.Tests/Observables/PluginPropertyObservableTests.cs`).
//!
//! Upstream passes the source object and reads it back in the getter; the port's getter is a
//! closure over the object (`Fn() -> Option<T>`). The member expression has no counterpart: a
//! step is named by its property name. A getter that throws is a getter that panics; the tests
//! catch the panic.

use crate::tests::binding::test_models::RecordingObserver;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::Observer;
use ferroui_reactiveui::*;
use ferroui_reactive_testing::new_error;
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyOwner};
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

const OBSERVED_PROPERTY_NAME: &str = "Name";
const INITIAL_NAME: &str = "initial";
const UPDATED_NAME: &str = "updated";
const COMPLETION_MARKER: &str = "complete";
const ERROR_MARKER: &str = "error";

/// A notification of the registration.
type Change = ObservedChange<PropertyOwner, ()>;

fn initial_only() -> Vec<String> {
    vec![INITIAL_NAME.to_string()]
}

fn initial_then_updated() -> Vec<String> {
    vec![INITIAL_NAME.to_string(), UPDATED_NAME.to_string()]
}

/// The observable of `Name` of `fixture` through `notifications` (`Build`).
fn build(fixture: &Rc<ObservedFixture>, notifications: &Rc<PluginNotifications>, distinct: bool) -> PluginPropertyObservable<String> {
    let reader = Rc::downgrade(fixture);
    PluginPropertyObservable::new(
        notifications.clone() as Rc<dyn ICreatesObservableForProperty>,
        &ObservedFixture::owner(fixture),
        OBSERVED_PROPERTY_NAME,
        move || reader.upgrade().map(|fixture| fixture.name()),
        false,
        distinct,
    )
}

/// An observer that adds each value, and a marker for the error and the completion, to the
/// returned list.
fn sequence_observer() -> (Rc<RefCell<Vec<String>>>, Rc<dyn IObserver<String>>) {
    let sequence: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let (values, errors, completions) = (sequence.clone(), sequence.clone(), sequence.clone());
    let observer = Observer::create_all(
        move |value: String| values.borrow_mut().push(value),
        move |_| errors.borrow_mut().push(ERROR_MARKER.to_string()),
        move || completions.borrow_mut().push(COMPLETION_MARKER.to_string()),
    );
    (sequence, observer)
}

// Not ported: OnNext_WaitsForCompetingWriter_DeliversLatestValueWithoutOverlap - real threads

#[test]
fn subscribe_terminal_during_attachment_delivers_initial_value_before_terminal() {
    const EXPECTED_NOTIFICATIONS: usize = 2;
    for fault in [false, true] {
        let fixture = ObservedFixture::new(INITIAL_NAME);
        let notifications = PluginNotifications::new();
        let weak_notifications = Rc::downgrade(&notifications);
        notifications.set_after_subscribe(move || {
            let Some(notifications) = weak_notifications.upgrade() else {
                return;
            };
            if fault {
                notifications.fault(new_error("provider error"));
            }
            notifications.complete();
            notifications.fault(new_error("provider error"));
        });
        let (sequence, observer) = sequence_observer();

        let subscription = build(&fixture, &notifications, true).subscribe(observer);
        fixture.set_name(UPDATED_NAME);
        notifications.raise(&ObservedFixture::owner(&fixture));

        let sequence = sequence.borrow().clone();
        assert_eq!(sequence.len(), EXPECTED_NOTIFICATIONS);
        assert_eq!(sequence[0], INITIAL_NAME);
        assert_eq!(sequence[1], if fault { ERROR_MARKER } else { COMPLETION_MARKER });
        subscription.dispose();
    }
}

/// One row of `OnNext_ReentrantChangeAndTerminal_DeliversPendingValueBeforeTerminal`.
fn reentrant_change_and_terminal(fault: bool, same_value: bool, distinct: bool) {
    const VALUE_AND_TERMINAL_COUNT: usize = 2;
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let sequence: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let inside_observer = Rc::new(Cell::new(false));
    let completion_overlapped = Rc::new(Cell::new(false));

    let (values, inside, weak_fixture, weak_notifications) =
        (sequence.clone(), inside_observer.clone(), Rc::downgrade(&fixture), Rc::downgrade(&notifications));
    let (errors, error_inside, error_overlapped) = (sequence.clone(), inside_observer.clone(), completion_overlapped.clone());
    let (completions, completion_inside, completed_overlapped) = (sequence.clone(), inside_observer.clone(), completion_overlapped.clone());
    let observer = Observer::create_all(
        move |value: String| {
            inside.set(true);
            let count = {
                let mut values = values.borrow_mut();
                values.push(value);
                values.len()
            };
            if count == 1 {
                if let (Some(fixture), Some(notifications)) = (weak_fixture.upgrade(), weak_notifications.upgrade()) {
                    fixture.set_name(if same_value { INITIAL_NAME } else { UPDATED_NAME });
                    notifications.raise(&ObservedFixture::owner(&fixture));
                    if fault {
                        notifications.fault(new_error(ERROR_MARKER));
                    } else {
                        notifications.complete();
                    }
                }
            }
            inside.set(false);
        },
        move |_| {
            error_overlapped.set(error_inside.get());
            errors.borrow_mut().push(ERROR_MARKER.to_string());
        },
        move || {
            completed_overlapped.set(completion_inside.get());
            completions.borrow_mut().push(COMPLETION_MARKER.to_string());
        },
    );
    let subscription = build(&fixture, &notifications, distinct).subscribe(observer);

    let emits_pending = !same_value || !distinct;
    let sequence = sequence.borrow().clone();
    let row = format!("fault: {fault}, same_value: {same_value}, distinct: {distinct}");
    assert_eq!(sequence.len(), VALUE_AND_TERMINAL_COUNT + usize::from(emits_pending), "{row}");
    assert_eq!(sequence[0], INITIAL_NAME, "{row}");
    if emits_pending {
        assert_eq!(sequence[1], if same_value { INITIAL_NAME } else { UPDATED_NAME }, "{row}");
    }
    assert_eq!(sequence[sequence.len() - 1], if fault { ERROR_MARKER } else { COMPLETION_MARKER }, "{row}");
    assert!(!completion_overlapped.get(), "{row}");
    subscription.dispose();
}

#[test]
fn on_next_reentrant_change_and_terminal_delivers_pending_value_before_terminal() {
    for fault in [false, true] {
        for same_value in [false, true] {
            for distinct in [false, true] {
                reentrant_change_and_terminal(fault, same_value, distinct);
            }
        }
    }
}

#[test]
fn on_next_final_getter_disposes_stops_final_value_and_terminal() {
    for fault in [false, true] {
        let fixture = ObservedFixture::new(INITIAL_NAME);
        let notifications = PluginNotifications::new();
        let values: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let terminated = Rc::new(Cell::new(false));
        let dispose_on_read = Rc::new(Cell::new(false));
        let subscription_slot: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));

        let (reader, read_disposes, slot) = (Rc::downgrade(&fixture), dispose_on_read.clone(), subscription_slot.clone());
        let observable = PluginPropertyObservable::new(
            notifications.clone() as Rc<dyn ICreatesObservableForProperty>,
            &ObservedFixture::owner(&fixture),
            OBSERVED_PROPERTY_NAME,
            move || {
                if read_disposes.get() {
                    let subscription = slot.borrow().clone();
                    if let Some(subscription) = subscription {
                        subscription.dispose();
                    }
                }
                reader.upgrade().map(|fixture| fixture.name())
            },
            false,
            true,
        );

        let (sink, weak_fixture, weak_notifications, disposes) =
            (values.clone(), Rc::downgrade(&fixture), Rc::downgrade(&notifications), dispose_on_read.clone());
        let (error_terminated, completion_terminated) = (terminated.clone(), terminated.clone());
        let subscription = observable.subscribe(Observer::create_all(
            move |value: String| {
                let updated = value == UPDATED_NAME;
                sink.borrow_mut().push(value);
                if !updated {
                    return;
                }
                if let (Some(fixture), Some(notifications)) = (weak_fixture.upgrade(), weak_notifications.upgrade()) {
                    fixture.set_name("final");
                    notifications.raise(&ObservedFixture::owner(&fixture));
                    disposes.set(true);
                    if fault {
                        notifications.fault(new_error(ERROR_MARKER));
                    } else {
                        notifications.complete();
                    }
                }
            },
            move |_| error_terminated.set(true),
            move || completion_terminated.set(true),
        ));
        *subscription_slot.borrow_mut() = Some(subscription.clone());

        fixture.set_name(UPDATED_NAME);
        notifications.raise(&ObservedFixture::owner(&fixture));
        assert_eq!(*values.borrow(), initial_then_updated(), "fault: {fault}");
        assert!(!terminated.get(), "fault: {fault}");
        assert_eq!(notifications.subscriber_count(), 0, "fault: {fault}");
        subscription.dispose();
        *subscription_slot.borrow_mut() = None;
    }
}

#[test]
fn subscribe_before_any_notification_reports_the_current_value() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();

    let subscription = build(&fixture, &notifications, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    assert_eq!(observer.values(), initial_only());
    subscription.dispose();
}

#[test]
fn subscribe_registration_notifies_reads_the_value_back_off_the_object() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();

    let subscription = build(&fixture, &notifications, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    fixture.set_name(UPDATED_NAME);
    notifications.raise(&ObservedFixture::owner(&fixture));
    assert_eq!(observer.values(), initial_then_updated());
    subscription.dispose();
}

#[test]
fn subscribe_registration_notifies_without_a_change_suppresses_the_duplicate() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();

    let subscription = build(&fixture, &notifications, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    notifications.raise(&ObservedFixture::owner(&fixture));
    assert_eq!(observer.values(), initial_only());
    subscription.dispose();
}

#[test]
fn dispose_after_subscribing_stops_observing_and_releases_the_registration() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();

    let subscription = build(&fixture, &notifications, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    subscription.dispose();
    fixture.set_name(UPDATED_NAME);
    notifications.raise(&ObservedFixture::owner(&fixture));

    assert_eq!(observer.values(), initial_only());
    assert_eq!(notifications.subscriber_count(), 0);
}

#[test]
fn subscribe_getter_throws_releases_the_registration() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();
    let fail = Rc::new(Cell::new(true));
    let (reader, read_fails) = (Rc::downgrade(&fixture), fail.clone());
    let observable = PluginPropertyObservable::new(
        notifications.clone() as Rc<dyn ICreatesObservableForProperty>,
        &ObservedFixture::owner(&fixture),
        OBSERVED_PROPERTY_NAME,
        move || {
            if read_fails.get() {
                panic!("getter");
            }
            reader.upgrade().map(|fixture| fixture.name())
        },
        false,
        true,
    );

    let outcome = catch_unwind(AssertUnwindSafe(|| observable.subscribe(observer.clone() as Rc<dyn IObserver<String>>)));
    assert!(outcome.is_err());
    fail.set(false);
    notifications.raise(&ObservedFixture::owner(&fixture));

    assert_eq!(notifications.subscriber_count(), 0);
    assert!(observer.values().is_empty());
}

#[test]
fn subscribe_registration_notifies_during_subscribe_and_getter_throws_releases_the_registration() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    *notifications.notify_during_subscribe.borrow_mut() = Some(ObservedFixture::owner(&fixture));
    let observer = RecordingObserver::<String>::new();
    let fail = Rc::new(Cell::new(true));
    let (reader, read_fails) = (Rc::downgrade(&fixture), fail.clone());
    let observable = PluginPropertyObservable::new(
        notifications.clone() as Rc<dyn ICreatesObservableForProperty>,
        &ObservedFixture::owner(&fixture),
        OBSERVED_PROPERTY_NAME,
        move || {
            if read_fails.get() {
                panic!("getter");
            }
            reader.upgrade().map(|fixture| fixture.name())
        },
        false,
        true,
    );

    let outcome = catch_unwind(AssertUnwindSafe(|| observable.subscribe(observer.clone() as Rc<dyn IObserver<String>>)));
    assert!(outcome.is_err());
    fail.set(false);
    notifications.raise(&ObservedFixture::owner(&fixture));

    assert_eq!(notifications.subscriber_count(), 0);
    assert!(observer.values().is_empty());
}

// Not ported: Constructor_NullRegistration_ThrowsArgumentNullException - argument validation

// Not ported: Constructor_NullSource_ThrowsArgumentNullException - argument validation

// Not ported: Constructor_NullGetter_ThrowsArgumentNullException - argument validation

#[test]
fn subscribe_registration_faults_passes_the_fault_on() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();

    let subscription = build(&fixture, &notifications, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    notifications.fault(new_error("registration gave up"));
    // The recording observer of the test models keeps the last error: one fault reached it.
    assert!(observer.error.borrow().is_some());
    subscription.dispose();
}

#[test]
fn subscribe_registration_completes_ends_the_observation() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();

    let subscription = build(&fixture, &notifications, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    notifications.complete();
    assert!(observer.is_completed.get());
    subscription.dispose();
}

#[test]
fn dispose_called_twice_releases_the_registration_once() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();

    let subscription = build(&fixture, &notifications, true).subscribe(RecordingObserver::<String>::new() as Rc<dyn IObserver<String>>);
    subscription.dispose();
    subscription.dispose();

    assert_eq!(notifications.subscriber_count(), 0);
}

#[test]
fn subscribe_notified_after_disposal_reports_nothing_further() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();

    let subscription = build(&fixture, &notifications, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    subscription.dispose();
    fixture.set_name(UPDATED_NAME);
    notifications.raise_past_unsubscription(&ObservedFixture::owner(&fixture));

    assert_eq!(observer.values(), initial_only());
}

#[test]
fn subscribe_faulted_after_disposal_reports_nothing_further() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();

    let subscription = build(&fixture, &notifications, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    subscription.dispose();
    notifications.fault_past_unsubscription(new_error("after disposal"));

    assert!(observer.error.borrow().is_none());
}

#[test]
fn subscribe_completed_after_disposal_reports_nothing_further() {
    let fixture = ObservedFixture::new(INITIAL_NAME);
    let notifications = PluginNotifications::new();
    let observer = RecordingObserver::<String>::new();

    let subscription = build(&fixture, &notifications, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    subscription.dispose();
    notifications.complete_past_unsubscription();

    assert!(!observer.is_completed.get());
}

/// An object with a notifying name (`ObservedFixture`).
struct ObservedFixture {
    name: RefCell<String>,
    property_changed: Event<str>,
}

impl ObservedFixture {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(name.to_string()), property_changed: Event::new() })
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: &str) {
        *self.name.borrow_mut() = value.to_string();
        self.property_changed.raise("Name");
    }

    /// The fixture as the owner of an observed property.
    fn owner(this: &Rc<Self>) -> PropertyOwner {
        PropertyOwner { object: Some(this.clone() as Rc<dyn Any>), notify: Some(this.clone() as Rc<dyn INotifyPropertyChanged>) }
    }
}

impl INotifyPropertyChanged for ObservedFixture {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

/// A property observer that outranks every other and whose notifications the test raises by hand
/// (`PluginNotifications`).
struct PluginNotifications {
    observers: RefCell<Vec<(u64, Rc<dyn IObserver<Change>>)>>,
    next_id: Cell<u64>,
    last_observer: RefCell<Option<Rc<dyn IObserver<Change>>>>,
    notify_during_subscribe: RefCell<Option<PropertyOwner>>,
    after_subscribe: RefCell<Option<Rc<dyn Fn()>>>,
    this: Weak<PluginNotifications>,
}

impl PluginNotifications {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            observers: RefCell::new(Vec::new()),
            next_id: Cell::new(0),
            last_observer: RefCell::new(None),
            notify_during_subscribe: RefCell::new(None),
            after_subscribe: RefCell::new(None),
            this: this.clone(),
        })
    }

    fn set_after_subscribe(&self, hook: impl Fn() + 'static) {
        *self.after_subscribe.borrow_mut() = Some(Rc::new(hook));
    }

    /// The number of subscribed observers (`SubscriberCount`).
    fn subscriber_count(&self) -> usize {
        self.observers.borrow().len()
    }

    fn current_observers(&self) -> Vec<Rc<dyn IObserver<Change>>> {
        self.observers.borrow().iter().map(|(_, observer)| observer.clone()).collect()
    }

    /// Notifies every subscribed observer (`Raise`).
    fn raise(&self, sender: &PropertyOwner) {
        for observer in self.current_observers() {
            observer.on_next(ObservedChange::new(sender.clone(), String::new(), ()));
        }
    }

    /// Notifies the last observer that subscribed, even once unsubscribed
    /// (`RaisePastUnsubscription`).
    fn raise_past_unsubscription(&self, sender: &PropertyOwner) {
        let last = self.last_observer.borrow().clone();
        if let Some(observer) = last {
            observer.on_next(ObservedChange::new(sender.clone(), String::new(), ()));
        }
    }

    /// Faults the last observer that subscribed, even once unsubscribed
    /// (`FaultPastUnsubscription`).
    fn fault_past_unsubscription(&self, error: ObservableError) {
        let last = self.last_observer.borrow().clone();
        if let Some(observer) = last {
            observer.on_error(error);
        }
    }

    /// Completes the last observer that subscribed, even once unsubscribed
    /// (`CompletePastUnsubscription`).
    fn complete_past_unsubscription(&self) {
        let last = self.last_observer.borrow().clone();
        if let Some(observer) = last {
            observer.on_completed();
        }
    }

    /// Faults every subscribed observer (`Fault`).
    fn fault(&self, error: ObservableError) {
        for observer in self.current_observers() {
            observer.on_error(error.clone());
        }
    }

    /// Completes every subscribed observer (`Complete`).
    fn complete(&self) {
        for observer in self.current_observers() {
            observer.on_completed();
        }
    }
}

impl ICreatesObservableForProperty for PluginNotifications {
    fn get_affinity_for_object(&self, _owner: &PropertyOwner, _property_name: &str, _before_changed: bool) -> i32 {
        i32::MAX
    }

    fn get_notification_for_property(
        &self,
        _owner: &PropertyOwner,
        _property_name: &str,
        _before_changed: bool,
        _suppress_warnings: bool,
    ) -> Rc<dyn IObservable<Change>> {
        self.this.upgrade().expect("the notifications are alive")
    }
}

impl IObservable<Change> for PluginNotifications {
    fn subscribe(&self, observer: Rc<dyn IObserver<Change>>) -> Rc<dyn IDisposable> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.observers.borrow_mut().push((id, observer.clone()));
        *self.last_observer.borrow_mut() = Some(observer.clone());
        let sender = self.notify_during_subscribe.borrow().clone();
        if let Some(sender) = sender {
            observer.on_next(ObservedChange::new(sender, String::new(), ()));
        }
        let hook = self.after_subscribe.borrow().clone();
        if let Some(hook) = hook {
            hook();
        }
        let notifications = self.this.clone();
        Disposable::create(move || {
            if let Some(notifications) = notifications.upgrade() {
                notifications.observers.borrow_mut().retain(|(observer_id, _)| *observer_id != id);
            }
        })
    }
}
