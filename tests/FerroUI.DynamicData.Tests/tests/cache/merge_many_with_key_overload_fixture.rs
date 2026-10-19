//! `MergeManyWithKeyOverloadFixture` (`DynamicData.Tests/Cache/MergeManyWithKeyOverloadFixture.cs`):
//! `MergeMany` with a selector that takes the key, and when the merged sequence completes or fails.

use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `ObjectWithObservable`: an item with a sequence of its own, which a test can complete or fail.
/// Upstream's private `_value`, which is written and never read, is left out.
struct ObjectWithObservable {
    id: i32,
    changed: Rc<Subject<bool>>,
}

impl ObjectWithObservable {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id, changed: Subject::new() })
    }

    fn id(&self) -> i32 {
        self.id
    }

    /// `Observable`.
    fn observable(&self) -> Rc<dyn IObservable<bool>> {
        self.changed.as_observable()
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
        self.changed.on_next(value);
    }
}

/// The selector of the tests (`(o, key) => o.Observable`).
fn selector(o: &Rc<ObjectWithObservable>, _key: &i32) -> Rc<dyn IObservable<bool>> {
    o.observable()
}

/// The fixture: the source, disposed with the fixture (`Dispose`).
struct Fixture {
    source: SourceCache<Rc<ObjectWithObservable>, i32>,
}

impl Fixture {
    fn new() -> Self {
        Self { source: SourceCache::new(|p: &Rc<ObjectWithObservable>| p.id()) }
    }

    fn add_or_update(&self, item: &Rc<ObjectWithObservable>) {
        let item = item.clone();
        self.source.edit(move |updater| updater.add_or_update(item));
    }

    fn remove(&self, item: &Rc<ObjectWithObservable>) {
        self.source.edit(|updater| updater.remove_item(item));
    }

    /// `_source.Dispose()`.
    fn dispose_source(&self) {
        self.source.dispose();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

#[test]
fn everything_is_unsubscribed_when_stream_is_disposed() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let flag = invoked.clone();
    let stream = fixture.source.connect().merge_many_with_key(selector).subscribe_next(move |_: bool| flag.set(true));

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);

    stream.dispose();

    item.invoke_observable(true);
    assert!(!invoked.get());
}

#[test]
fn invocation_only_when_child_is_invoked() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let flag = invoked.clone();
    let stream = fixture.source.connect().merge_many_with_key(selector).subscribe_next(move |_: bool| flag.set(true));

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);

    assert!(!invoked.get());

    item.invoke_observable(true);
    assert!(invoked.get());
    stream.dispose();
}

#[test]
fn removed_item_will_not_cause_invocation() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let flag = invoked.clone();
    let stream = fixture.source.connect().merge_many_with_key(selector).subscribe_next(move |_: bool| flag.set(true));

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);
    fixture.remove(&item);
    assert!(!invoked.get());

    item.invoke_observable(true);
    assert!(!invoked.get());
    stream.dispose();
}

#[test]
fn single_item_complete_will_not_merged_stream() {
    let fixture = Fixture::new();
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();
    let stream = fixture.source.connect().merge_many_with_key(selector).subscribe_next_completed(|_: bool| {}, move || flag.set(true));

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);

    item.invoke_observable(true);
    item.complete_observable();

    stream.dispose();

    assert!(!completed.get());
}

#[test]
fn single_item_fail_will_not_fail_merged_stream() {
    let fixture = Fixture::new();
    let failed = Rc::new(Cell::new(false));
    let flag = failed.clone();
    let stream = fixture.source.connect().merge_many_with_key(selector).subscribe_next_error(|_: bool| {}, move |_: ObservableError| flag.set(true));

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);

    item.fail_observable(new_error("Test exception"));

    stream.dispose();

    assert!(!failed.get());
}

/// Merged stream does not complete if a child stream is still active.
#[test]
fn merged_stream_does_not_complete_while_item_stream_active() {
    let fixture = Fixture::new();
    let stream_completed = Rc::new(Cell::new(false));
    let source_completed = Rc::new(Cell::new(false));

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);

    let (source_flag, stream_flag) = (source_completed.clone(), stream_completed.clone());
    let stream = fixture
        .source
        .connect()
        .do_with_completed(|_: &ChangeSet<Rc<ObjectWithObservable>, i32>| {}, move || source_flag.set(true))
        .merge_many_with_key(selector)
        .subscribe_next_completed(|_: bool| {}, move || stream_flag.set(true));

    fixture.dispose_source();

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
    fixture.add_or_update(&item);

    let (source_flag, stream_flag) = (source_completed.clone(), stream_completed.clone());
    let stream = fixture
        .source
        .connect()
        .do_with_completed(|_: &ChangeSet<Rc<ObjectWithObservable>, i32>| {}, move || source_flag.set(true))
        .merge_many_with_key(selector)
        .subscribe_next_completed(|_: bool| {}, move || stream_flag.set(true));

    fixture.dispose_source();
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
    fixture.add_or_update(&item);

    let (source_flag, stream_flag, error_target) = (source_completed.clone(), stream_completed.clone(), received_error.clone());
    let stream = fixture
        .source
        .connect()
        .do_with_completed(|_: &ChangeSet<Rc<ObjectWithObservable>, i32>| {}, move || source_flag.set(true))
        .merge_many_with_key(selector)
        .subscribe_all(|_: bool| {}, move |error: ObservableError| *error_target.borrow_mut() = Some(error), move || stream_flag.set(true));

    fixture.dispose_source();
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
    let expected_error: ObservableError = new_error("Test exception");
    let throw_observable = Observable::throw::<ChangeSet<Rc<ObjectWithObservable>, i32>>(expected_error.clone());
    let error_target = received_error.clone();
    let _stream = fixture
        .source
        .connect()
        .concat(throw_observable)
        .merge_many_with_key(selector)
        .subscribe_next_error(|_: bool| {}, move |error: ObservableError| *error_target.borrow_mut() = Some(error));

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);

    fixture.dispose_source();

    let received = received_error.borrow().clone().expect("the merged stream should have failed");
    assert!(Rc::ptr_eq(&received, &expected_error));
}
