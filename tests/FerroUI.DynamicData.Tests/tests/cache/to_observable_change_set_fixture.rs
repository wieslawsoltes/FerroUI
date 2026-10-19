//! `ToObservableChangeSetFixture` (`DynamicData.Tests/Cache/ToObservableChangeSetFixture.cs` and
//! its partial files `.Items.cs`, `.Items.UnitTests.cs`, `.Items.IntegrationTests.cs`,
//! `.Sequences.cs`, `.Sequences.UnitTests.cs`, `.Sequences.IntegrationTests.cs`): the items of a
//! plain sequence, or of a sequence of collections, as the changes of a cache, with expiration and a
//! limit of size.
//!
//! Upstream's `ToObservableChangeSet(IObservable<TObject>, ..)` is
//! [`ObservableItemsEx::to_observable_change_set_cache_with`] and its form on
//! `IObservable<IEnumerable<TObject>>` is
//! [`ObservableSequencesEx::to_observable_change_set_cache_many_with`]; the nested classes `Items`
//! and `Sequences` are the modules [`items`] and [`sequences`].

use crate::utilities::{assert_equivalent_because, assert_should_not_support_sorting, CacheObservableExtensions, ObservableExtensions};
use ferroui_dynamic_data::{ObservableItemsEx, ObservableSequencesEx};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::Observable;
use ferroui_reactive_testing::{duration_to_ticks, new_error, TestScheduler};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

// Not ported: IntegrationTestItemCount - only the integration tests, which are not ported, use it.
// Not ported: SchedulerType - only the integration tests, which are not ported, use it.

/// `SourceType`: whether the source notifies after the subscription (a subject) or during it
/// (`Observable.Return` / `Observable.Throw`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceType {
    Immediate,
    Asynchronous,
}

/// `Item`, a record: equal when its parts are, the error by reference.
#[derive(Clone, Default)]
struct Item {
    id: i32,
    error: Option<ObservableError>,
    lifetime: Option<Duration>,
}

impl Item {
    /// `Item.SelectId`.
    fn select_id(item: &Item) -> i32 {
        item.id
    }

    /// `Item.SelectLifetime`.
    fn select_lifetime(item: &Item) -> Option<Duration> {
        item.lifetime
    }

    /// `new Item() { Id = id }`.
    fn new(id: i32) -> Self {
        Self { id, ..Self::default() }
    }

    /// `new Item() { Id = id, Lifetime = lifetime }`.
    fn with_lifetime(id: i32, lifetime: Duration) -> Self {
        Self { id, lifetime: Some(lifetime), ..Self::default() }
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        let same_error = match (&self.error, &other.error) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        self.id == other.id && same_error && self.lifetime == other.lifetime
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Item")
            .field("id", &self.id)
            .field("error", &self.error.as_ref().map(|error| error.to_string()))
            .field("lifetime", &self.lifetime)
            .finish()
    }
}

/// The expiration of `Item.SelectLifetime`, as the operators take it.
fn select_lifetime() -> Option<Rc<dyn Fn(&Item) -> Option<Duration>>> {
    Some(Rc::new(Item::select_lifetime))
}

/// The ticks of a number of seconds (`TimeSpan.FromSeconds(seconds).Ticks`).
fn seconds_ticks(seconds: u64) -> i64 {
    duration_to_ticks(Duration::from_secs(seconds))
}

/// The source of a test of `SourceType`: a subject for `Asynchronous`, which the test drives, or
/// the sequence of `immediate` for `Immediate`.
fn create_source<T: Clone + 'static>(source_type: SourceType, immediate: impl FnOnce() -> Rc<dyn IObservable<T>>) -> (Rc<dyn IObservable<T>>, Option<Rc<Subject<T>>>) {
    match source_type {
        SourceType::Asynchronous => {
            let subject = Subject::<T>::new();
            (subject.as_observable(), Some(subject))
        }
        SourceType::Immediate => (immediate(), None),
    }
}

/// `ToObservableChangeSetFixture.Items`: the sequences of items.
mod items {
    /// `ToObservableChangeSetFixture.Items.UnitTests`.
    mod unit_tests {
        use super::super::*;

        // Not ported: ExpireAfterThrows_ErrorPropagates - needs a fallible expiration selector (the port has no try_ overload of to_observable_change_set_cache_with)

        // Not ported: KeySelectorIsNull_ThrowsException - argument validation

        // Not ported: KeySelectorThrows_ErrorPropagates - needs a fallible key selector (the port has no try_ overload of to_observable_change_set_cache)

        #[test]
        fn size_limit_is_exceeded_oldest_items_are_removed() {
            // Setup
            let source = Subject::<Item>::new();

            // UUT Initialization
            let (subscription, results) = source
                .as_observable()
                .to_observable_change_set_cache_with(Item::select_id, None, Some(5), None)
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            assert!(results.recorded_items().is_empty(), "no items have been emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: Not enough items to reach the limit
            let item1 = Item::new(1);
            source.on_next(item1.clone());

            let item2 = Item::new(2);
            source.on_next(item2.clone());

            let item3 = Item::new(3);
            source.on_next(item3.clone());

            let item4 = Item::new(4);
            source.on_next(item4.clone());

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 4, "4 items were emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone(), item4.clone()], "4 items were emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: Limit is reached
            let item5 = Item::new(5);
            source.on_next(item5.clone());

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone(), item4.clone(), item5.clone()], "1 item was emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: New item exceeds the limit
            let item6 = Item::new(6);
            source.on_next(item6.clone());

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(6).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item2.clone(), item3.clone(), item4.clone(), item5.clone(), item6.clone()], "1 item was emitted, and 1 was evicted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: Replacement leaves all other items in-place
            let item7 = Item::new(4);
            source.on_next(item7.clone());

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(7).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item2, item3, item7, item5, item6], "1 item was emitted, and replaced an existing item.");
            assert!(!results.has_completed(), "the source has not completed");

            subscription.dispose();
            source.dispose();
        }

        fn source_completes_when_expirations_are_pending_completion_waits_for_expirations(source_type: SourceType) {
            // Setup
            let item = Item::with_lifetime(1, Duration::from_secs(10));

            let immediate_item = item.clone();
            let (source, subject) = create_source(source_type, move || Observable::return_(immediate_item));

            let scheduler = TestScheduler::new();

            // UUT Initialization & Action
            let (subscription, results) = source
                .to_observable_change_set_cache_with(Item::select_id, select_lifetime(), None, Some(scheduler.scheduler()))
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            if let Some(subject) = &subject {
                subject.on_next(item.clone());
                subject.on_completed();
            }

            assert!(results.error().is_none());
            if source_type == SourceType::Asynchronous {
                assert_eq!(results.recorded_change_sets().len(), 2, "1 item was emitted, after initialization");
            } else {
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            }
            assert_equivalent_because(&results.recorded_items(), &[item.clone()], "1 item was emitted");
            assert!(!results.has_completed(), "1 item has yet to expire");

            // UUT Action
            scheduler.advance_to(seconds_ticks(10));

            assert!(results.error().is_none());
            if source_type == SourceType::Asynchronous {
                assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 item should have expired");
            } else {
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 item should have expired");
            }
            assert!(results.recorded_items().is_empty(), "all items have expired");
            assert!(results.has_completed(), "the source, and all outstanding expirations, have completed");

            assert_should_not_support_sorting(&results, "");

            subscription.dispose();
        }

        #[test]
        fn source_completes_when_expirations_are_pending_completion_waits_for_expirations_case1() {
            source_completes_when_expirations_are_pending_completion_waits_for_expirations(SourceType::Asynchronous);
        }

        #[test]
        fn source_completes_when_expirations_are_pending_completion_waits_for_expirations_case2() {
            source_completes_when_expirations_are_pending_completion_waits_for_expirations(SourceType::Immediate);
        }

        fn source_completes_when_no_expirations_are_pending_completion_propagates(source_type: SourceType) {
            // Setup
            let item = Item::new(1);

            let immediate_item = item.clone();
            let (source, subject) = create_source(source_type, move || Observable::return_(immediate_item));

            let scheduler = TestScheduler::new();

            // UUT Initialization & Action
            let (subscription, results) = source
                .to_observable_change_set_cache_with(Item::select_id, select_lifetime(), None, Some(scheduler.scheduler()))
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            if let Some(subject) = &subject {
                subject.on_next(item.clone());
                subject.on_completed();
            }

            assert!(results.error().is_none());
            if source_type == SourceType::Asynchronous {
                assert_eq!(results.recorded_change_sets().len(), 2, "1 item was emitted, after initialization");
            } else {
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            }
            assert_equivalent_because(&results.recorded_items(), &[item], "1 item was emitted");
            assert!(results.has_completed(), "the source has completed, and no items remain to be expired");

            assert_should_not_support_sorting(&results, "");

            subscription.dispose();
        }

        #[test]
        fn source_completes_when_no_expirations_are_pending_completion_propagates_case1() {
            source_completes_when_no_expirations_are_pending_completion_propagates(SourceType::Asynchronous);
        }

        #[test]
        fn source_completes_when_no_expirations_are_pending_completion_propagates_case2() {
            source_completes_when_no_expirations_are_pending_completion_propagates(SourceType::Immediate);
        }

        #[test]
        fn source_emits_repeated_items_repeated_items_are_updated_and_expiration_is_rescheduled() {
            // Setup
            let source = Subject::<Item>::new();

            let scheduler = TestScheduler::new();

            // UUT Initialization
            let (subscription, results) = source
                .as_observable()
                .to_observable_change_set_cache_with(Item::select_id, select_lifetime(), None, Some(scheduler.scheduler()))
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            assert!(results.recorded_items().is_empty(), "no items have been emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item1 = Item::with_lifetime(1, Duration::from_secs(1));
            source.on_next(item1.clone());
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "1 item was emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item2 = Item::new(2);
            source.on_next(item2.clone());
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone()], "1 item was emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item3 = Item::with_lifetime(3, Duration::from_secs(1));
            source.on_next(item3.clone());
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone()], "1 item was emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item4 = Item::with_lifetime(1, Duration::from_secs(1));
            source.on_next(item4.clone());
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item4.clone(), item2.clone(), item3.clone()], "1 item was emitted, and replaced an existing item");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item5 = Item::new(2);
            source.on_next(item5.clone());
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item4.clone(), item5.clone(), item3.clone()], "1 item was emitted, and replaced an existing item");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item6 = Item::with_lifetime(3, Duration::from_secs(3));
            source.on_next(item6.clone());
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(6).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item4.clone(), item5.clone(), item6.clone()], "1 item was emitted, and replaced an existing item");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(1));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(7).count(), 1, "1 expiration should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item5.clone(), item6.clone()], "1 item reached its expiration");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(2));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(8).count(), 0, "no expirations should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item5.clone(), item6.clone()], "no changes were made");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(3));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(8).count(), 1, "1 expiration should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item5.clone()], "1 item reached its expiration");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(4));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(9).count(), 0, "no expirations should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item5], "no changes were made");
            assert!(!results.has_completed(), "the source has not completed");

            assert_should_not_support_sorting(&results, "");

            subscription.dispose();
            source.dispose();
        }

        #[test]
        fn source_emits_unique_items_items_are_added_and_removed_when_expired() {
            // Setup
            let source = Subject::<Item>::new();

            let scheduler = TestScheduler::new();

            // UUT Initialization
            let (subscription, results) = source
                .as_observable()
                .to_observable_change_set_cache_with(Item::select_id, select_lifetime(), None, Some(scheduler.scheduler()))
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            assert!(results.recorded_items().is_empty(), "no items have been emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item1 = Item::with_lifetime(1, Duration::from_secs(3));
            source.on_next(item1.clone());
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "1 item was emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item2 = Item::new(2);
            source.on_next(item2.clone());
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone()], "1 item was emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item3 = Item::with_lifetime(3, Duration::from_secs(1));
            source.on_next(item3.clone());
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 item was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3], "1 item was emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(1));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 1, "1 expiration should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone()], "1 item expired, and 1 had its lifetime extended");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(2));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 0, "no expirations should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item1, item2.clone()], "no changes were made");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(3));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 1, "1 expiration should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item2.clone()], "1 item reached its expiration");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(4));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(6).count(), 0, "no expirations should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item2], "no changes were made");
            assert!(!results.has_completed(), "the source has not completed");

            assert_should_not_support_sorting(&results, "");

            subscription.dispose();
            source.dispose();
        }

        fn source_fails_error_propagates(source_type: SourceType) {
            // Setup
            let error = new_error("Test Exception");

            let immediate_error = error.clone();
            let (source, subject) = create_source::<Item>(source_type, move || Observable::throw(immediate_error));

            // UUT Initialization & Action
            let (subscription, results) = source
                .to_observable_change_set_cache(Item::select_id)
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            if let Some(subject) = &subject {
                subject.on_error(error.clone());
            }

            assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)), "errors should propagate");
            if source_type == SourceType::Asynchronous {
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            } else {
                assert!(results.recorded_change_sets().is_empty(), "an error occurred during initialization");
            }
            assert!(results.recorded_items().is_empty(), "no items were emitted");

            subscription.dispose();
        }

        #[test]
        fn source_fails_error_propagates_case1() {
            source_fails_error_propagates(SourceType::Asynchronous);
        }

        #[test]
        fn source_fails_error_propagates_case2() {
            source_fails_error_propagates(SourceType::Immediate);
        }

        // Not ported: SourceIsNull_ThrowsException - argument validation
    }

    // `ToObservableChangeSetFixture.Items.IntegrationTests`:
    // Not ported: MultipleSubscriptionsRunInParallel_SchedulerUsageIsThreadSafe - real threads
}

/// `ToObservableChangeSetFixture.Sequences`: the sequences of collections of items.
mod sequences {
    /// `ToObservableChangeSetFixture.Sequences.UnitTests`.
    mod unit_tests {
        use super::super::*;

        // Not ported: ExpireAfterThrows_ErrorPropagates - needs a fallible expiration selector (the port has no try_ overload of to_observable_change_set_cache_many_with)

        // Not ported: KeySelectorIsNull_ThrowsException - argument validation

        // Not ported: KeySelectorThrows_ErrorPropagates - needs a fallible key selector (the port has no try_ overload of to_observable_change_set_cache_many)

        #[test]
        fn size_limit_is_exceeded_oldest_items_are_removed() {
            // Setup
            let source = Subject::<Vec<Item>>::new();

            // UUT Initialization
            let (subscription, results) = source
                .as_observable()
                .to_observable_change_set_cache_many_with(Item::select_id, None, Some(5), None)
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            assert!(results.recorded_items().is_empty(), "no source items have been emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: Not enough items to reach the limit
            let item1 = Item::new(1);
            let item2 = Item::new(2);
            let item3 = Item::new(3);
            let item4 = Item::new(4);
            source.on_next(vec![item1.clone(), item2.clone(), item3.clone(), item4.clone()]);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone(), item4.clone()], "4 source items were emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: Limit is reached
            let item5 = Item::new(5);
            source.on_next(vec![item5.clone()]);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source operation was performed");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone(), item4.clone(), item5.clone()], "1 source item was emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: New item exceeds the limit
            let item6 = Item::new(6);
            source.on_next(vec![item6.clone()]);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 source operation was performed");
            assert_equivalent_because(&results.recorded_items(), &[item2.clone(), item3.clone(), item4.clone(), item5.clone(), item6.clone()], "1 source item was emitted, and 1 was evicted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: Replacement leaves all other items in-place
            let item7 = Item::new(4);
            source.on_next(vec![item7.clone()]);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 1, "1 source operation was performed");
            assert_equivalent_because(&results.recorded_items(), &[item2, item3, item7, item5, item6], "1 source item was emitted, and replaced an existing item.");
            assert!(!results.has_completed(), "the source has not completed");

            subscription.dispose();
            source.dispose();
        }

        fn source_completes_when_expirations_are_pending_completion_waits_for_expirations(source_type: SourceType) {
            // Setup
            let items = vec![Item::with_lifetime(1, Duration::from_secs(10)), Item::new(2), Item::with_lifetime(3, Duration::from_secs(20))];

            let immediate_items = items.clone();
            let (source, subject) = create_source(source_type, move || Observable::return_(immediate_items));

            let scheduler = TestScheduler::new();

            // UUT Initialization & Action
            let (subscription, results) = source
                .to_observable_change_set_cache_many_with(Item::select_id, select_lifetime(), None, Some(scheduler.scheduler()))
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            if let Some(subject) = &subject {
                subject.on_next(items.clone());
                subject.on_completed();
            }

            assert!(results.error().is_none());
            if source_type == SourceType::Asynchronous {
                assert_eq!(results.recorded_change_sets().len(), 2, "1 sequence was emitted, after initialization");
            } else {
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            }
            assert_equivalent_because(&results.recorded_items(), &items, "3 items were emitted");
            assert!(!results.has_completed(), "2 items have yet to expire");

            // UUT Action
            scheduler.advance_to(seconds_ticks(30));

            assert!(results.error().is_none());
            if source_type == SourceType::Asynchronous {
                assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 2, "2 items should have expired, at different times");
            } else {
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 2, "2 items should have expired, at different times");
            }
            let unexpirable: Vec<Item> = items.iter().filter(|item| item.lifetime.is_none()).cloned().collect();
            assert_equivalent_because(&results.recorded_items(), &unexpirable, "all expirable items have expired");
            assert!(results.has_completed(), "the source, and all outstanding expirations, have completed");

            assert_should_not_support_sorting(&results, "");

            subscription.dispose();
        }

        #[test]
        fn source_completes_when_expirations_are_pending_completion_waits_for_expirations_case1() {
            source_completes_when_expirations_are_pending_completion_waits_for_expirations(SourceType::Asynchronous);
        }

        #[test]
        fn source_completes_when_expirations_are_pending_completion_waits_for_expirations_case2() {
            source_completes_when_expirations_are_pending_completion_waits_for_expirations(SourceType::Immediate);
        }

        fn source_completes_when_no_expirations_are_pending_completion_propagates(source_type: SourceType) {
            // Setup
            let items = vec![Item::new(1), Item::new(2), Item::new(3)];

            let immediate_items = items.clone();
            let (source, subject) = create_source(source_type, move || Observable::return_(immediate_items));

            let scheduler = TestScheduler::new();

            // UUT Initialization & Action
            let (subscription, results) = source
                .to_observable_change_set_cache_many_with(Item::select_id, select_lifetime(), None, Some(scheduler.scheduler()))
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            if let Some(subject) = &subject {
                subject.on_next(items.clone());
                subject.on_completed();
            }

            assert!(results.error().is_none());
            if source_type == SourceType::Asynchronous {
                assert_eq!(results.recorded_change_sets().len(), 2, "1 sequence was emitted, after initialization");
            } else {
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            }
            assert_equivalent_because(&results.recorded_items(), &items, "3 items were emitted");
            assert!(results.has_completed(), "the source has completed, and no items remain to be expired");

            assert_should_not_support_sorting(&results, "");

            subscription.dispose();
        }

        #[test]
        fn source_completes_when_no_expirations_are_pending_completion_propagates_case1() {
            source_completes_when_no_expirations_are_pending_completion_propagates(SourceType::Asynchronous);
        }

        #[test]
        fn source_completes_when_no_expirations_are_pending_completion_propagates_case2() {
            source_completes_when_no_expirations_are_pending_completion_propagates(SourceType::Immediate);
        }

        #[test]
        fn source_emits_repeated_items_repeated_items_are_updated_and_expiration_is_rescheduled() {
            // Setup
            let source = Subject::<Vec<Item>>::new();

            let scheduler = TestScheduler::new();

            // UUT Initialization
            let (subscription, results) = source
                .as_observable()
                .to_observable_change_set_cache_many_with(Item::select_id, select_lifetime(), None, Some(scheduler.scheduler()))
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            assert!(results.recorded_items().is_empty(), "no source items have been emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item1 = Item::with_lifetime(1, Duration::from_secs(1));
            let item2 = Item::new(2);
            let item3 = Item::with_lifetime(3, Duration::from_secs(1));
            source.on_next(vec![item1.clone(), item2.clone(), item3.clone()]);
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 sequence was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1, item2, item3], "3 items were emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item4 = Item::with_lifetime(1, Duration::from_secs(1));
            let item5 = Item::new(2);
            let item6 = Item::with_lifetime(3, Duration::from_secs(3));
            source.on_next(vec![item4.clone(), item5.clone(), item6.clone()]);
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 sequence was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item4, item5.clone(), item6.clone()], "3 items were emitted, and replaced existing items");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(1));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 expiration should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item5.clone(), item6.clone()], "1 source item reached its expiration");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(2));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 0, "no expirations should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item5.clone(), item6], "no changes were made");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(3));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 1, "1 expiration should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item5.clone()], "1 source item reached its expiration");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(4));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 0, "no expirations should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item5], "no changes were made");
            assert!(!results.has_completed(), "the source has not completed");

            assert_should_not_support_sorting(&results, "");

            subscription.dispose();
            source.dispose();
        }

        #[test]
        fn source_emits_unique_items_items_are_added_and_removed_when_expired() {
            // Setup
            let source = Subject::<Vec<Item>>::new();

            let scheduler = TestScheduler::new();

            // UUT Initialization
            let (subscription, results) = source
                .as_observable()
                .to_observable_change_set_cache_many_with(Item::select_id, select_lifetime(), None, Some(scheduler.scheduler()))
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            assert!(results.recorded_items().is_empty(), "no source items have been emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let item1 = Item::with_lifetime(1, Duration::from_secs(3));
            let item2 = Item::new(2);
            let item3 = Item::with_lifetime(3, Duration::from_secs(1));
            source.on_next(vec![item1.clone(), item2.clone(), item3.clone()]);
            scheduler.advance_by(1);

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 sequence was emitted");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3], "3 items were emitted");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(1));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 expiration should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone()], "1 item expired, and 1 had its lifetime extended");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(2));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 0, "no expirations should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item1, item2.clone()], "no changes were made");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(3));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 expiration should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item2.clone()], "1 item reached its expiration");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            scheduler.advance_to(seconds_ticks(4));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 0, "no expirations should have occurred");
            assert_equivalent_because(&results.recorded_items(), &[item2], "no changes were made");
            assert!(!results.has_completed(), "the source has not completed");

            assert_should_not_support_sorting(&results, "");

            subscription.dispose();
            source.dispose();
        }

        fn source_fails_error_propagates(source_type: SourceType) {
            // Setup
            let error = new_error("Test Exception");

            let immediate_error = error.clone();
            let (source, subject) = create_source::<Vec<Item>>(source_type, move || Observable::throw(immediate_error));

            // UUT Initialization & Action
            let (subscription, results) = source
                .to_observable_change_set_cache_many(Item::select_id)
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

            if let Some(subject) = &subject {
                subject.on_error(error.clone());
            }

            assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)), "errors should propagate");
            if source_type == SourceType::Asynchronous {
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
            } else {
                assert!(results.recorded_change_sets().is_empty(), "an error occurred during initialization");
            }
            assert!(results.recorded_items().is_empty(), "no source items were emitted");

            subscription.dispose();
        }

        #[test]
        fn source_fails_error_propagates_case1() {
            source_fails_error_propagates(SourceType::Asynchronous);
        }

        #[test]
        fn source_fails_error_propagates_case2() {
            source_fails_error_propagates(SourceType::Immediate);
        }

        // Not ported: SourceIsNull_ThrowsException - argument validation
    }

    // `ToObservableChangeSetFixture.Sequences.IntegrationTests`:
    // Not ported: MultipleSubscriptionsRunInParallel_SchedulerUsageIsThreadSafe - real threads
}
