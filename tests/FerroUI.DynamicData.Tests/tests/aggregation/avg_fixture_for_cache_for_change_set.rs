//! `AvgFixture.ForCache.ForChangeSet`
//! (`DynamicData.Tests/AggregationTests/AvgFixture.ForCache.ForChangeSet.cs`): the form of `Avg` on
//! the change sets of a cache (`avg_with`) forwards to the form on aggregate change sets, which
//! carries the behavioral coverage (`avg_fixture_for_cache_for_aggregate_change_set`).

use crate::domain::Person;
use crate::utilities::{ObservableExtensions, TestSourceCache};
use ferroui_dynamic_data::aggregation::AggregationCacheEx;
use ferroui_dynamic_data::cache::IConnectableCache;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// `CreatePopulatedSource`.
fn create_populated_source() -> TestSourceCache<Rc<Person>, String> {
    let source = TestSourceCache::new(|person: &Rc<Person>| person.name());
    source.edit(|updater| {
        updater.add_or_update(Person::new("A", 10));
        updater.add_or_update(Person::new("B", 20));
        updater.add_or_update(Person::new("C", 30));
    });
    source
}

#[test]
fn change_set_overload_forwards_to_aggregate_overload() {
    let source = create_populated_source();

    let (subscription, results) = source.connect().avg_with(|person: &Rc<Person>| person.age(), -1i32).record_values();

    source.edit(|updater| updater.clear());

    assert_eq!(results.recorded_values(), vec![20.0f64, -1.0f64]);

    subscription.dispose();
    source.dispose();
}
