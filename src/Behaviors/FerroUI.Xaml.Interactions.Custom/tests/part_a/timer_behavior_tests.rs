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
use super::support::{load, run_jobs, start_app};
use crate::{DelayedLoadBehavior, DelayedLoadTrigger, DelayedShowControlAction, InactivityTrigger, VisualDebugBehavior};
use ferroui_base::animation::SignedDuration;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::threading::{Dispatcher, DispatcherTimer};
use ferroui_base::{ferro_model, BoxedValue, FerroObject, FerroProperty, Ref};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Border, Control, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_xaml_interactions::InvokeCommandAction;
use ferroui_xaml_interactivity::{IAction, Interaction};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The interval of the tests: one no timer of the test application has.
const INTERVAL: Duration = Duration::from_millis(1234);

/// The interval of the tests as the properties take it.
fn interval() -> Duration {
    INTERVAL
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

// --- negative intervals (decision O2) -----------------------------------------------------
//
// Not from upstream (C5 of #423, step T3): the delays, the timeout and the duration of the
// highlight are `Duration` properties. A negative time span is a load error of markup, and a
// binding that gives one reports a conversion error and leaves the property at the value it
// has without the binding.

/// The documents of the timer properties; `{}` is the time span.
const TIMER_DOCUMENTS: &[(&str, &str)] = &[
    ("DelayedLoadBehavior.Delay", "<DelayedLoadBehavior xmlns='https://github.com/ferroui' Delay='{}'/>"),
    ("DelayedLoadTrigger.Delay", "<DelayedLoadTrigger xmlns='https://github.com/ferroui' Delay='{}'/>"),
    ("DelayedShowControlAction.Delay", "<DelayedShowControlAction xmlns='https://github.com/ferroui' Delay='{}'/>"),
    ("InactivityTrigger.Timeout", "<InactivityTrigger xmlns='https://github.com/ferroui' Timeout='{}'/>"),
    ("VisualDebugBehavior.Duration", "<VisualDebugBehavior xmlns='https://github.com/ferroui' Duration='{}'/>"),
];

/// Loads `xaml` with the run-time loader: the root object, or the error in its debug form
/// (with the errors it wraps).
fn try_load(xaml: &str) -> Result<BoxedValue, String> {
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&crate::ASSEMBLY);
    FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        .map_err(|error| format!("{error:?}"))
}

/// The document of `TIMER_DOCUMENTS` at `index` with the time span `span`.
fn timer_document(index: usize, span: &str) -> String {
    TIMER_DOCUMENTS[index].1.replace("{}", span)
}

#[test]
fn a_time_span_literal_sets_the_timer_properties() {
    let _app = start_app();
    assert_eq!(INTERVAL, load::<DelayedLoadBehavior>("behavior", &timer_document(0, "0:0:1.234")).delay());
    assert_eq!(INTERVAL, load::<DelayedLoadTrigger>("trigger", &timer_document(1, "0:0:1.234")).delay());
    assert_eq!(INTERVAL, load::<DelayedShowControlAction>("action", &timer_document(2, "1.234")).delay());
    assert_eq!(Duration::from_secs(30), load::<InactivityTrigger>("trigger", &timer_document(3, "00:00:30")).timeout());
    assert_eq!(Duration::ZERO, load::<VisualDebugBehavior>("behavior", &timer_document(4, "0:0:0")).duration());
}

#[test]
fn a_negative_time_span_literal_for_a_timer_property_is_an_error() {
    let _app = start_app();
    for (index, (property, _)) in TIMER_DOCUMENTS.iter().enumerate() {
        let error = match try_load(&timer_document(index, "-0:0:1")) {
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
fn a_negative_span_from_a_binding_leaves_the_timer_properties_at_their_defaults() {
    let _app = start_app();
    let behavior = DelayedLoadBehavior::new();
    a_negative_span_keeps_the_default(
        &behavior,
        DelayedLoadBehavior::delay_property().as_property(),
        || behavior.delay(),
        Duration::from_millis(500),
    );

    let trigger = DelayedLoadTrigger::new();
    a_negative_span_keeps_the_default(
        &trigger,
        DelayedLoadTrigger::delay_property().as_property(),
        || trigger.delay(),
        Duration::from_millis(500),
    );

    let action = DelayedShowControlAction::new();
    a_negative_span_keeps_the_default(
        &action,
        DelayedShowControlAction::delay_property().as_property(),
        || action.delay(),
        Duration::from_millis(500),
    );

    let trigger = InactivityTrigger::new();
    a_negative_span_keeps_the_default(
        &trigger,
        InactivityTrigger::timeout_property().as_property(),
        || trigger.timeout(),
        Duration::from_secs(5),
    );

    let behavior = VisualDebugBehavior::new();
    a_negative_span_keeps_the_default(
        &behavior,
        VisualDebugBehavior::duration_property().as_property(),
        || behavior.duration(),
        Duration::from_millis(500),
    );
}
