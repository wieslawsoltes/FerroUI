//! Ported from `Splat.Logging.Tests/WrappingFullLoggers/WrappingFullLoggerNullMessageTests.cs` of
//! Splat.
//! Nothing is ported: every test passes a `null` message, which a `&str` cannot be, and checks the
//! placeholder `(null)` that the wrapping logger writes for it.

// Not ported: Debug_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Debug_Generic_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Info_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Info_Generic_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Warn_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Warn_Generic_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Error_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Error_Generic_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Fatal_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
// Not ported: Fatal_Generic_Null_Message_Forwards_Placeholder - null message (&str cannot be null)
