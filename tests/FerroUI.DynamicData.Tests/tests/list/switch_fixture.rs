//! `SwitchFixture` (`DynamicData.Tests/List/SwitchFixture.cs`): the change sets of the latest of a
//! sequence of lists or of sequences of change sets.

use crate::utilities::{assert_equivalent, RawAnonymousObservable};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{Change, ChangeSet, IObservableList, ListChangeReason, ObservableListEx, SourceList, SourceListEditConvenienceEx, SwitchListEx};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_reactive::Observable;
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// The change sets of a list of numbers (`IObservable<IChangeSet<int>>`).
type NumberChanges = Rc<dyn IObservable<ChangeSet<i32>>>;

/// The fixture: the first list, the subject of the lists and the results.
struct Fixture {
    results: ChangeSetAggregator<i32>,
    source: SourceList<i32>,
    switchable: Rc<BehaviorSubject<SourceList<i32>>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();
        let switchable = BehaviorSubject::new(source.clone());
        let results = switchable.as_observable().switch_list().as_aggregator();
        Self { results, source, switchable }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

/// `Enumerable.Range(start, count)`.
fn range(start: i32, count: i32) -> Vec<i32> {
    (start..start + count).collect()
}

#[test]
fn clears_for_new_source() {
    let f = Fixture::new();
    let inital = range(1, 100);
    f.source.add_range(inital.clone());

    assert_eq!(f.results.data().count(), 100);

    let new_source = SourceList::new();
    f.switchable.on_next(new_source.clone());

    assert_eq!(f.results.data().count(), 0);

    new_source.add_range(inital);
    assert_eq!(f.results.data().count(), 100);

    let next_updates = range(100, 100);
    new_source.add_range(next_updates);
    assert_eq!(f.results.data().count(), 200);
}

#[test]
fn poulates_first_source() {
    let f = Fixture::new();
    let inital = range(1, 100);
    f.source.add_range(inital.clone());

    assert_eq!(f.results.data().count(), 100);

    assert_equivalent(&inital, &f.source.items());
}

#[test]
fn propagates_outer_errors() {
    let source = SourceList::<i32>::new();
    let switchable = BehaviorSubject::<NumberChanges>::new(source.connect());
    let results = switchable.as_observable().switch_list().as_aggregator();

    source.add_range(range(1, 100));

    let error = new_error("Test");
    switchable.on_error(error.clone());

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));

    results.dispose();
    switchable.dispose();
    source.dispose();
}

#[test]
fn propagates_inner_errors() {
    let source = SourceList::<i32>::new();
    let switchable = BehaviorSubject::<NumberChanges>::new(source.connect());
    let results = switchable.as_observable().switch_list().as_aggregator();

    source.add_range(range(1, 100));

    let source2 = Subject::<ChangeSet<i32>>::new();

    let source2_changes: NumberChanges = source2.clone();
    switchable.on_next(source2_changes);

    let error = new_error("Test");
    source2.on_error(error.clone());

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));

    source2.dispose();
    results.dispose();
    switchable.dispose();
    source.dispose();
}

#[test]
fn completes_when_sources_and_inner_complete() {
    let source = SourceList::<i32>::new();
    let switchable = BehaviorSubject::<NumberChanges>::new(source.connect());
    let results = switchable.as_observable().switch_list().as_aggregator();

    source.add_range(range(1, 100));

    switchable.on_completed();
    assert!(!results.is_completed(), "the inner sequence is still running");

    source.dispose();

    assert!(results.is_completed(), "both the sources and the inner sequence have completed");
    assert!(results.error().is_none());
    assert_eq!(results.data().count(), 100, "all data should have been received before completion");

    results.dispose();
    switchable.dispose();
}

#[test]
fn does_not_complete_while_inner_is_still_running() {
    let source = SourceList::<i32>::new();
    let switchable = BehaviorSubject::<NumberChanges>::new(source.connect());
    let results = switchable.as_observable().switch_list().as_aggregator();

    switchable.on_completed();
    source.add(1);

    assert!(!results.is_completed(), "the inner sequence has not completed");
    assert_eq!(results.data().count(), 1, "changes should still flow after the sources sequence completes");

    results.dispose();
    switchable.dispose();
    source.dispose();
}

#[test]
fn does_not_complete_when_only_a_superseded_inner_completes() {
    let first = SourceList::<i32>::new();
    let second = SourceList::<i32>::new();
    let switchable = BehaviorSubject::<NumberChanges>::new(first.connect());
    let results = switchable.as_observable().switch_list().as_aggregator();

    switchable.on_next(second.connect());
    switchable.on_completed();

    first.dispose();

    assert!(!results.is_completed(), "the superseded sequence is not the current one");

    second.add(1);
    assert_eq!(results.data().count(), 1, "the current sequence should still be delivering");

    second.dispose();
    assert!(results.is_completed(), "the current sequence has now completed");

    results.dispose();
    switchable.dispose();
}

#[test]
fn completes_when_sources_and_inner_complete_synchronously() {
    let results = Observable::return_(Observable::empty::<ChangeSet<i32>>()).switch_list().as_aggregator();

    assert!(results.is_completed(), "everything completed during subscription");
    assert!(results.error().is_none());

    results.dispose();
}

#[test]
fn delivers_changes_emitted_before_synchronous_completion() {
    let change = ChangeSet::from(vec![Change::new(ListChangeReason::Add, 42, None)]);
    let results = Observable::return_(Observable::return_(change)).switch_list().as_aggregator();

    assert_eq!(results.data().count(), 1, "changes emitted before a synchronous completion must not be lost");
    assert!(results.is_completed(), "the source completed");
    assert!(results.error().is_none());

    results.dispose();
}

#[test]
fn ignores_changes_from_a_superseded_source() {
    let first = Subject::<ChangeSet<i32>>::new();
    let second = Subject::<ChangeSet<i32>>::new();
    let first_changes: NumberChanges = first.clone();
    let switchable = BehaviorSubject::new(first_changes);
    let results = switchable.as_observable().switch_list().as_aggregator();

    first.on_next(ChangeSet::from(vec![Change::new(ListChangeReason::Add, 1, None)]));
    assert_eq!(results.data().count(), 1);

    let second_changes: NumberChanges = second.clone();
    switchable.on_next(second_changes);
    assert_eq!(results.data().count(), 0, "moving to a new source drops what the previous one contributed");

    first.on_next(ChangeSet::from(vec![Change::new(ListChangeReason::Add, 2, None)]));

    assert_eq!(results.data().count(), 0, "a superseded source must not be able to write into the result");
    assert!(results.error().is_none());

    results.dispose();
    switchable.dispose();
    second.dispose();
    first.dispose();
}

#[test]
fn propagates_inner_errors_raised_synchronously() {
    let error = new_error("Test");
    let results = Observable::return_(Observable::throw::<ChangeSet<i32>>(error.clone())).switch_list().as_aggregator();

    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)), "the error was raised during subscription");

    results.dispose();
}

// Not ported: DoesNotHoldALockWhileDeliveringDownstream - real threads

#[test]
fn ignores_errors_from_a_superseded_source() {
    // Switching away from a source means anything it produces afterwards belongs to a source that
    // is no longer selected, and that includes its failures. Ordinarily disposal stops a
    // superseded source being heard from again, but disposal cannot reach a notification that is
    // already in flight, so the operator has to discard it on arrival. The raw observable hands
    // back the observer directly, which is how that in-flight failure is reproduced here without
    // needing a race to land.
    let superseded_observer: Rc<RefCell<Option<Rc<dyn IObserver<ChangeSet<i32>>>>>> = Rc::new(RefCell::new(None));
    let captured = superseded_observer.clone();
    let superseded = RawAnonymousObservable::create(move |observer: Rc<dyn IObserver<ChangeSet<i32>>>| {
        *captured.borrow_mut() = Some(observer);
        Disposable::empty()
    });

    let switchable = Subject::<NumberChanges>::new();
    let current = Subject::<ChangeSet<i32>>::new();

    let results = switchable.as_observable().switch_list().as_aggregator();

    switchable.on_next(superseded);
    let current_changes: NumberChanges = current.clone();
    switchable.on_next(current_changes);

    let observer = superseded_observer.borrow().clone();
    assert!(observer.is_some(), "the superseded source should have been subscribed");
    observer.expect("the superseded source was subscribed").on_error(new_error("Test"));

    assert!(results.error().is_none(), "the failed source had already been switched away from");

    current.on_next(ChangeSet::from(vec![Change::new(ListChangeReason::Add, 1, Some(0))]));

    assert_eq!(results.data().count(), 1, "the selected source should still be delivering");

    results.dispose();
    current.dispose();
    switchable.dispose();
}
