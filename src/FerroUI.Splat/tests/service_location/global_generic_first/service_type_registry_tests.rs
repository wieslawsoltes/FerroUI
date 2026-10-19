//! Ported from `Splat.Tests/ServiceLocation/GlobalGenericFirst/ServiceTypeRegistryTests.cs` of
//! Splat. `ServiceTypeRegistry` is the global registry of the `Type`-based (non-generic) API, which
//! is not ported: the generic registrations are the only ones.

// Not ported: TrackNonGenericRegistration_TracksType - the non-generic (Type-based) registry is not ported
// Not ported: TrackNonGenericRegistration_WithContract_TracksTypeAndContract - the non-generic (Type-based) registry is not ported
// Not ported: HasNonGenericRegistrations_WhenNotTracked_ReturnsFalse - the non-generic (Type-based) registry is not ported
// Not ported: Register_StoresFactory - the non-generic (Type-based) registry is not ported
// Not ported: Register_WithContract_StoresFactoryUnderContract - the non-generic (Type-based) registry is not ported
// Not ported: Register_MultipleForSameType_ReturnsLatest - the non-generic (Type-based) registry is not ported
// Not ported: GetService_WhenEmpty_ReturnsNull - the non-generic (Type-based) registry is not ported
// Not ported: GetService_WithWrongContract_ReturnsNull - the non-generic (Type-based) registry is not ported
// Not ported: GetService_InvokesFactoryEachTime - the non-generic (Type-based) registry is not ported
// Not ported: GetServices_WhenEmpty_ReturnsEmptyArray - the non-generic (Type-based) registry is not ported
// Not ported: GetServices_WithMultipleRegistrations_ReturnsAllValues - the non-generic (Type-based) registry is not ported
// Not ported: GetServices_OnlyReturnsMatchingContract - the non-generic (Type-based) registry is not ported
// Not ported: GetServices_FiltersOutNullValues - the non-generic (Type-based) registry is not ported
// Not ported: HasRegistration_WhenEmpty_ReturnsFalse - the non-generic (Type-based) registry is not ported
// Not ported: HasRegistration_AfterRegistering_ReturnsTrue - the non-generic (Type-based) registry is not ported
// Not ported: HasRegistration_WithContract_OnlyReturnsTrueForMatchingContract - the non-generic (Type-based) registry is not ported
// Not ported: UnregisterCurrent_WhenEmpty_DoesNotThrow - the non-generic (Type-based) registry is not ported
// Not ported: UnregisterCurrent_WithSingleItem_ClearsRegistration - the non-generic (Type-based) registry is not ported
// Not ported: UnregisterCurrent_WithMultipleItems_RemovesLatest - the non-generic (Type-based) registry is not ported
// Not ported: UnregisterCurrent_OnlyAffectsSpecifiedContract - the non-generic (Type-based) registry is not ported
// Not ported: UnregisterAll_RemovesAllRegistrationsForType - the non-generic (Type-based) registry is not ported
// Not ported: UnregisterAll_WhenNotRegistered_DoesNotThrow - the non-generic (Type-based) registry is not ported
// Not ported: GetServices_WithAllNullFactories_ReturnsEmptyArray - the non-generic (Type-based) registry is not ported
// Not ported: UnregisterAll_OnlyAffectsSpecifiedTypeAndContract - the non-generic (Type-based) registry is not ported
// Not ported: Clear_RemovesAllRegistrations - the non-generic (Type-based) registry is not ported
// Not ported: Clear_WhenEmpty_DoesNotThrow - the non-generic (Type-based) registry is not ported
// Not ported: Registry_WithDifferentTypes_AreIsolated - the non-generic (Type-based) registry is not ported
// Not ported: Registry_ThreadSafety_ConcurrentAdds - the non-generic (Type-based) registry is not ported
// Not ported: Register_WithComplexTypes_WorksCorrectly - the non-generic (Type-based) registry is not ported
// Not ported: Register_LargeNumberOfTypes_MaintainsPerformance - the non-generic (Type-based) registry is not ported
// Not ported: GetAllFactoriesForDisposal_WhenEmpty_ReturnsEmptyEnumerable - the non-generic (Type-based) registry is not ported
// Not ported: GetAllFactoriesForDisposal_WithMultipleRegistrations_ReturnsAllFactories - the non-generic (Type-based) registry is not ported
// Not ported: GetAllFactoriesForDisposal_ReturnsSnapshot_NotAffectedBySubsequentChanges - the non-generic (Type-based) registry is not ported
