//! `InitialBindingSignalTests` (`ReactiveUI.Binding.Tests/Observables/InitialBindingSignalTests.cs`).
//!
//! An observer that throws is an observer that panics; the test catches the panic.

use super::emission_recorder::EmissionRecorder;
use crate::tests::binding::test_models::{ManualObservable, RecordingObserver};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactiveui::*;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// An error of the tests (`InvalidOperationException`).
#[derive(Debug)]
struct InvalidOperation(&'static str);

impl fmt::Display for InvalidOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for InvalidOperation {}

/// Records the values, the error and the completion, running `on_next` on each value
/// (`RecordingObserver<T>(Action<T>)`).
struct CallbackRecorder {
    values: RefCell<Vec<bool>>,
    on_next: Box<dyn Fn(bool)>,
}

impl IObserver<bool> for CallbackRecorder {
    fn on_next(&self, value: bool) {
        self.values.borrow_mut().push(value);
        (self.on_next)(value);
    }
}

// Not ported: Subscribe_NullObserver_Throws - argument validation

#[test]
fn subscribe_synchronous_snapshots_emits_one_initial_signal_after_attachment() {
    let attached = Rc::new(Cell::new(false));
    let was_attached_at_delivery = Rc::new(Cell::new(false));
    let attaching = attached.clone();
    let source = Observable::create(move |observer: Rc<dyn IObserver<bool>>| {
        observer.on_next(false);
        observer.on_next(true);
        attaching.set(true);
        Disposable::empty()
    });
    let (seen, delivered) = (attached.clone(), was_attached_at_delivery.clone());
    let observer = Rc::new(CallbackRecorder { values: RefCell::new(Vec::new()), on_next: Box::new(move |_| delivered.set(seen.get())) });

    let subscription = InitialBindingSignal::new(source).subscribe(observer.clone());

    assert_eq!(observer.values.borrow().len(), 1);
    assert!(observer.values.borrow()[0]);
    assert!(was_attached_at_delivery.get());
    subscription.dispose();
}

#[test]
fn subscribe_reentrant_update_serializes_behind_initial_signal() {
    const EXPECTED_VALUES: usize = 2;
    let source = ManualObservable::<bool>::new();
    let observer = EmissionRecorder::<bool>::new();
    let reentrant = source.clone();
    observer.set_on_first_value(move || reentrant.on_next(false));

    let subscription = InitialBindingSignal::new(source.clone()).subscribe(observer.observer());
    let values = observer.snapshot();

    assert_eq!(values.len(), EXPECTED_VALUES);
    assert!(values[0]);
    assert!(!values[1]);
    assert_eq!(observer.max_concurrent_emissions(), 1);
    subscription.dispose();
}

/// `Subscribe_SynchronousTerminal_DeliversInitialThenTerminalAndDisposes(bool fault)`, for one row.
fn synchronous_terminal_delivers_initial_then_terminal_and_disposes(fault: bool) {
    let disposals = Rc::new(Cell::new(0));
    let error: ObservableError = Rc::new(InvalidOperation("source error"));
    let (counter, raised) = (disposals.clone(), error.clone());
    let source = Observable::create(move |observer: Rc<dyn IObserver<bool>>| {
        if fault {
            observer.on_error(raised.clone());
        } else {
            observer.on_completed();
        }
        let counter = counter.clone();
        Disposable::create(move || counter.set(counter.get() + 1))
    });
    let observer = RecordingObserver::<bool>::new();

    let subscription = InitialBindingSignal::new(source).subscribe(observer.clone());

    assert_eq!(observer.values(), [true]);
    let received = observer.error.borrow().clone();
    if fault {
        assert!(received.is_some_and(|received| Rc::ptr_eq(&received, &error)));
    } else {
        assert!(received.is_none());
    }
    assert_eq!(observer.is_completed.get(), !fault);
    assert_eq!(disposals.get(), 1);
    subscription.dispose();
}

#[test]
fn subscribe_synchronous_terminal_delivers_initial_then_terminal_and_disposes_without_fault() {
    synchronous_terminal_delivers_initial_then_terminal_and_disposes(false);
}

#[test]
fn subscribe_synchronous_terminal_delivers_initial_then_terminal_and_disposes_with_fault() {
    synchronous_terminal_delivers_initial_then_terminal_and_disposes(true);
}

#[test]
fn subscribe_initial_observer_throws_disposes_source() {
    let source: Rc<Subject<bool>> = Subject::new();
    let observer = Rc::new(CallbackRecorder { values: RefCell::new(Vec::new()), on_next: Box::new(|_| panic!("observer error")) });

    let signal = InitialBindingSignal::new(source.as_observable());
    let outcome = catch_unwind(AssertUnwindSafe(|| signal.subscribe(observer.clone())));

    assert!(outcome.is_err());
    assert!(!source.has_observers());
}

#[test]
fn dispose_late_notifications_are_ignored() {
    let source = ManualObservable::<bool>::new();
    let observer = RecordingObserver::<bool>::new();
    let subscription = InitialBindingSignal::new(source.clone()).subscribe(observer.clone());
    subscription.dispose();
    subscription.dispose();
    let late = source.observer().expect("the signal subscribed");
    late.on_next(false);
    late.on_error(Rc::new(InvalidOperation("late error")));
    late.on_completed();

    assert_eq!(observer.values().len(), 1);
    assert!(observer.error.borrow().is_none());
    assert!(!observer.is_completed.get());
}
