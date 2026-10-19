//! Ported from `Splat.Logging.Tests/BaseTests/FullLoggerTestBase.cs` of Splat.
//! Adapted: the abstract class `FullLoggerTestBase` is the macro `full_logger_test_base!`,
//! invoked as `logger_base!` is (see `logger_base`); it expands only the tests of this class, so a
//! test file also invokes the macros of the bases it inherits upstream (`logger_base!`,
//! `allocate_free_error_logger_test_base!`, `allocation_free_logger_base_test_base!`). The
//! overloads map to the methods of `IFullLogger` and `FullLoggerExt`: `Debug<T>(Func<string>)` is
//! `debug_fn_for::<T>`, `Debug(Func<string>)` is `debug_fn`, `Debug<T>(string)` is
//! `debug_for::<T>`, `Debug<T>(T value)` is `debug_value`, and `Debug(IFormatProvider, T)` is
//! `debug_format(format_args!("{}", value))` (Rust's formatting has no culture). The flag the
//! deferred message sets is a `Cell<bool>`. The text of `DummyObjectClass1` is the name of its
//! type (`typeof(DummyObjectClass1).ToString()` is `std::any::type_name`).

/// The tests of a full logger (`FullLoggerTestBase`): the deferred messages (not computed when the
/// level is disabled), the messages of a type, of a format provider, of a value and plain ones.
/// `$get_logger` makes the logger (an `IFullLogger`) and its target for a level.
#[allow(unused_macros)]
macro_rules! full_logger_test_base {
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
        fn debug_with_generic_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn debug_with_generic_type_should_write_message_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn info_with_generic_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn info_with_generic_type_should_write_message_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn warn_with_generic_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn warn_with_generic_type_should_write_message_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn error_with_generic_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn error_with_generic_type_should_write_message_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn fatal_with_generic_type_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_for::<DummyObjectClass1>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn fatal_with_generic_type_should_write_message_provided() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_for::<DummyObjectClass2>(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn debug_enabled_format_provider_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_format(format_args!("{}", TEST_MESSAGE));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn debug_message_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn debug_object_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_value(&DummyObjectClass1);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                ::std::any::type_name::<DummyObjectClass1>()
            );
        }

        #[test]
        fn info_enabled_format_provider_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_format(format_args!("{}", TEST_MESSAGE));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn info_object_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_value(&DummyObjectClass1);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                ::std::any::type_name::<DummyObjectClass1>()
            );
        }

        #[test]
        fn info_message_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn warn_enabled_format_provider_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_format(format_args!("{}", TEST_MESSAGE));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn warn_message_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn warn_object_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_value(&DummyObjectClass1);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                ::std::any::type_name::<DummyObjectClass1>()
            );
        }

        #[test]
        fn error_enabled_format_provider_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.error_format(format_args!("{}", TEST_MESSAGE));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn error_message_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.error(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn error_object_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.error_value(&DummyObjectClass1);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                ::std::any::type_name::<DummyObjectClass1>()
            );
        }

        #[test]
        fn fatal_enabled_format_provider_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.fatal_format(format_args!("{}", TEST_MESSAGE));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn fatal_object_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.fatal_value(&DummyObjectClass1);

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                ::std::any::type_name::<DummyObjectClass1>()
            );
        }

        #[test]
        fn fatal_message_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.fatal(TEST_MESSAGE);

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
        }

        #[test]
        fn debug_func_disabled_should_not_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.debug_fn(&|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert!(target.logs().is_empty());
            assert!(!invoked.get());
        }

        #[test]
        fn debug_func_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            let invoked = Cell::new(false);
            logger.debug_fn(&|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }

        #[test]
        fn info_func_disabled_should_not_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.info_fn(&|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert!(target.logs().is_empty());
            assert!(!invoked.get());
        }

        #[test]
        fn info_func_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            let invoked = Cell::new(false);
            logger.info_fn(&|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }

        #[test]
        fn warn_func_disabled_should_not_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.warn_fn(&|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert!(target.logs().is_empty());
            assert!(!invoked.get());
        }

        #[test]
        fn warn_func_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            let invoked = Cell::new(false);
            logger.warn_fn(&|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }

        #[test]
        fn error_func_disabled_should_not_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.error_fn(&|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert!(target.logs().is_empty());
            assert!(!invoked.get());
        }

        #[test]
        fn error_func_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            let invoked = Cell::new(false);
            logger.error_fn(&|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }

        #[test]
        fn fatal_func_enabled_should_emit() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            let invoked = Cell::new(false);
            logger.fatal_fn(&|| {
                invoked.set(true);
                TEST_MESSAGE.to_owned()
            });

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), TEST_MESSAGE);
            assert!(invoked.get());
        }
    };
}

#[allow(unused_imports)]
pub(crate) use full_logger_test_base;
