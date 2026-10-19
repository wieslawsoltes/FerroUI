//! `DisposeManyFixture` (`DynamicData.Tests/Cache/DisposeManyFixture.cs`): the items of a cache are
//! disposed when they are removed or replaced, and the remaining ones when the sequence ends, after
//! the notifications downstream have been processed.

use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
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

/// `DisposableObject`: an item that records whether it has been disposed.
struct DisposableObject {
    id: i32,
    is_disposed: Cell<bool>,
}

impl DisposableObject {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id, is_disposed: Cell::new(false) })
    }

    fn id(&self) -> i32 {
        self.id
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

type Item = Rc<DisposableObject>;

const NOT_DISPOSED_YET: &str = "items should not be disposed until after downstream notifications are processed";

struct Fixture {
    change_sets_source: Rc<Subject<ChangeSet<Item, i32>>>,
    items_source: SourceCache<Item, i32>,
    results: ChangeSetAggregator<Item, i32>,
}

impl Fixture {
    fn new() -> Self {
        let change_sets_source = Subject::<ChangeSet<Item, i32>>::new();
        let items_source = SourceCache::new(|item: &Item| item.id());
        let on_error_source = items_source.clone();
        let on_completed_source = items_source.clone();
        let results = change_sets_source
            .as_observable()
            .merge(items_source.connect())
            .dispose_many()
            .do_all(
                |change_set: &ChangeSet<Item, i32>| {
                    for change in change_set {
                        assert!(!change.current().is_disposed(), "{NOT_DISPOSED_YET}");
                        if let Some(previous) = change.previous() {
                            assert!(!previous.is_disposed(), "{NOT_DISPOSED_YET}");
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

    /// `_itemsSource.AddOrUpdate(item)`.
    fn add_or_update(&self, item: Item) {
        self.items_source.edit(move |updater| updater.add_or_update(item));
    }

    /// `_itemsSource.AddOrUpdate(items)`.
    fn add_or_update_items(&self, items: &[Item]) {
        self.items_source.edit(|updater| updater.add_or_update_items(&mut items.iter().cloned()));
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

    let source = Observable::throw::<ChangeSet<Item, i32>>(error.clone()).dispose_many();

    // `Subscribe()` without an error handler throws the error; here it panics with it.
    let thrown = catch_unwind(AssertUnwindSafe(|| {
        source.subscribe_ignore();
    }));
    let payload = thrown.expect_err("subscribing without an error handler should fail with the error");
    assert!(panic_message(payload.as_ref()).contains("Test Exception"), "the error should be the one of the source");

    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let received = received_error.clone();
    let _subscription = source.subscribe_next_error(|_: ChangeSet<Item, i32>| {}, move |error| *received.borrow_mut() = Some(error));
    let received_error = received_error.borrow().clone().expect("the error should be received");
    assert!(Rc::ptr_eq(&received_error, &error));
}

#[test]
fn items_are_disposed_after_removal_or_replacement() {
    let fixture = Fixture::new();
    let items: Vec<Item> = [1, 2, 3, 4, 5, 1, 6, 7, 8].into_iter().map(DisposableObject::new).collect();

    // Exercise a variety of types of changesets.
    fixture.add_or_update(items[0].clone()); // Single add
    fixture.add_or_update_items(&items[1..5]); // Range add
    fixture.add_or_update(items[5].clone()); // Replace
    fixture.add_or_update(items[5].clone()); // Redundant update
    fixture.items_source.edit(|updater| updater.remove_key(&4)); // Single remove
    fixture.items_source.edit(|updater| updater.remove_keys(&mut [1, 2].into_iter())); // Range remove
    fixture.items_source.edit(|updater| updater.clear()); // Clear
    fixture.add_or_update_items(&items[6..9]);

    // Refresh
    let first = fixture.items_source.items()[0].clone();
    fixture.change_sets_source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Refresh, first.id(), first.clone())]));

    // Move
    let first = fixture.items_source.items()[0].clone();
    fixture.change_sets_source.on_next(ChangeSet::from(vec![Change::moved(first.id(), first.clone(), 1, 0)]));

    assert!(fixture.results.error().is_none());
    assert_eq!(fixture.results.messages().len(), 10, "10 updates were made to the source");
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
    fixture.add_or_update_items(&[DisposableObject::new(1), DisposableObject::new(2), DisposableObject::new(3)]);

    fixture.items_source.dispose();
    fixture.change_sets_source.on_completed();

    assert!(fixture.results.error().is_none());
    assert_eq!(fixture.results.messages().len(), 1, "1 update was made to the source");
    assert_eq!(fixture.results.data().count(), 3, "3 items were not removed from the list");
    assert!(fixture.results.data().items().iter().all(|item| item.is_disposed()), "Items remaining in the list should be disposed");
}

#[test]
fn remaining_items_are_disposed_after_error() {
    let fixture = Fixture::new();
    fixture.add_or_update(DisposableObject::new(1));

    let error: ObservableError = Rc::new(TestError);
    fixture.change_sets_source.on_error(error.clone());

    fixture.add_or_update(DisposableObject::new(2));

    let received_error = fixture.results.error().expect("the error should be propagated");
    assert!(Rc::ptr_eq(&received_error, &error));
    assert_eq!(fixture.results.messages().len(), 1, "1 update was made to the source");
    assert_eq!(fixture.results.data().count(), 1, "1 item was not removed from the list");
    assert!(fixture.results.data().items().iter().all(|item| item.is_disposed()), "items remaining in the list should be disposed");
}

#[test]
fn remaining_items_are_disposed_after_unsubscription() {
    let fixture = Fixture::new();
    let items = vec![DisposableObject::new(1), DisposableObject::new(2), DisposableObject::new(3)];

    fixture.add_or_update_items(&items);

    fixture.results.dispose();

    assert!(items.iter().all(|item| item.is_disposed()), "Items remaining in the list should be disposed");
}
