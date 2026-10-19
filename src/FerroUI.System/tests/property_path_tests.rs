//! Tests of `component_model::PropertyPath` and `prop!`.

use crate::component_model::{property_name_of, INotifyPropertyChanged};
use crate::events::Event;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Address {
    street: RefCell<String>,
    changed: Event<str>,
}

impl Address {
    fn street(&self) -> String {
        self.street.borrow().clone()
    }

    fn set_street(&self, street: &str) {
        *self.street.borrow_mut() = street.to_string();
        self.changed.raise("Street");
    }
}

impl INotifyPropertyChanged for Address {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

struct Person {
    address: RefCell<Rc<Address>>,
    changed: Event<str>,
}

impl Person {
    fn address(&self) -> Rc<Address> {
        self.address.borrow().clone()
    }

    fn set_address(&self, address: Rc<Address>) {
        *self.address.borrow_mut() = address;
        self.changed.raise("Address");
    }
}

impl INotifyPropertyChanged for Person {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

fn address(street: &str) -> Rc<Address> {
    Rc::new(Address { street: RefCell::new(street.to_string()), changed: Event::new() })
}

#[test]
fn names_are_pascal_case() {
    assert_eq!(property_name_of("favorite_color"), "FavoriteColor");
    assert_eq!(property_name_of("age"), "Age");
    assert_eq!(crate::prop!(Person => address as "Home").path(), "Home");
}

#[test]
fn a_nested_path_follows_the_object_of_each_step() {
    let first = address("First");
    let person = Rc::new(Person { address: RefCell::new(first.clone()), changed: Event::new() });
    let path = crate::prop!(Person => address.street);
    assert_eq!(path.path(), "Address.Street");
    assert_eq!(path.get(&person), "First");

    let changes = Rc::new(Cell::new(0));
    let counter = changes.clone();
    let subscription = path.observe(&person, Rc::new(move || counter.set(counter.get() + 1)));

    first.set_street("First, renamed");
    assert_eq!(changes.get(), 1);

    let second = address("Second");
    person.set_address(second.clone());
    assert_eq!(changes.get(), 2);
    assert_eq!(path.get(&person), "Second");

    // The old address is no longer observed, the new one is.
    first.set_street("ignored");
    assert_eq!(changes.get(), 2);
    second.set_street("Second, renamed");
    assert_eq!(changes.get(), 3);

    subscription.dispose();
    second.set_street("after");
    person.set_address(address("Third"));
    assert_eq!(changes.get(), 3);
    assert!(!second.changed.has_handlers() && !person.changed.has_handlers());
}
