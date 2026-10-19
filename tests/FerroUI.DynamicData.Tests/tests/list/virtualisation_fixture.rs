//! `VirtualisationFixture` of lists (`DynamicData.Tests/List/VirtualisationFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::VirtualRequest;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{clone_changes, IExtendedList, IObservableList, ListChangeReason, ObservableListEx, SourceList, VirtualChangeSet};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

struct Fixture {
    generator: RandomPersonGenerator,
    request_subject: Rc<BehaviorSubject<VirtualRequest>>,
    results: ChangeSetAggregator<Rc<Person>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();
        let request_subject = BehaviorSubject::new(VirtualRequest::new(0, 25));
        let results = source.connect().virtualise(request_subject.as_observable()).as_aggregator();
        Self { generator: RandomPersonGenerator::new(), request_subject, results, source }
    }

    fn add_range(&self, people: &[Rc<Person>]) {
        let people = people.to_vec();
        self.source.edit(|list| list.add_range(people));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn insert_after_page_produces_nothing() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_range(&people);

    let expected: Vec<Rc<Person>> = people.iter().take(25).cloned().collect();

    let more = fixture.generator.take(100);
    fixture.source.edit(|list| IExtendedList::insert_range(list, more, 50));
    assert_equivalent(&fixture.results.data().items(), &expected);
}

#[test]
fn insert_in_page_reflects_change() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_range(&people);

    let new_person = Person::new("A", 1);
    fixture.source.edit(|list| list.insert(10, new_person.clone()));

    let message = fixture.results.messages()[1][0].clone();
    let removed_person = people[24].clone();

    assert_eq!(fixture.results.data().items()[10], new_person);
    assert_eq!(message.item().current(), &removed_person);
    assert_eq!(message.reason(), ListChangeReason::Remove);
}

#[test]
fn move_to_next_page() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_range(&people);

    fixture.request_subject.on_next(VirtualRequest::new(25, 25));

    let expected: Vec<Rc<Person>> = people.iter().skip(25).take(25).cloned().collect();
    assert_equivalent(&fixture.results.data().items(), &expected);
}

#[test]
fn move_within_same_page() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_range(&people);

    let person_to_move = people[0].clone();
    fixture.source.edit(|list| IExtendedList::move_index(list, 0, 10));

    let actual_person_at_index10 = fixture.results.data().items()[10].clone();
    assert_eq!(actual_person_at_index10, person_to_move);
}

#[test]
fn move_within_same_page2() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_range(&people);

    let person_to_move = people[10].clone();
    fixture.source.edit(|list| IExtendedList::move_index(list, 10, 0));

    let actual_person_at_index0 = fixture.results.data().items()[0].clone();
    assert_eq!(actual_person_at_index0, person_to_move);
}

#[test]
fn remove_before_shifts_page() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_range(&people);

    fixture.request_subject.on_next(VirtualRequest::new(25, 25));
    fixture.source.edit(|list| list.remove_at(0));

    let expected: Vec<Rc<Person>> = people.iter().skip(26).take(25).cloned().collect();
    assert_equivalent(&fixture.results.data().items(), &expected);

    let messages = fixture.results.messages();
    let removed_message = &messages[2][0];
    let removed_person = &people[25];
    assert_eq!(removed_message.item().current(), removed_person);
    assert_eq!(removed_message.reason(), ListChangeReason::Remove);

    let added_message = &messages[2][1];
    let added_person = &people[50];
    assert_eq!(added_message.item().current(), added_person);
    assert_eq!(added_message.reason(), ListChangeReason::Add);
}

#[test]
fn virtualise_initial() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_range(&people);

    let expected: Vec<Rc<Person>> = people.iter().take(25).cloned().collect();
    assert_equivalent(&fixture.results.data().items(), &expected);
}

/// See https://github.com/reactivemarbles/DynamicData/issues/540.
#[test]
fn does_not_throw_with_duplicates() {
    let result = Rc::new(RefCell::new(Vec::<String>::new()));
    let source = SourceList::<String>::new();

    source.edit(|list| list.add_range(vec!["item".to_owned(); 10]));

    let target = result.clone();
    let _subscription = source
        .connect()
        .virtualise(BehaviorSubject::new(VirtualRequest::new(0, 3)).as_observable())
        .subscribe_next(move |changes: VirtualChangeSet<String>| clone_changes(&mut *target.borrow_mut(), &changes));

    assert_eq!(result.borrow().len(), 1);
}
