//! Ported from `Splat.Tests/TargetFrameworkExtensionsTests.cs` of Splat.
//! Nothing is ported: `TargetFrameworkExtensions.GetTargetFrameworkName` reads the
//! `TargetFrameworkAttribute` of an assembly through reflection, which has no counterpart in Rust,
//! and the crate does not port it.

// Not ported: ReturnsName - reflection over assemblies (TargetFrameworkExtensions), not ported
// Not ported: GetTargetFrameworkName_UnknownFramework_ReturnsNull - reflection over assemblies (TargetFrameworkExtensions), not ported
// Not ported: GetTargetFrameworkName_NullFramework_ReturnsNull - reflection over assemblies (TargetFrameworkExtensions), not ported
// Not ported: GetTargetFrameworkName_ForCurrentAssembly_ReturnsExpectedShortName - reflection over assemblies (TargetFrameworkExtensions), not ported
