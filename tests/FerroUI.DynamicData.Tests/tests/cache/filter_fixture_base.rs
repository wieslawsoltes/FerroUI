//! `FilterFixture.Base` (`DynamicData.Tests/Cache/FilterFixture.Base.cs`): the tests that every
//! filter operator of caches passes, run by the `Static`, `DynamicPredicate`,
//! `DynamicPredicateAndReFiltering` and `DynamicPredicateState` fixtures.
//!
//! The abstract class is the macro [`filter_fixture_base_tests`]: it takes the name of the
//! function of the invoking module that builds the unit under test (upstream's abstract
//! `BuildUut`), with the signature
//! `fn(Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>, ItemPredicate, bool) -> Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>`
//! (the source, the predicate, and whether empty change sets are suppressed), and makes a module
//! `base` with the tests.

/// The tests of `FilterFixture.Base` for the unit under test that `$build_uut` builds.
macro_rules! filter_fixture_base_tests {
    ($build_uut:ident) => {
        /// The tests of `FilterFixture.Base` (`DynamicData.Tests/Cache/FilterFixture.Base.cs`).
        mod base {
            use $crate::tests::cache::filter_fixture::{EmptyChangesetPolicy, Item, ItemPredicate};
            use $crate::utilities::{
                assert_cache_should_have_refreshed, assert_equivalent_because, assert_should_not_support_sorting, CacheObservableExtensions, ObservableExtensions,
                StreamCompletionStrategy, TestSourceCache,
            };
            use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, IObservableCache};
            use ferroui_reactive::subjects::Subject;
            use ferroui_reactive::ObservableExt;
            use ferroui_reactive_testing::new_error;
            use ferroui_system::{IDisposable, IObservable, IObserver};
            use std::rc::Rc;

            /// `BuildUut`.
            fn build_uut(
                source: Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>>,
                predicate: ItemPredicate,
                suppress_empty_change_sets: bool,
            ) -> Rc<dyn IObservable<ChangeSet<Rc<Item>, i32>>> {
                super::$build_uut(source, predicate, suppress_empty_change_sets)
            }

            fn included_items(items: Vec<Rc<Item>>) -> Vec<Rc<Item>> {
                items.into_iter().filter(Item::filter_by_is_included).collect()
            }

            fn excluded_items_are_removed_no_changes_are_made(empty_changeset_policy: EmptyChangesetPolicy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                source.edit(|updater| {
                    updater.add_or_update_items(
                        &mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter(),
                    )
                });

                // UUT Initialization
                let (subscription, results) = build_uut(
                    source.connect(),
                    Rc::new(Item::filter_by_is_included),
                    empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
                )
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset was published");
                assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all matching items should have been added");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                let excluded: Vec<Rc<Item>> = source.items().into_iter().filter(|item| !item.is_included()).collect();
                source.edit(|updater| updater.remove_items(&mut excluded.into_iter()));

                assert!(results.error().is_none());
                if empty_changeset_policy == EmptyChangesetPolicy::IncludeEmptyChangesets {
                    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
                    assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "only excluded items were manipulated");
                } else {
                    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "empty changesets should be suppressed");
                }
                assert!(!results.has_completed(), "the source has not completed");

                // Final verification
                assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

                subscription.dispose();
                source.dispose();
            }

            #[test]
            fn excluded_items_are_removed_no_changes_are_made_case1() {
                excluded_items_are_removed_no_changes_are_made(EmptyChangesetPolicy::IncludeEmptyChangesets);
            }

            #[test]
            fn excluded_items_are_removed_no_changes_are_made_case2() {
                excluded_items_are_removed_no_changes_are_made(EmptyChangesetPolicy::SuppressEmptyChangesets);
            }

            fn items_are_added_matching_items_propagate(empty_changeset_policy: EmptyChangesetPolicy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                // UUT Intialization
                let (subscription, results) = build_uut(
                    source.connect(),
                    Rc::new(Item::filter_by_is_included),
                    empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
                )
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

                assert!(results.error().is_none());
                assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                source.edit(|updater| {
                    updater.add_or_update_items(
                        &mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter(),
                    )
                });

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "1 source operation was performed");
                assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all matching items should have been added");
                assert!(!results.has_completed(), "the source has not completed");

                // Final verification
                assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

                subscription.dispose();
                source.dispose();
            }

            #[test]
            fn items_are_added_matching_items_propagate_case1() {
                items_are_added_matching_items_propagate(EmptyChangesetPolicy::IncludeEmptyChangesets);
            }

            #[test]
            fn items_are_added_matching_items_propagate_case2() {
                items_are_added_matching_items_propagate(EmptyChangesetPolicy::SuppressEmptyChangesets);
            }

            fn items_are_moved_movements_are_ignored(empty_changeset_policy: EmptyChangesetPolicy) {
                // Setup
                let source = Subject::<ChangeSet<Rc<Item>, i32>>::new();

                let items = [Item::new(1, true), Item::new(2, true), Item::new(3, true)];

                // UUT Initialization
                let initial: ChangeSet<Rc<Item>, i32> = items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| Change::with_index(ChangeReason::Add, item.id(), item.clone(), Some(index)))
                    .collect();
                let (subscription, results) = build_uut(
                    source.as_observable().prepend(initial),
                    Rc::new(Item::filter_by_is_included),
                    empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
                )
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset was published");
                assert_equivalent_because(&results.recorded_items(), &items, "all matching items should have been added");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                source.on_next(ChangeSet::from(vec![
                    Change::moved(items[2].id(), items[2].clone(), 0, 2),
                    Change::moved(items[1].id(), items[1].clone(), 1, 2),
                ]));

                assert!(results.error().is_none());
                if empty_changeset_policy == EmptyChangesetPolicy::IncludeEmptyChangesets {
                    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source opreation was performed");
                    assert_equivalent_because(&results.recorded_items(), &items, "no changes should have been made");
                } else {
                    assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "empty changesets should be suppressed");
                }
                assert!(!results.has_completed(), "the source has not completed");

                // Final verification
                assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

                subscription.dispose();
                source.dispose();
            }

            #[test]
            fn items_are_moved_movements_are_ignored_case1() {
                items_are_moved_movements_are_ignored(EmptyChangesetPolicy::IncludeEmptyChangesets);
            }

            #[test]
            fn items_are_moved_movements_are_ignored_case2() {
                items_are_moved_movements_are_ignored(EmptyChangesetPolicy::SuppressEmptyChangesets);
            }

            fn items_are_refreshed_items_are_re_filtered_or_refreshed(empty_changeset_policy: EmptyChangesetPolicy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                source.edit(|updater| {
                    updater.add_or_update_items(
                        &mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter(),
                    )
                });

                // UUT Initialization
                let (subscription, results) = build_uut(
                    source.connect(),
                    Rc::new(Item::filter_by_is_included),
                    empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
                )
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset was published");
                assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all matching items should have propagated");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (add items)
                for item in source.items() {
                    item.set_is_included(true);
                }

                source.edit(|updater| updater.refresh_all());

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
                let first_three: Vec<Rc<Item>> = source.items().into_iter().take(3).collect();
                assert_cache_should_have_refreshed(&results.recorded_change_sets()[1], &first_three, "all unchanged items should have been refreshed");
                assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all newly-matching items should have been added");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (remove items)
                for item in source.items().into_iter().take(3) {
                    item.set_is_included(false);
                }

                source.edit(|updater| updater.refresh_all());

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source operation was performed");
                let last_three: Vec<Rc<Item>> = source.items().into_iter().skip(3).collect();
                assert_cache_should_have_refreshed(&results.recorded_change_sets()[2], &last_three, "all unchanged items should have been refreshed");
                assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all newly-excluded items should have been removed");
                assert!(!results.has_completed(), "the source has not completed");

                // Final verification
                assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

                subscription.dispose();
                source.dispose();
            }

            #[test]
            fn items_are_refreshed_items_are_re_filtered_or_refreshed_case1() {
                items_are_refreshed_items_are_re_filtered_or_refreshed(EmptyChangesetPolicy::IncludeEmptyChangesets);
            }

            #[test]
            fn items_are_refreshed_items_are_re_filtered_or_refreshed_case2() {
                items_are_refreshed_items_are_re_filtered_or_refreshed(EmptyChangesetPolicy::SuppressEmptyChangesets);
            }

            fn items_are_updated_items_are_re_filtered(empty_changeset_policy: EmptyChangesetPolicy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                source.edit(|updater| {
                    updater.add_or_update_items(
                        &mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter(),
                    )
                });

                // UUT Intialization
                let (subscription, results) = build_uut(
                    source.connect(),
                    Rc::new(Item::filter_by_is_included),
                    empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
                )
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset was published");
                assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all matching items should have propagated");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (add and update items)
                source.edit(|updater| {
                    updater.add_or_update_items(
                        &mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, true), Item::new(5, true), Item::new(6, true)].into_iter(),
                    )
                });

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
                assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all newly-matching items should have been added");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action (remove and update items)
                source.edit(|updater| {
                    updater.add_or_update_items(
                        &mut [Item::new(1, false), Item::new(2, false), Item::new(3, false), Item::new(4, true), Item::new(5, true), Item::new(6, true)].into_iter(),
                    )
                });

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source operation was performed");
                assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all newly-excluded items should have been removed");
                assert!(!results.has_completed(), "the source has not completed");

                // Final verification
                assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

                subscription.dispose();
                source.dispose();
            }

            #[test]
            fn items_are_updated_items_are_re_filtered_case1() {
                items_are_updated_items_are_re_filtered(EmptyChangesetPolicy::SuppressEmptyChangesets);
            }

            #[test]
            fn items_are_updated_items_are_re_filtered_case2() {
                items_are_updated_items_are_re_filtered(EmptyChangesetPolicy::IncludeEmptyChangesets);
            }

            fn matching_items_are_removed_removals_propagate(empty_changeset_policy: EmptyChangesetPolicy) {
                // Setup
                let source = TestSourceCache::new(Item::select_id);

                source.edit(|updater| {
                    updater.add_or_update_items(
                        &mut [Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)].into_iter(),
                    )
                });

                // UUT Initialization
                let (subscription, results) = build_uut(
                    source.connect(),
                    Rc::new(Item::filter_by_is_included),
                    empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
                )
                .validate_synchronization()
                .validate_cache_change_sets(Item::select_id)
                .record_cache_items();

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset was published");
                assert_equivalent_because(&results.recorded_items(), &included_items(source.items()), "all matching items should have propagated");
                assert!(!results.has_completed(), "the source has not completed");

                // UUT Action
                let matching = included_items(source.items());
                source.edit(|updater| updater.remove_items(&mut matching.into_iter()));

                assert!(results.error().is_none());
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
                assert!(results.recorded_items().is_empty(), "all matching items were removed");
                assert!(!results.has_completed(), "the source has not completed");

                // Final verification
                assert_should_not_support_sorting(&results, "sorting is not supported by filter operators");

                subscription.dispose();
                source.dispose();
            }

            #[test]
            fn matching_items_are_removed_removals_propagate_case1() {
                matching_items_are_removed_removals_propagate(EmptyChangesetPolicy::IncludeEmptyChangesets);
            }

            #[test]
            fn matching_items_are_removed_removals_propagate_case2() {
                matching_items_are_removed_removals_propagate(EmptyChangesetPolicy::SuppressEmptyChangesets);
            }

            fn source_fails_error_propagates(completion_strategy: StreamCompletionStrategy) {
                let error = new_error("Test");

                let source = TestSourceCache::new(Item::select_id);

                if completion_strategy == StreamCompletionStrategy::Immediate {
                    source.set_error(error.clone());
                }

                let (subscription, results) = build_uut(source.connect(), Rc::new(Item::filter_by_is_included), true)
                    .validate_synchronization()
                    .validate_cache_change_sets(Item::select_id)
                    .record_cache_items();

                if completion_strategy == StreamCompletionStrategy::Asynchronous {
                    source.set_error(error.clone());
                }

                assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
                assert!(results.recorded_change_sets().is_empty(), "no source operations were performed");

                subscription.dispose();
                source.dispose();
            }

            #[test]
            fn source_fails_error_propagates_case1() {
                source_fails_error_propagates(StreamCompletionStrategy::Asynchronous);
            }

            #[test]
            fn source_fails_error_propagates_case2() {
                source_fails_error_propagates(StreamCompletionStrategy::Immediate);
            }

            // Not ported: SourceIsNull_ThrowsException - argument validation
        }
    };
}
pub(crate) use filter_fixture_base_tests;
