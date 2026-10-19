//! `NotifyPropertyChangedExFixture` (`DynamicData.Tests/Binding/NotifyPropertyChangedExFixture.cs`).

use crate::domain::Person;
use crate::utilities::ObservableExtensions;
use ferroui_dynamic_data::binding::{AbstractNotifyPropertyChanged, NotifyPropertyChangedEx, PropertyChain, PropertyValue};
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::prop;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn subscribe_to_property_change_for_all_items_in_list(notify_on_initial_value: bool) {
    let last_change = Rc::new(RefCell::new(PropertyValue::new(Person::unknown(), -1)));
    let source = SourceList::<Rc<Person>>::new();
    let sink = last_change.clone();
    source
        .connect()
        .when_property_changed_with(prop!(Person => age), notify_on_initial_value)
        .subscribe_next(move |c: PropertyValue<Rc<Person>, i32>| *sink.borrow_mut() = c);
    let person = Person::new("Name", 10);
    let another_person = Person::new("AnotherName", 10);
    source.edit(|list| list.add(person.clone()));
    source.edit(|list| list.add(another_person.clone()));

    if notify_on_initial_value {
        assert_eq!(another_person, *last_change.borrow().sender());
        assert_eq!(last_change.borrow().value(), Some(&10));
    } else {
        assert_eq!(last_change.borrow().sender().name(), "unknown");
        assert_eq!(last_change.borrow().value(), Some(&-1));
    }

    person.set_age(12);
    assert_eq!(*last_change.borrow().sender(), person);
    assert_eq!(last_change.borrow().value(), Some(&12));
    another_person.set_age(13);
    assert_eq!(*last_change.borrow().sender(), another_person);
    assert_eq!(last_change.borrow().value(), Some(&13));
}

#[test]
fn subscribe_to_property_change_for_all_items_in_list_case1() {
    subscribe_to_property_change_for_all_items_in_list(true);
}

#[test]
fn subscribe_to_property_change_for_all_items_in_list_case2() {
    subscribe_to_property_change_for_all_items_in_list(false);
}

fn subscribe_to_propery_changed_on_a_single_item(notify_on_initial_value: bool) {
    let last_change = Rc::new(RefCell::new(PropertyValue::new(Person::unknown(), -1)));
    let person = Person::new("Name", 10);
    let sink = last_change.clone();
    person
        .when_property_changed_with(prop!(Person => age), notify_on_initial_value)
        .subscribe_next(move |c: PropertyValue<Rc<Person>, i32>| *sink.borrow_mut() = c);

    if notify_on_initial_value {
        assert_eq!(*last_change.borrow().sender(), person);
        assert_eq!(last_change.borrow().value(), Some(&10));
    } else {
        assert_eq!(last_change.borrow().sender().name(), "unknown");
        assert_eq!(last_change.borrow().value(), Some(&-1));
    }

    person.set_age(12);
    assert_eq!(*last_change.borrow().sender(), person);
    assert_eq!(last_change.borrow().value(), Some(&12));
    person.set_age(13);
    assert_eq!(*last_change.borrow().sender(), person);
    assert_eq!(last_change.borrow().value(), Some(&13));
}

#[test]
fn subscribe_to_propery_changed_on_a_single_item_case1() {
    subscribe_to_propery_changed_on_a_single_item(true);
}

#[test]
fn subscribe_to_propery_changed_on_a_single_item_case2() {
    subscribe_to_propery_changed_on_a_single_item(false);
}

fn subscribe_to_value_changed_on_a_single_item(notify_on_initial_value: bool) {
    let age = Rc::new(Cell::new(-1));
    let person = Person::new("Name", 10);
    let sink = age.clone();
    person.when_value_changed_with(prop!(Person => age), notify_on_initial_value).subscribe_next(move |i: i32| sink.set(i));

    assert_eq!(if notify_on_initial_value { 10 } else { -1 }, age.get());
    person.set_age(12);
    assert_eq!(age.get(), 12);
    person.set_age(13);
    assert_eq!(age.get(), 13);
}

#[test]
fn subscribe_to_value_changed_on_a_single_item_case1() {
    subscribe_to_value_changed_on_a_single_item(true);
}

#[test]
fn subscribe_to_value_changed_on_a_single_item_case2() {
    subscribe_to_value_changed_on_a_single_item(false);
}

fn subscribe_to_value_change_for_all_items_in_list(notify_on_initial_value: bool) {
    let last_age_change = Rc::new(Cell::new(-1));
    let source = SourceList::<Rc<Person>>::new();
    let sink = last_age_change.clone();
    // The list form emits the default value for a value it cannot read (`None` here).
    source
        .connect()
        .when_value_changed_with(prop!(Person => age), notify_on_initial_value)
        .subscribe_next(move |i: Option<i32>| sink.set(i.unwrap_or_default()));
    let person = Person::new("Name", 10);
    let another_person = Person::new("AnotherName", 10);
    source.edit(|list| list.add(person.clone()));
    source.edit(|list| list.add(another_person.clone()));

    assert_eq!(if notify_on_initial_value { 10 } else { -1 }, last_age_change.get());
    person.set_age(12);
    assert_eq!(last_age_change.get(), 12);
    another_person.set_age(13);
    assert_eq!(last_age_change.get(), 13);
}

#[test]
fn subscribe_to_value_change_for_all_items_in_list_case1() {
    subscribe_to_value_change_for_all_items_in_list(true);
}

#[test]
fn subscribe_to_value_change_for_all_items_in_list_case2() {
    subscribe_to_value_change_for_all_items_in_list(false);
}

#[test]
fn cast_to_nullable() {
    let parent = TestEntity::new(1, 10);

    // entity => (int?)entity.Child.Age
    let path = PropertyChain::nullable(prop!(TestEntity => child)).then(prop!(TestEntity => age)).map(Some);
    let (subscription, results) = parent.when_value_changed_with_fallback(path, true, || None).record_values();

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "additional changes could be made");
    assert_eq!(results.recorded_values().len(), 1, "an initial value should have been published");
    assert_eq!(results.recorded_values()[0], None, "the target entity has no child");

    let child = TestEntity::new(2, 5);
    parent.set_child(Some(child.clone()));

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "additional changes could be made");
    assert_eq!(results.recorded_values().len() - 1, 1, "a single change was performed");
    assert_eq!(results.recorded_values()[1], Some(child.age()), "a child of age 5 was added");

    child.set_age(6);

    assert!(results.error().is_none(), "no errors should have occurred");
    assert!(!results.has_completed(), "additional changes could be made");
    assert_eq!(results.recorded_values().len() - 2, 1, "a single change was performed");
    assert_eq!(results.recorded_values()[2], Some(child.age()), "the child entity's age was changed");

    subscription.dispose();
}

/// `TestEntity`: an id, an age and a child of its own type, raising their names when they
/// change (`SetPropertyField`); the child is compared by reference, as upstream's default
/// comparer of a class without `Equals`.
pub(crate) struct TestEntity {
    notify: AbstractNotifyPropertyChanged,
    id: i64,
    age: Cell<i32>,
    child: RefCell<Option<Rc<TestEntity>>>,
}

impl TestEntity {
    /// `new TestEntity { Id = id, Age = age }`.
    pub(crate) fn new(id: i64, age: i32) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), id, age: Cell::new(age), child: RefCell::new(None) })
    }

    /// `Id`.
    #[allow(dead_code)]
    pub(crate) fn id(&self) -> i64 {
        self.id
    }

    /// `Age`.
    pub(crate) fn age(&self) -> i32 {
        self.age.get()
    }

    /// Sets `Age`, raising it when it changes.
    pub(crate) fn set_age(&self, value: i32) {
        self.notify.set_and_raise(&self.age, value, "Age");
    }

    /// `Child`.
    pub(crate) fn child(&self) -> Option<Rc<TestEntity>> {
        self.child.borrow().clone()
    }

    /// Sets `Child`, raising it when it is another object.
    pub(crate) fn set_child(&self, value: Option<Rc<TestEntity>>) {
        self.notify.set_and_raise_with(
            &self.child,
            value,
            |current, value| match (current, value) {
                (Some(current), Some(value)) => Rc::ptr_eq(current, value),
                (None, None) => true,
                _ => false,
            },
            "Child",
        );
    }
}

impl INotifyPropertyChanged for TestEntity {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}
