//! `MergeManyChangeSetsFixture.ForCacheChangeSets.WithoutSourceComparer.IntegrationTests`
//! (`DynamicData.Tests/Cache/MergeManyChangeSetsFixture.ForCacheChangeSets.WithoutSourceComparer.IntegrationTests.cs`):
//! the stress test of the merged change sets of the caches of the items of a cache.
//!
//! The only test adds and removes markets and prices from the threads of the task pool while it
//! subscribes and unsubscribes in a loop of `await Task.Yield()`; its meaning is the safety of the
//! operator under concurrent notifications, which a single thread cannot exercise.

// Not ported: MultiThreadedStressTest(5, 7) - real threads
// Not ported: MultiThreadedStressTest(10, 50) - real threads
