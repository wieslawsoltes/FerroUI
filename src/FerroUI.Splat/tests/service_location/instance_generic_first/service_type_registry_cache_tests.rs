//! Ported from `Splat.Tests/ServiceLocation/InstanceGenericFirst/ServiceTypeRegistryCacheTests.cs`
//! of Splat. `ServiceTypeRegistryCache` is the per-resolver registry of the `Type`-based
//! (non-generic) API, which is not ported: the generic registrations are the only ones.

// Not ported: GetAllFactoriesForDisposal_WhenEmpty_ReturnsEmpty - the non-generic (Type-based) registry is not ported
// Not ported: GetAllFactoriesForDisposal_WithRegistrations_ReturnsAllFactories - the non-generic (Type-based) registry is not ported
// Not ported: GetServices_WithAllNullFactories_ReturnsEmptyArray - the non-generic (Type-based) registry is not ported
// Not ported: GetServices_WithMixedNullFactories_ReturnsOnlyNonNullValues - the non-generic (Type-based) registry is not ported
