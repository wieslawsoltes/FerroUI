//! Ported from `Splat.Tests/ServiceLocation/NullServiceTypeTests.cs` of Splat.
//! Nothing is ported: `NullServiceType`, the stand-in of upstream for a registration whose
//! service type is `null`, belongs to the `Type`-based (non-generic) API, which is not ported; a
//! service of the port always has a type.

// Not ported: Constructor_ShouldPreserveFactory - NullServiceType (the null service type of the Type-based API), not ported
// Not ported: Factory_ShouldBeInvokable - NullServiceType (the null service type of the Type-based API), not ported
// Not ported: Factory_CanReturnNull - NullServiceType (the null service type of the Type-based API), not ported
// Not ported: CachedType_ShouldBeNullServiceType - NullServiceType (the null service type of the Type-based API), not ported
// Not ported: CachedType_ShouldBeSameInstance - NullServiceType (the null service type of the Type-based API), not ported
// Not ported: MultipleInstances_CanHaveDifferentFactories - NullServiceType (the null service type of the Type-based API), not ported
// Not ported: Factory_CanBeClosure - NullServiceType (the null service type of the Type-based API), not ported
