//! Not from upstream: when the actions that wait run their actions (`DebounceAction`,
//! `ThrottleAction`, `DelayAction`). The reference has no tests of their timing; these were
//! written before their intervals became `Duration` (step T3 of C5, #423,
//! docs/idiomatic-rust-signatures.md) to show that the change keeps the behaviour.
//!
//! The timers are the timers of the dispatcher of the test application: a test finds the timer
//! of an interval among them and fires it at once (`force_fire_timer_for_unit_tests`), where
//! the interval would have elapsed.

use super::command::Command;
use crate::tests::support::{run_jobs, start_app};
use crate::{ActionTask, DebounceAction, DelayAction, InvokeCommandAction, ThrottleAction};
use ferroui_base::animation::SignedDuration;
use ferroui_base::threading::{Dispatcher, DispatcherTimer};
use ferroui_base::{BoxedValue, FerroObject, Ref};
use ferroui_xaml_interactivity::IAction;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The interval of the tests: one no timer of the test application has.
const INTERVAL: Duration = Duration::from_millis(1234);

/// An action that counts its runs.
fn counting_action(runs: &Rc<Cell<i32>>) -> Ref<InvokeCommandAction> {
    let action = InvokeCommandAction::new();
    let runs = runs.clone();
    action.set_command(Some(Command::new(move |_| runs.set(runs.get() + 1))));
    action
}

/// Executes `action` as a trigger does, without a sender or a parameter.
fn execute(action: &FerroObject) -> Option<BoxedValue> {
    <dyn IAction>::of(action).expect("an action").execute(None, None)
}

/// The timers of the dispatcher with `interval`.
fn timers_with(interval: Duration) -> Vec<Rc<DispatcherTimer>> {
    Dispatcher::timers_for_unit_tests().into_iter().filter(|timer| timer.interval() == interval).collect()
}

#[test]
fn debounce_action_runs_its_actions_when_its_delay_elapses_after_the_last_invocation() {
    let _app = start_app();
    let runs = Rc::new(Cell::new(0));
    let action = DebounceAction::new();
    action.set_delay(SignedDuration::from_milliseconds(1234.0));
    action.actions().expect("the actions").add(counting_action(&runs).upcast());

    assert!(execute(&action).is_none());
    assert!(execute(&action).is_none());

    // One timer of the delay runs, started again by the second invocation; the actions wait.
    let timers = timers_with(INTERVAL);
    assert_eq!(1, timers.len());
    assert!(timers[0].is_enabled());
    assert_eq!(0, runs.get());

    // When the delay elapses the actions run once, and the timer stops.
    Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
    assert_eq!(1, runs.get());
    assert!(!timers[0].is_enabled());
    assert!(timers_with(INTERVAL).is_empty());

    // A later invocation waits for the delay again.
    execute(&action);
    assert_eq!(1, runs.get());
    let timers = timers_with(INTERVAL);
    assert_eq!(1, timers.len());
    Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
    assert_eq!(2, runs.get());
}

#[test]
fn debounce_action_without_a_delay_waits_for_the_dispatcher() {
    let _app = start_app();
    let runs = Rc::new(Cell::new(0));
    let action = DebounceAction::new();
    action.actions().expect("the actions").add(counting_action(&runs).upcast());

    let before = timers_with(Duration::ZERO);
    execute(&action);

    // A zero delay is a timer of zero: the actions run when the dispatcher runs it, not in the
    // invocation.
    assert_eq!(0, runs.get());
    let timer = timers_with(Duration::ZERO)
        .into_iter()
        .find(|timer| !before.iter().any(|old| Rc::ptr_eq(old, timer)))
        .expect("the timer of the delay");
    Dispatcher::force_fire_timer_for_unit_tests(&timer);
    assert_eq!(1, runs.get());
}

#[test]
fn throttle_action_ignores_the_invocations_within_its_interval() {
    let _app = start_app();
    let runs = Rc::new(Cell::new(0));
    let action = ThrottleAction::new();
    action.actions().expect("the actions").add(counting_action(&runs).upcast());

    // Without an interval every invocation runs the actions at once.
    execute(&action);
    execute(&action);
    assert_eq!(2, runs.get());

    // Within the interval since the last run an invocation is ignored; no timer is involved.
    action.set_interval(SignedDuration::from_seconds(3600.0));
    execute(&action);
    execute(&action);
    assert_eq!(2, runs.get());
    assert!(timers_with(Duration::from_secs(3600)).is_empty());

    // Once the interval is shorter than the time since the last run, the actions run again.
    action.set_interval(SignedDuration::ZERO);
    execute(&action);
    assert_eq!(3, runs.get());
}

#[test]
fn delay_action_completes_its_task_when_its_duration_elapses() {
    let _app = start_app();
    let action = DelayAction::new();
    action.set_duration(SignedDuration::from_milliseconds(1234.0));

    let result = execute(&action).expect("the task of the wait");
    let task = result.downcast_ref::<ActionTask>().expect("an action task").clone();
    run_jobs();

    // The wait runs a timer of the duration; the task waits for it.
    let timers = timers_with(INTERVAL);
    assert_eq!(1, timers.len());
    assert!(!task.is_completed());

    Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
    run_jobs();
    assert!(task.is_completed_successfully());
}

#[test]
fn delay_action_without_a_duration_completes_at_once() {
    let _app = start_app();
    let action = DelayAction::new();
    let timers = Dispatcher::timers_for_unit_tests().len();

    let result = execute(&action).expect("the task of the wait");
    let task = result.downcast_ref::<ActionTask>().expect("an action task").clone();
    run_jobs();

    // The wait needs no timer.
    assert!(task.is_completed_successfully());
    assert_eq!(timers, Dispatcher::timers_for_unit_tests().len());
}

#[test]
fn delay_action_with_the_infinite_duration_never_completes() {
    let _app = start_app();
    let action = DelayAction::new();
    // `Timeout.InfiniteTimeSpan` of the reference.
    action.set_duration(SignedDuration::from_milliseconds(-1.0));
    let timers = Dispatcher::timers_for_unit_tests().len();

    let result = execute(&action).expect("the task of the wait");
    let task = result.downcast_ref::<ActionTask>().expect("an action task").clone();
    run_jobs();

    // No timer is started, and the task does not complete.
    assert_eq!(timers, Dispatcher::timers_for_unit_tests().len());
    assert!(!task.is_completed());
}
