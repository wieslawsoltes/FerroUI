//! The tests of the internal machinery (`DynamicData.Tests/Internal`), one module per upstream
//! fixture, with the observers they share.
//!
//! Not ported: `NotificationFixture` - the notification of the queues is private to
//! `internal/delivery_queue.rs` and `internal/shared_delivery_queue.rs`, and its cases are those
//! of the queues; `SwappableLockFixture` - real threads (a lock that is swapped between threads).

mod cache_parent_subscription_fixture;
mod delivery_queue_fixture;
mod delivery_queue_merge_fixture;
mod keyed_disposable_fixture;
mod shared_delivery_queue_fixture;
mod unsynchronized_combine_latest_fixture;
mod unsynchronized_merge_fixture;

use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// An error of a test (`InvalidOperationException`).
#[derive(Debug)]
pub(super) struct TestError(pub(super) String);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TestError {}

/// A new error with `message`.
pub(super) fn test_error(message: &str) -> ObservableError {
    Rc::new(TestError(message.to_owned()))
}

/// Whether `actual` is the error `expected` (`BeSameAs`).
pub(super) fn same_error(actual: &Option<ObservableError>, expected: &ObservableError) -> bool {
    actual.as_ref().is_some_and(|actual| Rc::ptr_eq(actual, expected))
}

/// An observer that records its notifications (`ListObserver<T>`).
pub(super) struct ListObserver<T> {
    items: RefCell<Vec<T>>,
    error: RefCell<Option<ObservableError>>,
    is_completed: Cell<bool>,
}

impl<T: Clone> ListObserver<T> {
    pub(super) fn new() -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(Vec::new()), error: RefCell::new(None), is_completed: Cell::new(false) })
    }

    pub(super) fn items(&self) -> Vec<T> {
        self.items.borrow().clone()
    }

    pub(super) fn error(&self) -> Option<ObservableError> {
        self.error.borrow().clone()
    }

    pub(super) fn is_completed(&self) -> bool {
        self.is_completed.get()
    }
}

impl<T> IObserver<T> for ListObserver<T> {
    fn on_next(&self, value: T) {
        self.items.borrow_mut().push(value);
    }

    fn on_error(&self, error: ObservableError) {
        *self.error.borrow_mut() = Some(error);
    }

    fn on_completed(&self) {
        self.is_completed.set(true);
    }
}

/// An observer whose values go to a function (`DelegateObserver<T>`); errors and completions are
/// ignored.
pub(super) struct DelegateObserver<T> {
    on_next: Box<dyn Fn(T)>,
}

impl<T: 'static> DelegateObserver<T> {
    pub(super) fn new(on_next: impl Fn(T) + 'static) -> Rc<Self> {
        Rc::new(Self { on_next: Box::new(on_next) })
    }
}

impl<T> IObserver<T> for DelegateObserver<T> {
    fn on_next(&self, value: T) {
        (self.on_next)(value);
    }
}

/// Runs `action`, and whether it panicked (`act.Should().Throw()`).
pub(super) fn panics(action: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).is_err()
}
