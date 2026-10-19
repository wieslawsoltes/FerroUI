//! `ObservableAsPropertyHelperTests` (`ReactiveUI.Binding.Tests/Bindings/ObservableAsPropertyHelperTests.cs`).
//!
//! The partial files of the class are the modules `observable_as_property_helper_tests_*`; the
//! constants and helpers they share are here. Upstream's `ManualSequencer` (a scheduler that runs
//! its work when the test says so) is a `TestScheduler`. The constructors of upstream are the
//! named constructors of the port (`with_initial_value`, `with_options`, ...).

use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::Observable;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The initial value the tests supply.
pub(super) const SUPPLIED_VALUE: i32 = 4;

/// A source that never produces anything (`NeverSource`).
pub(super) fn never_source() -> Rc<dyn IObservable<i32>> {
    Observable::never::<i32>()
}

/// A list that a callback fills, and the callback.
pub(super) fn recording() -> (Rc<RefCell<Vec<i32>>>, impl Fn(i32) + 'static) {
    let values: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    (values, move |value: i32| sink.borrow_mut().push(value))
}

#[test]
fn constructor_two_arg_seeds_default_value() {
    let fixture = ObservableAsPropertyHelper::new(never_source(), |_| {});
    assert_eq!(fixture.value(), 0);
    fixture.dispose();
}

#[test]
fn constructor_with_initial_value_seeds_supplied_value() {
    let fixture = ObservableAsPropertyHelper::with_initial_value(never_source(), |_| {}, SUPPLIED_VALUE);
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}

#[test]
fn constructor_with_initial_value_and_scheduler_seeds_supplied_value() {
    let scheduler = TestScheduler::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value_on(never_source(), |_| {}, SUPPLIED_VALUE, Some(scheduler.scheduler()));
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}

#[test]
fn constructor_with_initial_value_and_defer_subscription_does_not_subscribe_until_value_read() {
    let source = EagerSubscribeObservable::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value_deferred(source.clone() as Rc<dyn IObservable<i32>>, |_| {}, SUPPLIED_VALUE, true);
    assert!(!source.was_subscribed.get());
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    assert!(source.was_subscribed.get());
    fixture.dispose();
}

#[test]
fn constructor_with_initial_value_defer_subscription_and_scheduler_usable() {
    let scheduler = TestScheduler::new();
    let fixture = ObservableAsPropertyHelper::with_options(
        never_source(),
        |_| {},
        None,
        ObservableAsPropertyInitialValue::Value(SUPPLIED_VALUE),
        true,
        Some(scheduler.scheduler()),
    );
    assert!(!fixture.is_subscribed());
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    assert!(fixture.is_subscribed());
    fixture.dispose();
}

#[test]
fn constructor_with_on_changing_only_seeds_default_value() {
    let fixture = ObservableAsPropertyHelper::with_changing(never_source(), |_| {}, |_| {});
    assert_eq!(fixture.value(), 0);
    fixture.dispose();
}

#[test]
fn constructor_with_on_changing_and_initial_value_seeds_supplied_value() {
    let fixture = ObservableAsPropertyHelper::with_options(
        never_source(),
        |_| {},
        Some(Rc::new(|_: i32| {})),
        ObservableAsPropertyInitialValue::Value(SUPPLIED_VALUE),
        false,
        None,
    );
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}

#[test]
fn constructor_with_on_changing_initial_value_and_defer_seeds_supplied_value() {
    let fixture = ObservableAsPropertyHelper::with_options(
        never_source(),
        |_| {},
        Some(Rc::new(|_: i32| {})),
        ObservableAsPropertyInitialValue::Value(SUPPLIED_VALUE),
        true,
        None,
    );
    assert!(!fixture.is_subscribed());
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}

#[test]
fn constructor_full_overload_with_scheduler_and_defer_usable() {
    let scheduler = TestScheduler::new();
    let fixture = ObservableAsPropertyHelper::with_options(
        never_source(),
        |_| {},
        Some(Rc::new(|_: i32| {})),
        ObservableAsPropertyInitialValue::Value(SUPPLIED_VALUE),
        true,
        Some(scheduler.scheduler()),
    );
    assert!(!fixture.is_subscribed());
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    assert!(fixture.is_subscribed());
    fixture.dispose();
}

#[test]
fn constructor_with_get_initial_value_factory_evaluates_factory_eagerly() {
    let accessed = Rc::new(Cell::new(false));
    let factory_accessed = accessed.clone();
    let fixture = ObservableAsPropertyHelper::with_options(
        never_source(),
        |_| {},
        Some(Rc::new(|_: i32| {})),
        ObservableAsPropertyInitialValue::factory(move || {
            factory_accessed.set(true);
            SUPPLIED_VALUE
        }),
        false,
        None,
    );
    assert!(accessed.get());
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}

#[test]
fn constructor_with_get_initial_value_factory_and_defer_does_not_evaluate_factory_until_value_read() {
    let accessed = Rc::new(Cell::new(false));
    let factory_accessed = accessed.clone();
    let fixture = ObservableAsPropertyHelper::with_options(
        never_source(),
        |_| {},
        Some(Rc::new(|_: i32| {})),
        ObservableAsPropertyInitialValue::factory(move || {
            factory_accessed.set(true);
            SUPPLIED_VALUE
        }),
        true,
        None,
    );
    assert!(!accessed.get());
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    assert!(accessed.get());
    fixture.dispose();
}

#[test]
fn constructor_with_func_initial_value_and_defer_but_no_on_changing_usable() {
    let fixture = ObservableAsPropertyHelper::with_options(
        never_source(),
        |_| {},
        None,
        ObservableAsPropertyInitialValue::factory(|| SUPPLIED_VALUE),
        true,
        None,
    );
    assert!(!fixture.is_subscribed());
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}

#[test]
fn constructor_with_null_get_initial_value_factory_falls_back_to_default() {
    // Upstream passes a null factory; without one, the initial value is the default of the type.
    let fixture = ObservableAsPropertyHelper::with_options(never_source(), |_| {}, None, ObservableAsPropertyInitialValue::default(), false, None);
    assert_eq!(fixture.value(), 0);
    fixture.dispose();
}

#[test]
fn default_no_arguments_returns_default_value_and_never_changes() {
    let fixture = ObservableAsPropertyHelper::<i32>::default();
    assert_eq!(fixture.value(), 0);
    assert!(fixture.is_subscribed());
    fixture.dispose();
}

#[test]
fn default_with_initial_value_returns_supplied_value_and_never_changes() {
    let fixture = ObservableAsPropertyHelper::default_with_initial_value(SUPPLIED_VALUE);
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}

#[test]
fn default_with_initial_value_and_scheduler_returns_supplied_value() {
    let scheduler = TestScheduler::new();
    let fixture = ObservableAsPropertyHelper::default_with_initial_value_on(SUPPLIED_VALUE, Some(scheduler.scheduler()));
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}

// Not ported: Constructor_NullObservable_ThrowsArgumentNullException - argument validation

// Not ported: Constructor_NullOnChanged_ThrowsArgumentNullException - argument validation

#[test]
fn eager_subscribes_to_source_immediately() {
    let source = EagerSubscribeObservable::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.clone() as Rc<dyn IObservable<i32>>, |_| {}, 0);
    assert!(source.was_subscribed.get());
    assert!(fixture.is_subscribed());
    fixture.dispose();
}

#[test]
fn eager_delivers_initial_value_immediately_through_both_callbacks() {
    let (changed, on_changed) = recording();
    let (changing, on_changing) = recording();
    let fixture = ObservableAsPropertyHelper::with_options(
        never_source(),
        on_changed,
        Some(Rc::new(on_changing)),
        ObservableAsPropertyInitialValue::Value(SUPPLIED_VALUE),
        false,
        None,
    );
    assert_eq!(*changed.borrow(), vec![SUPPLIED_VALUE]);
    assert_eq!(*changing.borrow(), vec![SUPPLIED_VALUE]);
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    fixture.dispose();
}

#[test]
fn deferred_is_subscribed_is_false_until_value_is_first_read() {
    let fixture = ObservableAsPropertyHelper::with_initial_value_deferred(never_source(), |_| {}, 0, true);
    assert!(!fixture.is_subscribed());
    let _ = fixture.value();
    assert!(fixture.is_subscribed());
    fixture.dispose();
}

#[test]
fn deferred_reading_value_subscribes_and_returns_initial_value_without_raising_callbacks() {
    let (changed, on_changed) = recording();
    let (changing, on_changing) = recording();
    let fixture = ObservableAsPropertyHelper::with_options(
        never_source(),
        on_changed,
        Some(Rc::new(on_changing)),
        ObservableAsPropertyInitialValue::Value(SUPPLIED_VALUE),
        true,
        None,
    );
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    assert!(changed.borrow().is_empty());
    assert!(changing.borrow().is_empty());
    fixture.dispose();
}

#[test]
fn deferred_skips_a_source_value_equal_to_initial_value() {
    const UPDATED_VALUE: i32 = 2;
    let source = Subject::<i32>::new();
    let (received, on_changed) = recording();
    let fixture = ObservableAsPropertyHelper::with_initial_value_deferred(source.as_observable(), on_changed, SUPPLIED_VALUE, true);
    let _ = fixture.value();
    assert!(received.borrow().is_empty());
    source.on_next(SUPPLIED_VALUE);
    assert!(received.borrow().is_empty());
    source.on_next(UPDATED_VALUE);
    assert_eq!(*received.borrow(), vec![UPDATED_VALUE]);
    fixture.dispose();
}

#[test]
fn deferred_with_factory_does_not_access_factory_until_value_is_read() {
    let source = Subject::<i32>::new();
    let accessed = Rc::new(Cell::new(false));
    let factory_accessed = accessed.clone();
    let fixture = ObservableAsPropertyHelper::with_options(
        source.as_observable(),
        |_| {},
        None,
        ObservableAsPropertyInitialValue::factory(move || {
            factory_accessed.set(true);
            SUPPLIED_VALUE
        }),
        true,
        None,
    );
    assert!(!accessed.get());
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    assert!(accessed.get());
    fixture.dispose();
}

#[test]
fn deferred_with_factory_activating_does_not_raise_callbacks_even_when_source_repeats_it() {
    let source = Subject::<i32>::new();
    let changed = Rc::new(Cell::new(false));
    let changing = Rc::new(Cell::new(false));
    let (on_changed, on_changing) = (changed.clone(), changing.clone());
    let fixture = ObservableAsPropertyHelper::with_options(
        source.as_observable(),
        move |_| on_changed.set(true),
        Some(Rc::new(move |_: i32| on_changing.set(true))),
        ObservableAsPropertyInitialValue::factory(|| SUPPLIED_VALUE),
        true,
        None,
    );
    let _ = fixture.value();
    source.on_next(SUPPLIED_VALUE);
    assert!(!changed.get());
    assert!(!changing.get());
    fixture.dispose();
}

#[test]
fn distinct_until_changed_skips_consecutive_equal_values() {
    const UPDATED_VALUE: i32 = 2;
    let source = Subject::<i32>::new();
    let (received, on_changed) = recording();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.as_observable(), on_changed, 0);
    received.borrow_mut().clear();
    source.on_next(1);
    source.on_next(1);
    source.on_next(UPDATED_VALUE);
    assert_eq!(*received.borrow(), vec![1, UPDATED_VALUE]);
    fixture.dispose();
}

#[test]
fn distinct_until_changed_allows_a_repeated_value_after_a_different_one_in_between() {
    const UPDATED_VALUE: i32 = 2;
    let source = Subject::<i32>::new();
    let (received, on_changed) = recording();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.as_observable(), on_changed, 0);
    received.borrow_mut().clear();
    source.on_next(1);
    source.on_next(UPDATED_VALUE);
    source.on_next(1);
    assert_eq!(*received.borrow(), vec![1, UPDATED_VALUE, 1]);
    fixture.dispose();
}

/// A source that records that it was subscribed to and produces nothing
/// (`EagerSubscribeObservable`).
pub(super) struct EagerSubscribeObservable {
    pub(super) was_subscribed: Cell<bool>,
}

impl EagerSubscribeObservable {
    pub(super) fn new() -> Rc<Self> {
        Rc::new(Self { was_subscribed: Cell::new(false) })
    }
}

impl IObservable<i32> for EagerSubscribeObservable {
    fn subscribe(&self, _observer: Rc<dyn IObserver<i32>>) -> Rc<dyn IDisposable> {
        self.was_subscribed.set(true);
        Disposable::empty()
    }
}
