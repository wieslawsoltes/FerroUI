//! `ActivationSignalsTests` (`ReactiveUI.Tests/Activation/ActivationSignalsTests.cs`).

use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObserver};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

const SENTINEL_VALUE: i32 = 42;

/// Whether `action` panics, as the use of a disposed subject does (`ObjectDisposedException`).
fn panics(action: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(action)).is_err()
}

#[test]
fn raise_notifies_subscribers() {
    let signal = Subject::<()>::new();
    let raised = Rc::new(Cell::new(0));
    let count = raised.clone();
    let subscription = signal.as_observable().subscribe_next(move |_| count.set(count.get() + 1));

    ActivationSignals::raise(&signal);

    assert_eq!(raised.get(), 1);
    subscription.dispose();
}

#[test]
fn raise_disposed_signal_does_nothing() {
    let signal = Subject::<()>::new();
    signal.dispose();

    assert!(!panics(|| ActivationSignals::raise(&signal)));
}

#[test]
fn schedule_activation_signals_disposed_before_the_work_runs_does_nothing() {
    let scheduler = TestScheduler::new();
    let original = RxSchedulers::main_thread_scheduler();
    RxSchedulers::set_main_thread_scheduler(scheduler.scheduler());

    let activated = Subject::<()>::new();
    let deactivated = Subject::<()>::new();
    ActivationSignals::schedule_activation(true, &activated, &deactivated);
    ActivationSignals::dispose_when(true, &activated, &deactivated);

    assert!(!panics(|| scheduler.run()));
    RxSchedulers::set_main_thread_scheduler(original);
}

#[test]
fn dispose_when_not_disposing_leaves_the_signals_usable() {
    let activated = Subject::<()>::new();
    let deactivated = Subject::<()>::new();
    let raised = Rc::new(Cell::new(0));
    let count = raised.clone();
    let _subscription = activated.as_observable().subscribe_next(move |_| count.set(count.get() + 1));

    ActivationSignals::dispose_when(false, &activated, &deactivated);
    activated.on_next(());

    assert_eq!(raised.get(), 1);
}

#[test]
fn dispose_when_disposing_closes_both_signals() {
    let activated = Subject::<()>::new();
    let deactivated = Subject::<()>::new();

    ActivationSignals::dispose_when(true, &activated, &deactivated);

    assert!(panics(|| activated.on_next(())));
    assert!(panics(|| deactivated.on_next(())));
}

#[test]
fn dispose_when_with_additional_signal_not_disposing_leaves_everything_usable() {
    let activated = Subject::<()>::new();
    let deactivated = Subject::<()>::new();
    let additional = Subject::<i32>::new();
    let received = Rc::new(Cell::new(0));
    let seen = received.clone();
    let _subscription = additional.as_observable().subscribe_next(move |value| seen.set(value));

    ActivationSignals::dispose_when_with(false, &activated, &deactivated, &additional);
    additional.on_next(SENTINEL_VALUE);

    assert_eq!(received.get(), SENTINEL_VALUE);
}

#[test]
fn dispose_when_with_additional_signal_disposing_closes_the_additional_signal() {
    let activated = Subject::<()>::new();
    let deactivated = Subject::<()>::new();
    let additional = Subject::<i32>::new();

    ActivationSignals::dispose_when_with(true, &activated, &deactivated, &additional);

    assert!(panics(|| activated.on_next(())));
    assert!(panics(|| deactivated.on_next(())));
    assert!(panics(|| additional.on_next(1)));
}

#[test]
fn schedule_activation_when_activating_raises_only_the_activated_signal() {
    let (activations, deactivations) = schedule_and_count(true);
    assert_eq!(activations, 1);
    assert_eq!(deactivations, 0);
}

#[test]
fn schedule_activation_when_deactivating_raises_only_the_deactivated_signal() {
    let (activations, deactivations) = schedule_and_count(false);
    assert_eq!(activations, 0);
    assert_eq!(deactivations, 1);
}

/// Schedules the activation on the immediate scheduler and counts the two signals
/// (`ScheduleAndCount`).
fn schedule_and_count(is_activating: bool) -> (i32, i32) {
    let original = RxSchedulers::main_thread_scheduler();
    RxSchedulers::set_main_thread_scheduler(ImmediateScheduler::instance());
    let activated = Subject::<()>::new();
    let deactivated = Subject::<()>::new();
    let activations = Rc::new(Cell::new(0));
    let deactivations = Rc::new(Cell::new(0));
    let (counted_activations, counted_deactivations) = (activations.clone(), deactivations.clone());
    let _activated_subscription = activated.as_observable().subscribe_next(move |_| counted_activations.set(counted_activations.get() + 1));
    let _deactivated_subscription = deactivated.as_observable().subscribe_next(move |_| counted_deactivations.set(counted_deactivations.get() + 1));

    ActivationSignals::schedule_activation(is_activating, &activated, &deactivated);

    RxSchedulers::set_main_thread_scheduler(original);
    (activations.get(), deactivations.get())
}
