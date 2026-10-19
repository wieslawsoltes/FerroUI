//! `...ObjectReactiveExtensionsBehaviorTests` (the tests of the integration).

use super::binding_support::start_app;
use crate::{FerroObjectReactiveExtensions, FerroPropertySignal};
use ferroui_base::data::BindingValue;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

/// The value of the untyped round trip.
const UNTYPED_SUBJECT_VALUE: i32 = 42;

/// The value of the typed round trip.
const GENERIC_SUBJECT_VALUE: i32 = 7;

/// The value of the untyped binding-value test.
const UNTYPED_BINDING_VALUE: i32 = 99;

/// The value of the typed binding-value test.
const GENERIC_BINDING_VALUE: i32 = 5;

/// The value pushed after the signal completed, which is dropped.
const IGNORED_POST_COMPLETION_VALUE: i32 = 123;

/// A control with an integer styled property (`TestControl`).
#[repr(C)]
struct TestControl {
    base: Control,
}

ferro_class!(TestControl: Control);
ferro_impl_classes!(
    TestControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferroui_base::ferro_properties! { impl TestControl {
    ferro_property!(
        /// The styled property of `int_prop` (`IntPropProperty`).
        pub fn int_prop_property() -> StyledProperty<i32> {
            FerroProperty::register::<TestControl, _>("IntProp", 0)
        }
    );
} }

impl TestControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct() })
    }

    /// The integer property (`IntProp`).
    fn int_prop(&self) -> i32 {
        self.get_value(Self::int_prop_property())
    }
}

/// An observable that panics when it is subscribed (`ThrowingObservable`).
struct ThrowingObservable;

impl IObservable<i32> for ThrowingObservable {
    fn subscribe(&self, _observer: Rc<dyn IObserver<i32>>) -> Rc<dyn IDisposable> {
        panic!("InvalidOperationException: expected");
    }
}

/// An error of the tests (`new InvalidOperationException("expected")`).
#[derive(Debug)]
struct ExpectedError;

impl std::fmt::Display for ExpectedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvalidOperationException: expected")
    }
}

impl std::error::Error for ExpectedError {}

#[test]
fn get_subject_object_roundtrip_writes_and_observes() {
    let _app = start_app();
    let ctrl = TestControl::new();
    let subject = ctrl.get_subject_untyped(TestControl::int_prop_property().as_property());
    let observed: Rc<RefCell<Option<BoxedValue>>> = Rc::new(RefCell::new(None));
    let seen = observed.clone();
    let subscription = (subject.clone() as Rc<dyn IObservable<BoxedValue>>).subscribe_next(move |value| *seen.borrow_mut() = Some(value));

    subject.on_next(Rc::new(UNTYPED_SUBJECT_VALUE) as BoxedValue);

    assert_eq!(ctrl.int_prop(), UNTYPED_SUBJECT_VALUE);
    let observed = observed.borrow().clone().expect("a value");
    assert_eq!(observed.downcast_ref::<i32>(), Some(&UNTYPED_SUBJECT_VALUE));
    subscription.dispose();
}

#[test]
fn get_subject_generic_roundtrip_writes_and_observes() {
    let _app = start_app();
    let ctrl = TestControl::new();
    let subject = ctrl.get_subject(TestControl::int_prop_property());
    let observed = Rc::new(Cell::new(None));
    let seen = observed.clone();
    let subscription = (subject.clone() as Rc<dyn IObservable<i32>>).subscribe_next(move |value| seen.set(Some(value)));

    subject.on_next(GENERIC_SUBJECT_VALUE);

    assert_eq!(ctrl.int_prop(), GENERIC_SUBJECT_VALUE);
    assert_eq!(observed.get(), Some(GENERIC_SUBJECT_VALUE));
    subscription.dispose();
}

#[test]
fn get_binding_subject_object_roundtrip_writes_only_on_has_value() {
    let _app = start_app();
    let ctrl = TestControl::new();
    let subject = ctrl.get_binding_subject_untyped(TestControl::int_prop_property().as_property());
    let observed: Rc<RefCell<Option<BindingValue<BoxedValue>>>> = Rc::new(RefCell::new(None));
    let seen = observed.clone();
    let subscription =
        (subject.clone() as Rc<dyn IObservable<BindingValue<BoxedValue>>>).subscribe_next(move |value| *seen.borrow_mut() = Some(value));

    subject.on_next(BindingValue::do_nothing());
    assert_eq!(ctrl.int_prop(), 0);

    subject.on_next(BindingValue::new(Rc::new(UNTYPED_BINDING_VALUE) as BoxedValue));
    assert_eq!(ctrl.int_prop(), UNTYPED_BINDING_VALUE);
    assert!(observed.borrow().as_ref().expect("a value").has_value());
    subscription.dispose();
}

#[test]
fn get_binding_subject_generic_roundtrip_writes_only_on_has_value() {
    let _app = start_app();
    let ctrl = TestControl::new();
    let subject = ctrl.get_binding_subject(TestControl::int_prop_property());
    let observed: Rc<RefCell<Option<BindingValue<i32>>>> = Rc::new(RefCell::new(None));
    let seen = observed.clone();
    let subscription = (subject.clone() as Rc<dyn IObservable<BindingValue<i32>>>).subscribe_next(move |value| *seen.borrow_mut() = Some(value));

    subject.on_next(BindingValue::do_nothing());
    assert_eq!(ctrl.int_prop(), 0);

    subject.on_next(BindingValue::new(GENERIC_BINDING_VALUE));
    assert_eq!(ctrl.int_prop(), GENERIC_BINDING_VALUE);
    assert!(observed.borrow().as_ref().expect("a value").has_value());
    subscription.dispose();
}

#[test]
fn get_subject_signal_tracks_observers_and_disposed_state() {
    let _app = start_app();
    let ctrl = TestControl::new();
    let subject = ctrl.get_subject(TestControl::int_prop_property());

    assert!(!subject.has_observers());
    assert!(!subject.is_disposed());

    let subscription = (subject.clone() as Rc<dyn IObservable<i32>>).subscribe_next(|_| {});
    assert!(subject.has_observers());
    subscription.dispose();

    assert!(!subject.has_observers());

    subject.on_completed();
    subject.on_next(IGNORED_POST_COMPLETION_VALUE);

    assert!(subject.is_disposed());
    assert_eq!(ctrl.int_prop(), 0);
}

#[test]
fn get_subject_signal_dispose_sets_disposed() {
    let _app = start_app();
    let ctrl = TestControl::new();
    let subject = ctrl.get_subject(TestControl::int_prop_property());

    subject.dispose();

    assert!(subject.is_disposed());
}

/// `GetSubject_SignalOnErrorSetsDisposedAndThrowsNonNullError`: the half that passes a null error
/// has no counterpart (an error is never null in the port).
#[test]
fn get_subject_signal_on_error_sets_disposed_and_throws_non_null_error() {
    let _app = start_app();
    let ctrl = TestControl::new();
    let error_subject = ctrl.get_subject(TestControl::int_prop_property());

    let raised = panic::catch_unwind(AssertUnwindSafe(|| error_subject.on_error(Rc::new(ExpectedError) as ObservableError)));

    let message = raised.expect_err("the error is raised again");
    let message = message.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(message.contains("InvalidOperationException"));
    assert!(error_subject.is_disposed());
}

#[test]
fn get_subject_when_subscribe_throws_restores_observer_count() {
    let subject = FerroPropertySignal::new(|_: i32| {}, Rc::new(ThrowingObservable));

    let raised = panic::catch_unwind(AssertUnwindSafe(|| (subject.clone() as Rc<dyn IObservable<i32>>).subscribe_next(|_| {})));

    assert!(raised.is_err());
    assert!(!subject.has_observers());
}
