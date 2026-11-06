//! Not from upstream: when the actions that wait run their actions (`DebounceAction`,
//! `ThrottleAction`, `DelayAction`). The reference has no tests of their timing; these were
//! written before their intervals became `Duration` (step T3 of C5, #423,
//! docs/idiomatic-rust-signatures.md) to show that the change keeps the behaviour.
//!
//! The timers are the timers of the dispatcher of the test application: a test finds the timer
//! of an interval among them and fires it at once (`force_fire_timer_for_unit_tests`), where
//! the interval would have elapsed.

use super::command::Command;
use crate::tests::support::{load, run_jobs, start_app};
use crate::{ActionTask, DebounceAction, DelayAction, InvokeCommandAction, ThrottleAction};
use ferroui_base::animation::SignedDuration;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::threading::{Dispatcher, DispatcherTimer};
use ferroui_base::{ferro_model, BoxedValue, FerroObject, FerroProperty, Ref};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
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
    action.set_delay(INTERVAL);
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

    execute(&action);

    // A zero delay is a timer of zero, which the dispatcher promotes at once instead of
    // scheduling it (`DispatcherTimer::restart`): the actions run when the dispatcher runs its
    // jobs, not in the invocation.
    assert_eq!(0, runs.get());
    Dispatcher::ui_thread().run_jobs(None);
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
    action.set_interval(Duration::from_secs(3600));
    execute(&action);
    execute(&action);
    assert_eq!(2, runs.get());
    assert!(timers_with(Duration::from_secs(3600)).is_empty());

    // Once the interval is shorter than the time since the last run, the actions run again.
    action.set_interval(Duration::ZERO);
    execute(&action);
    assert_eq!(3, runs.get());
}

#[test]
fn delay_action_completes_its_task_when_its_duration_elapses() {
    let _app = start_app();
    let action = DelayAction::new();
    action.set_duration(INTERVAL);

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
    // `Timeout.InfiniteTimeSpan` of the reference (minus one millisecond, which is not a
    // `Duration`) is the longest duration, and any duration as long as the longest time span
    // (`TimeSpan.MaxValue`, which markup can name).
    let longest_time_span = Duration::try_from(SignedDuration::MAX_VALUE).expect("a duration");
    for duration in [Duration::MAX, longest_time_span] {
        let action = DelayAction::new();
        action.set_duration(duration);
        let timers = Dispatcher::timers_for_unit_tests().len();

        let result = execute(&action).expect("the task of the wait");
        let task = result.downcast_ref::<ActionTask>().expect("an action task").clone();
        run_jobs();

        // No timer is started, and the task does not complete.
        assert_eq!(timers, Dispatcher::timers_for_unit_tests().len(), "{duration:?}");
        assert!(!task.is_completed(), "{duration:?}");
    }
}

// --- negative intervals (decision O2) -----------------------------------------------------
//
// Not from upstream (C5 of #423, step T3): the intervals of the actions are `Duration`
// properties. The reference refuses a negative interval with an argument exception of its
// setter; here a negative time span is a load error of markup, and a binding that gives one
// reports a conversion error and leaves the property at the value it has without the binding.

/// The documents of the intervals of the actions; `{}` is the time span.
const ACTION_INTERVAL_DOCUMENTS: &[(&str, &str)] = &[
    ("DebounceAction.Delay", "<DebounceAction xmlns='https://github.com/ferroui' Delay='{}'/>"),
    ("ThrottleAction.Interval", "<ThrottleAction xmlns='https://github.com/ferroui' Interval='{}'/>"),
    ("DelayAction.Duration", "<DelayAction xmlns='https://github.com/ferroui' Duration='{}'/>"),
];

/// Loads `xaml` with the run-time loader: the root object, or the error in its debug form
/// (with the errors it wraps).
fn try_load(xaml: &str) -> Result<BoxedValue, String> {
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&crate::ASSEMBLY);
    FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        .map_err(|error| format!("{error:?}"))
}

#[test]
fn a_time_span_literal_sets_the_intervals_of_the_actions() {
    let _app = start_app();
    let debounce = load::<DebounceAction>("debounce", &ACTION_INTERVAL_DOCUMENTS[0].1.replace("{}", "0:0:1.234"));
    assert_eq!(INTERVAL, debounce.delay());
    let throttle = load::<ThrottleAction>("throttle", &ACTION_INTERVAL_DOCUMENTS[1].1.replace("{}", "0.25"));
    assert_eq!(Duration::from_millis(250), throttle.interval());
    let delay = load::<DelayAction>("delay", &ACTION_INTERVAL_DOCUMENTS[2].1.replace("{}", "00:00:02"));
    assert_eq!(Duration::from_secs(2), delay.duration());
}

#[test]
fn a_negative_time_span_literal_for_an_interval_of_the_actions_is_an_error() {
    let _app = start_app();
    for (property, document) in ACTION_INTERVAL_DOCUMENTS {
        let error = match try_load(&document.replace("{}", "-0:0:1")) {
            Ok(_) => panic!("{property}: a negative time span is not a duration"),
            Err(error) => error,
        };
        assert!(error.contains("The TimeSpan -00:00:01 is negative and cannot be converted to a Duration."), "{property}: {error}");
    }
}

/// A view model with a time span.
struct Timing {
    span: Cell<SignedDuration>,
    property_changed: Event<str>,
}

impl Timing {
    fn new(span: SignedDuration) -> Rc<Self> {
        Model::new_model(Self { span: Cell::new(span), property_changed: Event::new() })
    }

    fn set_span(&self, value: SignedDuration) {
        self.span.set(value);
        self.property_changed.raise("Span");
    }
}

impl INotifyPropertyChanged for Timing {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Timing, |b| b
    .notify_property_changed()
    .property::<Value<SignedDuration>>("Span", |o| o.span.get(), |o, v| o.set_span(v)));

/// Binds `property` of `target` to a time span that starts negative: the property keeps
/// `default` (what `get` reads without the binding); a span that is not negative is set as the
/// duration of the same value; a negative one again returns the property to `default`.
fn a_negative_span_keeps_the_default(
    target: &FerroObject,
    property: &'static FerroProperty,
    get: impl Fn() -> Duration,
    default: Duration,
) {
    let name = property.name();
    assert_eq!(get(), default, "{name}: the default");
    let source = Timing::new(SignedDuration::from_seconds(-1.0));
    let binding = ReflectionBinding::new("Span");
    binding.set_source(Some(source.clone() as BoxedValue));
    target.bind_binding(property, &*binding);
    assert_eq!(get(), default, "{name}: a negative span is not set");

    source.set_span(SignedDuration::from_milliseconds(1234.0));
    assert_eq!(get(), INTERVAL, "{name}: a span that is not negative is set");

    source.set_span(SignedDuration::from_seconds(-2.0));
    assert_eq!(get(), default, "{name}: a negative span returns the property to its default");
}

#[test]
fn a_negative_span_from_a_binding_leaves_the_intervals_of_the_actions_at_their_defaults() {
    let _app = start_app();
    let debounce = DebounceAction::new();
    a_negative_span_keeps_the_default(
        &debounce,
        DebounceAction::delay_property().as_property(),
        || debounce.delay(),
        Duration::ZERO,
    );

    let throttle = ThrottleAction::new();
    a_negative_span_keeps_the_default(
        &throttle,
        ThrottleAction::interval_property().as_property(),
        || throttle.interval(),
        Duration::ZERO,
    );

    let delay = DelayAction::new();
    a_negative_span_keeps_the_default(
        &delay,
        DelayAction::duration_property().as_property(),
        || delay.duration(),
        Duration::ZERO,
    );
}
