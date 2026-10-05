use super::SelectionModel;
use crate::items_source::{items_equal, unbox_item, IItemsList, ItemsChangedHandler, ItemsSource};
use crate::utils::CollectionUtils;
use ferroui_base::collections::{FerroList, NotifyCollectionChangedEventArgs};
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Target = Rc<SelectionModel<String>>;

fn create_target_with_data() -> (Target, Rc<FerroList<String>>) {
    let result = SelectionModel::<String>::new();
    result.set_single_select(false);
    let data = Rc::new(FerroList::from_items(
        [
            "foo", "bar", "baz", "qux", "quux", "corge", "grault", "garply", "waldo", "fred", "plugh", "xyzzy",
            "thud",
        ]
        .map(str::to_string),
    ));
    result.set_source(Some(data.clone().into()));
    (result, data)
}

fn create_target(create_data: bool) -> Target {
    if create_data {
        create_target_with_data().0
    } else {
        let result = SelectionModel::<String>::new();
        result.set_single_select(false);
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

fn count_indexes_changed(target: &Target, raised: &Rc<Cell<i32>>) {
    let r = raised.clone();
    target.indexes_changed(move |_| inc(&r));
}

mod no_source {
    use super::*;

    #[test]
    fn can_select_multiple_items_before_source_assigned() {
        let target = create_target(false);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            let index = match r.get() {
                0 => 5,
                1 => 10,
                2 => 100,
                _ => panic!("not supported"),
            };

            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![index]);
            assert_eq!(e.selected_items().to_vec(), vec![None]);
            inc(&r);
        });

        target.set_selected_index(5);
        target.select(10);
        target.select(100);

        assert_eq!(target.selected_index(), 5);
        assert_eq!(target.selected_indexes().to_vec(), vec![5, 10, 100]);
        assert_eq!(target.selected_item(), None);
        assert_eq!(target.selected_items().to_vec(), vec![None, None, None]);
        assert_eq!(raised.get(), 3);
    }

    #[test]
    fn initializing_source_retains_valid_selection_and_removes_invalid() {
        let target = create_target(false);
        let raised = counter();

        target.set_selected_index(1);
        target.select(2);
        target.select(10);
        target.select(100);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![10, 100]);
            assert_eq!(e.deselected_items().to_vec(), vec![None, None]);
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1, 2]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar", "baz"]));
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn initializing_source_coerces_selected_index() {
        let target = create_target(false);

        target.set_selected_index(100);
        target.select(2);

        let raised = count_property(&target, "SelectedIndex");

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), 2);
        assert_eq!(target.selected_indexes().to_vec(), vec![2]);
        assert_eq!(target.selected_item(), Some("baz".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["baz"]));
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn initializing_source_doesnt_raise_selection_changed_if_selection_valid() {
        let target = create_target(false);
        let raised = counter();

        target.select(1);
        target.select(2);

        count_selection_changed(&target, &raised);

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn initializing_source_raises_selected_items_property_changed() {
        let target = create_target(false);

        target.select(1);
        target.select(2);

        let selected_item_raised = count_property(&target, "SelectedItem");
        let selected_items_raised = count_property(&target, "SelectedItems");

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(selected_item_raised.get(), 1);
        assert_eq!(selected_items_raised.get(), 1);
    }

    #[test]
    fn initializing_source_respects_range_source_item_order() {
        let target = create_target(false);

        target.select_range(2, 2);
        target.set_selected_item(Some("bar".to_string()));

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
    }

    #[test]
    fn initializing_source_respects_source_item_range_order() {
        let target = create_target(false);

        target.set_selected_item(Some("baz".to_string()));
        target.select_range(1, 1);

        target.set_source(source(&["foo", "bar", "baz"]));

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
    }

    #[test]
    fn changing_source_to_null_raises_selected_items_property_changed() {
        let target = create_target(true);

        target.select(1);
        target.select(2);

        let selected_item_raised = count_property(&target, "SelectedItem");
        let selected_items_raised = count_property(&target, "SelectedItems");

        target.set_source(None);

        assert_eq!(selected_item_raised.get(), 1);
        assert_eq!(selected_items_raised.get(), 1);
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

        target.set_selected_index(15);

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
    fn property_changed_is_raised() {
        let target = create_target(true);
        let raised = count_property(&target, "SelectedIndex");

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
}

mod selected_item {
    use super::*;

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(true);
        let raised = count_property(&target, "SelectedItem");

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
}

mod select {
    use super::*;

    #[test]
    fn select_sets_selected_index_if_previously_unset() {
        let target = create_target(true);
        let raised = count_property(&target, "SelectedIndex");

        target.select(1);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn select_adds_to_selection() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(0);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![1]);
            assert_eq!(e.selected_items().to_vec(), strs(&["bar"]));
            inc(&r);
        });

        target.select(1);

        assert_eq!(target.selected_index(), 0);
        assert_eq!(target.selected_indexes().to_vec(), vec![0, 1]);
        assert_eq!(target.selected_item(), Some("foo".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["foo", "bar"]));
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn select_with_invalid_index_does_nothing() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(0);

        count_properties(&target, &raised);
        count_selection_changed(&target, &raised);

        target.select(15);

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
    fn select_range_selects_items() {
        let target = create_target(true);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![1, 2]);
            assert_eq!(e.selected_items().to_vec(), strs(&["bar", "baz"]));
            inc(&r);
        });

        target.select_range(1, 2);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1, 2]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar", "baz"]));
        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn select_range_ignores_out_of_bounds_items() {
        let target = create_target(true);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![11, 12]);
            assert_eq!(e.selected_items().to_vec(), strs(&["xyzzy", "thud"]));
            inc(&r);
        });

        target.select_range(11, 20);

        assert_eq!(target.selected_index(), 11);
        assert_eq!(target.selected_indexes().to_vec(), vec![11, 12]);
        assert_eq!(target.selected_item(), Some("xyzzy".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["xyzzy", "thud"]));
        assert_eq!(target.anchor_index(), 11);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn select_range_does_nothing_for_non_intersecting_range() {
        let target = create_target(true);
        let raised = counter();

        count_selection_changed(&target, &raised);

        target.select_range(18, 30);

        assert_eq!(target.selected_index(), -1);
        assert_eq!(target.anchor_index(), -1);
        assert_eq!(raised.get(), 0);
    }
}

mod deselect {
    use super::*;

    #[test]
    fn deselect_clears_selected_item() {
        let target = create_target(true);
        let raised = counter();

        target.set_selected_index(0);
        target.select(1);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![1]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.deselect(1);

        assert_eq!(target.selected_index(), 0);
        assert_eq!(target.selected_indexes().to_vec(), vec![0]);
        assert_eq!(target.selected_item(), Some("foo".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["foo"]));
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn deselect_updates_selected_item_to_first_selected_item() {
        let target = create_target(true);

        target.select_range(3, 5);
        target.deselect(3);

        assert_eq!(target.selected_index(), 4);
    }
}

mod deselect_range {
    use super::*;

    #[test]
    fn deselect_range_clears_identical_range() {
        let target = create_target(true);
        let raised = counter();

        target.select_range(1, 2);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![1, 2]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar", "baz"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.deselect_range(1, 2);

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn deselect_range_clears_intersecting_range() {
        let target = create_target(true);
        let raised = counter();

        target.select_range(1, 2);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![1]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.deselect_range(0, 1);

        assert_eq!(target.selected_index(), 2);
        assert_eq!(target.selected_indexes().to_vec(), vec![2]);
        assert_eq!(target.selected_item(), Some("baz".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["baz"]));
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
        target.select(2);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![1, 2]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar", "baz"]));
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

        target.set_selected_index(0);

        let raised = count_property(&target, "AnchorIndex");

        target.select(1);

        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn select_range_doesnt_overwrite_anchor_index() {
        let target = create_target(true);

        target.set_anchor_index(0);

        let raised = count_property(&target, "AnchorIndex");

        target.select_range(1, 2);

        assert_eq!(target.anchor_index(), 0);
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn deselect_doesnt_clear_anchor_index() {
        let target = create_target(true);

        target.select(0);
        target.select(1);

        let raised = count_property(&target, "AnchorIndex");

        target.deselect(1);

        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 0);
    }
}

mod single_select {
    use super::*;

    #[test]
    fn converting_to_single_selection_removes_multiple_selection() {
        let target = create_target(true);
        let raised = counter();

        target.select_range(1, 3);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![2, 3]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["baz", "qux"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        target.set_single_select(true);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn raises_property_changed() {
        let target = create_target(true);
        let raised = count_property(&target, "SingleSelect");

        target.set_single_select(true);

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
        assert_eq!(selection_changed_raised.get(), 0);
    }

    #[test]
    fn adding_item_after_selected_doesnt_raise_events() {
        let (target, data) = create_target_with_data();
        let raised = counter();

        target.set_selected_index(1);

        count_properties(&target, &raised);
        count_selection_changed(&target, &raised);
        count_indexes_changed(&target, &raised);

        data.insert(2, "new".to_string());

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn adding_item_at_beginning_of_selected_range_updates_indexes() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.select_range(4, 8);

        count_selection_changed(&target, &selection_changed_raised);

        let r = indexes_changed_raised.clone();
        target.indexes_changed(move |e| {
            assert_eq!(e.start_index(), 4);
            assert_eq!(e.delta(), 2);
            inc(&r);
        });

        data.insert_range(4, ["frank", "tank"].map(str::to_string));

        assert_eq!(target.selected_index(), 6);
        assert_eq!(target.selected_indexes().to_vec(), vec![6, 7, 8, 9, 10]);
        assert_eq!(target.selected_item(), Some("quux".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["quux", "corge", "grault", "garply", "waldo"]));
        assert_eq!(target.anchor_index(), 6);
        assert_eq!(indexes_changed_raised.get(), 1);
        assert_eq!(selection_changed_raised.get(), 0);
    }

    #[test]
    fn adding_item_at_end_of_selected_range_updates_indexes() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.select_range(4, 8);

        count_selection_changed(&target, &selection_changed_raised);

        let r = indexes_changed_raised.clone();
        target.indexes_changed(move |e| {
            assert_eq!(e.start_index(), 8);
            assert_eq!(e.delta(), 2);
            inc(&r);
        });

        data.insert_range(8, ["frank", "tank"].map(str::to_string));

        assert_eq!(target.selected_index(), 4);
        assert_eq!(target.selected_indexes().to_vec(), vec![4, 5, 6, 7, 10]);
        assert_eq!(target.selected_item(), Some("quux".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["quux", "corge", "grault", "garply", "waldo"]));
        assert_eq!(target.anchor_index(), 4);
        assert_eq!(indexes_changed_raised.get(), 1);
        assert_eq!(selection_changed_raised.get(), 0);
    }

    #[test]
    fn adding_item_in_middle_of_selected_range_updates_indexes() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.select_range(4, 8);

        count_selection_changed(&target, &selection_changed_raised);

        let r = indexes_changed_raised.clone();
        target.indexes_changed(move |e| {
            assert_eq!(e.start_index(), 6);
            assert_eq!(e.delta(), 2);
            inc(&r);
        });

        data.insert_range(6, ["frank", "tank"].map(str::to_string));

        assert_eq!(target.selected_index(), 4);
        assert_eq!(target.selected_indexes().to_vec(), vec![4, 5, 8, 9, 10]);
        assert_eq!(target.selected_item(), Some("quux".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["quux", "corge", "grault", "garply", "waldo"]));
        assert_eq!(target.anchor_index(), 4);
        assert_eq!(indexes_changed_raised.get(), 1);
        assert_eq!(selection_changed_raised.get(), 0);
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
        count_indexes_changed(&target, &raised);

        data.remove_at(2);

        assert_eq!(target.selected_index(), 1);
        assert_eq!(target.selected_indexes().to_vec(), vec![1]);
        assert_eq!(target.selected_item(), Some("bar".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["bar"]));
        assert_eq!(target.anchor_index(), 1);
        assert_eq!(raised.get(), 0);
    }

    #[test]
    fn removing_selected_range_raises_events() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select_range(4, 8);

        let selected_index_raised = count_property(&target, "SelectedIndex");

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["quux", "corge", "grault", "garply", "waldo"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        data.remove_range(4, 5);

        assert_eq!(target.selected_index(), -1);
        assert!(target.selected_indexes().is_empty());
        assert_eq!(target.selected_item(), None);
        assert!(target.selected_items().is_empty());
        assert_eq!(target.anchor_index(), -1);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 1);
    }

    #[test]
    fn removing_partial_selected_range_raises_events_1() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select_range(4, 8);

        let selected_index_raised = count_property(&target, "SelectedIndex");

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["quux", "corge", "grault"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        data.remove_range(0, 7);

        assert_eq!(target.selected_index(), 0);
        assert_eq!(target.selected_indexes().to_vec(), vec![0, 1]);
        assert_eq!(target.selected_item(), Some("garply".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["garply", "waldo"]));
        assert_eq!(target.anchor_index(), 0);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 1);
    }

    #[test]
    fn removing_partial_selected_range_raises_events_2() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select_range(4, 8);

        let selected_index_raised = count_property(&target, "SelectedIndex");

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["garply", "waldo"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        data.remove_range(7, 3);

        assert_eq!(target.selected_index(), 4);
        assert_eq!(target.selected_indexes().to_vec(), vec![4, 5, 6]);
        assert_eq!(target.selected_item(), Some("quux".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["quux", "corge", "grault"]));
        assert_eq!(target.anchor_index(), 4);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 0);
    }

    #[test]
    fn removing_partial_selected_range_raises_events_3() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select_range(4, 8);

        let selected_index_raised = count_property(&target, "SelectedIndex");

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["corge", "grault", "garply"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        data.remove_range(5, 3);

        assert_eq!(target.selected_index(), 4);
        assert_eq!(target.selected_indexes().to_vec(), vec![4, 5]);
        assert_eq!(target.selected_item(), Some("quux".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["quux", "waldo"]));
        assert_eq!(target.anchor_index(), 4);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 0);
    }

    #[test]
    fn replacing_selected_item_updates_state() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select_range(1, 4);

        let selected_index_raised = count_property(&target, "SelectedIndex");
        let selected_item_raised = count_property(&target, "SelectedItem");

        count_indexes_changed(&target, &indexes_changed_raised);

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        data.set(1, "new".to_string());

        assert_eq!(target.selected_index(), 2);
        assert_eq!(target.selected_indexes().to_vec(), vec![2, 3, 4]);
        assert_eq!(target.selected_item(), Some("baz".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["baz", "qux", "quux"]));
        assert_eq!(target.anchor_index(), 2);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 1);
        assert_eq!(selected_item_raised.get(), 1);
        assert_eq!(indexes_changed_raised.get(), 0);
    }

    #[test]
    fn moving_selected_item_updates_state() {
        let (target, data) = create_target_with_data();
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_source(Some(data.clone().into()));
        target.select_range(1, 4);

        let selected_index_raised = count_property(&target, "SelectedIndex");
        let selected_item_raised = count_property(&target, "SelectedItem");

        count_indexes_changed(&target, &indexes_changed_raised);

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        data.move_item(1, 0);

        assert_eq!(target.selected_index(), 2);
        assert_eq!(target.selected_indexes().to_vec(), vec![2, 3, 4]);
        assert_eq!(target.selected_item(), Some("baz".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["baz", "qux", "quux"]));
        assert_eq!(target.anchor_index(), 2);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(selected_index_raised.get(), 1);
        assert_eq!(selected_item_raised.get(), 1);
        assert_eq!(indexes_changed_raised.get(), 0);
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
        // - Items changes from empty to having 2 items
        // - ViewModel auto-selects range 0..1 in CollectionChanged
        // - SelectionModel receives CollectionChanged
        // - And so adjusts the selected item from 0..1 to 2..4, which is past the end of
        //   the items.
        //
        // There's not much we can do about this situation because the order in which
        // CollectionChanged handlers are called can't be known (the problem also exists with
        // WPF). The best we can do is not select an invalid index.
        let target = create_target(false);
        let data: Rc<FerroList<String>> = Rc::new(FerroList::new());

        let t = target.clone();
        data.add_collection_changed(Rc::new(move |_: &NotifyCollectionChangedEventArgs<'_, String>| {
            t.select_range(0, 1);
        }));

        target.set_source(Some(data.clone().into()));
        data.add_range(["foo", "bar"].map(str::to_string));

        assert_eq!(target.selected_index(), 0);
        assert_eq!(target.selected_indexes().to_vec(), vec![0, 1]);
        assert_eq!(target.selected_item(), Some("foo".to_string()));
        assert_eq!(target.selected_items().to_vec(), strs(&["foo", "bar"]));
        assert_eq!(target.anchor_index(), 0);
    }
}

mod batch_update {
    use super::*;

    #[test]
    fn correctly_batches_selects() {
        let target = create_target(true);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![2, 3]);
            assert_eq!(e.selected_items().to_vec(), strs(&["baz", "qux"]));
            inc(&r);
        });

        let update = target.batch_update();
        target.select(2);
        target.select(3);
        update.dispose();

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn correctly_batches_select_ranges() {
        let target = create_target(true);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![2, 3, 5, 6]);
            assert_eq!(e.selected_items().to_vec(), strs(&["baz", "qux", "corge", "grault"]));
            inc(&r);
        });

        let update = target.batch_update();
        target.select_range(2, 3);
        target.select_range(5, 6);
        update.dispose();

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn correctly_batches_select_deselect() {
        let target = create_target(true);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![2, 3]);
            assert_eq!(e.selected_items().to_vec(), strs(&["baz", "qux"]));
            inc(&r);
        });

        let update = target.batch_update();
        target.select(2);
        target.select(3);
        target.select(4);
        target.deselect(4);
        update.dispose();

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn correctly_batches_deselect_select() {
        let target = create_target(true);
        let raised = counter();

        target.select_range(2, 8);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![2, 3]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["baz", "qux"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        let update = target.batch_update();
        target.deselect(2);
        target.deselect(3);
        target.deselect(4);
        target.select(4);
        update.dispose();

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn correctly_batches_select_deselect_range() {
        let target = create_target(true);
        let raised = counter();

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![2, 3]);
            assert_eq!(e.selected_items().to_vec(), strs(&["baz", "qux"]));
            inc(&r);
        });

        let update = target.batch_update();
        target.select_range(2, 6);
        target.deselect_range(4, 8);
        update.dispose();

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn correctly_batches_deselect_select_range() {
        let target = create_target(true);
        let raised = counter();

        target.select_range(2, 8);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![2, 3]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["baz", "qux"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        let update = target.batch_update();
        target.deselect_range(2, 6);
        target.select_range(4, 8);
        update.dispose();

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn correctly_batches_clear_select() {
        let target = create_target(true);
        let raised = counter();

        target.select_range(2, 3);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![3]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["qux"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        let update = target.batch_update();
        target.clear();
        target.select(2);
        update.dispose();

        assert_eq!(raised.get(), 1);
    }

    #[test]
    fn correctly_batches_clear_selected_index() {
        let target = create_target(true);
        let raised = counter();

        target.select_range(2, 3);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert_eq!(e.deselected_indexes().to_vec(), vec![3]);
            assert_eq!(e.deselected_items().to_vec(), strs(&["qux"]));
            assert!(e.selected_indexes().is_empty());
            assert!(e.selected_items().is_empty());
            inc(&r);
        });

        let update = target.batch_update();
        target.clear();
        target.set_selected_index(2);
        update.dispose();

        assert_eq!(raised.get(), 1);
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
    fn lost_selection_called_when_selection_removed() {
        let (target, data) = create_target_with_data();
        let raised = counter();

        target.select_range(1, 3);

        let r = raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert_eq!(e.deselected_items().to_vec(), strs(&["bar", "baz", "qux"]));
            assert_eq!(e.selected_indexes().to_vec(), vec![0]);
            assert_eq!(e.selected_items().to_vec(), strs(&["quux"]));
            inc(&r);
        });

        let t = target.clone();
        target.lost_selection(move || {
            t.select(0);
        });

        data.remove_range(0, 4);

        assert_eq!(target.selected_index(), 0);
        assert_eq!(raised.get(), 1);
    }
}

mod source_reset {
    use super::*;

    #[test]
    fn can_restore_selection_in_source_reset_event() {
        let data = Rc::new(ResettingList::new(&["foo", "bar", "baz"]));
        let target = create_target(false);
        let source_reset_raised = counter();
        let selection_changed_raised = counter();

        target.set_source(Some(ItemsSource::new(data.clone())));
        target.set_selected_index(1);

        let (t, d, r) = (target.clone(), data.clone(), source_reset_raised.clone());
        target.source_reset(move || {
            t.set_selected_index(d.index_of_str("bar"));
            inc(&r);
        });

        let r = selection_changed_raised.clone();
        target.selection_changed(move |e| {
            assert!(e.deselected_indexes().is_empty());
            assert!(e.deselected_items().is_empty());
            assert_eq!(e.selected_indexes().to_vec(), vec![3]);
            assert_eq!(e.selected_items().to_vec(), strs(&["bar"]));
            inc(&r);
        });

        data.reset(Some(&["qux", "foo", "quux", "bar", "baz"]));

        assert_eq!(target.selected_index(), 3);
        assert_eq!(selection_changed_raised.get(), 1);
        assert_eq!(source_reset_raised.get(), 1);
    }
}

/// A list that only notifies of resets.
struct ResettingList {
    items: RefCell<Vec<Option<BoxedValue>>>,
    collection_changed: HandlerList<ItemsChangedHandler>,
}

impl ResettingList {
    fn new(items: &[&str]) -> Self {
        Self { items: RefCell::new(ItemsSource::from_strs(items.iter().copied()).to_vec()), collection_changed: HandlerList::new() }
    }

    fn index_of_str(&self, item: &str) -> i32 {
        let items = self.items.borrow();
        items.iter().position(|i| unbox_item::<String>(i).as_deref() == Some(item)).map_or(-1, |i| i as i32)
    }

    fn reset(&self, items: Option<&[&str]>) {
        if let Some(items) = items {
            *self.items.borrow_mut() = ItemsSource::from_strs(items.iter().copied()).to_vec();
        }

        for (_, handler) in self.collection_changed.snapshot().iter() {
            handler(&CollectionUtils::RESET_EVENT_ARGS);
        }
    }
}

impl IItemsList for ResettingList {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.items.borrow()[index].clone()
    }

    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        self.items.borrow().iter().position(|i| items_equal(i, item)).map_or(-1, |i| i as i32)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.collection_changed.add(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.collection_changed.remove(token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
