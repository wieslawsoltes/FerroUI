//! `AutoRefreshOnObservableFixture.Base`
//! (`DynamicData.Tests/Cache/AutoRefreshOnObservableFixture.Base.cs`): the tests of
//! `AutoRefreshOnObservable` that run for each form of the operator.
//!
//! Upstream's abstract class is the macro [`auto_refresh_on_observable_fixture_base_tests`], which
//! each derived file invokes with its `build_uut`, a function of the form
//! `fn<A: 'static>(source, reevaluator: impl Fn(&Rc<Item>) -> Rc<dyn IObservable<A>> + 'static, change_set_buffer: Option<Duration>, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>`.
//! The tests go in a module `base` of the invoking file. Upstream's tests skipped as existing
//! defects are `#[ignore]`d.

/// The tests of `AutoRefreshOnObservableFixture.Base`, for the unit under test that `$build_uut`
/// (a function of the invoking module) builds.
macro_rules! auto_refresh_on_observable_fixture_base_tests {
    ($build_uut:ident) => {
        mod base {
            use crate::tests::cache::auto_refresh_on_observable_fixture::{Item, NotificationStrategy};
            use crate::utilities::{assert_equivalent_because, CacheObservableExtensions, ObservableExtensions, TestSourceCache};
            use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, IObservableCache};
            use ferroui_reactive::subjects::{Subject, SubjectBase};
            use ferroui_reactive::ObservableExt;
            use ferroui_reactive_testing::{duration_to_ticks, new_error, TestScheduler};
            use ferroui_system::IObserver;
            use std::cell::Cell;
            use std::rc::Rc;
            use std::time::Duration;

            fn add_or_update(source: &TestSourceCache<Rc<Item>, i32>, items: &[Rc<Item>]) {
                source.edit(|updater| updater.add_or_update_items(&mut items.iter().cloned()));
            }

            /// `++item.Value`.
            fn increment_value(item: &Item) {
                item.set_value(item.value() + 1);
            }

            fn ticks(seconds: u64) -> i64 {
                duration_to_ticks(Duration::from_secs(seconds))
            }

            #[test]
            fn change_set_buffer_is_given_reevaluator_notifications_are_buffered_on_scheduler() {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1.clone(), item2.clone(), item3.clone()]);

                let scheduler = TestScheduler::new();

                // UUT Initialization
                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, Some(Duration::from_secs(10)), Some(scheduler.scheduler()))
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (publish reevaluator notification)
                increment_value(&item2);

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "the reevaluator notification should have been buffered");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (advance time, within buffer window)
                scheduler.advance_to(ticks(5));

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "the buffer window has not yet ended");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (advance time, to buffer window)
                scheduler.advance_to(ticks(10));

                assert!(results.error().is_none());
                let change_sets = results.recorded_change_sets();
                assert_eq!(change_sets.iter().skip(1).count(), 1, "a buffer window expired");
                assert_eq!(change_sets[1].count(), 1, "1 item published a reevaluator notification");
                assert_eq!(change_sets[1].refreshes(), 1, "1 item published a reevaluator notification");
                assert_eq!(change_sets[1][0].current(), &item2, "item #2 published a reevaluator notification");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "no items should have changed, within the source");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (publish reevaluator notification)
                increment_value(&item1);

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "the reevaluator notification should have been buffered");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (advance time, within buffer window)
                scheduler.advance_to(ticks(15));

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "the buffer window has not yet ended");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (publish additional reevaluator notification)
                increment_value(&item3);

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "the reevaluator notification should have been buffered");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (advance time, to buffer window)
                scheduler.advance_to(ticks(20));

                assert!(results.error().is_none());
                let change_sets = results.recorded_change_sets();
                assert_eq!(change_sets.iter().skip(2).count(), 1, "a buffer window expired");
                assert_eq!(change_sets[2].count(), 2, "2 items published a reevaluator notification");
                assert_eq!(change_sets[2].refreshes(), 2, "2 items published a reevaluator notification");
                assert_equivalent_because(
                    &change_sets[2].iter().map(|change| change.current().clone()).collect::<Vec<_>>(),
                    &[item1.clone(), item3.clone()],
                    "items #2 and #3 published reevaluator notification",
                );
                assert_equivalent_because(&results.recorded_items(), &source.items(), "no items should have changed, within the source");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (normal refresh)
                source.edit(|updater| updater.refresh_item(&item2));

                // Normal refreshes should not be buffered
                assert!(results.error().is_none());
                let change_sets = results.recorded_change_sets();
                assert_eq!(change_sets.iter().skip(3).count(), 1, "one source operation was performed");
                assert_eq!(change_sets[3].count(), 1, "1 item was refreshed, within the source");
                assert_eq!(change_sets[3].refreshes(), 1, "1 item was refreshed, within the source");
                assert_eq!(change_sets[3][0].current(), &item2, "item #2 was refreshed, within the source");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "no items should have changed, within the source");
                assert!(!results.has_completed(), "the source has not completed");
            }

            #[test]
            fn item_is_added_subscribes_to_reevaluator() {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                // UUT Initialization
                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                assert!(results.error().is_none());
                assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                add_or_update(&source, &[item1.clone(), item2.clone(), item3.clone()]);

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "one source operation was performed");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                assert!(!results.has_completed(), "the source has not completed");

                assert!(item1.has_observers(), "the reevaluator should be invoked and subscribed to, for each added item");
                assert!(item2.has_observers(), "the reevaluator should be invoked and subscribed to, for each added item");
                assert!(item3.has_observers(), "the reevaluator should be invoked and subscribed to, for each added item");
            }

            #[test]
            fn item_is_moved_notification_propagates() {
                // Setup
                let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                let items = vec![item1.clone(), item2.clone(), item3.clone()];

                let initial_changeset = ChangeSet::from(vec![
                    Change::with_index(ChangeReason::Add, item1.id(), item1.clone(), Some(0)),
                    Change::with_index(ChangeReason::Add, item2.id(), item2.clone(), Some(1)),
                    Change::with_index(ChangeReason::Add, item3.id(), item3.clone(), Some(2)),
                ]);

                // UUT Initialization
                let (_subscription, results) = super::$build_uut(source.as_observable().prepend(initial_changeset), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &items, "3 items were added to the source");
                assert_eq!(results.recorded_items_sorted(), items, "item indexes should propagate");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                source.on_next(ChangeSet::from(vec![Change::moved(item3.id(), item3.clone(), 0, 2)]));

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "one source operation was performed");
                assert_equivalent_because(&results.recorded_items(), &items, "an item was moved within the source");
                assert!(!results.has_completed(), "the source has not completed");
                assert_eq!(results.recorded_items_sorted(), vec![item3.clone(), item1.clone(), item2.clone()], "an item was moved within the source");
                assert!(!results.has_completed(), "the source has not completed");
            }

            #[test]
            fn item_is_refreshed_notification_propagates() {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1.clone(), item2.clone(), item3.clone()]);

                let reevaluator_invocation_count = Rc::new(Cell::new(0));

                // UUT Initialization
                let invocation_count = reevaluator_invocation_count.clone();
                let (_subscription, results) = super::$build_uut(
                    source.connect(),
                    move |item: &Rc<Item>| {
                        invocation_count.set(invocation_count.get() + 1);
                        Item::observe_value_changed(item)
                    },
                    None,
                    None,
                )
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                assert!(!results.has_completed(), "the source has not completed");

                assert_eq!(reevaluator_invocation_count.get(), 3, "the reevaluator should be invoked and subscribed to, for each added item");

                // UUT Action
                source.edit(|updater| updater.refresh_item(&item2));

                assert!(results.error().is_none());
                let change_sets = results.recorded_change_sets();
                assert_eq!(change_sets.iter().skip(1).count(), 1, "one source operation was performed");
                assert_eq!(change_sets[1].count(), 1, "1 item was refreshed within the source");
                assert_eq!(change_sets[1].refreshes(), 1, "1 item was refreshed within the source");
                assert_eq!(change_sets[1][0].current(), &item2, "item #2 was refreshed within the source");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "no items were changed, within the source");
                assert!(!results.has_completed(), "the source has not completed");

                assert_eq!(reevaluator_invocation_count.get(), 3, "the reevaluator should only be invoked for items being added to the collection.");
            }

            #[test]
            fn item_is_removed_unsubscribes_from_reevaluator() {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1.clone(), item2.clone(), item3.clone()]);

                // UUT Initialization
                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                source.edit(|updater| updater.remove_item(&item2));

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "one source operation was performed");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "1 item was removed from the source");
                assert!(!results.has_completed(), "the source has not completed");

                assert!(!item2.has_observers(), "removing an item should trigger unsubscription from its reevaluator");
                assert!(item1.has_observers(), "the item was not removed from the source");
                assert!(item3.has_observers(), "the item was not removed from the source");
            }

            #[test]
            fn item_is_updated_re_invokes_reevaluator() {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1.clone(), item2.clone(), item3.clone()]);

                // UUT Initialization
                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                let item4 = Item::new(2);
                source.edit(|updater| updater.add_or_update(item4.clone()));

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "one source operation was performed");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "1 item was replaced within the source");
                assert!(!results.has_completed(), "the source has not completed");

                assert!(!item2.has_observers(), "replacing an item should trigger unsubscription from its reevaluator");
                assert!(item4.has_observers(), "adding an item should invoke its reevaluator and subscribe to it");
                assert!(item1.has_observers(), "the item was not removed from the source");
                assert!(item3.has_observers(), "the item was not removed from the source");

                // UUT Action (updated item publishes reevaluator notification)
                increment_value(&item4);

                assert!(results.error().is_none());
                let change_sets = results.recorded_change_sets();
                assert_eq!(change_sets.iter().skip(2).count(), 1, "1 item published a reevaluation notification");
                assert_eq!(change_sets[2].count(), 1, "1 item published a reevaluation notification");
                assert_eq!(change_sets[2].refreshes(), 1, "1 item published a reevaluation notification");
                assert_eq!(change_sets[2][0].current(), &item4, "item #4 published a reevaluation notification");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "no source operations were performed");
                assert!(!results.has_completed(), "the source has not completed");
            }

            fn reevaluator_completes_when_not_only_item_in_source_completion_waits_for_source_completion_and_other_reevaluator_completions(
                notification_strategy: NotificationStrategy,
            ) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1.clone(), item2.clone(), item3.clone()]);

                // UUT Initialization & Action (initial completion)
                if notification_strategy == NotificationStrategy::Immediate {
                    item2.complete();
                }

                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                if notification_strategy == NotificationStrategy::Asynchronous {
                    item2.complete();
                }

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                assert!(!results.has_completed(), "not all notification sources have completed");

                // UUT Action (remaining reevaluator completions)
                item1.complete();
                item3.complete();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no source operations were performed");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (source completion)
                source.complete();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no source operations were performed");
                assert!(results.has_completed(), "all notification sources have completed");
            }

            /// `[InlineData(NotificationStrategy.Immediate)]`.
            #[test]
            fn reevaluator_completes_when_not_only_item_in_source_completion_waits_for_source_completion_and_other_reevaluator_completions_case1() {
                reevaluator_completes_when_not_only_item_in_source_completion_waits_for_source_completion_and_other_reevaluator_completions(NotificationStrategy::Immediate);
            }

            /// `[InlineData(NotificationStrategy.Asynchronous)]`.
            #[test]
            fn reevaluator_completes_when_not_only_item_in_source_completion_waits_for_source_completion_and_other_reevaluator_completions_case2() {
                reevaluator_completes_when_not_only_item_in_source_completion_waits_for_source_completion_and_other_reevaluator_completions(NotificationStrategy::Asynchronous);
            }

            fn reevaluator_completes_when_only_item_in_source_completion_waits_for_source_completion(notification_strategy: NotificationStrategy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item = Item::new(1);

                source.edit(|updater| updater.add_or_update(item.clone()));

                // UUT Initialization & Action (reevaluator completion)
                if notification_strategy == NotificationStrategy::Immediate {
                    item.complete();
                }

                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                if notification_strategy == NotificationStrategy::Asynchronous {
                    item.complete();
                }

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "1 item was added to the source");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (source completion)
                source.complete();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no source operations were performed");
                assert!(results.has_completed(), "all notification sources have completed");
            }

            /// `[InlineData(NotificationStrategy.Immediate)]`.
            #[test]
            fn reevaluator_completes_when_only_item_in_source_completion_waits_for_source_completion_case1() {
                reevaluator_completes_when_only_item_in_source_completion_waits_for_source_completion(NotificationStrategy::Immediate);
            }

            /// `[InlineData(NotificationStrategy.Asynchronous)]`.
            #[test]
            fn reevaluator_completes_when_only_item_in_source_completion_waits_for_source_completion_case2() {
                reevaluator_completes_when_only_item_in_source_completion_waits_for_source_completion(NotificationStrategy::Asynchronous);
            }

            #[test]
            fn reevaluator_emits_asynchronously_item_refreshes() {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1.clone(), item2.clone(), item3.clone()]);

                // UUT Initialization
                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                increment_value(&item2);

                assert!(results.error().is_none());
                let change_sets = results.recorded_change_sets();
                assert_eq!(change_sets.iter().skip(1).count(), 1, "1 item published a reevaluation notification");
                assert_eq!(change_sets[1].count(), 1, "1 item published a reevaluation notification");
                assert_eq!(change_sets[1].refreshes(), 1, "1 item published a reevaluation notification");
                assert_eq!(change_sets[1][0].current(), &item2, "item #2 published a reevaluation notification");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "no source operations were performed");
                assert!(!results.has_completed(), "the source has not completed");
            }

            #[test]
            #[ignore = "Existing defect, #1099 (skipped upstream)"]
            fn reevaluator_emits_immediately_item_does_not_refresh() {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1, item2, item3]);

                // UUT Initialization & Action
                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_eq!(results.recorded_change_sets()[0].refreshes(), 0, "re-evaluation notifications should be ignored within the initial subscription frame");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                assert!(!results.has_completed(), "the source has not completed");
            }

            fn reevaluator_fails_error_propagates(notification_strategy: NotificationStrategy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1.clone(), item2.clone(), item3.clone()]);

                let error = new_error("Test");

                // UUT Initialization & Action
                if notification_strategy == NotificationStrategy::Immediate {
                    item2.set_error(error.clone());
                }

                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                if notification_strategy == NotificationStrategy::Asynchronous {
                    item2.set_error(error.clone());
                }

                assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)), "upstream errors should propagate downstream");
                if notification_strategy == NotificationStrategy::Immediate {
                    assert!(results.recorded_change_sets().is_empty(), "an error occurred during processing of the initial changeset");
                } else {
                    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                    assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                }
            }

            /// `[InlineData(NotificationStrategy.Immediate)]`.
            #[test]
            #[ignore = "Existing defect (skipped upstream): reevaluator errors are ignored, by design of upstream's documentation"]
            fn reevaluator_fails_error_propagates_case1() {
                reevaluator_fails_error_propagates(NotificationStrategy::Immediate);
            }

            /// `[InlineData(NotificationStrategy.Asynchronous)]`.
            #[test]
            #[ignore = "Existing defect (skipped upstream): reevaluator errors are ignored, by design of upstream's documentation"]
            fn reevaluator_fails_error_propagates_case2() {
                reevaluator_fails_error_propagates(NotificationStrategy::Asynchronous);
            }

            // Not ported: ReevaluatorThrows_ExceptionPropagates - a getter cannot fail (the reevaluator is a closure that cannot throw; skipped upstream as an existing defect)

            fn source_completes_when_empty_completion_propagates(notification_strategy: NotificationStrategy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                // UUT Initialization & Action
                if notification_strategy == NotificationStrategy::Immediate {
                    source.complete();
                }

                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                if notification_strategy == NotificationStrategy::Asynchronous {
                    source.complete();
                }

                assert!(results.error().is_none());
                assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
                assert!(results.has_completed(), "all notification sources have completed");
            }

            /// `[InlineData(NotificationStrategy.Immediate)]`.
            #[test]
            fn source_completes_when_empty_completion_propagates_case1() {
                source_completes_when_empty_completion_propagates(NotificationStrategy::Immediate);
            }

            /// `[InlineData(NotificationStrategy.Asynchronous)]`.
            #[test]
            fn source_completes_when_empty_completion_propagates_case2() {
                source_completes_when_empty_completion_propagates(NotificationStrategy::Asynchronous);
            }

            fn source_completes_when_not_empty_completion_waits_for_reevaluator_completions(notification_strategy: NotificationStrategy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1.clone(), item2.clone(), item3.clone()]);

                // UUT Initialization & Action (source completion)
                if notification_strategy == NotificationStrategy::Immediate {
                    source.complete();
                }

                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                if notification_strategy == NotificationStrategy::Asynchronous {
                    source.complete();
                }

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                assert!(!results.has_completed(), "not all notification sources have completed");

                // UUT Action (initial reevaluator completion)
                item2.complete();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no source operations were performed");
                assert!(!results.has_completed(), "not all notification sources have completed");

                // UUT Action (remaining reevaluator completions)
                item1.complete();
                item3.complete();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no source operations were performed");
                assert!(results.has_completed(), "all notification sources have completed");
            }

            /// `[InlineData(NotificationStrategy.Immediate)]`.
            #[test]
            fn source_completes_when_not_empty_completion_waits_for_reevaluator_completions_case1() {
                source_completes_when_not_empty_completion_waits_for_reevaluator_completions(NotificationStrategy::Immediate);
            }

            /// `[InlineData(NotificationStrategy.Asynchronous)]`.
            #[test]
            fn source_completes_when_not_empty_completion_waits_for_reevaluator_completions_case2() {
                source_completes_when_not_empty_completion_waits_for_reevaluator_completions(NotificationStrategy::Asynchronous);
            }

            fn source_fails_error_propagates(notification_strategy: NotificationStrategy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                add_or_update(&source, &[item1, item2, item3]);

                let error = new_error("Test");

                // UUT Initialization & Action
                if notification_strategy == NotificationStrategy::Immediate {
                    source.set_error(error.clone());
                }

                let (_subscription, results) = super::$build_uut(source.connect(), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                if notification_strategy == NotificationStrategy::Asynchronous {
                    source.set_error(error.clone());
                }

                assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)), "upstream errors should propagate downstream");
                if notification_strategy == NotificationStrategy::Immediate {
                    assert!(results.recorded_change_sets().is_empty(), "an error occurred before the initial changeset");
                } else {
                    assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                    assert_equivalent_because(&results.recorded_items(), &source.items(), "3 items were added to the source");
                }
            }

            /// `[InlineData(NotificationStrategy.Immediate)]`.
            #[test]
            fn source_fails_error_propagates_case1() {
                source_fails_error_propagates(NotificationStrategy::Immediate);
            }

            /// `[InlineData(NotificationStrategy.Asynchronous)]`.
            #[test]
            fn source_fails_error_propagates_case2() {
                source_fails_error_propagates(NotificationStrategy::Asynchronous);
            }

            // Not ported: SourceIsNull_ThrowsException - argument validation

            #[test]
            fn subscription_is_disposed_subscription_disposal_propagates() {
                // Setup
                let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();

                let item1 = Item::new(1);
                let item2 = Item::new(2);
                let item3 = Item::new(3);

                let initial_changeset = ChangeSet::from(vec![
                    Change::new(ChangeReason::Add, item1.id(), item1.clone()),
                    Change::new(ChangeReason::Add, item2.id(), item2.clone()),
                    Change::new(ChangeReason::Add, item3.id(), item3.clone()),
                ]);

                // UUT Initialization
                let (subscription, results) = super::$build_uut(source.as_observable().prepend(initial_changeset), Item::observe_value_changed, None, None)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert_equivalent_because(&results.recorded_items(), &[item1.clone(), item2.clone(), item3.clone()], "3 items were added to the source");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                subscription.dispose();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "no source operations were performed");
                assert!(!results.has_completed(), "the source has not completed");

                assert!(!source.has_observers(), "subscription disposal should propagate");
                assert!(!item1.has_observers(), "subscription disposal should propagate");
                assert!(!item2.has_observers(), "subscription disposal should propagate");
                assert!(!item3.has_observers(), "subscription disposal should propagate");
            }
        }
    };
}
pub(crate) use auto_refresh_on_observable_fixture_base_tests;
