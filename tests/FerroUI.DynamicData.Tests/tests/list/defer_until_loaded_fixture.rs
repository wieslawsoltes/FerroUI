//! `DeferUntilLoadedFixture` (`DynamicData.Tests/List/DeferUntilLoadedFixture.cs`, the class
//! `DeferAnsdSkipFixture`): `DeferUntilLoaded` and `SkipInitial`.

use crate::domain::Person;
use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_reactive::ObservableExt;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The item of the first change of `changes` with its ranges spread out (`Unified().First().Current`).
fn first_unified_current<T: Clone>(changes: &ChangeSet<T>) -> T {
    changes
        .iter()
        .find_map(|change| match change.try_item() {
            Some(item) => Some(item.current().clone()),
            None => change.range().items().first().cloned(),
        })
        .expect("the change set has an item")
}

#[test]
fn defer_until_loaded_does_nothing_until_data_has_been_received() {
    let update_received = Rc::new(Cell::new(false));
    let result: Rc<RefCell<Option<ChangeSet<Rc<Person>>>>> = Rc::new(RefCell::new(None));

    let cache = SourceList::<Rc<Person>>::new();

    let defer_stream = {
        let (update_received, result) = (update_received.clone(), result.clone());
        cache.connect().defer_until_loaded().subscribe_next(move |changes: ChangeSet<Rc<Person>>| {
            update_received.set(true);
            *result.borrow_mut() = Some(changes);
        })
    };

    let person = Person::new("Test", 1);

    assert!(!update_received.get());
    cache.add(person.clone());

    assert!(update_received.get());

    // `InvalidOperationException(nameof(result))`.
    let result = result.borrow().clone().expect("result");

    assert_eq!(result.adds(), 1);
    assert_eq!(first_unified_current(&result), person);
    defer_stream.dispose();
}

#[test]
fn skip_initial_does_not_return_the_first_batch_of_data() {
    let update_received = Rc::new(Cell::new(false));

    let cache = SourceList::<Rc<Person>>::new();

    let defer_stream = {
        let update_received = update_received.clone();
        cache.connect().skip_initial().subscribe_next(move |_| update_received.set(true))
    };

    assert!(!update_received.get());

    cache.add(Person::new("P1", 1));

    assert!(!update_received.get());

    cache.add(Person::new("P2", 2));
    assert!(update_received.get());
    defer_stream.dispose();
}
