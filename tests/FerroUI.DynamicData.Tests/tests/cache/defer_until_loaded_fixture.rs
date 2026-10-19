//! `DeferUntilLoadedFixture` (`DynamicData.Tests/Cache/DeferUntilLoadedFixture.cs`, the class
//! `DeferAnsdSkipFixture`): `DeferUntilLoaded` and `SkipInitial`.

use crate::domain::Person;
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::ObservableExt;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn defer_until_loaded_does_nothing_until_data_has_been_received() {
    let update_received = Rc::new(Cell::new(false));
    let result: Rc<RefCell<Option<ChangeSet<Rc<Person>, String>>>> = Rc::new(RefCell::new(None));

    let cache = SourceCache::new(|p: &Rc<Person>| p.name());

    let defer_stream = {
        let (update_received, result) = (update_received.clone(), result.clone());
        cache.connect().defer_until_loaded().subscribe_next(move |changes: ChangeSet<Rc<Person>, String>| {
            update_received.set(true);
            *result.borrow_mut() = Some(changes);
        })
    };

    let person = Person::new("Test", 1);
    assert!(!update_received.get());
    cache.add_or_update(person.clone());

    assert!(update_received.get());

    let result = result.borrow().clone().expect("result");

    assert_eq!(result.adds(), 1);
    assert_eq!(result.first().expect("a change").current(), &person);
    defer_stream.dispose();
}

#[test]
fn skip_initial_does_not_return_the_first_batch_of_data() {
    let update_received = Rc::new(Cell::new(false));

    let cache = SourceCache::new(|p: &Rc<Person>| p.name());

    let defer_stream = {
        let update_received = update_received.clone();
        cache.connect().skip_initial().subscribe_next(move |_changes: ChangeSet<Rc<Person>, String>| update_received.set(true))
    };

    assert!(!update_received.get());

    cache.add_or_update(Person::new("P1", 1));

    assert!(!update_received.get());

    cache.add_or_update(Person::new("P2", 2));
    assert!(update_received.get());
    defer_stream.dispose();
}
