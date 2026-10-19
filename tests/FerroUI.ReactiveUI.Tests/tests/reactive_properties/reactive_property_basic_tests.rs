//! `ReactivePropertyBasicTests` (`ReactiveUI.Tests/ReactiveProperties/ReactivePropertyBasicTests.cs`).
//!
//! Upstream's `Create` factories are the constructors of the port (`with_value`, `with_options`,
//! `with_scheduler`); a nullable string is an `Option<String>`.

use crate::mocks::*;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui::*;
use ferroui_system::component_model::{DataErrorsChangedEventArgs, INotifyDataErrorInfo, INotifyPropertyChanged};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `InitialValue`.
const INITIAL_VALUE: i32 = 42;
/// `UpdatedValue`.
const UPDATED_VALUE: i32 = 100;
/// `RequiredError`.
const REQUIRED_ERROR: &str = "Required";
/// `ValuePropertyName`.
const VALUE_PROPERTY_NAME: &str = "Value";

fn null_string_property() -> ReactiveProperty<Option<String>> {
    ReactiveProperty::with_scheduler(None, Some(immediate()), false, false)
}

fn is_null_or_empty(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(str::is_empty)
}

fn recorder<T: 'static>() -> (Rc<RefCell<Vec<T>>>, impl Fn(T) + 'static) {
    let values = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    (values, move |value| sink.borrow_mut().push(value))
}

#[test]
fn add_validation_error_ignore_initial_error() {
    let rp = null_string_property();
    let _ = rp.add_validation_error_with(|x: Option<String>| if is_null_or_empty(&x) { Some(REQUIRED_ERROR) } else { None }, true);

    // The initial error is ignored.
    assert!(!rp.has_errors());

    // The next ones are detected.
    rp.set_value(Some(String::new()));
    assert!(rp.has_errors());
    rp.dispose();
}

#[test]
fn add_validation_error_with_enumerable_function() {
    let rp = null_string_property();
    let _ = rp.add_validation_error(|x: Option<String>| if is_null_or_empty(&x) { Some(vec![REQUIRED_ERROR]) } else { None });

    assert!(rp.has_errors());

    rp.set_value(Some("test".to_owned()));
    assert!(!rp.has_errors());
    rp.dispose();
}

#[test]
fn add_validation_error_with_observable_function() {
    let rp = null_string_property();
    let _ = rp.add_validation_error_observable(|xs| xs.select(|x: Option<String>| if is_null_or_empty(&x) { Some(REQUIRED_ERROR) } else { None }));

    assert!(rp.has_errors());

    rp.set_value(Some("test".to_owned()));
    assert!(!rp.has_errors());
    rp.dispose();
}

#[test]
fn add_validation_error_with_sync_function() {
    let rp = null_string_property();
    let _ = rp.add_validation_error(|x: Option<String>| if is_null_or_empty(&x) { Some(REQUIRED_ERROR) } else { None });

    assert!(rp.has_errors());

    rp.set_value(Some("test".to_owned()));
    assert!(!rp.has_errors());
    rp.dispose();
}

#[test]
fn allow_duplicate_values_sends_multiple_identical_values() {
    let rp = ReactiveProperty::with_scheduler(0, Some(immediate()), false, true);
    let (values, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    let initial_count = values.borrow().len();

    // The same value.
    rp.set_value(0);
    assert!(values.borrow().len() > initial_count);
    rp.dispose();
}

#[test]
fn check_validation_invokes_validation() {
    let rp = ReactiveProperty::with_value(0);
    // It does not panic.
    rp.check_validation();
    assert_eq!(rp.value(), 0);
    rp.dispose();
}

#[test]
fn constructor_with_initial_value_sets_value() {
    let rp = ReactiveProperty::with_value(INITIAL_VALUE);
    assert_eq!(rp.value(), INITIAL_VALUE);
    rp.dispose();
}

#[test]
fn default_constructor_creates_property_with_null_value() {
    let rp = ReactiveProperty::<Option<String>>::new();
    assert_eq!(rp.value(), None);
    rp.dispose();
}

#[test]
fn dispose_sets_is_disposed() {
    let rp = ReactiveProperty::<i32>::new();
    assert!(!rp.is_disposed());

    rp.dispose();
    assert!(rp.is_disposed());
}

#[test]
fn distinct_until_changed_does_not_send_duplicates() {
    let rp = ReactiveProperty::with_scheduler(0, Some(immediate()), false, false);
    let (values, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    let initial_count = values.borrow().len();

    // The same value notifies nothing.
    rp.set_value(0);
    assert_eq!(values.borrow().len(), initial_count);
    rp.dispose();
}

#[test]
fn errors_changed_event_fires() {
    let rp = null_string_property();
    let fired = Rc::new(Cell::new(false));
    let seen = fired.clone();

    rp.errors_changed().add(Rc::new(move |_: &DataErrorsChangedEventArgs| seen.set(true)));
    let _ = rp.add_validation_error(|x: Option<String>| if is_null_or_empty(&x) { Some(REQUIRED_ERROR) } else { None });

    assert!(fired.get());
    rp.dispose();
}

#[test]
fn get_errors_i_notify_data_error_info_returns_empty_when_no_errors() {
    let rp = ReactiveProperty::<Option<String>>::new();
    // Upstream checks that the list is not null; the list of the port is empty.
    let errors = INotifyDataErrorInfo::get_errors(&rp, Some(VALUE_PROPERTY_NAME));
    assert!(errors.is_empty());
    rp.dispose();
}

#[test]
fn get_errors_returns_null_when_no_errors() {
    let rp = ReactiveProperty::<Option<String>>::new();
    let errors = rp.get_errors(Some(VALUE_PROPERTY_NAME));
    assert!(errors.is_none());
    rp.dispose();
}

#[test]
fn has_errors_initially_false() {
    let rp = ReactiveProperty::<Option<String>>::new();
    assert!(!rp.has_errors());
    rp.dispose();
}

#[test]
fn multiple_subscribers_receive_updates() {
    const SECOND_VALUE: i32 = 2;
    let rp = ReactiveProperty::with_scheduler(0, Some(immediate()), false, false);
    let (values1, add1) = recorder::<i32>();
    let (values2, add2) = recorder::<i32>();

    let _first = rp.subscribe_next(add1);

    rp.set_value(1);

    let _second = rp.subscribe_next(add2);

    rp.set_value(SECOND_VALUE);

    assert!(values1.borrow().contains(&0));
    assert!(values1.borrow().contains(&1));
    assert!(values1.borrow().contains(&SECOND_VALUE));

    // The second subscriber receives the current value when it subscribes.
    assert!(values2.borrow().contains(&1));
    assert!(values2.borrow().contains(&SECOND_VALUE));
    rp.dispose();
}

#[test]
fn multiple_validation_errors_are_combined() {
    const MINIMUM_LENGTH: usize = 3;
    let rp = null_string_property();
    let _ = rp
        .add_validation_error(|x: Option<String>| if is_null_or_empty(&x) { Some(REQUIRED_ERROR) } else { None })
        .add_validation_error(|x: Option<String>| match x {
            Some(text) if text.len() < MINIMUM_LENGTH => Some("Too short"),
            _ => None,
        });

    assert!(rp.has_errors());

    let errors = rp.get_errors(Some(VALUE_PROPERTY_NAME));
    assert!(errors.is_some());
    rp.dispose();
}

#[test]
fn observe_error_changed_emits_errors() {
    let rp = null_string_property();
    let (errors, add) = recorder::<Option<DataErrors>>();
    let _subscription = rp.observe_error_changed().subscribe_next(add);

    let _ = rp.add_validation_error(|x: Option<String>| if is_null_or_empty(&x) { Some(REQUIRED_ERROR) } else { None });

    assert!(!errors.borrow().is_empty());
    rp.dispose();
}

#[test]
fn observe_has_errors_emits_error_state() {
    let rp = null_string_property();
    let (has_errors_values, add) = recorder::<bool>();
    let _subscription = rp.observe_has_errors().subscribe_next(add);

    let _ = rp.add_validation_error(|x: Option<String>| if is_null_or_empty(&x) { Some(REQUIRED_ERROR) } else { None });

    assert!(has_errors_values.borrow().contains(&true));

    rp.set_value(Some("test".to_owned()));
    assert!(has_errors_values.borrow().contains(&false));
    rp.dispose();
}

#[test]
fn property_changed_event_fires() {
    let rp = ReactiveProperty::with_value(0);
    let fired = Rc::new(Cell::new(false));
    let property_name = Rc::new(RefCell::new(String::new()));

    let (seen, name) = (fired.clone(), property_name.clone());
    INotifyPropertyChanged::property_changed(&rp).add(Rc::new(move |args: &str| {
        seen.set(true);
        *name.borrow_mut() = args.to_owned();
    }));

    rp.set_value(INITIAL_VALUE);

    assert!(fired.get());
    assert_eq!(*property_name.borrow(), "Value");
    rp.dispose();
}

#[test]
fn refresh_sends_current_value_even_if_unchanged() {
    let rp = ReactiveProperty::with_scheduler(INITIAL_VALUE, Some(immediate()), false, false);
    let (values, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    let count_before = values.borrow().len();

    rp.refresh();
    assert!(values.borrow().len() > count_before);
    assert_eq!(values.borrow().last().copied(), Some(INITIAL_VALUE));
    rp.dispose();
}

#[test]
fn scheduler_is_used_for_notifications() {
    let scheduler = TestScheduler::new();
    let rp = ReactiveProperty::with_scheduler(0, Some(scheduler.scheduler()), false, false);
    let (values, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    // No value arrives before the scheduler runs.
    assert!(values.borrow().is_empty());

    scheduler.run();
    assert!(values.borrow().contains(&0));
    rp.dispose();
}

#[test]
fn skip_current_value_on_subscribe() {
    let rp = ReactiveProperty::with_scheduler(INITIAL_VALUE, Some(immediate()), true, false);
    let (values, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    // The initial value does not arrive.
    assert!(values.borrow().is_empty());

    rp.set_value(UPDATED_VALUE);
    assert!(values.borrow().contains(&UPDATED_VALUE));
    rp.dispose();
}

#[test]
fn static_create_methods_work() {
    let rp1 = ReactiveProperty::<i32>::new();
    assert!(!rp1.is_disposed());

    let rp2 = ReactiveProperty::with_value(INITIAL_VALUE);
    assert_eq!(rp2.value(), INITIAL_VALUE);

    let rp3 = ReactiveProperty::with_options(INITIAL_VALUE, true, false);
    assert_eq!(rp3.value(), INITIAL_VALUE);

    let rp4 = ReactiveProperty::with_scheduler(INITIAL_VALUE, Some(immediate()), false, false);
    assert_eq!(rp4.value(), INITIAL_VALUE);

    for rp in [rp1, rp2, rp3, rp4] {
        rp.dispose();
    }
}

#[test]
fn subscribe_after_dispose_completes_immediately() {
    let rp = ReactiveProperty::with_value(INITIAL_VALUE);
    rp.dispose();

    let completed = Rc::new(Cell::new(false));
    let seen = completed.clone();
    let _subscription = rp.subscribe_next_completed(|_| {}, move || seen.set(true));

    assert!(completed.get());
}

#[test]
fn subscribe_receives_current_value() {
    let rp = ReactiveProperty::with_scheduler(INITIAL_VALUE, Some(immediate()), false, false);
    let received = Rc::new(Cell::new(0));
    let sink = received.clone();
    let _subscription = rp.subscribe_next(move |x| sink.set(x));

    assert_eq!(received.get(), INITIAL_VALUE);
    rp.dispose();
}

#[test]
fn subscribe_receives_value_changes() {
    const SECOND_VALUE: i32 = 2;
    let rp = ReactiveProperty::with_scheduler(0, Some(immediate()), false, false);
    let (values, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    rp.set_value(1);
    rp.set_value(SECOND_VALUE);

    assert!(values.borrow().contains(&0));
    assert!(values.borrow().contains(&1));
    assert!(values.borrow().contains(&SECOND_VALUE));
    rp.dispose();
}

// Not ported: SubscribeWithNullObserverReturnsEmptyDisposable - argument validation

#[test]
fn value_property_getter_returns_current_value() {
    let rp = ReactiveProperty::with_value(Some("test".to_owned()));
    assert_eq!(rp.value().as_deref(), Some("test"));
    rp.dispose();
}

#[test]
fn value_property_setter_updates_value() {
    let rp = ReactiveProperty::<Option<String>>::new();
    rp.set_value(Some("new value".to_owned()));
    assert_eq!(rp.value().as_deref(), Some("new value"));
    rp.dispose();
}

#[test]
fn parameterless_constructor_creates_default_value() {
    let rp = ReactiveProperty::<i32>::new();
    assert_eq!(rp.value(), 0);
    rp.dispose();
}

#[test]
fn single_value_constructor_stores_value() {
    let rp = ReactiveProperty::with_value(INITIAL_VALUE);
    assert_eq!(rp.value(), INITIAL_VALUE);
    rp.dispose();
}

#[test]
fn value_skip_duplicate_constructor_stores_value_and_skips() {
    let rp = ReactiveProperty::with_options(INITIAL_VALUE, true, false);
    let (values, add) = recorder::<i32>();
    let _subscription = rp.subscribe_next(add);

    assert_eq!(rp.value(), INITIAL_VALUE);
    // Skipping the current value means no first notification.
    assert!(values.borrow().is_empty());
    rp.dispose();
}

#[test]
fn dispose_twice_is_no_op() {
    let rp = ReactiveProperty::with_value(INITIAL_VALUE);

    rp.dispose();
    // The second call returns early.
    rp.dispose();

    assert!(rp.is_disposed());
}

#[test]
fn set_value_after_dispose_stores_value_without_notifying() {
    let rp = ReactiveProperty::with_value(INITIAL_VALUE);
    rp.dispose();

    // The value is stored, and nothing is notified.
    rp.set_value(UPDATED_VALUE);

    assert_eq!(rp.value(), UPDATED_VALUE);
}
