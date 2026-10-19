//! Ported from `Splat.Logging.Tests/ActionLoggerTests.cs` of Splat.
//! Adapted: the actions are boxed closures that record into a shared cell, and a `null` action is
//! `None`. The generic `Debug<T>(string)` and its siblings are `debug_for::<T>` of `FullLoggerExt`;
//! a `Type` is the name of the type (`std::any::type_name`).

use super::base_tests::logger_test_constants::TEST_MESSAGE;
use crate::tests::common::{DummyObjectClass1, DummyObjectClass2};
use ferroui_splat::{ActionLogger, FullLoggerExt, ILogger, LogLevel, WrappingFullLogger};
use std::cell::RefCell;
use std::rc::Rc;

type Typed = Rc<RefCell<Option<(String, &'static str, LogLevel)>>>;

/// A full logger over an action logger whose typed action records what it is passed.
fn typed_logger() -> (WrappingFullLogger, Typed) {
    let passed: Typed = Rc::new(RefCell::new(None));
    let sink = passed.clone();
    let logger = ActionLogger::new(
        None,
        Some(Box::new(move |message: &str, type_name: &'static str, level: LogLevel| {
            *sink.borrow_mut() = Some((message.to_owned(), type_name, level));
        })),
        None,
        None,
    );
    (WrappingFullLogger::new(Rc::new(logger)), passed)
}

fn assert_passed(passed: &Typed, level: LogLevel, type_name: &'static str) {
    assert_eq!(*passed.borrow(), Some((TEST_MESSAGE.to_owned(), type_name, level)));
}

#[test]
fn write_should_emit_message() {
    let passed: Rc<RefCell<Option<(String, LogLevel)>>> = Rc::new(RefCell::new(None));
    let sink = passed.clone();
    let logger = ActionLogger::new(
        Some(Box::new(move |message: &str, level: LogLevel| {
            *sink.borrow_mut() = Some((message.to_owned(), level));
        })),
        None,
        None,
        None,
    );

    let full_logger = WrappingFullLogger::new(Rc::new(logger));

    full_logger.write(TEST_MESSAGE, LogLevel::Debug);

    assert_eq!(*passed.borrow(), Some((TEST_MESSAGE.to_owned(), LogLevel::Debug)));
}

#[test]
fn debug_with_generic_type_should_emit_message_and_type() {
    let (full_logger, passed) = typed_logger();
    full_logger.debug_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Debug, std::any::type_name::<DummyObjectClass1>());
}

#[test]
fn debug_with_generic_type_should_emit_message_and_type_provided() {
    let (full_logger, passed) = typed_logger();
    full_logger.debug_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Debug, std::any::type_name::<DummyObjectClass2>());
}

#[test]
fn info_with_generic_type_should_emit_message_and_type() {
    let (full_logger, passed) = typed_logger();
    full_logger.info_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Info, std::any::type_name::<DummyObjectClass1>());
}

#[test]
fn info_with_generic_type_should_emit_message_and_type_provided() {
    let (full_logger, passed) = typed_logger();
    full_logger.info_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Info, std::any::type_name::<DummyObjectClass2>());
}

#[test]
fn warn_with_generic_type_should_emit_message_and_type() {
    let (full_logger, passed) = typed_logger();
    full_logger.warn_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Warn, std::any::type_name::<DummyObjectClass1>());
}

#[test]
fn warn_with_generic_type_should_emit_message_and_type_provided() {
    let (full_logger, passed) = typed_logger();
    full_logger.warn_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Warn, std::any::type_name::<DummyObjectClass2>());
}

#[test]
fn error_with_generic_type_should_emit_message_and_type() {
    let (full_logger, passed) = typed_logger();
    full_logger.error_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Error, std::any::type_name::<DummyObjectClass1>());
}

#[test]
fn error_with_generic_type_should_emit_message_and_type_provided() {
    let (full_logger, passed) = typed_logger();
    full_logger.error_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Error, std::any::type_name::<DummyObjectClass2>());
}

#[test]
fn fatal_with_generic_type_should_emit_message_and_type() {
    let (full_logger, passed) = typed_logger();
    full_logger.fatal_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Fatal, std::any::type_name::<DummyObjectClass1>());
}

#[test]
fn fatal_with_generic_type_should_emit_message_and_type_provided() {
    let (full_logger, passed) = typed_logger();
    full_logger.fatal_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_passed(&passed, LogLevel::Fatal, std::any::type_name::<DummyObjectClass2>());
}
