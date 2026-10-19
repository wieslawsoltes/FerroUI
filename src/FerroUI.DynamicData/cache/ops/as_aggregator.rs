//! `AsAggregator` of caches (`Cache/Tests/TestEx.cs`).

macro_rules! as_aggregator_methods {
    () => {
        /// Records the change sets, the items and the end of the sequence, for tests
        /// (`TestEx.AsAggregator`).
        fn as_aggregator(&self) -> crate::cache::testing::ChangeSetAggregator<T, K> {
            crate::cache::testing::ChangeSetAggregator::new(self.change_sets())
        }
    };
}
pub(crate) use as_aggregator_methods;
