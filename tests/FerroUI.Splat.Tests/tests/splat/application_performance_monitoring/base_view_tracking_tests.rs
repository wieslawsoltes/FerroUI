//! Ported from `Splat.Tests/ApplicationPerformanceMonitoring/BaseViewTrackingTests.cs` of Splat.
//! Nothing is ported: the abstract test class `ConstructorMethod<TViewTracking>` checks that an
//! `IViewTracking` implementation can be made; only the adapter test projects (Exceptionless,
//! Raygun, ApplicationInsights), which are not ported, inherit it, and ferroui-splat has no
//! implementation of `IViewTracking` to instantiate it for.

// Not ported: ReturnsInstance - a test base for IViewTracking implementations, which only the adapter projects (not ported) have; ferroui-splat has none
