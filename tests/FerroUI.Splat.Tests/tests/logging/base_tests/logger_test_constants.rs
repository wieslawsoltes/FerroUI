//! Ported from `Splat.Logging.Tests/BaseTests/LoggerTestConstants.cs` of Splat.
//! Adapted: the composite format strings (`Format1` = `"{0}"`, ...) are `format_args!` literals at
//! the call sites, since Rust's format strings are checked at compile time; the arguments and the
//! expected texts are kept. The text of the test exception is the `Display` of
//! `FormatHelper::exception()`.

#![allow(dead_code)]

pub const ARG1: i32 = 1;
pub const ARG2: i32 = 2;
pub const ARG3: i32 = 3;
pub const ARG4: i32 = 4;
pub const ARG5: i32 = 5;
pub const ARG6: i32 = 6;
pub const ARG7: i32 = 7;
pub const ARG8: i32 = 8;
pub const ARG9: i32 = 9;
pub const ARG10: i32 = 10;

pub const EXPECTED1: &str = "1";
pub const EXPECTED2: &str = "1, 2";
pub const EXPECTED3: &str = "1, 2, 3";
pub const EXPECTED4: &str = "1, 2, 3, 4";
pub const EXPECTED5: &str = "1, 2, 3, 4, 5";
pub const EXPECTED6: &str = "1, 2, 3, 4, 5, 6";
pub const EXPECTED7: &str = "1, 2, 3, 4, 5, 6, 7";
pub const EXPECTED8: &str = "1, 2, 3, 4, 5, 6, 7, 8";
pub const EXPECTED9: &str = "1, 2, 3, 4, 5, 6, 7, 8, 9";
pub const EXPECTED10: &str = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10";

/// What a test logger appends for the exception of `FormatHelper::exception()`.
pub const EXCEPTION_SUFFIX: &str = " Exception of type 'Exception' was thrown.";

/// `ExpectedException1` to `ExpectedException10`: the expected text followed by the exception.
pub fn expected_exception(expected: &str) -> String {
    format!("{expected}{EXCEPTION_SUFFIX}")
}

pub const TEST_MESSAGE: &str = "This is a test.";
