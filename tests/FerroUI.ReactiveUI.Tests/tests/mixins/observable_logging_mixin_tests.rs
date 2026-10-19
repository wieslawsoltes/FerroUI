//! `ObservableLoggingMixinTests` (`ReactiveUI.Tests/Mixins/ObservableLoggingMixinTests.cs`).
//!
//! Upstream runs the class under `LoggingRegistrationExecutor`, which registers a log manager whose
//! loggers write to a `TestLogger`; each test here does it with [`register_test_logger`]. A log
//! entry `Info(CultureInfo.InvariantCulture, "{0} OnNext: {1}", message, x)` is
//! `info_format(format_args!(..))`.

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
#[allow(unused_imports)]
use ferroui_splat::{IEnableLogger, IFullLogger, LogLevel};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const TEST_MESSAGE: &str = "Test";

/// A logging object whose counts are those of the logger of the tests (`TestEnableLogger`).
struct TestEnableLogger {
    logger: Rc<TestLogger>,
}

impl TestEnableLogger {
    fn new() -> Rc<Self> {
        Rc::new(Self { logger: register_test_logger() })
    }

    /// The number of messages at the level `Info` (`InfoCount`).
    fn info_count(&self) -> usize {
        self.logger.count(LogLevel::Info)
    }

    /// The number of messages at the level `Warn` (`WarnCount`).
    fn warn_count(&self) -> usize {
        self.logger.count(LogLevel::Warn)
    }
}

impl IEnableLogger for TestEnableLogger {}

/// Calls the handlers on each notification, then forwards it (`Tap`, the `Do` of three handlers).
fn tap<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    on_next: impl Fn(&T) + 'static,
    on_error: impl Fn(&ObservableError) + 'static,
    on_completed: impl Fn() + 'static,
) -> Rc<dyn IObservable<T>> {
    source.do_all(on_next, move |error| on_error(&error), on_completed)
}

/// What `Log` does, written out (the `Tap` of the tests, and `LogNonGeneric`).
fn log_by_hand<T: std::fmt::Display + 'static>(source: Rc<dyn IObservable<T>>, log_object: Rc<dyn IEnableLogger>, message: &str) -> Rc<dyn IObservable<T>> {
    let (next_object, error_object, completed_object) = (log_object.clone(), log_object.clone(), log_object);
    let (next_message, error_message, completed_message) = (message.to_owned(), message.to_owned(), message.to_owned());
    tap(
        source,
        move |x| next_object.log().info_format(format_args!("{next_message} OnNext: {x}")),
        move |error| error_object.log().warn_exception(&**error, &format!("{error_message} OnError")),
        move || completed_object.log().info_format(format_args!("{completed_message} OnCompleted")),
    )
}

fn recorder() -> (Rc<RefCell<Vec<i32>>>, impl Fn(i32) + 'static) {
    let values = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    (values, move |value| sink.borrow_mut().push(value))
}

#[test]
fn local_interface_variable_works_with_ienable_logger() {
    let logger = TestEnableLogger::new();
    let test_logger = logger.clone();
    let subject = Subject::<i32>::new();

    let logged = log_by_hand(subject.as_observable(), logger.clone(), TEST_MESSAGE);

    let (values, on_next) = recorder();
    let _ = logged.observe_on(immediate()).subscribe_next(on_next);

    subject.on_next(1);
    subject.on_next(2);
    subject.on_completed();

    assert_eq!(*values.borrow(), [1, 2]);
    // Two entries for OnNext and one for OnCompleted.
    assert_eq!(test_logger.info_count(), 3);
}

#[test]
fn log_completes_normally_without_errors() {
    let logger = TestEnableLogger::new();
    let values = Observable::to_observable(vec![1, 2, 3, 4, 5]);

    let logged = values.log(logger.clone());

    let (results, on_next) = recorder();
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();
    let _ = logged.observe_on(immediate()).subscribe_next_completed(on_next, move || flag.set(true));

    assert_eq!(*results.borrow(), [1, 2, 3, 4, 5]);
    assert!(completed.get());
    // Five entries for OnNext and one for OnCompleted.
    assert!(logger.info_count() >= 6);
}

#[test]
fn log_logs_errors() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    let logged = subject.as_observable().log_message(logger.clone(), Some(TEST_MESSAGE));

    let error_caught = Rc::new(Cell::new(false));
    let flag = error_caught.clone();
    let _ = logged.observe_on(immediate()).subscribe_next_error(|_| {}, move |_| flag.set(true));

    subject.on_error(InvalidOperationException::error("Test error"));

    assert!(error_caught.get());
    assert!(logger.warn_count() >= 1);
}

#[test]
fn log_logs_on_next_on_error_and_on_completed() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    let logged = subject.as_observable().log_message(logger.clone(), Some(TEST_MESSAGE));

    let (values, on_next) = recorder();
    let _ = logged.observe_on(immediate()).subscribe_next(on_next);

    subject.on_next(1);
    subject.on_next(2);
    subject.on_completed();

    assert_eq!(*values.borrow(), [1, 2]);
    assert!(logger.info_count() >= 3);
}

#[test]
fn log_with_null_message_uses_empty_string() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    let logged = subject.as_observable().log(logger.clone());

    let (values, on_next) = recorder();
    let _ = logged.observe_on(immediate()).subscribe_next(on_next);

    subject.on_next(1);
    subject.on_completed();

    assert!(values.borrow().contains(&1));
    assert!(logger.info_count() >= 2);
    assert!(logger.logger.messages().iter().any(|(message, _, _)| message == " OnNext: 1"));
}

#[test]
fn log_with_stringifier_uses_custom_conversion() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    let logged = subject.as_observable().log_with_stringifier(logger.clone(), Some(TEST_MESSAGE), |x| format!("Value: {x}"));

    let (values, on_next) = recorder();
    let _ = logged.observe_on(immediate()).subscribe_next(on_next);

    subject.on_next(42);
    subject.on_completed();

    assert!(values.borrow().contains(&42));
    assert!(logger.info_count() >= 2);
    assert!(logger.logger.messages().iter().any(|(message, _, _)| message == "Test OnNext: Value: 42"));
}

#[test]
fn log_extension_method_works_with_test_enable_logger() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    // The Log extension method itself.
    let logged = subject.as_observable().log_message(logger.clone(), Some(TEST_MESSAGE));

    let (values, on_next) = recorder();
    let _ = logged.observe_on(immediate()).subscribe_next(on_next);

    subject.on_next(1);
    subject.on_next(2);
    subject.on_completed();

    assert_eq!(*values.borrow(), [1, 2]);
    assert_eq!(logger.info_count(), 3);
}

#[test]
fn logged_catch_catches_exception_and_returns_next() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();
    let fallback = Observable::return_(99);

    let caught = subject.as_observable().logged_catch_to(logger.clone(), fallback, Some("Error occurred"));

    let (values, on_next) = recorder();
    let _ = caught.observe_on(immediate()).subscribe_next(on_next);

    subject.on_error(InvalidOperationException::error(""));

    assert!(values.borrow().contains(&99));
    assert!(logger.warn_count() >= 1);
}

#[test]
fn logged_catch_with_exception_func_passes_exception_to_factory() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();
    let captured_ex: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let captured = captured_ex.clone();

    let caught = subject.as_observable().logged_catch_of::<_, InvalidOperationException>(
        logger.clone(),
        move |error| {
            *captured.borrow_mut() = Some(error);
            Observable::return_(100)
        },
        None,
    );

    let (values, on_next) = recorder();
    let _ = caught.observe_on(immediate()).subscribe_next(on_next);

    let thrown_exception = InvalidOperationException::error(TEST_MESSAGE);
    subject.on_error(thrown_exception.clone());

    assert!(captured_ex.borrow().as_ref().is_some_and(|error| Rc::ptr_eq(error, &thrown_exception)));
    assert!(values.borrow().contains(&100));
}

#[test]
fn logged_catch_with_exception_type_catches_specific_exception() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    let caught = subject.as_observable().logged_catch_of::<_, InvalidOperationException>(
        logger.clone(),
        |_| Observable::return_(42),
        Some("Specific error"),
    );

    let (values, on_next) = recorder();
    let _ = caught.observe_on(immediate()).subscribe_next(on_next);

    subject.on_error(InvalidOperationException::error(""));

    assert!(values.borrow().contains(&42));
    assert!(logger.warn_count() >= 1);
}

#[test]
fn logged_catch_with_null_message_uses_empty_string() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    let caught = subject.as_observable().logged_catch_to(logger.clone(), Observable::return_(1), None);

    let (values, on_next) = recorder();
    let _ = caught.observe_on(immediate()).subscribe_next(on_next);

    subject.on_error(InvalidOperationException::error(""));

    assert!(values.borrow().contains(&1));
    assert!(logger.warn_count() >= 1);
}

#[test]
fn logged_catch_with_null_next_uses_default() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    let caught = subject.as_observable().logged_catch(logger.clone(), None, Some("Error"));

    let _ = caught.observe_on(immediate()).subscribe_all(|_| {}, |_| {}, || {});

    subject.on_error(InvalidOperationException::error(""));

    assert!(logger.warn_count() >= 1);
}

#[test]
fn manual_inline_works_with_test_enable_logger() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    // What Log() does, written out.
    let logged = log_by_hand(subject.as_observable(), logger.clone(), TEST_MESSAGE);

    let (values, on_next) = recorder();
    let _ = logged.observe_on(immediate()).subscribe_next(on_next);

    subject.on_next(1);
    subject.on_next(2);
    subject.on_completed();

    assert_eq!(*values.borrow(), [1, 2]);
    assert_eq!(logger.info_count(), 3);
}

#[test]
fn non_generic_helper_works_with_ienable_logger() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    // `LogNonGeneric`: the logging object as a trait object.
    let log_object: Rc<dyn IEnableLogger> = logger.clone();
    let logged = log_by_hand(subject.as_observable(), log_object, TEST_MESSAGE);

    let (values, on_next) = recorder();
    let _ = logged.observe_on(immediate()).subscribe_next(on_next);

    subject.on_next(1);
    subject.on_next(2);
    subject.on_completed();

    assert_eq!(*values.borrow(), [1, 2]);
    assert_eq!(logger.info_count(), 3);
}

#[test]
fn test_enable_logger_captures_generic_info_with_two_arguments() {
    let logger = TestEnableLogger::new();
    let subject = Subject::<i32>::new();

    let (next_object, error_object, completed_object) = (logger.clone(), logger.clone(), logger.clone());
    let logged = tap(
        subject.as_observable(),
        move |x| next_object.log().info_format(format_args!("{TEST_MESSAGE} OnNext: {x}")),
        move |error| error_object.log().warn_exception(&**error, "Test OnError"),
        move || completed_object.log().info_format(format_args!("{TEST_MESSAGE} OnCompleted")),
    );

    let (values, on_next) = recorder();
    let _ = logged.observe_on(immediate()).subscribe_next(on_next);

    subject.on_next(1);
    subject.on_next(2);
    subject.on_completed();

    assert_eq!(*values.borrow(), [1, 2]);
    assert_eq!(logger.info_count(), 3);
}

#[test]
fn test_enable_logger_captures_info_and_warn_messages() {
    let logger = TestEnableLogger::new();

    // Info and Warn, called directly.
    logger.log().info_format(format_args!("Test message {}", "arg1"));
    logger.log().warn_exception(&InvalidOperationException::new(""), "Error occurred");

    assert_eq!(logger.info_count(), 1);
    assert_eq!(logger.warn_count(), 1);
}
