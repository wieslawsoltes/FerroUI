//! Ported from
//! `Splat.Logging.Tests/WrappingFullLoggers/WrappingFullLoggerParameterizedMessageTests.cs` of
//! Splat.
//! Adapted: `Debug(string, params object[])` is `debug_format(format_args!(..))`, and
//! `Debug<T>(string, params object[])` is `debug_format_for::<T>(format_args!(..))`; the composite
//! format `Format1` is the literal `"{}"`.

use super::wrapping_full_logger_test_factory::create_logger;
use crate::tests::common::{DummyObjectClass1, IMockLogTarget, TextLogger};
use crate::tests::logging::base_tests::logger_test_constants::{ARG1, EXPECTED1};
use ferroui_splat::{FullLoggerExt, IFullLogger, LogLevel};

/// The last entry is the rendered text, at `expected_level` (`AssertLast`).
fn assert_last(target: &TextLogger, expected_level: LogLevel) {
    let logs = target.logs();
    let entry = logs.last().unwrap();
    assert_eq!(entry.1, EXPECTED1);
    assert_eq!(entry.0, expected_level);
}

#[test]
fn debug_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.debug_format(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Debug);
}

#[test]
fn debug_generic_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Debug);

    logger.debug_format_for::<DummyObjectClass1>(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Debug);
}

#[test]
fn info_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Info);

    logger.info_format(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Info);
}

#[test]
fn info_generic_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Info);

    logger.info_format_for::<DummyObjectClass1>(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Info);
}

#[test]
fn warn_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Warn);

    logger.warn_format(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Warn);
}

#[test]
fn warn_generic_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Warn);

    logger.warn_format_for::<DummyObjectClass1>(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Warn);
}

#[test]
fn error_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Error);

    logger.error_format(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Error);
}

#[test]
fn error_generic_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Error);

    logger.error_format_for::<DummyObjectClass1>(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Error);
}

#[test]
fn fatal_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Fatal);

    logger.fatal_format(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Fatal);
}

#[test]
fn fatal_generic_params_forwards_rendered_text() {
    let (logger, target) = create_logger(LogLevel::Fatal);

    logger.fatal_format_for::<DummyObjectClass1>(format_args!("{}", ARG1));

    assert_last(&target, LogLevel::Fatal);
}
