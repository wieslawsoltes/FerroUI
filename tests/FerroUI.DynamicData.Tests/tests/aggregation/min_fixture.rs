//! `MinFixture` (`DynamicData.Tests/AggregationTests/MinFixture.cs`): the running minimum of the
//! ages of a cache (`Minimum`), invalidated by an inline change (`InvalidateWhen`).

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
fn added_items_contribute_to_sum() {
    let fixture = Fixture::new();
    let result = Rc::new(Cell::new(0));

    let accumulator = {
        let result = result.clone();
        fixture.source.connect().minimum(|p: &Rc<Person>| p.age()).subscribe_next(move |x: i32| result.set(x))
    };

    fixture.source.add_or_update(Person::new("A", 10));
    fixture.source.add_or_update(Person::new("B", 20));
    fixture.source.add_or_update(Person::new("C", 30));

    assert_eq!(result.get(), 10, "Min value should be 10");

    accumulator.dispose();
}

#[test]
fn inline_change_re_evaluates_totals() {
    let fixture = Fixture::new();
    let min = Rc::new(Cell::new(0.0f64));

    let someprop_changed = fixture.source.connect().when_value_changed(prop!(Person => age));

    let accumulator = {
        let min = min.clone();
        fixture
            .source
            .connect()
            .minimum(|p: &Rc<Person>| p.age())
            .invalidate_when(someprop_changed)
            .subscribe_next(move |x: i32| min.set(f64::from(x)))
    };

    let personc = Person::new("C", 5);
    fixture.source.add_or_update(Person::new("A", 10));
    fixture.source.add_or_update(Person::new("B", 11));
    fixture.source.add_or_update(personc.clone());
    assert_eq!(min.get(), 5.0);

    fixture.source.add_or_update(personc.clone());

    personc.set_age(11);

    assert_eq!(min.get(), 10.0);
    accumulator.dispose();
}

#[test]
fn remove_produce_correct_result() {
    let fixture = Fixture::new();
    let result = Rc::new(Cell::new(0));

    let accumulator = {
        let result = result.clone();
        fixture.source.connect().minimum(|p: &Rc<Person>| p.age()).subscribe_next(move |x: i32| result.set(x))
    };

    fixture.source.add_or_update(Person::new("A", 10));
    fixture.source.add_or_update(Person::new("B", 20));
    fixture.source.add_or_update(Person::new("C", 30));

    fixture.source.remove_key("A".to_owned());
    assert_eq!(result.get(), 20, "Min value should be 20 after remove");
    accumulator.dispose();
}
