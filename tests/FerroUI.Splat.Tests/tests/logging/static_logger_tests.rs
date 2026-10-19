//! Ported from `Splat.Logging.Tests/StaticLoggerTests.cs` of Splat.
//! Adapted: the abstract `BaseStaticLoggerTests` is the macro `base_static_logger_tests!`, which
//! takes the level and the methods of the level of `IStaticFullLogger` (the message, exception,
//! format and generic methods); each nested test class of upstream is a submodule that invokes it.
//! `[CallerMemberName]` is the explicit `caller_member_name` argument: the tests pass the name of
//! the upstream member the lambda is written in (`GetMethodToWriteMessage`, ...) or the test
//! method itself, so that the expected texts match upstream. The overloads with an
//! `IFormatProvider` and one to three generic arguments are the `_format` method of the level with
//! `format_args!`; Rust rejects a format argument that the format string does not use, so the
//! extra arguments of the two- and three-argument tests are part of the message (upstream formats
//! `"{0}"` and drops them; the tests only check that one message with a caller suffix is written).
//! `Type` is the name of the type (`std::any::type_name`).

use crate::tests::common::{DummyObjectClass1, DummyObjectClass2, FormatHelper, IMockLogTarget, TextLogger, TestException};
use ferroui_splat::{IStaticFullLogger, LogLevel, StaticFullLogger, StaticFullLoggerExt, WrappingFullLogger};
use std::rc::Rc;

const TEST_MESSAGE: &str = "Message";

const THIRD_ARGUMENT_FLOAT_VALUE: f32 = 2.3;

fn get_logger_over(text_logger: &Rc<TextLogger>) -> StaticFullLogger {
    StaticFullLogger::new(Rc::new(WrappingFullLogger::new(text_logger.clone())))
}

fn get_logger(log_level: LogLevel) -> StaticFullLogger {
    let text_logger = Rc::new(TextLogger::with_level(log_level));
    get_logger_over(&text_logger)
}

/// `Test_Write_Message`: the test method writes one message, which ends with a caller suffix.
fn test_write_message(test_method_func: impl FnOnce(&StaticFullLogger)) {
    let text_logger = Rc::new(TextLogger::new());
    let static_logger = get_logger_over(&text_logger);

    assert_eq!(text_logger.logs().len(), 0);

    test_method_func(&static_logger);

    let logs = text_logger.logs();
    assert_eq!(logs.len(), 1);

    let line = &logs[0].1;
    let start_of_caller_member_suffix = line.find('(').expect("the message has a caller suffix");
    assert!(start_of_caller_member_suffix > 0);

    let end_of_caller_member_suffix = start_of_caller_member_suffix
        + line[start_of_caller_member_suffix..].find(')').expect("the caller suffix is closed");
    assert!(end_of_caller_member_suffix > start_of_caller_member_suffix + 1);
}

/// The last message of `text_logger`, trimmed (`Logs.Last().message.Trim(NewLine).Trim()`).
fn last_message(text_logger: &TextLogger) -> String {
    let logs = text_logger.logs();
    let (_, message) = logs.last().expect("a message was written");
    FormatHelper::trim(message).to_owned()
}

/// `BaseStaticLoggerTests`, for the level `$level` whose methods of `IStaticFullLogger` are
/// `$write` (`Debug(string)`), `$write_exception` (`Debug(Exception, string)`), `$write_format`
/// (`Debug(IFormatProvider, string, ...)`) and `$write_for` (`Debug<T>(string)`).
macro_rules! base_static_logger_tests {
    ($level:expr, $write:ident, $write_exception:ident, $write_format:ident, $write_for:ident) => {
        use super::*;

        fn get_log_level() -> LogLevel {
            $level
        }

        #[test]
        fn method_with_generic_type_should_write_message_and_type() {
            let text_logger = Rc::new(TextLogger::with_level(get_log_level()));
            let logger = get_logger_over(&text_logger);

            logger.$write_for::<DummyObjectClass1>(
                "This is a test.",
                Some("GetMethodWithGenericTypeShouldWriteMessageAndType"),
            );

            assert_eq!(
                last_message(&text_logger),
                "This is a test. (GetMethodWithGenericTypeShouldWriteMessageAndType)"
            );
        }

        #[test]
        fn method_with_generic_type_should_write_message_and_type_provided() {
            let text_logger = Rc::new(TextLogger::with_level(get_log_level()));
            let logger = get_logger_over(&text_logger);

            logger.$write_for::<DummyObjectClass2>(
                "This is a test.",
                Some("GetMethodWithGenericTypeShouldWriteMessageAndType"),
            );

            assert_eq!(
                last_message(&text_logger),
                "This is a test. (GetMethodWithGenericTypeShouldWriteMessageAndType)"
            );
        }

        #[test]
        fn method_writes_message() {
            test_write_message(|logger| logger.$write(TEST_MESSAGE, Some("GetMethodToWriteMessage")));
        }

        #[test]
        fn method_writes_message_invariant_culture() {
            test_write_message(|logger| {
                logger.$write_format(
                    format_args!("{}", TEST_MESSAGE),
                    Some("GetMethodToWriteMessageWithInvariantCulture"),
                )
            });
        }

        #[test]
        fn method_writes_message_invariant_culture_with_generic_two_args() {
            test_write_message(|logger| {
                logger.$write_format(
                    format_args!("{} {}", TEST_MESSAGE, 1),
                    Some("GetMethodToWriteMessageWithInvariantCultureWithTwoGenericArgs"),
                )
            });
        }

        #[test]
        fn method_writes_message_invariant_culture_with_generic_three_args() {
            test_write_message(|logger| {
                logger.$write_format(
                    format_args!("{} {} {}", TEST_MESSAGE, 1, THIRD_ARGUMENT_FLOAT_VALUE),
                    Some("GetMethodToWriteMessageWithInvariantCultureWithThreeGenericArgs"),
                )
            });
        }

        #[test]
        fn method_writes_exception_and_message() {
            let static_logger = get_logger(get_log_level());
            static_logger.$write_exception(
                &TestException::with_message("TEST"),
                TEST_MESSAGE,
                Some("GetMethodToWriteExceptionAndMessage"),
            );
        }

        #[test]
        fn direct_write_at_level_message() {
            let static_logger = get_logger(get_log_level());
            static_logger.write(TEST_MESSAGE, get_log_level(), Some("Direct_Write_At_Level_Message"));
        }

        #[test]
        fn direct_write_at_level_message_and_type() {
            let static_logger = get_logger(get_log_level());
            static_logger.write_with_type(
                TEST_MESSAGE,
                std::any::type_name::<DummyObjectClass1>(),
                get_log_level(),
                Some("Direct_Write_At_Level_Message_And_Type"),
            );
        }

        #[test]
        fn direct_write_at_level_message_and_exception() {
            let static_logger = get_logger(get_log_level());
            let exception = TestException::with_message("bleh");
            static_logger.write_exception(
                &exception,
                TEST_MESSAGE,
                get_log_level(),
                Some("Direct_Write_At_Level_Message_And_Exception"),
            );
        }

        #[test]
        fn direct_write_at_level_message_exception_and_type() {
            let static_logger = get_logger(get_log_level());
            let exception = TestException::with_message("bleh");
            static_logger.write_exception_with_type(
                &exception,
                TEST_MESSAGE,
                std::any::type_name::<DummyObjectClass1>(),
                get_log_level(),
                Some("Direct_Write_At_Level_Message_Exception_And_Type"),
            );
        }

        #[test]
        fn logger_level_at_boundary_should_be_correct() {
            let level = get_log_level();
            let logger = get_logger(level);

            assert_eq!(logger.level(), level);
        }
    };
}

mod debug_static_logger_tests {
    base_static_logger_tests!(LogLevel::Debug, debug, debug_exception, debug_format, debug_for);
}

mod info_static_logger_tests {
    base_static_logger_tests!(LogLevel::Info, info, info_exception, info_format, info_for);
}

mod warning_static_logger_tests {
    base_static_logger_tests!(LogLevel::Warn, warn, warn_exception, warn_format, warn_for);
}

mod error_static_logger_tests {
    base_static_logger_tests!(LogLevel::Error, error, error_exception, error_format, error_for);
}

mod fatal_static_logger_tests {
    base_static_logger_tests!(LogLevel::Fatal, fatal, fatal_exception, fatal_format, fatal_for);
}

// Not ported: Constructor_WithNullLogger_ThrowsArgumentNullException - argument-null validation, which Rust's types make impossible
