//! `MergeManyFixture` (`DynamicData.Tests/List/MergeManyFixture.cs`): the values of the sequence of
//! each item of a list, merged, for as long as the item is in the list.

use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `ObjectWithObservable`: an object with a sequence of its own, which the test drives. It is
/// equal only to itself (upstream's class does not override `Equals`).
struct ObjectWithObservable {
    changed: Rc<Subject<bool>>,
    #[allow(dead_code)] // upstream's field, which no test reads
    value: Cell<bool>,
    #[allow(dead_code)] // upstream's field, which no test reads
    id: i32,
}

impl ObjectWithObservable {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { changed: Subject::new(), value: Cell::new(false), id })
    }

    /// `Observable`.
    fn observable(&self) -> Rc<dyn IObservable<bool>> {
        self.changed.clone()
    }

    /// `CompleteObservable`.
    fn complete_observable(&self) {
        self.changed.on_completed();
    }

    /// `FailObservable`.
    fn fail_observable(&self, error: ObservableError) {
        self.changed.on_error(error);
    }

    /// `InvokeObservable`.
    fn invoke_observable(&self, value: bool) {
        self.value.set(value);
        self.changed.on_next(value);
    }
}

impl PartialEq for ObjectWithObservable {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

type Item = Rc<ObjectWithObservable>;

/// The fixture: the source list, disposed at the end of each test.
struct Fixture {
    source: SourceList<Item>,
}

impl Fixture {
    fn new() -> Self {
        Self { source: SourceList::new() }
    }

    fn add(&self, item: &Item) {
        let item = item.clone();
        self.source.edit(move |list| list.add(item));
    }

    fn remove(&self, item: &Item) {
        let item = item.clone();
        self.source.edit(move |list| {
            list.remove(&item);
        });
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

fn select_observable(o: &Item) -> Rc<dyn IObservable<bool>> {
    o.observable()
}

#[test]
fn everything_is_unsubscribed_when_stream_is_disposed() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let on_next = invoked.clone();
    let stream = fixture.source.connect().merge_many(select_observable).subscribe_next(move |_: bool| on_next.set(true));

    let item = ObjectWithObservable::new(1);
    fixture.add(&item);

    stream.dispose();

    item.invoke_observable(true);
    assert!(!invoked.get());
}

/// Invocations the only when child is invoked.
#[test]
fn invocation_only_when_child_is_invoked() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let on_next = invoked.clone();
    let stream = fixture.source.connect().merge_many(select_observable).subscribe_next(move |_: bool| on_next.set(true));

    let item = ObjectWithObservable::new(1);
    fixture.add(&item);

    assert!(!invoked.get());

    item.invoke_observable(true);
    assert!(invoked.get());
    stream.dispose();
}

#[test]
fn removed_item_will_not_cause_invocation() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let on_next = invoked.clone();
    let stream = fixture.source.connect().merge_many(select_observable).subscribe_next(move |_: bool| on_next.set(true));

    let item = ObjectWithObservable::new(1);
    fixture.add(&item);
    fixture.remove(&item);
    assert!(!invoked.get());

    item.invoke_observable(true);
    assert!(!invoked.get());
    stream.dispose();
}

/// Merged stream does not complete if a child stream is still active.
#[test]
fn merged_stream_does_not_complete_while_item_stream_active() {
    let fixture = Fixture::new();
    let stream_completed = Rc::new(Cell::new(false));
    let source_completed = Rc::new(Cell::new(false));

    let item = ObjectWithObservable::new(1);
    fixture.add(&item);

    let (on_source_completed, on_stream_completed) = (source_completed.clone(), stream_completed.clone());
    let stream = fixture
        .source
        .connect()
        .do_with_completed(|_: &ChangeSet<Item>| {}, move || on_source_completed.set(true))
        .merge_many(select_observable)
        .subscribe_next_completed(|_: bool| {}, move || on_stream_completed.set(true));

    fixture.source.dispose();

    assert!(source_completed.get());
    assert!(!stream_completed.get());
    stream.dispose();
}

/// Stream completes only when source and all child are complete.
#[test]
fn merged_stream_completes_when_source_and_items_complete() {
    let fixture = Fixture::new();
    let stream_completed = Rc::new(Cell::new(false));
    let source_completed = Rc::new(Cell::new(false));

    let item = ObjectWithObservable::new(1);
    fixture.add(&item);

    let (on_source_completed, on_stream_completed) = (source_completed.clone(), stream_completed.clone());
    let stream = fixture
        .source
        .connect()
        .do_with_completed(|_: &ChangeSet<Item>| {}, move || on_source_completed.set(true))
        .merge_many(select_observable)
        .subscribe_next_completed(|_: bool| {}, move || on_stream_completed.set(true));

    fixture.source.dispose();
    item.complete_observable();

    assert!(source_completed.get());
    assert!(stream_completed.get());
    stream.dispose();
}

/// Stream completes even if one of the children fails.
#[test]
fn merged_stream_completes_if_last_item_fails() {
    let fixture = Fixture::new();
    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let stream_completed = Rc::new(Cell::new(false));
    let source_completed = Rc::new(Cell::new(false));

    let item = ObjectWithObservable::new(1);
    fixture.add(&item);

    let (on_source_completed, on_stream_completed, on_error) = (source_completed.clone(), stream_completed.clone(), received_error.clone());
    let stream = fixture
        .source
        .connect()
        .do_with_completed(|_: &ChangeSet<Item>| {}, move || on_source_completed.set(true))
        .merge_many(select_observable)
        .subscribe_all(|_: bool| {}, move |error: ObservableError| *on_error.borrow_mut() = Some(error), move || on_stream_completed.set(true));

    fixture.source.dispose();
    item.fail_observable(new_error("Test exception"));

    assert!(received_error.borrow().is_none());
    assert!(source_completed.get());
    assert!(stream_completed.get());
    stream.dispose();
}

/// If the source stream has an error, the merged steam should also.
#[test]
fn merged_stream_fails_when_source_fails() {
    let fixture = Fixture::new();
    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let expected_error = new_error("Test exception");
    let throw_observable = Observable::throw::<ChangeSet<Item>>(expected_error.clone());
    let on_error = received_error.clone();
    let _stream = fixture
        .source
        .connect()
        .concat(throw_observable)
        .merge_many(select_observable)
        .subscribe_next_error(|_: bool| {}, move |error: ObservableError| *on_error.borrow_mut() = Some(error));

    let item = ObjectWithObservable::new(1);
    fixture.add(&item);

    fixture.source.dispose();

    let received = received_error.borrow().clone().expect("the merged stream should have failed");
    assert!(Rc::ptr_eq(&received, &expected_error));
}
