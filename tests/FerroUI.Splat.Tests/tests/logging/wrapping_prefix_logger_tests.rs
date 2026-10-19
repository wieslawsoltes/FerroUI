//! Ported from `Splat.Logging.Tests/WrappingPrefixLoggerTests.cs` of Splat.
//! Adapted: a `Type` is the name of the type (`std::any::type_name`) and `nameof(T)` its short
//! name; the generic `Debug<T>(string)` and its siblings are `debug_for::<T>` of `FullLoggerExt`.
//! The inner `TextLogger` is shared through an `Rc` so that the test can read its logs.

use super::base_tests::logger_test_constants::TEST_MESSAGE;
use crate::tests::common::{DummyObjectClass1, DummyObjectClass2, FormatHelper, IMockLogTarget, TextLogger};
use ferroui_splat::{FullLoggerExt, ILogger, LogLevel, WrappingFullLogger, WrappingPrefixLogger};
use std::rc::Rc;

/// A full logger over a prefix logger for `DummyObjectClass1` over a new `TextLogger`.
fn create_logger() -> (WrappingFullLogger, Rc<TextLogger>) {
    let text_logger = Rc::new(TextLogger::new());
    let logger = WrappingFullLogger::new(Rc::new(WrappingPrefixLogger::new(
        text_logger.clone(),
        std::any::type_name::<DummyObjectClass1>(),
    )));
    (logger, text_logger)
}

/// `Logs.Last().message.Trim(NewLine).Trim()`.
fn last_message(text_logger: &TextLogger) -> String {
    let logs = text_logger.logs();
    let (_, message) = logs.last().expect("a message was written");
    FormatHelper::trim(message).to_owned()
}

const EXPECTED_1: &str = "DummyObjectClass1: This is a test.";

const EXPECTED_2: &str = "DummyObjectClass2: This is a test.";

#[test]
fn write_should_write_message() {
    let (logger, text_logger) = create_logger();

    logger.write(TEST_MESSAGE, LogLevel::Debug);

    assert_eq!(last_message(&text_logger), EXPECTED_1);
}

#[test]
fn write_should_write_message_and_type() {
    let (logger, text_logger) = create_logger();

    logger.write_with_type(TEST_MESSAGE, std::any::type_name::<DummyObjectClass1>(), LogLevel::Debug);

    assert_eq!(last_message(&text_logger), EXPECTED_1);
}

#[test]
fn debug_with_generic_type_should_write_message_and_type() {
    let (logger, text_logger) = create_logger();
    logger.debug_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_1);
}

#[test]
fn debug_with_generic_type_should_write_message_and_type_provided() {
    let (logger, text_logger) = create_logger();
    logger.debug_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_2);
}

#[test]
fn info_with_generic_type_should_write_message_and_type() {
    let (logger, text_logger) = create_logger();
    logger.info_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_1);
}

#[test]
fn info_with_generic_type_should_write_message_and_type_provided() {
    let (logger, text_logger) = create_logger();
    logger.info_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_2);
}

#[test]
fn warn_with_generic_type_should_write_message_and_type() {
    let (logger, text_logger) = create_logger();
    logger.warn_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_1);
}

#[test]
fn warn_with_generic_type_should_write_message_and_type_provided() {
    let (logger, text_logger) = create_logger();
    logger.warn_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_2);
}

#[test]
fn error_with_generic_type_should_write_message_and_type() {
    let (logger, text_logger) = create_logger();
    logger.error_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_1);
}

#[test]
fn error_with_generic_type_should_write_message_and_type_provided() {
    let (logger, text_logger) = create_logger();
    logger.error_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_2);
}

#[test]
fn fatal_with_generic_type_should_write_message_and_type() {
    let (logger, text_logger) = create_logger();
    logger.fatal_for::<DummyObjectClass1>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_1);
}

#[test]
fn fatal_with_generic_type_should_write_message_and_type_provided() {
    let (logger, text_logger) = create_logger();
    logger.fatal_for::<DummyObjectClass2>(TEST_MESSAGE);
    assert_eq!(last_message(&text_logger), EXPECTED_2);
}
