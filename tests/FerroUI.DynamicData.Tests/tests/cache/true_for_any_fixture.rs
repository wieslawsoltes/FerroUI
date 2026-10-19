//! `TrueForAnyFixture` (`DynamicData.Tests/Cache/TrueForAnyFixture.cs`): whether a condition holds
//! for the latest value of an observable of some item (`TrueForAny`).

use crate::utilities::ObservableExtensions;
use ferroui_dynamic_data::cache::{IConnectableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

/// `ObjectWithObservable`: an item with an observable of `bool` and its latest value.
struct ObjectWithObservable {
    id: i32,
    changed: Rc<Subject<bool>>,
    value: Cell<bool>,
}

impl ObjectWithObservable {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id, changed: Subject::new(), value: Cell::new(false) })
    }

    fn id(&self) -> i32 {
        self.id
    }

    fn observable(&self) -> Rc<dyn IObservable<bool>> {
        self.changed.as_observable()
    }

    fn value(&self) -> bool {
        self.value.get()
    }

    fn invoke_observable(&self, value: bool) {
        self.value.set(value);
        self.changed.on_next(value);
    }
}

/// The fixture (`TrueForAnyFixture` with its constructor and `Dispose`).
struct Fixture {
    observable: Rc<dyn IObservable<bool>>,
    source: SourceCache<Rc<ObjectWithObservable>, i32>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<ObjectWithObservable>| p.id());
        let observable = source.connect().true_for_any(|o: &Rc<ObjectWithObservable>| o.observable().start_with([o.value()]), |o: &bool| *o);
        Self { observable, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

/// Subscribes to the result, recording the latest value (`bool? valueReturned`).
fn subscribe_latest(fixture: &Fixture) -> (Rc<dyn IDisposable>, Rc<Cell<Option<bool>>>) {
    let value_returned = Rc::new(Cell::new(None));
    let subscribed = {
        let value_returned = value_returned.clone();
        fixture.observable.subscribe_next(move |result: bool| value_returned.set(Some(result)))
    };
    (subscribed, value_returned)
}

#[test]
fn initial_item_returns_false_when_observale_has_no_value() {
    let fixture = Fixture::new();
    let (subscribed, value_returned) = subscribe_latest(&fixture);

    let item = ObjectWithObservable::new(1);
    fixture.source.add_or_update(item);

    assert!(value_returned.get().is_some());
    assert_eq!(value_returned.get(), Some(false), "The intial value should be false");

    subscribed.dispose();
}

#[test]
fn inline_observable_change_produces_result() {
    let fixture = Fixture::new();
    let (subscribed, value_returned) = subscribe_latest(&fixture);

    let item = ObjectWithObservable::new(1);
    item.invoke_observable(true);
    fixture.source.add_or_update(item);

    assert!(value_returned.get().is_some());
    assert_eq!(value_returned.get(), Some(true), "Value should be true");
    subscribed.dispose();
}

#[test]
fn multiple_values_return_true() {
    let fixture = Fixture::new();
    let (subscribed, value_returned) = subscribe_latest(&fixture);

    let item1 = ObjectWithObservable::new(1);
    let item2 = ObjectWithObservable::new(2);
    let item3 = ObjectWithObservable::new(3);
    fixture.source.add_or_update(item1.clone());
    fixture.source.add_or_update(item2);
    fixture.source.add_or_update(item3);

    let value = value_returned.get().expect("valueReturned");

    assert!(!value, "Value should be false");

    item1.invoke_observable(true);
    assert_eq!(value_returned.get(), Some(true), "Value should be true");
    subscribed.dispose();
}

/// https://github.com/reactivemarbles/DynamicData/issues/922
#[test]
fn values_published_on_subscription_do_not_trigger_premature_output() {
    let fixture = Fixture::new();

    let item1 = ObjectWithObservable::new(1);
    let item2 = ObjectWithObservable::new(2);

    item2.invoke_observable(true);

    fixture.source.add_or_update(item1);
    fixture.source.add_or_update(item2);

    let (subscription, results) = fixture.observable.validate_synchronization().record_values();

    let recorded_values = results.recorded_values();
    assert_eq!(recorded_values.len(), 1, "No items were added to the source, and no value changes were made to the items");
    assert!(recorded_values[0], "One of the two items in the source has a true value");

    subscription.dispose();
}
