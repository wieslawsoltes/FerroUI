//! `WhenPropertyChangedFixture.IntegrationTests` (`DynamicData.Tests/Binding/WhenPropertyChangedFixture.IntegrationTests.cs`).
//!
//! Upstream's `ConditionTimeout` and `WaitForCondition` wait for other threads; the tests of the
//! class that keep their meaning on one thread do not need them, and the others are not ported.

// Not ported: AutoRefreshThenFilter_ConcurrentAddsAndPropertyActivation_AllItemsObserved - real threads (skipped upstream: "AutoRefresh has a separate concurrency bug")

// Not ported: AutoRefreshThenFilter_DualSubscribers_AllItemsObserved - real threads (skipped upstream: "AutoRefresh has a separate concurrency bug")
