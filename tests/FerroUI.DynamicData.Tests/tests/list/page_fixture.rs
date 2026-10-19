//! `PageFixture`, `PageFixtureWithNoInitialData` and `SimplePaging` of lists
//! (`DynamicData.Tests/List/PageFixture.cs`).

use crate::domain::{Animal, AnimalFamily, Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::PageRequest;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{clone_changes, IExtendedList, IObservableList, ListChangeReason, ObservableList, ObservableListEx, PageChangeSet, SourceList};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

struct Fixture {
    generator: RandomPersonGenerator,
    request_subject: Rc<BehaviorSubject<PageRequest>>,
    results: ChangeSetAggregator<Rc<Person>>,
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();
        let request_subject = BehaviorSubject::new(PageRequest::new(1, 25));
        let results = source.connect().page(request_subject.as_observable()).as_aggregator();
        Self { generator: RandomPersonGenerator::new(), request_subject, results, source }
    }

    fn add_range(&self, people: &[Rc<Person>]) {
        let people = people.to_vec();
        self.source.edit(|list| list.add_range(people));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.request_subject.on_completed();
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

    fixture.request_subject.on_next(PageRequest::new(2, 25));

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

    fixture.request_subject.on_next(PageRequest::new(2, 25));
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

// `PageFixtureWithNoInitialData`.

fn items() -> Vec<Rc<Animal>> {
    vec![
        Animal::new("Holly", "Cat", AnimalFamily::Mammal),
        Animal::new("Rover", "Dog", AnimalFamily::Mammal),
        Animal::new("Rex", "Dog", AnimalFamily::Mammal),
        Animal::new("Whiskers", "Cat", AnimalFamily::Mammal),
        Animal::new("Nemo", "Fish", AnimalFamily::Fish),
        Animal::new("Moby Dick", "Whale", AnimalFamily::Mammal),
        Animal::new("Fred", "Frog", AnimalFamily::Amphibian),
        Animal::new("Isaac", "Next", AnimalFamily::Amphibian),
        Animal::new("Sam", "Snake", AnimalFamily::Reptile),
        Animal::new("Sharon", "Red Backed Shrike", AnimalFamily::Bird),
    ]
}

/// `SimplePaging`: the paged items of a list as an observable list.
struct SimplePaging {
    paged: ObservableList<Rc<Animal>>,
}

impl SimplePaging {
    fn new(source: &dyn IObservableList<Rc<Animal>>, pager: Rc<dyn IObservable<PageRequest>>) -> Self {
        // Upstream writes the total changes to the console, "a quick and dirty way to debug".
        let paged = source.connect().page(pager).do_(|_changes: &PageChangeSet<Rc<Animal>>| {}).as_observable_list();
        Self { paged }
    }
}

impl Drop for SimplePaging {
    fn drop(&mut self) {
        self.paged.dispose();
    }
}

#[test]
fn page_fixture_with_no_initial_data_simple_paging() {
    let pager = BehaviorSubject::new(PageRequest::new(0, 0));
    let source_list = SourceList::<Rc<Animal>>::new();
    let sut = SimplePaging::new(&source_list, pager.as_observable());

    // Add items to the source.
    source_list.edit(|list| list.add_range(items()));

    assert_eq!(sut.paged.count(), 0);

    pager.on_next(PageRequest::new(1, 2));
    assert_eq!(sut.paged.count(), 2);

    pager.on_next(PageRequest::new(1, 4));
    assert_eq!(sut.paged.count(), 4);

    pager.on_next(PageRequest::new(2, 3));
    assert_eq!(sut.paged.count(), 3);

    drop(sut);
    source_list.dispose();
    pager.dispose();
}

/// See https://github.com/reactivemarbles/DynamicData/issues/540.
#[test]
fn page_fixture_with_no_initial_data_does_not_throw_with_duplicates() {
    let result = Rc::new(RefCell::new(Vec::<String>::new()));
    let source = SourceList::<String>::new();

    source.edit(|list| list.add_range(vec!["item".to_owned(); 10]));

    let target = result.clone();
    let _subscription = source
        .connect()
        .page(BehaviorSubject::new(PageRequest::new(0, 3)).as_observable())
        .subscribe_next(move |changes: PageChangeSet<String>| clone_changes(&mut *target.borrow_mut(), &changes));

    assert_eq!(result.borrow().len(), 1);
}

