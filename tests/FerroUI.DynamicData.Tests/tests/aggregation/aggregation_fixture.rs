//! `AggregationFixture` (`DynamicData.Tests/AggregationTests/AggregationFixture.cs`): an
//! accumulation over the aggregate change sets of a cache (`ForAggregation` with `Scan`).

use crate::domain::Person;
use ferroui_dynamic_data::aggregation::{AggregateType, AggregationCacheEx, IAggregateChangeSet};
use ferroui_dynamic_data::cache::{IConnectableCache, SourceCache, SourceCacheEx};
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable};
use std::cell::Cell;
use std::rc::Rc;

/// The fixture: a cache of people keyed by name, and the sum of their ages accumulated from the
/// aggregate change sets.
struct Fixture {
    accumulator: Rc<dyn IObservable<i32>>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let accumulator = source.connect().for_aggregation().scan_with(0, |current: i32, items: Rc<dyn IAggregateChangeSet<Rc<Person>>>| {
            let mut current = current;
            for x in items.items() {
                if x.aggregate_type() == AggregateType::Add {
                    current += x.item().age();
                } else {
                    current -= x.item().age();
                }
            }
            current
        });
        Self { accumulator, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

/// Subscribes to the accumulator, recording the latest value and the count of values.
fn subscribe(fixture: &Fixture) -> (Rc<dyn IDisposable>, Rc<Cell<i32>>, Rc<Cell<i32>>) {
    let latest = Rc::new(Cell::new(0));
    let counter = Rc::new(Cell::new(0));
    let accumulator = {
        let (latest, counter) = (latest.clone(), counter.clone());
        fixture.accumulator.subscribe_next(move |value: i32| {
            latest.set(value);
            counter.set(counter.get() + 1);
        })
    };
    (accumulator, latest, counter)
}

#[test]
fn can_accumulate() {
    let fixture = Fixture::new();
    let (accumulator, latest, counter) = subscribe(&fixture);

    fixture.source.add_or_update(Person::new("A", 10));
    fixture.source.add_or_update(Person::new("B", 20));
    fixture.source.add_or_update(Person::new("C", 30));

    assert_eq!(counter.get(), 3, "Should be 3 updates");
    assert_eq!(latest.get(), 60, "Accumulated value should be 60");
    fixture.source.add_or_update(Person::new("A", 5));

    accumulator.dispose();
}

#[test]
fn can_handle_updated_item() {
    let fixture = Fixture::new();
    let (accumulator, latest, counter) = subscribe(&fixture);

    fixture.source.add_or_update(Person::new("A", 10));
    fixture.source.add_or_update(Person::new("A", 15));

    assert_eq!(counter.get(), 2, "Should be 2 updates");
    assert_eq!(latest.get(), 15, "Accumulated value should be 60");
    accumulator.dispose();
}
