//! `ForEachChangeFixture` (`DynamicData.Tests/List/ForEachChangeFixture.cs`): an action run for each
//! change of a list, and for the change of each item with the ranges spread into their items.

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::list::{Change, IObservableList, ItemChange, ObservableListEx, SourceList};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

struct Fixture {
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        Self { source: SourceList::new() }
    }

    /// `_source.AddRange(items)`.
    fn add_range(&self, items: Vec<Rc<Person>>) {
        self.source.edit(move |list| list.add_range(items));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

#[test]
fn each_change_inokes_the_callback() {
    let fixture = Fixture::new();
    let messages: Rc<RefCell<Vec<Change<Rc<Person>>>>> = Rc::new(RefCell::new(Vec::new()));

    let recorder = messages.clone();
    let message_writer = fixture
        .source
        .connect()
        .for_each_change(move |change: &Change<Rc<Person>>| recorder.borrow_mut().push(change.clone()))
        .subscribe_ignore();

    let people = RandomPersonGenerator::new().take(100);
    for person in people {
        fixture.source.edit(move |list| list.add(person));
    }

    assert_eq!(messages.borrow().len(), 100);
    message_writer.dispose();
}

#[test]
fn each_item_change_inokes_the_callbac2() {
    let fixture = Fixture::new();
    let messages: Rc<RefCell<Vec<ItemChange<Rc<Person>>>>> = Rc::new(RefCell::new(Vec::new()));

    let recorder = messages.clone();
    let message_writer = fixture
        .source
        .connect()
        .for_each_item_change(move |change: &ItemChange<Rc<Person>>| recorder.borrow_mut().push(change.clone()))
        .subscribe_ignore();
    fixture.add_range(RandomPersonGenerator::new().take(5));
    let inserted = RandomPersonGenerator::new().take(5);
    fixture.source.edit(move |list| list.insert_range(inserted, 2));
    fixture.add_range(RandomPersonGenerator::new().take(5));

    assert_eq!(messages.borrow().len(), 15);
    message_writer.dispose();
}

#[test]
fn each_item_change_inokes_the_callback() {
    let fixture = Fixture::new();
    let messages: Rc<RefCell<Vec<ItemChange<Rc<Person>>>>> = Rc::new(RefCell::new(Vec::new()));

    let recorder = messages.clone();
    let message_writer = fixture
        .source
        .connect()
        .for_each_item_change(move |change: &ItemChange<Rc<Person>>| recorder.borrow_mut().push(change.clone()))
        .subscribe_ignore();

    fixture.add_range(RandomPersonGenerator::new().take(100));

    assert_eq!(messages.borrow().len(), 100);
    message_writer.dispose();
}
