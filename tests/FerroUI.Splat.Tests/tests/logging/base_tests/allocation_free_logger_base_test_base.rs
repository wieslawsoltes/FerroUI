//! Ported from `Splat.Logging.Tests/BaseTests/AllocationFreeLoggerBaseTestBase.cs` of Splat.
//! Adapted: the abstract class `AllocationFreeLoggerBaseTestBase<T>` is the macro
//! `allocation_free_logger_base_test_base!`, invoked as `logger_base!` is (see `logger_base`); it
//! expands only the tests of this class, so a test file also invokes the macros of the bases it
//! inherits upstream (`logger_base!`, `allocate_free_error_logger_test_base!`). The ten generated
//! overloads per level, `Debug<TArgument...>(string, TArgument...)`, are one method per level,
//! `debug_args(format_args!(..))`: each upstream test (each arity) is a test of its own that calls
//! it with the same number of arguments, and the composite format `FormatN` is the literal
//! `"{}, {}, ..."`. The levels of the loggers are those of upstream, which vary between the
//! arities (`Fatal` or the next level for the tests that do not write; `Warn` for the
//! `Error` and `Fatal` tests that write).

/// The tests of the allocation-free logging of messages (`AllocationFreeLoggerBaseTestBase<T>`):
/// one to ten arguments per level, written when the level is enabled and not otherwise.
/// `$get_logger` makes the logger (an `IAllocationFreeLogger`) and its target for a level.
#[allow(unused_macros)]
macro_rules! allocation_free_logger_base_test_base {
    ($get_logger:expr) => {
        #[allow(unused_imports)]
        use $crate::tests::logging::base_tests::logger_base::prelude::*;

        #[test]
        fn debug_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(format_args!("{}", ARG1));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED1);
        }

        #[test]
        fn debug_one_argument_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_args(format_args!("{}", ARG1));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(format_args!("{}, {}", ARG1, ARG2));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED2);
        }

        #[test]
        fn debug_two_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_args(format_args!("{}, {}", ARG1, ARG2));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(format_args!("{}, {}, {}", ARG1, ARG2, ARG3));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED3);
        }

        #[test]
        fn debug_three_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_args(format_args!("{}, {}, {}", ARG1, ARG2, ARG3));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED4);
        }

        #[test]
        fn debug_four_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_args(format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED5);
        }

        #[test]
        fn debug_five_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_args(format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED6);
        }

        #[test]
        fn debug_six_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_args(
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED7);
        }

        #[test]
        fn debug_seven_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.debug_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED8);
        }

        #[test]
        fn debug_eight_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.debug_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED9);
        }

        #[test]
        fn debug_nine_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.debug_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                    ARG10,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED10);
        }

        #[test]
        fn debug_ten_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.debug_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                    ARG10,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(format_args!("{}", ARG1));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED1);
        }

        #[test]
        fn info_one_argument_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.info_args(format_args!("{}", ARG1));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(format_args!("{}, {}", ARG1, ARG2));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED2);
        }

        #[test]
        fn info_two_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.info_args(format_args!("{}, {}", ARG1, ARG2));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(format_args!("{}, {}, {}", ARG1, ARG2, ARG3));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED3);
        }

        #[test]
        fn info_three_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.info_args(format_args!("{}, {}, {}", ARG1, ARG2, ARG3));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED4);
        }

        #[test]
        fn info_four_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.info_args(format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED5);
        }

        #[test]
        fn info_five_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.info_args(format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED6);
        }

        #[test]
        fn info_six_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.info_args(
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED7);
        }

        #[test]
        fn info_seven_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED8);
        }

        #[test]
        fn info_eight_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED9);
        }

        #[test]
        fn info_nine_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.info_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                    ARG10,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED10);
        }

        #[test]
        fn info_ten_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                    ARG10,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(format_args!("{}", ARG1));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED1);
        }

        #[test]
        fn warn_one_argument_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.warn_args(format_args!("{}", ARG1));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(format_args!("{}, {}", ARG1, ARG2));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED2);
        }

        #[test]
        fn warn_two_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.warn_args(format_args!("{}, {}", ARG1, ARG2));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(format_args!("{}, {}, {}", ARG1, ARG2, ARG3));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED3);
        }

        #[test]
        fn warn_three_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.warn_args(format_args!("{}, {}, {}", ARG1, ARG2, ARG3));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED4);
        }

        #[test]
        fn warn_four_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.warn_args(format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED5);
        }

        #[test]
        fn warn_five_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.warn_args(format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED6);
        }

        #[test]
        fn warn_six_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Error);
            logger.warn_args(
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED7);
        }

        #[test]
        fn warn_seven_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED8);
        }

        #[test]
        fn warn_eight_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED9);
        }

        #[test]
        fn warn_nine_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.warn_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                    ARG10,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED10);
        }

        #[test]
        fn warn_ten_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                    ARG10,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(format_args!("{}", ARG1));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED1);
        }

        #[test]
        fn error_one_argument_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(format_args!("{}", ARG1));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(format_args!("{}, {}", ARG1, ARG2));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED2);
        }

        #[test]
        fn error_two_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(format_args!("{}, {}", ARG1, ARG2));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(format_args!("{}, {}, {}", ARG1, ARG2, ARG3));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED3);
        }

        #[test]
        fn error_three_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(format_args!("{}, {}, {}", ARG1, ARG2, ARG3));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED4);
        }

        #[test]
        fn error_four_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED5);
        }

        #[test]
        fn error_five_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED6);
        }

        #[test]
        fn error_six_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED7);
        }

        #[test]
        fn error_seven_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED8);
        }

        #[test]
        fn error_eight_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED9);
        }

        #[test]
        fn error_nine_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.error_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                    ARG10,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED10);
        }

        #[test]
        fn error_ten_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                    ARG10,
                ),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn fatal_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(format_args!("{}", ARG1));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED1);
        }

        #[test]
        fn fatal_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(format_args!("{}, {}", ARG1, ARG2));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED2);
        }

        #[test]
        fn fatal_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(format_args!("{}, {}, {}", ARG1, ARG2, ARG3));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED3);
        }

        #[test]
        fn fatal_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED4);
        }

        #[test]
        fn fatal_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5));

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED5);
        }

        #[test]
        fn fatal_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED6);
        }

        #[test]
        fn fatal_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED7);
        }

        #[test]
        fn fatal_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED8);
        }

        #[test]
        fn fatal_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED9);
        }

        #[test]
        fn fatal_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.fatal_args(
                format_args!(
                    "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    ARG1,
                    ARG2,
                    ARG3,
                    ARG4,
                    ARG5,
                    ARG6,
                    ARG7,
                    ARG8,
                    ARG9,
                    ARG10,
                ),
            );

            assert_eq!(FormatHelper::trim(&target.logs().last().unwrap().1), EXPECTED10);
        }
    };
}

#[allow(unused_imports)]
pub(crate) use allocation_free_logger_base_test_base;
