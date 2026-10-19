//! `MaxFixture` (`DynamicData.Tests/AggregationTests/MaxFixture.cs`): the running maximum of the
//! ages of a cache (`Maximum`), invalidated by an inline change (`InvalidateWhen`).

use crate::domain::Person;
use ferroui_dynamic_data::aggregation::{AggregationCacheEx, InvalidateWhenEx};
use ferroui_dynamic_data::cache::{IConnectableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::ObservableExt;
use ferroui_system::{prop, IDisposable};
use std::cell::Cell;
use std::rc::Rc;

/// The fixture: a cache of people keyed by name.
struct Fixture {
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        Self { source: SourceCache::new(|p: &Rc<Person>| p.name()) }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

#[test]
fn add_items() {
    let fixture = Fixture::new();
    let result = Rc::new(Cell::new(0));

    let accumulator = {
        let result = result.clone();
        fixture.source.connect().maximum(|p: &Rc<Person>| p.age()).subscribe_next(move |x: i32| result.set(x))
    };

    fixture.source.add_or_update(Person::new("A", 10));
    fixture.source.add_or_update(Person::new("B", 20));
    fixture.source.add_or_update(Person::new("C", 30));

    assert_eq!(result.get(), 30, "Max value should be 30");

    accumulator.dispose();
}

#[test]
fn inline_change_re_evaluates_totals() {
    let fixture = Fixture::new();
    let max = Rc::new(Cell::new(0.0f64));

    let someprop_changed = fixture.source.connect().when_value_changed(prop!(Person => age));

    let accumulator = {
        let max = max.clone();
        fixture
            .source
            .connect()
            .maximum(|p: &Rc<Person>| p.age())
            .invalidate_when(someprop_changed)
            .subscribe_next(move |x: i32| max.set(f64::from(x)))
    };

    let personc = Person::new("C", 5);
    fixture.source.add_or_update(Person::new("A", 10));
    fixture.source.add_or_update(Person::new("B", 11));
    fixture.source.add_or_update(personc.clone());

    assert_eq!(max.get(), 11.0, "Max should be 11");

    personc.set_age(100);

    assert_eq!(max.get(), 100.0, "Max should be 100 after inline change");
    accumulator.dispose();
}

#[test]
fn remove_items() {
    let fixture = Fixture::new();
    let result = Rc::new(Cell::new(0));

    let accumulator = {
        let result = result.clone();
        fixture.source.connect().maximum(|p: &Rc<Person>| p.age()).subscribe_next(move |x: i32| result.set(x))
    };

    fixture.source.add_or_update(Person::new("A", 10));
    fixture.source.add_or_update(Person::new("B", 20));
    fixture.source.add_or_update(Person::new("C", 30));

    fixture.source.remove_key("C".to_owned());
    assert_eq!(result.get(), 20, "Max value should be 20 after remove");
    accumulator.dispose();
}
