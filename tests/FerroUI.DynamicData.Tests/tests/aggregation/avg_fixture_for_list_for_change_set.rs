//! `AvgFixture.ForList.ForChangeSet`
//! (`DynamicData.Tests/AggregationTests/AvgFixture.ForList.ForChangeSet.cs`): the form of `Avg` on
//! the change sets of a list (`avg_with`) forwards to the form on aggregate change sets, which
//! carries the behavioral coverage (`avg_fixture_for_list_for_aggregate_change_set`).

use crate::utilities::{ObservableExtensions, TestSourceList};
use ferroui_dynamic_data::aggregation::AggregationListEx;
use ferroui_dynamic_data::list::{IObservableList, SourceListEditConvenienceEx};
use ferroui_system::IDisposable;

/// `CreatePopulatedSource`.
fn create_populated_source() -> TestSourceList<i32> {
    let source = TestSourceList::new();
    source.add_range(vec![10, 20, 30]);
    source
}

#[test]
fn change_set_overload_forwards_to_aggregate_overload() {
    let source = create_populated_source();

    let (subscription, results) = source.connect().avg_with(|value: &i32| *value, -1i32).record_values();

    source.clear();

    assert_eq!(results.recorded_values(), vec![20.0f64, -1.0f64]);

    subscription.dispose();
    source.dispose();
}
