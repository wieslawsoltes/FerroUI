//! `AsyncDisposeManyFixture.IntegrationTests`
//! (`DynamicData.Tests/Cache/AsyncDisposeManyFixture.IntegrationTests.cs`): the disposals of the
//! items of a source cache when the sequence ends, and the signal of their end.
//!
//! Upstream awaits `WhenFinalized`, because Rx and the TPL do not run the continuations of a task
//! with its completion; the continuations of the tasks of `ferroui-system` run with it, so the
//! signal has ended when the disposals end, and the tests check that it has.

use super::async_dispose_many_fixture::{DisposalsCompletedRecorder, ItemBase, ItemType};
use crate::utilities::{assert_equivalent_because, CacheObservableExtensions, ObservableExtensions};
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::rc::Rc;

/// `ItemDisposalErrors_ErrorPropagatesToDisposalsCompleted(ItemType itemType)`.
fn item_disposal_errors_error_propagates_to_disposals_completed(item_type: ItemType) {
    let source = SourceCache::new(ItemBase::select_id);
    let source_completion_source = Subject::<()>::new();

    let disposals_completed = DisposalsCompletedRecorder::new();

    let (subscription, results) = source
        .connect()
        .take_until(source_completion_source.as_observable())
        .async_dispose_many_with(ItemBase::to_item_disposal, disposals_completed.accessor())
        .validate_synchronization()
        .validate_cache_change_sets(ItemBase::select_id)
        .record_cache_items();

    let disposals_completed_results = disposals_completed.results();

    source.add_or_update_items([ItemBase::create(item_type, 1, 1), ItemBase::create(item_type, 2, 1), ItemBase::create(item_type, 3, 1)]);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added");
    assert!(!results.has_completed());

    assert!(disposals_completed_results.error().is_none());
    assert!(disposals_completed_results.recorded_values().is_empty(), "no disposals should have occurred");
    assert!(!disposals_completed_results.has_completed(), "no disposals should have occurred");

    let error: ObservableError = new_error("Test");
    source.items()[1].fail_disposal(error.clone());

    source_completion_source.on_next(());

    assert!(disposals_completed_results.has_finalized(), "the disposals have ended");

    assert!(results.error().is_none(), "disposal errors should be propagated on disposalsCompleted");
    assert_eq!(results.recorded_change_sets().len(), 1, "no source operations were performed");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "no items were changed");
    assert!(results.has_completed());

    let disposal_error = disposals_completed_results.error().expect("disposal errors should be caught and propagated on disposalsCompleted");
    assert!(Rc::ptr_eq(&disposal_error, &error), "disposal errors should be caught and propagated on disposalsCompleted");

    subscription.dispose();
    source_completion_source.dispose();
    source.dispose();
}

// Not ported: ItemDisposalErrors_ErrorPropagatesToDisposalsCompleted(ItemType.Disposable) -
// `IDisposable::dispose` cannot fail; upstream's exception of `Dispose` is a panic here, which
// `AsyncDisposeMany` does not catch (docs/porting/reactive.md, section 5.2)

#[test]
fn item_disposal_errors_error_propagates_to_disposals_completed_case2() {
    item_disposal_errors_error_propagates_to_disposals_completed(ItemType::AsyncDisposable);
}

#[test]
fn item_disposal_errors_error_propagates_to_disposals_completed_case3() {
    item_disposal_errors_error_propagates_to_disposals_completed(ItemType::ImmediateAsyncDisposable);
}

/// `ItemDisposalsComplete_DisposalsCompletedOccursAndCompletes(ItemType itemType)`.
fn item_disposals_complete_disposals_completed_occurs_and_completes(item_type: ItemType) {
    let source = SourceCache::new(ItemBase::select_id);
    let source_completion_source = Subject::<()>::new();

    let disposals_completed = DisposalsCompletedRecorder::new();

    let (subscription, results) = source
        .connect()
        .take_until(source_completion_source.as_observable())
        .async_dispose_many_with(ItemBase::to_item_disposal, disposals_completed.accessor())
        .validate_synchronization()
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

    source_completion_source.on_next(());
    for item in source.items() {
        item.complete_disposal();
    }

    assert!(disposals_completed_results.has_finalized(), "the disposals have ended");

    assert!(results.error().is_none());
    assert_eq!(results.recorded_change_sets().len(), 1, "no source operations were performed");
    assert_equivalent_because(&results.recorded_items(), &source.items(), "no items were changed");
    assert!(results.has_completed());

    assert!(disposals_completed_results.error().is_none());
    assert_eq!(disposals_completed_results.recorded_values().len(), 1, "the source and all disposals have completed");
    assert!(disposals_completed_results.has_completed(), "the source and all disposals have completed");

    subscription.dispose();
    source_completion_source.dispose();
    source.dispose();
}

#[test]
fn item_disposals_complete_disposals_completed_occurs_and_completes_case1() {
    item_disposals_complete_disposals_completed_occurs_and_completes(ItemType::Plain);
}

#[test]
fn item_disposals_complete_disposals_completed_occurs_and_completes_case2() {
    item_disposals_complete_disposals_completed_occurs_and_completes(ItemType::Disposable);
}

#[test]
fn item_disposals_complete_disposals_completed_occurs_and_completes_case3() {
    item_disposals_complete_disposals_completed_occurs_and_completes(ItemType::AsyncDisposable);
}

#[test]
fn item_disposals_complete_disposals_completed_occurs_and_completes_case4() {
    item_disposals_complete_disposals_completed_occurs_and_completes(ItemType::ImmediateAsyncDisposable);
}

// Not ported: ItemDisposalsOccurOnMultipleThreads_DisposalIsThreadSafe - real threads
