//! `ExpireAfterFixture` (`DynamicData.Tests/Cache/ExpireAfterFixture.cs`,
//! `ExpireAfterFixture.ForSource.cs` and `ExpireAfterFixture.ForStream.cs`): the tests of
//! `ExpireAfter` on a source cache (module [`for_source`]) and on a stream of change sets (module
//! [`for_stream`]).
//!
//! The clock of the port's `TestScheduler` starts at the Unix epoch (tick 0), so upstream's
//! `DateTimeOffset.FromUnixTimeMilliseconds(n).Ticks` is [`ticks_at`]`(n)` and its
//! `DateTimeOffset.FromUnixTimeMilliseconds(n)` is [`ms`]`(n)`.
//!
//! Upstream's `StressItem` is used only by the stress tests on the thread pool, which are not
//! ported (real threads), so it is left out.

use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// `DateTimeOffset.MaxValue.Ticks`: the latest clock of the test scheduler that still converts to
/// a `SystemTime` (100 nanoseconds per tick in a `u64` of nanoseconds), far past every expiration
/// of the tests.
const MAX_TICKS: i64 = i64::MAX / 100;

/// `DateTimeOffset.FromUnixTimeMilliseconds(milliseconds)`.
fn ms(milliseconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_millis(milliseconds)
}

/// `DateTimeOffset.FromUnixTimeMilliseconds(milliseconds).Ticks`, on the clock of the port's test
/// scheduler.
fn ticks_at(milliseconds: u64) -> i64 {
    duration_to_ticks(Duration::from_millis(milliseconds))
}

/// `CreateTestScheduler`: a test scheduler at the Unix epoch.
fn create_test_scheduler() -> TestScheduler {
    let scheduler = TestScheduler::new();
    scheduler.advance_to(ticks_at(0));
    scheduler
}

/// `CreateTimeSelector`: the time from the clock of `scheduler` to the expiration of the item.
///
/// Upstream's `TimeSpan?` may be negative for an expiration in the past; a `Duration` cannot, so
/// an expiration at or before the clock gives zero (still `Some`: the item expires).
fn create_time_selector(scheduler: Rc<dyn IScheduler>) -> impl Fn(&TestItem) -> Option<Duration> + 'static {
    move |item: &TestItem| item.expiration().map(|expiration| expiration.duration_since(scheduler.now()).unwrap_or(Duration::ZERO))
}

struct TestItemData {
    id: i32,
    expiration: Cell<Option<SystemTime>>,
}

/// `TestItem`: a class with an id and a mutable expiration. A clone is a handle to the same
/// object, and two items are equal only when they are the same object, as upstream's class (the
/// default equality that the validators of change sets use).
///
/// Upstream's `BeEquivalentTo` compares these items member by member; the port compares them by
/// identity, which is stricter, and holds since every compared item is the very instance expected.
#[derive(Clone)]
struct TestItem(Rc<TestItemData>);

impl TestItem {
    /// `new TestItem() { Id = id, Expiration = DateTimeOffset.FromUnixTimeMilliseconds(ms) }`;
    /// `None` for no expiration.
    fn new(id: i32, expiration_ms: Option<u64>) -> Self {
        Self(Rc::new(TestItemData { id, expiration: Cell::new(expiration_ms.map(ms)) }))
    }

    /// `Id`.
    fn id(&self) -> i32 {
        self.0.id
    }

    /// `Expiration`.
    fn expiration(&self) -> Option<SystemTime> {
        self.0.expiration.get()
    }

    /// `Expiration`'s setter, in milliseconds since the Unix epoch.
    fn set_expiration(&self, expiration_ms: Option<u64>) {
        self.0.expiration.set(expiration_ms.map(ms));
    }
}

impl PartialEq for TestItem {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for TestItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let expiration = self.expiration().map(|expiration| expiration.duration_since(SystemTime::UNIX_EPOCH).map(|since| since.as_millis()).unwrap_or(0));
        f.debug_struct("TestItem").field("id", &self.id()).field("expiration_ms", &expiration).finish()
    }
}

/// `items.Select(item => new KeyValuePair<int, TestItem>(item.Id, item))`.
fn key_values(items: &[&TestItem]) -> Vec<(i32, TestItem)> {
    items.iter().map(|item| (item.id(), (*item).clone())).collect()
}

/// `ExpireAfterFixture.ForSource`.
mod for_source {
    use super::{create_test_scheduler, create_time_selector, key_values, ticks_at, TestItem, MAX_TICKS};
    use crate::utilities::{assert_equivalent_because, FakeScheduler, ObservableExtensions, TestSourceCache};
    use ferroui_dynamic_data::cache::{IObservableCache, SourceCacheEx};
    use ferroui_reactive_testing::new_error;
    use std::rc::Rc;
    use std::time::Duration;

    /// `CreateTestSource`.
    fn create_test_source() -> TestSourceCache<TestItem, i32> {
        TestSourceCache::new(|item: &TestItem| item.id())
    }

    #[test]
    fn item_is_removed_before_expiration_expiration_is_cancelled() {
        let source = create_test_source();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        let item3 = TestItem::new(3, Some(10));
        let item2 = TestItem::new(2, Some(10));
        source.add_or_update_items(vec![item1.clone(), item2.clone(), item3.clone()]);
        scheduler.advance_by(1);

        let item4 = TestItem::new(4, None);
        source.add_or_update(item4.clone());
        scheduler.advance_by(1);

        source.remove_key(2);
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no items should have expired");
        assert_equivalent_because(&source.items(), &[item1.clone(), item3.clone(), item4.clone()], "3 items were added, and one was removed");

        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.len(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[0], &key_values(&[&item1, &item3]), "items #1 and #3 should have expired");
        assert_equivalent_because(&source.items(), &[item4.clone()], "items #1 and #3 should have been removed");

        assert!(!results.has_completed());
    }

    #[test]
    fn next_item_to_expire_is_replaced_expiration_is_rescheduled_if_needed() {
        let source = create_test_source();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        source.add_or_update(item1.clone());
        scheduler.advance_by(1);

        // Extend the expiration to a later time
        let item2 = TestItem::new(1, Some(20));
        source.add_or_update(item2.clone());
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item2.clone()], "item #1 was added, and then replaced");

        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item2.clone()], "no changes should have occurred");

        // Shorten the expiration to an earlier time
        let item3 = TestItem::new(1, Some(15));
        source.add_or_update(item3.clone());
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item3.clone()], "item #1 was replaced");

        // One more update with no changes to the expiration
        let item4 = TestItem::new(1, Some(15));
        source.add_or_update(item4.clone());
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item4.clone()], "item #1 was replaced");

        scheduler.advance_to(ticks_at(15));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.len(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[0], &key_values(&[&item4]), "item #1 should have expired");
        assert!(source.items().is_empty(), "item #1 should have expired");

        scheduler.advance_to(MAX_TICKS);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_values().iter().skip(1).count(), 0, "no expirations should have occurred");
        assert!(source.items().is_empty(), "no changes should have occurred");

        assert!(!results.has_completed());
    }

    #[test]
    fn polling_interval_is_given_removals_are_scheduled_at_interval() {
        let source = create_test_source();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .expire_after_with_polling_on(create_time_selector(scheduler.scheduler()), Some(Duration::from_millis(20)), scheduler.scheduler())
            .validate_synchronization()
            .record_values_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        let item2 = TestItem::new(2, Some(20));
        let item3 = TestItem::new(3, Some(30));
        let item4 = TestItem::new(4, Some(40));
        let item5 = TestItem::new(5, Some(100));
        source.add_or_update_items(vec![item1.clone(), item2.clone(), item3.clone(), item4.clone(), item5.clone()]);
        scheduler.advance_by(1);

        // Additional expirations at 20ms.
        let item6 = TestItem::new(6, Some(20));
        let item7 = TestItem::new(7, Some(20));
        source.add_or_update_items(vec![item6.clone(), item7.clone()]);
        scheduler.advance_by(1);

        // Out-of-order expiration
        let item8 = TestItem::new(8, Some(15));
        source.add_or_update(item8.clone());
        scheduler.advance_by(1);

        // Non-expiring item
        let item9 = TestItem::new(9, None);
        source.add_or_update(item9.clone());
        scheduler.advance_by(1);

        // Replacement changing lifetime.
        let item10 = TestItem::new(4, Some(45));
        source.add_or_update(item10.clone());
        scheduler.advance_by(1);

        // Replacement not-affecting lifetime.
        let item11 = TestItem::new(5, Some(100));
        source.add_or_update(item11.clone());
        scheduler.advance_by(1);

        // Refresh should not affect scheduled expiration.
        item3.set_expiration(Some(55));
        source.refresh(&item3);
        scheduler.advance_by(1);

        // Not testing Move changes, since ISourceCache<T> doesn't actually provide an API to generate them.

        // Verify initial state, after all emissions
        let all_items = [item1.clone(), item2.clone(), item3.clone(), item6.clone(), item7.clone(), item8.clone(), item9.clone(), item10.clone(), item11.clone()];
        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &all_items, "9 items were added, 2 were replaced, and 1 was refreshed");

        // Item scheduled to expire at 10ms, but won't be picked up yet
        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &all_items, "no changes should have occurred");

        // Item scheduled to expire at 15ms, but won't be picked up yet
        scheduler.advance_to(ticks_at(15));

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &all_items, "no changes should have occurred");

        // Expired items should be polled
        scheduler.advance_to(ticks_at(20));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.len(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[0], &key_values(&[&item1, &item2, &item6, &item7, &item8]), "items #1, #2, #6, #7, and #8 should have expired");
        assert_equivalent_because(&source.items(), &[item3.clone(), item9.clone(), item10.clone(), item11.clone()], "items #1, #2, #6, #7, and #8 should have been removed");

        // Item scheduled to expire at 30ms, but won't be picked up yet
        scheduler.advance_to(ticks_at(30));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_values().iter().skip(1).count(), 0, "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item3.clone(), item9.clone(), item10.clone(), item11.clone()], "no changes should have occurred");

        // Expired items should be polled, but should exclude the one that was changed from 40ms to 45ms.
        scheduler.advance_to(ticks_at(40));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.iter().skip(1).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[1], &key_values(&[&item3]), "item #3 should have expired");
        assert_equivalent_because(&source.items(), &[item9.clone(), item10.clone(), item11.clone()], "item #3 should have been removed");

        // Item scheduled to expire at 45ms, but won't be picked up yet
        scheduler.advance_to(ticks_at(45));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_values().iter().skip(2).count(), 0, "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item9.clone(), item10.clone(), item11.clone()], "no changes should have occurred");

        // Expired items should be polled
        scheduler.advance_to(ticks_at(60));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.iter().skip(2).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[2], &key_values(&[&item10]), "item #10 should have expired");
        assert_equivalent_because(&source.items(), &[item9.clone(), item11.clone()], "item #10 should have been removed");

        // Expired items should be polled, but none should be found
        scheduler.advance_to(ticks_at(80));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_values().iter().skip(3).count(), 0, "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item9.clone(), item11.clone()], "no changes should have occurred");

        // Expired items should be polled
        scheduler.advance_to(ticks_at(100));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.iter().skip(3).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[3], &key_values(&[&item11]), "item #11 should have expired");
        assert_equivalent_because(&source.items(), &[item9.clone()], "item #11 should have been removed");

        // Next poll should not find anything to expire.
        scheduler.advance_to(ticks_at(120));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_values().iter().skip(4).count(), 0, "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item9.clone()], "no changes should have occurred");

        assert!(!results.has_completed());
    }

    #[test]
    fn polling_interval_is_not_given_removals_are_scheduled_immediately() {
        let source = create_test_source();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        let item2 = TestItem::new(2, Some(20));
        let item3 = TestItem::new(3, Some(30));
        let item4 = TestItem::new(4, Some(40));
        let item5 = TestItem::new(5, Some(50));
        source.add_or_update_items(vec![item1.clone(), item2.clone(), item3.clone(), item4.clone(), item5.clone()]);
        scheduler.advance_by(1);

        // Additional expirations at 20ms.
        let item6 = TestItem::new(6, Some(20));
        let item7 = TestItem::new(7, Some(20));
        source.add_or_update_items(vec![item6.clone(), item7.clone()]);
        scheduler.advance_by(1);

        // Out-of-order expiration
        let item8 = TestItem::new(8, Some(15));
        source.add_or_update(item8.clone());
        scheduler.advance_by(1);

        // Non-expiring item
        let item9 = TestItem::new(9, None);
        source.add_or_update(item9.clone());
        scheduler.advance_by(1);

        // Replacement changing lifetime.
        let item10 = TestItem::new(4, Some(45));
        source.add_or_update(item10.clone());
        scheduler.advance_by(1);

        // Replacement not-affecting lifetime.
        let item11 = TestItem::new(5, Some(50));
        source.add_or_update(item11.clone());
        scheduler.advance_by(1);

        // Refresh should not affect scheduled expiration.
        item3.set_expiration(Some(55));
        source.refresh(&item3);
        scheduler.advance_by(1);

        // Not testing Move changes, since ISourceCache<T> doesn't actually provide an API to generate them.

        // Verify initial state, after all emissions
        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert_equivalent_because(
            &source.items(),
            &[item1.clone(), item2.clone(), item3.clone(), item6.clone(), item7.clone(), item8.clone(), item9.clone(), item10.clone(), item11.clone()],
            "11 items were added, 2 were replaced, and 1 was refreshed",
        );

        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.len(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[0], &key_values(&[&item1]), "item #1 should have expired");
        assert_equivalent_because(
            &source.items(),
            &[item2.clone(), item3.clone(), item6.clone(), item7.clone(), item8.clone(), item9.clone(), item10.clone(), item11.clone()],
            "item #1 should have been removed",
        );

        scheduler.advance_to(ticks_at(15));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.iter().skip(1).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[1], &key_values(&[&item8]), "item #8 should have expired");
        assert_equivalent_because(&source.items(), &[item2.clone(), item3.clone(), item6.clone(), item7.clone(), item9.clone(), item10.clone(), item11.clone()], "item #8 should have expired");

        scheduler.advance_to(ticks_at(20));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.iter().skip(2).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[2], &key_values(&[&item2, &item6, &item7]), "items #2, #6, and #7 should have expired");
        assert_equivalent_because(&source.items(), &[item3.clone(), item9.clone(), item10.clone(), item11.clone()], "items #2, #6, and #7 should have been removed");

        scheduler.advance_to(ticks_at(30));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.iter().skip(3).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[3], &key_values(&[&item3]), "item #3 should have expired");
        assert_equivalent_because(&source.items(), &[item9.clone(), item10.clone(), item11.clone()], "item #3 should have been removed");

        scheduler.advance_to(ticks_at(40));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_values().iter().skip(4).count(), 0, "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item9.clone(), item10.clone(), item11.clone()], "no changes should have occurred");

        scheduler.advance_to(ticks_at(45));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.iter().skip(4).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[4], &key_values(&[&item10]), "item #10 should have expired");
        assert_equivalent_because(&source.items(), &[item9.clone(), item11.clone()], "item #10 should have expired");

        scheduler.advance_to(ticks_at(50));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.iter().skip(5).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[5], &key_values(&[&item11]), "item #11 should have expired");
        assert_equivalent_because(&source.items(), &[item9.clone()], "item #11 should have expired");

        // Remaining item should never expire
        scheduler.advance_to(MAX_TICKS);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_values().iter().skip(6).count(), 0, "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item9.clone()], "no changes should have occurred");

        assert!(!results.has_completed());
    }

    // Covers https://github.com/reactivemarbles/DynamicData/issues/716
    #[test]
    fn scheduler_is_inaccurate_removals_are_not_skipped() {
        let source = create_test_source();

        let scheduler = FakeScheduler::new();
        scheduler.set_now(super::ms(0));

        let (_subscription, results) = source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        source.add_or_update(item1.clone());

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert_equivalent_because(&source.items(), &[item1.clone()], "1 item was added");

        // TimeSpan.FromMilliseconds(-1), in ticks.
        scheduler.simulate_until_idle_with(-ferroui_reactive_testing::duration_to_ticks(Duration::from_millis(1)));

        assert!(results.error().is_none());
        let values = results.recorded_values();
        assert_eq!(values.len(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&values[0], &key_values(&[&item1]), "item #1 should have expired");
        assert!(source.items().is_empty(), "item #1 should have been removed");

        assert!(!results.has_completed());
    }

    #[test]
    fn source_completes_completion_is_propagated() {
        let source = create_test_source();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

        source.add_or_update(TestItem::new(1, Some(10)));
        scheduler.advance_by(1);

        source.complete();

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert!(results.has_completed());

        // Ensure that the operator does not attept to continue removing items.
        scheduler.advance_to(ticks_at(10));
    }

    #[test]
    fn source_completes_immediately_completion_is_propagated() {
        let source = create_test_source();

        let scheduler = create_test_scheduler();

        let item1 = TestItem::new(1, Some(10));
        source.add_or_update(item1.clone());
        scheduler.advance_by(1);

        source.complete();

        let (_subscription, results) = source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

        assert!(results.error().is_none());
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert!(results.has_completed());
        assert_equivalent_because(&source.items(), &[item1.clone()], "no changes should have occurred");
    }

    #[test]
    fn source_errors_error_is_propagated() {
        let source = create_test_source();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

        source.add_or_update(TestItem::new(1, Some(10)));
        scheduler.advance_by(1);

        let error = new_error("This is a test");
        source.set_error(error.clone());

        assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)), "an error was published");
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert!(!results.has_completed());

        // Ensure that the operator does not attept to continue removing items.
        scheduler.advance_to(ticks_at(10));
    }

    #[test]
    fn source_errors_immediately_error_is_propagated() {
        let source = create_test_source();

        let scheduler = create_test_scheduler();

        let item1 = TestItem::new(1, Some(10));
        source.add_or_update(item1.clone());
        scheduler.advance_by(1);

        let error = new_error("This is a test");
        source.set_error(error.clone());

        let (_subscription, results) = source.expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler()).validate_synchronization().record_values_on(scheduler.scheduler());

        assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)), "an error was published");
        assert!(results.recorded_values().is_empty(), "no expirations should have occurred");
        assert!(!results.has_completed());
        assert_equivalent_because(&source.items(), &[item1.clone()], "no changes should have occurred");

        // Ensure that the operator does not attept to continue removing items.
        scheduler.advance_to(ticks_at(10));
    }

    // Not ported: SourceIsNull_ThrowsException - argument validation

    // Not ported: ThreadPoolSchedulerIsUsedWithoutPolling_ExpirationIsThreadSafe - real threads

    // Not ported: ThreadPoolSchedulerIsUsedWithPolling_ExpirationIsThreadSafe - real threads

    // Not ported: TimeSelectorIsNull_ThrowsException - argument validation

    // Not ported: TimeSelectorThrows_ErrorIsPropagated - the time selector cannot fail: expire_after has no try_ form (a selector returning Result) whose error ends the sequence
}

/// `ExpireAfterFixture.ForStream`.
mod for_stream {
    use super::{create_test_scheduler, create_time_selector, ticks_at, TestItem, MAX_TICKS};
    use crate::utilities::{assert_equivalent_because, CacheObservableExtensions, FakeScheduler, ObservableExtensions};
    use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, ObservableCacheEx};
    use ferroui_reactive::disposables::Disposable;
    use ferroui_reactive::subjects::Subject;
    use ferroui_reactive::Observable;
    use ferroui_reactive_testing::new_error;
    use ferroui_system::{IObservable, IObserver};
    use std::rc::Rc;
    use std::time::Duration;

    /// `new(reason: ChangeReason.Add, key: item.Id, current: item)`.
    fn add(item: &TestItem) -> Change<TestItem, i32> {
        Change::new(ChangeReason::Add, item.id(), item.clone())
    }

    /// `new(reason: ChangeReason.Add, key: item.Id, current: item, index: index)`.
    fn add_at(item: &TestItem, index: usize) -> Change<TestItem, i32> {
        Change::with_index(ChangeReason::Add, item.id(), item.clone(), Some(index))
    }

    /// `new(reason: ChangeReason.Update, key: current.Id, current: current, previous: previous, currentIndex: index, previousIndex: index)`.
    fn update_at(current: &TestItem, previous: &TestItem, index: usize) -> Change<TestItem, i32> {
        Change::with_previous(ChangeReason::Update, current.id(), current.clone(), Some(previous.clone()), Some(index), Some(index))
    }

    #[test]
    fn expired_item_is_removed_removal_is_skipped() {
        let source = Subject::<ChangeSet<TestItem, i32>>::new();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .as_observable()
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        let item2 = TestItem::new(2, Some(20));
        let item3 = TestItem::new(3, Some(30));
        source.on_next(ChangeSet::from(vec![add(&item1), add(&item2), add(&item3)]));
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
        assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone()], "3 items were added");

        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item2.clone(), item3.clone()], "item #1 should have been removed");

        // Send a notification to remove an item that's already been removed
        source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Remove, item1.id(), item1.clone())]));
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "no changes should have occurred");

        assert!(!results.has_completed());
    }

    #[test]
    fn item_is_removed_before_expiration_expiration_is_cancelled() {
        let source = Subject::<ChangeSet<TestItem, i32>>::new();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .as_observable()
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        let item3 = TestItem::new(3, Some(10));
        let item2 = TestItem::new(2, Some(10));
        source.on_next(ChangeSet::from(vec![add(&item1), add(&item2), add(&item3)]));
        scheduler.advance_by(1);

        let item4 = TestItem::new(4, None);
        source.on_next(ChangeSet::from(vec![add(&item4)]));
        scheduler.advance_by(1);

        source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Remove, item2.id(), item2.clone())]));
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 3, "3 source operations were performed");
        assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item3.clone(), item4.clone()], "3 items were added, and one was removed");

        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item4.clone()], "items #1 and #3 should have been removed");

        assert!(!results.has_completed());
    }

    #[test]
    fn next_item_to_expire_is_replaced_expiration_is_rescheduled_if_needed() {
        let source = Subject::<ChangeSet<TestItem, i32>>::new();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .as_observable()
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        source.on_next(ChangeSet::from(vec![add(&item1)]));
        scheduler.advance_by(1);

        // Extend the expiration to a later time
        let item2 = TestItem::new(1, Some(20));
        source.on_next(ChangeSet::from(vec![Change::update(item2.id(), item2.clone(), item1.clone())]));
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 2, "2 source operations were performed");
        assert_equivalent_because(&results.recorded_items(), &[item2.clone()], "item #1 was added, and then replaced");

        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "no expirations should have occurred");

        // Shorten the expiration to an earlier time
        let item3 = TestItem::new(1, Some(15));
        source.on_next(ChangeSet::from(vec![Change::update(item3.id(), item3.clone(), item2.clone())]));
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source operation was performed");
        assert_equivalent_because(&results.recorded_items(), &[item3.clone()], "item #1 was replaced");

        // One more update with no changes to the expiration
        let item4 = TestItem::new(1, Some(15));
        source.on_next(ChangeSet::from(vec![Change::update(item4.id(), item4.clone(), item3.clone())]));
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(3).count(), 1, "1 source operation was performed");
        assert_equivalent_because(&results.recorded_items(), &[item4.clone()], "item #1 was replaced");

        scheduler.advance_to(ticks_at(15));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(4).count(), 1, "1 expiration should have occurred");
        assert!(results.recorded_items().is_empty(), "item #1 should have expired");

        scheduler.advance_to(MAX_TICKS);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(5).count(), 0, "no expirations should have occurred");

        assert!(!results.has_completed());
    }

    #[test]
    fn polling_interval_is_given_removals_are_scheduled_at_interval() {
        let source = Subject::<ChangeSet<TestItem, i32>>::new();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .as_observable()
            .expire_after_with_polling_on(create_time_selector(scheduler.scheduler()), Some(Duration::from_millis(20)), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        let item2 = TestItem::new(2, Some(20));
        let item3 = TestItem::new(3, Some(30));
        let item4 = TestItem::new(4, Some(40));
        let item5 = TestItem::new(5, Some(100));
        source.on_next(ChangeSet::from(vec![add_at(&item1, 0), add_at(&item2, 1), add_at(&item3, 2), add_at(&item4, 3), add_at(&item5, 4)]));
        scheduler.advance_by(1);

        // Additional expirations at 20ms.
        let item6 = TestItem::new(6, Some(20));
        let item7 = TestItem::new(7, Some(20));
        source.on_next(ChangeSet::from(vec![add_at(&item6, 5), add_at(&item7, 6)]));
        scheduler.advance_by(1);

        // Out-of-order expiration
        let item8 = TestItem::new(8, Some(15));
        source.on_next(ChangeSet::from(vec![add_at(&item8, 7)]));
        scheduler.advance_by(1);

        // Non-expiring item
        let item9 = TestItem::new(9, None);
        source.on_next(ChangeSet::from(vec![add_at(&item9, 8)]));
        scheduler.advance_by(1);

        // Replacement changing lifetime.
        let item10 = TestItem::new(4, Some(45));
        source.on_next(ChangeSet::from(vec![update_at(&item10, &item4, 3)]));
        scheduler.advance_by(1);

        // Replacement not-affecting lifetime.
        let item11 = TestItem::new(5, Some(100));
        source.on_next(ChangeSet::from(vec![update_at(&item11, &item5, 4)]));
        scheduler.advance_by(1);

        // Refresh should not affect scheduled expiration.
        item3.set_expiration(Some(55));
        source.on_next(ChangeSet::from(vec![Change::with_index(ChangeReason::Refresh, item3.id(), item3.clone(), Some(2))]));
        scheduler.advance_by(1);

        // Move changes should be ignored completely.
        source.on_next(ChangeSet::from(vec![Change::moved(item3.id(), item3.clone(), 3, 2), Change::moved(item1.id(), item1.clone(), 1, 0), Change::moved(item1.id(), item1.clone(), 4, 1)]));

        // Verify initial state, after all emissions
        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 7, "8 source operations were performed, and 1 should have been ignored");
        assert_equivalent_because(
            &results.recorded_items(),
            &[item1.clone(), item2.clone(), item3.clone(), item6.clone(), item7.clone(), item8.clone(), item9.clone(), item10.clone(), item11.clone()],
            "9 items were added, 2 were replaced, and 1 was refreshed",
        );
        assert!(results.recorded_items_sorted().is_empty());

        // Item scheduled to expire at 10ms, but won't be picked up yet
        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(7).count(), 0, "no changes should have occurred");

        // Item scheduled to expire at 15ms, but won't be picked up yet
        scheduler.advance_to(ticks_at(15));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(7).count(), 0, "no changes should have occurred");

        // Expired items should be polled
        scheduler.advance_to(ticks_at(20));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(7).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item3.clone(), item9.clone(), item10.clone(), item11.clone()], "items #1, #2, #6, #7, and #8 should have been removed");
        assert!(results.recorded_items_sorted().is_empty());

        // Item scheduled to expire at 30ms, but won't be picked up yet
        scheduler.advance_to(ticks_at(30));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(8).count(), 0, "no changes should have occurred");

        // Expired items should be polled, but should exclude the one that was changed from 40ms to 45ms.
        scheduler.advance_to(ticks_at(40));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(8).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item9.clone(), item10.clone(), item11.clone()], "item #3 should have been removed");
        assert!(results.recorded_items_sorted().is_empty());

        // Item scheduled to expire at 45ms, but won't be picked up yet
        scheduler.advance_to(ticks_at(45));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(9).count(), 0, "no changes should have occurred");

        // Expired items should be polled
        scheduler.advance_to(ticks_at(60));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(9).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item9.clone(), item11.clone()], "item #10 should have been removed");
        assert!(results.recorded_items_sorted().is_empty());

        // Expired items should be polled, but none should be found
        scheduler.advance_to(ticks_at(80));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(10).count(), 0, "no changes should have occurred");

        // Expired items should be polled
        scheduler.advance_to(ticks_at(100));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(10).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item9.clone()], "item #11 should have been removed");
        assert!(results.recorded_items_sorted().is_empty());

        // Next poll should not find anything to expire.
        scheduler.advance_to(ticks_at(120));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(11).count(), 0, "no changes should have occurred");

        assert!(!results.has_completed());
    }

    #[test]
    fn polling_interval_is_not_given_removals_are_scheduled_immediately() {
        let source = Subject::<ChangeSet<TestItem, i32>>::new();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .as_observable()
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        let item2 = TestItem::new(2, Some(20));
        let item3 = TestItem::new(3, Some(30));
        let item4 = TestItem::new(4, Some(40));
        let item5 = TestItem::new(5, Some(50));
        source.on_next(ChangeSet::from(vec![add_at(&item1, 0), add_at(&item2, 1), add_at(&item3, 2), add_at(&item4, 3), add_at(&item5, 4)]));
        scheduler.advance_by(1);

        // Additional expirations at 20ms.
        let item6 = TestItem::new(6, Some(20));
        let item7 = TestItem::new(7, Some(20));
        source.on_next(ChangeSet::from(vec![add_at(&item6, 5), add_at(&item7, 6)]));
        scheduler.advance_by(1);

        // Out-of-order expiration
        let item8 = TestItem::new(8, Some(15));
        source.on_next(ChangeSet::from(vec![add_at(&item8, 7)]));
        scheduler.advance_by(1);

        // Non-expiring item
        let item9 = TestItem::new(9, None);
        source.on_next(ChangeSet::from(vec![add_at(&item9, 8)]));
        scheduler.advance_by(1);

        // Replacement changing lifetime.
        let item10 = TestItem::new(4, Some(45));
        source.on_next(ChangeSet::from(vec![update_at(&item10, &item4, 3)]));
        scheduler.advance_by(1);

        // Replacement not-affecting lifetime.
        let item11 = TestItem::new(5, Some(50));
        source.on_next(ChangeSet::from(vec![update_at(&item11, &item5, 4)]));
        scheduler.advance_by(1);

        // Refresh should not affect scheduled expiration.
        item3.set_expiration(Some(55));
        source.on_next(ChangeSet::from(vec![Change::with_index(ChangeReason::Refresh, item3.id(), item3.clone(), Some(2))]));
        scheduler.advance_by(1);

        // Move changes should be ignored completely.
        source.on_next(ChangeSet::from(vec![Change::moved(item3.id(), item3.clone(), 3, 2), Change::moved(item1.id(), item1.clone(), 1, 0), Change::moved(item1.id(), item1.clone(), 4, 1)]));

        // Verify initial state, after all emissions
        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 7, "8 source operations were performed, and 1 should have been ignored");
        assert_equivalent_because(
            &results.recorded_items(),
            &[item1.clone(), item2.clone(), item3.clone(), item6.clone(), item7.clone(), item8.clone(), item9.clone(), item10.clone(), item11.clone()],
            "11 items were added, 2 were replaced, and 1 was refreshed",
        );
        assert!(results.recorded_items_sorted().is_empty());

        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(7).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(
            &results.recorded_items(),
            &[item2.clone(), item3.clone(), item6.clone(), item7.clone(), item8.clone(), item9.clone(), item10.clone(), item11.clone()],
            "item #1 should have been removed",
        );
        assert!(results.recorded_items_sorted().is_empty());

        scheduler.advance_to(ticks_at(15));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(8).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item2.clone(), item3.clone(), item6.clone(), item7.clone(), item9.clone(), item10.clone(), item11.clone()], "item #8 should have expired");
        assert!(results.recorded_items_sorted().is_empty());

        scheduler.advance_to(ticks_at(20));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(9).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item3.clone(), item9.clone(), item10.clone(), item11.clone()], "items #2, #6, and #7 should have been removed");
        assert!(results.recorded_items_sorted().is_empty());

        scheduler.advance_to(ticks_at(30));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(10).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item9.clone(), item10.clone(), item11.clone()], "item #3 should have been removed");
        assert!(results.recorded_items_sorted().is_empty());

        scheduler.advance_to(ticks_at(40));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(11).count(), 0, "no changes should have occurred");

        scheduler.advance_to(ticks_at(45));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(11).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item9.clone(), item11.clone()], "item #10 should have expired");
        assert!(results.recorded_items_sorted().is_empty());

        scheduler.advance_to(ticks_at(50));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(12).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item9.clone()], "item #11 should have expired");
        assert!(results.recorded_items_sorted().is_empty());

        // Remaining item should never expire
        scheduler.advance_to(MAX_TICKS);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(13).count(), 0, "no changes should have occurred");

        assert!(!results.has_completed());
    }

    #[test]
    fn removals_are_pending_completion_waits_for_removals() {
        let source = Subject::<ChangeSet<TestItem, i32>>::new();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .as_observable()
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        let item2 = TestItem::new(2, None);
        let item3 = TestItem::new(3, Some(20));
        source.on_next(ChangeSet::from(vec![add(&item1), add(&item2), add(&item3)]));
        scheduler.advance_by(1);

        // Verify initial state, after all emissions
        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
        assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone()], "3 items were added");

        scheduler.advance_to(ticks_at(10));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item2.clone(), item3.clone()], "item #1 should have been removed");

        source.on_completed();

        assert!(results.error().is_none());
        assert!(!results.has_completed(), "removals are pending");
        assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "no changes should have occurred");

        scheduler.advance_to(ticks_at(20));

        assert!(results.error().is_none());
        assert!(results.has_completed());
        assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 expiration should have occurred");
        assert_equivalent_because(&results.recorded_items(), &[item2.clone()], "item #3 should have expired");
    }

    // Covers https://github.com/reactivemarbles/DynamicData/issues/716
    #[test]
    fn scheduler_is_inaccurate_removals_are_not_skipped() {
        let source = Subject::<ChangeSet<TestItem, i32>>::new();

        let scheduler = FakeScheduler::new();
        scheduler.set_now(super::ms(0));

        let (_subscription, results) = source
            .as_observable()
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        source.on_next(ChangeSet::from(vec![add(&item1)]));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
        assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "1 item was added");

        // TimeSpan.FromMilliseconds(-1), in ticks.
        scheduler.simulate_until_idle_with(-ferroui_reactive_testing::duration_to_ticks(Duration::from_millis(1)));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 expiration should have occurred");
        assert!(results.recorded_items().is_empty(), "item #1 should have been removed");

        assert!(!results.has_completed());
    }

    #[test]
    fn source_completes_completion_is_propagated() {
        let source = Subject::<ChangeSet<TestItem, i32>>::new();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .as_observable()
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        let item1 = TestItem::new(1, None);
        let item2 = TestItem::new(2, None);
        let item3 = TestItem::new(3, None);
        source.on_next(ChangeSet::from(vec![add(&item1), add(&item2), add(&item3)]));

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
        assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone()], "3 items were added");

        source.on_completed();

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no changes should have occurred");

        assert!(results.has_completed());
    }

    #[test]
    fn source_completes_immediately_completion_is_propagated() {
        let item1 = TestItem::new(1, None);
        let item2 = TestItem::new(2, None);
        let item3 = TestItem::new(3, None);

        let source: Rc<dyn IObservable<ChangeSet<TestItem, i32>>> = {
            let (item1, item2, item3) = (item1.clone(), item2.clone(), item3.clone());
            Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<TestItem, i32>>>| {
                observer.on_next(ChangeSet::from(vec![add(&item1), add(&item2), add(&item3)]));

                observer.on_completed();

                Disposable::empty()
            })
        };

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
        assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone()], "3 items were added");

        assert!(results.has_completed());
    }

    #[test]
    fn source_errors_error_is_propagated() {
        let source = Subject::<ChangeSet<TestItem, i32>>::new();

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .as_observable()
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        let item1 = TestItem::new(1, Some(10));
        source.on_next(ChangeSet::from(vec![add(&item1)]));
        scheduler.advance_by(1);

        assert!(results.error().is_none());
        assert_eq!(results.recorded_change_sets().len(), 1, "1 source operations was performed");
        assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "1 item was added");

        let error = new_error("This is a test");
        source.on_error(error.clone());

        assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
        assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no changes should have occurred");
        assert!(!results.has_completed());

        scheduler.advance_to(ticks_at(10));

        assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "notifications should not get published after an error");
    }

    #[test]
    fn source_errors_immediately_error_is_propagated() {
        let item1 = TestItem::new(1, Some(10));

        let error = new_error("This is a test");

        let source: Rc<dyn IObservable<ChangeSet<TestItem, i32>>> = {
            let (item1, error) = (item1.clone(), error.clone());
            Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<TestItem, i32>>>| {
                observer.on_next(ChangeSet::from(vec![add(&item1)]));

                observer.on_error(error.clone());

                Disposable::empty()
            })
        };

        let scheduler = create_test_scheduler();

        let (_subscription, results) = source
            .expire_after_on(create_time_selector(scheduler.scheduler()), scheduler.scheduler())
            .validate_synchronization()
            .validate_cache_change_sets(|item: &TestItem| item.id())
            .record_cache_items_on(scheduler.scheduler());

        assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
        assert_eq!(results.recorded_change_sets().len(), 1, "1 source operations was performed");
        assert_equivalent_because(&results.recorded_items(), &[item1.clone()], "1 item was added");
        assert!(!results.has_completed());

        scheduler.advance_to(ticks_at(10));

        assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "notifications should not get published after an error");
    }

    // Not ported: SourceIsNull_ThrowsException - argument validation

    // Not ported: ThreadPoolSchedulerIsUsedWithoutPolling_ExpirationIsThreadSafe - real threads

    // Not ported: ThreadPoolSchedulerIsUsedWithPolling_ExpirationIsThreadSafe - real threads

    // Not ported: TimeSelectorIsNull_ThrowsException - argument validation

    // Not ported: TimeSelectorThrows_ErrorIsPropagated - the time selector cannot fail: expire_after has no try_ form (a selector returning Result) whose error ends the sequence
}
