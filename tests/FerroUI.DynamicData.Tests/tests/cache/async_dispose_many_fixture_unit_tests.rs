//! `AsyncDisposeManyFixture.UnitTests` (`DynamicData.Tests/Cache/AsyncDisposeManyFixture.UnitTests.cs`):
//! the disposal of the items of a cache, at once or by tasks, after the change sets have been
//! processed downstream (`async_dispose_many`).

use super::async_dispose_many_fixture::{DisposalsCompletedRecorder, ItemBase, ItemType, SourceType};
use crate::utilities::{assert_equivalent_because, assert_equivalent_strict, CacheObservableExtensions, ObservableExtensions};
use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

const ITEM_TYPES: [ItemType; 4] = [ItemType::Plain, ItemType::Disposable, ItemType::AsyncDisposable, ItemType::ImmediateAsyncDisposable];

/// `ItemsAreAddedMovedOrRefreshed_ItemsAreNotDisposed(ItemType itemType)`.
fn items_are_added_moved_or_refreshed_items_are_not_disposed(item_type: ItemType) {
    let source = Subject::<ChangeSet<Rc<ItemBase>, i32>>::new();

    let disposals_completed = DisposalsCompletedRecorder::new();

    let (subscription, results) = source
        .as_observable()
        .async_dispose_many_with(ItemBase::to_item_disposal, disposals_completed.accessor())
        .validate_synchronization()
        .validate_cache_change_sets(ItemBase::select_id)
        .record_cache_items();

    let disposals_completed_results = disposals_completed.results();

    // Addition
    let mut items = vec![ItemBase::create(item_type, 1, 1), ItemBase::create(item_type, 2, 1), ItemBase::create(item_type, 3, 1)];

    source.on_next(items.iter().enumerate().map(|(index, item)| Change::with_index(ChangeReason::Add, item.id(), item.clone(), Some(index))).collect());

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_strict(&results.recorded_items_sorted(), &items);
    assert!(!results.has_completed());

    assert!(items.iter().all(|item| !item.has_been_disposed()), "items should not be disposed upon add");

    assert!(disposals_completed_results.error().is_none());
    assert!(disposals_completed_results.recorded_values().is_empty(), "the source has not completed");
    assert!(!disposals_completed_results.has_completed(), "the source has not completed");

    // Movement
    let moved = items.remove(2);
    items.insert(0, moved);
    let moved = items.remove(2);
    items.insert(1, moved);
    source.on_next(ChangeSet::from(vec![Change::moved(items[0].id(), items[0].clone(), 0, 2), Change::moved(items[1].id(), items[1].clone(), 1, 2)]));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len() - 1, 1, "1 source operation was performed");
    assert_equivalent_strict(&results.recorded_items_sorted(), &items);
    assert!(!results.has_completed());

    assert!(items.iter().all(|item| !item.has_been_disposed()), "items should not be disposed upon movement");

    assert!(disposals_completed_results.error().is_none());
    assert!(disposals_completed_results.recorded_values().is_empty(), "the source has not completed");
    assert!(!disposals_completed_results.has_completed(), "the source has not completed");

    // Refreshing
    source.on_next(items.iter().enumerate().map(|(index, item)| Change::with_index(ChangeReason::Refresh, item.id(), item.clone(), Some(index))).collect());

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len() - 2, 1, "1 source operation was performed");
    assert_equivalent_strict(&results.recorded_items_sorted(), &items);
    assert!(!results.has_completed());

    assert!(items.iter().all(|item| !item.has_been_disposed()), "items should not be disposed upon refresh");

    assert!(disposals_completed_results.error().is_none());
    assert!(disposals_completed_results.recorded_values().is_empty(), "the source has not completed");
    assert!(!disposals_completed_results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_added_moved_or_refreshed_items_are_not_disposed_case1() {
    items_are_added_moved_or_refreshed_items_are_not_disposed(ITEM_TYPES[0]);
}

#[test]
fn items_are_added_moved_or_refreshed_items_are_not_disposed_case2() {
    items_are_added_moved_or_refreshed_items_are_not_disposed(ITEM_TYPES[1]);
}

#[test]
fn items_are_added_moved_or_refreshed_items_are_not_disposed_case3() {
    items_are_added_moved_or_refreshed_items_are_not_disposed(ITEM_TYPES[2]);
}

#[test]
fn items_are_added_moved_or_refreshed_items_are_not_disposed_case4() {
    items_are_added_moved_or_refreshed_items_are_not_disposed(ITEM_TYPES[3]);
}

/// Checks that no removed item has been disposed when the change set reaches downstream.
fn assert_removed_not_yet_disposed(changes: &ChangeSet<Rc<ItemBase>, i32>) {
    for change in changes {
        if change.reason() == ChangeReason::Remove {
            assert!(!change.current().has_been_disposed(), "disposal should only occur after downstream processing has completed");
        }
    }
}

/// `ItemsAreRemoved_ItemsAreDisposedAfterDownstreamProcessing(ItemType itemType)`.
fn items_are_removed_items_are_disposed_after_downstream_processing(item_type: ItemType) {
    let source = SourceCache::new(ItemBase::select_id);

    let disposals_completed = DisposalsCompletedRecorder::new();

    let (subscription, results) = source
        .connect()
        .async_dispose_many_with(ItemBase::to_item_disposal, disposals_completed.accessor())
        .validate_synchronization()
        .do_(assert_removed_not_yet_disposed)
        .validate_cache_change_sets(ItemBase::select_id)
        .record_cache_items();

    let disposals_completed_results = disposals_completed.results();

    source.add_or_update_items([ItemBase::create(item_type, 1, 1), ItemBase::create(item_type, 2, 1), ItemBase::create(item_type, 3, 1)]);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added");
    assert!(!results.has_completed());

    assert!(disposals_completed_results.error().is_none());
    assert!(disposals_completed_results.recorded_values().is_empty(), "the source has not completed");
    assert!(!disposals_completed_results.has_completed(), "the source has not completed");

    let items = source.items();
    source.clear();

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len() - 1, 1, "1 source operation was performed");
    assert!(results.recorded_items().is_empty(), "all items were removed");
    assert!(!results.has_completed());

    assert!(items.iter().filter(|item| item.can_be_disposed()).all(|item| item.has_been_disposed()), "disposable items should be disposed after removal");

    assert!(disposals_completed_results.error().is_none());
    assert!(disposals_completed_results.recorded_values().is_empty(), "the source has not completed");
    assert!(!disposals_completed_results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_removed_items_are_disposed_after_downstream_processing_case1() {
    items_are_removed_items_are_disposed_after_downstream_processing(ITEM_TYPES[0]);
}

#[test]
fn items_are_removed_items_are_disposed_after_downstream_processing_case2() {
    items_are_removed_items_are_disposed_after_downstream_processing(ITEM_TYPES[1]);
}

#[test]
fn items_are_removed_items_are_disposed_after_downstream_processing_case3() {
    items_are_removed_items_are_disposed_after_downstream_processing(ITEM_TYPES[2]);
}

#[test]
fn items_are_removed_items_are_disposed_after_downstream_processing_case4() {
    items_are_removed_items_are_disposed_after_downstream_processing(ITEM_TYPES[3]);
}

/// `ItemsAreUpdated_PreviousItemsAreDisposedAfterDownstreamProcessing(ItemType itemType)`.
fn items_are_updated_previous_items_are_disposed_after_downstream_processing(item_type: ItemType) {
    let source = SourceCache::new(ItemBase::select_id);

    let disposals_completed = DisposalsCompletedRecorder::new();

    let (subscription, results) = source
        .connect()
        .async_dispose_many_with(ItemBase::to_item_disposal, disposals_completed.accessor())
        .validate_synchronization()
        .do_(assert_removed_not_yet_disposed)
        .validate_cache_change_sets(ItemBase::select_id)
        .record_cache_items();

    let disposals_completed_results = disposals_completed.results();

    source.add_or_update_items([ItemBase::create(item_type, 1, 1), ItemBase::create(item_type, 2, 1), ItemBase::create(item_type, 3, 1)]);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added");
    assert!(!results.has_completed());

    assert!(disposals_completed_results.error().is_none());
    assert!(disposals_completed_results.recorded_values().is_empty(), "the source has not completed");
    assert!(!disposals_completed_results.has_completed(), "the source has not completed");

    let previous_items = source.items();
    source.add_or_update_items([ItemBase::create(item_type, 1, 2), ItemBase::create(item_type, 2, 2), ItemBase::create(item_type, 3, 2)]);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len() - 1, 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "all items were replaced");
    assert!(!results.has_completed());

    assert!(
        previous_items.iter().filter(|item| item.can_be_disposed()).all(|item| item.has_been_disposed()),
        "disposable items should be disposed after replacement"
    );

    assert!(disposals_completed_results.error().is_none());
    assert!(disposals_completed_results.recorded_values().is_empty(), "the source has not completed");
    assert!(!disposals_completed_results.has_completed(), "the source has not completed");

    subscription.dispose();
    source.dispose();
}

#[test]
fn items_are_updated_previous_items_are_disposed_after_downstream_processing_case1() {
    items_are_updated_previous_items_are_disposed_after_downstream_processing(ITEM_TYPES[0]);
}

#[test]
fn items_are_updated_previous_items_are_disposed_after_downstream_processing_case2() {
    items_are_updated_previous_items_are_disposed_after_downstream_processing(ITEM_TYPES[1]);
}

#[test]
fn items_are_updated_previous_items_are_disposed_after_downstream_processing_case3() {
    items_are_updated_previous_items_are_disposed_after_downstream_processing(ITEM_TYPES[2]);
}

#[test]
fn items_are_updated_previous_items_are_disposed_after_downstream_processing_case4() {
    items_are_updated_previous_items_are_disposed_after_downstream_processing(ITEM_TYPES[3]);
}

// Not ported: OnDisposalsCompletedIsNull_ThrowsException - argument validation

/// The three `ImmediateAsyncDisposableItem`s and their change set of additions.
fn immediate_items() -> (Vec<Rc<ItemBase>>, ChangeSet<Rc<ItemBase>, i32>) {
    let items = vec![
        ItemBase::create(ItemType::ImmediateAsyncDisposable, 1, 1),
        ItemBase::create(ItemType::ImmediateAsyncDisposable, 2, 1),
        ItemBase::create(ItemType::ImmediateAsyncDisposable, 3, 1),
    ];
    let change_set = items.iter().map(|item| Change::new(ChangeReason::Add, item.id(), item.clone())).collect();
    (items, change_set)
}

/// `SourceCompletes_ItemsAreDisposedAndCompletionPropagates(SourceType sourceType)`: the items are
/// `IAsyncDisposable`s, disposed through [`AsAsyncDisposable`](ferroui_dynamic_data::AsAsyncDisposable)
/// (`async_dispose_many`).
fn source_completes_items_are_disposed_and_completion_propagates(source_type: SourceType) {
    let (items, change_set) = immediate_items();

    let subject = Subject::<ChangeSet<Rc<ItemBase>, i32>>::new();
    let source: Rc<dyn IObservable<ChangeSet<Rc<ItemBase>, i32>>> = match source_type {
        SourceType::Immediate => Observable::return_(change_set.clone()),
        SourceType::Subject => subject.as_observable(),
    };

    let disposals_completed = DisposalsCompletedRecorder::new();

    let (subscription, results) = source
        .async_dispose_many(disposals_completed.accessor())
        .validate_synchronization()
        .validate_cache_change_sets(ItemBase::select_id)
        .record_cache_items();

    let disposals_completed_results = disposals_completed.results();

    if source_type == SourceType::Subject {
        subject.on_next(change_set);
        subject.on_completed();
    }

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &items, "3 items were added");
    assert!(results.has_completed());

    assert!(items.iter().all(|item| item.has_been_disposed()), "disposable items should be disposed upon source completion");

    assert!(disposals_completed_results.error().is_none());
    assert_eq!(disposals_completed_results.recorded_values().len(), 1, "all items have completed disposal");
    assert!(disposals_completed_results.has_completed(), "all items have completed disposal");

    subscription.dispose();
}

#[test]
fn source_completes_items_are_disposed_and_completion_propagates_case1() {
    source_completes_items_are_disposed_and_completion_propagates(SourceType::Subject);
}

#[test]
fn source_completes_items_are_disposed_and_completion_propagates_case2() {
    source_completes_items_are_disposed_and_completion_propagates(SourceType::Immediate);
}

/// `SourceErrors_ItemsAreDisposedAndErrorPropagates(SourceType sourceType)`.
fn source_errors_items_are_disposed_and_error_propagates(source_type: SourceType) {
    let (items, change_set) = immediate_items();

    let error: ObservableError = new_error("Test");

    let subject = Subject::<ChangeSet<Rc<ItemBase>, i32>>::new();
    let source: Rc<dyn IObservable<ChangeSet<Rc<ItemBase>, i32>>> = match source_type {
        SourceType::Immediate => Observable::return_(change_set.clone()).concat(Observable::throw(error.clone())),
        SourceType::Subject => subject.as_observable(),
    };

    let disposals_completed = DisposalsCompletedRecorder::new();

    let (subscription, results) = source
        .async_dispose_many(disposals_completed.accessor())
        .validate_synchronization()
        .validate_cache_change_sets(ItemBase::select_id)
        .record_cache_items();

    let disposals_completed_results = disposals_completed.results();

    if source_type == SourceType::Subject {
        subject.on_next(change_set);
        subject.on_error(error.clone());
    }

    assert!(Rc::ptr_eq(&results.error().expect("the source failed"), &error));
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &items, "3 items were added");

    assert!(items.iter().all(|item| item.has_been_disposed()), "disposable items should be disposed upon source failure");

    assert!(disposals_completed_results.error().is_none());
    assert_eq!(disposals_completed_results.recorded_values().len(), 1, "all items have completed disposal");
    assert!(disposals_completed_results.has_completed(), "all items have completed disposal");

    subscription.dispose();
}

#[test]
fn source_errors_items_are_disposed_and_error_propagates_case1() {
    source_errors_items_are_disposed_and_error_propagates(SourceType::Subject);
}

#[test]
fn source_errors_items_are_disposed_and_error_propagates_case2() {
    source_errors_items_are_disposed_and_error_propagates(SourceType::Immediate);
}

// Not ported: SourceIsNull_ThrowsException - argument validation
