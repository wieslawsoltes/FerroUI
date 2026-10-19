//! `MergeManyChangeSetsFixture.ForListChangesets.UnitTests`
//! (`DynamicData.Tests/Cache/MergeManyChangeSetsFixture.ForListChangesets.UnitTests.cs`): the
//! change sets of the lists of the items of a cache, merged into those of one list
//! (`merge_many_list_change_sets`).
//!
//! The constants and `CheckResultContents` of the partial class
//! (`MergeManyChangeSetsFixture.ForListChangesets.cs`) are repeated here, with the counts of
//! upstream's debug build.

use crate::domain::{fakers, Animal, AnimalOwner, Faker, Guid, Randomizer};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator as CacheChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ISourceUpdater, ObservableCacheEx, SourceCache};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{remove_many, IExtendedList, IObservableList, ObservableListEx};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, ObservableError};
use std::rc::Rc;

/// `InitialOwnerCount` (debug build).
const INITIAL_OWNER_COUNT: usize = 7;
/// `AddRangeSize` (debug build).
const ADD_RANGE_SIZE: usize = 5;
/// `RemoveRangeSize` (debug build).
const REMOVE_RANGE_SIZE: usize = 3;

/// The seed of every test.
const SEED: i32 = 0x0122_1948;

type OwnerResults = CacheChangeSetAggregator<Rc<AnimalOwner>, Guid>;
type AnimalResults = ChangeSetAggregator<Rc<Animal>>;

/// The arrangement every test shares: a cache of owners keyed by their identifiers and seeded
/// fakers of animals and owners with their first animals.
struct Arranged {
    animal_owners: SourceCache<Rc<AnimalOwner>, Guid>,
    randomizer: Randomizer,
    animal_faker: Faker<Rc<Animal>>,
    animal_owner_faker: Faker<Rc<AnimalOwner>>,
}

impl Arranged {
    fn new() -> Self {
        let animal_owners = SourceCache::new(|owner: &Rc<AnimalOwner>| owner.id());
        let randomizer = Randomizer::new(SEED);
        let animal_faker = fakers::animal().duplicate().with_seed(&randomizer);
        let animal_owner_faker = fakers::animal_owner().duplicate().with_seed(&randomizer).with_initial_animals(&animal_faker);
        Self { animal_owners, randomizer, animal_faker, animal_owner_faker }
    }

    /// `animalOwners.AddOrUpdate(animalOwnerFaker.Generate(InitialOwnerCount))`.
    fn add_initial_owners(&self) {
        let owners = self.animal_owner_faker.generate(INITIAL_OWNER_COUNT);
        add_or_update_owners(&self.animal_owners, owners);
    }

    /// `animalOwners.Connect().AsAggregator()`.
    fn owner_results(&self) -> OwnerResults {
        self.animal_owners.connect().as_aggregator()
    }

    /// `animalOwners.Connect().MergeManyChangeSets(owner => owner.Animals.Connect()).AsAggregator()`.
    fn animal_results(&self) -> AnimalResults {
        self.animal_owners.connect().merge_many_list_change_sets(|owner: &Rc<AnimalOwner>| owner.animals().connect()).as_aggregator()
    }

    /// `randomizer.ListItem(animalOwners.Items.ToList())`.
    fn random_owner(&self) -> Rc<AnimalOwner> {
        self.randomizer.list_item(&self.animal_owners.items())
    }
}

fn add_or_update_owners(cache: &SourceCache<Rc<AnimalOwner>, Guid>, owners: Vec<Rc<AnimalOwner>>) {
    cache.edit(|updater: &mut dyn ISourceUpdater<Rc<AnimalOwner>, Guid>| updater.add_or_update_items(&mut owners.into_iter()));
}

fn total_animals(owners: &[Rc<AnimalOwner>]) -> usize {
    owners.iter().map(|owner| owner.animals().count()).sum()
}

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
    assert_eq!(animal_results.data().count(), total_animals(owners));
}

/// `CreateWithSameId`: a new owner of the identifier of `original`, with the animals of a new
/// owner of the faker.
fn create_with_same_id(animal_owner_faker: &Faker<Rc<AnimalOwner>>, original: &Rc<AnimalOwner>) -> Rc<AnimalOwner> {
    let new_owner = animal_owner_faker.generate_one();
    let same_id = AnimalOwner::with_id(new_owner.name(), Some(original.id()), true);
    let animals = new_owner.animals().items();
    same_id.animals().edit(|list: &mut dyn IExtendedList<Rc<Animal>>| list.add_range(animals));
    same_id
}

// Not ported: NullChecks - argument validation

fn result_completes_only_when_source_and_all_children_complete(complete_source: bool, complete_children: bool) {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    // Act
    for owner in arranged.animal_owners.items().iter().skip(if complete_children { 0 } else { 1 }) {
        owner.dispose();
    }
    if complete_source {
        arranged.animal_owners.dispose();
    }

    // Assert
    assert_eq!(animal_owner_results.is_completed(), complete_source);
    assert_eq!(animal_results.is_completed(), complete_source && complete_children);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case1() {
    result_completes_only_when_source_and_all_children_complete(false, false);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case2() {
    result_completes_only_when_source_and_all_children_complete(false, true);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case3() {
    result_completes_only_when_source_and_all_children_complete(true, false);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case4() {
    result_completes_only_when_source_and_all_children_complete(true, true);
}

#[test]
fn result_contains_all_initial_children() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 1);
    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_contains_children_from_added_parents() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let add_this = arranged.animal_owner_faker.generate_one();

    // Act
    add_or_update_owners(&arranged.animal_owners, vec![add_this.clone()]);

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT + 1);
    assert_eq!(animal_results.messages().len(), 2);
    let result_animals = animal_results.data().items();
    for added in add_this.animals().items() {
        assert!(result_animals.contains(&added));
    }

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_contains_children_added_with_add_range() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let initial_count = total_animals(&arranged.animal_owners.items());
    let mut total_added: Vec<Rc<Animal>> = Vec::new();

    // Act
    for owner in arranged.animal_owners.items() {
        let added = arranged.animal_faker.generate(ADD_RANGE_SIZE);
        total_added.extend(added.iter().cloned());
        owner.animals().edit(|list: &mut dyn IExtendedList<Rc<Animal>>| list.add_range(added));
    }

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 1 + INITIAL_OWNER_COUNT); // Initial + 1 for each Range Added
    let result_animals = animal_results.data().items();
    for animal in &total_added {
        assert!(result_animals.contains(animal));
    }
    assert_eq!(total_animals(&arranged.animal_owners.items()), initial_count + total_added.len());

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_contains_children_added_with_insert() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let random_owner = arranged.random_owner();
    let insert_index = arranged.randomizer.number(random_owner.animals().items().len() as i32) as usize;
    let insert_this = arranged.animal_faker.generate_one();
    let initial_count = total_animals(&arranged.animal_owners.items());

    // Act
    {
        let insert_this = insert_this.clone();
        random_owner.animals().edit(|list: &mut dyn IExtendedList<Rc<Animal>>| list.insert(insert_index, insert_this));
    }

    // Assert
    assert_eq!(random_owner.animals().items()[insert_index], insert_this);
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 2);
    assert!(animal_results.data().items().contains(&insert_this));
    assert_eq!(total_animals(&arranged.animal_owners.items()), initial_count + 1);

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_contains_correct_items_after_child_clear() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let random_owner = arranged.random_owner();
    let removed_animals = random_owner.animals().items();

    // Act
    random_owner.animals().edit(|list: &mut dyn IExtendedList<Rc<Animal>>| list.clear());

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 2);
    assert_eq!(random_owner.animals().count(), 0);
    let result_animals = animal_results.data().items();
    for removed in &removed_animals {
        assert!(!result_animals.contains(removed));
    }

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_contains_correct_items_after_child_replacement() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let random_owner = arranged.random_owner();
    let replace_this = arranged.randomizer.list_item(&random_owner.animals().items());
    let with_this = arranged.animal_faker.generate_one();

    // Act: `Replace` (`ListEx.Replace`)
    {
        let (replace_this, with_this) = (replace_this.clone(), with_this.clone());
        random_owner.animals().edit(move |list: &mut dyn IExtendedList<Rc<Animal>>| {
            let index = list.index_of(&replace_this).expect("the animal to replace is in the list");
            list.set(index, with_this);
        });
    }

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 2);
    let owner_animals = random_owner.animals().items();
    assert!(!owner_animals.contains(&replace_this));
    assert!(owner_animals.contains(&with_this));

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_contains_correct_items_after_parent_update() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let replace_this = arranged.random_owner();
    let with_this = create_with_same_id(&arranged.animal_owner_faker, &replace_this);

    // Act
    add_or_update_owners(&arranged.animal_owners, vec![with_this.clone()]);

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT); // Owner Count should not change
    assert_eq!(animal_results.messages().len(), 2); // 2 = Initial Add and one changeset with remove old items / add new items
    let result_animals = animal_results.data().items();
    for removed in replace_this.animals().items() {
        assert!(!result_animals.contains(&removed));
    }
    for added in with_this.animals().items() {
        assert!(result_animals.contains(&added));
    }

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);

    replace_this.dispose();
}

#[test]
fn result_does_not_contain_children_from_parents_batch_removed() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let remove_these = arranged.randomizer.list_items(&arranged.animal_owners.items(), REMOVE_RANGE_SIZE);

    // Act
    {
        let remove_these = remove_these.clone();
        arranged
            .animal_owners
            .edit(move |updater: &mut dyn ISourceUpdater<Rc<AnimalOwner>, Guid>| updater.remove_items(&mut remove_these.into_iter()));
    }

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT - REMOVE_RANGE_SIZE);
    assert_eq!(animal_results.messages().len(), 2);
    let result_animals = animal_results.data().items();
    for removed in remove_these.iter().flat_map(|owner| owner.animals().items()) {
        assert!(!result_animals.contains(&removed));
    }

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);

    for owner in &remove_these {
        owner.dispose();
    }
}

// Not ported: ResultDoesNotContainChildrenFromParentAddedAndRemovedWhileAnotherThreadIsDelivering - real threads

#[test]
fn result_does_not_contain_children_from_parents_removed_with_remove() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let remove_this = arranged.random_owner();

    // Act
    {
        let remove_this = remove_this.clone();
        arranged.animal_owners.edit(move |updater: &mut dyn ISourceUpdater<Rc<AnimalOwner>, Guid>| updater.remove_item(&remove_this));
    }

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT - 1);
    assert_eq!(animal_results.messages().len(), 2);
    let result_animals = animal_results.data().items();
    for removed in remove_this.animals().items() {
        assert!(!result_animals.contains(&removed));
    }

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);

    remove_this.dispose();
}

#[test]
fn result_does_not_contain_children_removed_with_remove() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let random_owner = arranged.random_owner();
    let remove_this = arranged.randomizer.list_item(&random_owner.animals().items());
    let initial_count = total_animals(&arranged.animal_owners.items());

    // Act
    {
        let remove_this = remove_this.clone();
        random_owner.animals().edit(move |list: &mut dyn IExtendedList<Rc<Animal>>| {
            list.remove(&remove_this);
        });
    }

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 2);
    assert!(!animal_results.data().items().contains(&remove_this));
    assert_eq!(total_animals(&arranged.animal_owners.items()), initial_count - 1);

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_does_not_contain_children_removed_with_remove_at() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let random_owner = arranged.random_owner();
    let remove_index = arranged.randomizer.number(random_owner.animals().count() as i32 - 1) as usize;
    let remove_this = random_owner.animals().items()[remove_index].clone();
    let initial_count = total_animals(&arranged.animal_owners.items());

    // Act
    random_owner.animals().edit(|list: &mut dyn IExtendedList<Rc<Animal>>| list.remove_at(remove_index));

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 2);
    assert!(!animal_results.data().items().contains(&remove_this));
    assert_eq!(total_animals(&arranged.animal_owners.items()), initial_count - 1);

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_does_not_contain_children_removed_with_remove_many() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let random_owner = arranged.random_owner();
    let remove_count = arranged.randomizer.number_range(1, random_owner.animals().count() as i32 - 1) as usize;
    let remove_these = arranged.randomizer.list_items(&random_owner.animals().items(), remove_count);

    // Act: `RemoveMany` (`ListEx.RemoveMany`)
    {
        let remove_these = remove_these.clone();
        random_owner.animals().edit(move |list: &mut dyn IExtendedList<Rc<Animal>>| remove_many(&mut *list, &remove_these));
    }

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 2);
    let owner_animals = random_owner.animals().items();
    for removed in &remove_these {
        assert!(!owner_animals.contains(removed));
    }

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_does_not_contain_children_removed_with_remove_range() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let random_owner = arranged.random_owner();
    let remove_count = arranged.randomizer.number_range(1, random_owner.animals().count() as i32 - 1) as usize;
    let remove_index = arranged.randomizer.number(random_owner.animals().count() as i32 - remove_count as i32 - 1) as usize;
    // `Items` is a copy, so these are the animals at those places before the removal.
    let remove_these: Vec<Rc<Animal>> = random_owner.animals().items().into_iter().skip(remove_index).take(remove_count).collect();

    // Act
    random_owner.animals().edit(|list: &mut dyn IExtendedList<Rc<Animal>>| list.remove_range(remove_index, remove_count));

    // Assert
    assert_eq!(animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(animal_results.messages().len(), 2);
    let owner_animals = random_owner.animals().items();
    for removed in &remove_these {
        assert!(!owner_animals.contains(removed));
    }

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);
}

#[test]
fn result_empty_if_source_is_cleared() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let animal_owner_results = arranged.owner_results();
    let animal_results = arranged.animal_results();

    let items = arranged.animal_owners.items();

    // Act
    arranged.animal_owners.edit(|updater: &mut dyn ISourceUpdater<Rc<AnimalOwner>, Guid>| updater.clear());

    // Assert
    assert_eq!(animal_owner_results.data().count(), 0);
    assert_eq!(animal_results.data().count(), 0);

    check_result_contents(&arranged.animal_owners.items(), &animal_owner_results, &animal_results);

    for owner in &items {
        owner.dispose();
    }
}

#[test]
fn result_fails_if_source_fails() {
    let arranged = Arranged::new();

    // Arrange
    arranged.add_initial_owners();

    let expected_error: ObservableError = new_error("Expected");
    let throw_observable = Observable::throw::<ChangeSet<Rc<AnimalOwner>, Guid>>(expected_error.clone());
    let results: AnimalResults = arranged
        .animal_owners
        .connect()
        .concat(throw_observable)
        .merge_many_list_change_sets(|owner: &Rc<AnimalOwner>| owner.animals().connect())
        .as_aggregator();

    // Act
    arranged.animal_owners.dispose();

    // Assert
    let error = results.error().expect("the result failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}
