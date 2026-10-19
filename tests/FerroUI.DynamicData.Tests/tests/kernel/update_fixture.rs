//! `UpdateFixture` (`DynamicData.Tests/Kernal/UpdateFixture.cs`): the parts of a change of a cache.

use crate::domain::Person;
use ferroui_dynamic_data::cache::{Change, ChangeReason};
use std::rc::Rc;

#[test]
fn add() {
    let person = Person::new("Person", 10);
    let update = Change::new(ChangeReason::Add, "Person".to_string(), person.clone());

    assert_eq!(update.key(), "Person");
    assert_eq!(update.reason(), ChangeReason::Add);
    assert_eq!(update.current(), &person);
    assert_eq!(update.previous(), None::<&Rc<Person>>);
}

#[test]
fn remove() {
    let person = Person::new("Person", 10);
    let update = Change::new(ChangeReason::Remove, "Person".to_string(), person.clone());

    assert_eq!(update.key(), "Person");
    assert_eq!(update.reason(), ChangeReason::Remove);
    assert_eq!(update.current(), &person);
    assert_eq!(update.previous(), None::<&Rc<Person>>);
}

#[test]
fn update() {
    let current = Person::new("Person", 10);
    let previous = Person::new("Person", 9);
    let update = Change::with_previous(ChangeReason::Update, "Person".to_string(), current.clone(), Some(previous.clone()), None, None);

    assert_eq!(update.key(), "Person");
    assert_eq!(update.reason(), ChangeReason::Update);
    assert_eq!(update.current(), &current);
    assert!(update.previous().is_some());
    assert_eq!(update.previous(), Some(&previous));
    assert!(Rc::ptr_eq(update.previous().expect("an update has a previous item"), &previous));
}

/// `Assert.Throws<ArgumentException>` becomes the panic of the constructor.
#[test]
#[should_panic(expected = "For ChangeReason.Update, must supply previous value")]
fn update_will_throw_if_no_previous_value_is_supplied() {
    let current = Person::new("Person", 10);
    let _ = Change::new(ChangeReason::Update, "Person".to_string(), current);
}
