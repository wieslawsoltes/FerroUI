//! Ported from `Splat.Logging.Tests/WrappingFullLoggerFormatProviderTests.cs` of Splat.
//! Adapted: Rust's formatting does not depend on a culture, so `Debug<T>(IFormatProvider, T)` and
//! its siblings are the `_format` methods of the level with `format_args!("{}", value)`; the
//! invariant culture of upstream formats the value as Rust does.

use crate::tests::common::{IMockLogTarget, TextLogger};
use ferroui_splat::{IFullLogger, LogLevel, WrappingFullLogger};
use std::rc::Rc;

const SAMPLE_VALUE: i32 = 42;

const EXPECTED_TEXT: &str = "42";

fn create_logger() -> (WrappingFullLogger, Rc<TextLogger>) {
    let target = Rc::new(TextLogger::with_level(LogLevel::Debug));
    (WrappingFullLogger::new(target.clone()), target)
}

fn assert_last_entry(target: &TextLogger, level: LogLevel) {
    let logs = target.logs();
    let (log_level, message) = logs.last().expect("a message was written");
    assert_eq!(message, EXPECTED_TEXT);
    assert_eq!(*log_level, level);
}

#[test]
fn debug_with_format_provider_value_forwards_formatted_text() {
    let (logger, target) = create_logger();

    logger.debug_format(format_args!("{}", SAMPLE_VALUE));

    assert_last_entry(&target, LogLevel::Debug);
}

#[test]
fn info_with_format_provider_value_forwards_formatted_text() {
    let (logger, target) = create_logger();

    logger.info_format(format_args!("{}", SAMPLE_VALUE));

    assert_last_entry(&target, LogLevel::Info);
}

#[test]
fn warn_with_format_provider_value_forwards_formatted_text() {
    let (logger, target) = create_logger();

    logger.warn_format(format_args!("{}", SAMPLE_VALUE));

    assert_last_entry(&target, LogLevel::Warn);
}

#[test]
fn error_with_format_provider_value_forwards_formatted_text() {
    let (logger, target) = create_logger();

    logger.error_format(format_args!("{}", SAMPLE_VALUE));

    assert_last_entry(&target, LogLevel::Error);
}

#[test]
fn fatal_with_format_provider_value_forwards_formatted_text() {
    let (logger, target) = create_logger();

    logger.fatal_format(format_args!("{}", SAMPLE_VALUE));

    assert_last_entry(&target, LogLevel::Fatal);
}
