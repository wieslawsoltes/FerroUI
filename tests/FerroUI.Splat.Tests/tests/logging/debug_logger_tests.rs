//! Ported from `Splat.Logging.Tests/DebugLoggerTests.cs` of Splat.
//! Adapted: the debug output is the standard error in the port, which the test does not capture;
//! as upstream, it checks that the writes do not panic. A `Type` is the name of the type
//! (`std::any::type_name`).

use crate::tests::common::TestException;
use ferroui_splat::{DebugLogger, ILogger, LogLevel};

/// The type that writes the typed messages (`typeof(DebugLoggerTests)`).
#[allow(dead_code)] // only named as a type
struct DebugLoggerTests;

#[test]
fn write_should_write_to_debug_output() {
    let logger = DebugLogger::new();
    logger.set_level(LogLevel::Debug);
    let this_type = std::any::type_name::<DebugLoggerTests>();

    logger.write("Test message", LogLevel::Debug);
    logger.write_with_type("Test message", this_type, LogLevel::Info);
    logger.write_exception(&TestException::with_message("Test"), "Test exception", LogLevel::Error);
    logger.write_exception_with_type(&TestException::with_message("Test"), "Test exception", this_type, LogLevel::Fatal);
}

#[test]
fn level_should_be_settable() {
    let logger = DebugLogger::new();
    logger.set_level(LogLevel::Warn);

    assert_eq!(logger.level(), LogLevel::Warn);

    logger.set_level(LogLevel::Error);
    assert_eq!(logger.level(), LogLevel::Error);
}
