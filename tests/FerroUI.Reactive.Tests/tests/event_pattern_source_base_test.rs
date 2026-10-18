//! Ported from `Tests.System.Reactive/Tests/EventPatternSourceBaseTest.cs` of Rx.NET.
//! The only test of the file checks the null arguments of `EventPatternSourceBase<TSender,
//! TEventArgs>` (`ferroui_reactive::linq::EventPatternSourceBase`), which the types of Rust rule
//! out; the base is exercised by the tests of `ToEventPattern`.

// Not ported: ArgumentChecking - argument validation
