//! Ported from `Splat.Logging.Tests/WrappingFullLoggers` of Splat: the tests of
//! `WrappingFullLogger`, over the console logger and over a `TextLogger` that records what the
//! wrapping logger forwards.

mod console_logger_tests;
mod wrapping_full_logger_deferred_message_tests;
mod wrapping_full_logger_exception_message_tests;
mod wrapping_full_logger_fatal_suppression_tests;
mod wrapping_full_logger_null_message_tests;
mod wrapping_full_logger_parameterized_message_tests;
mod wrapping_full_logger_test_factory;
mod wrapping_full_logger_value_tests;
