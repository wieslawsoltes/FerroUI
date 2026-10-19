//! `ObservableMixinsTest` (`ReactiveUI.Tests/ObservableMixinsTest.cs`).
//!
//! `WhereNotNull` is `where_not_null` on an observable of `Option`s. The internal members the
//! tests reach upstream (`FromAsyncWithAllNotifications`, `DeferredValueObservable`) are public
//! and hidden from the documentation here. `IsSameReferenceAs` on an error is `Rc::ptr_eq`.

use crate::mocks::*;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::threading::tasks::{block_on, canceled, Task, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A reference type of the tests (`TestClass`).
struct TestClass {
    #[allow(dead_code)]
    value: Option<String>,
}

/// The execution the deferred source notifies at once (`Materialize`).
fn materialize<T: 'static>(source: &Rc<dyn IObservable<AsyncExecution<T>>>) -> AsyncExecution<T> {
    let captured: Rc<RefCell<Option<AsyncExecution<T>>>> = Rc::new(RefCell::new(None));
    let sink = captured.clone();
    let subscription = source.subscribe_next(move |execution| *sink.borrow_mut() = Some(execution));
    subscription.dispose();
    let execution = captured.borrow_mut().take();
    execution.expect("The deferred source did not emit a value synchronously.")
}

/// Runs the execution the source notifies and gives its last value, or its error
/// (`RunToCompletionAsync`).
fn run_to_completion<T: Clone + 'static>(source: &Rc<dyn IObservable<AsyncExecution<T>>>) -> Result<T, ObservableError> {
    let (result, _) = materialize(source);
    let completion = TaskCompletionSource::<T>::new();
    let last: Rc<RefCell<Option<T>>> = Rc::new(RefCell::new(None));
    let (on_next_last, on_completed_last) = (last.clone(), last);
    let (failed, completed) = (completion.clone(), completion.clone());
    let _subscription = result.subscribe_all(
        move |value| *on_next_last.borrow_mut() = Some(value),
        move |error| {
            failed.try_set_exception(error);
        },
        move || {
            if let Some(value) = on_completed_last.borrow_mut().take() {
                completed.try_set_result(value);
            }
        },
    );
    block_on(completion.task())
}

/// Upstream resets the builder first (`RxAppBuilder.ResetForTesting()`) to show that
/// `WhereNotNull` does not need it; the builder of ReactiveUI is not ported yet, and
/// `where_not_null` does not use it.
#[test]
fn where_not_null_does_not_require_builder_initialization() {
    let subject = Subject::<Option<i32>>::new();
    let results = Rc::new(RefCell::new(Vec::new()));
    let sink = results.clone();

    let _ = subject.as_observable().where_not_null().subscribe_next(move |value| sink.borrow_mut().push(value));

    subject.on_next(Some(1));
    subject.on_next(None);
    subject.on_next(Some(2));

    assert_eq!(results.borrow().len(), 2);
}

#[test]
fn where_not_null_emits_all_non_null_values() {
    let subject = Subject::<Option<i32>>::new();
    let results = Rc::new(RefCell::new(Vec::new()));
    let sink = results.clone();

    let _ = subject.as_observable().where_not_null().observe_on(immediate()).subscribe_next(move |value| sink.borrow_mut().push(value));

    subject.on_next(Some(1));
    subject.on_next(Some(2));
    subject.on_next(Some(3));

    assert_eq!(*results.borrow(), [1, 2, 3]);
}

#[test]
fn where_not_null_filters_null_values() {
    let subject = Subject::<Option<String>>::new();
    let results = Rc::new(RefCell::new(Vec::new()));
    let sink = results.clone();

    let _ = subject.as_observable().where_not_null().observe_on(immediate()).subscribe_next(move |value| sink.borrow_mut().push(value));

    subject.on_next(Some("value1".to_owned()));
    subject.on_next(None);
    subject.on_next(Some("value2".to_owned()));
    subject.on_next(None);
    subject.on_next(Some("value3".to_owned()));

    assert_eq!(*results.borrow(), ["value1", "value2", "value3"]);
}

#[test]
fn where_not_null_with_only_nulls_emits_nothing() {
    let subject = Subject::<Option<String>>::new();
    let results = Rc::new(RefCell::new(Vec::new()));
    let sink = results.clone();

    let _ = subject.as_observable().where_not_null().observe_on(immediate()).subscribe_next(move |value: String| sink.borrow_mut().push(value));

    subject.on_next(None);
    subject.on_next(None);
    subject.on_next(None);

    assert!(results.borrow().is_empty());
}

#[test]
fn where_not_null_works_with_reference_types() {
    let subject = Subject::<Option<Rc<TestClass>>>::new();
    let results: Rc<RefCell<Vec<Rc<TestClass>>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = results.clone();
    let obj1 = Rc::new(TestClass { value: Some("test1".to_owned()) });
    let obj2 = Rc::new(TestClass { value: Some("test2".to_owned()) });

    let _ = subject.as_observable().where_not_null().observe_on(immediate()).subscribe_next(move |value| sink.borrow_mut().push(value));

    subject.on_next(Some(obj1.clone()));
    subject.on_next(None);
    subject.on_next(Some(obj2.clone()));

    assert_eq!(results.borrow().len(), 2);
    assert!(Rc::ptr_eq(&results.borrow()[0], &obj1));
    assert!(Rc::ptr_eq(&results.borrow()[1], &obj2));
}

#[test]
fn where_not_null_forwards_completion() {
    let subject = Subject::<Option<i32>>::new();
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();

    let subscription = subject.as_observable().where_not_null().subscribe_all(|_| {}, |_| {}, move || flag.set(true));

    subject.on_completed();

    assert!(completed.get());
    subscription.dispose();
}

#[test]
fn where_not_null_forwards_error() {
    let subject = Subject::<Option<i32>>::new();
    let expected = InvalidOperationException::error("source failure");
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let sink = captured.clone();

    let subscription = subject.as_observable().where_not_null().subscribe_all(|_| {}, move |error| *sink.borrow_mut() = Some(error), || {});

    subject.on_error(expected.clone());

    assert!(captured.borrow().as_ref().is_some_and(|error| Rc::ptr_eq(error, &expected)));
    subscription.dispose();
}

#[test]
fn deferred_value_observable_forwards_factory_exception_as_on_error() {
    let expected = InvalidOperationException::error("factory failure");
    let failure = expected.clone();
    let observable: Rc<dyn IObservable<i32>> = Rc::new(DeferredValueObservable::new(move || Err(failure.clone())));
    let captured: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let sink = captured.clone();

    let subscription = observable.subscribe_all(|_| {}, move |error| *sink.borrow_mut() = Some(error), || {});

    assert!(captured.borrow().as_ref().is_some_and(|error| Rc::ptr_eq(error, &expected)));
    subscription.dispose();
}

#[test]
fn from_async_with_all_notifications_runs_action() {
    let source = from_async_with_all_notifications(|_| Task::from_result(()));

    let result = run_to_completion(&source);

    assert!(result.is_ok());
}

#[test]
fn from_async_with_all_notifications_with_parameter_runs_action() {
    const PARAMETER: i32 = 7;
    let observed = Rc::new(Cell::new(0));
    let sink = observed.clone();

    let source = from_async_with_all_notifications_param(
        move |param: i32, _| {
            sink.set(param);
            Task::from_result(())
        },
        PARAMETER,
    );

    let _ = run_to_completion(&source);

    assert_eq!(observed.get(), PARAMETER);
}

#[test]
fn from_async_with_all_notifications_produces_result() {
    let source = from_async_with_all_notifications(|_| Task::from_result(42));

    let result = run_to_completion(&source);

    assert_eq!(result.expect("the action produces a result"), 42);
}

#[test]
fn from_async_with_all_notifications_with_parameter_produces_result() {
    const PARAMETER: i32 = 21;
    const MULTIPLIER: i32 = 2;

    let source = from_async_with_all_notifications_param(|param: i32, _| Task::from_result(param * MULTIPLIER), PARAMETER);

    let result = run_to_completion(&source);

    assert_eq!(result.expect("the action produces a result"), 42);
}

#[test]
fn from_async_with_all_notifications_propagates_error() {
    let expected = InvalidOperationException::error("async failure");

    let source = from_async_with_all_notifications_param(|error: ObservableError, _| Task::<i32>::from_exception(error), expected.clone());

    let (result, _) = materialize(&source);

    let completion = TaskCompletionSource::<ObservableError>::new();
    let failed = completion.clone();
    let _subscription = result.subscribe_all(
        |_| {},
        move |error| {
            failed.try_set_result(error);
        },
        || {},
    );

    let actual = block_on(completion.task()).expect("the error is notified");

    assert!(Rc::ptr_eq(&actual, &expected));
}

#[test]
fn from_async_with_all_notifications_cancel_delegate_is_invokable() {
    let started = TaskCompletionSource::<()>::new();
    let release = TaskCompletionSource::<()>::new();

    let source = from_async_with_all_notifications_param(
        |(start, release): (TaskCompletionSource<()>, TaskCompletionSource<()>), token: CancellationToken| async move {
            start.try_set_result(());
            release.task().await?;
            if token.is_cancellation_requested() {
                return canceled();
            }
            Ok(0)
        },
        (started.clone(), release.clone()),
    );

    let (result, cancel) = materialize(&source);

    let completion = TaskCompletionSource::<()>::new();
    let (failed, completed) = (completion.clone(), completion.clone());
    let _subscription = result.subscribe_all(
        |_| {},
        move |_| {
            failed.try_set_result(());
        },
        move || {
            completed.try_set_result(());
        },
    );

    assert!(started.task().is_completed());
    cancel();
    release.try_set_result(());

    block_on(completion.task()).expect("the execution ends");

    assert!(completion.task().is_completed_successfully());
}
