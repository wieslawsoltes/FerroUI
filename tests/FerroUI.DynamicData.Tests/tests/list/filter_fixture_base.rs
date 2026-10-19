//! `FilterFixture.Base` (`DynamicData.Tests/List/FilterFixture.Base.cs`): the tests of a filter
//! of lists that a derived fixture runs with the unit under test it builds (`BuildUut`).
//!
//! The abstract class becomes the macro [`filter_fixture_base_tests`], which takes the path of a
//! function `fn(source, predicate, suppress_empty_change_sets) -> change sets`:
//!
//! ```ignore
//! fn build_uut(
//!     source: Rc<dyn IObservable<ChangeSet<Rc<Item>>>>,
//!     predicate: fn(&Rc<Item>) -> bool,
//!     suppress_empty_change_sets: bool,
//! ) -> Rc<dyn IObservable<ChangeSet<Rc<Item>>>> { ... }
//!
//! mod base {
//!     super::super::filter_fixture_base::filter_fixture_base_tests!(super::build_uut);
//! }
//! ```
//!
//! The tests keep upstream's names, which the derived fixtures of the list share
//! (`ItemsAreAdded_MatchingItemsPropagate`, ...), so the macro is invoked in a module of its own.
//!
//! Upstream has no class derived from `Base` (neither `Static` nor `WithPredicateState` derives
//! from it), so xUnit never runs these tests; as there, nothing invokes the macro.

/// The tests of `FilterFixture.Base`, for the unit under test that `$build_uut` builds.
#[allow(unused_macros)]
macro_rules! filter_fixture_base_tests {
    ($build_uut:path) => {
        /// The names that the tests of `Base` use, in a module of their own so that they do not
        /// clash with those of the module that invokes the macro.
        #[allow(unused_imports)]
        mod filter_fixture_base_support {
            pub(super) use $crate::tests::list::filter_fixture::{is_same_error, EmptyChangesetPolicy, Item, SourceType};
            pub(super) use $crate::utilities::{assert_equivalent_because, assert_list_should_have_refreshed, ListObservableExtensions, ObservableExtensions, TestSourceList};
            pub(super) use ferroui_dynamic_data::list::{remove_many, IExtendedList, IList, IObservableList};
            pub(super) use ferroui_reactive_testing::new_error;
            pub(super) use ferroui_system::IDisposable;
            pub(super) use std::rc::Rc;
        }

        fn excluded_items_are_removed_no_changes_are_made(empty_changeset_policy: filter_fixture_base_support::EmptyChangesetPolicy) {
            #[allow(unused_imports)]
            use filter_fixture_base_support::*;

            // Setup
            let source = TestSourceList::<Rc<Item>>::new();

            source.edit(|list| {
                list.add_range(vec![Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)])
            });

            // UUT Initialization
            let (subscription, results) = $build_uut(
                source.connect(),
                Item::filter_by_is_included,
                empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
            )
            .validate_synchronization()
            .validate_list_change_sets()
            .record_list_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
            assert_eq!(
                results.recorded_items(),
                source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all matching items should have been added"
            );
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let excluded_items: Vec<Rc<Item>> = source.items().into_iter().filter(|item| !item.is_included()).collect();
            source.edit(|list| remove_many(list, &excluded_items));

            assert!(results.error().is_none());
            if empty_changeset_policy == EmptyChangesetPolicy::IncludeEmptyChangesets {
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
                assert_eq!(
                    results.recorded_items(),
                    source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                    "only excluded items were manipulated"
                );
            } else {
                assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 0, "empty changesets should be suppressed");
            }
            assert!(!results.has_completed(), "the source has not completed");

            subscription.dispose();
            source.dispose();
        }

        #[test]
        fn excluded_items_are_removed_no_changes_are_made_case1() {
            excluded_items_are_removed_no_changes_are_made(filter_fixture_base_support::EmptyChangesetPolicy::IncludeEmptyChangesets);
        }

        #[test]
        fn excluded_items_are_removed_no_changes_are_made_case2() {
            excluded_items_are_removed_no_changes_are_made(filter_fixture_base_support::EmptyChangesetPolicy::SuppressEmptyChangesets);
        }

        fn items_are_added_matching_items_propagate(empty_changeset_policy: filter_fixture_base_support::EmptyChangesetPolicy) {
            #[allow(unused_imports)]
            use filter_fixture_base_support::*;

            // Setup
            let source = TestSourceList::<Rc<Item>>::new();

            // UUT Intialization
            let (subscription, results) = $build_uut(
                source.connect(),
                Item::filter_by_is_included,
                empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
            )
            .validate_synchronization()
            .validate_list_change_sets()
            .record_list_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
            assert!(results.recorded_items().is_empty(), "no items have been added to the source");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            source.edit(|list| {
                list.add_range(vec![Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)])
            });

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
            assert_eq!(
                results.recorded_items(),
                source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all matching items should have been added"
            );
            assert!(!results.has_completed(), "the source has not completed");

            subscription.dispose();
            source.dispose();
        }

        #[test]
        fn items_are_added_matching_items_propagate_case1() {
            items_are_added_matching_items_propagate(filter_fixture_base_support::EmptyChangesetPolicy::IncludeEmptyChangesets);
        }

        #[test]
        fn items_are_added_matching_items_propagate_case2() {
            items_are_added_matching_items_propagate(filter_fixture_base_support::EmptyChangesetPolicy::SuppressEmptyChangesets);
        }

        fn items_are_moved_matching_movements_propagate(empty_changeset_policy: filter_fixture_base_support::EmptyChangesetPolicy) {
            #[allow(unused_imports)]
            use filter_fixture_base_support::*;

            // Setup
            let source = TestSourceList::<Rc<Item>>::new();

            source.edit(|list| {
                list.add_range(vec![Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)])
            });

            // UUT Initialization
            let (subscription, results) = $build_uut(
                source.connect(),
                Item::filter_by_is_included,
                empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
            )
            .validate_synchronization()
            .validate_list_change_sets()
            .record_list_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
            assert_eq!(results.recorded_items(), source.items(), "all matching items should have been added");
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: Moves for matching items,
            source.edit(|items| {
                items.move_index(2, 0);
                items.move_index(1, 5);
            });

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source opreation was performed");
            assert_eq!(
                results.recorded_items(),
                source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all matching items should have been moved, accordingly"
            );
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action: Moves for excluded items
            source.edit(|items| {
                items.move_index(4, 1);
                items.move_index(3, 5);
            });

            assert!(results.error().is_none());
            if empty_changeset_policy == EmptyChangesetPolicy::IncludeEmptyChangesets {
                assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source opreation was performed");
                assert_eq!(
                    results.recorded_items(),
                    source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                    "no matching items were moved"
                );
            } else {
                assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 0, "empty changesets should be suppressed");
            }
            assert!(!results.has_completed(), "the source has not completed");

            subscription.dispose();
            source.dispose();
        }

        #[test]
        fn items_are_moved_matching_movements_propagate_case1() {
            items_are_moved_matching_movements_propagate(filter_fixture_base_support::EmptyChangesetPolicy::IncludeEmptyChangesets);
        }

        #[test]
        fn items_are_moved_matching_movements_propagate_case2() {
            items_are_moved_matching_movements_propagate(filter_fixture_base_support::EmptyChangesetPolicy::SuppressEmptyChangesets);
        }

        fn items_are_refreshed_items_are_re_filtered_or_refreshed(empty_changeset_policy: filter_fixture_base_support::EmptyChangesetPolicy) {
            #[allow(unused_imports)]
            use filter_fixture_base_support::*;

            // Setup
            let source = TestSourceList::<Rc<Item>>::new();

            source.edit(|list| {
                list.add_range(vec![Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)])
            });

            // UUT Initialization
            let (subscription, results) = $build_uut(
                source.connect(),
                Item::filter_by_is_included,
                empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
            )
            .validate_synchronization()
            .validate_list_change_sets()
            .record_list_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
            assert_equivalent_because(
                &results.recorded_items(),
                &source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all matching items should have propagated",
            );
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action (add items)
            for item in source.items() {
                item.set_is_included(true);
            }

            source.refresh();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
            assert_list_should_have_refreshed(&results.recorded_change_sets()[1], &source.items()[..3], "all unchanged items should have been refreshed");
            assert_equivalent_because(
                &results.recorded_items(),
                &source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all newly-matching items should have been added",
            );
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action (remove items)
            for item in source.items().into_iter().take(3) {
                item.set_is_included(false);
            }

            source.refresh();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source operation was performed");
            assert_list_should_have_refreshed(&results.recorded_change_sets()[2], &source.items()[3..], "all unchanged items should have been refreshed");
            assert_equivalent_because(
                &results.recorded_items(),
                &source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all newly-excluded items should have been removed",
            );
            assert!(!results.has_completed(), "the source has not completed");

            subscription.dispose();
            source.dispose();
        }

        #[test]
        fn items_are_refreshed_items_are_re_filtered_or_refreshed_case1() {
            items_are_refreshed_items_are_re_filtered_or_refreshed(filter_fixture_base_support::EmptyChangesetPolicy::IncludeEmptyChangesets);
        }

        #[test]
        fn items_are_refreshed_items_are_re_filtered_or_refreshed_case2() {
            items_are_refreshed_items_are_re_filtered_or_refreshed(filter_fixture_base_support::EmptyChangesetPolicy::SuppressEmptyChangesets);
        }

        fn items_are_replaced_items_are_re_filtered(empty_changeset_policy: filter_fixture_base_support::EmptyChangesetPolicy) {
            #[allow(unused_imports)]
            use filter_fixture_base_support::*;

            // Setup
            let source = TestSourceList::<Rc<Item>>::new();

            source.edit(|list| {
                list.add_range(vec![Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)])
            });

            // UUT Intialization
            let (subscription, results) = $build_uut(
                source.connect(),
                Item::filter_by_is_included,
                empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
            )
            .validate_synchronization()
            .validate_list_change_sets()
            .record_list_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
            assert_eq!(
                results.recorded_items(),
                source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all matching items should have propagated"
            );
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action (add and replace items): upstream's `source.ReplaceAt`, an edit nested in
            // the edit.
            source.edit(|_items| {
                source.edit(|list| list.set(0, Item::new(1, true)));
                source.edit(|list| list.set(1, Item::new(2, true)));
                source.edit(|list| list.set(2, Item::new(3, true)));
                source.edit(|list| list.set(3, Item::new(4, true)));
                source.edit(|list| list.set(4, Item::new(5, true)));
                source.edit(|list| list.set(5, Item::new(6, true)));
            });

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
            assert_eq!(
                results.recorded_items(),
                source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all newly-matching items should have been added"
            );
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action (remove and replace items)
            source.edit(|_items| {
                source.edit(|list| list.set(0, Item::new(1, false)));
                source.edit(|list| list.set(1, Item::new(2, false)));
                source.edit(|list| list.set(2, Item::new(3, false)));
                source.edit(|list| list.set(3, Item::new(4, true)));
                source.edit(|list| list.set(4, Item::new(5, true)));
                source.edit(|list| list.set(5, Item::new(6, true)));
            });

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(2).count(), 1, "1 source operation was performed");
            assert_eq!(
                results.recorded_items(),
                source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all newly-excluded items should have been removed"
            );
            assert!(!results.has_completed(), "the source has not completed");

            subscription.dispose();
            source.dispose();
        }

        #[test]
        fn items_are_replaced_items_are_re_filtered_case1() {
            items_are_replaced_items_are_re_filtered(filter_fixture_base_support::EmptyChangesetPolicy::SuppressEmptyChangesets);
        }

        #[test]
        fn items_are_replaced_items_are_re_filtered_case2() {
            items_are_replaced_items_are_re_filtered(filter_fixture_base_support::EmptyChangesetPolicy::IncludeEmptyChangesets);
        }

        fn matching_items_are_removed_removals_propagate(empty_changeset_policy: filter_fixture_base_support::EmptyChangesetPolicy) {
            #[allow(unused_imports)]
            use filter_fixture_base_support::*;

            // Setup
            let source = TestSourceList::<Rc<Item>>::new();

            source.edit(|list| {
                list.add_range(vec![Item::new(1, true), Item::new(2, true), Item::new(3, true), Item::new(4, false), Item::new(5, false), Item::new(6, false)])
            });

            // UUT Initialization
            let (subscription, results) = $build_uut(
                source.connect(),
                Item::filter_by_is_included,
                empty_changeset_policy == EmptyChangesetPolicy::SuppressEmptyChangesets,
            )
            .validate_synchronization()
            .validate_list_change_sets()
            .record_list_items();

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
            assert_eq!(
                results.recorded_items(),
                source.items().into_iter().filter(Item::filter_by_is_included).collect::<Vec<_>>(),
                "all matching items should have propagated"
            );
            assert!(!results.has_completed(), "the source has not completed");

            // UUT Action
            let matching_items: Vec<Rc<Item>> = source.items().into_iter().filter(Item::filter_by_is_included).collect();
            source.edit(|list| remove_many(list, &matching_items));

            assert!(results.error().is_none());
            assert_eq!(results.recorded_change_sets().iter().skip(1).count(), 1, "1 source operation was performed");
            assert!(results.recorded_items().is_empty(), "all matching items were removed");
            assert!(!results.has_completed(), "the source has not completed");

            subscription.dispose();
            source.dispose();
        }

        #[test]
        fn matching_items_are_removed_removals_propagate_case1() {
            matching_items_are_removed_removals_propagate(filter_fixture_base_support::EmptyChangesetPolicy::IncludeEmptyChangesets);
        }

        #[test]
        fn matching_items_are_removed_removals_propagate_case2() {
            matching_items_are_removed_removals_propagate(filter_fixture_base_support::EmptyChangesetPolicy::SuppressEmptyChangesets);
        }

        fn source_fails_error_propagates(source_type: filter_fixture_base_support::SourceType) {
            #[allow(unused_imports)]
            use filter_fixture_base_support::*;

            let source = TestSourceList::<Rc<Item>>::new();

            let error = new_error("Test");

            if source_type == SourceType::Immediate {
                source.set_error(error.clone());
            }

            let (subscription, results) = $build_uut(source.connect(), Item::filter_by_is_included, true)
                .validate_synchronization()
                .validate_list_change_sets()
                .record_list_items();

            if source_type == SourceType::Asynchronous {
                source.set_error(error.clone());
            }

            assert!(is_same_error(results.error(), &error));
            if source_type == SourceType::Asynchronous {
                assert_eq!(results.recorded_change_sets().len(), 1, "the initial changeset should propagate");
                assert!(results.recorded_items().is_empty(), "no source items were added");
            } else {
                assert!(results.recorded_change_sets().is_empty(), "an error occurred during initialization");
            }

            subscription.dispose();
            source.dispose();
        }

        #[test]
        fn source_fails_error_propagates_case1() {
            source_fails_error_propagates(filter_fixture_base_support::SourceType::Asynchronous);
        }

        #[test]
        fn source_fails_error_propagates_case2() {
            source_fails_error_propagates(filter_fixture_base_support::SourceType::Immediate);
        }

        // Not ported: SourceIsNull_ThrowsException - argument validation
    };
}
#[allow(unused_imports)]
pub(crate) use filter_fixture_base_tests;
