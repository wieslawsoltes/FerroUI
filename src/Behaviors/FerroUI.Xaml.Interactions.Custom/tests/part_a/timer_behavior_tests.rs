//! Not from upstream: when the timed behaviors, triggers and actions of the crate act
//! (`DelayedLoadBehavior`, `DelayedLoadTrigger`, `DelayedShowControlAction`,
//! `InactivityTrigger`, `VisualDebugBehavior`). The reference has no tests of their timing;
//! these were written before their delays became `Duration` (step T3 of C5, #423,
//! docs/idiomatic-rust-signatures.md) to show that the change keeps the behaviour. The types
//! belong to several parts of the crate; the tests share the test application of part A.
//!
//! The timers are the timers of the dispatcher of the test application: a test finds the timer
//! of an interval among them and fires it at once (`force_fire_timer_for_unit_tests`), where
//! the interval would have elapsed.

use super::command::Command;
use super::support::{run_jobs, start_app};
use crate::{DelayedLoadBehavior, DelayedLoadTrigger, DelayedShowControlAction, InactivityTrigger, VisualDebugBehavior};
use ferroui_base::animation::SignedDuration;
use ferroui_base::threading::{Dispatcher, DispatcherTimer};
use ferroui_base::{FerroObject, Ref};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Border, Control, Window};
use ferroui_xaml_interactions::InvokeCommandAction;
use ferroui_xaml_interactivity::{IAction, Interaction};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The interval of the tests: one no timer of the test application has.
const INTERVAL: Duration = Duration::from_millis(1234);

/// The interval of the tests as the time span the properties take.
fn interval() -> SignedDuration {
    SignedDuration::from_milliseconds(1234.0)
}

/// An action that counts its runs.
fn counting_action(runs: &Rc<Cell<i32>>) -> Ref<InvokeCommandAction> {
    let action = InvokeCommandAction::new();
    let runs = runs.clone();
    action.set_command(Some(Command::new(move |_| runs.set(runs.get() + 1))));
    action
}

/// The timers of the dispatcher with `interval`.
fn timers_with(interval: Duration) -> Vec<Rc<DispatcherTimer>> {
    Dispatcher::timers_for_unit_tests().into_iter().filter(|timer| timer.interval() == interval).collect()
}

/// The one timer of the dispatcher with `interval`.
fn timer_with(interval: Duration) -> Rc<DispatcherTimer> {
    let timers = timers_with(interval);
    assert_eq!(1, timers.len(), "the timers of {interval:?}");
    timers[0].clone()
}

/// A window that shows `content`.
fn show_window_with(content: &Ref<Border>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(content.clone())));
    window.show();
    run_jobs();
    window
}

/// Adds `behavior` to the behaviors of `target`.
fn add_behavior(target: &Ref<Border>, behavior: Ref<FerroObject>) {
    Interaction::get_behaviors(target).add(behavior);
}

#[test]
fn delayed_load_behavior_shows_its_control_when_its_delay_elapses() {
    let _app = start_app();
    let target = Border::new();
    let behavior = DelayedLoadBehavior::new();
    behavior.set_delay(interval());
    add_behavior(&target, behavior.clone().upcast());
    let _window = show_window_with(&target);

    // Attached to the visual tree, the control is hidden until the delay elapses.
    assert!(!target.is_visible());
    let timer = timer_with(INTERVAL);
    assert!(timer.is_enabled());

    Dispatcher::force_fire_timer_for_unit_tests(&timer);
    assert!(target.is_visible());
    assert!(timers_with(INTERVAL).is_empty());
}

#[test]
fn delayed_load_trigger_runs_its_actions_when_its_delay_elapses() {
    let _app = start_app();
    let runs = Rc::new(Cell::new(0));
    let target = Border::new();
    let trigger = DelayedLoadTrigger::new();
    trigger.set_delay(interval());
    trigger.actions().expect("the actions of the trigger").add(counting_action(&runs).upcast());
    add_behavior(&target, trigger.clone().upcast());
    let _window = show_window_with(&target);

    // Attached to the visual tree, the trigger waits for the delay.
    assert_eq!(0, runs.get());
    let timer = timer_with(INTERVAL);

    Dispatcher::force_fire_timer_for_unit_tests(&timer);
    assert_eq!(1, runs.get());
    assert!(timers_with(INTERVAL).is_empty());
}

#[test]
fn delayed_show_control_action_shows_its_control_when_its_delay_elapses() {
    let _app = start_app();
    let target = Border::new();
    target.set_is_visible(false);
    let action = DelayedShowControlAction::new();
    action.set_delay(interval());

    let sender = Control::boxed(target.clone());
    let result = <dyn IAction>::of(&action).expect("an action").execute(Some(&sender), None);
    assert_eq!(result.and_then(|result| result.downcast_ref::<bool>().copied()), Some(true));

    // The control stays hidden until the delay elapses.
    assert!(!target.is_visible());
    let timer = timer_with(INTERVAL);

    Dispatcher::force_fire_timer_for_unit_tests(&timer);
    assert!(target.is_visible());
    assert!(!timer.is_enabled());
}

#[test]
fn inactivity_trigger_runs_its_actions_when_its_timeout_elapses() {
    let _app = start_app();
    let runs = Rc::new(Cell::new(0));
    let target = Border::new();
    let trigger = InactivityTrigger::new();
    trigger.set_timeout(interval());
    trigger.actions().add(counting_action(&runs).upcast());
    add_behavior(&target, trigger.clone().upcast());
    let _window = show_window_with(&target);

    // Attached to the visual tree of a top level, the trigger waits for the timeout.
    assert_eq!(0, runs.get());
    let timer = timer_with(INTERVAL);
    assert!(timer.is_enabled());

    // Without input until it elapses, the actions run once and the timer stops.
    Dispatcher::force_fire_timer_for_unit_tests(&timer);
    assert_eq!(1, runs.get());
    assert!(!timer.is_enabled());
}

#[test]
fn visual_debug_behavior_removes_its_highlight_when_its_duration_elapses() {
    let _app = start_app();
    let target = Border::new();
    let behavior = VisualDebugBehavior::new();
    behavior.set_duration(interval());
    add_behavior(&target, behavior.clone().upcast());
    let _window = show_window_with(&target);

    // The event (attached to the visual tree, the default) highlights the control with an
    // adorner until the duration elapses.
    let adorner_layer = AdornerLayer::get_adorner_layer(&target).expect("the adorner layer of the window");
    let adorners = adorner_layer.children().count();
    assert!(adorners >= 1, "the highlight is an adorner");
    let timer = timer_with(INTERVAL);

    Dispatcher::force_fire_timer_for_unit_tests(&timer);
    assert_eq!(adorners - 1, adorner_layer.children().count());
    assert!(timers_with(INTERVAL).is_empty());
}
