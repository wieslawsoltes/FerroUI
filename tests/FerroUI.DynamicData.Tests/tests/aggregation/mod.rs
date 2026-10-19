//! `DynamicData.Tests/AggregationTests`: the tests of the aggregations.
//!
//! `AvgFixture.cs`, `AvgFixture.ForCache.cs` and `AvgFixture.ForList.cs` only declare the partial
//! classes of the fixtures below; they have no tests.

mod aggregation_fixture;
mod avg_fixture_for_cache_for_aggregate_change_set;
mod avg_fixture_for_cache_for_change_set;
mod avg_fixture_for_list_for_aggregate_change_set;
mod avg_fixture_for_list_for_change_set;
mod max_fixture;
mod min_fixture;
mod std_dev_fixture;
mod sum_fixture_for_cache;
mod sum_fixture_for_list;
