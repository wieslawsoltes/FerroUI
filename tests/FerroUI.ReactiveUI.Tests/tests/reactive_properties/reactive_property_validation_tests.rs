//! `ReactivePropertyValidationTests` (`ReactiveUI.Tests/ReactiveProperties/ReactivePropertyValidationTests.cs`).
//!
//! The manually driven validators are subjects of [`DataErrors`] (upstream's `Signal<IEnumerable?>`).

use crate::mocks::*;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::threading::tasks::{run_pending_wakes, Task};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// `ObservableError`.
const OBSERVABLE_ERROR: &str = "observable-error";
/// `AsyncError`.
const ASYNC_ERROR: &str = "async-error";
/// `NegativeError`.
const NEGATIVE_ERROR: &str = "negative";
/// `PositiveError`.
const POSITIVE_ERROR: &str = "positive";
/// `PollAttempts`.
const POLL_ATTEMPTS: usize = 500;

/// An error of a test (`InvalidOperationException`).
#[derive(Debug)]
struct TestError(&'static str);

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for TestError {}

fn error(message: &'static str) -> ObservableError {
    Rc::new(TestError(message))
}

/// `NegativeErrorObjectArray`: one message, as an untyped collection.
fn negative_error_object_array() -> Option<DataErrors> {
    Some(vec![Rc::new(NEGATIVE_ERROR.to_owned()) as Rc<dyn Any>])
}

/// `NegativeErrorArray`.
fn negative_error_array() -> Option<DataErrors> {
    Some(vec![Rc::new(NEGATIVE_ERROR.to_owned()) as Rc<dyn Any>])
}

/// `PositiveErrorArray`.
fn positive_error_array() -> Option<DataErrors> {
    Some(vec![Rc::new(POSITIVE_ERROR.to_owned()) as Rc<dyn Any>])
}

fn int_property(initial: i32) -> ReactiveProperty<i32> {
    ReactiveProperty::with_scheduler(initial, Some(immediate()), false, false)
}

fn manual_validator(subject: &Rc<Subject<Option<DataErrors>>>) -> impl Fn(Rc<dyn IObservable<i32>>) -> Rc<dyn IObservable<Option<DataErrors>>> + 'static {
    let observable = subject.as_observable();
    move |_| observable.clone()
}

/// `Task.Yield()`: a future that is pending once.
struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        context.waker().wake_by_ref();
        Poll::Pending
    }
}

#[test]
fn observable_validator_surfaces_error() {
    let rp = int_property(0).add_validation_error_observable(|xs| xs.select(|x: i32| if x < 0 { Some(OBSERVABLE_ERROR) } else { None }));

    assert!(!rp.has_errors());

    rp.set_value(-1);

    assert!(rp.has_errors());
    assert!(string_errors(&rp.get_errors(None)).contains(&OBSERVABLE_ERROR.to_owned()));
    rp.dispose();
}

#[test]
fn async_validator_surfaces_error() {
    let rp = int_property(0).add_validation_error_async(|x: i32| Task::from_result(if x < 0 { Some(ASYNC_ERROR) } else { None }));

    rp.set_value(-1);

    assert!(string_errors(&rp.get_errors(None)).contains(&ASYNC_ERROR.to_owned()));
    rp.dispose();
}

#[test]
fn multiple_validators_aggregate_errors() {
    let rp = int_property(0)
        .add_validation_error(|x: i32| if x < 0 { Some(NEGATIVE_ERROR) } else { None })
        .add_validation_error(|x: i32| if x > 0 { Some(POSITIVE_ERROR) } else { None });

    rp.set_value(-1);
    assert!(string_errors(&rp.get_errors(None)).contains(&NEGATIVE_ERROR.to_owned()));

    rp.set_value(1);
    assert!(string_errors(&rp.get_errors(None)).contains(&POSITIVE_ERROR.to_owned()));
    rp.dispose();
}

#[test]
fn observable_enumerable_validator_single_arg_overload() {
    let rp = ReactiveProperty::<Option<String>>::with_scheduler(None, Some(immediate()), false, false).add_validation_error_observable(
        |xs| xs.select(|x: Option<String>| if x.as_deref().is_none_or(str::is_empty) { Some(vec![OBSERVABLE_ERROR]) } else { None }),
    );

    assert!(rp.has_errors());
    assert!(string_errors(&rp.get_errors(None)).contains(&OBSERVABLE_ERROR.to_owned()));
    rp.dispose();
}

#[test]
fn async_enumerable_validator_single_arg_overload() {
    let rp = int_property(-1).add_validation_error_async(|x: i32| Task::from_result(if x < 0 { Some(vec![ASYNC_ERROR]) } else { None }));

    assert!(string_errors(&rp.get_errors(None)).contains(&ASYNC_ERROR.to_owned()));
    rp.dispose();
}

#[test]
fn async_string_validator_single_arg_overload() {
    let rp = int_property(-1).add_validation_error_async(|x: i32| Task::from_result(if x < 0 { Some(ASYNC_ERROR) } else { None }));

    assert!(string_errors(&rp.get_errors(None)).contains(&ASYNC_ERROR.to_owned()));
    rp.dispose();
}

#[test]
fn manual_validators_aggregate_mixed_error_elements() {
    let sig0 = Subject::<Option<DataErrors>>::new();
    let sig1 = Subject::<Option<DataErrors>>::new();

    let rp = int_property(0).add_validation_error_observable(manual_validator(&sig0)).add_validation_error_observable(manual_validator(&sig1));

    // Only the first validator reported: the aggregate waits for the second.
    sig0.on_next(negative_error_object_array());
    assert!(!rp.has_errors());

    // Both validators reported; the errors of both are aggregated.
    sig1.on_next(positive_error_array());

    assert!(rp.has_errors());
    assert!(string_errors(&rp.get_errors(None)).contains(&NEGATIVE_ERROR.to_owned()));
    assert!(string_errors(&rp.get_errors(None)).contains(&POSITIVE_ERROR.to_owned()));

    // A later report of no errors from both clears the aggregate.
    sig0.on_next(None);
    sig1.on_next(None);
    assert!(!rp.has_errors());
    rp.dispose();
}

#[test]
fn manual_validator_error_is_forwarded_and_stops() {
    let erroring = Subject::<Option<DataErrors>>::new();
    let second_erroring = Subject::<Option<DataErrors>>::new();
    let live = Subject::<Option<DataErrors>>::new();

    let rp = int_property(0)
        .add_validation_error_observable(manual_validator(&erroring))
        .add_validation_error_observable(manual_validator(&second_erroring))
        .add_validation_error_observable(manual_validator(&live));

    // The first validator fails, which ends the aggregate.
    erroring.on_error(error("boom"));

    // A second error after the end returns early.
    second_erroring.on_error(error("second"));

    // A report of a validator still running after the end is ignored.
    live.on_next(negative_error_array());

    assert!(!rp.has_errors());
    rp.dispose();
}

#[test]
fn async_validator_dispose_completes_source() {
    let rp = int_property(0).add_validation_error_async(|x: i32| Task::from_result(if x < 0 { Some(ASYNC_ERROR) } else { None }));

    rp.set_value(-1);
    assert!(string_errors(&rp.get_errors(None)).contains(&ASYNC_ERROR.to_owned()));

    // Disposing ends the source of the validator and the subscriptions of the aggregate.
    rp.dispose();

    assert!(rp.is_disposed());
}

#[test]
fn manual_validators_complete_when_all_complete() {
    let sig0 = Subject::<Option<DataErrors>>::new();
    let sig1 = Subject::<Option<DataErrors>>::new();

    let rp = int_property(0).add_validation_error_observable(manual_validator(&sig0)).add_validation_error_observable(manual_validator(&sig1));

    // Both validators report, so the aggregate has a full set of latest values.
    sig0.on_next(negative_error_array());
    sig1.on_next(None);
    assert!(string_errors(&rp.get_errors(None)).contains(&NEGATIVE_ERROR.to_owned()));

    // One end does not end the aggregate.
    sig0.on_completed();

    // The validator still running drives the aggregate.
    sig1.on_next(positive_error_array());
    assert!(string_errors(&rp.get_errors(None)).contains(&POSITIVE_ERROR.to_owned()));

    // The end of the last validator ends the aggregate.
    sig1.on_completed();
    assert!(rp.has_errors());
    rp.dispose();
}

#[test]
fn async_validator_faulted_task_is_handled() {
    let rp = int_property(0).add_validation_error_async(|_: i32| Task::<Option<String>>::from_exception(error("nope")));

    rp.set_value(-1);

    // The failed task is an error of the validation pipeline, which the property ignores: no
    // errors are recorded, and the property stays usable.
    assert!(!rp.has_errors());
    rp.dispose();
}

#[test]
fn async_validator_selector_throws_synchronously() {
    let rp = int_property(0).add_validation_error_async(|x: i32| {
        if x < 0 {
            panic!("sync throw");
        }
        Task::<Option<String>>::from_result(None)
    });

    rp.set_value(-1);

    assert!(!rp.has_errors());
    rp.dispose();
}

#[test]
fn async_validator_awaited_task_delivers_result() {
    let rp = int_property(0).add_validation_error_async_with(
        |x: i32| {
            Task::spawn_local(async move {
                YieldNow(false).await;
                Ok::<_, ObservableError>(if x < 0 { Some(ASYNC_ERROR) } else { None })
            })
        },
        true,
    );

    rp.set_value(-1);

    // Upstream polls with delays; the wakes of the tasks of the thread run here.
    for _ in 0..POLL_ATTEMPTS {
        if rp.has_errors() {
            break;
        }
        let _ = run_pending_wakes();
    }

    assert!(string_errors(&rp.get_errors(None)).contains(&ASYNC_ERROR.to_owned()));
    rp.dispose();
}
