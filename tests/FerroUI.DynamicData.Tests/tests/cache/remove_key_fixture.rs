//! `RemoveKeyFixture` (`DynamicData.Tests/Cache/RemoveKeyFixture.cs`): the changes of a cache as
//! the changes of a list, without their keys (`RemoveKey`).

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::{IConnectableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_dynamic_data::list::ObservableListEx;
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// The fixture (`RemoveKeyFixture` with its constructor and `Dispose`): the cleanup holds the
/// source first, then the subscriptions of the tests (`CompositeDisposable`).
struct Fixture {
    generator: RandomPersonGenerator,
    source: SourceCache<Rc<Person>, String>,
    cleanup: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl Fixture {
    fn new() -> Self {
        Self { generator: RandomPersonGenerator::new(), source: SourceCache::new(|p: &Rc<Person>| p.key()), cleanup: RefCell::new(Vec::new()) }
    }

    /// `_cleanup.Add(disposable)`.
    fn add_cleanup(&self, disposable: Rc<dyn IDisposable>) {
        self.cleanup.borrow_mut().push(disposable);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        for disposable in self.cleanup.borrow_mut().drain(..) {
            disposable.dispose();
        }
    }
}

#[test]
fn cache_remove_key_add_key_is_removed() {
    let fixture = Fixture::new();

    let (bound, collection) = fixture.source.connect().remove_key().bind_read_only();
    fixture.add_cleanup(bound.subscribe_ignore());
    let people = fixture.generator.take(100);
    fixture.source.add_or_update_items(people.iter().cloned());

    assert_equivalent(&people, &collection.to_vec());
}

#[test]
fn cache_remove_key_filter_items_filter_key_is_removed() {
    let fixture = Fixture::new();

    let people = fixture.generator.take(100);
    let average = people.iter().map(|x| f64::from(x.age())).sum::<f64>() / people.len() as f64;

    let (bound, collection) = fixture.source.connect().remove_key().filter(move |x: &Rc<Person>| f64::from(x.age()) < average).bind_read_only();
    fixture.add_cleanup(bound.subscribe_ignore());
    fixture.source.add_or_update_items(people.iter().cloned());

    let expected: Vec<Rc<Person>> = people.iter().filter(|x| f64::from(x.age()) < average).cloned().collect();
    assert_equivalent(&expected, &collection.to_vec());
}

#[test]
fn cache_remove_key_auto_refresh_update_i_tems_collection_updated() {
    let fixture = Fixture::new();

    let (bound, collection) = fixture.source.connect().auto_refresh(ferroui_system::prop!(Person => age)).remove_key().bind_read_only();
    fixture.add_cleanup(bound.subscribe_ignore());
    let people = fixture.generator.take(100);
    fixture.source.add_or_update_items(people.iter().cloned());

    assert_equivalent(&people, &collection.to_vec());

    for person in &people {
        person.set_age(person.age() + 1);
    }
    assert_equivalent(&people, &collection.to_vec());
}
