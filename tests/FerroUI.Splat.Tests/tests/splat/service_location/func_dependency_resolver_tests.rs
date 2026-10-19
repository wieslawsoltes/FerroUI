//! Ported from `Splat.Tests/ServiceLocation/FuncDependencyResolverTests.cs` of Splat.
//! Adapted: nothing is ported. `FuncDependencyResolver` (`Splat/ServiceLocation/FuncDependencyResolver.cs`)
//! is built on the `Type`-based API (its delegates take a `Type?`:
//! `Func<Type?, string?, IEnumerable<object>>`, `Action<Func<object?>, Type?, string?>`, ...),
//! which is not ported, so the resolver is not ported either. The class would also have
//! instantiated the 73 tests of
//! `BaseDependencyResolverTests<FuncDependencyResolver>` (`base_dependency_resolver_tests!`), with
//! its override of `Dispose_DisposesRegisteredServices`; none of them runs for this resolver.

// Not ported: Constructor_ShouldAcceptMinimalParameters - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Constructor_ShouldAcceptAllParameters - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetService_ShouldReturnLastService_WhenMultipleRegistered - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetService_ShouldReturnNull_WhenNoServicesRegistered - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetService_WithContract_ShouldReturnLastService - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetServices_ShouldReturnEmptyList_WhenGetAllServicesReturnsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetServices_ShouldReturnAllServices - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetServices_WithContract_ShouldPassContractToGetAllServices - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetServices_WithNullServiceType_ShouldUseNullServiceType - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: HasRegistration_ShouldReturnTrue_WhenGetAllServicesReturnsNonNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: HasRegistration_ShouldReturnFalse_WhenGetAllServicesReturnsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: HasRegistration_WithContract_ShouldPassContractToGetAllServices - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Register_ShouldThrowNotSupportedException_WhenRegisterDelegateIsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Register_ShouldCallRegisterDelegate_WhenProvided - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Register_WithContract_ShouldPassContractToDelegate - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Register_WithNullServiceType_ShouldWrapInNullServiceType - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Register_Generic_ShouldThrowArgumentNullException_WhenFactoryIsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: RegisterConstant_ShouldRegisterValue - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: RegisterLazySingleton_ShouldThrowArgumentNullException_WhenFactoryIsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: RegisterLazySingleton_ShouldRegisterLazyValue - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterCurrent_ShouldThrowNotSupportedException_WhenDelegateIsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterCurrent_ShouldCallDelegate_WhenProvided - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterCurrent_WithContract_ShouldPassContractToDelegate - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterAll_ShouldThrowNotSupportedException_WhenDelegateIsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterAll_ShouldCallDelegate_WhenProvided - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterAll_WithContract_ShouldPassContractToDelegate - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: ServiceRegistrationCallback_ShouldRegisterCallback - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: ServiceRegistrationCallback_ShouldThrowArgumentNullException_WhenCallbackIsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: ServiceRegistrationCallback_Dispose_ShouldRemoveCallback - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: ServiceRegistrationCallback_WithContract_ShouldRegisterCallback - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: ServiceRegistrationCallback_ShouldNotInvokeCallback_ForDifferentContract - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: ServiceRegistrationCallback_ShouldRemoveCallbackAfterDisposal - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Dispose_ShouldDisposeInnerDisposable - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Dispose_ShouldBeIdempotent - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Dispose_ShouldNotThrow_WhenInnerDisposableIsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Dispose_DisposesRegisteredServices - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Register_TwoTypeParameters_ShouldCreateNewInstance - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Register_TwoTypeParametersWithContract_ShouldPassContract - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetServices_WithNullServiceTypeOverNonCollection_UnwrapsMarkers - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Dispose_WhenLazySingletonDisposalThrows_DoesNotPropagate - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Dispose_WhenRegisteredServiceDisposalThrows_DoesNotPropagate - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterCurrent_WithContract_ShouldThrowNotSupportedException_WhenDelegateIsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterAll_WithContract_ShouldThrowNotSupportedException_WhenDelegateIsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterCurrent_WithNullServiceType_UsesNullServiceType - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: UnregisterAll_WithNullServiceType_UsesNullServiceType - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: HasRegistration_WithNullServiceType_UsesNullServiceType - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetService_OverReadOnlyList_ReturnsLastElement - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetService_OverEmptyReadOnlyList_ReturnsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetService_OverEnumerable_ReturnsLastElement - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetService_OverEmptyEnumerable_ReturnsNull - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetServices_WithNullServiceTypeOverCollection_UnwrapsMarkers - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: GetService_WhenLazySingletonAccessedAfterDispose_DisposesValueAndThrows - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
// Not ported: Dispose_WithNullGetAllServices_DoesNotThrow - `FuncDependencyResolver` is built on the `Type`-based API, which is not ported
