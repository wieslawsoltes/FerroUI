//! Ported from `Splat.Logging.Tests/BaseTests/AllocateFreeErrorLoggerTestBase.cs` of Splat.
//! Adapted: the abstract class `AllocateFreeErrorLoggerTestBase<T>` is the macro
//! `allocate_free_error_logger_test_base!`, invoked as `logger_base!` is (see `logger_base`); it
//! expands only the tests of this class, so a test file also invokes `logger_base!`, which
//! upstream inherits. The ten generated overloads per level, `Debug<TArgument...>(Exception,
//! string, TArgument...)`, are one method per level, `debug_exception_args(&dyn Error,
//! format_args!(..))`: each upstream test (each arity) is a test of its own that calls it with the
//! same number of arguments, and the composite format `FormatN` is the literal `"{}, {}, ..."`.
//! The exception is `FormatHelper::exception()`, whose text the expected messages end with.

/// The tests of the allocation-free logging of an exception (`AllocateFreeErrorLoggerTestBase<T>`):
/// one to ten arguments per level, written when the level is enabled and not otherwise.
/// `$get_logger` makes the logger (an `IAllocationFreeErrorLogger`) and its target for a level.
#[allow(unused_macros)]
macro_rules! allocate_free_error_logger_test_base {
    ($get_logger:expr) => {
        #[allow(unused_imports)]
        use $crate::tests::logging::base_tests::logger_base::prelude::*;

        #[test]
        fn debug_exception_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(&FormatHelper::exception(), format_args!("{}", ARG1));

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED1)
            );
        }

        #[test]
        fn debug_exception_one_argument_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(&FormatHelper::exception(), format_args!("{}", ARG1));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_exception_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}", ARG1, ARG2),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED2)
            );
        }

        #[test]
        fn debug_exception_two_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}", ARG1, ARG2),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_exception_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}", ARG1, ARG2, ARG3),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED3)
            );
        }

        #[test]
        fn debug_exception_three_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}", ARG1, ARG2, ARG3),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_exception_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED4)
            );
        }

        #[test]
        fn debug_exception_four_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_exception_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED5)
            );
        }

        #[test]
        fn debug_exception_five_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_exception_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED6)
            );
        }

        #[test]
        fn debug_exception_six_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn debug_exception_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED7)
            );
        }

        #[test]
        fn debug_exception_seven_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(
                &FormatHelper::exception(),
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
        fn debug_exception_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED8)
            );
        }

        #[test]
        fn debug_exception_eight_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(
                &FormatHelper::exception(),
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
        fn debug_exception_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED9)
            );
        }

        #[test]
        fn debug_exception_nine_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(
                &FormatHelper::exception(),
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
        fn debug_exception_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.debug_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED10)
            );
        }

        #[test]
        fn debug_exception_ten_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Info);
            logger.debug_exception_args(
                &FormatHelper::exception(),
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
        fn info_exception_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(&FormatHelper::exception(), format_args!("{}", ARG1));

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED1)
            );
        }

        #[test]
        fn info_exception_one_argument_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_exception_args(&FormatHelper::exception(), format_args!("{}", ARG1));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_exception_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}", ARG1, ARG2),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED2)
            );
        }

        #[test]
        fn info_exception_two_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}", ARG1, ARG2),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_exception_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}", ARG1, ARG2, ARG3),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED3)
            );
        }

        #[test]
        fn info_exception_three_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}", ARG1, ARG2, ARG3),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_exception_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED4)
            );
        }

        #[test]
        fn info_exception_four_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_exception_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED5)
            );
        }

        #[test]
        fn info_exception_five_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_exception_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED6)
            );
        }

        #[test]
        fn info_exception_six_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn info_exception_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED7)
            );
        }

        #[test]
        fn info_exception_seven_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Warn);
            logger.info_exception_args(
                &FormatHelper::exception(),
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
        fn info_exception_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED8)
            );
        }

        #[test]
        fn info_exception_eight_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_exception_args(
                &FormatHelper::exception(),
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
        fn info_exception_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED9)
            );
        }

        #[test]
        fn info_exception_nine_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_exception_args(
                &FormatHelper::exception(),
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
        fn info_exception_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.info_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED10)
            );
        }

        #[test]
        fn info_exception_ten_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.info_exception_args(
                &FormatHelper::exception(),
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
        fn warn_exception_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(&FormatHelper::exception(), format_args!("{}", ARG1));

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED1)
            );
        }

        #[test]
        fn warn_exception_one_argument_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(&FormatHelper::exception(), format_args!("{}", ARG1));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_exception_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}", ARG1, ARG2),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED2)
            );
        }

        #[test]
        fn warn_exception_two_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}", ARG1, ARG2),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_exception_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}", ARG1, ARG2, ARG3),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED3)
            );
        }

        #[test]
        fn warn_exception_three_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}", ARG1, ARG2, ARG3),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_exception_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED4)
            );
        }

        #[test]
        fn warn_exception_four_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_exception_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED5)
            );
        }

        #[test]
        fn warn_exception_five_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_exception_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED6)
            );
        }

        #[test]
        fn warn_exception_six_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn warn_exception_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED7)
            );
        }

        #[test]
        fn warn_exception_seven_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(
                &FormatHelper::exception(),
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
        fn warn_exception_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED8)
            );
        }

        #[test]
        fn warn_exception_eight_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(
                &FormatHelper::exception(),
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
        fn warn_exception_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED9)
            );
        }

        #[test]
        fn warn_exception_nine_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(
                &FormatHelper::exception(),
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
        fn warn_exception_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.warn_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED10)
            );
        }

        #[test]
        fn warn_exception_ten_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.warn_exception_args(
                &FormatHelper::exception(),
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
        fn error_exception_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(&FormatHelper::exception(), format_args!("{}", ARG1));

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED1)
            );
        }

        #[test]
        fn error_exception_one_argument_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(&FormatHelper::exception(), format_args!("{}", ARG1));

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_exception_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}", ARG1, ARG2),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED2)
            );
        }

        #[test]
        fn error_exception_two_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}", ARG1, ARG2),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_exception_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}", ARG1, ARG2, ARG3),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED3)
            );
        }

        #[test]
        fn error_exception_three_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}", ARG1, ARG2, ARG3),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_exception_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED4)
            );
        }

        #[test]
        fn error_exception_four_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_exception_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED5)
            );
        }

        #[test]
        fn error_exception_five_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_exception_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED6)
            );
        }

        #[test]
        fn error_exception_six_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert!(target.logs().is_empty());
        }

        #[test]
        fn error_exception_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED7)
            );
        }

        #[test]
        fn error_exception_seven_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(
                &FormatHelper::exception(),
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
        fn error_exception_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED8)
            );
        }

        #[test]
        fn error_exception_eight_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(
                &FormatHelper::exception(),
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
        fn error_exception_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED9)
            );
        }

        #[test]
        fn error_exception_nine_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(
                &FormatHelper::exception(),
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
        fn error_exception_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.error_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED10)
            );
        }

        #[test]
        fn error_exception_ten_arguments_method_should_not_write_if_higher_level() {
            let (logger, target) = ($get_logger)(LogLevel::Fatal);
            logger.error_exception_args(
                &FormatHelper::exception(),
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
        fn fatal_exception_one_argument_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(&FormatHelper::exception(), format_args!("{}", ARG1));

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED1)
            );
        }

        #[test]
        fn fatal_exception_two_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}", ARG1, ARG2),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED2)
            );
        }

        #[test]
        fn fatal_exception_three_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}", ARG1, ARG2, ARG3),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED3)
            );
        }

        #[test]
        fn fatal_exception_four_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED4)
            );
        }

        #[test]
        fn fatal_exception_five_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED5)
            );
        }

        #[test]
        fn fatal_exception_six_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(
                &FormatHelper::exception(),
                format_args!("{}, {}, {}, {}, {}, {}", ARG1, ARG2, ARG3, ARG4, ARG5, ARG6),
            );

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED6)
            );
        }

        #[test]
        fn fatal_exception_seven_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED7)
            );
        }

        #[test]
        fn fatal_exception_eight_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED8)
            );
        }

        #[test]
        fn fatal_exception_nine_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED9)
            );
        }

        #[test]
        fn fatal_exception_ten_arguments_method_should_write_message() {
            let (logger, target) = ($get_logger)(LogLevel::Debug);
            logger.fatal_exception_args(
                &FormatHelper::exception(),
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

            assert_eq!(
                FormatHelper::trim(&target.logs().last().unwrap().1),
                expected_exception(EXPECTED10)
            );
        }
    };
}

#[allow(unused_imports)]
pub(crate) use allocate_free_error_logger_test_base;
