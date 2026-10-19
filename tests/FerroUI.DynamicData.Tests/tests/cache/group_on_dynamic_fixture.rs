//! `GroupOnDynamicFixture` (`DynamicData.Tests/Cache/GroupOnDynamicFixture.cs`).
//!
//! The counts are those of upstream's release build.

use crate::domain::{fakers, AnimalFamily, Faker, Person, Randomizer};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::{ChangeSetAggregator, GroupChangeSetAggregator, GroupChangeSetAggregatorEx};
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

// Not ported: MultiThreadedStressTest - real threads

const INITIAL_COUNT: usize = 103;
const ADD_COUNT: usize = 53;
const REMOVE_COUNT: usize = 37;
const UPDATE_COUNT: usize = 101;

/// The selector of the group key of a person and its key (`Func<Person, string, string>`).
type KeySelector = Rc<dyn Fn(&Rc<Person>, &String) -> String>;

#[derive(Debug)]
struct ExpectedError;

impl fmt::Display for ExpectedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Expected")
    }
}

impl std::error::Error for ExpectedError {}

struct Fixture {
    cache: SourceCache<Rc<Person>, String>,
    results: ChangeSetAggregator<Rc<Person>, String>,
    group_results: GroupChangeSetAggregator<Rc<Person>, String, String>,
    faker: Faker<Rc<Person>>,
    randomizer: Randomizer,
    key_selection_subject: Rc<BehaviorSubject<Option<KeySelector>>>,
    regroup_subject: Rc<Subject<()>>,
    // The selectors are made once, as upstream's static methods compare equal as delegates.
    fav_color: KeySelector,
    parent_name: KeySelector,
    pet_type: KeySelector,
}

impl Fixture {
    fn new() -> Self {
        let randomizer = Randomizer::new(0xc001_d00du32 as i32);
        let faker = fakers::person().with_seed(&randomizer);
        let cache = SourceCache::new(|p: &Rc<Person>| p.unique_key());
        let key_selection_subject: Rc<BehaviorSubject<Option<KeySelector>>> = BehaviorSubject::new(None);
        let regroup_subject = Subject::new();
        let results = cache.connect().as_aggregator();
        let group_results = cache.connect().group_dynamic_with_key_with(key_selection_observable(&key_selection_subject), regroup_subject.as_observable()).as_group_aggregator();
        Self {
            cache,
            results,
            group_results,
            faker,
            randomizer,
            key_selection_subject,
            regroup_subject,
            fav_color: Rc::new(|person: &Rc<Person>, _: &String| format!("{:?}", person.favorite_color())),
            parent_name: Rc::new(|person: &Rc<Person>, _: &String| person.parent_name()),
            pet_type: Rc::new(|person: &Rc<Person>, _: &String| format!("{:?}", person.pet_type())),
        }
    }

    fn add_or_update(&self, people: Vec<Rc<Person>>) {
        self.cache.edit(|updater| updater.add_or_update_items(&mut people.into_iter()));
    }

    /// `InitialPopulate`.
    fn initial_populate(&self) {
        self.add_or_update(self.faker.generate(INITIAL_COUNT));
    }

    /// `ForceRegroup`.
    fn force_regroup(&self) {
        self.regroup_subject.on_next(());
    }

    /// `GroupByFavColor`.
    fn group_by_fav_color(&self) {
        self.key_selection_subject.on_next(Some(self.fav_color.clone()));
    }

    /// `GroupByParentName`.
    #[allow(dead_code)]
    fn group_by_parent_name(&self) {
        self.key_selection_subject.on_next(Some(self.parent_name.clone()));
    }

    /// `GroupByPetType`.
    fn group_by_pet_type(&self) {
        self.key_selection_subject.on_next(Some(self.pet_type.clone()));
    }

    /// `VerifyGroupingResults`.
    fn verify_grouping_results(&self) {
        let group_key_selector = self.key_selection_subject.value().ok().flatten();
        verify_grouping_results(&self.cache, &self.results, &self.group_results, group_key_selector);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.group_results.dispose();
        self.results.dispose();
        self.cache.dispose();
        self.key_selection_subject.dispose();
        self.regroup_subject.dispose();
    }
}

/// `KeySelectionObservable`: the selectors that are set.
fn key_selection_observable(subject: &Rc<BehaviorSubject<Option<KeySelector>>>) -> Rc<dyn IObservable<KeySelector>> {
    subject.as_observable().where_(|selector: &Option<KeySelector>| selector.is_some()).select(|selector: Option<KeySelector>| selector.expect("filtered"))
}

fn verify_grouping_results(
    cache: &SourceCache<Rc<Person>, String>,
    cache_results: &ChangeSetAggregator<Rc<Person>, String>,
    group_results: &GroupChangeSetAggregator<Rc<Person>, String, String>,
    group_key_selector: Option<KeySelector>,
) {
    let Some(group_key_selector) = group_key_selector else {
        assert_eq!(group_results.data().count(), 0);
        assert_eq!(group_results.groups().count(), 0);
        return;
    };

    let expected_items = cache.items();
    let mut expected_groupings: HashMap<String, Vec<Rc<Person>>> = HashMap::new();
    for person in &expected_items {
        expected_groupings.entry(group_key_selector(person, &String::new())).or_default().push(person.clone());
    }

    // These sets should be equivalent.
    assert_equivalent(&expected_items, &cache_results.data().items());
    let expected_keys: Vec<String> = expected_groupings.keys().cloned().collect();
    assert_equivalent(&expected_keys, &group_results.groups().keys());

    // Check each group.
    for (key, grouping) in &expected_groupings {
        let group = group_results.groups().lookup(key).expect("the group is there");
        assert_equivalent(grouping, &group.data().items());
    }

    // No group should be empty.
    for group in group_results.groups().items() {
        assert!(group.data().count() > 0, "Empty groups should be removed");
    }
}

#[test]
fn result_empty_if_selection_key_does_not_fire() {
    let fixture = Fixture::new();

    fixture.initial_populate();

    assert_eq!(fixture.results.data().count(), INITIAL_COUNT);
    assert_eq!(fixture.results.messages().len(), 1, "The child observables fire on subscription so everything should appear as a single changeset");
    assert_eq!(fixture.group_results.messages().len(), 0);
    fixture.verify_grouping_results();
}

#[test]
fn result_contains_all_initial_children() {
    let fixture = Fixture::new();
    fixture.initial_populate();

    fixture.group_by_fav_color();

    assert_eq!(fixture.results.data().count(), INITIAL_COUNT);
    assert_eq!(fixture.results.messages().len(), 1, "The child observables fire on subscription so everything should appear as a single changeset");
    for group in fixture.group_results.groups().items() {
        assert_eq!(group.messages().len(), 1);
    }
    fixture.verify_grouping_results();
}

#[test]
fn result_contains_all_added_children() {
    let fixture = Fixture::new();
    fixture.group_by_fav_color();

    fixture.initial_populate();

    assert_eq!(fixture.results.data().count(), INITIAL_COUNT);
    assert_eq!(fixture.results.messages().len(), 1, "The child observables fire on subscription so everything should appear as a single changeset");
    for group in fixture.group_results.groups().items() {
        assert_eq!(group.messages().len(), 1);
    }
    fixture.verify_grouping_results();
}

#[test]
fn result_contains_added_values() {
    let fixture = Fixture::new();
    fixture.initial_populate();
    fixture.group_by_pet_type();

    fixture.add_or_update(fixture.faker.generate(ADD_COUNT));

    assert_eq!(fixture.results.data().count(), INITIAL_COUNT + ADD_COUNT);
    assert_eq!(fixture.results.messages().len(), 2, "Initial Adds and then the subsequent Additions should each be a single message");
    for group in fixture.group_results.groups().items() {
        assert!(group.messages().len() <= 2);
    }
    fixture.verify_grouping_results();
}

#[test]
fn result_does_not_contain_removed_values() {
    let fixture = Fixture::new();
    fixture.initial_populate();
    fixture.group_by_pet_type();

    let removed: Vec<String> = fixture.randomizer.list_items(&fixture.cache.items(), REMOVE_COUNT).iter().map(|p| p.unique_key()).collect();
    fixture.cache.edit(|updater| updater.remove_keys(&mut removed.into_iter()));

    assert_eq!(fixture.results.data().count(), INITIAL_COUNT - REMOVE_COUNT);
    assert_eq!(fixture.results.messages().len(), 2, "1 for Adds and 1 for Removes");
    for group in fixture.group_results.groups().items() {
        assert!(group.messages().len() <= 2);
    }
    fixture.verify_grouping_results();
}

#[test]
fn result_contains_updated_values() {
    let fixture = Fixture::new();
    fixture.group_by_pet_type();
    fixture.initial_populate();
    let replacements: Vec<Rc<Person>> =
        fixture.randomizer.list_items(&fixture.cache.items(), UPDATE_COUNT).iter().map(|replace_person| Person::clone_unique_id(&fixture.faker.generate_one(), replace_person)).collect();

    fixture.add_or_update(replacements);

    assert_eq!(fixture.results.data().count(), INITIAL_COUNT, "Only replacements were made");
    assert_eq!(fixture.results.messages().len(), 2, "1 for Adds and 1 for Updates");
    for group in fixture.group_results.groups().items() {
        assert!(group.messages().len() <= 2);
    }
    fixture.verify_grouping_results();
}

#[test]
fn result_contains_refreshed_values() {
    let fixture = Fixture::new();
    fixture.group_by_pet_type();
    fixture.initial_populate();
    let refresh_list = fixture.randomizer.list_items(&fixture.cache.items(), UPDATE_COUNT);
    for person in &refresh_list {
        person.set_pet_type(fixture.randomizer.enum_value::<AnimalFamily>());
    }

    fixture.cache.edit(|updater| updater.refresh_items(&mut refresh_list.into_iter()));

    assert_eq!(fixture.results.data().count(), INITIAL_COUNT, "Only replacements were made");
    assert_eq!(fixture.results.messages().len(), 2, "1 for Adds and 1 for Updates");
    fixture.verify_grouping_results();
}

#[test]
fn result_is_correct_when_group_selector_changes() {
    let fixture = Fixture::new();
    fixture.initial_populate();
    fixture.group_by_fav_color();
    let mut used_color_list: Vec<String> = fixture.cache.items().iter().map(|p| format!("{:?}", p.favorite_color())).collect();
    used_color_list.sort();
    used_color_list.dedup();
    let mut used_pet_type_list: Vec<String> = fixture.cache.items().iter().map(|p| format!("{:?}", p.pet_type())).collect();
    used_pet_type_list.sort();
    used_pet_type_list.dedup();

    fixture.group_by_pet_type();

    assert_eq!(fixture.results.data().count(), INITIAL_COUNT);
    assert_eq!(fixture.results.messages().len(), 1, "The child observables fire on subscription so everything should appear as a single changeset");
    assert_eq!(fixture.group_results.summary().overall.adds, used_color_list.len() + used_pet_type_list.len());
    assert_eq!(fixture.group_results.summary().overall.removes, used_color_list.len());
    for group in fixture.group_results.groups().items() {
        assert!(group.messages().len() <= 2);
    }
    fixture.verify_grouping_results();
}

#[test]
fn result_is_correct_after_forced_regroup() {
    let fixture = Fixture::new();
    fixture.initial_populate();
    fixture.group_by_fav_color();
    for person in fixture.cache.items() {
        person.set_favorite_color(fixture.randomizer.random_color_except(person.favorite_color()));
    }

    fixture.force_regroup();

    assert_eq!(fixture.results.data().count(), INITIAL_COUNT);
    assert_eq!(fixture.results.messages().len(), 1, "The child observables fire on subscription so everything should appear as a single changeset");
    for group in fixture.group_results.groups().items() {
        assert!(group.messages().len() <= 2, "1 for adds and 1 for regrouping");
    }
    fixture.verify_grouping_results();
}

/// `ResultCompletesOnlyWhenAllInputsComplete`, one row of its data.
fn result_completes_only_when_all_inputs_complete(complete_source: bool, complete_key_selector: bool, complete_regrouper: bool) {
    let fixture = Fixture::new();
    fixture.initial_populate();
    fixture.group_by_fav_color();

    if complete_source {
        fixture.cache.dispose();
    }
    if complete_key_selector {
        fixture.key_selection_subject.on_completed();
    }
    if complete_regrouper {
        fixture.regroup_subject.on_completed();
    }

    assert_eq!(fixture.results.is_completed(), complete_source);
    assert_eq!(fixture.group_results.is_completed(), complete_source && complete_key_selector && complete_regrouper);
}

#[test]
fn result_completes_only_when_all_inputs_complete_case1() {
    result_completes_only_when_all_inputs_complete(false, false, false);
}

#[test]
fn result_completes_only_when_all_inputs_complete_case2() {
    result_completes_only_when_all_inputs_complete(false, false, true);
}

#[test]
fn result_completes_only_when_all_inputs_complete_case3() {
    result_completes_only_when_all_inputs_complete(false, true, false);
}

#[test]
fn result_completes_only_when_all_inputs_complete_case4() {
    result_completes_only_when_all_inputs_complete(false, true, true);
}

#[test]
fn result_completes_only_when_all_inputs_complete_case5() {
    result_completes_only_when_all_inputs_complete(true, false, false);
}

#[test]
fn result_completes_only_when_all_inputs_complete_case6() {
    result_completes_only_when_all_inputs_complete(true, false, true);
}

#[test]
fn result_completes_only_when_all_inputs_complete_case7() {
    result_completes_only_when_all_inputs_complete(true, true, false);
}

#[test]
fn result_completes_only_when_all_inputs_complete_case8() {
    result_completes_only_when_all_inputs_complete(true, true, true);
}

#[test]
fn result_fails_if_source_fails() {
    let fixture = Fixture::new();
    fixture.initial_populate();
    let expected_error: ObservableError = Rc::new(ExpectedError);
    let throw_observable = Observable::throw::<ChangeSet<Rc<Person>, String>>(expected_error.clone());
    let results = fixture
        .cache
        .connect()
        .concat(throw_observable)
        .group_dynamic_with_key_with(key_selection_observable(&fixture.key_selection_subject), fixture.regroup_subject.as_observable())
        .as_group_aggregator();

    fixture.cache.dispose();

    let error = results.error().expect("the result failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}

#[test]
fn result_fails_if_group_observable_fails() {
    let fixture = Fixture::new();
    fixture.initial_populate();
    let expected_error: ObservableError = Rc::new(ExpectedError);

    fixture.key_selection_subject.on_error(expected_error.clone());

    let error = fixture.group_results.error().expect("the result failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}

#[test]
fn result_fails_if_regrouper_fails() {
    let fixture = Fixture::new();
    fixture.initial_populate();
    let expected_error: ObservableError = Rc::new(ExpectedError);

    fixture.regroup_subject.on_error(expected_error.clone());

    let error = fixture.group_results.error().expect("the result failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}
