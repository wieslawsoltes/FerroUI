//! Ported from `Splat.Logging.Tests/WrappingFullLoggers/WrappingFullLoggerFatalSuppressionTests.cs`
//! of Splat.
//! Nothing is ported: both tests disable `Fatal` with a level above it (`LogLevel.Fatal + 1`),
//! which `LogLevel` does not have.

// Not ported: FatalTypedArguments_WhenFatalIsDisabled_WriteNothing - an enum value that does not exist in Rust (LogLevel.Fatal + 1)
// Not ported: FatalTypedArgumentsWithException_WhenFatalIsDisabled_WriteNothing - an enum value that does not exist in Rust (LogLevel.Fatal + 1)
