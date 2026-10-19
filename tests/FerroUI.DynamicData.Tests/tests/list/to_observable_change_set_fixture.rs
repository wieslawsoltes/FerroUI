//! `ToObservableChangeSetFixture` (`DynamicData.Tests/List/ToObservableChangeSetFixture.cs` and
//! its partial files `.Items.cs`, `.Items.UnitTests.cs`, `.Items.IntegrationTests.cs`,
//! `.Sequences.cs`, `.Sequences.UnitTests.cs`, `.Sequences.IntegrationTests.cs`): the items of a
//! plain sequence, or of a sequence of collections, as the changes of a list, with their
//! expiration, the limit of size, and the propagation of the end and of errors.
//!
//! Upstream's nested classes `Items.UnitTests` and `Sequences.UnitTests` are the modules
//! [`items`] and [`sequences`]; the overload on `IObservable<IEnumerable<T>>` is
//! `to_observable_change_set_many` here.

use crate::utilities::{ListItemRecordingObserver, ListObservableExtensions, ObservableExtensions};
use ferroui_dynamic_data::list::ChangeSet;
use ferroui_dynamic_data::ExpireAfterSelector;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::Observable;
use ferroui_reactive_testing::{duration_to_ticks, new_error, TestScheduler};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;
use std::time::Duration;

/// `SourceType`: whether the source emits after the subscription (a subject) or during it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SourceType {
    Immediate,
    Asynchronous,
}

/// `Item`: a record, equal by its parts.
///
/// Upstream's `Error` property is left out: only the tests of a failing expiration selector use
/// it, and they are not ported.
#[derive(Clone, PartialEq, Debug)]
struct Item {
    id: i32,
    lifetime: Option<Duration>,
}

impl Item {
    /// `new Item() { Id = id }`.
    fn new(id: i32) -> Self {
        Self { id, lifetime: None }
    }

    /// `new Item() { Id = id, Lifetime = lifetime }`.
    fn with_lifetime(id: i32, lifetime: Duration) -> Self {
        Self { id, lifetime: Some(lifetime) }
    }

    /// `Item.SelectLifetime`.
    fn select_lifetime(item: &Item) -> Option<Duration> {
        item.lifetime
    }
}

/// `expireAfter: Item.SelectLifetime`.
fn select_lifetime() -> Option<ExpireAfterSelector<Item>> {
    let selector: ExpireAfterSelector<Item> = Rc::new(Item::select_lifetime);
    Some(selector)
}

/// `.ValidateSynchronization().ValidateChangeSets().RecordListItems(out var results)`.
fn record(changes: Rc<dyn IObservable<ChangeSet<Item>>>) -> (Rc<dyn IDisposable>, Rc<ListItemRecordingObserver<Item>>) {
    changes.validate_synchronization().validate_list_change_sets().record_list_items()
}

/// `TimeSpan.FromSeconds(seconds).Ticks`.
fn seconds_ticks(seconds: u64) -> i64 {
    duration_to_ticks(Duration::from_secs(seconds))
}

fn error_is(actual: Option<ObservableError>, expected: &ObservableError) -> bool {
    actual.is_some_and(|actual| Rc::ptr_eq(&actual, expected))
}

/// `ToObservableChangeSetFixture.Items.UnitTests`: the overloads on `IObservable<T>`.
mod items {
    use super::*;
    use ferroui_dynamic_data::ObservableItemsEx;

    // Not ported: ExpireAfterThrows_ErrorPropagates - the expiration selector cannot fail (no try_ form of to_observable_change_set_with)

    #[test]
    fn size_limit_is_exceeded_oldest_items_are_removed() {
        // Setup
        let source = Subject::<Item>::new();

        // UUT Initialization
        let (_subscription, results) = record(source.as_observable().to_observable_change_set_with(None, Some(5), None));

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
        assert_eq!(results.recorded_items(), vec![item1.clone(), item2.clone(), item3.clone(), item4.clone()], "4 items were emitted");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action: Limit is reached
        let item5 = Item::new(5);
        source.on_next(item5.clone());

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 1, "1 item was emitted");
        assert_eq!(results.recorded_items(), vec![item1.clone(), item2.clone(), item3.clone(), item4.clone(), item5.clone()], "1 item was emitted");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action: New item exceeds the limit
        let item6 = Item::new(6);
        source.on_next(item6.clone());

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(6).count(), 1, "1 item was emitted");
        assert_eq!(results.recorded_items(), vec![item2, item3, item4, item5, item6], "1 item was emitted, and 1 was evicted");
        assert!(!results.has_completed(), "the source has not completed");
    }

    fn source_completes_when_expirations_are_pending_completion_waits_for_expirations(source_type: SourceType) {
        // Setup
        let item = Item::with_lifetime(1, Duration::from_secs(10));

        let subject = Subject::<Item>::new();
        let source: Rc<dyn IObservable<Item>> = match source_type {
            SourceType::Asynchronous => subject.as_observable(),
            SourceType::Immediate => Observable::return_(item.clone()),
        };

        let scheduler = TestScheduler::new();

        // UUT Initialization & Action
        let (_subscription, results) = record(source.to_observable_change_set_with(select_lifetime(), None, Some(scheduler.scheduler())));

        if source_type == SourceType::Asynchronous {
            subject.on_next(item.clone());
            subject.on_completed();
        }

        assert!(results.error().is_none());
        if source_type == SourceType::Asynchronous {
            assert_eq!(results.recorded_change_sets().len(), 2, "1 item was emitted, after initialization");
        } else {
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
        }
        assert_eq!(results.recorded_items(), vec![item.clone()], "1 item was emitted");
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
    }

    #[test]
    fn source_completes_when_expirations_are_pending_completion_waits_for_expirations_asynchronous() {
        source_completes_when_expirations_are_pending_completion_waits_for_expirations(SourceType::Asynchronous);
    }

    #[test]
    fn source_completes_when_expirations_are_pending_completion_waits_for_expirations_immediate() {
        source_completes_when_expirations_are_pending_completion_waits_for_expirations(SourceType::Immediate);
    }

    fn source_completes_when_no_expirations_are_pending_completion_propagates(source_type: SourceType) {
        // Setup
        let item = Item::new(1);

        let subject = Subject::<Item>::new();
        let source: Rc<dyn IObservable<Item>> = match source_type {
            SourceType::Asynchronous => subject.as_observable(),
            SourceType::Immediate => Observable::return_(item.clone()),
        };

        let scheduler = TestScheduler::new();

        // UUT Initialization & Action
        let (_subscription, results) = record(source.to_observable_change_set_with(select_lifetime(), None, Some(scheduler.scheduler())));

        if source_type == SourceType::Asynchronous {
            subject.on_next(item.clone());
            subject.on_completed();
        }

        assert!(results.error().is_none());
        if source_type == SourceType::Asynchronous {
            assert_eq!(results.recorded_change_sets().len(), 2, "1 item was emitted, after initialization");
        } else {
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
        }
        assert_eq!(results.recorded_items(), vec![item], "1 item was emitted");
        assert!(results.has_completed(), "the source has completed, and no items remain to be expired");
    }

    #[test]
    fn source_completes_when_no_expirations_are_pending_completion_propagates_asynchronous() {
        source_completes_when_no_expirations_are_pending_completion_propagates(SourceType::Asynchronous);
    }

    #[test]
    fn source_completes_when_no_expirations_are_pending_completion_propagates_immediate() {
        source_completes_when_no_expirations_are_pending_completion_propagates(SourceType::Immediate);
    }

    #[test]
    fn source_emits_items_items_are_added_and_removed_when_expired() {
        // Setup
        let source = Subject::<Item>::new();

        let scheduler = TestScheduler::new();

        // UUT Initialization
        let (_subscription, results) = record(source.as_observable().to_observable_change_set_with(select_lifetime(), None, Some(scheduler.scheduler())));

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
        assert_eq!(results.recorded_items(), vec![item1.clone()], "1 item was emitted");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        let item2 = Item::new(2);
        source.on_next(item2.clone());
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 item was emitted");
        assert_eq!(results.recorded_items(), vec![item1.clone(), item2.clone()], "1 item was emitted");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        let item3 = Item::with_lifetime(3, Duration::from_secs(1));
        source.on_next(item3.clone());
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 item was emitted");
        assert_eq!(results.recorded_items(), vec![item1.clone(), item2.clone(), item3], "1 item was emitted");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        scheduler.advance_to(seconds_ticks(1));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 1, "1 expiration should have occurred");
        assert_eq!(results.recorded_items(), vec![item1.clone(), item2.clone()], "1 item expired, and 1 had its lifetime extended");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        scheduler.advance_to(seconds_ticks(2));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 0, "no expirations should have occurred");
        assert_eq!(results.recorded_items(), vec![item1, item2.clone()], "no changes were made");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        scheduler.advance_to(seconds_ticks(3));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 1, "1 expiration should have occurred");
        assert_eq!(results.recorded_items(), vec![item2.clone()], "1 item reached its expiration");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        scheduler.advance_to(seconds_ticks(4));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(6).count(), 0, "no expirations should have occurred");
        assert_eq!(results.recorded_items(), vec![item2], "no changes were made");
        assert!(!results.has_completed(), "the source has not completed");
    }

    fn source_fails_error_propagates(source_type: SourceType) {
        // Setup
        let error = new_error("Test Exception");

        let subject = Subject::<Item>::new();
        let source: Rc<dyn IObservable<Item>> = match source_type {
            SourceType::Asynchronous => subject.as_observable(),
            SourceType::Immediate => Observable::throw::<Item>(error.clone()),
        };

        // UUT Initialization & Action
        let (_subscription, results) = record(source.to_observable_change_set());

        if source_type == SourceType::Asynchronous {
            subject.on_error(error.clone());
        }

        assert!(error_is(results.error(), &error), "errors should propagate");
        if source_type == SourceType::Asynchronous {
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
        } else {
            assert!(results.recorded_change_sets().is_empty(), "an error occurred during initialization");
        }
        assert!(results.recorded_items().is_empty(), "no items were emitted");
    }

    #[test]
    fn source_fails_error_propagates_asynchronous() {
        source_fails_error_propagates(SourceType::Asynchronous);
    }

    #[test]
    fn source_fails_error_propagates_immediate() {
        source_fails_error_propagates(SourceType::Immediate);
    }

    // Not ported: SourceIsNull_ThrowsException - argument validation

    // `ToObservableChangeSetFixture.Items.IntegrationTests`.

    // Not ported: MultipleSubscriptionsRunInParallel_SchedulerUsageIsThreadSafe - real threads
}

/// `ToObservableChangeSetFixture.Sequences.UnitTests`: the overloads on
/// `IObservable<IEnumerable<T>>`.
mod sequences {
    use super::*;
    use ferroui_dynamic_data::ObservableSequencesEx;

    // Not ported: ExpireAfterThrows_ErrorPropagates - the expiration selector cannot fail (no try_ form of to_observable_change_set_many_with)

    #[test]
    fn size_limit_is_exceeded_oldest_items_are_removed() {
        // Setup
        let source = Subject::<Vec<Item>>::new();

        // UUT Initialization
        let (_subscription, results) = record(source.as_observable().to_observable_change_set_many_with(None, Some(5), None));

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
        assert_eq!(results.recorded_items(), vec![item1.clone(), item2.clone(), item3.clone(), item4.clone()], "4 source items were emitted");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action: Limit is reached
        let item5 = Item::new(5);
        source.on_next(vec![item5.clone()]);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source operation was performed");
        assert_eq!(results.recorded_items(), vec![item1.clone(), item2.clone(), item3.clone(), item4.clone(), item5.clone()], "1 source item was emitted");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action: New item exceeds the limit
        let item6 = Item::new(6);
        source.on_next(vec![item6.clone()]);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 source operation was performed");
        assert_eq!(results.recorded_items(), vec![item2, item3, item4, item5, item6], "1 source item was emitted, and 1 was evicted");
        assert!(!results.has_completed(), "the source has not completed");
    }

    fn source_completes_when_expirations_are_pending_completion_waits_for_expirations(source_type: SourceType) {
        // Setup
        let items = vec![Item::with_lifetime(1, Duration::from_secs(10)), Item::new(2), Item::with_lifetime(3, Duration::from_secs(20))];

        let subject = Subject::<Vec<Item>>::new();
        let source: Rc<dyn IObservable<Vec<Item>>> = match source_type {
            SourceType::Asynchronous => subject.as_observable(),
            SourceType::Immediate => Observable::return_(items.clone()),
        };

        let scheduler = TestScheduler::new();

        // UUT Initialization & Action
        let (_subscription, results) = record(source.to_observable_change_set_many_with(select_lifetime(), None, Some(scheduler.scheduler())));

        if source_type == SourceType::Asynchronous {
            subject.on_next(items.clone());
            subject.on_completed();
        }

        assert!(results.error().is_none());
        if source_type == SourceType::Asynchronous {
            assert_eq!(results.recorded_change_sets().len(), 2, "1 sequence was emitted, after initialization");
        } else {
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
        }
        assert_eq!(results.recorded_items(), items, "3 items were emitted");
        assert!(!results.has_completed(), "2 items have yet to expire");

        // UUT Action
        scheduler.advance_to(seconds_ticks(30));

        assert!(results.error().is_none());
        if source_type == SourceType::Asynchronous {
            assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 2, "2 items should have expired, at different times");
        } else {
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 2, "2 items should have expired, at different times");
        }
        let without_lifetime: Vec<Item> = items.iter().filter(|item| item.lifetime.is_none()).cloned().collect();
        assert_eq!(results.recorded_items(), without_lifetime, "all expirable items have expired");
        assert!(results.has_completed(), "the source, and all outstanding expirations, have completed");
    }

    #[test]
    fn source_completes_when_expirations_are_pending_completion_waits_for_expirations_asynchronous() {
        source_completes_when_expirations_are_pending_completion_waits_for_expirations(SourceType::Asynchronous);
    }

    #[test]
    fn source_completes_when_expirations_are_pending_completion_waits_for_expirations_immediate() {
        source_completes_when_expirations_are_pending_completion_waits_for_expirations(SourceType::Immediate);
    }

    fn source_completes_when_no_expirations_are_pending_completion_propagates(source_type: SourceType) {
        // Setup
        let items = vec![Item::new(1), Item::new(2), Item::new(3)];

        let subject = Subject::<Vec<Item>>::new();
        let source: Rc<dyn IObservable<Vec<Item>>> = match source_type {
            SourceType::Asynchronous => subject.as_observable(),
            SourceType::Immediate => Observable::return_(items.clone()),
        };

        let scheduler = TestScheduler::new();

        // UUT Initialization & Action
        let (_subscription, results) = record(source.to_observable_change_set_many_with(select_lifetime(), None, Some(scheduler.scheduler())));

        if source_type == SourceType::Asynchronous {
            subject.on_next(items.clone());
            subject.on_completed();
        }

        assert!(results.error().is_none());
        if source_type == SourceType::Asynchronous {
            assert_eq!(results.recorded_change_sets().len(), 2, "1 sequence was emitted, after initialization");
        } else {
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
        }
        assert_eq!(results.recorded_items(), items, "3 items were emitted");
        assert!(results.has_completed(), "the source has completed, and no items remain to be expired");
    }

    #[test]
    fn source_completes_when_no_expirations_are_pending_completion_propagates_asynchronous() {
        source_completes_when_no_expirations_are_pending_completion_propagates(SourceType::Asynchronous);
    }

    #[test]
    fn source_completes_when_no_expirations_are_pending_completion_propagates_immediate() {
        source_completes_when_no_expirations_are_pending_completion_propagates(SourceType::Immediate);
    }

    #[test]
    fn source_emits_items_items_are_added_and_removed_when_expired() {
        // Setup
        let source = Subject::<Vec<Item>>::new();

        let scheduler = TestScheduler::new();

        // UUT Initialization
        let (_subscription, results) = record(source.as_observable().to_observable_change_set_many_with(select_lifetime(), None, Some(scheduler.scheduler())));

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
        assert_eq!(results.recorded_items(), vec![item1.clone(), item2.clone(), item3], "3 items were emitted");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        scheduler.advance_to(seconds_ticks(1));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 expiration should have occurred");
        assert_eq!(results.recorded_items(), vec![item1.clone(), item2.clone()], "1 item expired, and 1 had its lifetime extended");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        scheduler.advance_to(seconds_ticks(2));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 0, "no expirations should have occurred");
        assert_eq!(results.recorded_items(), vec![item1, item2.clone()], "no changes were made");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        scheduler.advance_to(seconds_ticks(3));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 expiration should have occurred");
        assert_eq!(results.recorded_items(), vec![item2.clone()], "1 item reached its expiration");
        assert!(!results.has_completed(), "the source has not completed");

        // UUT Action
        scheduler.advance_to(seconds_ticks(4));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 0, "no expirations should have occurred");
        assert_eq!(results.recorded_items(), vec![item2], "no changes were made");
        assert!(!results.has_completed(), "the source has not completed");
    }

    fn source_fails_error_propagates(source_type: SourceType) {
        // Setup
        let error = new_error("Test Exception");

        let subject = Subject::<Vec<Item>>::new();
        let source: Rc<dyn IObservable<Vec<Item>>> = match source_type {
            SourceType::Asynchronous => subject.as_observable(),
            SourceType::Immediate => Observable::throw::<Vec<Item>>(error.clone()),
        };

        // UUT Initialization & Action
        let (_subscription, results) = record(source.to_observable_change_set_many());

        if source_type == SourceType::Asynchronous {
            subject.on_error(error.clone());
        }

        assert!(error_is(results.error(), &error), "errors should propagate");
        if source_type == SourceType::Asynchronous {
            assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should always be emitted");
        } else {
            assert!(results.recorded_change_sets().is_empty(), "an error occurred during initialization");
        }
        assert!(results.recorded_items().is_empty(), "no source items were emitted");
    }

    #[test]
    fn source_fails_error_propagates_asynchronous() {
        source_fails_error_propagates(SourceType::Asynchronous);
    }

    #[test]
    fn source_fails_error_propagates_immediate() {
        source_fails_error_propagates(SourceType::Immediate);
    }

    // Not ported: SourceIsNull_ThrowsException - argument validation

    // `ToObservableChangeSetFixture.Sequences.IntegrationTests`.

    // Not ported: MultipleSubscriptionsRunInParallel_SchedulerUsageIsThreadSafe - real threads
}
