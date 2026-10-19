//! `MergeManyChangeSetsListFixture` (`DynamicData.Tests/List/MergeManyChangeSetsListFixture.cs`):
//! the change sets of the animal lists of the owners in a list, merged into one list, as owners and
//! animals are added, removed and replaced.

use crate::domain::{fakers, Animal, AnimalOwner, Faker, Randomizer};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{remove_many, replace, ChangeSet, IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::IDisposable;
use std::fmt::Debug;
use std::rc::Rc;

const INITIAL_OWNER_COUNT: usize = if cfg!(debug_assertions) { 7 } else { 103 };
const ADD_RANGE_SIZE: usize = if cfg!(debug_assertions) { 5 } else { 53 };
const REMOVE_RANGE_SIZE: usize = if cfg!(debug_assertions) { 3 } else { 37 };

/// The fixture: a list of owners with their animals, the results of the owners and those of the
/// merged animals; disposed at the end of each test.
struct Fixture {
    animal_owners: SourceList<Rc<AnimalOwner>>,
    animal_owner_results: ChangeSetAggregator<Rc<AnimalOwner>>,
    animal_results: ChangeSetAggregator<Rc<Animal>>,
    animal_owner_faker: Faker<Rc<AnimalOwner>>,
    animal_faker: Faker<Rc<Animal>>,
    randomizer: Randomizer,
}

impl Fixture {
    fn new() -> Self {
        let randomizer = Randomizer::new(0x12291977);
        let animal_faker = fakers::animal().with_seed(&randomizer);
        let animal_owner_faker = fakers::animal_owner().with_seed(&randomizer).with_initial_animals(&animal_faker);
        let animal_owners: SourceList<Rc<AnimalOwner>> = SourceList::new();
        let initial = animal_owner_faker.generate(INITIAL_OWNER_COUNT);
        animal_owners.edit(move |list| list.add_range(initial));

        let animal_owner_results = animal_owners.connect().as_aggregator();
        let animal_results = animal_owners.connect().merge_many_change_sets(|owner: &Rc<AnimalOwner>| owner.animals().connect()).as_aggregator();
        Self { animal_owners, animal_owner_results, animal_results, animal_owner_faker, animal_faker, randomizer }
    }

    /// `_animalOwners.Items.ToList()`.
    fn owners(&self) -> Vec<Rc<AnimalOwner>> {
        self.animal_owners.items()
    }

    /// `_animalOwners.Items.Sum(owner => owner.Animals.Count)`.
    fn total_animal_count(&self) -> usize {
        self.owners().iter().map(|owner| owner.animals().count()).sum()
    }

    /// `CheckResultContents()`.
    fn check_result_contents(&self) {
        check_result_contents(&self.owners(), &self.animal_owner_results, &self.animal_results);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for owner in self.animal_owners.items() {
            owner.dispose();
        }
        self.animal_owner_results.dispose();
        self.animal_results.dispose();
        self.animal_owners.dispose();
    }
}

/// `BeSubsetOf`: every item of `subset` is in `superset`.
fn assert_subset<T: PartialEq + Debug>(subset: &[T], superset: &[T]) {
    for item in subset {
        assert!(superset.contains(item), "expected {item:?} to be in {superset:?}");
    }
}

/// `CheckResultContents(IEnumerable<AnimalOwner>, ChangeSetAggregator<AnimalOwner>, ChangeSetAggregator<Animal>)`.
fn check_result_contents(owners: &[Rc<AnimalOwner>], owner_results: &ChangeSetAggregator<Rc<AnimalOwner>>, animal_results: &ChangeSetAggregator<Rc<Animal>>) {
    let expected_owners = owners.to_vec();
    let owner_items = owner_results.data().items();

    // These should be subsets of each other
    assert_subset(&expected_owners, &owner_items);
    assert_eq!(owner_items.len(), expected_owners.len());

    // All owner animals should be in the results
    let animal_items = animal_results.data().items();
    for owner in owners {
        assert_subset(&owner.animals().items(), &animal_items);
    }

    // Results should not have more than the total number of animals
    assert_eq!(animal_results.data().count(), owners.iter().map(|owner| owner.animals().count()).sum::<usize>());
}

// Not ported: MultiThreadedStressTest - real threads

// Not ported: NullChecks - argument validation

#[test]
fn result_contains_all_initial_children() {
    let fixture = Fixture::new();

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 1);
    fixture.check_result_contents();
}

#[test]
fn result_contains_children_from_parents_added_with_add_range() {
    let fixture = Fixture::new();
    let add_these = fixture.animal_owner_faker.generate(ADD_RANGE_SIZE);

    let added = add_these.clone();
    fixture.animal_owners.edit(move |list| list.add_range(added));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT + ADD_RANGE_SIZE);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for additional add
    let animal_items = fixture.animal_results.data().items();
    for added in add_these.iter().flat_map(|owner| owner.animals().items()) {
        assert!(animal_items.contains(&added));
    }
    fixture.check_result_contents();
}

#[test]
fn result_contains_children_from_parents_added_with_add() {
    let fixture = Fixture::new();
    let add_this = fixture.animal_owner_faker.generate_one();

    let added = add_this.clone();
    fixture.animal_owners.edit(move |list| list.add(added));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT + 1);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for additional add
    let animal_items = fixture.animal_results.data().items();
    for added in add_this.animals().items() {
        assert!(animal_items.contains(&added));
    }
    fixture.check_result_contents();
}

#[test]
fn result_contains_children_from_parents_added_with_insert() {
    let fixture = Fixture::new();
    let insert_index = fixture.randomizer.number(fixture.animal_owners.count() as i32) as usize;
    let insert_this = fixture.animal_owner_faker.generate_one();

    let inserted = insert_this.clone();
    fixture.animal_owners.edit(move |list| list.insert(insert_index, inserted));

    assert_eq!(fixture.animal_owners.items()[insert_index], insert_this);
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT + 1);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for additional add
    let animal_items = fixture.animal_results.data().items();
    for added in insert_this.animals().items() {
        assert!(animal_items.contains(&added));
    }
    fixture.check_result_contents();
}

#[test]
fn result_does_not_contain_children_from_parents_removed_with_remove() {
    let fixture = Fixture::new();
    let remove_this = fixture.randomizer.list_item(&fixture.owners());

    let removed = remove_this.clone();
    fixture.animal_owners.edit(move |list| {
        list.remove(&removed);
    });

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT - 1);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    let animal_items = fixture.animal_results.data().items();
    for removed in remove_this.animals().items() {
        assert!(!animal_items.contains(&removed));
    }
    fixture.check_result_contents();
    remove_this.dispose();
}

#[test]
fn result_does_not_contain_children_from_parents_removed_with_remove_at() {
    let fixture = Fixture::new();
    let remove_index = fixture.randomizer.number(fixture.animal_owners.count() as i32 - 1) as usize;
    let remove_this = fixture.animal_owners.items()[remove_index].clone();

    fixture.animal_owners.edit(move |list| list.remove_at(remove_index));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT - 1);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    let animal_items = fixture.animal_results.data().items();
    for removed in remove_this.animals().items() {
        assert!(!animal_items.contains(&removed));
    }
    fixture.check_result_contents();
    remove_this.dispose();
}

#[test]
fn result_does_not_contain_children_from_parents_removed_with_remove_range() {
    let fixture = Fixture::new();
    let remove_index = fixture.randomizer.number(fixture.animal_owners.count() as i32 - REMOVE_RANGE_SIZE as i32 - 1) as usize;
    let remove_these: Vec<Rc<AnimalOwner>> = fixture.owners().into_iter().skip(remove_index).take(REMOVE_RANGE_SIZE).collect();

    fixture.animal_owners.edit(move |list| list.remove_range(remove_index, REMOVE_RANGE_SIZE));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT - REMOVE_RANGE_SIZE);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    let animal_items = fixture.animal_results.data().items();
    for removed in remove_these.iter().flat_map(|owner| owner.animals().items()) {
        assert!(!animal_items.contains(&removed));
    }
    fixture.check_result_contents();
    for owner in &remove_these {
        owner.dispose();
    }
}

#[test]
fn result_does_not_contain_children_from_parents_removed_with_remove_many() {
    let fixture = Fixture::new();
    let remove_these = fixture.randomizer.list_items(&fixture.owners(), REMOVE_RANGE_SIZE);

    let removed = remove_these.clone();
    fixture.animal_owners.edit(move |list| remove_many(list, &removed));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT - REMOVE_RANGE_SIZE);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    let animal_items = fixture.animal_results.data().items();
    for removed in remove_these.iter().flat_map(|owner| owner.animals().items()) {
        assert!(!animal_items.contains(&removed));
    }
    fixture.check_result_contents();
    for owner in &remove_these {
        owner.dispose();
    }
}

#[test]
fn result_contains_correct_items_after_parent_replacement() {
    let fixture = Fixture::new();
    let replace_this = fixture.randomizer.list_item(&fixture.owners());
    let with_this = fixture.animal_owner_faker.generate_one();

    let (original, replacement) = (replace_this.clone(), with_this.clone());
    fixture.animal_owners.edit(move |list| replace(list, &original, replacement));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT); // Owner Count should not change
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    let animal_items = fixture.animal_results.data().items();
    for removed in replace_this.animals().items() {
        assert!(!animal_items.contains(&removed));
    }
    for added in with_this.animals().items() {
        assert!(animal_items.contains(&added));
    }
    fixture.check_result_contents();
    replace_this.dispose();
}

#[test]
fn result_empty_if_source_is_cleared() {
    let fixture = Fixture::new();
    let items = fixture.owners();

    fixture.animal_owners.edit(|list| list.clear());

    assert_eq!(fixture.animal_owner_results.data().count(), 0);
    assert_eq!(fixture.animal_results.data().count(), 0);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    fixture.check_result_contents();
    for owner in &items {
        owner.dispose();
    }
}

#[test]
fn result_contains_children_added_with_add_range() {
    let fixture = Fixture::new();
    let random_owner = fixture.randomizer.list_item(&fixture.owners());
    let add_these = fixture.animal_faker.generate(ADD_RANGE_SIZE);
    let initial_count = fixture.total_animal_count();

    let added = add_these.clone();
    random_owner.animals().edit(move |list| list.add_range(added));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for additional add
    let animal_items = fixture.animal_results.data().items();
    for animal in &add_these {
        assert!(animal_items.contains(animal));
    }
    assert_eq!(fixture.total_animal_count(), initial_count + ADD_RANGE_SIZE);
    fixture.check_result_contents();
}

#[test]
fn result_contains_children_added_with_add() {
    let fixture = Fixture::new();
    let random_owner = fixture.randomizer.list_item(&fixture.owners());
    let add_this = fixture.animal_faker.generate_one();
    let initial_count = fixture.total_animal_count();

    let added = add_this.clone();
    random_owner.animals().edit(move |list| list.add(added));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    assert!(fixture.animal_results.data().items().contains(&add_this));
    assert_eq!(fixture.total_animal_count(), initial_count + 1);
    fixture.check_result_contents();
}

#[test]
fn result_contains_children_added_with_insert() {
    let fixture = Fixture::new();
    let random_owner = fixture.randomizer.list_item(&fixture.owners());
    let insert_index = fixture.randomizer.number(random_owner.animals().items().len() as i32) as usize;
    let insert_this = fixture.animal_faker.generate_one();
    let initial_count = fixture.total_animal_count();

    let inserted = insert_this.clone();
    random_owner.animals().edit(move |list| list.insert(insert_index, inserted));

    assert_eq!(random_owner.animals().items()[insert_index], insert_this);
    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for additional add
    assert!(fixture.animal_results.data().items().contains(&insert_this));
    assert_eq!(fixture.total_animal_count(), initial_count + 1);
    fixture.check_result_contents();
}

#[test]
fn result_does_not_contain_children_removed_with_remove() {
    let fixture = Fixture::new();
    let random_owner = fixture.randomizer.list_item(&fixture.owners());
    let remove_this = fixture.randomizer.list_item(&random_owner.animals().items());
    let initial_count = fixture.total_animal_count();

    let removed = remove_this.clone();
    random_owner.animals().edit(move |list| {
        list.remove(&removed);
    });

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    assert!(!fixture.animal_results.data().items().contains(&remove_this));
    assert_eq!(fixture.total_animal_count(), initial_count - 1);
    fixture.check_result_contents();
}

#[test]
fn result_does_not_contain_children_removed_with_remove_at() {
    let fixture = Fixture::new();
    let random_owner = fixture.randomizer.list_item(&fixture.owners());
    let remove_index = fixture.randomizer.number(random_owner.animals().count() as i32 - 1) as usize;
    let remove_this = random_owner.animals().items()[remove_index].clone();
    let initial_count = fixture.total_animal_count();

    random_owner.animals().edit(move |list| list.remove_at(remove_index));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    assert!(!fixture.animal_results.data().items().contains(&remove_this));
    assert_eq!(fixture.total_animal_count(), initial_count - 1);
    fixture.check_result_contents();
}

#[test]
fn result_does_not_contain_children_removed_with_remove_range() {
    let fixture = Fixture::new();
    let random_owner = fixture.randomizer.list_item(&fixture.owners());
    let animal_count = random_owner.animals().count() as i32;
    let remove_count = fixture.randomizer.number_range(1, animal_count - 1);
    let remove_index = fixture.randomizer.number(animal_count - remove_count - 1);
    let (remove_index, remove_count) = (remove_index as usize, remove_count as usize);
    let remove_these: Vec<Rc<Animal>> = random_owner.animals().items().into_iter().skip(remove_index).take(remove_count).collect();

    random_owner.animals().edit(move |list| list.remove_range(remove_index, remove_count));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    let owner_animals = random_owner.animals().items();
    for removed in &remove_these {
        assert!(!owner_animals.contains(removed));
    }
    fixture.check_result_contents();
}

#[test]
fn result_does_not_contain_children_removed_with_remove_many() {
    let fixture = Fixture::new();
    let random_owner = fixture.randomizer.list_item(&fixture.owners());
    let remove_count = fixture.randomizer.number_range(1, random_owner.animals().count() as i32 - 1) as usize;
    let remove_these = fixture.randomizer.list_items(&random_owner.animals().items(), remove_count);

    let removed = remove_these.clone();
    random_owner.animals().edit(move |list| remove_many(list, &removed));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    let owner_animals = random_owner.animals().items();
    for removed in &remove_these {
        assert!(!owner_animals.contains(removed));
    }
    fixture.check_result_contents();
}

#[test]
fn result_contains_correct_items_after_child_replacement() {
    let fixture = Fixture::new();
    let random_owner = fixture.randomizer.list_item(&fixture.owners());
    let replace_this = fixture.randomizer.list_item(&random_owner.animals().items());
    let with_this = fixture.animal_faker.generate_one();

    let (original, replacement) = (replace_this.clone(), with_this.clone());
    random_owner.animals().edit(move |list| replace(list, &original, replacement));

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for update
    let owner_animals = random_owner.animals().items();
    assert!(!owner_animals.contains(&replace_this));
    assert!(owner_animals.contains(&with_this));
    fixture.check_result_contents();
}

#[test]
fn result_contains_correct_items_after_child_clear() {
    let fixture = Fixture::new();
    let random_owner = fixture.randomizer.list_item(&fixture.owners());
    let removed_animals = random_owner.animals().items();

    random_owner.animals().edit(|list| list.clear());

    assert_eq!(fixture.animal_owner_results.data().count(), INITIAL_OWNER_COUNT);
    assert_eq!(fixture.animal_results.messages().len(), 2); // 1 for initial add, 1 for removing
    assert_eq!(random_owner.animals().count(), 0);
    let animal_items = fixture.animal_results.data().items();
    for removed in &removed_animals {
        assert!(!animal_items.contains(removed));
    }
    fixture.check_result_contents();
}

/// `ResultCompletesOnlyWhenSourceAndAllChildrenComplete(bool completeSource, bool completeChildren)`.
fn result_completes_only_when_source_and_all_children_complete(complete_source: bool, complete_children: bool) {
    let fixture = Fixture::new();

    for owner in fixture.owners().iter().skip(if complete_children { 0 } else { 1 }) {
        owner.dispose();
    }
    if complete_source {
        fixture.animal_owners.dispose();
    }

    assert_eq!(fixture.animal_owner_results.is_completed(), complete_source);
    assert_eq!(fixture.animal_results.is_completed(), complete_source && complete_children);
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
fn result_fails_if_source_fails() {
    let fixture = Fixture::new();
    let expected_error = new_error("Expected");
    let throw_observable = Observable::throw::<ChangeSet<Rc<AnimalOwner>>>(expected_error.clone());
    let results = fixture
        .animal_owners
        .connect()
        .concat(throw_observable)
        .merge_many_change_sets(|owner: &Rc<AnimalOwner>| owner.animals().connect())
        .as_aggregator();

    fixture.animal_owners.dispose();

    let error = results.error().expect("the merge should have failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}
