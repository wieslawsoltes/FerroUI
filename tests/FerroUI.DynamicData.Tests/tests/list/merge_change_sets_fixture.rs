//! `MergeChangeSetsFixture` (`DynamicData.Tests/List/MergeChangeSetsFixture.cs`): the change sets of
//! the animal lists of several owners, merged from a sequence of sequences and from an enumerable of
//! sequences.
//!
//! Upstream runs the edits of each owner on a task of its own (`ForOwnersAsync`, `Task.Run`); the
//! port runs them one owner after the other, which the merge sees the same way on one thread.

use crate::domain::{fakers, Animal, AnimalOwner, Faker, Randomizer};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{replace, ChangeSet, IObservableList, ListChangeSetSourcesEx, ObservableListChangeSetsEx, ObservableListEx};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::{new_error, TestScheduler};
use ferroui_system::{IDisposable, IObservable};
use std::fmt::Debug;
use std::rc::Rc;

const INITIAL_OWNER_COUNT: usize = if cfg!(debug_assertions) { 7 } else { 103 };
const ADD_RANGE_SIZE: usize = if cfg!(debug_assertions) { 5 } else { 53 };

type Animals = Rc<dyn IObservable<ChangeSet<Rc<Animal>>>>;

/// The fixture: owners with `ADD_RANGE_SIZE` animals each, disposed at the end of each test.
struct Fixture {
    animal_owners: Vec<Rc<AnimalOwner>>,
    animal_faker: Faker<Rc<Animal>>,
    randomizer: Randomizer,
}

impl Fixture {
    fn new() -> Self {
        let randomizer = Randomizer::new(0x10131948);
        let animal_faker = fakers::animal().with_seed(&randomizer);
        let animal_owner_faker = fakers::animal_owner().with_seed(&randomizer).with_initial_animals_between(&animal_faker, ADD_RANGE_SIZE, ADD_RANGE_SIZE);
        let animal_owners = animal_owner_faker.generate(INITIAL_OWNER_COUNT);
        Self { animal_owners, animal_faker, randomizer }
    }

    /// `GetEnumerableObservable`.
    fn enumerable_observable(&self) -> Vec<Animals> {
        self.animal_owners.iter().map(|owner| owner.animals().connect()).collect()
    }

    /// `GetObservableObservable`.
    fn observable_observable(&self) -> Rc<dyn IObservable<Animals>> {
        Observable::to_observable(self.enumerable_observable())
    }

    /// `ForOwnersAsync`, one owner after the other.
    fn for_owners<R>(&self, f: impl Fn(&Self, &Rc<AnimalOwner>) -> R) -> Vec<R> {
        self.animal_owners.iter().map(|owner| f(self, owner)).collect()
    }

    /// `UseAdd`.
    fn use_add(&self, owner: &Rc<AnimalOwner>) -> Rc<Animal> {
        let animal = self.animal_faker.generate_one();
        let added = animal.clone();
        owner.animals().edit(move |list| list.add(added));
        animal
    }

    /// `UseAddRange`.
    fn use_add_range(&self, owner: &Rc<AnimalOwner>) -> Vec<Rc<Animal>> {
        let animals = self.animal_faker.generate(self.randomizer.number(ADD_RANGE_SIZE as i32) as usize);
        let added = animals.clone();
        owner.animals().edit(move |list| list.add_range(added));
        animals
    }

    /// `ReplaceAnimal`: the old animal and the new one.
    fn replace_animal(&self, owner: &Rc<AnimalOwner>) -> (Rc<Animal>, Rc<Animal>) {
        let replace_this = self.randomizer.list_item(&owner.animals().items());
        let with_this = self.animal_faker.generate_one();
        let (original, replacement) = (replace_this.clone(), with_this.clone());
        owner.animals().edit(move |list| replace(list, &original, replacement));
        (replace_this, with_this)
    }

    /// `UseInsert`.
    fn use_insert(&self, owner: &Rc<AnimalOwner>) -> Rc<Animal> {
        let new_animal = self.animal_faker.generate_one();
        let index = self.randomizer.number(owner.animals().count() as i32) as usize;
        let inserted = new_animal.clone();
        owner.animals().edit(move |list| list.insert(index, inserted));
        new_animal
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for owner in &self.animal_owners {
            owner.dispose();
        }
    }
}

/// `BeSubsetOf`: every item of `subset` is in `superset`.
fn assert_subset<T: PartialEq + Debug>(subset: &[T], superset: &[T]) {
    for item in subset {
        assert!(superset.contains(item), "expected {item:?} to be in {superset:?}");
    }
}

/// `CheckResultContents`.
fn check_result_contents(expected_owners: &[Rc<AnimalOwner>], animal_results: &ChangeSetAggregator<Rc<Animal>>) {
    let expected_animals: Vec<Rc<Animal>> = expected_owners.iter().flat_map(|owner| owner.animals().items()).collect();
    let items = animal_results.data().items();

    // These should be subsets of each other, so check one subset and the size
    assert_subset(&expected_animals, &items);
    assert_eq!(items.len(), expected_animals.len());
}

/// The merge of the sequence of sequences.
fn merge_obs(obs: &Rc<dyn IObservable<Animals>>) -> Animals {
    ObservableListChangeSetsEx::merge_change_sets(obs)
}

/// The merge of the enumerable of sequences.
fn merge_enum(obs: &[Animals]) -> Animals {
    ListChangeSetSourcesEx::merge_change_sets(obs)
}

// Not ported: MultiThreadedStressTest - real threads

// Not ported: NullChecks - argument validation

#[test]
fn result_contains_all_initial_children_obs_obs() {
    let fixture = Fixture::new();
    let obs = fixture.observable_observable();

    let results = merge_obs(&obs).as_aggregator();

    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_contains_all_initial_children_enum() {
    let fixture = Fixture::new();
    let obs = fixture.enumerable_observable();

    let results = merge_enum(&obs).as_aggregator();

    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_empty_if_source_is_cleared_obs() {
    let fixture = Fixture::new();
    let obs = fixture.observable_observable();
    let results = merge_obs(&obs).as_aggregator();

    for owner in &fixture.animal_owners {
        owner.animals().edit(|list| list.clear());
    }

    assert_eq!(results.data().count(), 0);
}

#[test]
fn result_empty_if_source_is_cleared_enum() {
    let fixture = Fixture::new();
    let obs = fixture.enumerable_observable();
    let results = merge_enum(&obs).as_aggregator();

    for owner in &fixture.animal_owners {
        owner.animals().edit(|list| list.clear());
    }

    assert_eq!(results.data().count(), 0);
}

#[test]
fn result_contains_children_added_with_add_range_obs() {
    let fixture = Fixture::new();
    let obs = fixture.observable_observable();
    let results = merge_obs(&obs).as_aggregator();

    let added: Vec<Rc<Animal>> = fixture.for_owners(Fixture::use_add_range).into_iter().flatten().collect();

    assert_subset(&added, &results.data().items());
    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_contains_children_added_with_add_range_enum() {
    let fixture = Fixture::new();
    let obs = fixture.enumerable_observable();
    let results = merge_enum(&obs).as_aggregator();

    let added: Vec<Rc<Animal>> = fixture.for_owners(Fixture::use_add_range).into_iter().flatten().collect();

    assert_subset(&added, &results.data().items());
    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_contains_children_added_with_add_obs() {
    let fixture = Fixture::new();
    let obs = fixture.observable_observable();
    let results = merge_obs(&obs).as_aggregator();

    let added = fixture.for_owners(Fixture::use_add);

    assert_subset(&added, &results.data().items());
    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_contains_children_added_with_add_enum() {
    let fixture = Fixture::new();
    let obs = fixture.enumerable_observable();
    let results = merge_enum(&obs).as_aggregator();

    fixture.for_owners(|fixture, owner| {
        let animal = fixture.animal_faker.generate_one();
        owner.animals().edit(move |list| list.add(animal));
    });

    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_contains_children_added_with_insert_obs() {
    let fixture = Fixture::new();
    let obs = fixture.observable_observable();
    let results = merge_obs(&obs).as_aggregator();

    let added = fixture.for_owners(Fixture::use_insert);

    assert_subset(&added, &results.data().items());
    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_contains_children_added_with_insert_enum() {
    let fixture = Fixture::new();
    let obs = fixture.enumerable_observable();
    let results = merge_enum(&obs).as_aggregator();

    let added = fixture.for_owners(Fixture::use_insert);

    assert_subset(&added, &results.data().items());
    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_contains_correct_items_after_child_replacement_obs() {
    let fixture = Fixture::new();
    let obs = fixture.observable_observable();
    let results = merge_obs(&obs).as_aggregator();

    let replacements = fixture.for_owners(Fixture::replace_animal);

    let items = results.data().items();
    let new_items: Vec<Rc<Animal>> = replacements.iter().map(|(_, new)| new.clone()).collect();
    assert_subset(&new_items, &items);
    for (old, _) in &replacements {
        assert!(!items.contains(old), "expected {old:?} not to be in the results");
    }
    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_contains_correct_items_after_child_replacement_enum() {
    let fixture = Fixture::new();
    let obs = fixture.enumerable_observable();
    let results = merge_enum(&obs).as_aggregator();

    let replacements = fixture.for_owners(Fixture::replace_animal);

    let items = results.data().items();
    let new_items: Vec<Rc<Animal>> = replacements.iter().map(|(_, new)| new.clone()).collect();
    assert_subset(&new_items, &items);
    for (old, _) in &replacements {
        assert!(!items.contains(old), "expected {old:?} not to be in the results");
    }
    check_result_contents(&fixture.animal_owners, &results);
}

#[test]
fn result_fails_if_source_fails() {
    let fixture = Fixture::new();
    let expected_error = new_error("Expected");
    let throw_observable = Observable::throw::<Animals>(expected_error.clone());
    let obs = fixture.observable_observable();

    let results = merge_obs(&obs.concat(throw_observable)).as_aggregator();

    let error = results.error().expect("the merge should have failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}

#[test]
fn result_fails_if_any_child_change_set_fails() {
    let fixture = Fixture::new();
    let expected_error = new_error("Test exception");
    let throw_observable = Observable::throw::<ChangeSet<Rc<Animal>>>(expected_error.clone());
    let mut obs = fixture.enumerable_observable();
    obs.push(throw_observable);

    let results = merge_enum(&obs).as_aggregator();

    let error = results.error().expect("the merge should have failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}

/// `ResultCompletesOnlyWhenSourceAndAllChildrenComplete(bool completeAll)`.
fn result_completes_only_when_source_and_all_children_complete(complete_all: bool) {
    let fixture = Fixture::new();
    let obs = fixture.observable_observable();
    let results = merge_obs(&obs).as_aggregator();

    for owner in fixture.animal_owners.iter().skip(if complete_all { 0 } else { 1 }) {
        owner.animals().dispose();
    }

    assert_eq!(results.is_completed(), complete_all);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case1() {
    result_completes_only_when_source_and_all_children_complete(false);
}

#[test]
fn result_completes_only_when_source_and_all_children_complete_case2() {
    result_completes_only_when_source_and_all_children_complete(true);
}

/// `MergedObservableRespectsCompletableFlag(bool completeSource, bool completeChildren)`.
fn merged_observable_respects_completable_flag(complete_source: bool, complete_children: bool) {
    let fixture = Fixture::new();
    let obs = fixture.enumerable_observable();
    let results = ListChangeSetSourcesEx::merge_change_sets_with(obs.as_slice(), None, None, complete_source).as_aggregator();

    for owner in fixture.animal_owners.iter().skip(if complete_children { 0 } else { 1 }) {
        owner.animals().dispose();
    }

    assert_eq!(results.is_completed(), complete_source && complete_children);
}

#[test]
fn merged_observable_respects_completable_flag_case1() {
    merged_observable_respects_completable_flag(false, false);
}

#[test]
fn merged_observable_respects_completable_flag_case2() {
    merged_observable_respects_completable_flag(false, true);
}

#[test]
fn merged_observable_respects_completable_flag_case3() {
    merged_observable_respects_completable_flag(true, false);
}

#[test]
fn merged_observable_respects_completable_flag_case4() {
    merged_observable_respects_completable_flag(true, true);
}

/// `EnumObservableUsesTheScheduler(bool advance)`.
fn enum_observable_uses_the_scheduler(advance: bool) {
    let fixture = Fixture::new();
    let scheduler = TestScheduler::new();
    let obs = fixture.enumerable_observable();
    let results = ListChangeSetSourcesEx::merge_change_sets_with(obs.as_slice(), None, Some(scheduler.scheduler()), true).as_aggregator();

    if advance {
        scheduler.advance_by(INITIAL_OWNER_COUNT as i64);
    }

    if advance {
        check_result_contents(&fixture.animal_owners, &results);
    } else {
        assert_eq!(results.data().count(), 0);
        assert_eq!(results.messages().len(), 0);
    }
}

#[test]
fn enum_observable_uses_the_scheduler_case1() {
    enum_observable_uses_the_scheduler(false);
}

#[test]
fn enum_observable_uses_the_scheduler_case2() {
    enum_observable_uses_the_scheduler(true);
}
