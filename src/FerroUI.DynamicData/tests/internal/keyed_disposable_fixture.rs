//! `KeyedDisposableFixture` (`DynamicData.Tests/Internal/KeyedDisposableFixture.cs`).
//!
//! Upstream's `Add<TItem>` tests whether the item is an `IDisposable`; here the item is given as a
//! disposable or as `None` (an item that is not one, upstream's `"not disposable"`).

use super::panics;
use crate::internal::KeyedDisposable;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// `TestDisposable`: a disposable that runs `on_dispose` each time it is disposed.
struct TestDisposable {
    on_dispose: Box<dyn Fn()>,
}

impl TestDisposable {
    fn new(on_dispose: impl Fn() + 'static) -> Option<Rc<dyn IDisposable>> {
        let disposable: Rc<dyn IDisposable> = Rc::new(Self { on_dispose: Box::new(on_dispose) });
        Some(disposable)
    }
}

impl IDisposable for TestDisposable {
    fn dispose(&self) {
        (self.on_dispose)();
    }
}

fn flag() -> (Rc<Cell<bool>>, impl Fn() + 'static) {
    let flag = Rc::new(Cell::new(false));
    let target = flag.clone();
    (flag, move || target.set(true))
}

#[test]
fn add_tracks_disposable() {
    let tracker = KeyedDisposable::<String>::new();
    let (disposed, on_dispose) = flag();

    tracker.add("key".to_owned(), TestDisposable::new(on_dispose));

    assert!(tracker.contains_key(&"key".to_owned()));
    assert!(!disposed.get());
}

#[test]
fn remove_disposes_item() {
    let tracker = KeyedDisposable::<String>::new();
    let (disposed, on_dispose) = flag();
    tracker.add("key".to_owned(), TestDisposable::new(on_dispose));

    tracker.remove(&"key".to_owned());

    assert!(disposed.get());
    assert!(!tracker.contains_key(&"key".to_owned()));
}

#[test]
fn add_with_same_key_dispose_previous() {
    let tracker = KeyedDisposable::<String>::new();
    let (disposed1, on_dispose1) = flag();
    let (disposed2, on_dispose2) = flag();
    tracker.add("key".to_owned(), TestDisposable::new(on_dispose1));

    tracker.add("key".to_owned(), TestDisposable::new(on_dispose2));

    assert!(disposed1.get(), "previous item should be disposed");
    assert!(!disposed2.get(), "new item should not be disposed");
}

#[test]
fn add_with_same_reference_does_not_dispose() {
    let tracker = KeyedDisposable::<String>::new();
    let dispose_count = Rc::new(Cell::new(0));
    let count = dispose_count.clone();
    let item = TestDisposable::new(move || count.set(count.get() + 1));

    tracker.add("key".to_owned(), item.clone());
    // The same reference.
    tracker.add("key".to_owned(), item);

    assert_eq!(dispose_count.get(), 0, "same reference should not be disposed");
    assert!(tracker.contains_key(&"key".to_owned()));
}

#[test]
fn dispose_disposes_all_items() {
    let tracker = KeyedDisposable::<i32>::new();
    let disposed_count = Rc::new(Cell::new(0));
    for i in 0..5 {
        let count = disposed_count.clone();
        tracker.add(i, TestDisposable::new(move || count.set(count.get() + 1)));
    }

    tracker.dispose();

    assert_eq!(disposed_count.get(), 5);
    assert!(tracker.is_disposed());
}

#[test]
fn dispose_is_idempotent() {
    let tracker = KeyedDisposable::<String>::new();
    let dispose_count = Rc::new(Cell::new(0));
    let count = dispose_count.clone();
    tracker.add("key".to_owned(), TestDisposable::new(move || count.set(count.get() + 1)));

    tracker.dispose();
    tracker.dispose();

    assert_eq!(dispose_count.get(), 1);
}

#[test]
fn add_after_dispose_disposes_immediately() {
    let tracker = KeyedDisposable::<String>::new();
    tracker.dispose();

    let (disposed, on_dispose) = flag();
    tracker.add("key".to_owned(), TestDisposable::new(on_dispose));

    assert!(disposed.get(), "item added after Dispose should be disposed immediately");
}

#[test]
fn dispose_aggregates_exceptions() {
    let tracker = KeyedDisposable::<i32>::new();
    let disposed_count = Rc::new(Cell::new(0));
    let count = disposed_count.clone();
    tracker.add(1, TestDisposable::new(|| panic!("boom1")));
    tracker.add(2, TestDisposable::new(move || count.set(count.get() + 1)));
    tracker.add(3, TestDisposable::new(|| panic!("boom3")));

    // The panics are reported as one, after every item has been disposed (`AggregateException`).
    assert!(panics(|| tracker.dispose()));
    assert_eq!(disposed_count.get(), 1);
    assert_eq!(tracker.count(), 0, "all items should be cleared even after exceptions");
}

#[test]
fn add_non_disposable_tracks_nothing() {
    let tracker = KeyedDisposable::<String>::new();

    tracker.add("key".to_owned(), None);

    assert!(!tracker.contains_key(&"key".to_owned()));
}

#[test]
fn add_non_disposable_removes_previous() {
    let tracker = KeyedDisposable::<String>::new();
    let (disposed, on_dispose) = flag();
    tracker.add("key".to_owned(), TestDisposable::new(on_dispose));

    tracker.add("key".to_owned(), None);

    assert!(disposed.get(), "previous disposable should be disposed");
    assert!(!tracker.contains_key(&"key".to_owned()));
}

#[test]
fn remove_non_existent_key_is_no_op() {
    let tracker = KeyedDisposable::<String>::new();
    // Does not panic.
    tracker.remove(&"nonexistent".to_owned());
    let _unused: Rc<dyn IDisposable> = Disposable::empty();
}
