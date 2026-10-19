//! Ported from `Splat.Logging.Tests/NullLoggerTests.cs` of Splat.
//! Adapted: a `Type` is the name of the type (`std::any::type_name`); `ThrowsNothing` is a call
//! that does not panic.

use crate::tests::common::TestException;
use ferroui_splat::{ILogger, LogLevel, NullLogger};

/// The type that writes the typed messages (`typeof(NullLoggerTests)`).
#[allow(dead_code)] // only named as a type
struct NullLoggerTests;

#[test]
fn write_should_not_throw() {
    let logger = NullLogger::new();
    let this_type = std::any::type_name::<NullLoggerTests>();

    logger.write("Test message", LogLevel::Debug);
    logger.write_with_type("Test message", this_type, LogLevel::Info);
    logger.write_exception(&TestException::with_message("Test"), "Test exception", LogLevel::Error);
    logger.write_exception_with_type(&TestException::with_message("Test"), "Test exception", this_type, LogLevel::Fatal);
}

#[test]
fn level_should_be_settable() {
    let logger = NullLogger::new();
    logger.set_level(LogLevel::Debug);

    assert_eq!(logger.level(), LogLevel::Debug);

    logger.set_level(LogLevel::Error);
    assert_eq!(logger.level(), LogLevel::Error);
}
