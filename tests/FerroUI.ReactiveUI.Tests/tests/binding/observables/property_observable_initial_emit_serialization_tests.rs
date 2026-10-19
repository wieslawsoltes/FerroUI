//! `PropertyObservableInitialEmitSerializationTests`
//! (`ReactiveUI.Binding.Tests/Observables/PropertyObservableInitialEmitSerializationTests.cs`).
//!
//! The tests whose competing write runs on another thread are not ported; the ones whose
//! competing write is re-entrant, on the delivering thread, are.

use super::emission_recorder::EmissionRecorder;
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable};
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

const INITIAL_NAME: &str = "Alice";
const REPLACEMENT_NAME: &str = "Bob";
const THIRD_NAME: &str = "Carol";

fn some(name: &str) -> Option<String> {
    Some(name.to_string())
}

fn assert_no_errors<T: Clone + 'static>(recorder: &EmissionRecorder<T>) {
    assert!(recorder.error_snapshot().is_empty());
}

fn assert_sequence<T: PartialEq + Debug>(actual: &[T], expected: &[T]) {
    assert_eq!(actual.len(), expected.len());
    for (index, value) in expected.iter().enumerate() {
        assert_eq!(&actual[index], value);
    }
}

fn name_of(source: &Rc<HookedViewModel>, distinct_until_changed: bool) -> PropertyObservable<HookedViewModel, Option<String>> {
    PropertyObservable::new(source, "Name", |x: &Rc<HookedViewModel>| x.name(), distinct_until_changed)
}

// Not ported: Subscribe_PropertyChangedRaisedOnAnotherThreadWhileAttaching_EmitsTheValueOnce - real threads

#[test]
fn subscribe_property_changed_raised_reentrantly_during_initial_read_emits_the_value_once() {
    let source = HookedViewModel::new();
    source.set_name(some(INITIAL_NAME));
    let recorder = EmissionRecorder::<Option<String>>::new();
    let raised = Rc::new(Cell::new(false));

    let read_raised = raised.clone();
    let observable = PropertyObservable::new(
        &source,
        "Name",
        move |instance: &Rc<HookedViewModel>| {
            if !read_raised.replace(true) {
                instance.raise_property_changed("Name");
            }
            instance.name()
        },
        true,
    );

    let subscription = observable.subscribe(recorder.observer());
    assert_no_errors(&recorder);
    assert_sequence(&recorder.snapshot(), &[some(INITIAL_NAME)]);
    subscription.dispose();
}

// Not ported: Subscribe_PropertyChangedRaisedOnAnotherThreadDuringInitialEmit_HandsOffWithoutOverlap - real threads

#[test]
fn on_next_observer_changes_the_observed_property_delivers_the_change_after_the_observer_returns() {
    let source = HookedViewModel::new();
    source.set_name(some(INITIAL_NAME));
    let recorder = EmissionRecorder::<Option<String>>::new();
    let writer = Rc::downgrade(&source);
    recorder.set_on_first_value(move || {
        if let Some(source) = writer.upgrade() {
            source.set_name(some(REPLACEMENT_NAME));
        }
    });
    let observable = name_of(&source, true);

    let subscription = observable.subscribe(recorder.observer());
    assert_no_errors(&recorder);
    assert_eq!(recorder.max_concurrent_emissions(), 1);
    assert_sequence(&recorder.snapshot(), &[some(INITIAL_NAME), some(REPLACEMENT_NAME)]);
    subscription.dispose();
}

#[test]
fn on_property_changed_read_throws_the_next_change_still_emits() {
    let source = HookedViewModel::new();
    source.set_name(some(INITIAL_NAME));
    let recorder = EmissionRecorder::<Option<String>>::new();
    let fail_next_read = Rc::new(Cell::new(false));

    let read_fails = fail_next_read.clone();
    let observable = PropertyObservable::new(
        &source,
        "Name",
        move |instance: &Rc<HookedViewModel>| {
            if read_fails.replace(false) {
                panic!("ReadOrFail");
            }
            instance.name()
        },
        true,
    );

    let subscription = observable.subscribe(recorder.observer());
    fail_next_read.set(true);
    let outcome = catch_unwind(AssertUnwindSafe(|| source.set_name(some(REPLACEMENT_NAME))));
    assert!(outcome.is_err());
    source.set_name(some(THIRD_NAME));
    assert_sequence(&recorder.snapshot(), &[some(INITIAL_NAME), some(THIRD_NAME)]);
    subscription.dispose();
}

#[test]
fn subscribe_value_is_the_type_default_still_emits_the_initial_value() {
    for distinct_until_changed in [true, false] {
        let source = HookedViewModel::new();
        source.set_name(None);
        source.set_count(0);
        let name_recorder = EmissionRecorder::<Option<String>>::new();
        let name_observable = name_of(&source, distinct_until_changed);
        let count_recorder = EmissionRecorder::<i32>::new();
        let count_observable = PropertyObservable::new(&source, "Count", |x: &Rc<HookedViewModel>| x.count(), distinct_until_changed);

        let name_subscription = name_observable.subscribe(name_recorder.observer());
        let count_subscription = count_observable.subscribe(count_recorder.observer());
        assert_eq!(name_recorder.snapshot().len(), 1);
        assert_eq!(name_recorder.snapshot()[0], None);
        assert_sequence(&count_recorder.snapshot(), &[0]);
        count_subscription.dispose();
        name_subscription.dispose();
    }
}

#[test]
fn subscribe_value_already_delivered_to_another_subscriber_still_emits_the_initial_value() {
    for distinct_until_changed in [true, false] {
        let source = HookedViewModel::new();
        source.set_name(some(INITIAL_NAME));
        let observable = name_of(&source, distinct_until_changed);
        let first = EmissionRecorder::<Option<String>>::new();
        let second = EmissionRecorder::<Option<String>>::new();
        let after_disposal = EmissionRecorder::<Option<String>>::new();

        let first_subscription = observable.subscribe(first.observer());
        let second_subscription = observable.subscribe(second.observer());
        assert_sequence(&first.snapshot(), &[some(INITIAL_NAME)]);
        assert_sequence(&second.snapshot(), &[some(INITIAL_NAME)]);
        second_subscription.dispose();
        first_subscription.dispose();

        let after_subscription = observable.subscribe(after_disposal.observer());
        assert_sequence(&after_disposal.snapshot(), &[some(INITIAL_NAME)]);
        after_subscription.dispose();
    }
}

// Not ported: Subscribe_ConcurrentWritesThroughoutSubscription_NeverEmitsConsecutiveDuplicates - real threads

#[test]
fn subscribe_handler_attaches_after_the_raiser_snapshots_its_delegate_still_observes_the_written_value() {
    let source = ConventionalRaiseOrderViewModel::new(INITIAL_NAME);
    let recorder = EmissionRecorder::<Option<String>>::new();
    let subscriptions: Rc<RefCell<Vec<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(Vec::new()));

    let (hook_source, hook_recorder, bag) = (Rc::downgrade(&source), recorder.clone(), subscriptions.clone());
    source.set_before_raise(move || {
        let Some(source) = hook_source.upgrade() else {
            return;
        };
        let observable = PropertyObservable::new(&source, "Name", |x: &Rc<ConventionalRaiseOrderViewModel>| x.name(), true);
        bag.borrow_mut().push(observable.subscribe(hook_recorder.observer()));
    });
    source.set_name(some(REPLACEMENT_NAME));

    assert!(!source.handler_was_invoked.get());
    assert_sequence(&recorder.snapshot(), &[some(REPLACEMENT_NAME)]);
    for subscription in subscriptions.borrow().iter() {
        subscription.dispose();
    }
}

#[test]
fn subscribe_source_raises_property_changed_before_writing_the_field_observes_the_pre_write_value() {
    let source = RaiseBeforeWriteViewModel::new(INITIAL_NAME);
    let recorder = EmissionRecorder::<Option<String>>::new();
    let subscriptions: Rc<RefCell<Vec<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(Vec::new()));

    let (hook_source, hook_recorder, bag) = (Rc::downgrade(&source), recorder.clone(), subscriptions.clone());
    source.set_before_raise(move || {
        let Some(source) = hook_source.upgrade() else {
            return;
        };
        let observable = PropertyObservable::new(&source, "Name", |x: &Rc<RaiseBeforeWriteViewModel>| x.name(), true);
        bag.borrow_mut().push(observable.subscribe(hook_recorder.observer()));
    });
    source.set_name(some(REPLACEMENT_NAME));

    assert!(!source.handler_was_invoked.get());
    assert_eq!(source.name(), some(REPLACEMENT_NAME));
    assert_sequence(&recorder.snapshot(), &[some(INITIAL_NAME)]);
    for subscription in subscriptions.borrow().iter() {
        subscription.dispose();
    }
}

/// A view model that writes, then raises (`HookedViewModel`; the hook run when a handler is
/// attached serves the tests on other threads only, which are not ported).
struct HookedViewModel {
    name: RefCell<Option<String>>,
    count: Cell<i32>,
    property_changed: Event<str>,
}

impl HookedViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(None), count: Cell::new(0), property_changed: Event::new() })
    }

    fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
        self.raise_property_changed("Name");
    }

    fn count(&self) -> i32 {
        self.count.get()
    }

    fn set_count(&self, value: i32) {
        self.count.set(value);
        self.raise_property_changed("Count");
    }

    fn raise_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }
}

impl INotifyPropertyChanged for HookedViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

/// A view model that writes, takes the handlers, runs a hook, and raises to the handlers it took
/// (`ConventionalRaiseOrderViewModel`).
struct ConventionalRaiseOrderViewModel {
    name: RefCell<Option<String>>,
    before_raise: RefCell<Option<Box<dyn FnOnce()>>>,
    handler_was_invoked: Cell<bool>,
    property_changed: Event<str>,
}

impl ConventionalRaiseOrderViewModel {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self {
            name: RefCell::new(some(name)),
            before_raise: RefCell::new(None),
            handler_was_invoked: Cell::new(false),
            property_changed: Event::new(),
        })
    }

    fn set_before_raise(&self, hook: impl FnOnce() + 'static) {
        *self.before_raise.borrow_mut() = Some(Box::new(hook));
    }

    fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
        let snapshot = self.property_changed.snapshot();
        let hook = self.before_raise.borrow_mut().take();
        if let Some(hook) = hook {
            hook();
        }
        if snapshot.is_empty() {
            return;
        }
        self.handler_was_invoked.set(true);
        for (_, handler) in snapshot.iter() {
            handler("Name");
        }
    }
}

impl INotifyPropertyChanged for ConventionalRaiseOrderViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

/// A view model that takes the handlers, runs a hook, raises to the handlers it took, and only
/// then writes (`RaiseBeforeWriteViewModel`).
struct RaiseBeforeWriteViewModel {
    name: RefCell<Option<String>>,
    before_raise: RefCell<Option<Box<dyn FnOnce()>>>,
    handler_was_invoked: Cell<bool>,
    property_changed: Event<str>,
}

impl RaiseBeforeWriteViewModel {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self {
            name: RefCell::new(some(name)),
            before_raise: RefCell::new(None),
            handler_was_invoked: Cell::new(false),
            property_changed: Event::new(),
        })
    }

    fn set_before_raise(&self, hook: impl FnOnce() + 'static) {
        *self.before_raise.borrow_mut() = Some(Box::new(hook));
    }

    fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: Option<String>) {
        let snapshot = self.property_changed.snapshot();
        let hook = self.before_raise.borrow_mut().take();
        if let Some(hook) = hook {
            hook();
        }
        if !snapshot.is_empty() {
            self.handler_was_invoked.set(true);
            for (_, handler) in snapshot.iter() {
                handler("Name");
            }
        }
        *self.name.borrow_mut() = value;
    }
}

impl INotifyPropertyChanged for RaiseBeforeWriteViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
