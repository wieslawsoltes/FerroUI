//! Ported from `Splat.Logging.Tests/WrappingFullLoggers/WrappingFullLoggerExceptionMessageTests.cs`
//! of Splat.
//! Adapted: `Debug(Exception, string?)` is `debug_exception`, and the deprecated
//! `DebugException(string?, Exception)` is `debug_exception_message` (the tests allow the
//! deprecation); the `InvalidOperationException` is a `TestException` with the same message. The
//! tests of a `null` message are not ported: a `&str` cannot be `null`.

use super::wrapping_full_logger_test_factory::create_logger;
use crate::tests::common::{IMockLogTarget, TestException, TextLogger};
use ferroui_splat::{IFullLogger, LogLevel};

const EXCEPTION_MESSAGE: &str = "explosion-detail";

/// The exception of the tests (`new InvalidOperationException("wrapped-failure")`).
fn exception() -> TestException {
    TestException::with_message("wrapped-failure")
}

/// The last entry contains `expected_text`, at `expected_level` (`AssertContains`).
fn assert_contains(target: &TextLogger, expected_text: &str, expected_level: LogLevel) {
    let logs = target.logs();
    let entry = logs.last().unwrap();
    assert!(entry.1.contains(expected_text));
    assert_eq!(entry.0, expected_level);
}

#[test]
fn debug_exception_message_forwards_at_debug() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.debug_exception(&exception(), EXCEPTION_MESSAGE);

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Debug);
}

#[test]
#[allow(deprecated)]
fn debug_exception_forwards_at_debug() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.debug_exception_message(EXCEPTION_MESSAGE, &exception());

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Debug);
}

#[test]
fn info_exception_message_forwards_at_info() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.info_exception(&exception(), EXCEPTION_MESSAGE);

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Info);
}

#[test]
#[allow(deprecated)]
fn info_exception_forwards_at_info() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.info_exception_message(EXCEPTION_MESSAGE, &exception());

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Info);
}

#[test]
fn warn_exception_message_forwards_at_warn() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.warn_exception(&exception(), EXCEPTION_MESSAGE);

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Warn);
}

#[test]
#[allow(deprecated)]
fn warn_exception_forwards_at_warn() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.warn_exception_message(EXCEPTION_MESSAGE, &exception());

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Warn);
}

#[test]
fn error_exception_message_forwards_at_error() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.error_exception(&exception(), EXCEPTION_MESSAGE);

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Error);
}

#[test]
#[allow(deprecated)]
fn error_exception_forwards_at_error() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.error_exception_message(EXCEPTION_MESSAGE, &exception());

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Error);
}

#[test]
fn fatal_exception_message_forwards_at_fatal() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.fatal_exception(&exception(), EXCEPTION_MESSAGE);

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Fatal);
}

#[test]
#[allow(deprecated)]
fn fatal_exception_forwards_at_fatal() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.fatal_exception_message(EXCEPTION_MESSAGE, &exception());

    assert_contains(&target, EXCEPTION_MESSAGE, LogLevel::Fatal);
}

// Not ported: Debug_Exception_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Info_Exception_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Warn_Exception_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Error_Exception_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Fatal_Exception_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
