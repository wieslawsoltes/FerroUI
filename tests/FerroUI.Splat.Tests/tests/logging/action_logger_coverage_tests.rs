//! Ported from `Splat.Logging.Tests/ActionLoggerCoverageTests.cs` of Splat.
//! Adapted: the actions are boxed closures that record into shared cells, and a `null` action is
//! `None`. A `Type` is the name of the type (`std::any::type_name`). `IsSameReferenceAs` on the
//! exception compares the address of the `&dyn Error` the action receives with the address of the
//! exception that was written.

use super::base_tests::logger_test_constants::TEST_MESSAGE;
use crate::tests::common::TestException;
use ferroui_splat::{ActionLogger, ILogger, LogLevel};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;

/// The type that writes the typed messages (`typeof(ActionLoggerCoverageTests)`).
#[allow(dead_code)] // only named as a type
struct ActionLoggerCoverageTests;

fn address_of(exception: &dyn Error) -> *const () {
    std::ptr::from_ref(exception).cast::<()>()
}

#[test]
fn level_should_be_settable() {
    let logger = ActionLogger::new(None, None, None, None);
    logger.set_level(LogLevel::Warn);

    assert_eq!(logger.level(), LogLevel::Warn);
}

#[test]
fn write_message_invokes_no_type_action() {
    let captured: Rc<RefCell<Option<(String, LogLevel)>>> = Rc::new(RefCell::new(None));
    let sink = captured.clone();
    let logger = ActionLogger::new(
        Some(Box::new(move |m: &str, l: LogLevel| {
            *sink.borrow_mut() = Some((m.to_owned(), l));
        })),
        None,
        None,
        None,
    );

    logger.write(TEST_MESSAGE, LogLevel::Info);

    assert_eq!(*captured.borrow(), Some((TEST_MESSAGE.to_owned(), LogLevel::Info)));
}

#[test]
fn write_message_with_type_invokes_with_type_action() {
    let captured: Rc<RefCell<Option<(String, &'static str, LogLevel)>>> = Rc::new(RefCell::new(None));
    let sink = captured.clone();
    let logger = ActionLogger::new(
        None,
        Some(Box::new(move |m: &str, t: &'static str, l: LogLevel| {
            *sink.borrow_mut() = Some((m.to_owned(), t, l));
        })),
        None,
        None,
    );

    logger.write_with_type(TEST_MESSAGE, std::any::type_name::<ActionLoggerCoverageTests>(), LogLevel::Warn);

    let captured = captured.borrow();
    let (message, type_name, level) = captured.as_ref().expect("the action was called");
    assert_eq!(message, TEST_MESSAGE);
    assert_eq!(*type_name, std::any::type_name::<ActionLoggerCoverageTests>());
    assert_eq!(*level, LogLevel::Warn);
}

#[test]
fn write_exception_message_invokes_no_type_exception_action() {
    let captured_exception = Rc::new(Cell::new(std::ptr::null::<()>()));
    let captured: Rc<RefCell<Option<(String, LogLevel)>>> = Rc::new(RefCell::new(None));
    let exception_sink = captured_exception.clone();
    let sink = captured.clone();
    let expected = TestException::with_message("boom");
    let logger = ActionLogger::new(
        None,
        None,
        Some(Box::new(move |e: &dyn Error, m: &str, l: LogLevel| {
            exception_sink.set(address_of(e));
            *sink.borrow_mut() = Some((m.to_owned(), l));
        })),
        None,
    );

    logger.write_exception(&expected, TEST_MESSAGE, LogLevel::Error);

    assert_eq!(captured_exception.get(), address_of(&expected));
    assert_eq!(*captured.borrow(), Some((TEST_MESSAGE.to_owned(), LogLevel::Error)));
}

#[test]
fn write_exception_message_with_type_invokes_with_type_exception_action() {
    let captured_exception = Rc::new(Cell::new(std::ptr::null::<()>()));
    let captured: Rc<RefCell<Option<(String, &'static str, LogLevel)>>> = Rc::new(RefCell::new(None));
    let exception_sink = captured_exception.clone();
    let sink = captured.clone();
    let expected = TestException::with_message("boom");
    let logger = ActionLogger::new(
        None,
        None,
        None,
        Some(Box::new(move |e: &dyn Error, m: &str, t: &'static str, l: LogLevel| {
            exception_sink.set(address_of(e));
            *sink.borrow_mut() = Some((m.to_owned(), t, l));
        })),
    );

    logger.write_exception_with_type(
        &expected,
        TEST_MESSAGE,
        std::any::type_name::<ActionLoggerCoverageTests>(),
        LogLevel::Fatal,
    );

    assert_eq!(captured_exception.get(), address_of(&expected));
    let captured = captured.borrow();
    let (message, type_name, level) = captured.as_ref().expect("the action was called");
    assert_eq!(message, TEST_MESSAGE);
    assert_eq!(*type_name, std::any::type_name::<ActionLoggerCoverageTests>());
    assert_eq!(*level, LogLevel::Fatal);
}

#[test]
fn write_with_null_actions_should_not_throw() {
    let logger = ActionLogger::new(None, None, None, None);
    let type_name = std::any::type_name::<ActionLoggerCoverageTests>();

    logger.write(TEST_MESSAGE, LogLevel::Debug);
    logger.write_with_type(TEST_MESSAGE, type_name, LogLevel::Debug);
    logger.write_exception(&TestException::with_message("boom"), TEST_MESSAGE, LogLevel::Debug);
    logger.write_exception_with_type(&TestException::with_message("boom"), TEST_MESSAGE, type_name, LogLevel::Debug);
}
