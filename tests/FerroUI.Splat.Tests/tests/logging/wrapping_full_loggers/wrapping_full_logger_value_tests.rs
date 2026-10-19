//! Ported from `Splat.Logging.Tests/WrappingFullLoggers/WrappingFullLoggerValueTests.cs` of Splat.
//! Adapted: `Debug<T>(T value)` is `debug_value(&dyn Display)`. Only the tests of a disabled level
//! are ported: a value cannot be `null` (`&dyn Display`), the text of a value cannot be `null`
//! (`Display` writes a text), and disabling `Fatal` takes a level above it (`AboveFatal`), which
//! `LogLevel` does not have.

use super::wrapping_full_logger_test_factory::create_logger;
use crate::tests::common::{DummyObjectClass1, IMockLogTarget};
use ferroui_splat::{IFullLogger, LogLevel};

#[test]
fn debug_disabled_does_not_emit_value() {
    let (logger, target) = create_logger(LogLevel::Fatal);

    logger.debug_value(&DummyObjectClass1);

    assert!(target.logs().is_empty());
}

#[test]
fn info_disabled_does_not_emit_value() {
    let (logger, target) = create_logger(LogLevel::Fatal);

    logger.info_value(&DummyObjectClass1);

    assert!(target.logs().is_empty());
}

#[test]
fn warn_disabled_does_not_emit_value() {
    let (logger, target) = create_logger(LogLevel::Fatal);

    logger.warn_value(&DummyObjectClass1);

    assert!(target.logs().is_empty());
}

#[test]
fn error_disabled_does_not_emit_value() {
    let (logger, target) = create_logger(LogLevel::Fatal);

    logger.error_value(&DummyObjectClass1);

    assert!(target.logs().is_empty());
}

// Not ported: Debug_Null_Value_Does_Not_Emit - null value (&dyn Display cannot be null)
// Not ported: Debug_Null_ToString_Emits_Placeholder - a ToString that returns null (Display cannot)
// Not ported: Info_Null_Value_Does_Not_Emit - null value (&dyn Display cannot be null)
// Not ported: Info_Null_ToString_Emits_Placeholder - a ToString that returns null (Display cannot)
// Not ported: Warn_Null_Value_Does_Not_Emit - null value (&dyn Display cannot be null)
// Not ported: Warn_Null_ToString_Emits_Placeholder - a ToString that returns null (Display cannot)
// Not ported: Error_Null_Value_Does_Not_Emit - null value (&dyn Display cannot be null)
// Not ported: Error_Null_ToString_Emits_Placeholder - a ToString that returns null (Display cannot)
// Not ported: Fatal_Disabled_Does_Not_Emit_Value - an enum value that does not exist in Rust (AboveFatal, LogLevel.Fatal + 1)
// Not ported: Fatal_Null_Value_Does_Not_Emit - null value (&dyn Display cannot be null)
// Not ported: Fatal_Null_ToString_Emits_Placeholder - a ToString that returns null (Display cannot)
