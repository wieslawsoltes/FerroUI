//! `OptionObservableFixture` (`DynamicData.Tests/Kernal/OptionObservableFixture.cs`): the
//! operators of sequences of options.
//!
//! `ToObservable()` of an enumerable is `Observable::to_observable`, whose values arrive while the
//! subscription is made; `ToEnumerable()` is [`to_list`], which collects them.

use ferroui_dynamic_data::kernel::OptionObservableExtensions;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;

const NONE_COUNT: usize = 5;
const SOME_COUNT: usize = 10;

fn not_convertable_to_int() -> Option<String> {
    Some("NOT AN INT".to_string())
}

fn int_enum() -> Vec<i32> {
    (0..SOME_COUNT as i32).collect()
}

fn string_enum() -> Vec<String> {
    int_enum().iter().map(|n| n.to_string()).collect()
}

fn opt_int_enum() -> Vec<Option<i32>> {
    int_enum().into_iter().map(Some).collect()
}

fn opt_none_int_enum() -> Vec<Option<i32>> {
    vec![None; NONE_COUNT]
}

fn opt_none_string_enum() -> Vec<Option<String>> {
    vec![None; NONE_COUNT]
}

fn opt_string_enum() -> Vec<Option<String>> {
    string_enum().into_iter().map(Some).collect()
}

fn opt_string_with_none_enum() -> Vec<Option<String>> {
    opt_none_string_enum().into_iter().chain(opt_string_enum()).collect()
}

fn opt_string_with_bad_enum() -> Vec<Option<String>> {
    std::iter::once(not_convertable_to_int()).chain(opt_string_enum()).collect()
}

fn opt_string_with_bad_and_none_enum() -> Vec<Option<String>> {
    std::iter::once(not_convertable_to_int()).chain(opt_string_with_none_enum()).collect()
}

fn parse_int_opt(input: String) -> Option<i32> {
    input.parse::<i32>().ok()
}

/// `int.Parse`: a string that is not a number fails.
fn parse_int(input: String) -> i32 {
    input.parse::<i32>().expect("the string is a number")
}

/// The values of a sequence that ends while it is subscribed (`ToEnumerable().ToList()`).
fn to_list<T: Clone + 'static>(observable: Rc<dyn IObservable<T>>) -> Vec<T> {
    let values = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    let subscription = observable.subscribe_next(move |value: T| sink.borrow_mut().push(value));
    subscription.dispose();
    values.take()
}

/// `subset.Should().BeSubsetOf(superset)`: every item of `subset` is in `superset`.
fn assert_subset_of<T: PartialEq + Debug>(subset: &[T], superset: &[T]) {
    for item in subset {
        assert!(superset.contains(item), "expected {subset:?} to be a subset of {superset:?}, but {item:?} is not in it");
    }
}

// Not ported: NullChecks - argument validation

#[test]
fn convert_will_convert_values() {
    // having
    let observable = Observable::to_observable(opt_string_enum());

    // when
    let results = to_list(observable.convert(parse_int));
    let int_list = opt_int_enum();

    // then
    assert_subset_of(&results, &int_list);
    assert_subset_of(&int_list, &results);
}

#[test]
fn convert_preserves_none() {
    // having
    let enumerable = opt_string_with_none_enum();
    let observable = Observable::to_observable(enumerable.clone());

    // when
    let results = to_list(observable.convert(parse_int).where_(|opt: &Option<i32>| opt.is_none())).len();
    let expected = enumerable.iter().filter(|opt| opt.is_none()).count();

    // then
    assert_eq!(results, expected);
    assert_eq!(results, NONE_COUNT);
}

#[test]
fn convert_optional_will_convert_values() {
    // having
    let observable = Observable::to_observable(opt_string_with_bad_enum());

    // when
    let results = to_list(observable.convert_optional(parse_int_opt));
    let int_list = opt_int_enum();

    // then
    assert_subset_of(&int_list, &results);
    assert!(results.contains(&None));
}

#[test]
fn convert_optional_preserves_none() {
    // having
    let enumerable = opt_string_with_bad_and_none_enum();
    let observable = Observable::to_observable(enumerable);

    // when
    let results = to_list(observable.convert_optional(parse_int_opt).where_(|opt: &Option<i32>| opt.is_none())).len();
    let expected = opt_string_with_none_enum().iter().filter(|opt| opt.is_none()).count() + 1;

    // then
    assert_eq!(results, expected);
    assert!(results > 1);
}

#[test]
fn convert_or_converts_or_fallsback() {
    // having
    let observable = Observable::to_observable(opt_string_with_none_enum());

    // when
    let results = to_list(observable.convert_or(parse_int, || -1));
    let int_list: Vec<i32> = std::iter::once(-1).chain(int_enum()).collect();

    // then
    assert_subset_of(&results, &int_list);
    assert_subset_of(&int_list, &results);
}

#[test]
fn or_else_fallsback() {
    // having
    let observable = Observable::to_observable(opt_int_enum()).start_with([None]);

    // when
    let results = to_list(observable.or_else(|| Some(-1)));
    let int_list: Vec<Option<i32>> = std::iter::once(Some(-1)).chain(opt_int_enum()).collect();

    // then
    assert_subset_of(&results, &int_list);
    assert_subset_of(&int_list, &results);
}

#[test]
fn on_has_value_invokes_correct_action() {
    // having
    let value = Rc::new(Cell::new(0_usize));
    let no_value = Rc::new(Cell::new(0_usize));
    let on_val = {
        let value = value.clone();
        move |_: &i32| value.set(value.get() + 1)
    };
    let on_no_val = {
        let no_value = no_value.clone();
        move || no_value.set(no_value.get() + 1)
    };
    let source: Vec<Option<i32>> = opt_int_enum().into_iter().chain(opt_none_int_enum()).collect();
    let observable = Observable::to_observable(source).on_has_value_with_else(on_val, on_no_val);

    // when
    let _results = to_list(observable);

    // then
    assert_eq!(value.get(), SOME_COUNT);
    assert_eq!(no_value.get(), NONE_COUNT);
}

#[test]
fn on_has_no_value_invokes_correct_action() {
    // having
    let value = Rc::new(Cell::new(0_usize));
    let no_value = Rc::new(Cell::new(0_usize));
    let on_val = {
        let value = value.clone();
        move |_: &i32| value.set(value.get() + 1)
    };
    let on_no_val = {
        let no_value = no_value.clone();
        move || no_value.set(no_value.get() + 1)
    };
    let source: Vec<Option<i32>> = opt_int_enum().into_iter().chain(opt_none_int_enum()).collect();
    let observable = Observable::to_observable(source).on_has_no_value_with_else(on_no_val, on_val);

    // when
    let _results = to_list(observable);

    // then
    assert_eq!(value.get(), SOME_COUNT);
    assert_eq!(no_value.get(), NONE_COUNT);
}

#[test]
fn select_values_returns_the_values() {
    // having
    let enumerable: Vec<Option<i32>> = opt_int_enum().into_iter().chain(opt_none_int_enum()).collect();
    let observable = Observable::to_observable(enumerable.clone()).select_values();

    // when
    let expected = enumerable.iter().filter(|opt| opt.is_some()).count();
    let results = to_list(observable).len();

    // then
    assert_eq!(expected, results);
    assert_eq!(results, SOME_COUNT);
}

#[test]
fn value_or_invokes_selector() {
    // having
    let invoke_count = Rc::new(Cell::new(0_usize));
    let selector = {
        let invoke_count = invoke_count.clone();
        move || {
            invoke_count.set(invoke_count.get() + 1);
            -1
        }
    };
    let enumerable: Vec<Option<i32>> = opt_int_enum().into_iter().chain(opt_none_int_enum()).collect();
    let observable = Observable::to_observable(enumerable.clone()).value_or(selector);

    // when
    let expected = enumerable.iter().filter(|opt| opt.is_none()).count();
    let results = to_list(observable).into_iter().filter(|i| *i == -1).count();

    // then
    assert_eq!(expected, results);
    assert_eq!(results, NONE_COUNT);
    assert_eq!(invoke_count.get(), NONE_COUNT);
}

#[test]
fn value_or_default_returns_default_values() {
    // having
    let enumerable = opt_string_with_none_enum();
    let observable = Observable::to_observable(enumerable.clone()).value_or_default();

    // when
    let expected = enumerable.iter().filter(|opt| opt.is_none()).count();
    // The default of `String` is the empty string, where upstream's `default(string)` is null; no
    // value of the sequence is empty.
    let results = to_list(observable).into_iter().filter(|value| *value == String::default()).count();

    // then
    assert_eq!(expected, results);
    assert_eq!(results, NONE_COUNT);
}

#[test]
fn value_or_throw_fails_with_generated_error() {
    // having
    let expected_error: ObservableError = Rc::new(std::io::Error::other("Nope"));
    let exception_generator = {
        let expected_error = expected_error.clone();
        move || expected_error.clone()
    };
    let enumerable = opt_string_with_none_enum();
    let observable = Observable::to_observable(enumerable).value_or_throw(exception_generator);
    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    // when
    let sink = received_error.clone();
    let cleanup = observable.subscribe_next_error(|_| {}, move |err| *sink.borrow_mut() = Some(err));

    // then
    let received = received_error.borrow().clone().expect("the sequence failed");
    assert!(Rc::ptr_eq(&received, &expected_error));
    cleanup.dispose();
}
