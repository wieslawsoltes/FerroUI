//! `ToPropertyUnsafeSplitAssemblyTests`
//! (`ReactiveUI.Binding.Tests/Bindings/ToPropertyUnsafeSplitAssemblyTests.cs`).
//!
//! Each test emits its model and ReactiveUI's extension methods into separate dynamic assemblies
//! (`System.Reflection.Emit`) and checks that the reflection of `ToPropertyUnsafe` finds them. The
//! port's `to_property` raises through `IReactiveObject`, resolved by the type checker, so none of
//! these tests is ported; what they check of the owner (changing, then changed, for each value) is
//! checked in `to_property_unsafe_tests.rs`.

// Not ported: SplitAssembly_RaisesChangingThenChanged - reflection (the 4 argument rows)

// Not ported: MissingExtensions_RaisesThroughMethods - reflection

// Not ported: InterfaceAssemblyExtensions_RaiseBothNotifications - reflection

// Not ported: ReactiveRuntime_SplitAssembly_RaisesBothNotifications - reflection (the 2 argument rows)

// Not ported: ReturningExtensions_RaiseBothNotifications - reflection

// Not ported: ChangedOnlyExtensions_RaiseAfterChange - reflection
