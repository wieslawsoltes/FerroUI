//! Ported from `Splat.Logging.Tests/BaseTests/LoggerBase.cs` of Splat.
//! Adapted: the abstract class `LoggerBase<T>` is the macro `logger_base!`, which expands to its
//! tests in the module that invokes it, given the expression of upstream's `GetLogger(LogLevel
//! minimumLogLevel)`: anything callable with a `LogLevel` that returns the logger (an `ILogger`)
//! and the target that records what it writes (an `IMockLogTarget`), for example a function
//! `fn(LogLevel) -> (Rc<dyn IFullLogger>, Rc<dyn IMockLogTarget>)`. Each macro of the chain of
//! bases expands only its own tests, so a test file invokes the macro of each base it inherits
//! upstream, each in a module of its own. A `Type` is its name (`std::any::type_name`).
//!
//! The module `prelude` holds the names the macros of the bases use; each macro imports it with a
//! glob, so that the imports of two expansions never clash.

/// The names the test bases use in their expansions (the logging traits, the levels, the helpers
/// and the constants of the tests).
pub(crate) mod prelude {
    #![allow(unused_imports)]

    pub(crate) use crate::tests::common::{
        DummyObjectClass1, DummyObjectClass2, FormatHelper, IMockLogTarget,
    };
    pub(crate) use crate::tests::logging::base_tests::logger_test_constants::*;
    pub(crate) use ferroui_splat::{
        FullLoggerExt, IAllocationFreeErrorLogger, IAllocationFreeLogger, IFullLogger, ILogger,
        LogLevel,
    };
    pub(crate) use std::cell::Cell;
}

/// The tests of every logger (`LoggerBase<T>`): its level, and `Write` in each of its forms, with
/// and without a level high enough. `$get_logger` makes the logger and its target for a level.
#[allow(unused_macros)]
macro_rules! logger_base {
    ($get_logger:expr) => {
        #[allow(unused_imports)]
        use $crate::tests::logging::base_tests::logger_base::prelude::*;

        #[test]
        fn logger_level_debug_should_be_correct() {
            let (logger, _target) = ($get_logger)(LogLevel::Debug);

            assert_eq!(logger.level(), LogLevel::Debug);
        }

        #[test]
        fn logger_level_info_should_be_correct() {
            let (logger, _target) = ($get_logger)(LogLevel::Info);

            assert_eq!(logger.level(), LogLevel::Info);
        }

        #[test]
        fn logger_level_warn_should_be_correct() {
            let (logger, _target) = ($get_logger)(LogLevel::Warn);

            assert_eq!(logger.level(), LogLevel::Warn);
        }

        #[test]
        fn logger_level_error_should_be_correct() {
            let (logger, _target) = ($get_logger)(LogLevel::Error);

            assert_eq!(logger.level(), LogLevel::Error);
        }

        #[test]
        fn logger_level_fatal_should_be_correct() {
            let (logger, _target) = ($get_logger)(LogLevel::Fatal);

            assert_eq!(logger.level(), LogLevel::Fatal);
        }

        #[test]
        fn write_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.write(TEST_MESSAGE, LogLevel::Debug);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn info_write_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.write(TEST_MESSAGE, LogLevel::Info);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn warn_write_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.write(TEST_MESSAGE, LogLevel::Warn);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn error_write_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.write(TEST_MESSAGE, LogLevel::Error);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn fatal_write_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.write(TEST_MESSAGE, LogLevel::Fatal);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn debug_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.write_exception(&FormatHelper::exception(), "1", LogLevel::Debug);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }

        #[test]
        fn debug_exception_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.write_exception(&FormatHelper::exception(), "1", LogLevel::Debug);

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.write_exception(&FormatHelper::exception(), "1", LogLevel::Info);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }

        #[test]
        fn info_exception_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.write_exception(&FormatHelper::exception(), "1", LogLevel::Info);

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.write_exception(&FormatHelper::exception(), "1", LogLevel::Warn);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }

        #[test]
        fn warn_exception_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.write_exception(&FormatHelper::exception(), "1", LogLevel::Warn);

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.write_exception(&FormatHelper::exception(), "1", LogLevel::Error);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }

        #[test]
        fn error_exception_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.write_exception(&FormatHelper::exception(), "1", LogLevel::Error);

            assert!(target.logs().is_empty());
        }

        #[test]
        fn fatal_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.write_exception(&FormatHelper::exception(), "1", LogLevel::Fatal);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }

        #[test]
        fn debug_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.write_with_type(
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Debug,
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), "1");
        }

        #[test]
        fn debug_type_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.write_with_type(
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Debug,
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.write_with_type(
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Info,
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), "1");
        }

        #[test]
        fn info_type_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.write_with_type(
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Info,
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.write_with_type(
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Warn,
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), "1");
        }

        #[test]
        fn warn_type_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.write_with_type(
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Warn,
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.write_with_type(
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Error,
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), "1");
        }

        #[test]
        fn error_type_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.write_with_type(
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Error,
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn fatal_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.write_with_type(
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Fatal,
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), "1");
        }

        #[test]
        fn debug_type_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.write_exception_with_type(
                &FormatHelper::exception(),
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Debug,
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }

        #[test]
        fn debug_type_exception_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.write_exception_with_type(
                &FormatHelper::exception(),
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Debug,
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_type_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.write_exception_with_type(
                &FormatHelper::exception(),
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Info,
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }

        #[test]
        fn info_type_exception_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.write_exception_with_type(
                &FormatHelper::exception(),
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Info,
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_type_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.write_exception_with_type(
                &FormatHelper::exception(),
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Warn,
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }

        #[test]
        fn warn_type_exception_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.write_exception_with_type(
                &FormatHelper::exception(),
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Warn,
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_type_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.write_exception_with_type(
                &FormatHelper::exception(),
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Error,
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }

        #[test]
        fn error_type_exception_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.write_exception_with_type(
                &FormatHelper::exception(),
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Error,
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn fatal_type_exception_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.write_exception_with_type(
                &FormatHelper::exception(),
                "1",
                ::std::any::type_name::<DummyObjectClass1>(),
                LogLevel::Fatal,
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                format!("1 {}", FormatHelper::exception())
            );
        }
    };
}

#[allow(unused_imports)]
pub(crate) use logger_base;
