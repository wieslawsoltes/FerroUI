//! `FilterOnConnectFixture` (`DynamicData.Tests/Cache/FilterOnConnectFixture.cs`): the predicate of
//! `Connect` on a source cache (see https://github.com/reactivemarbles/DynamicData/issues/400).

use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};

#[test]
fn clearing_source_cache_with_predicate_should_clear_the_data() {
    // having
    let source = SourceCache::<i32, i32>::new(|it: &i32| *it);
    source.edit(|updater| updater.add_or_update(1));
    let results = source.connect_filtered(|_: &i32| true).as_aggregator();

    // when
    source.edit(|updater| updater.clear());

    // then
    assert_eq!(results.data().count(), 0, "Should be 0");
}

#[test]
fn updates_existed_before_connect_without_predicate_should_be_visible_as_previous_when_new_updates_triggered() {
    // having
    let source = SourceCache::<i32, i32>::new(|it: &i32| *it);
    source.edit(|updater| updater.add_or_update(1));
    let results = source.connect().as_aggregator();

    // when
    source.edit(|updater| updater.add_or_update(1));

    // then
    let messages = results.messages();
    assert_eq!(messages.len(), 2, "Should be 2 updates");
    assert!(messages[1].changes()[0].previous().is_some(), "Should have previous value");
}
