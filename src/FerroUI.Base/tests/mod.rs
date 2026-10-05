//! Unit tests of the property system, ported from the upstream test suite
//! (one file per upstream test class), plus tests specific to this port.
//!
//! Property definitions and the property registry are per-thread, and the
//! test harness runs every test on its own thread, so each test starts from
//! a clean set of property registrations.

#![allow(dead_code)]

use crate::data::BindingValue;
use crate::reactive::{Disposable, IDisposable, IObservable, IObserver, ObservableError};
use crate::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

mod attached_property_tests;
mod binding_default_value_converter_tests;
mod binding_expression_tests_attached_property;
mod binding_expression_tests_data_validation;
mod binding_expression_tests_ferro_property;
mod binding_expression_tests_get_value;
mod binding_expression_tests_mode;
mod binding_expression_tests_negation;
mod binding_expression_tests_observable;
mod binding_expression_tests_property;
mod binding_expression_tests_set_value;
mod binding_expression_tests_update_source_trigger;
mod binding_expression_observer_indexer_tests;
mod binding_expression_tests_indexer;
mod binding_expression_tests_task;
mod binding_model_tests;
mod binding_null_conditional_tests;
mod binding_operations_tests;
mod binding_plugin_tests;
mod binding_setter_tests;
mod reference_semantics_tests;
mod binding_test_support;
mod binding_typed_expression_tests;
mod class_registration_tests;
mod compiled_binding_tests_create;
mod direct_property_tests;
mod ferro_object_tests_add_owner;
mod ferro_object_tests_attached;
mod ferro_object_tests_binding;
mod ferro_object_tests_binding_two_way;
mod ferro_object_tests_coercion;
mod ferro_object_tests_data_validation;
mod ferro_object_tests_direct;
mod ferro_object_tests_get_observable;
mod ferro_object_tests_get_value;
mod ferro_object_tests_inheritance;
mod ferro_object_tests_metadata;
mod ferro_object_tests_on_property_changed;
mod ferro_object_tests_reentrancy;
mod ferro_object_tests_set_current_value;
mod ferro_object_tests_set_value;
mod ferro_object_tests_validation;
mod ferro_property_registry_tests;
mod ferro_property_tests;
mod property_registration_tests;
mod property_store;
mod styled_property_tests;

/// Declares a test class without fields or overrides.
macro_rules! test_class {
    ($name:ident : $base:ident) => {
        #[repr(C)]
        pub struct $name {
            base: $base,
        }

        $crate::ferro_class!($name: $base);
        $crate::ferro_impl_classes!($name: $crate::FerroObjectImpl);

        impl $name {
            pub fn construct() -> Self {
                Self { base: $base::construct() }
            }

            pub fn new() -> $crate::Ref<Self> {
                $crate::instantiate(Self::construct())
            }
        }
    };
}
pub(crate) use test_class;

/// Declares a non-instantiable type that can own attached properties.
macro_rules! static_type {
    ($name:ident) => {
        pub struct $name;

        impl $crate::StaticType for $name {
            const TYPE: &'static $crate::TypeInfo = {
                static TYPE: $crate::TypeInfo = $crate::TypeInfo::new(stringify!($name), None);
                &TYPE
            };
        }
    };
}
pub(crate) use static_type;

/// Runs a block once per thread: the equivalent of a static constructor for
/// per-thread property definitions.
macro_rules! once_per_thread {
    ($body:block) => {{
        ::std::thread_local! {
            static DONE: ::std::cell::Cell<bool> = const { ::std::cell::Cell::new(false) };
        }
        if !DONE.replace(true) $body
    }};
}
pub(crate) use once_per_thread;

/// Asserts that `f` panics: the equivalent of asserting that an argument or
/// invalid-operation exception is thrown.
#[track_caller]
pub(crate) fn assert_panics(f: impl FnOnce()) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    assert!(result.is_err(), "expected a panic");
}

pub(crate) fn s(value: &str) -> String {
    value.to_string()
}

/// A binding value holding a string.
pub(crate) fn bv(value: &str) -> BindingValue<String> {
    BindingValue::new(value.to_string())
}

/// An untyped value.
pub(crate) fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
    Rc::new(value)
}

/// An error for binding values.
pub(crate) fn error(message: &str) -> crate::data::BindingError {
    crate::data::BindingError::message(message)
}

struct SubjectInner<T> {
    /// The value replayed to new subscribers, if any.
    current: RefCell<Option<T>>,
    /// Whether `on_next` updates the replayed value.
    track_current: bool,
    /// Whether completing removes the subscribers.
    remove_on_completed: bool,
    observers: RefCell<Vec<(u64, Rc<dyn IObserver<T>>)>>,
    next_id: Cell<u64>,
    completed: Cell<bool>,
}

/// A test subject covering the three kinds used by the upstream tests:
///
/// * [`Subject::new`]: a plain subject;
/// * [`Subject::behavior`]: replays the latest value to new subscribers;
/// * [`Subject::test`]: replays a fixed initial value and counts subscribers
///   (subscribers are only removed when they unsubscribe).
pub(crate) struct Subject<T> {
    inner: Rc<SubjectInner<T>>,
}

impl<T> Clone for Subject<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + 'static> Subject<T> {
    fn create(current: Option<T>, track_current: bool, remove_on_completed: bool) -> Self {
        Self {
            inner: Rc::new(SubjectInner {
                current: RefCell::new(current),
                track_current,
                remove_on_completed,
                observers: RefCell::new(Vec::new()),
                next_id: Cell::new(0),
                completed: Cell::new(false),
            }),
        }
    }

    pub fn new() -> Self {
        Self::create(None, false, true)
    }

    pub fn behavior(initial: T) -> Self {
        Self::create(Some(initial), true, true)
    }

    pub fn test(initial: T) -> Self {
        Self::create(Some(initial), false, false)
    }

    pub fn subscriber_count(&self) -> usize {
        self.inner.observers.borrow().len()
    }

    pub fn observable(&self) -> Rc<dyn IObservable<T>> {
        Rc::new(self.clone())
    }

    fn snapshot(&self) -> Vec<Rc<dyn IObserver<T>>> {
        self.inner.observers.borrow().iter().map(|(_, o)| o.clone()).collect()
    }

    pub fn on_next(&self, value: T) {
        if self.inner.track_current {
            *self.inner.current.borrow_mut() = Some(value.clone());
        }
        for observer in self.snapshot() {
            observer.on_next(value.clone());
        }
    }

    pub fn on_completed(&self) {
        self.inner.completed.set(true);
        let observers = self.snapshot();
        if self.inner.remove_on_completed {
            self.inner.observers.borrow_mut().clear();
        }
        for observer in observers {
            observer.on_completed();
        }
    }

    pub fn on_error(&self, error: ObservableError) {
        self.inner.completed.set(true);
        let observers = self.snapshot();
        if self.inner.remove_on_completed {
            self.inner.observers.borrow_mut().clear();
        }
        for observer in observers {
            observer.on_error(error.clone());
        }
    }
}

impl<T: Clone + 'static> IObservable<T> for Subject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        if self.inner.completed.get() && self.inner.remove_on_completed {
            observer.on_completed();
            return Disposable::empty();
        }
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);
        self.inner.observers.borrow_mut().push((id, observer.clone()));
        let current = self.inner.current.borrow().clone();
        if let Some(current) = current {
            observer.on_next(current);
        }
        let inner = Rc::downgrade(&self.inner);
        Disposable::create(move || {
            if let Some(inner) = inner.upgrade() {
                inner.observers.borrow_mut().retain(|(i, _)| *i != id);
            }
        })
    }
}

/// Collects values into a shared list.
pub(crate) struct Recorder<T> {
    values: Rc<RefCell<Vec<T>>>,
}

impl<T> Clone for Recorder<T> {
    fn clone(&self) -> Self {
        Self { values: self.values.clone() }
    }
}

impl<T: Clone> Recorder<T> {
    pub fn new() -> Self {
        Self { values: Rc::new(RefCell::new(Vec::new())) }
    }

    pub fn push(&self, value: T) {
        self.values.borrow_mut().push(value);
    }

    pub fn clear(&self) {
        self.values.borrow_mut().clear();
    }

    pub fn len(&self) -> usize {
        self.values.borrow().len()
    }

    pub fn get(&self) -> Vec<T> {
        self.values.borrow().clone()
    }
}

/// A shared counter.
#[derive(Clone, Default)]
pub(crate) struct Counter(Rc<Cell<i32>>);

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment(&self) {
        self.0.set(self.0.get() + 1);
    }

    pub fn get(&self) -> i32 {
        self.0.get()
    }
}

/// A shared flag.
#[derive(Clone, Default)]
pub(crate) struct Flag(Rc<Cell<bool>>);

impl Flag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, value: bool) {
        self.0.set(value);
    }

    pub fn get(&self) -> bool {
        self.0.get()
    }
}

/// Whether the sender of a change notification is `target`.
pub(crate) fn is_sender<T: crate::ObjectType>(
    e: &crate::FerroPropertyChangedEventArgs<'_>,
    target: &crate::WeakRef<T>,
) -> bool {
    match target.upgrade() {
        Some(target) => e.sender().to_ref().ptr_eq(&target),
        None => false,
    }
}

/// The old value of a string property change.
pub(crate) fn old_string(e: &crate::FerroPropertyChangedEventArgs<'_>) -> Option<String> {
    e.get_old_value::<String>()
}

/// The new value of a string property change.
pub(crate) fn new_string(e: &crate::FerroPropertyChangedEventArgs<'_>) -> String {
    e.get_new_value::<String>()
}
