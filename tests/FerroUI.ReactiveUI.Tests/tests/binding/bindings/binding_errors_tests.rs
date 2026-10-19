//! `BindingErrorsTests` (`ReactiveUI.Binding.Tests/Bindings/BindingErrorsTests.cs`).
//!
//! A rethrown fault is a panic with a `TargetInvocationException` payload, which the test catches;
//! the inner error is kept as its message.

use ferroui_reactive::subjects::Subject;
use ferroui_reactiveui::*;
use ferroui_system::{IObserver, ObservableError};
use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// The expression the tests report faults against.
const BOUND_EXPRESSION: &str = "x => x.Caption";

/// A value written through the binding.
const WRITTEN_VALUE: i32 = 42;

/// An error with a message and, optionally, the error that caused it (`InvalidOperationException`).
#[derive(Debug)]
struct InvalidOperation {
    message: &'static str,
    inner: Option<Box<InvalidOperation>>,
}

impl fmt::Display for InvalidOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl Error for InvalidOperation {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.as_deref().map(|inner| inner as &(dyn Error + 'static))
    }
}

#[test]
fn subscribe_when_source_emits_applies_the_write() {
    let subject: Rc<Subject<i32>> = Subject::new();
    let written = Rc::new(RefCell::new(Vec::new()));
    let sink = written.clone();

    let subscription = BindingErrors::subscribe(subject.as_observable(), move |value| sink.borrow_mut().push(value), BOUND_EXPRESSION);
    subject.on_next(WRITTEN_VALUE);

    assert_eq!(*written.borrow(), [WRITTEN_VALUE]);
    subscription.dispose();
}

#[test]
fn subscribe_when_source_completes_does_not_throw() {
    let subject: Rc<Subject<i32>> = Subject::new();
    let written = Rc::new(RefCell::new(Vec::new()));
    let sink = written.clone();

    let subscription = BindingErrors::subscribe(subject.as_observable(), move |value| sink.borrow_mut().push(value), BOUND_EXPRESSION);
    subject.on_completed();

    assert!(written.borrow().is_empty());
    subscription.dispose();
}

#[test]
fn subscribe_when_source_faults_without_inner_exception_does_not_rethrow() {
    let subject: Rc<Subject<i32>> = Subject::new();

    let subscription = BindingErrors::subscribe(subject.as_observable(), |_| {}, BOUND_EXPRESSION);

    let error: ObservableError = Rc::new(InvalidOperation { message: "no inner", inner: None });
    subject.on_error(error);
    subscription.dispose();
}

#[test]
fn subscribe_when_source_faults_with_inner_exception_rethrows_as_target_invocation() {
    let subject: Rc<Subject<i32>> = Subject::new();

    let _subscription = BindingErrors::subscribe(subject.as_observable(), |_| {}, BOUND_EXPRESSION);

    let error: ObservableError = Rc::new(InvalidOperation { message: "outer", inner: Some(Box::new(InvalidOperation { message: "the setter threw", inner: None })) });
    let outcome = catch_unwind(AssertUnwindSafe(|| subject.on_error(error)));

    let payload = outcome.expect_err("the fault is raised again");
    let thrown = payload.downcast_ref::<TargetInvocationException>().expect("a TargetInvocationException");
    assert_eq!(thrown.inner_exception(), Some("the setter threw"));
    assert!(thrown.message().contains(BOUND_EXPRESSION));
}

// Not ported: Subscribe_NullSource_Throws - argument validation

// Not ported: Subscribe_NullOnNext_Throws - argument validation
