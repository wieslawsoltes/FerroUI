//! Tests of `component_model::PropertyPath` and `prop!`.

use crate::component_model::{property_name_of, INotifyPropertyChanged, PropertyOwner, PropertyPath};
use std::any::Any;
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

/// Runs the type-erased steps of `path` from `source`, as an observer chosen at run time does: the
/// owner of each step, then its value, which the next step reads from.
fn run_steps<S: Clone + 'static, V: 'static>(path: &PropertyPath<S, V>, source: &S) -> (Vec<PropertyOwner>, Box<dyn Any>) {
    let mut owners = Vec::new();
    let mut value: Box<dyn Any> = Box::new(source.clone());
    for step in path.steps() {
        owners.push((step.owner)(&*value));
        value = (step.get)(&*value);
    }
    (owners, value)
}

#[test]
fn the_steps_of_a_nested_path_read_each_property_from_the_step_before() {
    let home = address("Home");
    let person = Rc::new(Person { address: RefCell::new(home.clone()), changed: Event::new() });
    let path = crate::prop!(Person => address.street);
    let names: Vec<&str> = path.steps().iter().map(|step| step.name.as_str()).collect();
    assert_eq!(names, ["Address", "Street"]);

    let (owners, value) = run_steps(&path, &person);
    assert_eq!(*value.downcast::<String>().unwrap(), "Home");

    // The owner of each step is the object that holds the property, with its notification.
    let first = owners[0].object.clone().unwrap().downcast::<Person>().ok().unwrap();
    assert!(Rc::ptr_eq(&first, &person));
    let second = owners[1].object.clone().unwrap().downcast::<Address>().ok().unwrap();
    assert!(Rc::ptr_eq(&second, &home));
    assert!(owners.iter().all(|owner| owner.notify.is_some()));

    // The notification of an owner is the object's own.
    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();
    let token = owners[1].notify.as_ref().unwrap().property_changed().add(Rc::new(move |_: &str| counter.set(counter.get() + 1)));
    home.set_street("Elsewhere");
    assert_eq!(raised.get(), 1);
    home.changed.remove(token);
}

#[test]
fn the_steps_follow_the_object_the_path_holds_now() {
    let person = Rc::new(Person { address: RefCell::new(address("First")), changed: Event::new() });
    let path = crate::prop!(Person => address.street);
    let second = address("Second");
    person.set_address(second.clone());
    let (owners, value) = run_steps(&path, &person);
    assert_eq!(*value.downcast::<String>().unwrap(), "Second");
    assert!(owners[1].same_object(&PropertyOwner { object: Some(second as Rc<dyn Any>), notify: None }));
}

#[test]
fn a_value_step_has_no_owner() {
    let path = PropertyPath::<i32, String>::value("Text", |value: &i32| value.to_string());
    assert_eq!(path.steps().len(), 1);
    let (owners, value) = run_steps(&path, &42);
    assert!(owners[0].object.is_none() && owners[0].notify.is_none());
    assert_eq!(*value.downcast::<String>().unwrap(), "42");
}

#[test]
fn an_optional_step_has_an_owner_only_while_there_is_an_object() {
    let path = PropertyPath::<Option<Rc<Address>>, Option<String>>::optional_property("Street", |address: &Rc<Address>| address.street());
    assert_eq!(path.get(&None), None);

    let (owners, value) = run_steps(&path, &None);
    assert!(owners[0].object.is_none() && owners[0].notify.is_none());
    assert_eq!(*value.downcast::<Option<String>>().unwrap(), None);

    let home = address("Home");
    let (owners, value) = run_steps(&path, &Some(home.clone()));
    assert!(owners[0].notify.is_some());
    assert_eq!(*value.downcast::<Option<String>>().unwrap(), Some("Home".to_string()));
    assert_eq!(path.get(&Some(home)), Some("Home".to_string()));
}

#[test]
fn then_optional_property_continues_a_path_whose_object_may_be_absent() {
    struct Holder {
        address: RefCell<Option<Rc<Address>>>,
        changed: Event<str>,
    }
    impl Holder {
        fn address(&self) -> Option<Rc<Address>> {
            self.address.borrow().clone()
        }
    }
    impl INotifyPropertyChanged for Holder {
        fn property_changed(&self) -> &Event<str> {
            &self.changed
        }
    }

    let holder = Rc::new(Holder { address: RefCell::new(None), changed: Event::new() });
    let path = crate::prop!(Holder => address).then_optional_property("Street", |address: &Rc<Address>| address.street());
    assert_eq!(path.path(), "Address.Street");
    assert_eq!(path.steps().len(), 2);
    assert_eq!(path.get(&holder), None);

    let changes = Rc::new(Cell::new(0));
    let counter = changes.clone();
    let subscription = path.observe(&holder, Rc::new(move || counter.set(counter.get() + 1)));
    let home = address("Home");
    *holder.address.borrow_mut() = Some(home.clone());
    holder.changed.raise("Address");
    assert_eq!(changes.get(), 1);
    assert_eq!(path.get(&holder), Some("Home".to_string()));
    home.set_street("Elsewhere");
    assert_eq!(changes.get(), 2);
    subscription.dispose();
    assert!(!home.changed.has_handlers() && !holder.changed.has_handlers());
}

#[test]
fn a_cloned_path_shares_its_steps() {
    let path = crate::prop!(Person => address.street);
    let clone = path.clone();
    assert_eq!(clone.steps().len(), path.steps().len());
    assert!(Rc::ptr_eq(&clone.steps()[0].get, &path.steps()[0].get));
}

#[test]
fn optional_steps_and_then_do_not_nest_their_options() {
    struct Holder {
        address: RefCell<Option<Rc<Address>>>,
        changed: Event<str>,
    }
    impl Holder {
        fn address(&self) -> Option<Rc<Address>> {
            self.address.borrow().clone()
        }
    }
    impl INotifyPropertyChanged for Holder {
        fn property_changed(&self) -> &Event<str> {
            &self.changed
        }
    }
    struct Root {
        holder: RefCell<Option<Rc<Holder>>>,
        changed: Event<str>,
    }
    impl Root {
        fn holder(&self) -> Option<Rc<Holder>> {
            self.holder.borrow().clone()
        }
    }
    impl INotifyPropertyChanged for Root {
        fn property_changed(&self) -> &Event<str> {
            &self.changed
        }
    }

    let root = Rc::new(Root { holder: RefCell::new(None), changed: Event::new() });
    let path = crate::prop!(Root => holder)
        .then_optional_property_and_then("Address", |holder: &Rc<Holder>| holder.address())
        .then_optional_property("Street", |address: &Rc<Address>| address.street());
    assert_eq!(path.path(), "Holder.Address.Street");
    assert_eq!(path.steps().len(), 3);
    assert_eq!(path.get(&root), None);

    let (owners, value) = run_steps(&path, &root);
    assert!(owners[0].notify.is_some());
    assert!(owners[1].object.is_none() && owners[2].object.is_none());
    assert_eq!(*value.downcast::<Option<String>>().unwrap(), None);

    let holder = Rc::new(Holder { address: RefCell::new(Some(address("Home"))), changed: Event::new() });
    *root.holder.borrow_mut() = Some(holder.clone());
    assert_eq!(path.get(&root), Some("Home".to_string()));
    let (owners, _) = run_steps(&path, &root);
    assert!(owners[1].notify.is_some() && owners[2].notify.is_some());

    *holder.address.borrow_mut() = None;
    assert_eq!(path.get(&root), None);
}
