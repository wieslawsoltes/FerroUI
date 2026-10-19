//! `ObservableCollectionExFixture` (`DynamicData.Tests/ObservableCollectionExFixture.cs`).

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::ObservableCollectionEx;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx};
use ferroui_dynamic_data::{IObservableCache, ObservableCacheEx};
use ferroui_system::collections::object_model::ObservableCollection;
use std::rc::Rc;

struct Fixture {
    person1: Rc<Person>,
    person2: Rc<Person>,
    person3: Rc<Person>,
}

impl Fixture {
    fn new() -> Self {
        Self { person1: Person::new("One", 1), person2: Person::new("Two", 2), person3: Person::new("Three", 3) }
    }

    fn source(&self) -> ObservableCollection<Rc<Person>> {
        ObservableCollection::from_items(vec![self.person1.clone(), self.person2.clone(), self.person3.clone()])
    }
}

#[test]
fn can_convert_to_observable_change_set_list() {
    let fixture = Fixture::new();
    let source = fixture.source();
    let change_set = source.to_observable_change_set().as_observable_list();

    assert_equivalent(&change_set.items(), &source.to_vec());
}

#[test]
fn can_convert_to_observable_change_set_cache() {
    let fixture = Fixture::new();
    let source = fixture.source();
    let change_set = source.to_observable_change_set_with_key(|x: &Rc<Person>| x.name()).as_observable_cache();

    assert_equivalent(&change_set.items(), &source.to_vec());

    let one = change_set.lookup(&"One".to_owned()).expect("One");
    assert_eq!(one, fixture.person1);
}

#[test]
fn replacing_an_item_with_same_produces_update() {
    let fixture = Fixture::new();
    let source = fixture.source();
    let aggregator = source.to_observable_change_set_with_key(|x: &Rc<Person>| x.name()).as_aggregator();

    source.set(0, Person::new("One", 100));

    assert_eq!(aggregator.summary().latest.updates, 1);
    assert_eq!(aggregator.summary().latest.adds, 0);
    assert_eq!(aggregator.summary().latest.removes, 0);
}
