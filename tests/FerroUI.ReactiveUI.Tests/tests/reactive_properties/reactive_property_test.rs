//! `ReactivePropertyTest` (`ReactiveUI.Tests/ReactiveProperties/ReactivePropertyTest.cs`).
//!
//! `IEnumerable` errors are [`DataErrors`], read with [`string_errors`] (`OfType<string>()`); a
//! nullable string is an `Option<String>`.

use crate::mocks::*;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_reactiveui::*;
use ferroui_system::component_model::DataErrorsChangedEventArgs;
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// `ValuePropertyName`.
const VALUE_PROPERTY_NAME: &str = "Value";
/// `ErrorValue`.
const ERROR_VALUE: &str = "Error";
/// `ErrorLowerValue`.
const ERROR_LOWER_VALUE: &str = "error";
/// `ReactiveUiValue`.
const REACTIVE_UI_VALUE: &str = "ReactiveUI";
/// `ReactiveUiSecondValue`.
const REACTIVE_UI_SECOND_VALUE: &str = "ReactiveUI 2";
/// `RequiredErrorValue`.
const REQUIRED_ERROR_VALUE: &str = "required";
/// `ExclaimErrorValue`.
const EXCLAIM_ERROR_VALUE: &str = "error!";

fn text(value: &str) -> Option<String> {
    Some(value.to_owned())
}

fn null_string_property() -> ReactiveProperty<Option<String>> {
    ReactiveProperty::with_scheduler(None, Some(immediate()), false, false)
}

fn recorder<T: 'static>() -> (Rc<RefCell<Vec<T>>>, impl Fn(T) + 'static) {
    let values = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    (values, move |value| sink.borrow_mut().push(value))
}

#[test]
fn check_validation() {
    let min_value = Rc::new(Cell::new(0));
    let minimum = min_value.clone();
    let rp = ReactiveProperty::with_scheduler(0, Some(immediate()), false, false)
        .add_validation_error(move |x: i32| if x < minimum.get() { Some(ERROR_VALUE) } else { None });
    assert!(rp.get_errors(Some(VALUE_PROPERTY_NAME)).is_none());

    min_value.set(1);
    assert!(rp.get_errors(Some(VALUE_PROPERTY_NAME)).is_none());

    rp.check_validation();
    assert_eq!(string_errors(&rp.get_errors(Some(VALUE_PROPERTY_NAME))), vec![ERROR_VALUE.to_owned()]);
    rp.dispose();
}

#[test]
fn default_value_is_raised_on_subscribe() {
    let rp = null_string_property();
    assert_eq!(rp.value(), None);
    let received_value = Rc::new(Cell::new(false));
    let received = received_value.clone();
    let _subscription = rp.subscribe_next(move |_| received.set(true));

    assert!(received_value.get());
    rp.dispose();
}

#[test]
fn errors_changed_event_is_raised() {
    let rp = null_string_property()
        .add_validation_error(|x: Option<String>| if x.as_deref().is_none_or(str::is_empty) { Some(ERROR_LOWER_VALUE) } else { None });

    let event_args: Rc<RefCell<Option<DataErrorsChangedEventArgs>>> = Rc::new(RefCell::new(None));
    let seen = event_args.clone();
    rp.errors_changed().add(Rc::new(move |args: &DataErrorsChangedEventArgs| *seen.borrow_mut() = Some(args.clone())));

    // Upstream waits 10 milliseconds for the errors to propagate; the immediate scheduler raises
    // the event at once.
    rp.set_value(text("valid"));

    assert!(event_args.borrow().is_some());
    rp.dispose();
}

#[test]
fn ignore_init_error_and_update_value() {
    let rp = null_string_property().add_validation_error_with(
        |x: Option<String>| if x.as_deref().is_none_or(str::is_empty) { Some(ERROR_LOWER_VALUE) } else { None },
        true,
    );

    assert!(!rp.has_errors());
    rp.set_value(text(""));
    assert!(rp.has_errors());
    rp.dispose();
}

#[test]
fn ignore_initial_error_and_check_validation() {
    let rp = null_string_property().add_validation_error_with(
        |x: Option<String>| if x.as_deref().is_none_or(str::is_empty) { Some(ERROR_LOWER_VALUE) } else { None },
        true,
    );

    assert!(!rp.has_errors());
    rp.check_validation();
    assert!(rp.has_errors());
    rp.dispose();
}

#[test]
fn initial_value() {
    let rp = ReactiveProperty::with_scheduler(text(REACTIVE_UI_VALUE), Some(immediate()), false, false);
    assert_eq!(rp.value(), text(REACTIVE_UI_VALUE));
    let received = Rc::new(RefCell::new(None));
    let sink = received.clone();
    let _subscription = rp.subscribe_next(move |x| *sink.borrow_mut() = x);
    assert_eq!(*received.borrow(), text(REACTIVE_UI_VALUE));
    rp.dispose();
}

#[test]
fn initial_value_skip_current() {
    let rp = ReactiveProperty::with_scheduler(text(REACTIVE_UI_VALUE), Some(immediate()), true, false);
    assert_eq!(rp.value(), text(REACTIVE_UI_VALUE));

    // The current value is skipped.
    let received = Rc::new(RefCell::new(None));
    let sink = received.clone();
    let _subscription = rp.subscribe_next(move |x| *sink.borrow_mut() = x);
    rp.set_value(text(REACTIVE_UI_SECOND_VALUE));
    assert_eq!(*received.borrow(), text(REACTIVE_UI_SECOND_VALUE));
    assert_eq!(rp.value(), text(REACTIVE_UI_SECOND_VALUE));
    rp.dispose();
}

#[test]
fn multiple_subscribers_get_current_value() {
    const THIRD_VALUE: i32 = 2;
    const FOURTH_VALUE: i32 = 3;
    let rp = ReactiveProperty::with_scheduler(0, Some(immediate()), false, false);
    let (collector1, add1) = recorder::<i32>();
    let (collector2, add2) = recorder::<i32>();
    let obs = rp.clone();
    let _first = obs.subscribe_next(add1);

    assert_eq!(rp.value(), 0);
    assert_eq!(*collector1.borrow(), vec![0]);

    rp.set_value(1);
    assert_eq!(rp.value(), 1);
    assert_eq!(*collector1.borrow(), vec![0, 1]);

    rp.set_value(THIRD_VALUE);
    assert_eq!(rp.value(), THIRD_VALUE);
    assert_eq!(*collector1.borrow(), vec![0, 1, THIRD_VALUE]);

    // A second subscriber.
    let _second = obs.subscribe_next(add2);
    assert_eq!(rp.value(), THIRD_VALUE);
    assert_eq!(*collector2.borrow(), vec![THIRD_VALUE]);

    rp.set_value(FOURTH_VALUE);
    assert_eq!(rp.value(), FOURTH_VALUE);
    assert_eq!(*collector1.borrow(), vec![0, 1, THIRD_VALUE, FOURTH_VALUE]);
    assert_eq!(*collector2.borrow(), vec![THIRD_VALUE, FOURTH_VALUE]);
    rp.dispose();
}

#[test]
fn observe_errors() {
    let rp = null_string_property().add_validation_error(|x: Option<String>| if x.is_none() { Some(ERROR_VALUE) } else { None });

    const EXPECTED_EMISSION_COUNT: usize = 2;
    let (results, add) = recorder::<Option<DataErrors>>();
    let _subscription = rp.observe_error_changed().subscribe_next(add);
    rp.set_value(text("OK"));

    let results = results.borrow();
    assert_eq!(results.len(), EXPECTED_EMISSION_COUNT);
    assert_eq!(string_errors(&results[0]), vec![ERROR_VALUE.to_owned()]);
    assert!(results[1].is_none());
    rp.dispose();
}

#[test]
fn observe_has_error() {
    let rp = null_string_property().add_validation_error(|x: Option<String>| if x.is_none() { Some(ERROR_VALUE) } else { None });

    const EXPECTED_EMISSION_COUNT: usize = 2;
    let (results, add) = recorder::<bool>();
    let _subscription = rp.observe_has_errors().subscribe_next(add);
    rp.set_value(text("OK"));

    assert_eq!(results.borrow().len(), EXPECTED_EMISSION_COUNT);
    assert!(results.borrow()[0]);
    assert!(!results.borrow()[1]);
    rp.dispose();
}

#[test]
fn observe_validation_errors_handles_multiple_errors() {
    let target = ReactivePropertyVm::new();
    let (errors, add) = recorder::<Option<String>>();
    let _subscription = target.length_less_than_five_property.observe_validation_errors().subscribe_next(add);

    assert_eq!(errors.borrow().len(), 1);
    assert_eq!(errors.borrow()[0], text(REQUIRED_ERROR_VALUE));

    target.length_less_than_five_property.set_value(text("ok"));
    assert_eq!(errors.borrow().last().cloned().flatten(), None);

    target.length_less_than_five_property.set_value(text("toolong"));
    assert_eq!(errors.borrow().last().cloned().flatten(), text("5over"));
}

#[test]
fn observe_validation_errors_returns_error_messages() {
    let target = ReactivePropertyVm::new();
    let (errors, add) = recorder::<Option<String>>();
    let _subscription = target.is_required_property.observe_validation_errors().subscribe_next(add);

    const AFTER_VALID_COUNT: usize = 2;
    const AFTER_CLEARED_COUNT: usize = 3;
    assert_eq!(errors.borrow().len(), 1);
    assert_eq!(errors.borrow()[0], text(EXCLAIM_ERROR_VALUE));

    target.is_required_property.set_value(text("valid"));
    assert_eq!(errors.borrow().len(), AFTER_VALID_COUNT);
    assert_eq!(errors.borrow()[1], None);

    target.is_required_property.set_value(None);
    assert_eq!(errors.borrow().len(), AFTER_CLEARED_COUNT);
    assert_eq!(errors.borrow().last().cloned().flatten(), text(EXCLAIM_ERROR_VALUE));
}

// Not ported: ObserveValidationErrors_ThrowsOnNull - argument validation

#[test]
fn refresh() {
    let rp = ReactiveProperty::with_scheduler(0, Some(immediate()), false, false);
    let (collector, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    assert_eq!(*collector.borrow(), vec![0]);

    // A refresh always produces a value, even an equal one, when duplicates are not allowed.
    rp.refresh();
    assert_eq!(*collector.borrow(), vec![0, 0]);
    rp.dispose();
}

#[test]
fn set_value_raises_events() {
    let rp = null_string_property();
    assert_eq!(rp.value(), None);
    rp.set_value(text(REACTIVE_UI_VALUE));
    assert_eq!(rp.value(), text(REACTIVE_UI_VALUE));
    let received = Rc::new(RefCell::new(None));
    let sink = received.clone();
    let _subscription = rp.subscribe_next(move |x| *sink.borrow_mut() = x);
    assert_eq!(*received.borrow(), text(REACTIVE_UI_VALUE));
    rp.dispose();
}

// Not ported: Subscribe_WithNullObserver_ReturnsEmptyDisposable - argument validation

#[test]
fn test_multiple_substribers() {
    const SUBSCRIBER_COUNT: usize = 1_000;
    const MAX_STARTUP_TIME_MS: u128 = 2_000;
    let vm = SubcribeTestViewModel::new(SUBSCRIBER_COUNT);
    assert_eq!(vm.subscriber_count, SUBSCRIBER_COUNT);
    assert!(vm.startup_time < MAX_STARTUP_TIME_MS);
    assert_eq!(vm.subscriber_events, SUBSCRIBER_COUNT);
    vm.dispose();
}

#[test]
fn validation_error_changed_test() {
    let (errors, add) = recorder::<Option<DataErrors>>();
    let rprop = null_string_property()
        .add_validation_error(|x: Option<String>| if x.as_deref().is_none_or(|x| x.trim().is_empty()) { Some(ERROR_LOWER_VALUE) } else { None });

    // The former behaviour.
    let _subscription = rprop.observe_error_changed().skip(1).subscribe_next(add);

    assert_eq!(errors.borrow().len(), 0);

    rprop.set_value(text("OK"));
    assert_eq!(errors.borrow().len(), 1);
    assert!(errors.borrow().last().unwrap().is_none());

    const AFTER_REVALIDATION_COUNT: usize = 2;
    rprop.set_value(None);
    assert_eq!(errors.borrow().len(), AFTER_REVALIDATION_COUNT);
    assert_eq!(string_errors(errors.borrow().last().unwrap()), vec![ERROR_LOWER_VALUE.to_owned()]);
    rprop.dispose();
}

#[test]
fn validation_ignore_initial_error_and_refresh() {
    let rp = null_string_property().add_validation_error_with(
        |x: Option<String>| if x.as_deref().is_none_or(str::is_empty) { Some(ERROR_LOWER_VALUE) } else { None },
        true,
    );

    assert!(!rp.has_errors());
    rp.refresh();
    assert!(rp.has_errors());
    rp.dispose();
}

#[test]
fn validation_is_required_is_correctly_handled() {
    let target = ReactivePropertyVm::new();
    let (errors, add) = recorder::<Option<DataErrors>>();
    let _subscription = target.is_required_property.observe_error_changed().where_(|x| x.is_some()).subscribe_next(add);

    assert_eq!(errors.borrow().len(), 1);
    assert_eq!(string_errors(&errors.borrow()[0]), vec![EXCLAIM_ERROR_VALUE.to_owned()]);
    assert!(target.is_required_property.has_errors());

    target.is_required_property.set_value(text("a"));
    assert_eq!(errors.borrow().len(), 1);
    assert!(!target.is_required_property.has_errors());

    const AFTER_REVALIDATION_ERROR_COUNT: usize = 2;
    target.is_required_property.set_value(None);
    assert_eq!(errors.borrow().len(), AFTER_REVALIDATION_ERROR_COUNT);
    assert_eq!(string_errors(&errors.borrow()[1]), vec![EXCLAIM_ERROR_VALUE.to_owned()]);
    assert!(target.is_required_property.has_errors());
}

#[test]
fn validation_length_is_correctly_handled() {
    let target = ReactivePropertyVm::new();
    let error: Rc<RefCell<Option<DataErrors>>> = Rc::new(RefCell::new(None));
    let sink = error.clone();
    let _subscription = target.length_less_than_five_property.observe_error_changed().subscribe_next(move |x| *sink.borrow_mut() = x);

    assert!(target.length_less_than_five_property.has_errors());
    assert_eq!(string_errors(&error.borrow()).first().map(String::as_str), Some(REQUIRED_ERROR_VALUE));

    target.length_less_than_five_property.set_value(text("a"));
    assert!(!target.length_less_than_five_property.has_errors());
    assert!(error.borrow().is_none());

    target.length_less_than_five_property.set_value(text("aaaaaa"));
    assert!(target.length_less_than_five_property.has_errors());
    assert!(error.borrow().is_some());
    assert_eq!(string_errors(&error.borrow()).first().map(String::as_str), Some("5over"));

    target.length_less_than_five_property.set_value(None);
    assert!(target.length_less_than_five_property.has_errors());
    assert_eq!(string_errors(&error.borrow()).first().map(String::as_str), Some(REQUIRED_ERROR_VALUE));
}

#[test]
fn validation_task_test() {
    let target = ReactivePropertyVm::new();
    let (errors, add) = recorder::<Option<DataErrors>>();
    let _subscription = target.task_validation_test_property.observe_error_changed().where_(|x| x.is_some()).subscribe_next(add);
    assert_eq!(errors.borrow().len(), 1);
    assert_eq!(string_errors(&errors.borrow()[0]), vec![REQUIRED_ERROR_VALUE.to_owned()]);

    target.task_validation_test_property.set_value(text("a"));
    assert!(!target.task_validation_test_property.has_errors());
    assert_eq!(errors.borrow().len(), 1);

    const AFTER_REVALIDATION_ERROR_COUNT: usize = 2;
    target.task_validation_test_property.set_value(None);
    assert!(target.task_validation_test_property.has_errors());
    assert_eq!(errors.borrow().len(), AFTER_REVALIDATION_ERROR_COUNT);
}

#[test]
fn validation_with_async_failed_case() {
    const ERROR_MESSAGE: &str = "error occured!!";
    let rp = null_string_property()
        .add_validation_error(|x: Option<String>| if x.as_deref().is_none_or(str::is_empty) { None } else { Some(ERROR_MESSAGE) });

    let error: Rc<RefCell<Option<DataErrors>>> = Rc::new(RefCell::new(None));
    let sink = error.clone();
    let _subscription = rp.observe_error_changed().subscribe_next(move |x| *sink.borrow_mut() = x);

    assert!(!rp.has_errors());
    assert!(error.borrow().is_none());

    // Push a value that the validation rejects.
    rp.set_value(text("dummy"));

    assert!(rp.has_errors());
    assert!(error.borrow().is_some());
    assert_eq!(string_errors(&error.borrow()), vec![ERROR_MESSAGE.to_owned()]);
    assert_eq!(string_errors(&rp.get_errors(Some(VALUE_PROPERTY_NAME))), vec![ERROR_MESSAGE.to_owned()]);
    rp.dispose();
}

#[test]
fn validation_with_async_success_case() {
    let tcs = TaskCompletionSource::<Option<String>>::new();
    let task = tcs.task();
    let rp = null_string_property().add_validation_error_async(move |_: Option<String>| -> Task<Option<String>> { task.clone() });

    let error: Rc<RefCell<Option<DataErrors>>> = Rc::new(RefCell::new(None));
    let sink = error.clone();
    let _subscription = rp.observe_error_changed().subscribe_next(move |x| *sink.borrow_mut() = x);

    assert!(!rp.has_errors());
    assert!(error.borrow().is_none());

    rp.set_value(text("dummy"));
    tcs.set_result(None);

    assert!(!rp.has_errors());
    assert!(error.borrow().is_none());
    rp.dispose();
}

#[test]
fn validation_with_async_throttle_test() {
    const FIRST_ADVANCE_MS: u64 = 300;
    const SECOND_ADVANCE_MS: u64 = 700;
    const THIRD_ADVANCE_MS: u64 = 1_100;
    const FINAL_ADVANCE_MS: u64 = 2_500;
    let test_scheduler = TestScheduler::new();
    let scheduler = test_scheduler.scheduler();
    let throttle_scheduler = scheduler.clone();
    let rp = ReactiveProperty::<Option<String>>::with_scheduler(None, Some(scheduler), false, false).add_validation_error_observable(
        move |xs| {
            xs.throttle_on(Duration::from_secs(1), throttle_scheduler.clone())
                .select(|x: Option<String>| if x.as_deref().is_none_or(str::is_empty) { Some(REQUIRED_ERROR_VALUE) } else { None })
        },
    );

    let error: Rc<RefCell<Option<DataErrors>>> = Rc::new(RefCell::new(None));
    let sink = error.clone();
    let _subscription = rp.observe_error_changed().subscribe_next(move |x| *sink.borrow_mut() = x);

    let at = |milliseconds: u64| duration_to_ticks(Duration::from_millis(milliseconds));

    test_scheduler.advance_to(at(0));
    rp.set_value(text(""));
    assert!(!rp.has_errors());
    assert!(error.borrow().is_none());

    test_scheduler.advance_to(at(FIRST_ADVANCE_MS));
    rp.set_value(text("a"));
    assert!(!rp.has_errors());
    assert!(error.borrow().is_none());

    test_scheduler.advance_to(at(SECOND_ADVANCE_MS));
    rp.set_value(text("b"));
    assert!(!rp.has_errors());
    assert!(error.borrow().is_none());

    test_scheduler.advance_to(at(THIRD_ADVANCE_MS));
    rp.set_value(text(""));
    assert!(!rp.has_errors());
    assert!(error.borrow().is_none());

    test_scheduler.advance_to(at(FINAL_ADVANCE_MS));
    assert!(rp.has_errors());
    assert!(error.borrow().is_some());
    assert_eq!(string_errors(&error.borrow()), vec![REQUIRED_ERROR_VALUE.to_owned()]);
    rp.dispose();
}

// Not ported: ValidationWithCustomErrorMessage - reflection (the DataAnnotations attributes and their messages)
// Not ported: ValidationWithCustomErrorMessageWithDisplayName - reflection (the DataAnnotations attributes and their messages)
// Not ported: ValidationWithCustomErrorMessageWithResource - reflection (the DataAnnotations attributes and their messages)

#[test]
fn value_updates_multiple_times_with_different_values() {
    const THIRD_VALUE: i32 = 2;
    const FOURTH_VALUE: i32 = 3;
    let rp = ReactiveProperty::with_scheduler(0, Some(immediate()), false, false);
    let (collector, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    assert_eq!(rp.value(), 0);
    assert_eq!(*collector.borrow(), vec![0]);

    rp.set_value(1);
    assert_eq!(rp.value(), 1);
    assert_eq!(*collector.borrow(), vec![0, 1]);

    rp.set_value(THIRD_VALUE);
    assert_eq!(rp.value(), THIRD_VALUE);
    assert_eq!(*collector.borrow(), vec![0, 1, THIRD_VALUE]);

    rp.set_value(FOURTH_VALUE);
    assert_eq!(rp.value(), FOURTH_VALUE);
    assert_eq!(*collector.borrow(), vec![0, 1, THIRD_VALUE, FOURTH_VALUE]);
    rp.dispose();
}

#[test]
fn value_updates_multiple_times_with_same_values() {
    let rp = ReactiveProperty::with_scheduler(0, Some(immediate()), false, true);
    let (collector, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    assert_eq!(rp.value(), 0);
    assert_eq!(*collector.borrow(), vec![0]);

    rp.set_value(0);
    assert_eq!(rp.value(), 0);
    assert_eq!(*collector.borrow(), vec![0, 0]);

    rp.set_value(0);
    assert_eq!(rp.value(), 0);
    assert_eq!(*collector.borrow(), vec![0, 0, 0]);

    rp.set_value(0);
    assert_eq!(rp.value(), 0);
    assert_eq!(*collector.borrow(), vec![0, 0, 0, 0]);
    rp.dispose();
}
