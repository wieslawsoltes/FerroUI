//! `ObservedPropertyTests` (`ReactiveUI.Binding.Tests/Mixins/ObservedPropertyTests.cs`).
//!
//! Upstream gives `ObservedProperty` a lambda and a delegate that reads the property; here a
//! one-step property path carries both. The provider registrations are per thread, and each test
//! runs on a thread of its own, so the tests that change them need no restoring; they unregister
//! what they registered anyway, as upstream's `finally` blocks do.

use crate::mocks::collect_items;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyOwner, PropertyPath};
use ferroui_system::events::Event;
use ferroui_system::{prop, IObservable, IObserver};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The name the person starts with.
const ADA: &str = "Ada";

/// The name the person changes to.
const GRACE: &str = "Grace";

/// The age the person starts with.
const YOUNG: i32 = 3;

/// The age the person changes to.
const OLDER: i32 = 4;

/// A provider score above what change notifications get.
const PROVIDER_AFFINITY: i32 = 50;

/// The cities a followed path passes through.
const CITIES: [&str; 3] = ["Paris", "Rome", "Oslo"];

#[test]
fn create_notifying_source_delivers_current_value_then_distinct_changes() {
    let person = Person::new();
    person.set_name(Some(ADA.to_string()));

    let values = collect_items(&ObservedProperty::create(&person, prop!(Person => name)));
    person.set_name(Some(GRACE.to_string()));
    person.set_name(Some(GRACE.to_string()));
    person.raise("Name");

    assert_eq!(*values.borrow(), vec![Some(ADA.to_string()), Some(GRACE.to_string())]);
}

/// Upstream reads a value type as `object` through a converting lambda (`x => (object)x.Age`); a
/// path whose getter converts the value still names the property, so the conversion here is a
/// widening to `i64`.
#[test]
fn create_converted_property_is_read_by_name() {
    let person = Person::new();
    person.set_age(YOUNG);

    let age = PropertyPath::<Rc<Person>, i64>::property("Age", |person: &Rc<Person>| i64::from(person.age()));
    let values = collect_items(&ObservedProperty::create(&person, age));
    person.set_age(OLDER);

    assert_eq!(*values.borrow(), vec![i64::from(YOUNG), i64::from(OLDER)]);
}

/// Upstream unregisters every provider for the second observation, so that the property is read
/// directly. Here an empty registration falls back to the default providers, so the second
/// observation registers a provider that never scores instead, which leaves no provider able to
/// observe the property.
#[test]
fn create_source_without_notifications_is_read_once() {
    let model = Plain::new(ADA);

    let with_providers = collect_items(&ObservedProperty::create(&model, Plain::value_path()));
    AppLocator::unregister_all::<dyn ICreatesObservableForProperty>();
    AppLocator::register_constant::<dyn ICreatesObservableForProperty>(Rc::new(NeverScores) as Rc<dyn ICreatesObservableForProperty>);
    let without_providers = collect_items(&ObservedProperty::create(&model, Plain::value_path()));
    model.set_value(GRACE);
    AppLocator::unregister_all::<dyn ICreatesObservableForProperty>();

    assert_eq!(*with_providers.borrow(), vec![ADA.to_string()]);
    assert_eq!(*without_providers.borrow(), vec![ADA.to_string()]);
}

#[test]
fn create_two_properties_delivers_pairs() {
    let person = Person::new();
    person.set_name(Some(ADA.to_string()));
    person.set_age(YOUNG);

    let values = collect_items(&ObservedProperty::create2(&person, prop!(Person => name), prop!(Person => age)));
    person.set_age(OLDER);

    assert_eq!(*values.borrow(), vec![(Some(ADA.to_string()), YOUNG), (Some(ADA.to_string()), OLDER)]);
}

#[test]
fn create_two_properties_with_selector_delivers_projection() {
    let person = Person::new();
    person.set_name(Some(ADA.to_string()));
    person.set_age(YOUNG);

    let values = collect_items(&ObservedProperty::create2_select(&person, prop!(Person => name), prop!(Person => age), |name, age| {
        format!("{} {}", name.unwrap_or_default(), age)
    }));
    person.set_name(Some(GRACE.to_string()));

    assert_eq!(*values.borrow(), vec!["Ada 3".to_string(), "Grace 3".to_string()]);
}

#[test]
fn then_follows_each_replacement_of_the_parent() {
    let person = Person::new();
    person.set_home(Some(Address::new("Paris")));

    let values = collect_items(&ObservedProperty::then(ObservedProperty::create(&person, prop!(Person => home)), prop!(Address => city)));
    person.home().expect("the person has a home").set_city(Some("Rome".to_string()));
    person.set_home(None);
    person.set_home(Some(Address::new("Rome")));
    person.set_home(Some(Address::new("Oslo")));

    let expected: Vec<Option<String>> = CITIES.iter().map(|city| Some(city.to_string())).collect();
    assert_eq!(*values.borrow(), expected);
}

/// Upstream follows a property of type `IObservable<string>?`; an observable has no equality
/// here, so the property holds a [`Messages`], which compares the observables by reference as
/// upstream's distinct gate does, and the test unwraps it before switching.
#[test]
fn switch_follows_the_latest_held_observable() {
    let first = Subject::<String>::new();
    let second = Subject::<String>::new();
    let person = Person::new();
    person.set_messages(Messages(Some(first.as_observable())));

    let held = ObservedProperty::create(&person, prop!(Person => messages)).select(|messages: Messages| messages.0);
    let values = collect_items(&ObservedProperty::switch(held));
    first.on_next("one".to_string());
    person.set_messages(Messages(None));
    first.on_next("lost".to_string());
    person.set_messages(Messages(Some(second.as_observable())));
    first.on_next("stale".to_string());
    second.on_next("two".to_string());

    assert_eq!(*values.borrow(), vec!["one".to_string(), "two".to_string()]);
}

/// Upstream checks the expression the provider was handed (`x.Name`); a provider is handed the
/// property name here.
#[test]
fn create_higher_scoring_provider_takes_over() {
    AppLocator::unregister_all::<dyn ICreatesObservableForProperty>();
    let provider = Rc::new(ConstantProvider::new(PROVIDER_AFFINITY));
    AppLocator::register_constant::<dyn ICreatesObservableForProperty>(provider.clone() as Rc<dyn ICreatesObservableForProperty>);

    let person = Person::new();
    person.set_name(Some(ADA.to_string()));
    let plain = Plain::new(ADA);

    let names = collect_items(&ObservedProperty::create(&person, prop!(Person => name)));
    let plain_values = collect_items(&ObservedProperty::create(&plain, Plain::value_path()));
    provider.changes.on_next(ObservedChange::new(Person::owner(&person), "", ()));
    person.set_name(Some(GRACE.to_string()));
    provider.changes.on_next(ObservedChange::new(Person::owner(&person), "", ()));
    AppLocator::unregister_all::<dyn ICreatesObservableForProperty>();

    assert!(provider.property_names.borrow().iter().any(|name| name == "Name"));
    assert_eq!(*names.borrow(), vec![Some(ADA.to_string()), Some(GRACE.to_string())]);
    assert_eq!(*plain_values.borrow(), vec![ADA.to_string()]);
}

/// Upstream's rows are `x => x.Home!.City`, a path of two steps, and `x => x.ToString()`, a call;
/// a property path is made of properties, so only the first row has a counterpart.
#[test]
#[should_panic(expected = "must name a property")]
fn create_lambda_that_names_no_property_throws() {
    let person = Person::new();
    let home_city = prop!(Person => home).then_optional_property("City", |address: &Rc<Address>| address.city());
    let _ = ObservedProperty::create(&person, home_city);
}

// Not ported: NullArguments_Throw - argument validation

/// The stream of messages a person holds, compared by reference (`IObservable<string>?`).
#[derive(Clone, Default)]
struct Messages(Option<Rc<dyn IObservable<String>>>);

impl PartialEq for Messages {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(first), Some(second)) => Rc::ptr_eq(first, second),
            (None, None) => true,
            _ => false,
        }
    }
}

/// A person who notifies when a property changes; every setter notifies (`Person`).
struct Person {
    name: RefCell<Option<String>>,
    age: Cell<i32>,
    home: RefCell<Option<Rc<Address>>>,
    messages: RefCell<Messages>,
    property_changed: Event<str>,
}

impl Person {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            name: RefCell::new(None),
            age: Cell::new(0),
            home: RefCell::new(None),
            messages: RefCell::new(Messages::default()),
            property_changed: Event::new(),
        })
    }

    /// The person as the owner of a property, as a provider sees it.
    fn owner(person: &Rc<Person>) -> PropertyOwner {
        PropertyOwner {
            object: Some(person.clone() as Rc<dyn Any>),
            notify: Some(person.clone() as Rc<dyn INotifyPropertyChanged>),
        }
    }

    fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
        self.raise("Name");
    }

    fn age(&self) -> i32 {
        self.age.get()
    }

    fn set_age(&self, value: i32) {
        self.age.set(value);
        self.raise("Age");
    }

    fn home(&self) -> Option<Rc<Address>> {
        self.home.borrow().clone()
    }

    fn set_home(&self, value: Option<Rc<Address>>) {
        *self.home.borrow_mut() = value;
        self.raise("Home");
    }

    fn messages(&self) -> Messages {
        self.messages.borrow().clone()
    }

    fn set_messages(&self, value: Messages) {
        *self.messages.borrow_mut() = value;
        self.raise("Messages");
    }

    /// Raises a change notification (`Raise`).
    fn raise(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }
}

impl INotifyPropertyChanged for Person {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

/// An address that notifies when its city changes; addresses compare by identity (`Address`).
struct Address {
    city: RefCell<Option<String>>,
    property_changed: Event<str>,
}

impl Address {
    fn new(city: &str) -> Rc<Self> {
        Rc::new(Self { city: RefCell::new(Some(city.to_string())), property_changed: Event::new() })
    }

    fn city(&self) -> Option<String> {
        self.city.borrow().clone()
    }

    fn set_city(&self, value: Option<String>) {
        *self.city.borrow_mut() = value;
        self.property_changed.raise("City");
    }
}

impl INotifyPropertyChanged for Address {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A model that raises no notification (`Plain`).
struct Plain {
    value: RefCell<String>,
}

impl Plain {
    fn new(value: &str) -> Rc<Self> {
        Rc::new(Self { value: RefCell::new(value.to_string()) })
    }

    /// The path of `Value`, a value that does not notify its changes.
    fn value_path() -> PropertyPath<Rc<Plain>, String> {
        PropertyPath::value("Value", |plain: &Rc<Plain>| plain.value.borrow().clone())
    }

    fn set_value(&self, value: &str) {
        *self.value.borrow_mut() = value.to_string();
    }
}

/// A provider that scores every property the same and reports the changes a test pushes
/// (`ConstantProvider`).
struct ConstantProvider {
    affinity: i32,
    /// The changes the provider reports (`Changes`).
    changes: Rc<Subject<ObservedChange<PropertyOwner, ()>>>,
    /// The properties the provider was asked for (`Expressions`).
    property_names: RefCell<Vec<String>>,
}

impl ConstantProvider {
    fn new(affinity: i32) -> Self {
        Self { affinity, changes: Subject::new(), property_names: RefCell::new(Vec::new()) }
    }
}

impl ICreatesObservableForProperty for ConstantProvider {
    fn get_affinity_for_object(&self, _owner: &PropertyOwner, _property_name: &str, _before_changed: bool) -> i32 {
        self.affinity
    }

    fn get_notification_for_property(
        &self,
        _owner: &PropertyOwner,
        property_name: &str,
        _before_changed: bool,
        _suppress_warnings: bool,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
        self.property_names.borrow_mut().push(property_name.to_string());
        self.changes.as_observable()
    }
}

/// A provider that cannot observe anything, so that no registered provider scores.
struct NeverScores;

impl ICreatesObservableForProperty for NeverScores {
    fn get_affinity_for_object(&self, _owner: &PropertyOwner, _property_name: &str, _before_changed: bool) -> i32 {
        0
    }

    fn get_notification_for_property(
        &self,
        _owner: &PropertyOwner,
        _property_name: &str,
        _before_changed: bool,
        _suppress_warnings: bool,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
        Observable::never()
    }
}
