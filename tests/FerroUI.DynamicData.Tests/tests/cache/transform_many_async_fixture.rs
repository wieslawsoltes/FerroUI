//! `TransformManyAsyncFixture` (`DynamicData.Tests/Cache/TransformManyAsyncFixture.cs`): the
//! animals of owners, given by futures, merged into one cache.
//!
//! Upstream's "async" tests use `FakeDelay`, a task that has ended, so every future ends at its
//! first poll here as there. Its `TaskTracker` runs each action on the thread pool after that delay
//! (`Task.Run`); on one thread the actions run at once, in order, which the operator sees the same
//! way, and `WhenAll` has nothing left to wait for.
//!
//! `AnimalOwner.ObservableCollection` (`Animals.Connect().Bind(out var collection)`) is not in the
//! domain of the port; the fixture binds the animals of each owner on first use, and disposes the
//! bindings with the owners.

use crate::domain::{fakers, Animal, AnimalFamily, AnimalOwner, Faker, Guid, Randomizer};
use ferroui_dynamic_data::binding::ReadOnlyObservableCollection;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator as ListChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::threading::tasks::{block_on, Task};
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

const INITIAL_OWNER_COUNT: usize = if cfg!(debug_assertions) { 7 } else { 103 };
const ADD_COUNT: usize = if cfg!(debug_assertions) { 5 } else { 53 };
const REMOVE_COUNT: usize = if cfg!(debug_assertions) { 3 } else { 37 };

/// A delay before a selector gives its result (`Func<Task>`).
type DelayFactory = Rc<dyn Fn() -> Task<()>>;

/// `FakeDelay`: a task that has ended (`Task.CompletedTask`).
fn fake_delay() -> Option<DelayFactory> {
    Some(Rc::new(|| Task::from_result(())))
}

/// The bound collection of the animals of each owner (`AnimalOwner.ObservableCollection`), with
/// the subscription of its binding.
#[derive(Default)]
struct OwnerCollections {
    collections: RefCell<HashMap<Guid, (ReadOnlyObservableCollection<Rc<Animal>>, Rc<dyn IDisposable>)>>,
}

impl OwnerCollections {
    /// `owner.ObservableCollection`: `Animals.Connect().Bind(out var collection).Subscribe()` on
    /// first use.
    fn collection(&self, owner: &AnimalOwner) -> ReadOnlyObservableCollection<Rc<Animal>> {
        if let Some((collection, _)) = self.collections.borrow().get(&owner.id()) {
            return collection.clone();
        }
        let (bound, collection) = owner.animals().connect().bind_read_only();
        let subscription = bound.subscribe_ignore();
        self.collections.borrow_mut().insert(owner.id(), (collection.clone(), subscription));
        collection
    }

    fn dispose(&self) {
        let collections = std::mem::take(&mut *self.collections.borrow_mut());
        for (_, (_, subscription)) in collections {
            subscription.dispose();
        }
    }
}

struct Fixture {
    animal_owners: SourceCache<Rc<AnimalOwner>, Guid>,
    animal_owner_results: ChangeSetAggregator<Rc<AnimalOwner>, Guid>,
    animal_owner_faker: Faker<Rc<AnimalOwner>>,
    animal_faker: Faker<Rc<Animal>>,
    randomizer: Randomizer,
    collections: Rc<OwnerCollections>,
}

impl Fixture {
    fn new() -> Self {
        let randomizer = Randomizer::new(0xf7ee_bee7_u32 as i32);

        let animal_faker = fakers::animal().duplicate().with_seed(&randomizer);
        let animal_owner_faker = fakers::animal_owner().duplicate().with_seed(&randomizer).with_initial_animals(&animal_faker);

        let animal_owners = SourceCache::new(|o: &Rc<AnimalOwner>| o.id());
        let animal_owner_results = animal_owners.connect().as_aggregator();
        Self { animal_owners, animal_owner_results, animal_owner_faker, animal_faker, randomizer, collections: Rc::new(OwnerCollections::default()) }
    }

    fn add_initial_owners(&self) {
        self.animal_owners.add_or_update_items(self.animal_owner_faker.generate(INITIAL_OWNER_COUNT));
    }

    /// `CreateObservableCollectionChangeSet(keySelector, delayFactory, equalityComparer, comparer)`.
    fn create_observable_collection_change_set<K: Eq + Hash + Clone + 'static>(
        &self,
        key_selector: fn(&Rc<Animal>) -> K,
        delay_factory: Option<DelayFactory>,
        equality_comparer: Option<Rc<dyn Fn(&Rc<Animal>, &Rc<Animal>) -> bool>>,
        comparer: Option<Rc<dyn Fn(&Rc<Animal>, &Rc<Animal>) -> Ordering>>,
    ) -> Rc<dyn IObservable<ChangeSet<Rc<Animal>, K>>> {
        let collections = self.collections.clone();
        let selector = create_selector(move |owner: &Rc<AnimalOwner>| collections.collection(owner), delay_factory);
        self.animal_owners.connect().transform_many_async_observable_collection_with(move |owner: Rc<AnimalOwner>, guid: Guid| selector(owner, guid), key_selector, equality_comparer, comparer)
    }

    /// `CreateEnumerableChangeSet(keySelector, delayFactory, ..)`.
    fn create_enumerable_change_set(&self, delay_factory: Option<DelayFactory>) -> Rc<dyn IObservable<ChangeSet<Rc<Animal>, i32>>> {
        let selector = create_selector(|owner: &Rc<AnimalOwner>| owner.animals().items(), delay_factory);
        self.animal_owners.connect().transform_many_async_with(move |owner: Rc<AnimalOwner>, guid: Guid| selector(owner, guid), id_key, None, None)
    }

    /// `CreateObservableCacheChangeSet(delayFactory)`.
    fn create_observable_cache_change_set(&self, delay_factory: Option<DelayFactory>) -> Rc<dyn IObservable<ChangeSet<Rc<Animal>, i32>>> {
        let selector = create_selector(|owner: &Rc<AnimalOwner>| owner.observable_cache(), delay_factory);
        self.animal_owners.connect().transform_many_async_observable_cache_with_key(move |owner: Rc<AnimalOwner>, guid: Guid| selector(owner, guid))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.collections.dispose();
        for owner in self.animal_owners.items() {
            owner.dispose();
        }
        self.animal_owner_results.dispose();
        self.animal_owners.dispose();
    }
}

/// `IdKey`.
fn id_key(a: &Rc<Animal>) -> i32 {
    a.id()
}

/// `FamilyKey`.
fn family_key(a: &Rc<Animal>) -> AnimalFamily {
    a.family()
}

/// The future of a selector (`CreateSelector<T>(Func<AnimalOwner, T>, Func<Task>?)`): with a delay
/// factory, the result after the delay; otherwise a future that ends at once
/// (`Task.FromResult(selector(owner))`).
fn create_selector<T: 'static>(
    selector: impl Fn(&Rc<AnimalOwner>) -> T + 'static,
    delay_factory: Option<DelayFactory>,
) -> Rc<dyn Fn(Rc<AnimalOwner>, Guid) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, ObservableError>>>>> {
    let selector = Rc::new(selector);
    Rc::new(move |owner: Rc<AnimalOwner>, _guid: Guid| -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, ObservableError>>>> {
        match &delay_factory {
            // If a delay factory is given, make it async.
            Some(delay_factory) => {
                let delay = delay_factory();
                let selector = selector.clone();
                Box::pin(async move {
                    delay.await?;
                    Ok::<_, ObservableError>(selector(&owner))
                })
            }
            // Otherwise make it not async.
            None => {
                let result = selector(&owner);
                Box::pin(async move { Ok::<_, ObservableError>(result) })
            }
        }
    })
}

/// `CheckResultContents(owners, ownerResults, animalResults)`.
fn check_result_contents<K: Eq + Hash + Clone + 'static>(owners: &[Rc<AnimalOwner>], owner_results: &ChangeSetAggregator<Rc<AnimalOwner>, Guid>, animal_results: &ChangeSetAggregator<Rc<Animal>, K>) {
    let expected_owners = owners.to_vec();

    // These should be subsets of each other.
    let owner_items = owner_results.data().items();
    assert!(expected_owners.iter().all(|owner| owner_items.contains(owner)));
    assert_eq!(owner_items.len(), expected_owners.len());

    // All owner animals should be in the results.
    let animal_items = animal_results.data().items();
    for owner in owners {
        assert!(owner.animals().items().iter().all(|animal| animal_items.contains(animal)));
    }

    // Results should not have more than the total number of animals.
    assert_eq!(animal_results.data().count(), owners.iter().map(|owner| owner.animals().count()).sum::<usize>());
}

/// `CheckResultContents(owners, ownerResults, animalResults, keySelector, comparer)`.
fn check_result_contents_with<K: Eq + Hash + Clone + 'static>(
    owners: &[Rc<AnimalOwner>],
    owner_results: &ChangeSetAggregator<Rc<AnimalOwner>, Guid>,
    animal_results: &ChangeSetAggregator<Rc<Animal>, K>,
    key_selector: fn(&Rc<Animal>) -> K,
    comparer: &dyn Fn(&Rc<Animal>, &Rc<Animal>) -> Ordering,
) {
    let expected_owners = owners.to_vec();

    // These should be subsets of each other.
    let owner_items = owner_results.data().items();
    assert!(expected_owners.iter().all(|owner| owner_items.contains(owner)));
    assert_eq!(owner_items.len(), expected_owners.len());

    // `GroupBy(keySelector).Select(group => group.OrderBy(a => a, comparer).First())`: the least of
    // each key, the first among equals.
    let all_animals: Vec<Rc<Animal>> = expected_owners.iter().flat_map(|owner| owner.animals().items()).collect();
    let mut keys: Vec<K> = Vec::new();
    let mut expected_animals: Vec<Rc<Animal>> = Vec::new();
    for animal in all_animals {
        let key = key_selector(&animal);
        match keys.iter().position(|candidate| *candidate == key) {
            Some(index) => {
                if comparer(&animal, &expected_animals[index]) == Ordering::Less {
                    expected_animals[index] = animal;
                }
            }
            None => {
                keys.push(key);
                expected_animals.push(animal);
            }
        }
    }

    let animal_items = animal_results.data().items();
    assert!(expected_animals.iter().all(|animal| animal_items.contains(animal)));
    assert_eq!(animal_results.data().count(), expected_animals.len());
}

#[test]
fn enumerable_result_contains_all_initial_children_in_single_change_set() {
    let fixture = Fixture::new();

    // Arrange
    fixture.add_initial_owners();

    // Act
    let animal_results = fixture.create_enumerable_change_set(None).as_aggregator();

    // Assert
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 1);
    check_result_contents(&fixture.animal_owners.items(), &fixture.animal_owner_results, &animal_results);

    animal_results.dispose();
}

#[test]
fn result_contains_all_initial_children() {
    let fixture = Fixture::new();

    // Arrange
    fixture.add_initial_owners();

    // Act
    let animal_results = fixture.create_observable_collection_change_set(id_key, None, None, None).as_aggregator();

    // Assert
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 1);
    check_result_contents(&fixture.animal_owners.items(), &fixture.animal_owner_results, &animal_results);

    animal_results.dispose();
}

#[test]
fn result_contains_children_from_added_parents() {
    let fixture = Fixture::new();

    // Arrange
    let animal_results = fixture.create_observable_collection_change_set(id_key, None, None, None).as_aggregator();

    // Act
    fixture.add_initial_owners();

    // Assert
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 1);
    check_result_contents(&fixture.animal_owners.items(), &fixture.animal_owner_results, &animal_results);

    animal_results.dispose();
}

#[test]
fn result_contains_children_from_added_parents_async() {
    let fixture = Fixture::new();

    // Arrange
    let shared = fixture.create_observable_collection_change_set(id_key, fake_delay(), None, None).replay();
    let shared_observable: Rc<dyn IObservable<ChangeSet<Rc<Animal>, i32>>> = shared.clone();
    let animal_results = shared_observable.as_aggregator();
    let connect = shared.connect();
    fixture.add_initial_owners();

    // Act
    block_on(shared_observable.take(1).to_task()).expect("the replay has a change set");

    // Assert
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert!(!animal_results.messages().is_empty());
    check_result_contents(&fixture.animal_owners.items(), &fixture.animal_owner_results, &animal_results);

    connect.dispose();
    animal_results.dispose();
}

#[test]
fn result_contains_added_children_from_existing_parents() {
    let fixture = Fixture::new();

    // Arrange
    fixture.add_initial_owners();
    let animal_results = fixture.create_observable_collection_change_set(id_key, None, None, None).as_aggregator();

    // Act
    for owner in fixture.animal_owners.items() {
        owner.add_animals_between(&fixture.animal_faker, 1, ADD_COUNT);
    }

    // Assert
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    check_result_contents(&fixture.animal_owners.items(), &fixture.animal_owner_results, &animal_results);

    animal_results.dispose();
}

#[test]
fn result_does_not_contain_children_from_removed_parents_async() {
    let fixture = Fixture::new();

    // Arrange
    let animal_results = fixture.create_observable_collection_change_set(id_key, fake_delay(), None, None).as_aggregator();
    fixture.add_initial_owners();
    let removed_owners = fixture.randomizer.list_items(&fixture.animal_owners.items(), REMOVE_COUNT);
    // `taskTracker.Add(() => _animalOwners.Remove(removedOwners))`: at once.
    fixture.animal_owners.remove_items(removed_owners);

    // Act: `await taskTracker.WhenAll()` has nothing left to wait for.

    // Assert
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT - REMOVE_COUNT);
    check_result_contents(&fixture.animal_owners.items(), &fixture.animal_owner_results, &animal_results);

    animal_results.dispose();
}

#[test]
fn results_work_with_comparer() {
    let fixture = Fixture::new();

    // Arrange
    let animal_results = fixture.create_observable_collection_change_set(family_key, None, None, Some(Animal::name_comparer())).as_aggregator();

    // Act
    fixture.add_initial_owners();

    // Assert
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    check_result_contents_with(&fixture.animal_owners.items(), &fixture.animal_owner_results, &animal_results, family_key, &*Animal::name_comparer());

    animal_results.dispose();
}

#[test]
fn results_with_observable_cache_changes_async() {
    let fixture = Fixture::new();

    // Arrange
    let animal_results = fixture.create_observable_cache_change_set(fake_delay()).as_aggregator();
    fixture.add_initial_owners();
    let owner_add_count = fixture.randomizer.number_range(1, ADD_COUNT as i32) as usize;
    // `taskTracker.Add(action, count)`: each action at once.
    for _ in 0..owner_add_count {
        fixture.animal_owners.add_or_update(fixture.animal_owner_faker.generate_one());
    }
    for _ in 0..ADD_COUNT {
        let owner = fixture.randomizer.list_item(&fixture.animal_owners.items());
        owner.add_animals_between(&fixture.animal_faker, 1, ADD_COUNT);
    }

    // Act: `await taskTracker.WhenAll()` has nothing left to wait for.

    // Assert
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT + owner_add_count);
    check_result_contents(&fixture.animal_owners.items(), &fixture.animal_owner_results, &animal_results);

    animal_results.dispose();
}

/// `ResultCompletesOnlyWhenSourceCompletes(bool completeSource)`.
fn result_completes_only_when_source_completes(complete_source: bool) {
    let fixture = Fixture::new();

    // Arrange
    let animal_results = fixture.create_observable_collection_change_set(id_key, None, None, None).as_aggregator();
    fixture.add_initial_owners();

    // Act
    if complete_source {
        fixture.animal_owners.dispose();
    }

    // Assert
    assert_eq!(fixture.animal_owner_results.is_completed(), complete_source);
    check_result_contents(&fixture.animal_owners.items(), &fixture.animal_owner_results, &animal_results);

    animal_results.dispose();
}

#[test]
fn result_completes_only_when_source_completes_case1() {
    result_completes_only_when_source_completes(false);
}

#[test]
fn result_completes_only_when_source_completes_case2() {
    result_completes_only_when_source_completes(true);
}

#[test]
fn result_fails_if_source_fails() {
    let fixture = Fixture::new();

    // Arrange
    let expected_error: ObservableError = new_error("Expected");
    let throw_observable = Observable::throw::<ChangeSet<Rc<AnimalOwner>, Guid>>(expected_error.clone());
    let results: ListChangeSetAggregator<Rc<Animal>> =
        fixture.animal_owners.connect().concat(throw_observable).merge_many_list_change_sets(|owner: &Rc<AnimalOwner>| owner.animals().connect()).as_aggregator();

    // Act
    fixture.animal_owners.dispose();

    // Assert
    let error = results.error().expect("the sequence failed");
    assert!(Rc::ptr_eq(&error, &expected_error));

    results.dispose();
}
