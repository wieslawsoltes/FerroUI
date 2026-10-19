//! `EditDiffFixture` (`DynamicData.Tests/List/EditDiffFixture.cs`).

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    cache: SourceList<Rc<Person>>,
    result: ChangeSetAggregator<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let cache = SourceList::<Rc<Person>>::new();
        let result = cache.connect().as_aggregator();
        cache.add_range((1..=10).map(|i| Person::new(format!("Name{i}"), i)).collect::<Vec<_>>());
        Self { cache, result }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.cache.dispose();
        self.result.dispose();
    }
}

/// The people as their members, for `BeEquivalentTo`, which compares the members of objects
/// (`Person` is equal to a person of its name).
fn members(people: &[Rc<Person>]) -> Vec<(String, i32, String)> {
    people.iter().map(|p| (p.name(), p.age(), p.gender())).collect()
}

#[test]
fn amends() {
    let fixture = Fixture::new();
    let new_list: Vec<Rc<Person>> = (5..5 + 3).map(|i| Person::new(format!("Name{i}"), i + 10)).collect();
    fixture.cache.edit_diff_with_comparer(new_list.clone(), Person::name_age_gender_comparer());

    assert_eq!(fixture.cache.count(), 3);

    let last_change = fixture.result.messages().last().cloned().expect("a change set");
    assert_eq!(last_change.adds(), 3);
    assert_eq!(last_change.removes(), 10);

    assert_equivalent(&members(&fixture.cache.items()), &members(&new_list));
}

#[test]
fn edit_with_same_data() {
    let fixture = Fixture::new();
    let new_people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new(format!("Name{i}"), i)).collect();

    fixture.cache.edit_diff_with_comparer(new_people.clone(), Person::name_age_gender_comparer());

    assert_eq!(fixture.cache.count(), 10);
    assert_equivalent(&members(&fixture.cache.items()), &members(&new_people));
    assert_eq!(fixture.result.messages().len(), 1);
}

#[test]
fn new() {
    let fixture = Fixture::new();
    let new_people: Vec<Rc<Person>> = (1..=15).map(|i| Person::new(format!("Name{i}"), i)).collect();

    fixture.cache.edit_diff_with_comparer(new_people.clone(), Person::name_age_gender_comparer());

    assert_eq!(fixture.cache.count(), 15);
    assert_equivalent(&members(&fixture.cache.items()), &members(&new_people));
    let last_change = fixture.result.messages().last().cloned().expect("a change set");
    assert_eq!(last_change.adds(), 5);
}

#[test]
fn removes() {
    let fixture = Fixture::new();
    let new_list: Vec<Rc<Person>> = (1..=7).map(|i| Person::new(format!("Name{i}"), i)).collect();
    fixture.cache.edit_diff_with_comparer(new_list.clone(), Person::name_age_gender_comparer());

    assert_eq!(fixture.cache.count(), 7);

    let last_change = fixture.result.messages().last().cloned().expect("a change set");
    assert_eq!(last_change.adds(), 0);
    assert_eq!(last_change.removes(), 3);

    assert_equivalent(&members(&fixture.cache.items()), &members(&new_list));
}

#[test]
fn various_changes() {
    let fixture = Fixture::new();
    let new_list: Vec<Rc<Person>> = (6..6 + 10).map(|i| Person::new(format!("Name{i}"), i)).collect();

    fixture.cache.edit_diff_with_comparer(new_list.clone(), Person::name_age_gender_comparer());

    assert_eq!(fixture.cache.count(), 10);

    let last_change = fixture.result.messages().last().cloned().expect("a change set");
    assert_eq!(last_change.adds(), 5);
    assert_eq!(last_change.removes(), 5);

    assert_equivalent(&members(&fixture.cache.items()), &members(&new_list));
}
