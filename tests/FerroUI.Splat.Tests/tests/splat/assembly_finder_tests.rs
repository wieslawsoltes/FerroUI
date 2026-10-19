//! Ported from `Splat.Tests/AssemblyFinderTests.cs` of Splat.
//! Nothing is ported: `AssemblyFinder.AttemptToLoadType<T>` loads a type by its name through
//! reflection, which Rust has no counterpart of, and the crate does not port it.

// Not ported: AttemptToLoadType_ReturnsDefault_ForNonExistentType - AssemblyFinder (loading a type by name through reflection), not ported
// Not ported: AttemptToLoadType_ReturnsDefault_ForInvalidTypeName - AssemblyFinder (loading a type by name through reflection), not ported
// Not ported: AttemptToLoadType_Works_WithWellKnownTypes - AssemblyFinder (loading a type by name through reflection), not ported
// Not ported: AttemptToLoadType_ReturnsDefault_ForValueTypes - AssemblyFinder (loading a type by name through reflection), not ported
// Not ported: AttemptToLoadType_HandlesEmptyString - AssemblyFinder (loading a type by name through reflection), not ported
// Not ported: AttemptToLoadType_HandlesNull - AssemblyFinder (loading a type by name through reflection), not ported
// Not ported: AttemptToLoadType_InstantiatesExistingType - AssemblyFinder (loading a type by name through reflection), not ported
