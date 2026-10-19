//! Ported from `Splat.Logging.Tests/BaseTests` of Splat: the test bases that the logger tests
//! inherit upstream are macros here, which a test file invokes with the logger to test.

pub mod allocate_free_error_logger_test_base;
pub mod allocation_free_logger_base_test_base;
pub mod full_logger_test_base;
pub mod logger_base;
pub mod logger_test_constants;
pub mod wrapping_full_logger_test_base;
