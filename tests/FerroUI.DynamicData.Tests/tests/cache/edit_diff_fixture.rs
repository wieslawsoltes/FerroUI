//! `EditDiffFixture` (`DynamicData.Tests/Cache/EditDiffFixture.cs`): `EditDiff` of a source cache.
//!
//! Upstream's two overloads, with a function of equality and with an `IEqualityComparer`, are one
//! method here (`edit_diff(items, are_items_equal)`); the `_with_equality_comparer` tests pass
//! `Person::age_comparer()` itself, the others a closure that calls it, as upstream does.

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use std::rc::Rc;

/// The fixture: a cache of ten people, `Name1` to `Name10` aged 1 to 10, and the aggregator of
/// its changes (connected before the people are added).
struct Fixture {
    cache: SourceCache<Rc<Person>, String>,
    result: ChangeSetAggregator<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let cache = SourceCache::new(|p: &Rc<Person>| p.name());
        let result = cache.connect().as_aggregator();
        cache.add_or_update_items((1..=10).map(|i| Person::new(format!("Name{i}"), i)).collect::<Vec<_>>());
        Self { cache, result }
    }
}

/// The people `Name{i}` of the range, aged `i + age_offset`.
fn people(range: impl Iterator<Item = i32>, age_offset: i32) -> Vec<Rc<Person>> {
    range.map(|i| Person::new(format!("Name{i}"), i + age_offset)).collect()
}

#[test]
fn amends() {
    let fixture = Fixture::new();
    // `Enumerable.Range(5, 3)`.
    let new_list = people(5..8, 10);
    fixture.cache.edit_diff(new_list.clone(), |current, previous| (Person::age_comparer())(current, previous));

    assert_eq!(fixture.cache.count(), 3);

    let messages = fixture.result.messages();
    let last_change = messages.last().expect("a change set");
    assert_eq!(last_change.adds(), 0);
    assert_eq!(last_change.updates(), 3);
    assert_eq!(last_change.removes(), 7);

    assert_equivalent(&fixture.cache.items(), &new_list);
}

#[test]
fn amends_with_equality_comparer() {
    let fixture = Fixture::new();
    let new_list = people(5..8, 10);
    fixture.cache.edit_diff(new_list.clone(), Person::age_comparer());

    assert_eq!(fixture.cache.count(), 3);

    let messages = fixture.result.messages();
    let last_change = messages.last().expect("a change set");
    assert_eq!(last_change.adds(), 0);
    assert_eq!(last_change.updates(), 3);
    assert_eq!(last_change.removes(), 7);

    assert_equivalent(&fixture.cache.items(), &new_list);
}

#[test]
fn edit_with_same_data() {
    let fixture = Fixture::new();
    let new_people = people(1..11, 0);

    fixture.cache.edit_diff(new_people.clone(), |current, previous| (Person::age_comparer())(current, previous));

    assert_eq!(fixture.cache.count(), 10);
    assert_equivalent(&fixture.cache.items(), &new_people);
    assert_eq!(fixture.result.messages().len(), 1);
}

#[test]
fn edit_with_same_data_with_equality_comparer() {
    let fixture = Fixture::new();
    let new_people = people(1..11, 0);

    fixture.cache.edit_diff(new_people.clone(), Person::age_comparer());

    assert_eq!(fixture.cache.count(), 10);
    assert_equivalent(&fixture.cache.items(), &new_people);
    let messages = fixture.result.messages();
    let _last_change = messages.last().expect("a change set");
    assert_eq!(messages.len(), 1);
}

#[test]
fn new() {
    let fixture = Fixture::new();
    let new_people = people(1..16, 0);

    fixture.cache.edit_diff(new_people.clone(), |current, previous| (Person::age_comparer())(current, previous));

    assert_eq!(fixture.cache.count(), 15);
    assert_equivalent(&fixture.cache.items(), &new_people);
    let messages = fixture.result.messages();
    let last_change = messages.last().expect("a change set");
    assert_eq!(last_change.adds(), 5);
}

#[test]
fn new_with_equality_comparer() {
    let fixture = Fixture::new();
    let new_people = people(1..16, 0);

    fixture.cache.edit_diff(new_people.clone(), Person::age_comparer());

    assert_eq!(fixture.cache.count(), 15);
    assert_equivalent(&fixture.cache.items(), &new_people);
    let messages = fixture.result.messages();
    let last_change = messages.last().expect("a change set");
    assert_eq!(last_change.adds(), 5);
}

#[test]
fn removes() {
    let fixture = Fixture::new();
    let new_list = people(1..8, 0);
    fixture.cache.edit_diff(new_list.clone(), |current, previous| (Person::age_comparer())(current, previous));

    assert_eq!(fixture.cache.count(), 7);

    let messages = fixture.result.messages();
    let last_change = messages.last().expect("a change set");
    assert_eq!(last_change.adds(), 0);
    assert_eq!(last_change.updates(), 0);
    assert_eq!(last_change.removes(), 3);

    assert_equivalent(&fixture.cache.items(), &new_list);
}

#[test]
fn removes_with_equality_comparer() {
    let fixture = Fixture::new();
    let new_list = people(1..8, 0);
    fixture.cache.edit_diff(new_list.clone(), Person::age_comparer());

    assert_eq!(fixture.cache.count(), 7);

    let messages = fixture.result.messages();
    let last_change = messages.last().expect("a change set");
    assert_eq!(last_change.adds(), 0);
    assert_eq!(last_change.updates(), 0);
    assert_eq!(last_change.removes(), 3);

    assert_equivalent(&fixture.cache.items(), &new_list);
}

#[test]
fn various_changes() {
    let fixture = Fixture::new();
    // `Enumerable.Range(6, 10)`.
    let new_list = people(6..16, 10);

    fixture.cache.edit_diff(new_list.clone(), |current, previous| (Person::age_comparer())(current, previous));

    assert_eq!(fixture.cache.count(), 10);

    let messages = fixture.result.messages();
    let last_change = messages.last().expect("a change set");
    assert_eq!(last_change.adds(), 5);
    assert_eq!(last_change.updates(), 5);
    assert_eq!(last_change.removes(), 5);

    assert_equivalent(&fixture.cache.items(), &new_list);
}

#[test]
fn various_changes_with_equality_comparer() {
    let fixture = Fixture::new();
    let new_list = people(6..16, 10);

    fixture.cache.edit_diff(new_list.clone(), Person::age_comparer());

    assert_eq!(fixture.cache.count(), 10);

    let messages = fixture.result.messages();
    let last_change = messages.last().expect("a change set");
    assert_eq!(last_change.adds(), 5);
    assert_eq!(last_change.updates(), 5);
    assert_eq!(last_change.removes(), 5);

    assert_equivalent(&fixture.cache.items(), &new_list);
}
