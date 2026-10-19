//! Ported from `Splat.Logging.Tests/DebugLoggerCoverageTests.cs` of Splat.
//! Adapted: the debug output is the standard error in the port, so the tests check, as upstream
//! does, only that the writes do not panic. A `Type` is the name of the type
//! (`std::any::type_name`); `new("ex")` is `TestException::with_message("ex")`.

use crate::tests::common::TestException;
use ferroui_splat::{DebugLogger, ILogger, LogLevel};

const DEBUG_MESSAGE: &str = "debug";

const SAMPLE_MESSAGE: &str = "message";

/// The type that writes the typed messages (`typeof(DebugLoggerCoverageTests)`).
#[allow(dead_code)] // only named as a type
struct DebugLoggerCoverageTests;

fn this_type() -> &'static str {
    std::any::type_name::<DebugLoggerCoverageTests>()
}

fn logger_at(level: LogLevel) -> DebugLogger {
    let logger = DebugLogger::new();
    logger.set_level(level);
    logger
}

#[test]
fn level_is_settable() {
    let logger = logger_at(LogLevel::Warn);

    assert_eq!(logger.level(), LogLevel::Warn);
}

#[test]
fn write_at_or_above_level_should_not_throw() {
    let logger = logger_at(LogLevel::Info);

    logger.write("info", LogLevel::Info);
    logger.write("warn", LogLevel::Warn);
    logger.write_exception(&TestException::with_message("ex"), "error", LogLevel::Error);
    logger.write_with_type("fatal", this_type(), LogLevel::Fatal);
    logger.write_exception_with_type(&TestException::with_message("ex"), "fatal", this_type(), LogLevel::Fatal);
}

#[test]
fn write_below_level_should_be_gated_without_throwing() {
    let logger = logger_at(LogLevel::Fatal);

    logger.write(DEBUG_MESSAGE, LogLevel::Debug);
    logger.write_exception(&TestException::with_message("ex"), DEBUG_MESSAGE, LogLevel::Debug);
    logger.write_with_type(DEBUG_MESSAGE, this_type(), LogLevel::Info);
    logger.write_exception_with_type(&TestException::with_message("ex"), DEBUG_MESSAGE, this_type(), LogLevel::Warn);
}

#[test]
fn write_all_overloads_across_all_levels_should_not_throw() {
    let logger = logger_at(LogLevel::Debug);

    for level in [LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error, LogLevel::Fatal] {
        logger.write(SAMPLE_MESSAGE, level);
        logger.write_exception(&TestException::with_message("ex"), SAMPLE_MESSAGE, level);
        logger.write_with_type(SAMPLE_MESSAGE, this_type(), level);
        logger.write_exception_with_type(&TestException::with_message("ex"), SAMPLE_MESSAGE, this_type(), level);
    }
}
