//! `GroupOnObservableFixture` (`DynamicData.Tests/Cache/GroupOnObservableFixture.cs`).
//!
//! The counts are those of upstream's release build.

use crate::domain::{fakers, Color, Faker, Person, Randomizer};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::NotifyPropertyChangedEx;
use ferroui_dynamic_data::cache::testing::{ChangeSetAggregator, GroupChangeSetAggregator, GroupChangeSetAggregatorEx};
use ferroui_dynamic_data::cache::{ChangeSet, Group, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{prop, IDisposable, IObservable, IObserver, ObservableError};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

const INITIAL_COUNT: usize = 103;
const ADD_COUNT: usize = 53;
const REMOVE_COUNT: usize = 37;
const UPDATE_COUNT: usize = 101;

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
    group_results: GroupChangeSetAggregator<Rc<Person>, String, Color>,
    grouper_shutdown: Rc<Subject<()>>,
    faker: Faker<Rc<Person>>,
    randomizer: Randomizer,
}

/// `CreateFavoriteColorObservable`: the favorite color of the person, now and whenever it changes,
/// until the grouping shuts down.
fn create_favorite_color_observable(grouper_shutdown: &Rc<Subject<()>>) -> impl Fn(&Rc<Person>, &String) -> Rc<dyn IObservable<Color>> + Clone + 'static {
    let shutdown = grouper_shutdown.clone();
    move |person: &Rc<Person>, _key: &String| person.when_value_changed(prop!(Person => favorite_color)).take_until(shutdown.as_observable())
}

impl Fixture {
    fn new() -> Self {
        let randomizer = Randomizer::new(0x3141_5926);
        let faker = fakers::person().with_seed(&randomizer);
        let grouper_shutdown = Subject::new();
        let cache = SourceCache::new(|p: &Rc<Person>| p.unique_key());
        let results = cache.connect().as_aggregator();
        let group_results = cache.connect().group_on_observable_with_key(create_favorite_color_observable(&grouper_shutdown)).as_group_aggregator();
        Self { cache, results, group_results, grouper_shutdown, faker, randomizer }
    }

    fn add_or_update(&self, people: Vec<Rc<Person>>) {
        self.cache.edit(|updater| updater.add_or_update_items(&mut people.into_iter()));
    }

    fn used_colors(&self) -> Vec<Color> {
        let mut colors: Vec<Color> = Vec::new();
        for person in self.cache.items() {
            if !colors.contains(&person.favorite_color()) {
                colors.push(person.favorite_color());
            }
        }
        colors
    }

    /// Removes the people whose favorite color is `color`.
    fn remove_color(&self, color: Color) {
        self.cache.edit(|updater| {
            let keys: Vec<String> = updater.items().iter().filter(|p| p.favorite_color() == color).map(|p| p.unique_key()).collect();
            updater.remove_keys(&mut keys.into_iter());
        });
    }

    /// `RandomFavoriteColorChange`.
    fn random_favorite_color_change(&self) {
        let person = self.randomizer.list_item(&self.cache.items());
        // Pick a new favorite color.
        person.set_favorite_color(self.randomizer.random_color_except(person.favorite_color()));
    }

    /// `VerifyGroupingResults`.
    fn verify_grouping_results(&self) {
        verify_grouping_results(&self.cache, &self.results, &self.group_results);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.group_results.dispose();
        self.results.dispose();
        self.cache.dispose();
        self.grouper_shutdown.dispose();
    }
}

fn verify_grouping_results(
    cache: &SourceCache<Rc<Person>, String>,
    cache_results: &ChangeSetAggregator<Rc<Person>, String>,
    group_results: &GroupChangeSetAggregator<Rc<Person>, String, Color>,
) {
    let expected_persons = cache.items();
    let mut expected_groupings: HashMap<Color, Vec<Rc<Person>>> = HashMap::new();
    for person in &expected_persons {
        expected_groupings.entry(person.favorite_color()).or_default().push(person.clone());
    }

    // These sets should be equivalent.
    assert_equivalent(&expected_persons, &cache_results.data().items());
    let expected_keys: Vec<Color> = expected_groupings.keys().copied().collect();
    assert_equivalent(&group_results.groups().keys(), &expected_keys);

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
fn result_contains_all_initial_children() {
    let fixture = Fixture::new();

    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));

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
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));

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
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));

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
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));
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
fn group_removed_when_empty() {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));
    let used_color_list = fixture.used_colors();
    let remove_color = fixture.randomizer.list_item(&used_color_list);
    let color_count = used_color_list.len();

    fixture.remove_color(remove_color);

    assert_eq!(fixture.cache.items().iter().map(|p| p.favorite_color()).collect::<HashSet<_>>().len(), color_count - 1);
    assert_eq!(fixture.results.messages().len(), 2, "1 for Adds and 1 for Removes");
    assert_eq!(fixture.group_results.data().count(), color_count - 1, "{color_count} colors were used and then all of the {remove_color:?} were removed");
    assert_eq!(fixture.group_results.messages().len(), 2, "1 for Adds and 1 for Removes");
    assert_eq!(fixture.group_results.summary().overall.adds, color_count);
    assert_eq!(fixture.group_results.summary().overall.removes, 1);
    fixture.verify_grouping_results();
}

#[test]
fn group_not_removed_if_added_back_immediately() {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));
    let used_color_list = fixture.used_colors();
    let remove_color = fixture.randomizer.list_item(&used_color_list);
    let color_count = used_color_list.len();

    fixture.cache.edit(|updater| {
        let keys: Vec<String> = updater.items().iter().filter(|p| p.favorite_color() == remove_color).map(|p| p.unique_key()).collect();
        updater.remove_keys(&mut keys.into_iter());
        let new_person = fixture.faker.generate_one();
        new_person.set_favorite_color(remove_color);
        updater.add_or_update(new_person);
    });

    assert_eq!(fixture.cache.items().iter().map(|p| p.favorite_color()).collect::<HashSet<_>>().len(), color_count);
    assert_eq!(fixture.results.messages().len(), 2, "1 for Adds and 1 for Other Added Value");
    assert_eq!(fixture.group_results.data().count(), color_count);
    assert_eq!(fixture.group_results.messages().len(), 1, "Shouldn't be removed/re-added");
    assert_eq!(fixture.group_results.summary().overall.adds, color_count);
    assert_eq!(fixture.group_results.summary().overall.removes, 0);
    let group = fixture.group_results.groups().lookup(&remove_color).expect("the group is there");
    assert_eq!(group.data().count(), 1, "All the {remove_color:?} were removed and then 1 was added back");
    fixture.verify_grouping_results();
}

#[test]
fn grouping_sequence_completes_when_empty() {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));
    let used_color_list = fixture.used_colors();
    let remove_color = fixture.randomizer.list_item(&used_color_list);

    let results = fixture
        .cache
        .connect()
        .group_on_observable_with_key(create_favorite_color_observable(&fixture.grouper_shutdown))
        .filter(move |grp: &Group<Rc<Person>, String, Color>| *grp.key() == remove_color)
        .take(1)
        .merge_many(|grp: &Group<Rc<Person>, String, Color>| grp.cache().connect())
        .as_aggregator();

    fixture.remove_color(remove_color);

    assert!(results.is_completed());
    fixture.verify_grouping_results();
}

#[test]
fn all_sequences_should_complete_when_source_and_grouping_observables_complete() {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));

    let results = fixture
        .cache
        .connect()
        .group_on_observable_with_key(create_favorite_color_observable(&fixture.grouper_shutdown))
        .merge_many(|grp: &Group<Rc<Person>, String, Color>| grp.cache().connect())
        .as_aggregator();

    fixture.cache.dispose();
    fixture.grouper_shutdown.on_next(());

    assert!(results.is_completed());
    fixture.verify_grouping_results();
}

#[test]
fn all_groups_removed_when_cleared() {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));
    let color_count = fixture.used_colors().len();

    fixture.cache.edit(|updater| updater.clear());

    assert_eq!(fixture.cache.items().len(), 0);
    assert_eq!(fixture.results.messages().len(), 2, "1 for Adds and 1 for Removes");
    assert_eq!(fixture.group_results.summary().overall.adds, color_count);
    assert_eq!(fixture.group_results.summary().overall.removes, color_count);
    fixture.verify_grouping_results();
}

#[test]
fn results_contains_correct_regrouped_values() {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));

    for _ in 0..UPDATE_COUNT {
        fixture.random_favorite_color_change();
    }

    fixture.verify_grouping_results();
}

// Not ported: ResultsContainsCorrectRegroupedValuesAsync - real threads

/// `ResultCompletesOnlyWhenSourceAndAllGroupingObservablesComplete`, one row of its data.
fn result_completes_only_when_source_and_all_grouping_observables_complete(complete_source: bool, complete_groups: bool) {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));

    if complete_source {
        fixture.cache.dispose();
    }
    if complete_groups {
        fixture.grouper_shutdown.on_next(());
    }

    assert_eq!(fixture.results.is_completed(), complete_source);
    assert_eq!(fixture.group_results.is_completed(), complete_groups && complete_source);
}

#[test]
fn result_completes_only_when_source_and_all_grouping_observables_complete_case1() {
    result_completes_only_when_source_and_all_grouping_observables_complete(false, false);
}

#[test]
fn result_completes_only_when_source_and_all_grouping_observables_complete_case2() {
    result_completes_only_when_source_and_all_grouping_observables_complete(true, false);
}

#[test]
fn result_completes_only_when_source_and_all_grouping_observables_complete_case3() {
    result_completes_only_when_source_and_all_grouping_observables_complete(false, true);
}

#[test]
fn result_completes_only_when_source_and_all_grouping_observables_complete_case4() {
    result_completes_only_when_source_and_all_grouping_observables_complete(true, true);
}

#[test]
fn result_fails_if_source_fails() {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));
    let expected_error: ObservableError = Rc::new(ExpectedError);
    let throw_observable = Observable::throw::<ChangeSet<Rc<Person>, String>>(expected_error.clone());
    let results = fixture.cache.connect().concat(throw_observable).group_on_observable_with_key(create_favorite_color_observable(&fixture.grouper_shutdown)).as_group_aggregator();

    fixture.cache.dispose();

    let error = results.error().expect("the result failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}

#[test]
fn result_fails_if_group_observable_fails() {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));
    let expected_error: ObservableError = Rc::new(ExpectedError);
    let throw_observable = Observable::throw::<Color>(expected_error.clone());

    let favorite_color = create_favorite_color_observable(&fixture.grouper_shutdown);
    let results = fixture
        .cache
        .connect()
        .group_on_observable_with_key(move |person: &Rc<Person>, key: &String| favorite_color(person, key).take(1).concat(throw_observable.clone()))
        .as_group_aggregator();

    let error = results.error().expect("the result failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}

#[test]
fn on_error_fires_if_selector_throws() {
    let fixture = Fixture::new();
    fixture.add_or_update(fixture.faker.generate(INITIAL_COUNT));
    let expected_error: ObservableError = Rc::new(ExpectedError);

    let failure = expected_error.clone();
    let results = fixture.cache.connect().try_group_on_observable(move |_: &Rc<Person>| Err::<Rc<dyn IObservable<Color>>, _>(failure.clone())).as_group_aggregator();

    let error = results.error().expect("the result failed");
    assert!(Rc::ptr_eq(&error, &expected_error));
}
