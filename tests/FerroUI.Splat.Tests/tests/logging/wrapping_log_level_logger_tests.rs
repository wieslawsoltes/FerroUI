//! Ported from `Splat.Logging.Tests/WrappingLogLevelLoggerTests.cs` of Splat.
//! Adapted: the inner `TextLogger` is shared through an `Rc` so that the test can read its logs.

use crate::tests::common::{IMockLogTarget, TextLogger};
use ferroui_splat::{ILogger, LogLevel, WrappingLogLevelLogger};
use std::rc::Rc;

const EXPECTED_LOG_COUNT: usize = 3;

const WARN_LOG_INDEX: usize = 2;

#[test]
fn write_should_add_log_level_prefix() {
    let inner = Rc::new(TextLogger::with_level(LogLevel::Debug));
    let logger = WrappingLogLevelLogger::new(inner.clone());

    logger.write("Test message", LogLevel::Debug);
    logger.write("Info message", LogLevel::Info);
    logger.write("Warn message", LogLevel::Warn);

    let logs_list = inner.logs();

    assert_eq!(logs_list.len(), EXPECTED_LOG_COUNT);
    assert_eq!(logs_list[0].1, "Debug: Test message");
    assert_eq!(logs_list[1].1, "Info: Info message");
    assert_eq!(logs_list[WARN_LOG_INDEX].1, "Warn: Warn message");
}

#[test]
fn level_should_be_forwarded_from_inner() {
    let inner = Rc::new(TextLogger::with_level(LogLevel::Error));
    let logger = WrappingLogLevelLogger::new(inner);

    assert_eq!(logger.level(), LogLevel::Error);
}
