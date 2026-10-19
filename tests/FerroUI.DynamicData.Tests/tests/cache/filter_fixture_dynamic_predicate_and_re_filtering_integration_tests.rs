//! `FilterFixture.DynamicPredicateAndReFiltering.IntegrationTests`
//! (`DynamicData.Tests/Cache/FilterFixture.DynamicPredicateAndReFiltering.IntegrationTests.cs`):
//! the filter of caches by a stream of predicates and a stream of requests to filter again, under
//! a stress of changes, predicates and requests.

use super::filter_fixture::{generate_random_id_inclusion_masks, generate_stress_items_and_change_sets, Item, ItemPredicate};
use crate::domain::Randomizer;
use crate::utilities::{assert_equivalent_because, assert_should_not_support_sorting, CacheObservableExtensions, ObservableExtensions};
use ferroui_dynamic_data::cache::{ChangeSet, IQuery, ObservableCacheEx};
use ferroui_reactive::subjects::Subject;
use ferroui_system::{IDisposable, IObserver};
use std::rc::Rc;

/// The count of requests to filter again (`10_000`).
const REAPPLY_FILTER_COUNT: usize = 10_000;

/// Upstream publishes the change sets, the predicates and the requests to filter again from three
/// tasks of the thread pool at once, to check that the operator is thread-safe. The port has one
/// thread: the three tasks are interleaved, one change set, one predicate, then the requests of
/// that step (two, as there are twice as many requests), which is one of the schedules upstream's
/// tasks can take; the final state must still be the items that match the final predicate.
#[test]
fn notifications_occur_on_different_threads_operator_is_thread_safe() {
    // Setup
    let randomizer = Randomizer::new(0x1234567);

    let (items, change_sets) = generate_stress_items_and_change_sets(5_000, 20, &randomizer);

    let predicates: Vec<ItemPredicate> = generate_random_id_inclusion_masks(5_000, &randomizer)
        .into_iter()
        .map(|mask| -> ItemPredicate { Rc::new(move |item: &Rc<Item>| Item::filter_by_id_inclusion_mask(&mask, item)) })
        .collect();

    let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();
    let predicate_changed = Subject::<ItemPredicate>::new();
    let reapply_filter = Subject::<()>::new();

    // UUT Initialization
    let (subscription, results) = source
        .as_observable()
        .filter_dynamic_reapply(predicate_changed.as_observable(), reapply_filter.as_observable())
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    // UUT Action
    let steps = change_sets.len().max(predicates.len());
    let requests_per_step = REAPPLY_FILTER_COUNT.div_ceil(steps);
    let mut requests = 0;
    for index in 0..steps {
        if let Some(change_set) = change_sets.get(index) {
            source.on_next(change_set.clone());
        }
        if let Some(predicate) = predicates.get(index) {
            predicate_changed.on_next(predicate.clone());
        }
        for _ in 0..requests_per_step {
            if requests < REAPPLY_FILTER_COUNT {
                reapply_filter.on_next(());
                requests += 1;
            }
        }
    }
    while requests < REAPPLY_FILTER_COUNT {
        reapply_filter.on_next(());
        requests += 1;
    }

    let final_predicate = predicates.last().expect("predicates were generated").clone();

    assert!(results.error().is_none());
    let matching: Vec<Rc<Item>> = items.items().into_iter().filter(|item| final_predicate(item)).collect();
    assert_equivalent_because(&results.recorded_items(), &matching, "the source colleciton should be filtered to include only items matching the final predicate");
    assert!(!results.has_completed(), "the source has not completed");

    // Final verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
    reapply_filter.dispose();
    predicate_changed.dispose();
    source.dispose();
}
