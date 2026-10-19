//! Ported from `Splat.Logging.Tests/BaseTests/WrappingFullLoggerTestBase.cs` of Splat.
//! Adapted: the abstract class `WrappingFullLoggerTestBase` is the macro
//! `wrapping_full_logger_test_base!`, invoked as `logger_base!` is (see `logger_base`); it expands
//! only the tests of this class, so a test file also invokes the macros of the bases it inherits
//! upstream (`logger_base!`, `allocate_free_error_logger_test_base!`,
//! `allocation_free_logger_base_test_base!`). The overloads map as in `full_logger_test_base`.
//!
//! No test class inherits this base upstream (the test projects of the adapters of external
//! logging frameworks inherit `FullLoggerTestBase`), so nothing invokes the macro here either. Its
//! expectations fit no logger of Splat: a message of a type is expected bare for `Debug` with
//! `DummyObjectClass1` and with the name of the type in front otherwise (`DummyObjectClass1: This
//! is a test.` for `Info`), and `Write` bare; the console logger of `console_logger_tests` ignores
//! the type, and `WrappingPrefixLogger` writes a prefix in front of every message. The macro is
//! kept, one test per upstream test, for a logger that meets them.

/// The tests of a full logger over an adapter (`WrappingFullLoggerTestBase`): the deferred
/// messages, and the messages of a type, which the adapter writes with the name of the type in
/// front.
/// `$get_logger` makes the logger (an `IFullLogger`) and its target for a level.
#[allow(unused_macros)]
macro_rules! wrapping_full_logger_test_base {
    ($get_logger:expr) => {
        #[allow(unused_imports)]
        use $crate::tests::logging::base_tests::logger_base::prelude::*;

        #[test]
        fn debug_disabled_should_not_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.debug_fn_for::<DummyObjectClass1>(|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert!(target.logs().is_empty());
            assert!(!invoked.get());
        }

        #[test]
        fn debug_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            let invoked = Cell::new(false);
            logger.debug_fn_for::<DummyObjectClass1>(|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }

        #[test]
        fn info_disabled_should_not_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.info_fn_for::<DummyObjectClass1>(|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert!(target.logs().is_empty());
            assert!(!invoked.get());
        }

        #[test]
        fn info_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            let invoked = Cell::new(false);
            logger.info_fn_for::<DummyObjectClass1>(|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }

        #[test]
        fn warn_disabled_should_not_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.warn_fn_for::<DummyObjectClass1>(|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert!(target.logs().is_empty());
            assert!(!invoked.get());
        }

        #[test]
        fn warn_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            let invoked = Cell::new(false);
            logger.warn_fn_for::<DummyObjectClass1>(|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }

        #[test]
        fn error_disabled_should_not_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.error_fn_for::<DummyObjectClass1>(|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert!(target.logs().is_empty());
            assert!(!invoked.get());
        }

        #[test]
        fn error_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            let invoked = Cell::new(false);
            logger.error_fn_for::<DummyObjectClass1>(|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }

        #[test]
        fn fatal_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.fatal_fn_for::<DummyObjectClass1>(|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }

        #[test]
        fn debug_with_generic_type_should_write_message_and_type() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn write_should_write_message_and_type() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.write(TEST_MESSAGE, LogLevel::Debug);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn debug_with_generic_type_should_write_message_and_type_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                "DummyObjectClass2: This is a test."
            );
        }

        #[test]
        fn info_with_generic_type_should_write_message_and_type() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                "DummyObjectClass1: This is a test."
            );
        }

        #[test]
        fn info_with_generic_type_should_write_message_and_type_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                "DummyObjectClass2: This is a test."
            );
        }

        #[test]
        fn warn_with_generic_type_should_write_message_and_type() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                "DummyObjectClass1: This is a test."
            );
        }

        #[test]
        fn warn_with_generic_type_should_write_message_and_type_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                "DummyObjectClass2: This is a test."
            );
        }

        #[test]
        fn error_with_generic_type_should_write_message_and_type() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                "DummyObjectClass1: This is a test."
            );
        }

        #[test]
        fn error_with_generic_type_should_write_message_and_type_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                "DummyObjectClass2: This is a test."
            );
        }

        #[test]
        fn fatal_with_generic_type_should_write_message_and_type() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                "DummyObjectClass1: This is a test."
            );
        }

        #[test]
        fn fatal_with_generic_type_should_write_message_and_type_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                "DummyObjectClass2: This is a test."
            );
        }
    };
}

#[allow(unused_imports)]
pub(crate) use wrapping_full_logger_test_base;
