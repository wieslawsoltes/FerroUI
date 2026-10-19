//! `AutoRefreshFixture.WithoutPropertyAccessor`
//! (`DynamicData.Tests/Cache/AutoRefreshFixture.WithoutPropertyAccessor.cs`): `AutoRefresh` of
//! every property (`AutoRefresh(changeSetBuffer, propertyChangeThrottle, scheduler)`), with the
//! tests of the base in the module `base`.

use super::auto_refresh_fixture::Item;
use crate::utilities::{assert_equivalent_because, CacheObservableExtensions, ObservableExtensions, TestSourceCache};
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::IObservable;
use std::rc::Rc;
use std::time::Duration;

#[test]
fn property_changed_notification_does_not_specify_property_name_item_refreshes() {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    let item1 = Item::new(1);
    let item2 = Item::new(2);
    let item3 = Item::new(3);

    source.edit(|updater| updater.add_or_update_items(&mut vec![item1.clone(), item2.clone(), item3.clone()].into_iter()));

    // UUT Initialization
    let (_subscription, results) = build_uut(source.connect(), None, None, None)
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    item2.raise_all_properties_changed();

    assert!(results.error().is_none());
    let change_sets = results.recorded_change_sets();
    assert_eq!(change_sets.iter().skip(1).count(), 1, "1 item published a property change notification");
    assert_eq!(change_sets[1].count(), 1, "1 item published a property change notification");
    assert_eq!(change_sets[1].refreshes(), 1, "1 item published a property change notification");
    assert_eq!(change_sets[1][0].current(), &item2, "item #2 published a property change notification");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "no source operations were performed");
    assert!(!results.has_completed(), "the source has not completed");
}

/// `BuildUut`: `source.AutoRefresh(changeSetBuffer, propertyChangeThrottle, scheduler)`.
fn build_uut(
    source: Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>,
    change_set_buffer: Option<Duration>,
    property_change_throttle: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>> {
    source.auto_refresh_any_with(change_set_buffer, property_change_throttle, scheduler)
}

super::auto_refresh_fixture_base::auto_refresh_fixture_base_tests!(build_uut);
