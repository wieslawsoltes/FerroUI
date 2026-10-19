//! `DisposeManyFixture` (`DynamicData.Tests/List/DisposeManyFixture.cs`): the items of a list are
//! disposed when they are removed or replaced, and the remaining ones when the sequence ends, after
//! the notifications downstream have been processed.

use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{Change, ChangeSet, IObservableList, ListChangeReason, ObservableListEx, SourceList};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// The error of the tests (`new Exception("Test Exception")`).
#[derive(Debug)]
struct TestError;

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Test Exception")
    }
}

impl std::error::Error for TestError {}

/// `DisposableObject`: an item that records whether it has been disposed. It has no `Equals`, so
/// it is equal only to itself.
struct DisposableObject {
    #[allow(dead_code)]
    id: i32,
    is_disposed: Cell<bool>,
}

impl DisposableObject {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id, is_disposed: Cell::new(false) })
    }

    fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }
}

impl IDisposable for DisposableObject {
    fn dispose(&self) {
        self.is_disposed.set(true);
    }
}

impl PartialEq for DisposableObject {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

type Item = Rc<DisposableObject>;

const NOT_DISPOSED_YET: &str = "items should not be disposed until after downstream notifications are processed";

struct Fixture {
    change_sets_source: Rc<Subject<ChangeSet<Item>>>,
    items_source: SourceList<Item>,
    results: ChangeSetAggregator<Item>,
}

impl Fixture {
    fn new() -> Self {
        let change_sets_source = Subject::<ChangeSet<Item>>::new();
        let items_source = SourceList::<Item>::new();
        let on_error_source = items_source.clone();
        let on_completed_source = items_source.clone();
        let results = change_sets_source
            .as_observable()
            .merge(items_source.connect())
            .dispose_many()
            .do_all(
                |change_set: &ChangeSet<Item>| {
                    for change in change_set {
                        if let Some(item) = change.try_item() {
                            assert!(!item.current().is_disposed(), "{NOT_DISPOSED_YET}");

                            if let Some(previous) = item.previous() {
                                assert!(!previous.is_disposed(), "{NOT_DISPOSED_YET}");
                            }
                        }

                        for item in change.range().iter() {
                            assert!(!item.is_disposed(), "{NOT_DISPOSED_YET}");
                        }
                    }
                },
                move |_error: ObservableError| {
                    for item in on_error_source.items() {
                        assert!(!item.is_disposed(), "{NOT_DISPOSED_YET}");
                    }
                },
                move || {
                    for item in on_completed_source.items() {
                        assert!(!item.is_disposed(), "{NOT_DISPOSED_YET}");
                    }
                },
            )
            .as_aggregator();
        Self { change_sets_source, items_source, results }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.change_sets_source.dispose();
        self.items_source.dispose();
        self.results.dispose();
    }
}

/// The text of a panic.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        String::new()
    }
}

// Verifies https://github.com/reactivemarbles/DynamicData/issues/668
//
// Upstream's change sets are of `object`s; here they are of the disposable items of the fixture,
// which `DisposeMany` requires to be `AsDisposable`.
#[test]
fn errors_are_propagated() {
    let error: ObservableError = Rc::new(TestError);

    let source = Observable::throw::<ChangeSet<Item>>(error.clone()).dispose_many();

    // `Subscribe()` without an error handler throws the error; here it panics with it.
    let thrown = catch_unwind(AssertUnwindSafe(|| {
        source.subscribe_ignore();
    }));
    let payload = thrown.expect_err("subscribing without an error handler should fail with the error");
    assert!(panic_message(payload.as_ref()).contains("Test Exception"), "the error should be the one of the source");

    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let received = received_error.clone();
    let _subscription = source.subscribe_next_error(|_: ChangeSet<Item>| {}, move |error| *received.borrow_mut() = Some(error));
    let received_error = received_error.borrow().clone().expect("the error should be received");
    assert!(Rc::ptr_eq(&received_error, &error));
}

#[test]
fn items_are_disposed_after_removal_or_replacement() {
    let fixture = Fixture::new();
    let items: Vec<Item> = (1..=10).map(DisposableObject::new).collect();

    // Exercise a variety of types of changesets.
    let item = items[0].clone();
    fixture.items_source.edit(move |list| list.add(item)); // Trivial single add
    let range = items[1..3].to_vec();
    fixture.items_source.edit(move |list| list.add_range(range)); // Trivial range add
    let item = items[3].clone();
    fixture.items_source.edit(move |list| list.insert(1, item)); // Non-trivial single add
    let range = items[4..6].to_vec();
    fixture.items_source.edit(move |list| list.insert_range(range, 2)); // Non-trivial range add
    fixture.items_source.edit(|list| list.remove_at(3)); // Single remove
    fixture.items_source.edit(|list| list.remove_range(2, 2)); // Range remove
    let item = items[6].clone();
    fixture.items_source.edit(move |list| list.set(1, item)); // Replace
    fixture.items_source.edit(|list| list.move_index(1, 0)); // Move
    fixture.items_source.edit(|list| list.clear()); // Clear
    let range = items[7..10].to_vec();
    fixture.items_source.edit(move |list| list.add_range(range));

    // Refresh
    let first = fixture.items_source.items()[0].clone();
    fixture.change_sets_source.on_next(ChangeSet::from(vec![Change::new(ListChangeReason::Refresh, first, Some(0))]));

    assert!(fixture.results.error().is_none());
    assert_eq!(fixture.results.messages().len(), 11, "11 updates were made to the source");
    assert_eq!(fixture.results.data().count(), 3, "3 items were not removed from the list");

    let remaining = fixture.results.data().items();
    assert!(!remaining.iter().all(|item| item.is_disposed()), "items remaining in the list should not be disposed");
    assert!(
        items.iter().filter(|item| !remaining.iter().any(|kept| Rc::ptr_eq(kept, item))).all(|item| item.is_disposed()),
        "items removed from the list should be disposed"
    );
}

#[test]
fn remaining_items_are_disposed_after_completed() {
    let fixture = Fixture::new();
    fixture.items_source.edit(|list| list.add_range(vec![DisposableObject::new(1), DisposableObject::new(2), DisposableObject::new(3)]));
    fixture.items_source.dispose();
    fixture.change_sets_source.on_completed();

    assert!(fixture.results.error().is_none());
    assert_eq!(fixture.results.messages().len(), 1, "1 update was made to the list");
    assert_eq!(fixture.results.data().count(), 3, "3 items were not removed from the list");
    assert!(fixture.results.data().items().iter().all(|item| item.is_disposed()), "items remaining in the list should be disposed");
}

#[test]
fn remaining_items_are_disposed_after_error() {
    let fixture = Fixture::new();
    fixture.items_source.edit(|list| list.add(DisposableObject::new(1)));

    let error: ObservableError = Rc::new(TestError);
    fixture.change_sets_source.on_error(error.clone());

    fixture.items_source.edit(|list| list.add(DisposableObject::new(2)));

    let received_error = fixture.results.error().expect("the error should be propagated");
    assert!(Rc::ptr_eq(&received_error, &error));
    assert_eq!(fixture.results.messages().len(), 1, "1 update was made to the list");
    assert_eq!(fixture.results.data().count(), 1, "1 item was not removed from the list");
    assert!(fixture.results.data().items().iter().all(|item| item.is_disposed()), "Items remaining in the list should be disposed");
}

#[test]
fn remaining_items_are_disposed_after_unsubscription() {
    let fixture = Fixture::new();
    let items = vec![DisposableObject::new(1), DisposableObject::new(2), DisposableObject::new(3)];

    let added = items.clone();
    fixture.items_source.edit(move |list| list.add_range(added));

    fixture.results.dispose();

    assert!(items.iter().all(|item| item.is_disposed()), "Items remaining in the list should be disposed");
}
