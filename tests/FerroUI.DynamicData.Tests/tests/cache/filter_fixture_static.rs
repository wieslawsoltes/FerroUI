//! `FilterFixture.Static` (`DynamicData.Tests/Cache/FilterFixture.Static.cs`): the static filter
//! of caches (`filter`, `filter_with`), with the tests of `FilterFixture.Base`.

use super::filter_fixture::{Item, ItemPredicate};
use crate::domain::{fakers, Person, Randomizer};
use crate::utilities::{assert_equivalent_because, assert_should_not_support_sorting, CacheObservableExtensions, ObservableExtensions, StreamCompletionStrategy, TestSourceCache};
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, ObservableCacheEx};
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

const AUTO_REFRESH_SEED: i32 = 0x1165;

// Not ported: FilterIsNull_ThrowsException - argument validation

fn source_completes_completion_propagates(completion_strategy: StreamCompletionStrategy) {
    // Setup
    let source = TestSourceCache::new(Item::select_id);

    // UUT Initialization & Action
    if completion_strategy == StreamCompletionStrategy::Immediate {
        source.complete();
    }

    let (subscription, results) = source
        .connect_with(None, false)
        .filter(Item::filter_by_is_included)
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    if completion_strategy == StreamCompletionStrategy::Asynchronous {
        source.complete();
    }

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
    assert!(results.has_completed(), "the source has completed");

    // Final verification
    assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

    subscription.dispose();
    source.dispose();
}

#[test]
fn source_completes_completion_propagates_case1() {
    source_completes_completion_propagates(StreamCompletionStrategy::Asynchronous);
}

#[test]
fn source_completes_completion_propagates_case2() {
    source_completes_completion_propagates(StreamCompletionStrategy::Immediate);
}

#[test]
fn subscription_is_disposed_subscription_disposal_propagates() {
    // Setup
    let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();

    // UUT Intialization
    let (subscription, results) = source
        .as_observable()
        .filter(Item::filter_by_is_included)
        .validate_synchronization()
        .validate_cache_change_sets(Item::select_id)
        .record_cache_items();

    assert!(results.error().is_none());
    assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
    assert!(results.recorded_items().is_empty(), "the source has not initialized");
    assert!(!results.has_completed(), "the source has not completed");

    // UUT Action
    subscription.dispose();

    assert!(!source.has_observers(), "subscription disposal should propagate to all sources");

    source.dispose();
}

/// `BuildUut`.
fn build_uut(
    source: Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>,
    predicate: ItemPredicate,
    suppress_empty_change_sets: bool,
) -> Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>> {
    source.filter_with(move |item: &Rc<Item>| predicate(item), suppress_empty_change_sets)
}

super::filter_fixture_base::filter_fixture_base_tests!(build_uut);

/// Cache filtering must bind exactly the matching items, so an item that stops matching
/// is removed rather than retained as a stale extra entry.
#[test]
fn auto_refresh_filter_update_collection_updated() {
    // Setup: every generated item starts below a derived threshold, so all of them match initially.
    let randomizer = Randomizer::new(AUTO_REFRESH_SEED);
    let source = TestSourceCache::new(|person: &Rc<Person>| person.key());
    let people: Vec<Rc<Person>> = fakers::person().use_seed(randomizer.int()).generate(randomizer.int_range(3, 8) as usize);
    let exclusive_age = people.iter().map(|person| person.age()).max().expect("people were generated") + 1;

    // UUT Initialization
    let (bound, collection) = source
        .connect()
        .auto_refresh(ferroui_system::prop!(Person => age))
        .filter(move |person: &Rc<Person>| person.age() < exclusive_age)
        .validate_synchronization()
        .validate_cache_change_sets(|person: &Rc<Person>| person.key())
        .bind_read_only();
    let subscription = bound.subscribe_ignore();

    // UUT Action
    source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));

    assert_equivalent_because(&collection.to_vec(), &people, "every item matches the predicate");

    // UUT Action
    people[0].set_age(exclusive_age);

    let matching: Vec<Rc<Person>> = people.iter().filter(|person| person.age() < exclusive_age).cloned().collect();
    assert_equivalent_because(&collection.to_vec(), &matching, "an item that stops matching must be removed, not retained as a stale extra entry");

    // UUT Action
    for person in &people {
        person.set_age(exclusive_age);
    }

    assert!(collection.is_empty(), "no item matches the predicate any longer");

    subscription.dispose();
    source.dispose();
}
