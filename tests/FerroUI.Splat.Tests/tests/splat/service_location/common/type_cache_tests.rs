//! Ported from `Splat.Tests/ServiceLocation/Common/TypeCacheTests.cs` of Splat.
//! Nothing is ported: `TypeCache<T>` caches the `typeof(T)` of a generic type argument, which the
//! port does not need; the key of a service is `ServiceType::of::<S>()`, the `TypeId` of the type,
//! which the compiler gives without a cache.

// Not ported: TypeCache_Should_ReturnCorrectType - TypeCache<T> (a typeof cache), not ported; TypeId::of needs none
// Not ported: TypeCache_Should_ReturnSameInstanceForSameType - TypeCache<T> (a typeof cache), not ported; TypeId::of needs none
// Not ported: TypeCache_Should_ReturnDifferentInstancesForDifferentTypes - TypeCache<T> (a typeof cache), not ported; TypeId::of needs none
// Not ported: TypeCache_Should_WorkWithNullableTypes - TypeCache<T> (a typeof cache), not ported; TypeId::of needs none
// Not ported: TypeCache_Should_WorkWithGenericTypes - TypeCache<T> (a typeof cache), not ported; TypeId::of needs none
// Not ported: TypeCache_Should_WorkWithInterfaces - TypeCache<T> (a typeof cache), not ported; TypeId::of needs none
