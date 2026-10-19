//! `MergeManyChangeSetsFixture.ForListChangesets.IntegrationTests`
//! (`DynamicData.Tests/Cache/MergeManyChangeSetsFixture.ForListChangesets.IntegrationTests.cs`):
//! the merged change sets of the lists of owners while owners and animals are added and removed
//! at random times, and observers come and go.
//!
//! The constants and `CheckResultContents` of the partial class
//! (`MergeManyChangeSetsFixture.ForListChangesets.cs`) are repeated here, with the counts of
//! upstream's debug build; the cases of the theory are those of the debug build too.
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream runs the stress on the task pool
//! (`TaskPoolScheduler.Default`, `Environment.ProcessorCount` streams in parallel) and subscribes
//! and unsubscribes between `Task.Yield()`s while it runs. Here it runs on one thread, on a
//! `TestScheduler`: the clock advances by a millisecond before each subscription and before each
//! unsubscription, the streams are interleaved by their random due times, and their number is
//! fixed so that a seed gives the same run on every machine.

use crate::domain::{fakers, Animal, AnimalOwner, Guid, Randomizer};
use crate::utilities::{ObservableExtensions, StressAddRemoveExtensions};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator as CacheChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ISourceUpdater, ObservableCacheEx, SourceCache};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{self, IObservableList, ObservableListEx};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::IConnectableObservable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// `InitialOwnerCount` (debug build).
const INITIAL_OWNER_COUNT: usize = 7;

/// The seed of the test.
const SEED: i32 = 0x0122_1948;

/// The number of streams that add and remove at once (`Environment.ProcessorCount` upstream).
const PARALLEL: usize = 4;

/// The most rounds of subscribing and unsubscribing before the test gives up on the stress
/// ending.
const MAX_ROUNDS: usize = 1_000_000;

type OwnerResults = CacheChangeSetAggregator<Rc<AnimalOwner>, Guid>;
type AnimalResults = ChangeSetAggregator<Rc<Animal>>;

/// `CheckResultContents`.
fn check_result_contents(owners: &[Rc<AnimalOwner>], owner_results: &OwnerResults, animal_results: &AnimalResults) {
    let result_owners = owner_results.data().items();

    // These should be subsets of each other
    for owner in owners {
        assert!(result_owners.contains(owner), "the owner {owner} should be in the results");
    }
    assert_eq!(result_owners.len(), owners.len());

    // All owner animals should be in the results
    let result_animals = animal_results.data().items();
    for owner in owners {
        for animal in owner.animals().items() {
            assert!(result_animals.contains(&animal), "the animal {animal} should be in the results");
        }
    }

    // Results should not have more than the total number of animals
    assert_eq!(animal_results.data().count(), owners.iter().map(|owner| owner.animals().count()).sum::<usize>());
}

fn multi_threaded_stress_test(owner_count: usize, animal_count: usize) {
    let max_add_time = Duration::from_millis(250);
    let max_remove_time = Duration::from_millis(100);

    let test_scheduler = TestScheduler::new();
    let scheduler: Rc<dyn IScheduler> = test_scheduler.scheduler();

    let animal_owners: SourceCache<Rc<AnimalOwner>, Guid> = SourceCache::new(|owner: &Rc<AnimalOwner>| owner.id());

    let randomizer = Randomizer::new(SEED);
    let animal_faker = fakers::animal().duplicate().with_seed(&randomizer);
    let animal_owner_faker = fakers::animal_owner().duplicate().with_seed(&randomizer).with_initial_animals(&animal_faker);

    {
        let owners = animal_owner_faker.generate(INITIAL_OWNER_COUNT);
        animal_owners.edit(|updater: &mut dyn ISourceUpdater<Rc<AnimalOwner>, Guid>| updater.add_or_update_items(&mut owners.into_iter()));
    }

    let animal_owner_results: OwnerResults = animal_owners.connect().as_aggregator();

    let merge_animals: Rc<dyn IConnectableObservable<list::ChangeSet<Rc<Animal>>>> =
        animal_owners.connect().merge_many_list_change_sets(|owner: &Rc<AnimalOwner>| owner.animals().connect()).publish();
    let merge_animals_source: Rc<dyn IObservable<list::ChangeSet<Rc<Animal>>>> = merge_animals.clone();
    let adding_animals = Rc::new(Cell::new(true));

    // `mergeAnimals.LastOrDefaultAsync().ToTask()`: whether the merged sequence has completed.
    let cache_completed = Rc::new(Cell::new(false));
    let cache_completed_subscription = {
        let cache_completed = cache_completed.clone();
        merge_animals_source.subscribe_next_completed(|_| {}, move || cache_completed.set(true))
    };
    let animal_results: AnimalResults = merge_animals_source.as_aggregator();
    let connect = merge_animals.connect();

    // `GetRemoveTime`
    let get_remove_time: Rc<dyn Fn() -> Option<Duration>> = {
        let randomizer = randomizer.clone();
        Rc::new(move || if randomizer.bool() { Some(randomizer.time_span(max_remove_time)) } else { None })
    };

    // `AddRemoveOwners`
    let add_remove_owners: Rc<dyn IObservable<Rc<AnimalOwner>>> = {
        let owners = animal_owners.clone();
        let owners_to_dispose = animal_owners.clone();
        let get_remove_time = get_remove_time.clone();
        let stress_scheduler = scheduler.clone();
        animal_owner_faker
            .interval_generate(&randomizer, max_add_time, Some(scheduler.clone()))
            .parallelize_with(owner_count, PARALLEL, move |items: Rc<dyn IObservable<Rc<AnimalOwner>>>| {
                let get_remove_time = get_remove_time.clone();
                items.stress_add_remove_cache(owners.clone(), move |_: &Rc<AnimalOwner>| get_remove_time(), stress_scheduler.clone())
            })
            .finally(move || owners_to_dispose.dispose())
    };

    // `AddRemoveAnimals`
    let add_remove_animals: Rc<dyn Fn(&Rc<AnimalOwner>) -> Rc<dyn IObservable<Rc<Animal>>>> = {
        let animal_faker = animal_faker.clone();
        let randomizer = randomizer.clone();
        let get_remove_time = get_remove_time.clone();
        let scheduler = scheduler.clone();
        Rc::new(move |owner: &Rc<AnimalOwner>| {
            let animals = owner.animals().clone();
            let animals_to_dispose = owner.animals().clone();
            let get_remove_time = get_remove_time.clone();
            let stress_scheduler = scheduler.clone();
            animal_faker
                .interval_generate(&randomizer, max_add_time, Some(scheduler.clone()))
                .parallelize_with(animal_count, PARALLEL, move |items: Rc<dyn IObservable<Rc<Animal>>>| {
                    let get_remove_time = get_remove_time.clone();
                    items.stress_add_remove_list(animals.clone(), move |_: &Rc<Animal>| get_remove_time(), stress_scheduler.clone())
                })
                .finally(move || animals_to_dispose.dispose())
        })
    };

    // `AddRemoveAnimalsStress`
    let add_remove_animals_stress: Rc<dyn IObservable<()>> = {
        let add_remove_owners = add_remove_owners.clone();
        let add_remove_animals = add_remove_animals.clone();
        let owners = animal_owners.clone();
        Observable::create(move |observer: Rc<dyn IObserver<()>>| {
            let owners_error = observer.clone();
            let owners_subscription = add_remove_owners.subscribe_next_error(|_| {}, move |error: ObservableError| owners_error.on_error(error));

            let (animals_error, animals_completed) = (observer.clone(), observer.clone());
            let add_remove_animals = add_remove_animals.clone();
            let animals_subscription = owners
                .connect()
                .merge_many(move |owner: &Rc<AnimalOwner>| add_remove_animals(owner))
                .subscribe_all(|_| {}, move |error: ObservableError| animals_error.on_error(error), move || animals_completed.on_completed());

            Disposable::create(move || {
                owners_subscription.dispose();
                animals_subscription.dispose();
            })
        })
    };

    // Start modifying the parent list and the child lists
    let add_animals = {
        let adding_animals = adding_animals.clone();
        add_remove_animals_stress.finally(move || adding_animals.set(false)).subscribe_ignore()
    };

    // Subscribe / unsubscribe over and over while the collections are being modified
    let step = duration_to_ticks(Duration::from_millis(1));
    let mut rounds = 0;
    while adding_animals.get() {
        // Ensure items are being added before subscribing to the animal changes
        test_scheduler.advance_by(step);

        // Subscribe
        let merged_subscription = merge_animals_source.subscribe_ignore();

        // Let the other streams run
        test_scheduler.advance_by(step);

        // Unsubscribe
        merged_subscription.dispose();

        rounds += 1;
        assert!(rounds < MAX_ROUNDS, "the stress should end");
    }

    // The source cache has finished delivering all notifications.
    assert!(cache_completed.get(), "the merged sequence should complete");

    // Verify the results against the aggregator wired into the same Publish chain
    // that cacheCompleted observes.
    check_result_contents(&animal_owners.items(), &animal_owner_results, &animal_results);

    add_animals.dispose();
    connect.dispose();
    cache_completed_subscription.dispose();
}

#[test]
fn multi_threaded_stress_test_case1() {
    multi_threaded_stress_test(5, 7);
}

#[test]
fn multi_threaded_stress_test_case2() {
    multi_threaded_stress_test(10, 50);
}
