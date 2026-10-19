//! `TransformImmutableFixture` (`DynamicData.Tests/Cache/TransformImmutableFixture.cs`): the transform
//! that keeps every part of each change and caches nothing.

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, IObservableCache, ObservableCacheEx};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::Observable;
use ferroui_reactive_testing::new_error;
use ferroui_system::{IObserver, ObservableError};
use std::rc::Rc;

/// `Item`.
#[derive(Debug, PartialEq)]
struct Item {
    id: i32,
    name: String,
}

impl Item {
    fn new(id: i32, name: &str) -> Rc<Self> {
        Rc::new(Self { id, name: name.to_owned() })
    }

    /// `Item.NameSelector`.
    fn name_selector(item: &Rc<Item>) -> String {
        item.name.clone()
    }
}

fn current_indexes<T, K>(changes: &ChangeSet<T, K>) -> Vec<Option<usize>> {
    changes.iter().map(|change| change.current_index()).collect()
}

fn previous_indexes<T, K>(changes: &ChangeSet<T, K>) -> Vec<Option<usize>> {
    changes.iter().map(|change| change.previous_index()).collect()
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn items_are_manipulated_items_are_transformed() {
    let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();

    let results = source.as_observable().transform_immutable(Item::name_selector).as_aggregator();

    // Additions
    let item1 = Item::new(1, "Item #1");
    let item2 = Item::new(2, "Item #2");
    let operation1 = ChangeSet::from(vec![
        Change::with_index(ChangeReason::Add, item1.id, item1.clone(), Some(0)),
        Change::with_index(ChangeReason::Add, item2.id, item2.clone(), Some(1)),
    ]);
    source.on_next(operation1.clone());

    assert!(results.error().is_none());
    assert_eq!(results.messages().len(), 1, "1 source operation was performed");
    assert_equivalent(&current_indexes(&results.messages()[0]), &current_indexes(&operation1));
    assert_equivalent(&previous_indexes(&results.messages()[0]), &previous_indexes(&operation1));
    assert_equivalent(&results.data().items(), &names(&[item1.name.as_str(), item2.name.as_str()]));

    // Replace items, changing inclusion
    let item3 = Item::new(item1.id, "Item #3");
    let item4 = Item::new(item2.id, "Item #4");
    let operation2 = ChangeSet::from(vec![
        Change::with_previous(ChangeReason::Update, item3.id, item3.clone(), Some(item1.clone()), Some(0), Some(0)),
        Change::with_previous(ChangeReason::Update, item4.id, item4.clone(), Some(item2.clone()), Some(1), Some(1)),
    ]);
    source.on_next(operation2.clone());

    assert!(results.error().is_none());
    assert_eq!(results.messages().len() - 1, 1, "1 source operation was performed");
    assert_equivalent(&current_indexes(&results.messages()[1]), &current_indexes(&operation2));
    assert_equivalent(&previous_indexes(&results.messages()[1]), &previous_indexes(&operation2));
    assert_equivalent(&results.data().items(), &names(&[item3.name.as_str(), item4.name.as_str()]));

    // Refresh items
    let operation3 = ChangeSet::from(vec![
        Change::with_index(ChangeReason::Refresh, item3.id, item3.clone(), Some(0)),
        Change::with_index(ChangeReason::Refresh, item4.id, item4.clone(), Some(1)),
    ]);
    source.on_next(operation3.clone());

    assert!(results.error().is_none());
    assert_eq!(results.messages().len() - 2, 1, "1 source operation was performed");
    assert_equivalent(&current_indexes(&results.messages()[2]), &current_indexes(&operation3));
    assert_equivalent(&previous_indexes(&results.messages()[2]), &previous_indexes(&operation3));
    assert_equivalent(&results.data().items(), &names(&[item3.name.as_str(), item4.name.as_str()]));

    // Move items
    let operation4 = ChangeSet::from(vec![
        Change::with_previous(ChangeReason::Moved, item3.id, item3.clone(), None, Some(1), Some(0)),
        Change::with_previous(ChangeReason::Moved, item4.id, item4.clone(), None, Some(1), Some(0)),
    ]);
    source.on_next(operation4.clone());

    assert!(results.error().is_none());
    assert_eq!(results.messages().len() - 3, 1, "1 source operation was performed");
    assert_equivalent(&current_indexes(&results.messages()[3]), &current_indexes(&operation4));
    assert_equivalent(&previous_indexes(&results.messages()[3]), &previous_indexes(&operation4));
    assert_equivalent(&results.data().items(), &names(&[item4.name.as_str(), item3.name.as_str()]));

    // Remove items
    let operation5 = ChangeSet::from(vec![
        Change::with_index(ChangeReason::Remove, item3.id, item3.clone(), Some(0)),
        Change::with_index(ChangeReason::Remove, item4.id, item4.clone(), Some(1)),
    ]);
    source.on_next(operation5.clone());

    assert!(results.error().is_none());
    assert_eq!(results.messages().len() - 4, 1, "1 source operation was performed");
    assert_equivalent(&current_indexes(&results.messages()[4]), &current_indexes(&operation5));
    assert_equivalent(&previous_indexes(&results.messages()[4]), &previous_indexes(&operation5));
    assert!(results.data().items().is_empty(), "2 items were removed");

    assert!(!results.is_completed());
}

#[test]
fn source_completes_completion_is_propagated() {
    let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();

    let results = source.as_observable().transform_immutable(Item::name_selector).as_aggregator();

    let item1 = Item::new(1, "Item #1");
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item1.id, item1.clone())]));
    source.on_completed();

    assert!(results.error().is_none());
    assert!(results.is_completed());
    assert_eq!(results.messages().len(), 1, "1 source operation was performed");
    assert_equivalent(&results.data().items(), &names(&[item1.name.as_str()]));

    // Make sure no extraneous notifications are published.
    let item2 = Item::new(2, "Item #2");
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item2.id, item2.clone())]));

    assert_eq!(results.messages().len(), 1, "no source operations should have been processed");
}

#[test]
fn source_completes_immediately_completion_is_propagated() {
    let item1 = Item::new(1, "Item #1");

    let emitted = item1.clone();
    let source = Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<Item>, i32>>>| {
        observer.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, emitted.id, emitted.clone())]));
        observer.on_completed();
        Disposable::empty()
    });

    let results = source.transform_immutable(Item::name_selector).as_aggregator();

    assert!(results.error().is_none());
    assert!(results.is_completed());
    assert_eq!(results.messages().len(), 1, "1 source operation was performed");
    assert_equivalent(&results.data().items(), &names(&[item1.name.as_str()]));
}

#[test]
fn source_errors_error_is_propagated() {
    let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();

    let error = new_error("Exception");

    let results = source.as_observable().transform_immutable(Item::name_selector).as_aggregator();

    let item1 = Item::new(1, "Item #1");
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item1.id, item1.clone())]));
    source.on_error(error.clone());

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
    assert!(!results.is_completed());
    assert_eq!(results.messages().len(), 1, "1 source operation was performed");
    assert_equivalent(&results.data().items(), &names(&[item1.name.as_str()]));

    // Make sure no extraneous notifications are published.
    let item2 = Item::new(2, "Item #2");
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item2.id, item2.clone())]));

    assert_eq!(results.messages().len(), 1, "no source operations should have been processed");
}

#[test]
fn source_errors_immediately_error_is_propagated() {
    let item1 = Item::new(1, "Item #1");
    let error = new_error("Exception");

    let (emitted, raised) = (item1.clone(), error.clone());
    let source = Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<Item>, i32>>>| {
        observer.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, emitted.id, emitted.clone())]));
        observer.on_error(raised.clone());
        Disposable::empty()
    });

    let results = source.transform_immutable(Item::name_selector).as_aggregator();

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
    assert!(!results.is_completed());
    assert_eq!(results.messages().len(), 1, "1 source operation was performed");
    assert_equivalent(&results.data().items(), &names(&[item1.name.as_str()]));
}

// Not ported: SourceIsNull_ThrowsException - argument validation
// Not ported: TransformFactoryIsNull_ThrowsException - argument validation

#[test]
fn transform_factory_throws_exception_is_captured() {
    let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();

    let error = new_error("Exception");

    let raised = error.clone();
    let results = source.as_observable().try_transform_immutable(move |_: &Rc<Item>| -> Result<String, ObservableError> { Err(raised.clone()) }).as_aggregator();

    let item1 = Item::new(1, "Item #1");
    source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, item1.id, item1.clone())]));

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
    assert!(results.messages().is_empty(), "no source operations should have been processed");
    assert!(!results.is_completed());
}

// https://github.com/reactivemarbles/DynamicData/issues/925
#[test]
fn t_destination_is_value_type_does_not_throw_exception() {
    let source = Subject::<ChangeSet<String, String>>::new();

    let results = source.as_observable().transform_immutable(|value: &String| value.len()).as_aggregator();

    source.on_next(ChangeSet::from(vec![Change::with_index(ChangeReason::Add, "Item #1".to_owned(), "Item #1".to_owned(), Some(0))]));

    assert!(results.error().is_none());
    assert_eq!(results.messages().len(), 1, "1 source operation was performed");
}
