//! `ViewThreadObservableTests` (`ReactiveUI.Binding.Tests/Observables/ViewThreadObservableTests.cs`).
//!
//! "Another thread" is a call while the stub invoker answers that the calling thread has no
//! access, as upstream's stub does.

use crate::tests::binding::test_models::{ManualObservable, ManualSequencer, RecordingObserver, RecordingSynchronizationContext, StubViewThreadInvoker};
use ferroui_reactiveui::*;
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

const FIRST: &str = "first";

const SECOND: &str = "second";

const THIRD: &str = "third";

/// An error of the tests (`InvalidOperationException`).
#[derive(Debug)]
struct InvalidOperation;

impl fmt::Display for InvalidOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Operation is not valid due to the current state of the object.")
    }
}

impl std::error::Error for InvalidOperation {}

/// The object the stage writes to (`Target`).
fn target() -> Rc<dyn Any> {
    Rc::new(())
}

fn joined(observer: &RecordingObserver<String>) -> String {
    observer.values().join(",")
}

/// The source, the recording observer and the subscription of a stage on `invoker`.
fn subscribe(invoker: &Rc<StubViewThreadInvoker>) -> (Rc<ManualObservable<String>>, Rc<RecordingObserver<String>>, Rc<dyn IDisposable>) {
    let source = ManualObservable::<String>::new();
    let observer = RecordingObserver::<String>::new();
    let subscription = ViewThreadObservable::new(source.clone(), target(), invoker.clone()).subscribe(observer.clone());
    (source, observer, subscription)
}

fn send(source: &ManualObservable<String>, value: &str) {
    source.on_next(value.to_string());
}

// Not ported: Subscribe_WithNoObserver_Throws - argument validation

#[test]
fn on_next_on_the_owning_thread_delivers_inline() {
    let invoker = StubViewThreadInvoker::unclaimed();
    invoker.set_has_access(true);
    let (source, observer, subscription) = subscribe(&invoker);

    send(&source, FIRST);

    assert_eq!(joined(&observer), FIRST);
    assert_eq!(invoker.post_count(), 0);
    subscription.dispose();
}

#[test]
fn on_next_from_another_thread_waits_for_the_invoker() {
    let invoker = StubViewThreadInvoker::unclaimed();
    let (source, observer, subscription) = subscribe(&invoker);

    send(&source, FIRST);

    assert_eq!(observer.values().len(), 0);

    invoker.run_posted();

    assert_eq!(joined(&observer), FIRST);
    subscription.dispose();
}

#[test]
fn on_next_a_burst_from_another_thread_writes_only_the_latest_value() {
    let invoker = StubViewThreadInvoker::unclaimed();
    let (source, observer, subscription) = subscribe(&invoker);

    send(&source, FIRST);
    send(&source, SECOND);
    send(&source, THIRD);

    invoker.run_posted();

    assert_eq!(invoker.post_count(), 1);
    assert_eq!(joined(&observer), THIRD);
    subscription.dispose();
}

#[test]
fn on_next_on_a_sequencer_waits_for_the_sequencer() {
    let source = ManualObservable::<String>::new();
    let observer = RecordingObserver::<String>::new();
    let sequencer = ManualSequencer::new();

    let subscription = ViewThreadObservable::on_sequencer(source.clone(), sequencer.scheduler()).subscribe(observer.clone());
    send(&source, FIRST);

    assert_eq!(observer.values().len(), 0);

    let _ = sequencer.run_pending();

    assert_eq!(joined(&observer), FIRST);
    subscription.dispose();
}

#[test]
fn on_next_a_burst_on_a_sequencer_writes_only_the_latest_value() {
    let source = ManualObservable::<String>::new();
    let observer = RecordingObserver::<String>::new();
    let sequencer = ManualSequencer::new();

    let subscription = ViewThreadObservable::on_sequencer(source.clone(), sequencer.scheduler()).subscribe(observer.clone());
    send(&source, FIRST);
    send(&source, SECOND);
    send(&source, THIRD);

    assert_eq!(sequencer.run_pending(), 1);
    assert_eq!(joined(&observer), THIRD);
    subscription.dispose();
}

#[test]
fn on_next_on_the_owning_thread_while_a_write_waits_replaces_the_waiting_value() {
    let invoker = StubViewThreadInvoker::unclaimed();
    let (source, observer, subscription) = subscribe(&invoker);

    send(&source, FIRST);
    invoker.set_has_access(true);
    send(&source, SECOND);

    assert_eq!(observer.values().len(), 0);

    invoker.run_posted();

    assert_eq!(invoker.post_count(), 1);
    assert_eq!(joined(&observer), SECOND);
    subscription.dispose();
}

/// Forwards to the recording observer, and runs an action on completion (`CompletionOrderObserver`).
struct CompletionOrderObserver {
    inner: Rc<RecordingObserver<String>>,
    on_completed: Box<dyn Fn()>,
}

impl IObserver<String> for CompletionOrderObserver {
    fn on_next(&self, value: String) {
        self.inner.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.inner.on_error(error);
    }

    fn on_completed(&self) {
        (self.on_completed)();
    }
}

#[test]
fn on_completed_after_a_waiting_value_writes_the_value_first() {
    let invoker = StubViewThreadInvoker::unclaimed();
    let completed_after = Rc::new(RefCell::new(String::new()));
    let source = ManualObservable::<String>::new();
    let observer = RecordingObserver::<String>::new();
    let (recorded, after) = (observer.clone(), completed_after.clone());
    let completion = Rc::new(CompletionOrderObserver { inner: observer.clone(), on_completed: Box::new(move || *after.borrow_mut() = joined(&recorded)) });

    let subscription = ViewThreadObservable::new(source.clone(), target(), invoker.clone()).subscribe(completion);
    send(&source, FIRST);
    source.observer().expect("the stage subscribed").on_completed();
    invoker.run_posted();

    assert_eq!(*completed_after.borrow(), FIRST);
    subscription.dispose();
}

/// Records the values, raising `second` from the source when it receives `first`
/// (`RecordingObserver<string>(Action<string>)`).
struct ReentrantRecorder {
    values: RefCell<Vec<String>>,
    source: Rc<ManualObservable<String>>,
}

impl IObserver<String> for ReentrantRecorder {
    fn on_next(&self, value: String) {
        let first = value == FIRST;
        self.values.borrow_mut().push(value);
        if first {
            send(&self.source, SECOND);
        }
    }
}

#[test]
fn on_next_raised_while_draining_is_delivered_in_the_same_drain() {
    let invoker = StubViewThreadInvoker::unclaimed();
    let source = ManualObservable::<String>::new();
    let observer = Rc::new(ReentrantRecorder { values: RefCell::new(Vec::new()), source: source.clone() });

    let subscription = ViewThreadObservable::new(source.clone(), target(), invoker.clone()).subscribe(observer.clone());
    send(&source, FIRST);
    invoker.run_posted();

    assert_eq!(invoker.post_count(), 1);
    assert_eq!(observer.values.borrow().join(","), "first,second");
    subscription.dispose();
}

#[test]
fn on_error_on_the_owning_thread_delivers_inline() {
    let invoker = StubViewThreadInvoker::unclaimed();
    invoker.set_has_access(true);
    let (source, observer, subscription) = subscribe(&invoker);
    let error: ObservableError = Rc::new(InvalidOperation);

    source.observer().expect("the stage subscribed").on_error(error.clone());

    assert!(observer.error.borrow().as_ref().is_some_and(|received| Rc::ptr_eq(received, &error)));
    subscription.dispose();
}

#[test]
fn on_error_from_another_thread_is_delivered_by_the_drain() {
    let invoker = StubViewThreadInvoker::unclaimed();
    let (source, observer, subscription) = subscribe(&invoker);
    let error: ObservableError = Rc::new(InvalidOperation);

    send(&source, FIRST);
    source.observer().expect("the stage subscribed").on_error(error.clone());

    assert!(observer.error.borrow().is_none());

    invoker.run_posted();

    assert_eq!(joined(&observer), FIRST);
    assert!(observer.error.borrow().as_ref().is_some_and(|received| Rc::ptr_eq(received, &error)));
    subscription.dispose();
}

#[test]
fn on_completed_on_the_owning_thread_delivers_inline() {
    let invoker = StubViewThreadInvoker::unclaimed();
    invoker.set_has_access(true);
    let (source, observer, subscription) = subscribe(&invoker);

    source.observer().expect("the stage subscribed").on_completed();

    assert!(observer.is_completed.get());
    subscription.dispose();
}

#[test]
fn on_completed_from_another_thread_is_delivered_by_the_drain() {
    let invoker = StubViewThreadInvoker::unclaimed();
    let (source, observer, subscription) = subscribe(&invoker);

    source.observer().expect("the stage subscribed").on_completed();

    assert!(!observer.is_completed.get());

    invoker.run_posted();

    assert!(observer.is_completed.get());
    subscription.dispose();
}

#[test]
fn on_next_from_another_thread_when_the_host_sets_a_main_thread_drains_there() {
    let invoker = StubViewThreadInvoker::unclaimed();
    let host = RecordingSynchronizationContext::new();
    let (source, observer, subscription) = subscribe(&invoker);

    BindingSchedulers::use_synchronization_context(Some(host.clone() as Arc<dyn SynchronizationContext>));

    send(&source, FIRST);

    assert_eq!(host.post_count(), 1);
    assert_eq!(invoker.post_count(), 0);
    assert_eq!(joined(&observer), FIRST);
    subscription.dispose();
    BindingSchedulers::use_synchronization_context(None);
}

#[test]
fn dispose_before_the_drain_drops_what_was_queued() {
    let invoker = StubViewThreadInvoker::unclaimed();
    let (source, observer, subscription) = subscribe(&invoker);

    send(&source, FIRST);
    subscription.dispose();
    invoker.run_posted();

    assert_eq!(observer.values().len(), 0);
}

/// A source whose subscription counts its disposals (`CountingObservable`).
#[derive(Default)]
struct CountingObservable {
    dispose_count: Rc<Cell<usize>>,
}

impl IObservable<String> for CountingObservable {
    fn subscribe(&self, _observer: Rc<dyn IObserver<String>>) -> Rc<dyn IDisposable> {
        let count = self.dispose_count.clone();
        Rc::new(CountingSubscription(count))
    }
}

/// Counts every disposal, the repeated ones too.
struct CountingSubscription(Rc<Cell<usize>>);

impl IDisposable for CountingSubscription {
    fn dispose(&self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn dispose_twice_disposes_the_source_subscription_once() {
    let source = Rc::new(CountingObservable::default());
    let subscription = ViewThreadObservable::new(source.clone(), target(), StubViewThreadInvoker::unclaimed()).subscribe(RecordingObserver::<String>::new());

    subscription.dispose();
    subscription.dispose();

    assert_eq!(source.dispose_count.get(), 1);
}

#[test]
fn on_next_after_dispose_is_ignored() {
    let invoker = StubViewThreadInvoker::unclaimed();
    invoker.set_has_access(true);
    let (source, observer, subscription) = subscribe(&invoker);

    subscription.dispose();
    send(&source, FIRST);
    invoker.set_has_access(false);
    send(&source, SECOND);

    assert_eq!(observer.values().len(), 0);
    assert_eq!(invoker.post_count(), 0);
}
