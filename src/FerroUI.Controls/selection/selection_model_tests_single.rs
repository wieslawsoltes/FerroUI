use super::{ISelectionModel, SelectionModel, SelectionNodeBaseImpl};
use crate::items_source::{box_item, ItemsChangedEventArgs, ItemsSource};
use crate::utils::{CollectionChangedEventManager, ICollectionChangedListener};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::reactive::IDisposable;
use ferroui_base::BoxedValue;
use std::cell::Cell;
use std::rc::{Rc, Weak};

type Target = Rc<SelectionModel<String>>;

fn create_target_with_data() -> (Target, Rc<FerroList<String>>) {
    let result = SelectionModel::<String>::new();
    result.set_single_select(true);
    let data = Rc::new(FerroList::from_items(["foo", "bar", "baz"].map(str::to_string)));
    result.set_source(Some(data.clone().into()));
    (result, data)
}

fn create_target(create_data: bool) -> Target {
    if create_data {
        create_target_with_data().0
    } else {
        let result = SelectionModel::<String>::new();
        result.set_single_select(true);
        result
    }
}

fn strs(values: &[&str]) -> Vec<Option<String>> {
    values.iter().map(|v| Some(v.to_string())).collect()
}

fn source(values: &[&str]) -> Option<ItemsSource> {
    Some(ItemsSource::from_strs(values.iter().copied()))
}

fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

fn inc(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

/// Counts the property changed notifications for a property.
fn count_property(target: &Target, property_name: &'static str) -> Rc<Cell<i32>> {
    let raised = counter();
    let r = raised.clone();
    target.property_changed().add(Rc::new(move |name: &str| {
        if name == property_name {
            inc(&r);
        }
    }));
    raised
}

/// Counts every property changed notification.
fn count_properties(target: &Target, raised: &Rc<Cell<i32>>) {
    let r = raised.clone();
    target.property_changed().add(Rc::new(move |_: &str| inc(&r)));
}

fn count_selection_changed(target: &Target, raised: &Rc<Cell<i32>>) {
    let r = raised.clone();
    target.selection_changed(move |_| inc(&r));
}

mod source {
    use super::*;

    #[test]
    fn can_select_index_before_source_assigned() {
        let target = create_target(false);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![5]);
            assert_eq!(e.selected_items().to_vec(), vec![None]);
            inc(&r);
        });

        target.set_selected_index(5);

        assert_eq!(target.selected_index(), 5);
        assert_eq!(target.selected_indexes().to_vec(), vec![5]);
        assert_eq!(target.selected_item(), None);
        assert_eq!(target.selected_items().to_vec(), vec![None]);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn can_select_item_before_source_assigned() {
        let target = create_target(false);
        let raised = counter();

        count_selection_changed(&target, &raised);
        target.set_selected_item(Some("bar".to_string()));

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn initializing_source_retains_valid_index_selection() {
        let target = create_target(false);
        let raised = counter();

        target.set_selected_index(1);

        count_selection_changed(&target, &raised);

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn initializing_source_removes_invalid_index_selection() {
        let target = create_target(false);
        let raised = counter();

        target.set_selected_index(5);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![5]);
            assert_eq!(e.deselected_items().to_vec(), vec![None]);
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn initializing_source_retains_valid_item_selection() {
        let target = create_target(false);
        let raised = counter();

        target.set_selected_item(Some("bar".to_string()));

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![1]);
            assert_eq!(e.selected_items().to_vec(), strs(&["bar"]));
            inc(&r);
        });

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn initializing_source_removes_invalid_item_selection() {
        let target = create_target(false);
        let raised = counter();

        target.set_selected_item(Some("qux".to_string()));
        count_selection_changed(&target, &raised);
        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn initializing_source_respects_source_index_source_item_order() {
        let target = create_target(false);

        target.set_selected_index(0);
        target.set_selected_item(Some("bar".to_string()));

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
    }

    #[test]
    fn initializing_source_respects_source_item_source_index_order() {
        let target = create_target(false);

        target.set_selected_item(Some("foo".to_string()));
        target.set_selected_index(1);

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
    }

    #[test]
    fn initializing_source_raises_selected_items_property_changed() {
        let target = create_target(false);

        target.select(1);

        let selected_item_raised = count_property(&target, "SelectedItem");
        let selected_items_raised = count_property(&target, "SelectedItems");

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(selected_item_raised.get(), 1);
        assert_eq!(selected_items_raised.get(), 1);
    }

    #[test]
    fn changing_source_to_null_doesnt_clear_selection() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(2);

        count_selection_changed(&target, &raised);

        target.set_source(None);

        assert_eq!(target.selected_index(), 2);
        assert_eq!(target.selected_indexes().to_vec(), vec![2]);
        assert_eq!(target.selected_item(), None);
        assert_eq!(target.selected_items().to_vec(), vec![None]);
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn changing_source_to_non_null_first_clears_old_selection() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(2);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![2]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["baz"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.set_source(source(&["qux", "quux", "corge"]));

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn changing_source_to_null_raises_selected_items_property_changed() {
        let target = create_target(true);

        target.select(1);

        let selected_item_raised = count_property(&target, "SelectedItem");
        let selected_items_raised = count_property(&target, "SelectedItems");

        target.set_source(None);

        assert_eq!(selected_item_raised.get(), 1);
        assert_eq!(selected_items_raised.get(), 1);
    }

    #[test]
    fn raises_property_changed() {
        let target = create_target(true);
        let raised = count_property(&target, "Source");

        target.set_source(source(&["qux", "quux", "corge"]));

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn can_assign_value_type_collection_to_selection_model_of_object() {
        let target: Rc<dyn ISelectionModel> = SelectionModel::<BoxedValue>::new();

        target.set_source(Some(ItemsSource::from_values([1, 2, 3])));
    }

    #[test]
    fn can_change_source_in_selected_item_change_handler() {
        // Issue #11617
        let target = create_target(true);
        let raised = counter();

        let (t, r) = (target.clone(), raised.clone());
        target.property_changed().add(Rc::new(move |name: &str| {
            if name == "SelectedItem" && r.get() == 0 {
                inc(&r);
                t.set_source(source(&["foo", "baz", "bar"]));
            }
        }));

        target.set_selected_index(1);

        assert_eq!(target.selected_index(), -1);
    }
}

mod selected_index {
    use super::*;

    #[test]
    fn selected_index_larger_than_source_clears_selection() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(1);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![1]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.set_selected_index(5);

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn negative_selected_index_is_coerced_to_minus_1() {
        let target = create_target(true);
        let raised = counter();

        count_selection_changed(&target, &raised);

        target.set_selected_index(-5);

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn setting_selected_index_clears_old_selection() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(0);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![0]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["foo"]));
            assert_eq!(e.selected_indexes().to_vec(), vec![1]);
            assert_eq!(e.selected_items().to_vec(), strs(&["bar"]));
            inc(&r);
        });

        target.set_selected_index(1);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn setting_selected_index_during_collection_changed_results_in_correct_selection() {
        // Issue #4496
        let data: Rc<FerroList<String>> = Rc::new(FerroList::new());
        let target = create_target(true);
        let _binding = MockBinding::new(&target, &data);

        target.set_source(Some(data.clone().into()));

        data.add("foo".to_string());

        assert_eq!(target.selected_index(), 0);
    }

    #[test]
    fn property_changed_is_raised() {
        let target = create_target(true);
        let raised = count_property(&target, "SelectedIndex");

        target.set_selected_index(1);

        assert_eq!(raised.get(), 1);
    }

    struct MockBinding {
        target: Target,
    }

    impl MockBinding {
        fn new(target: &Target, data: &Rc<FerroList<String>>) -> Rc<Self> {
            let result = Rc::new(Self { target: target.clone() });
            let weak: Weak<Self> = Rc::downgrade(&result);
            let listener: Weak<dyn ICollectionChangedListener> = weak;
            CollectionChangedEventManager::add_listener(&data.clone().into(), listener);
            result
        }
    }

    impl ICollectionChangedListener for MockBinding {
        fn changed(&self, _e: &ItemsChangedEventArgs<'_>) {
            self.target.select(0);
        }

        fn post_changed(&self, _e: &ItemsChangedEventArgs<'_>) {}

        fn pre_changed(&self, _e: &ItemsChangedEventArgs<'_>) {}
    }
}

mod selected_item {
    use super::*;

    #[test]
    fn setting_selected_item_to_valid_item_updates_selection() {
        let target = create_target(true);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![1]);
            assert_eq!(e.selected_items().to_vec(), strs(&["bar"]));
            inc(&r);
        });

        target.set_selected_item(Some("bar".to_string()));

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(true);
        let raised = count_property(&target, "SelectedItem");

        target.set_selected_index(1);

        assert_eq!(raised.get(), 1);
    }
}

mod selected_indexes {
    use super::*;

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(true);
        let raised = count_property(&target, "SelectedIndexes");

        target.set_selected_index(1);

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn collection_changed_is_raised_when_selected_index_changes() {
        let target = create_target(true);
        let raised = counter();

        let r = raised.clone();
        let token = target.selected_indexes().add_collection_changed(Rc::new(
            move |e: &ItemsChangedEventArgs<'_>| {
                // For the moment, for simplicity, we raise a Reset event when the SelectedIndexes
                // collection changes - whatever the change. This can be improved later if necessary.
                assert_eq!(e.action, NotifyCollectionChangedAction::Reset);
                inc(&r);
            },
        ));
        assert!(token.is_some());

        target.set_selected_index(1);

        assert_eq!(raised.get(), 1);
    }
}

mod selected_items {
    use super::*;

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(true);
        let raised = count_property(&target, "SelectedItems");

        target.set_selected_index(1);

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn collection_changed_is_raised_when_selected_index_changes() {
        let target = create_target(true);
        let raised = counter();

        // As in the reference test, the list observed is the list of the selected indexes.
        let r = raised.clone();
        let token = target.selected_indexes().add_collection_changed(Rc::new(
            move |e: &ItemsChangedEventArgs<'_>| {
                // For the moment, for simplicity, we raise a Reset event when the SelectedItems
                // collection changes - whatever the change. This can be improved later if necessary.
                assert_eq!(e.action, NotifyCollectionChangedAction::Reset);
                inc(&r);
            },
        ));
        assert!(token.is_some());

        target.set_selected_index(1);

        assert_eq!(raised.get(), 1);
    }
}

mod select {
    use super::*;

    #[test]
    fn select_sets_selected_index() {
        let target = create_target(true);

        target.set_selected_index(0);

        let raised = count_property(&target, "SelectedIndex");

        target.select(1);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn select_clears_old_selection() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(0);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![0]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["foo"]));
            assert_eq!(e.selected_indexes().to_vec(), vec![1]);
            assert_eq!(e.selected_items().to_vec(), strs(&["bar"]));
            inc(&r);
        });

        target.select(1);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn select_with_invalid_index_does_nothing() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(0);

        count_properties(&target, &raised);
        count_selection_changed(&target, &raised);

        target.select(5);

        assert_eq!(target.selected_index(), 0);
        assert_eq!(target.selected_indexes().to_vec(), vec![0]);
        assert_eq!(target.selected_item(), Some("foo".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["foo"]));
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn selecting_already_selected_item_doesnt_raise_selection_changed() {
        let target = create_target(true);
        let raised = counter();

        target.select(2);
        count_selection_changed(&target, &raised);
        target.select(2);

        assert_eq!(raised.get(), 0);
    }
}

mod select_range {
    use super::*;

    #[test]
    #[should_panic(expected = "Cannot select range with single selection.")]
    fn select_range_throws() {
        let target = create_target(true);

        target.select_range(0, 10);
    }
}

mod deselect {
    use super::*;

    #[test]
    fn deselect_clears_current_selection() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(0);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![0]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["foo"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.deselect(0);

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn deselect_does_nothing_for_nonselected_item() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(1);
        count_selection_changed(&target, &raised);
        target.deselect(0);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(raised.get(), 0);
    }
}

mod deselect_range {
    use super::*;

    #[test]
    fn deselect_range_clears_current_selection_for_intersecting_range() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(0);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![0]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["foo"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.deselect_range(0, 2);

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn deselect_range_does_nothing_for_nonintersecting_range() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(0);
        count_selection_changed(&target, &raised);
        target.deselect_range(1, 2);

        assert_eq!(target.selected_index(), 0);
        assert_eq!(target.selected_indexes().to_vec(), vec![0]);
        assert_eq!(target.selected_item(), Some("foo".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["foo"]));
        assert_eq!(raised.get(), 0);
    }
}

mod clear {
    use super::*;

    #[test]
    fn clear_raises_selection_changed() {
        let target = create_target(true);
        let raised = counter();

        target.select(1);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![1]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.clear();

        assert_eq!(raised.get(), 1);
    }
}

mod anchor_index {
    use super::*;

    #[test]
    fn setting_selected_index_sets_anchor_index() {
        let target = create_target(true);
        let raised = count_property(&target, "AnchorIndex");

        target.set_selected_index(1);

        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn setting_selected_index_to_minus_1_doesnt_clear_anchor_index() {
        let target = create_target(true);

        target.set_selected_index(1);

        let raised = count_property(&target, "AnchorIndex");

        target.set_selected_index(-1);

        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn select_sets_anchor_index() {
        let target = create_target(true);
        let raised = count_property(&target, "AnchorIndex");

        target.select(1);

        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn deselect_doesnt_clear_anchor_index() {
        let target = create_target(true);

        target.select(1);

        let raised = count_property(&target, "AnchorIndex");

        target.deselect(1);

        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn raises_property_changed() {
        let target = create_target(true);
        let raised = count_property(&target, "AnchorIndex");

        target.set_selected_index(1);

        assert_eq!(raised.get(), 1);
    }
}

mod single_select {
    use super::*;

    #[test]
    fn converting_to_multiple_selection_preserves_selection() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(1);

        count_selection_changed(&target, &raised);

        target.set_single_select(false);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn raises_property_changed() {
        let target = create_target(true);
        let raised = count_property(&target, "SingleSelect");

        target.set_single_select(false);

        assert_eq!(raised.get(), 1);
    }
}

mod collection_changes {
    use super::*;

    #[test]
    fn adding_item_before_selected_item_updates_indexes() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(1);

        count_selection_changed(&target, &selection_changed_raised);

        let selected_index_raised = count_property(&target, "SelectedIndex");

        let r = indexes_changed_raised.clone();
        target.indexes_changed(move |e| {
            assert_eq!(e.start_index(), 0);
            assert_eq!(e.delta(), 1);
            inc(&r);
        });

        data.insert(0, "new".to_string());

        assert_eq!(target.selected_index(), 2);
        assert_eq!(target.selected_indexes().to_vec(), vec![2]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(target.anchor_index(), 2);
        assert_eq!(indexes_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 1);
        assert_eq!(selection_changed_raised.get(), 0);
    }

    #[test]
    fn adding_item_after_selected_doesnt_raise_events() {
        let (target, data) = create_target_with_data();
        let raised = counter();

        target.set_selected_index(1);

        count_properties(&target, &raised);
        count_selection_changed(&target, &raised);
        let r = raised.clone();
        target.indexes_changed(move |_| inc(&r));

        data.insert(2, "new".to_string());

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn removing_selected_item_updates_state() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select(1);

        let selected_index_raised = count_property(&target, "SelectedIndex");

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        data.remove_at(1);

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(target.anchor_index(), -1);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 1);
    }

    #[test]
    fn removing_item_before_selected_item_updates_indexes() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(1);

        count_selection_changed(&target, &selection_changed_raised);

        let r = indexes_changed_raised.clone();
        target.indexes_changed(move |e| {
            assert_eq!(e.start_index(), 0);
            assert_eq!(e.delta(), -1);
            inc(&r);
        });

        data.remove_at(0);

        assert_eq!(target.selected_index(), 0);
        assert_eq!(target.selected_indexes().to_vec(), vec![0]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(target.anchor_index(), 0);
        assert_eq!(indexes_changed_raised.get(), 1);
        assert_eq!(selection_changed_raised.get(), 0);
    }

    #[test]
    fn removing_item_after_selected_doesnt_raise_events() {
        let (target, data) = create_target_with_data();
        let raised = counter();

        target.set_selected_index(1);

        count_properties(&target, &raised);
        count_selection_changed(&target, &raised);
        let r = raised.clone();
        target.indexes_changed(move |_| inc(&r));

        data.remove_at(2);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn replacing_selected_item_updates_state() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select(1);

        let selected_index_raised = count_property(&target, "SelectedIndex");
        let selected_item_raised = count_property(&target, "SelectedItem");

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        data.set(1, "new".to_string());

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(target.anchor_index(), -1);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 1);
        assert_eq!(selected_item_raised.get(), 1);
    }

    #[test]
    fn moving_selected_item_updates_state() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select(1);

        let selected_index_raised = count_property(&target, "SelectedIndex");
        let selected_item_raised = count_property(&target, "SelectedItem");

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        data.move_item(1, 0);

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(target.anchor_index(), -1);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 1);
        assert_eq!(selected_item_raised.get(), 1);
    }

    #[test]
    fn resetting_source_updates_state() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();
        let reset_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select(1);

        let selected_index_raised = count_property(&target, "SelectedIndex");

        count_selection_changed(&target, &selection_changed_raised);
        let r = reset_raised.clone();
        target.source_reset(move || inc(&r));

        data.clear();

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(target.anchor_index(), -1);
        assert_eq!(selection_changed_raised.get(), 0);
        assert_eq!(reset_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 1);
    }

    #[test]
    fn handles_selection_made_in_collection_changed() {
        // Tests the following scenario:
        //
        // - Items changes from empty to having 1 item
        // - ViewModel auto-selects item 0 in CollectionChanged
        // - SelectionModel receives CollectionChanged
        // - And so adjusts the selected item from 0 to 1, which is past the end of the items.
        //
        // There's not much we can do about this situation because the order in which
        // CollectionChanged handlers are called can't be known (the problem also exists with
        // WPF). The best we can do is not select an invalid index.
        let target = create_target(false);
        let data: Rc<FerroList<String>> = Rc::new(FerroList::new());

        let t = target.clone();
        data.add_collection_changed(Rc::new(move |_: &NotifyCollectionChangedEventArgs<'_, String>| {
            t.select(0);
        }));

        target.set_source(Some(data.clone().into()));
        data.add("foo".to_string());

        assert_eq!(target.selected_index(), 0);
        assert_eq!(target.selected_indexes().to_vec(), vec![0]);
        assert_eq!(target.selected_item(), Some("foo".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["foo"]));
        assert_eq!(target.anchor_index(), 0);
    }

    #[test]
    fn selected_items_indexer_is_correct() {
        // Issue #7974
        let target = create_target(true);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.selected_items().iter().next().unwrap(), Some("bar".to_string()));
            assert_eq!(e.selected_items().get(0), Some("bar".to_string()));
            inc(&r);
        });

        target.select(1);
        assert_eq!(raised.get(), 1);
    }
}

mod batch_update {
    use super::*;

    #[test]
    fn changes_do_not_take_effect_until_end_update_called() {
        let target = create_target(true);

        target.begin_batch_update();
        target.select(0);

        assert_eq!(target.selected_index(), -1);

        target.end_batch_update();

        assert_eq!(target.selected_index(), 0);
    }

    #[test]
    fn correctly_batches_clear_selected_index() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(2);
        count_selection_changed(&target, &raised);

        let update = target.batch_update();
        target.clear();
        target.set_selected_index(2);
        update.dispose();

        assert_eq!(raised.get(), 0);
    }
}

mod lost_selection {
    use super::*;

    #[test]
    fn lost_selection_called_on_clear() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(1);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![1]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert_eq!(e.selected_indexes().to_vec(), vec![0]);
            assert_eq!(e.selected_items().to_vec(), strs(&["foo"]));
            inc(&r);
        });

        let t = target.clone();
        target.lost_selection(move || {
            t.select(0);
        });

        target.clear();

        assert_eq!(target.selected_index(), 0);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn lost_selection_called_when_selected_item_removed() {
        let (target, data) = create_target_with_data();
        let raised = counter();

        target.set_selected_index(1);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert_eq!(e.selected_indexes().to_vec(), vec![0]);
            assert_eq!(e.selected_items().to_vec(), strs(&["foo"]));
            inc(&r);
        });

        let t = target.clone();
        target.lost_selection(move || {
            t.select(0);
        });

        data.remove_at(1);

        assert_eq!(target.selected_index(), 0);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn lost_selection_not_called_with_old_source_when_changing_source() {
        let (target, data) = create_target_with_data();
        let raised = counter();

        let (t, r) = (target.clone(), raised.clone());
        let data_source: ItemsSource = data.clone().into();
        target.lost_selection(move || {
            if t.source().as_ref() == Some(&data_source) {
                inc(&r);
            }
        });

        target.set_source(None);

        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn lost_selection_is_called_when_source_changed_while_collection_change_in_progress() {
        // Issue #12733.
        let data1 = Rc::new(FerroList::from_items(["foo1", "bar1", "baz1"].map(str::to_string)));
        let data2 = Rc::new(FerroList::from_items(["foo1", "bar1", "baz1"].map(str::to_string)));
        let target = SelectionModel::<String>::with_source(Some(data1.clone().into()));
        let raised = counter();

        let (t, r) = (target.clone(), raised.clone());
        let data2_source: ItemsSource = data2.clone().into();
        target.lost_selection(move || {
            if t.source().as_ref() == Some(&data2_source) {
                inc(&r);
            }
        });

        update_source(&target, Some(data2.clone().into()));

        assert_eq!(raised.get(), 1);
    }

    /// What the derived selection model of the reference test does with its
    /// protected members.
    fn update_source(target: &Target, source: Option<ItemsSource>) {
        SelectionNodeBaseImpl::on_source_collection_change_started(&**target);
        target.set_source(source);
        SelectionNodeBaseImpl::on_source_collection_change_finished(&**target);
    }
}

mod untyped_interface {
    use super::*;

    #[test]
    fn raises_untyped_selection_changed_event() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(1);

        let untyped: Rc<dyn ISelectionModel> = target.clone();
        let r = raised.clone();
        untyped.selection_changed(Rc::new(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![1]);
            assert_eq!(e.deselected_items().to_vec(), vec![box_item(&"bar".to_string())]);
            assert_eq!(e.selected_indexes().to_vec(), vec![2]);
            assert_eq!(e.selected_items().to_vec(), vec![box_item(&"baz".to_string())]);
            inc(&r);
        }));

        target.set_selected_index(2);

        assert_eq!(raised.get(), 1);
    }
}
