//! Ported from `Splat.Logging.Tests/WrappingPrefixLoggerCoverageTests.cs` of Splat.
//! Adapted: a `Type` is the name of the type (`std::any::type_name`) and `nameof(T)` its short
//! name; the inner `TextLogger` is shared through an `Rc` so that the test can read its logs.
//! `new("boom")` is `TestException::with_message("boom")`.

use crate::tests::common::{DummyObjectClass1, DummyObjectClass2, IMockLogTarget, TestException, TextLogger};
use ferroui_splat::{ILogger, LogLevel, WrappingPrefixLogger};
use std::rc::Rc;

const TEST_MESSAGE: &str = "This is a test.";

fn last_message(text_logger: &TextLogger) -> String {
    text_logger.logs().last().expect("a message was written").1.clone()
}

fn logger_over(inner: &Rc<TextLogger>) -> WrappingPrefixLogger {
    WrappingPrefixLogger::new(inner.clone(), std::any::type_name::<DummyObjectClass1>())
}

#[test]
fn level_should_reflect_inner_logger_level() {
    let inner = Rc::new(TextLogger::with_level(LogLevel::Warn));
    let logger = logger_over(&inner);

    assert_eq!(logger.level(), LogLevel::Warn);
}

#[test]
fn write_message_should_apply_prefix() {
    let inner = Rc::new(TextLogger::new());
    let logger = logger_over(&inner);

    logger.write(TEST_MESSAGE, LogLevel::Debug);

    assert_eq!(last_message(&inner), "DummyObjectClass1: This is a test.");
}

#[test]
fn write_exception_message_should_apply_prefix() {
    let inner = Rc::new(TextLogger::new());
    let logger = logger_over(&inner);

    logger.write_exception(&TestException::with_message("boom"), TEST_MESSAGE, LogLevel::Error);

    assert!(last_message(&inner).starts_with("DummyObjectClass1: This is a test."));
}

#[test]
fn write_message_with_type_should_use_type_prefix() {
    let inner = Rc::new(TextLogger::new());
    let logger = logger_over(&inner);

    logger.write_with_type(TEST_MESSAGE, std::any::type_name::<DummyObjectClass2>(), LogLevel::Info);

    assert_eq!(last_message(&inner), "DummyObjectClass2: This is a test.");
}

#[test]
fn write_exception_message_with_type_should_use_type_prefix() {
    let inner = Rc::new(TextLogger::new());
    let logger = logger_over(&inner);

    logger.write_exception_with_type(
        &TestException::with_message("boom"),
        TEST_MESSAGE,
        std::any::type_name::<DummyObjectClass2>(),
        LogLevel::Fatal,
    );

    assert!(last_message(&inner).starts_with("DummyObjectClass2: This is a test."));
}

// Not ported: Write_Message_With_Null_Type_Throws - argument-null validation (a null Type), which Rust's types make impossible
// Not ported: Write_Exception_Message_With_Null_Type_Throws - argument-null validation (a null Type), which Rust's types make impossible
