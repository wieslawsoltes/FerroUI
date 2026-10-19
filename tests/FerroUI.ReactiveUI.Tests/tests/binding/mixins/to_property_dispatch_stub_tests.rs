//! `ToPropertyDispatchStubTests` (`ReactiveUI.Binding.Tests/Mixins/ToPropertyDispatchStubTests.cs`).
//!
//! Upstream checks that the overload a source generator displaces refuses the call when no
//! generated code claimed it. A property path carries its own getter, so every call is served and
//! there is no refusal to test.

// Not ported: ToProperty_WithNoGeneratedOverload_Refuses - expression trees (the stub that refuses a call without generated code has no counterpart: a property path always carries its getter)
