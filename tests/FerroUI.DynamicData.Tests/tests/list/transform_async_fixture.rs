//! `TransformAsyncFixture` (`DynamicData.Tests/List/TransformAsyncFixture.cs`): the transform of a
//! list by futures.
//!
//! Upstream's factory returns `Task.FromResult(..)`, a task that has ended; here it returns a
//! future that ends at its first poll, so the transforms end inside the edit, as upstream's do
//! (docs/porting/reactive.md, section 5.8).

use crate::domain::{Person, PersonWithGender};
use crate::utilities::{assert_equivalent_strict, ListObservableExtensions};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{Change, ChangeSet, IObservableList, ListChangeReason, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::threading::tasks::{block_on, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, ObservableError};
use std::future::Future;
use std::rc::Rc;

/// `_transformFactory`: a future that ends at once (`Task.FromResult(transformed)`).
fn transform_factory(p: Rc<Person>) -> impl Future<Output = Result<Rc<PersonWithGender>, ObservableError>> {
    let gender = if p.age() % 2 == 0 { "M" } else { "F" };
    let transformed = PersonWithGender::from_person(&p, gender);
    async move { Ok(transformed) }
}

struct Fixture {
    results: ChangeSetAggregator<Rc<PersonWithGender>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();
        let results = ChangeSetAggregator::new(source.connect().transform_async(transform_factory));
        Self { results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn add() {
    let fixture = Fixture::new();
    let person = Person::new("Adult1", 50);
    fixture.source.add(person.clone());

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");

    let transformed = block_on(transform_factory(person)).expect("the transform succeeds");
    assert_eq!(fixture.results.data().items()[0], transformed, "Should be same person");
}

#[test]
fn remove() {
    let fixture = Fixture::new();
    const KEY: &str = "Adult1";
    let person = Person::new(KEY, 50);

    fixture.source.add(person.clone());
    fixture.source.remove(&person);

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 1, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 1, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn update() {
    let fixture = Fixture::new();
    const KEY: &str = "Adult1";
    let newperson = Person::new(KEY, 50);
    let updated = Person::new(KEY, 51);

    fixture.source.add(newperson);
    fixture.source.add(updated);

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 1, "Should be 1 adds");
    assert_eq!(fixture.results.messages()[0].replaced(), 0, "Should be 1 update");
}

#[test]
fn batch_of_unique_updates() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect();

    fixture.source.add_range(people.iter().cloned());

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 100, "Should return 100 adds");

    let mut transformed: Vec<Rc<PersonWithGender>> = people.iter().map(|p| block_on(transform_factory(p.clone())).expect("the transform succeeds")).collect();
    transformed.sort_by_key(|p| p.age());
    let mut items = fixture.results.data().items();
    items.sort_by_key(|p| p.age());
    // Upstream compares the results with themselves; the transformed people are what it means.
    assert_eq!(items, transformed, "Incorrect transform result");
}

#[test]
fn same_key_changes() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new("Name", i)).collect();

    fixture.source.add_range(people);

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 10, "Should return 10 adds");
    assert_eq!(fixture.results.data().count(), 10, "Should result in 10 records");
}

#[test]
fn clear() {
    let fixture = Fixture::new();
    let people: Vec<Rc<Person>> = (1..=100).map(|l| Person::new(format!("Name{l}"), l)).collect();

    fixture.source.add_range(people);
    fixture.source.clear();

    assert_eq!(fixture.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(fixture.results.messages()[0].adds(), 100, "Should be 80 addes");
    assert_eq!(fixture.results.messages()[1].removes(), 100, "Should be 80 removes");
    assert_eq!(fixture.results.data().count(), 0, "Should be nothing cached");
}

/// Upstream's test is private, so xUnit does not run it ("disabled as it was flaky",
/// https://github.com/reactivemarbles/DynamicData/pull/625); on one thread it is not flaky, and
/// it runs here.
#[test]
fn transform_on_refresh() {
    let items: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Person{i}"), 1)).collect();

    // The result should only be true when all items are set to true.
    let list = SourceList::<Rc<Person>>::new();
    let results = list.connect().auto_refresh(ferroui_system::prop!(Person => age)).transform_async_with(|p: Rc<Person>| async move { Ok::<_, ObservableError>(p) }, true).as_aggregator();
    list.add_range(items.clone());

    assert_eq!(results.data().count(), 100);
    assert_eq!(results.messages().len(), 1);

    items[0].set_age(10);
    assert_eq!(results.data().count(), 100);
    assert_eq!(results.messages().len(), 2);

    assert_eq!(results.messages()[1][0].reason(), ListChangeReason::Replace);

    // Remove an item and check no change is fired.
    let to_remove = items[1].clone();
    list.remove(&to_remove);
    assert_eq!(results.data().count(), 99);
    assert_eq!(results.messages().len(), 3);
    to_remove.set_age(100);
    assert_eq!(results.messages().len(), 3);

    // Add it back in and check it updates.
    list.add(to_remove.clone());
    assert_eq!(results.messages().len(), 4);
    to_remove.set_age(101);
    assert_eq!(results.messages().len(), 5);

    let messages = results.messages();
    assert_eq!(messages.last().expect("there are messages")[0].reason(), ListChangeReason::Replace);

    results.dispose();
    list.dispose();
}

#[test]
fn transform_async_cancels_token_on_un_subscribe() {
    let source = SourceList::<Rc<Person>>::new();
    let tcs = TaskCompletionSource::<Rc<Person>>::new();
    let factory_tcs = tcs.clone();
    let sub = source
        .connect()
        .transform_async_with_token(
            move |_c: Rc<Person>, _p: Option<Rc<Person>>, _count: usize, cancel: CancellationToken| {
                let tcs = factory_tcs.clone();
                async move {
                    let canceled = tcs.clone();
                    let registration = cancel.register_local(move || canceled.set_canceled());
                    let result = tcs.task().await;
                    registration.dispose();
                    result
                }
            },
            false,
        )
        .subscribe_ignore();

    source.add(Person::unknown());

    sub.dispose();
    assert!(tcs.task().is_canceled());

    source.dispose();
}

// Covers https://github.com/reactivemarbles/DynamicData/issues/1169
#[test]
fn remove_without_index() {
    let source = Observable::to_observable(vec![
        ChangeSet::from(vec![Change::range_change(
            ListChangeReason::AddRange,
            vec![1, 2, 3, 2, 1], // Include a duplicate of the item to be removed.
            None,
        )]),
        ChangeSet::from(vec![Change::new(ListChangeReason::Remove, 2, None)]),
    ]);

    let (subscription, results) = source.transform_async(|item: i32| async move { Ok::<_, ObservableError>(item.to_string()) }).validate_list_change_sets().record_list_items();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert_eq!(results.recorded_change_sets().len(), 2, "2 source operations were performed");
    // The first occurrence of the given item should have been removed.
    assert_equivalent_strict(&results.recorded_items(), &["1", "3", "2", "1"].map(String::from));
    assert!(results.has_completed(), "the source, and all asynchronous operations, have completed");

    subscription.dispose();
}

// Covers https://github.com/reactivemarbles/DynamicData/issues/1169
#[test]
fn remove_range_without_index() {
    let source = Observable::to_observable(vec![
        ChangeSet::from(vec![Change::range_change(
            ListChangeReason::AddRange,
            vec![1, 2, 3, 4, 5, 4, 3, 2, 1], // Include duplicates of the items to be removed.
            None,
        )]),
        ChangeSet::from(vec![Change::range_change(ListChangeReason::RemoveRange, vec![1, 3, 5], None)]),
    ]);

    let (subscription, results) = source.transform_async(|item: i32| async move { Ok::<_, ObservableError>(item.to_string()) }).validate_list_change_sets().record_list_items();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert_eq!(results.recorded_change_sets().len(), 2, "2 source operations were performed");
    // The first occurrence of each odd-numbered item in the list should have been removed.
    assert_equivalent_strict(&results.recorded_items(), &["2", "4", "4", "3", "2", "1"].map(String::from));
    assert!(results.has_completed(), "the source, and all asynchronous operations, have completed");

    subscription.dispose();
}
