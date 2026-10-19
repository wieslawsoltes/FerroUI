//! `SkipFirstObserverTests` (`ReactiveUI.Tests/MessageBus/SkipFirstObserverTests.cs`).

use ferroui_reactiveui::SkipFirstObserver;
use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// Records what it is given (`CapturingObserver<T>`).
struct CapturingObserver<T> {
    values: RefCell<Vec<T>>,
    error: RefCell<Option<ObservableError>>,
    completed: Cell<bool>,
}

impl<T> CapturingObserver<T> {
    fn new() -> Rc<Self> {
        Rc::new(Self { values: RefCell::new(Vec::new()), error: RefCell::new(None), completed: Cell::new(false) })
    }
}

impl<T> IObserver<T> for CapturingObserver<T> {
    fn on_next(&self, value: T) {
        self.values.borrow_mut().push(value);
    }

    fn on_error(&self, error: ObservableError) {
        *self.error.borrow_mut() = Some(error);
    }

    fn on_completed(&self) {
        self.completed.set(true);
    }
}

#[derive(Debug)]
struct Boom;

impl fmt::Display for Boom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("boom")
    }
}

impl std::error::Error for Boom {}

#[test]
fn skips_first_value_then_forwards() {
    const FIRST: i32 = 1;
    const SECOND: i32 = 2;
    const THIRD: i32 = 3;
    let downstream = CapturingObserver::new();
    let observer = SkipFirstObserver::new(downstream.clone());

    observer.on_next(FIRST);
    observer.on_next(SECOND);
    observer.on_next(THIRD);

    assert_eq!(*downstream.values.borrow(), [SECOND, THIRD]);
}

#[test]
fn forwards_error_and_completion() {
    let error: ObservableError = Rc::new(Boom);
    let erroring = CapturingObserver::<i32>::new();
    SkipFirstObserver::new(erroring.clone()).on_error(error.clone());
    assert!(erroring.error.borrow().as_ref().is_some_and(|seen| Rc::ptr_eq(seen, &error)));

    let completing = CapturingObserver::<i32>::new();
    SkipFirstObserver::new(completing.clone()).on_completed();
    assert!(completing.completed.get());
}
