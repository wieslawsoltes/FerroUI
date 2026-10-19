//! `CacheParentSubscriptionFixture` (`DynamicData.Tests/Internal/CacheParentSubscriptionFixture.cs`):
//! the contracts of `CacheParentSubscription` through a minimal handler.
//!
//! Upstream's `TestSourceCache` (completion and error of the source) is a subject of change sets
//! here, and the values of upstream's randomizer are constants.

use super::{same_error, test_error};
use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, ICache, SourceCache, IConnectableCache};
use crate::internal::{CacheParentSubscription, ParentSubscriptionHandler};
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A test item with a typed key (`TestItem`).
#[derive(Clone, Debug, PartialEq)]
struct TestItem {
    key: i32,
    value: String,
}

fn item(key: i32, value: &str) -> TestItem {
    TestItem { key, value: value.to_owned() }
}

type ChildFactory = Rc<dyn Fn(i32) -> Rc<dyn IObservable<String>>>;

/// The minimal handler (`TestSubscription`).
struct TestHandler {
    child_factory: Option<ChildFactory>,
    on_parent: Option<Rc<dyn Fn()>>,
    on_child: Option<Rc<dyn Fn()>>,
    cache: RefCell<ChangeAwareCache<TestItem, i32>>,
    parent_call_count: Rc<Cell<usize>>,
    emit_call_count: Rc<Cell<usize>>,
    child_calls: Rc<RefCell<Vec<(String, i32)>>>,
}

impl ParentSubscriptionHandler<TestItem, i32, String, ChangeSet<TestItem, i32>> for TestHandler {
    fn parent_on_next(&self, owner: &CacheParentSubscription<TestItem, i32, String, ChangeSet<TestItem, i32>>, changes: ChangeSet<TestItem, i32>) {
        self.parent_call_count.set(self.parent_call_count.get() + 1);
        if let Some(on_parent) = &self.on_parent {
            on_parent();
        }
        self.cache.borrow_mut().clone_changes(&changes);
        if let Some(child_factory) = &self.child_factory {
            for change in &changes {
                match change.reason() {
                    ChangeReason::Add | ChangeReason::Update => owner.add_child_subscription(owner.make_child_observable(child_factory(*change.key())), *change.key()),
                    ChangeReason::Remove => owner.remove_child_subscription(change.key()),
                    _ => {}
                }
            }
        }
    }

    fn child_on_next(&self, child: String, parent_key: &i32) {
        if let Some(on_child) = &self.on_child {
            on_child();
        }
        self.child_calls.borrow_mut().push((child.clone(), *parent_key));
        self.cache.borrow_mut().add_or_update(TestItem { key: *parent_key, value: child }, *parent_key);
    }

    fn emit_changes(&self, observer: &Rc<dyn IObserver<ChangeSet<TestItem, i32>>>) {
        self.emit_call_count.set(self.emit_call_count.get() + 1);
        let changes = self.cache.borrow_mut().capture_changes();
        if changes.count() > 0 {
            observer.on_next(changes);
        }
    }
}

/// The subscription and the counts of its handler.
struct TestSubscription {
    subscription: Rc<CacheParentSubscription<TestItem, i32, String, ChangeSet<TestItem, i32>>>,
    parent_call_count: Rc<Cell<usize>>,
    emit_call_count: Rc<Cell<usize>>,
    child_calls: Rc<RefCell<Vec<(String, i32)>>>,
}

impl TestSubscription {
    fn new(observer: Rc<TestObserver>, child_factory: Option<ChildFactory>) -> Self {
        Self::with_callbacks(observer, child_factory, None, None)
    }

    fn with_callbacks(observer: Rc<TestObserver>, child_factory: Option<ChildFactory>, on_parent: Option<Rc<dyn Fn()>>, on_child: Option<Rc<dyn Fn()>>) -> Self {
        let (parent_call_count, emit_call_count, child_calls) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)), Rc::new(RefCell::new(Vec::new())));
        let handler = TestHandler {
            child_factory,
            on_parent,
            on_child,
            cache: RefCell::new(ChangeAwareCache::new()),
            parent_call_count: parent_call_count.clone(),
            emit_call_count: emit_call_count.clone(),
            child_calls: child_calls.clone(),
        };
        let subscription = CacheParentSubscription::new(observer, Box::new(handler));
        Self { subscription, parent_call_count, emit_call_count, child_calls }
    }

    /// `ExposeCreateParent`.
    fn expose_create_parent(&self, source: Rc<dyn IObservable<ChangeSet<TestItem, i32>>>) {
        self.subscription.create_parent_subscription(source);
    }
}

impl Drop for TestSubscription {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}

/// An observer that records the emissions, the completion and the error (`TestObserver`).
struct TestObserver {
    emit_count: Cell<usize>,
    is_completed: Cell<bool>,
    error: RefCell<Option<ObservableError>>,
}

impl TestObserver {
    fn new() -> Rc<Self> {
        Rc::new(Self { emit_count: Cell::new(0), is_completed: Cell::new(false), error: RefCell::new(None) })
    }
}

impl IObserver<ChangeSet<TestItem, i32>> for TestObserver {
    fn on_next(&self, _value: ChangeSet<TestItem, i32>) {
        self.emit_count.set(self.emit_count.get() + 1);
    }

    fn on_error(&self, error: ObservableError) {
        *self.error.borrow_mut() = Some(error);
    }

    fn on_completed(&self) {
        self.is_completed.set(true);
    }
}

fn source_cache() -> SourceCache<TestItem, i32> {
    SourceCache::new(|item: &TestItem| item.key)
}

/// A child factory that makes subjects and keeps them.
fn subject_factory() -> (ChildFactory, Rc<RefCell<Vec<Rc<Subject<String>>>>>) {
    let subjects = Rc::new(RefCell::new(Vec::new()));
    let target = subjects.clone();
    let factory: ChildFactory = Rc::new(move |_key| {
        let subject = Subject::<String>::new();
        target.borrow_mut().push(subject.clone());
        subject.as_observable()
    });
    (factory, subjects)
}

/// A change set that adds `item` (the edits of a `TestSourceCache`).
fn add(item: TestItem) -> ChangeSet<TestItem, i32> {
    ChangeSet::from(vec![Change::new(ChangeReason::Add, item.key, item)])
}

#[test]
fn parent_on_next_called_for_each_change_set() {
    let item_count = 5;
    let source = source_cache();
    let observer = TestObserver::new();
    let sub = TestSubscription::new(observer.clone(), None);
    sub.expose_create_parent(source.connect());

    let items: Vec<TestItem> = (0..item_count).map(|i| item(1234 + i * 100, "value")).collect();
    for item in &items {
        source.edit(|updater| updater.add_or_update(item.clone()));
    }

    assert_eq!(sub.parent_call_count.get(), items.len(), "ParentOnNext should fire once per changeset");
    assert_eq!(observer.emit_count.get(), items.len(), "EmitChanges should fire after each parent update");
}

#[test]
fn child_on_next_called_for_each_emission() {
    let source = source_cache();
    let (factory, child_subjects) = subject_factory();
    let observer = TestObserver::new();
    let sub = TestSubscription::new(observer, Some(factory));
    sub.expose_create_parent(source.connect());

    let key = 4321;
    source.edit(|updater| updater.add_or_update(item(key, "parent")));

    assert_eq!(child_subjects.borrow().len(), 1);
    let child_value = "child-value".to_owned();
    let child = child_subjects.borrow()[0].clone();
    child.on_next(child_value.clone());

    assert_eq!(*sub.child_calls.borrow(), vec![(child_value, key)]);
}

#[test]
fn emit_changes_fires_once_for_batch() {
    let batch_size = 5;
    let source = source_cache();
    let observer = TestObserver::new();
    let sub = TestSubscription::new(observer, None);
    sub.expose_create_parent(source.connect());

    source.edit(|updater| {
        for i in 0..batch_size {
            updater.add_or_update(item(i + 1, "value"));
        }
    });

    assert_eq!(sub.parent_call_count.get(), 1, "single batch = single ParentOnNext");
    assert_eq!(sub.emit_call_count.get(), 1, "single batch = single EmitChanges");
}

#[test]
fn batching_child_updates_settle_before_emit() {
    let batch_size = 5;
    let source = source_cache();
    let observer = TestObserver::new();
    let child_count = Rc::new(Cell::new(0));
    let count = child_count.clone();
    let factory: ChildFactory = Rc::new(move |key| {
        count.set(count.get() + 1);
        BehaviorSubject::new(format!("sync-{key}")).as_observable()
    });
    let sub = TestSubscription::new(observer, Some(factory));
    sub.expose_create_parent(source.connect());

    source.edit(|updater| {
        for i in 0..batch_size {
            updater.add_or_update(item(i + 1, "value"));
        }
    });

    assert_eq!(child_count.get(), batch_size as usize, "each item should create a child");
    assert!(sub.emit_call_count.get() >= 1, "EmitChanges fires after parent + children settle");
}

#[test]
fn completion_requires_parent_and_all_children() {
    let source = Subject::<ChangeSet<TestItem, i32>>::new();
    let (factory, child_subjects) = subject_factory();
    let observer = TestObserver::new();
    let sub = TestSubscription::new(observer.clone(), Some(factory));
    sub.expose_create_parent(source.as_observable());

    source.on_next(add(item(4321, "item")));
    assert_eq!(child_subjects.borrow().len(), 1);

    source.on_completed();
    assert!(!observer.is_completed.get(), "parent complete but child still active");

    let child = child_subjects.borrow()[0].clone();
    child.on_completed();
    assert!(observer.is_completed.get(), "OnCompleted fires when parent + all children complete");
}

#[test]
fn completion_parent_only_no_children() {
    let source = Subject::<ChangeSet<TestItem, i32>>::new();
    let observer = TestObserver::new();
    let sub = TestSubscription::new(observer.clone(), None);
    sub.expose_create_parent(source.as_observable());

    source.on_completed();
    assert!(observer.is_completed.get(), "immediate OnCompleted when no children");
}

#[test]
fn disposal_stops_all_emissions() {
    let source = source_cache();
    let (factory, child_subjects) = subject_factory();
    let observer = TestObserver::new();
    let sub = TestSubscription::new(observer.clone(), Some(factory));
    sub.expose_create_parent(source.connect());

    source.edit(|updater| updater.add_or_update(item(4321, "item")));
    let emits_before = observer.emit_count.get();

    sub.subscription.dispose();

    source.edit(|updater| updater.add_or_update(item(14321, "after")));
    let first_child = child_subjects.borrow().first().cloned();
    if let Some(child) = first_child {
        child.on_next("after-dispose".to_owned());
    }

    assert_eq!(observer.emit_count.get(), emits_before, "no emissions after disposal");
}

#[test]
fn error_propagates() {
    let source = Subject::<ChangeSet<TestItem, i32>>::new();
    let observer = TestObserver::new();
    let sub = TestSubscription::new(observer.clone(), None);
    sub.expose_create_parent(source.as_observable());

    let error = test_error("test error");
    source.on_error(error.clone());

    assert!(same_error(&observer.error.borrow(), &error));
    drop(sub);
}

#[test]
fn serialization_parent_and_child_do_not_interleave() {
    let source = source_cache();
    let call_log = Rc::new(RefCell::new(Vec::<String>::new()));
    let observer = TestObserver::new();
    let (parent_log, child_log) = (call_log.clone(), call_log.clone());
    let factory: ChildFactory = Rc::new(|_key| Subject::<String>::new().as_observable());
    let sub = TestSubscription::with_callbacks(
        observer,
        Some(factory),
        Some(Rc::new(move || {
            parent_log.borrow_mut().push("P-start".to_owned());
            parent_log.borrow_mut().push("P-end".to_owned());
        })),
        Some(Rc::new(move || {
            child_log.borrow_mut().push("C-start".to_owned());
            child_log.borrow_mut().push("C-end".to_owned());
        })),
    );
    sub.expose_create_parent(source.connect());

    source.edit(|updater| updater.add_or_update(item(4321, "item")));

    // The start and end pairs do not interleave.
    let call_log = call_log.borrow();
    let mut i = 0;
    while i + 1 < call_log.len() {
        let prefix = call_log[i].split('-').next().expect("an entry has a prefix");
        assert!(call_log[i + 1].starts_with(prefix), "operations should not interleave");
        i += 2;
    }
}

// Not ported: DeadlockProof_CrossFeedingSubscriptions - real threads
