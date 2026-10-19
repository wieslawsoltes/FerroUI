//! `ExpireAfterFixture` (`DynamicData.Tests/List/ExpireAfterFixture.cs`): the removal of the items
//! of a source list once their times have passed, at their due times or at a polling interval, the
//! rescheduling on replacements, an inaccurate scheduler, and the propagation of the end and of
//! errors of the source.
//!
//! Times: upstream's `DateTimeOffset.FromUnixTimeMilliseconds(ms)` is [`unix_ms`] (a
//! `SystemTime`), and its `.Ticks` on the `TestScheduler` is [`unix_ms_ticks`]: the clock of the
//! port's `TestScheduler` counts ticks from the epoch of `SystemTime`.

use crate::utilities::{assert_equivalent_because, FakeScheduler, ObservableExtensions, TestSourceList};
use ferroui_dynamic_data::list::{IObservableList, SourceListEditConvenienceEx, SourceListEx};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive_testing::{duration_to_ticks, new_error, TestScheduler};
use ferroui_system::ObservableError;
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// `TestItem`: an item with an identifier and a mutable expiration, equal only to itself (a class
/// that does not override `Equals`).
struct TestItem {
    id: i32,
    expiration: Cell<Option<SystemTime>>,
}

impl PartialEq for TestItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Debug for TestItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestItem").field("id", &self.id).field("expiration", &self.expiration.get()).finish()
    }
}

type Item = Rc<TestItem>;

/// `new TestItem() { Id = id, Expiration = DateTimeOffset.FromUnixTimeMilliseconds(ms) }`.
fn item(id: i32, expiration_ms: u64) -> Item {
    Rc::new(TestItem { id, expiration: Cell::new(Some(unix_ms(expiration_ms))) })
}

/// `new TestItem() { Id = id }`: an item that never expires.
fn item_without_expiration(id: i32) -> Item {
    Rc::new(TestItem { id, expiration: Cell::new(None) })
}

/// `DateTimeOffset.FromUnixTimeMilliseconds(ms)`.
fn unix_ms(ms: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_millis(ms)
}

/// `DateTimeOffset.FromUnixTimeMilliseconds(ms).Ticks`, on the clock of the `TestScheduler`.
fn unix_ms_ticks(ms: u64) -> i64 {
    duration_to_ticks(Duration::from_millis(ms))
}

/// `DateTimeOffset.MaxValue.Ticks`.
///
/// Deviation (DEVIATIONS.md, DynamicData): the largest time that the clock of the port's
/// `TestScheduler` converts to a `SystemTime` (292 years after the epoch), where upstream advances
/// to the year 9999.
const MAX_TICKS: i64 = i64::MAX / 100;

/// The items as the vector that the source and the recorded values hold.
fn items_of(items: &[&Item]) -> Vec<Item> {
    items.iter().map(|item| Rc::clone(item)).collect()
}

/// `CreateTestScheduler`: a scheduler at the epoch.
fn create_test_scheduler() -> TestScheduler {
    let scheduler = TestScheduler::new();
    scheduler.advance_to(unix_ms_ticks(0));
    scheduler
}

/// `CreateTimeSelector`: the time from now until the expiration of the item.
///
/// Deviation (DEVIATIONS.md, DynamicData): an expiration already passed is a lifetime of zero,
/// where upstream's `TimeSpan` is negative; both expire the item at the next management.
fn create_time_selector(scheduler: Rc<dyn IScheduler>) -> impl Fn(&Item) -> Option<Duration> + 'static {
    move |item: &Item| item.expiration.get().map(|expiration| expiration.duration_since(scheduler.now()).unwrap_or(Duration::ZERO))
}

fn error_is(actual: Option<ObservableError>, expected: &ObservableError) -> bool {
    actual.is_some_and(|actual| Rc::ptr_eq(&actual, expected))
}

#[test]
fn item_is_removed_before_expiration_expiration_is_cancelled() {
    let source = TestSourceList::<Item>::new();

    let scheduler = create_test_scheduler();

    let (_subscription, results) =
        source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

    let item1 = item(1, 10);
    let item2 = item(2, 10);
    let item3 = item(3, 10);
    let item4 = item(4, 10);
    let item5 = item(5, 10);
    let item6 = item(6, 10);
    source.add_range(items_of(&[&item1, &item2, &item3, &item4, &item5, &item6]));
    scheduler.advance_by(1);

    let item7 = item_without_expiration(7);
    source.add(item7.clone());
    scheduler.advance_by(1);

    source.remove(&item2);
    scheduler.advance_by(1);

    // item4 and item5
    source.remove_range(2, 2);
    scheduler.advance_by(1);

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no items should have expired");
    assert_equivalent_because(&source.items(), &items_of(&[&item1, &item3, &item6, &item7]), "7 items were added, and 3 were removed");

    scheduler.advance_to(unix_ms_ticks(10));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().len(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[0], &items_of(&[&item1, &item3, &item6]), "items #1, #3, and #6 should have expired");
    assert_equivalent_because(&source.items(), &items_of(&[&item7]), "items #1 and #3 should have been removed");

    assert!(!results.has_completed());
}

#[test]
fn next_item_to_expire_is_replaced_expiration_is_rescheduled_if_needed() {
    let source = TestSourceList::<Item>::new();

    let scheduler = create_test_scheduler();

    let (_subscription, results) =
        source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

    let item1 = item(1, 10);
    source.add(item1.clone());
    scheduler.advance_by(1);

    // Extend the expiration to a later time
    let item2 = item(2, 20);
    source.replace(&item1, item2.clone());
    scheduler.advance_by(1);

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert_equivalent_because(&source.items(), &items_of(&[&item2]), "item #1 was added, and then replaced");

    scheduler.advance_to(unix_ms_ticks(10));

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert_equivalent_because(&source.items(), &items_of(&[&item2]), "no changes should have occurred");

    // Shorten the expiration to an earlier time
    let item3 = item(3, 15);
    source.replace(&item2, item3.clone());
    scheduler.advance_by(1);

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert_equivalent_because(&source.items(), &items_of(&[&item3]), "item #2 was replaced");

    // One more update with no changes to the expiration
    let item4 = item(4, 15);
    source.replace(&item3, item4.clone());
    scheduler.advance_by(1);

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert_equivalent_because(&source.items(), &items_of(&[&item4]), "item #3 was replaced");

    scheduler.advance_to(unix_ms_ticks(15));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().len(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[0], &items_of(&[&item4]), "item #4 should have expired");
    assert!(source.items().is_empty(), "item #4 should have expired");

    scheduler.advance_to(MAX_TICKS);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(1).count(), 0, "no expirations should have occurred");
    assert!(source.items().is_empty(), "no changes should have occurred");

    assert!(!results.has_completed());
}

#[test]
fn polling_interval_is_given_removals_are_scheduled_at_interval() {
    let source = TestSourceList::<Item>::new();

    let scheduler = create_test_scheduler();

    let (_subscription, results) = source
        .expire_after_with_polling_on(create_time_selector(scheduler.scheduler()), Some(Duration::from_millis(20)), scheduler.scheduler())
        .validate_synchronization()
        .record_values_on(scheduler.scheduler());

    let item1 = item(1, 10);
    let item2 = item(2, 20);
    let item3 = item(3, 30);
    let item4 = item(4, 40);
    let item5 = item(5, 100);
    source.add_range(items_of(&[&item1, &item2, &item3, &item4, &item5]));
    scheduler.advance_by(1);

    // Additional expirations at 20ms.
    let item6 = item(6, 20);
    let item7 = item(7, 20);
    source.add_range(items_of(&[&item6, &item7]));
    scheduler.advance_by(1);

    // Out-of-order expiration
    let item8 = item(8, 15);
    source.add(item8.clone());
    scheduler.advance_by(1);

    // Non-expiring item
    let item9 = item_without_expiration(9);
    source.add(item9.clone());
    scheduler.advance_by(1);

    // Replacement changing lifetime.
    let item10 = item(4, 45);
    source.replace(&item4, item10.clone());
    scheduler.advance_by(1);

    // Replacement not-affecting lifetime.
    let item11 = item(5, 100);
    source.replace(&item5, item11.clone());
    scheduler.advance_by(1);

    // Move should not affect scheduled expiration.
    item3.expiration.set(Some(unix_ms(55)));
    source.move_item(2, 3);
    scheduler.advance_by(1);

    // Not testing Refresh changes, since ISourceList<T> doesn't actually provide an API to generate them.

    // Verify initial state, after all emissions
    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert_eq!(
        source.items(),
        items_of(&[&item1, &item2, &item10, &item3, &item11, &item6, &item7, &item8, &item9]),
        "9 items were added, 2 were replaced, and 1 was refreshed"
    );

    // Item scheduled to expire at 10ms, but won't be picked up yet
    scheduler.advance_to(unix_ms_ticks(10));

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert_eq!(source.items(), items_of(&[&item1, &item2, &item10, &item3, &item11, &item6, &item7, &item8, &item9]), "no changes should have occurred");

    // Item scheduled to expire at 15ms, but won't be picked up yet
    scheduler.advance_to(unix_ms_ticks(15));

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert_eq!(source.items(), items_of(&[&item1, &item2, &item10, &item3, &item11, &item6, &item7, &item8, &item9]), "no changes should have occurred");

    // Expired items should be polled
    scheduler.advance_to(unix_ms_ticks(20));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().len(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[0], &items_of(&[&item1, &item2, &item6, &item7, &item8]), "items #1, #2, #6, #7, and #8 should have expired");
    assert_eq!(source.items(), items_of(&[&item10, &item3, &item11, &item9]), "items #1, #2, #6, #7, and #8 should have been removed");

    // Item scheduled to expire at 30ms, but won't be picked up yet
    scheduler.advance_to(unix_ms_ticks(30));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(1).count(), 0, "no expirations should have occurred");
    assert_eq!(source.items(), items_of(&[&item10, &item3, &item11, &item9]), "no changes should have occurred");

    // Expired items should be polled, but should exclude the one that was changed from 40ms to 45ms.
    scheduler.advance_to(unix_ms_ticks(40));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(1).count(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[1], &items_of(&[&item3]), "item #3 should have expired");
    assert_eq!(source.items(), items_of(&[&item10, &item11, &item9]), "item #3 should have been removed");

    // Item scheduled to expire at 45ms, but won't be picked up yet
    scheduler.advance_to(unix_ms_ticks(45));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(2).count(), 0, "no expirations should have occurred");
    assert_eq!(source.items(), items_of(&[&item10, &item11, &item9]), "no changes should have occurred");

    // Expired items should be polled
    scheduler.advance_to(unix_ms_ticks(60));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(2).count(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[2], &items_of(&[&item10]), "item #10 should have expired");
    assert_equivalent_because(&source.items(), &items_of(&[&item11, &item9]), "item #10 should have been removed");

    // Expired items should be polled, but none should be found
    scheduler.advance_to(unix_ms_ticks(80));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(3).count(), 0, "no expirations should have occurred");
    assert_eq!(source.items(), items_of(&[&item11, &item9]), "no changes should have occurred");

    // Expired items should be polled
    scheduler.advance_to(unix_ms_ticks(100));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(3).count(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[3], &items_of(&[&item11]), "item #11 should have expired");
    assert_eq!(source.items(), items_of(&[&item9]), "item #11 should have been removed");

    // Next poll should not find anything to expire.
    scheduler.advance_to(unix_ms_ticks(120));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(4).count(), 0, "no expirations should have occurred");
    assert_eq!(source.items(), items_of(&[&item9]), "no changes should have occurred");

    assert!(!results.has_completed());
}

#[test]
fn polling_interval_is_not_given_removals_are_scheduled_immediately() {
    let source = TestSourceList::<Item>::new();

    let scheduler = create_test_scheduler();

    let (_subscription, results) =
        source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

    let item1 = item(1, 10);
    let item2 = item(2, 20);
    let item3 = item(3, 30);
    let item4 = item(4, 40);
    let item5 = item(5, 50);
    source.add_range(items_of(&[&item1, &item2, &item3, &item4, &item5]));
    scheduler.advance_by(1);

    // Additional expirations at 20ms.
    let item6 = item(6, 20);
    let item7 = item(7, 20);
    source.add_range(items_of(&[&item6, &item7]));
    scheduler.advance_by(1);

    // Out-of-order expiration
    let item8 = item(8, 15);
    source.add(item8.clone());
    scheduler.advance_by(1);

    // Non-expiring item
    let item9 = item_without_expiration(9);
    source.add(item9.clone());
    scheduler.advance_by(1);

    // Replacement changing lifetime.
    let item10 = item(10, 45);
    source.replace(&item4, item10.clone());
    scheduler.advance_by(1);

    // Replacement not-affecting lifetime.
    let item11 = item(11, 50);
    source.replace(&item5, item11.clone());
    scheduler.advance_by(1);

    // Moved items should still expire correctly, but its expiration time should not change.
    item3.expiration.set(Some(unix_ms(55)));
    source.move_item(2, 3);
    scheduler.advance_by(1);

    // Verify initial state, after all emissions
    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert_eq!(
        source.items(),
        items_of(&[&item1, &item2, &item10, &item3, &item11, &item6, &item7, &item8, &item9]),
        "9 items were added, 2 were replaced, and 1 was moved"
    );

    scheduler.advance_to(unix_ms_ticks(10));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().len(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[0], &items_of(&[&item1]), "item #1 should have expired");
    assert_eq!(source.items(), items_of(&[&item2, &item10, &item3, &item11, &item6, &item7, &item8, &item9]), "item #1 should have been removed");

    scheduler.advance_to(unix_ms_ticks(15));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(1).count(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[1], &items_of(&[&item8]), "item #8 should have expired");
    assert_eq!(source.items(), items_of(&[&item2, &item10, &item3, &item11, &item6, &item7, &item9]), "item #8 should have expired");

    scheduler.advance_to(unix_ms_ticks(20));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(2).count(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[2], &items_of(&[&item2, &item6, &item7]), "items #2, #6, and #7 should have expired");
    assert_eq!(source.items(), items_of(&[&item10, &item3, &item11, &item9]), "items #2, #6, and #7 should have been removed");

    scheduler.advance_to(unix_ms_ticks(30));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(3).count(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[3], &items_of(&[&item3]), "item #3 should have expired");
    assert_eq!(source.items(), items_of(&[&item10, &item11, &item9]), "item #3 should have been removed");

    scheduler.advance_to(unix_ms_ticks(40));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(4).count(), 0, "no expirations should have occurred");
    assert_eq!(source.items(), items_of(&[&item10, &item11, &item9]), "no changes should have occurred");

    scheduler.advance_to(unix_ms_ticks(45));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(4).count(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[4], &items_of(&[&item10]), "item #10 should have expired");
    assert_eq!(source.items(), items_of(&[&item11, &item9]), "item #10 should have expired");

    scheduler.advance_to(unix_ms_ticks(50));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(5).count(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[5], &items_of(&[&item11]), "item #11 should have expired");
    assert_eq!(source.items(), items_of(&[&item9]), "item #11 should have expired");

    scheduler.advance_to(MAX_TICKS);

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().iter().skip(6).count(), 0, "no expirations should have occurred");
    assert_eq!(source.items(), items_of(&[&item9]), "no changes should have occurred");

    assert!(!results.has_completed());
}

// Covers https://github.com/reactivemarbles/DynamicData/issues/716
#[test]
fn scheduler_is_inaccurate_removals_are_not_skipped() {
    let source = TestSourceList::<Item>::new();

    let scheduler = FakeScheduler::new();
    scheduler.set_now(unix_ms(0));

    let (_subscription, results) =
        source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

    let item1 = item(1, 10);
    source.add(item1.clone());

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert_equivalent_because(&source.items(), &items_of(&[&item1]), "1 item was added");

    // `SimulateUntilIdle(inaccuracyOffset: TimeSpan.FromMilliseconds(-1))`.
    scheduler.simulate_until_idle_with(-duration_to_ticks(Duration::from_millis(1)));

    assert!(results.error().is_none());
    assert_eq!(results.recorded_values().len(), 1, "1 expiration should have occurred");
    assert_equivalent_because(&results.recorded_values()[0], &items_of(&[&item1]), "item #1 should have expired");
    assert!(source.items().is_empty(), "item #1 should have been removed");

    assert!(!results.has_completed());
}

#[test]
fn source_completes_completion_is_propagated() {
    let source = TestSourceList::<Item>::new();

    let scheduler = create_test_scheduler();

    let (_subscription, results) =
        source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

    source.add(item(1, 10));
    scheduler.advance_by(1);

    source.complete();

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert!(results.has_completed());

    // Ensure that the operator does not attept to continue removing items.
    scheduler.advance_to(unix_ms_ticks(10));
}

#[test]
fn source_completes_immediately_completion_is_propagated() {
    let source = TestSourceList::<Item>::new();

    let scheduler = create_test_scheduler();

    let item1 = item(1, 10);
    source.add(item1.clone());
    scheduler.advance_by(1);

    source.complete();

    let (_subscription, results) =
        source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

    assert!(results.error().is_none());
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert!(results.has_completed());
    assert_equivalent_because(&source.items(), &items_of(&[&item1]), "no changes should have occurred");
}

#[test]
fn source_errors_error_is_propagated() {
    let source = TestSourceList::<Item>::new();

    let scheduler = create_test_scheduler();

    let (_subscription, results) =
        source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

    source.add(item(1, 10));
    scheduler.advance_by(1);

    let error = new_error("This is a test");
    source.set_error(error.clone());

    assert!(error_is(results.error(), &error), "an error was published");
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert!(!results.has_completed());

    // Ensure that the operator does not attept to continue removing items.
    scheduler.advance_to(unix_ms_ticks(10));
}

#[test]
fn source_errors_immediately_error_is_propagated() {
    let source = TestSourceList::<Item>::new();

    let scheduler = create_test_scheduler();

    let item1 = item(1, 10);
    source.add(item1.clone());
    scheduler.advance_by(1);

    let error = new_error("This is a test");
    source.set_error(error.clone());

    let (_subscription, results) =
        source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

    assert!(error_is(results.error(), &error), "an error was published");
    assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
    assert!(!results.has_completed());
    assert_equivalent_because(&source.items(), &items_of(&[&item1]), "no changes should have occurred");

    // Ensure that the operator does not attept to continue removing items.
    scheduler.advance_to(unix_ms_ticks(10));
}

// Not ported: SourceIsNull_ThrowsException - argument validation

// Not ported: ThreadPoolSchedulerIsUsedWithoutPolling_ExpirationIsThreadSafe - real threads

// Not ported: ThreadPoolSchedulerIsUsedWithPolling_ExpirationIsThreadSafe - real threads

// Not ported: TimeSelectorIsNull_ThrowsException - argument validation

// Not ported: TimeSelectorThrows_ThrowsException - the time selector cannot fail (no try_ form of expire_after)
