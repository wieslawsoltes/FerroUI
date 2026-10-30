// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root.
// Behavioral ports of Repeater/APITests/SelectionModelTests.cs and IndexPathTests.cs.
// Tests explicitly marked as FerroUI additions exercise Rust ownership and error boundaries.

use super::*;
use ferroui_controls::{IItemsList, ItemsSource};
use ferroui_base::{collections::FerroList, data::model::INotifyPropertyChanged, BoxedValue};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn path(indexes: &[i32]) -> IndexPath {
    IndexPath::create_from_indices(indexes.iter().copied())
}
fn flat() -> (Rc<SelectionModel>, FerroList<i32>) {
    let source = FerroList::from_items(0..10);
    let model = SelectionModel::new();
    model.set_source(Some(Rc::new(source.clone()).into()));
    (model, source)
}
fn nested() -> (Rc<SelectionModel>, Vec<FerroList<i32>>) {
    let groups = (0..3)
        .map(|_| FerroList::from_items(0..3))
        .collect::<Vec<_>>();
    let root = ItemsSource::from_values(
        groups
            .iter()
            .map(|group| ItemsSource::from(Rc::new(group.clone()))),
    );
    let model = SelectionModel::new();
    model.set_source(Some(root));
    (model, groups)
}
fn three_levels() -> (Rc<SelectionModel>, Vec<Vec<FerroList<i32>>>) {
    let groups = (0..2)
        .map(|_| {
            (0..2)
                .map(|_| FerroList::from_items(0..3))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let root = ItemsSource::from_values(groups.iter().map(|group| {
        ItemsSource::from_values(
            group
                .iter()
                .map(|leaf| ItemsSource::from(Rc::new(leaf.clone()))),
        )
    }));
    let model = SelectionModel::new();
    model.set_source(Some(root));
    (model, groups)
}
fn indices(model: &SelectionModel) -> Vec<String> {
    model
        .selected_indices()
        .iter()
        .map(|path| path.unwrap().to_string())
        .collect()
}
fn count_events(model: &SelectionModel) -> Rc<Cell<usize>> {
    let calls = Rc::new(Cell::new(0));
    let handler_calls = calls.clone();
    model.selection_changed(move |_| handler_calls.set(handler_calls.get() + 1));
    calls
}

fn nested_source(levels: usize, groups: usize, leaves: usize) -> ItemsSource {
    if levels == 0 {
        ItemsSource::from_values(0..leaves as i32)
    } else {
        ItemsSource::from_values((0..groups).map(|_| nested_source(levels - 1, groups, leaves)))
    }
}

fn mutable_groups() -> (Rc<SelectionModel>, FerroList<ItemsSource>) {
    let groups = FerroList::from_items((0..3).map(|_| ItemsSource::from_values(0..3)));
    let model = SelectionModel::new();
    model.set_source(Some(Rc::new(groups.clone()).into()));
    (model, groups)
}

#[test]
fn validate_index_path() {
    assert_eq!(path(&[]).to_string(), "R");
    assert_eq!(path(&[1, 2, 3]).to_string(), "R.1.2.3");
    assert_eq!(IndexPath::create_from(1), path(&[1]));
    assert_eq!(
        IndexPath::create_from_group_and_item_index(1, 2),
        path(&[1, 2])
    );
    for (lhs, rhs, order) in [
        (path(&[]), path(&[0]), -1),
        (path(&[1]), path(&[1, 2]), -1),
        (path(&[1, 2]), path(&[1]), 1),
        (path(&[3]), path(&[2, 4]), 1),
        (path(&[1, 2]), path(&[1, 2]), 0),
    ] {
        assert_eq!(lhs.compare_to(&rhs), order);
    }
    assert_eq!(path(&[2]).get_at(1), Err(SelectionError::IndexOutOfRange));
}

#[test]
fn validate_one_level_single_selection_no_source() {
    let model = SelectionModel::new();
    model.set_single_select(true);
    model.select(4).unwrap();
    assert_eq!(indices(&model), ["R.4"]);
    assert!(model.selected_items().is_empty());
    model.deselect(4).unwrap();
    assert!(indices(&model).is_empty());
}

#[test]
fn validate_one_level_single_selection() {
    let (model, _) = flat();
    model.set_single_select(true);
    model.select(3).unwrap();
    assert_eq!(indices(&model), ["R.3"]);
    model.deselect(3).unwrap();
    assert!(indices(&model).is_empty());
    model.select_at(&path(&[4])).unwrap();
    assert_eq!(indices(&model), ["R.4"]);
    model.deselect_at(&path(&[4])).unwrap();
    assert!(indices(&model).is_empty());
}

#[test]
fn validate_selection_changed_event_single_selection() {
    let (model, _) = flat();
    model.set_single_select(true);
    let calls = count_events(&model);
    model.select_at(&path(&[4])).unwrap();
    assert_eq!(calls.get(), 1);
    model.deselect_at(&path(&[4])).unwrap();
    assert_eq!(calls.get(), 2);
}

#[test]
fn validate_selection_changed_event_multiple_selection() {
    let (model, _) = flat();
    let calls = count_events(&model);
    let weak = Rc::downgrade(&model);
    model.selection_changed(move |_| assert_eq!(indices(&weak.upgrade().unwrap()), ["R.4"]));
    model.select(4).unwrap();
    assert_eq!(calls.get(), 1);
}

#[test]
fn validate_can_set_selected_index() {
    let model = SelectionModel::new();
    model.set_selected_index(Some(path(&[34]))).unwrap();
    assert_eq!(model.selected_index(), Some(path(&[34])));
}

#[test]
fn validate_one_level_multiple_selection() {
    let (model, _) = flat();
    model.select(4).unwrap();
    model.select_range_from_anchor(8).unwrap();
    assert_eq!(indices(&model), ["R.4", "R.5", "R.6", "R.7", "R.8"]);
    model.clear_selection();
    model.set_anchor_index_flat(6).unwrap();
    model.select_range_from_anchor(3).unwrap();
    assert_eq!(indices(&model), ["R.3", "R.4", "R.5", "R.6"]);
    model.set_anchor_index_flat(4).unwrap();
    model.deselect_range_from_anchor(5).unwrap();
    assert_eq!(indices(&model), ["R.3", "R.6"]);
}

#[test]
fn validate_two_level_single_selection() {
    let (model, _) = nested();
    model.select_with_group(1, 1).unwrap();
    assert_eq!(indices(&model), ["R.1.1"]);
    assert_eq!(model.is_selected(1), None);
    model.deselect_with_group(1, 1).unwrap();
    assert!(indices(&model).is_empty());
}

#[test]
fn validate_two_level_multiple_selection() {
    let (model, _) = nested();
    model.select_with_group(1, 2).unwrap();
    model.select_range_from_anchor_with_group(2, 2).unwrap();
    assert_eq!(indices(&model), ["R.1.2", "R.2.0", "R.2.1", "R.2.2"]);
    assert_eq!(model.is_selected(2), Some(true));
    assert_eq!(model.is_selected(1), None);
    model.clear_selection();
    model.set_anchor_index_with_group(2, 1).unwrap();
    model.select_range_from_anchor_with_group(0, 1).unwrap();
    assert_eq!(
        indices(&model),
        ["R.0.1", "R.0.2", "R.1.0", "R.1.1", "R.1.2", "R.2.0", "R.2.1"]
    );
}

#[test]
fn validate_nested_single_selection() {
    let model = SelectionModel::new();
    model.set_single_select(true);
    model.set_source(Some(nested_source(3, 2, 2)));
    model.select_at(&path(&[1, 0, 1, 1])).unwrap();
    assert_eq!(indices(&model), ["R.1.0.1.1"]);
    for ancestor in [&[][..], &[1][..], &[1, 0][..], &[1, 0, 1][..]] {
        assert_eq!(model.is_selected_at(&path(ancestor)).unwrap(), None);
    }
    model.select_at(&path(&[0, 0, 1, 0])).unwrap();
    assert_eq!(indices(&model), ["R.0.0.1.0"]);
    assert_eq!(model.is_selected_at(&path(&[1])).unwrap(), Some(false));
    model.deselect_at(&path(&[0, 0, 1, 0])).unwrap();
    assert!(indices(&model).is_empty());
}

#[test]
fn validate_nested_multiple_selection() {
    for custom_children in [true, false] {
        let model = SelectionModel::new();
        model.set_source(Some(nested_source(3, 2, 4)));
        let requests = Rc::new(RefCell::new(Vec::new()));
        if custom_children {
            let requests = requests.clone();
            model.children_requested(move |args| {
                requests
                    .borrow_mut()
                    .push(args.source_index().unwrap().to_string());
                args.set_children(
                    args.source()
                        .unwrap()
                        .as_ref()
                        .and_then(SelectionModel::source_from_item),
                );
            });
        }
        model.select_at(&path(&[1, 0, 1, 0])).unwrap();
        model
            .select_range_from_anchor_to(&path(&[1, 1, 1, 0]))
            .unwrap();
        assert_eq!(
            indices(&model),
            [
                "R.1.0.1.0",
                "R.1.0.1.1",
                "R.1.0.1.2",
                "R.1.0.1.3",
                "R.1.1.0.0",
                "R.1.1.0.1",
                "R.1.1.0.2",
                "R.1.1.0.3",
                "R.1.1.1.0"
            ]
        );
        assert_eq!(model.is_selected_at(&path(&[1, 0, 1])).unwrap(), Some(true));
        assert_eq!(model.is_selected_at(&path(&[1, 1, 1])).unwrap(), None);
        if custom_children {
            assert_eq!(
                *requests.borrow(),
                [
                    "R.1",
                    "R.1.0",
                    "R.1.0.1",
                    "R.1.1",
                    "R.1.0.1.3",
                    "R.1.0.1.2",
                    "R.1.0.1.1",
                    "R.1.0.1.0",
                    "R.1.1.1",
                    "R.1.1.0",
                    "R.1.1.0.3",
                    "R.1.1.0.2",
                    "R.1.1.0.1",
                    "R.1.1.0.0",
                    "R.1.1.1.0"
                ]
            );
        }
        model.clear_selection();
        model.set_anchor_index(Some(path(&[0, 1, 0, 2]))).unwrap();
        model
            .select_range_from_anchor_to(&path(&[0, 0, 0, 2]))
            .unwrap();
        assert_eq!(
            indices(&model),
            [
                "R.0.0.0.2",
                "R.0.0.0.3",
                "R.0.0.1.0",
                "R.0.0.1.1",
                "R.0.0.1.2",
                "R.0.0.1.3",
                "R.0.1.0.0",
                "R.0.1.0.1",
                "R.0.1.0.2"
            ]
        );
        model
            .deselect_range_from_anchor_to(&path(&[0, 0, 0, 2]))
            .unwrap();
        assert!(indices(&model).is_empty());
    }
}

#[test]
fn validate_inserts() {
    let (model, data) = flat();
    for i in 3..=5 {
        model.select(i).unwrap();
    }
    for value in [41, 42, 43] {
        data.insert(4, value);
    }
    assert_eq!(indices(&model), ["R.3", "R.7", "R.8"]);
    for value in [100, 101, 102] {
        data.insert(0, value);
    }
    assert_eq!(indices(&model), ["R.6", "R.10", "R.11"]);
    for value in [1000, 1001, 1002] {
        data.insert(12, value);
    }
    assert_eq!(indices(&model), ["R.6", "R.10", "R.11"]);
    assert_eq!(model.is_selected_at(&path(&[])).unwrap(), None);
}

#[test]
fn validate_group_inserts() {
    let (model, groups) = mutable_groups();
    model.select_with_group(1, 1).unwrap();
    groups.insert(0, ItemsSource::from_values([100]));
    assert_eq!(indices(&model), ["R.2.1"]);
    assert_eq!(model.is_selected(2), None);
    groups.insert(3, ItemsSource::from_values([1000]));
    assert_eq!(indices(&model), ["R.2.1"]);
    assert_eq!(model.anchor_index(), Some(path(&[2, 1])));
}

#[test]
fn validate_removes() {
    let (model, data) = flat();
    for i in 6..=8 {
        model.select(i).unwrap();
    }
    data.remove_at(0);
    assert_eq!(indices(&model), ["R.5", "R.6", "R.7"]);
    for _ in 0..3 {
        data.remove_at(3);
    }
    assert_eq!(indices(&model), ["R.3", "R.4"]);
    data.remove_at(5);
    assert_eq!(indices(&model), ["R.3", "R.4"]);
}

#[test]
fn validate_group_removes() {
    let (model, groups) = mutable_groups();
    model.select_with_group(1, 1).unwrap();
    model.select_with_group(1, 2).unwrap();
    groups.remove_at(0);
    assert_eq!(indices(&model), ["R.0.1", "R.0.2"]);
    groups.remove_at(1);
    assert_eq!(indices(&model), ["R.0.1", "R.0.2"]);
    groups.remove_at(0);
    assert!(indices(&model).is_empty());
}

#[test]
fn can_replace_item() {
    let (model, data) = flat();
    for i in 3..=5 {
        model.select(i).unwrap();
    }
    data.set(3, 300);
    data.set(4, 400);
    assert_eq!(indices(&model), ["R.5"]);
}

#[test]
fn validate_group_replace_loses_selection() {
    let (model, groups) = mutable_groups();
    model.select_with_group(1, 1).unwrap();
    groups.set(1, ItemsSource::from_values(0..5));
    assert!(indices(&model).is_empty());
}

#[test]
fn validate_clear() {
    let (model, data) = flat();
    for i in 3..=5 {
        model.select(i).unwrap();
    }
    data.clear();
    assert!(indices(&model).is_empty());
    assert!(model.selected_item().is_none());
}

#[test]
fn validate_group_clear() {
    let (model, groups) = nested();
    model.select_with_group(1, 1).unwrap();
    groups[1].clear();
    assert!(indices(&model).is_empty());
}

#[test]
fn validate_event_when_inner_node_changes_selection_state() {
    let (model, groups) = nested();
    for i in 0..3 {
        model.select_with_group(1, i).unwrap();
    }
    let calls = count_events(&model);
    groups[1].insert(0, 100);
    assert_eq!(calls.get(), 1);
    assert_eq!(model.is_selected(1), None);
    assert_eq!(indices(&model), ["R.1.1", "R.1.2", "R.1.3"]);
    groups[1].remove_at(0);
    assert_eq!(calls.get(), 2);
    assert_eq!(model.is_selected(1), Some(true));
}

#[test]
fn can_extend_selection_model_inpc() {
    // Rust service composition replaces inheriting the WinRT service; the explicit
    // notification hook preserves the binding contract of custom service properties.
    struct CustomModel {
        selection: Rc<SelectionModel>,
        value: Cell<i32>,
    }
    let custom = CustomModel {
        selection: SelectionModel::new(),
        value: Cell::new(0),
    };
    let names = Rc::new(RefCell::new(Vec::new()));
    let observed = names.clone();
    custom
        .selection
        .property_changed()
        .add(Rc::new(move |name| {
            observed.borrow_mut().push(name.to_string())
        }));
    custom.value.set(5);
    custom.selection.on_property_changed("IntProperty");
    assert_eq!(*names.borrow(), ["IntProperty"]);
}

#[test]
fn select_range_regression_test() {
    let model = SelectionModel::new();
    model.set_source(Some(nested_source(1, 2, 3)));
    model.select_range(&path(&[0]), &path(&[1, 1])).unwrap();
    assert_eq!(
        indices(&model),
        ["R.0.0", "R.0.1", "R.0.2", "R.1.0", "R.1.1"]
    );
    assert_eq!(model.is_selected(0), Some(true));
    assert_eq!(model.is_selected(1), None);

    // The source test's second endpoint names group 2 in a two-group source.
    // Checked Rust traversal returns the contract error for that invalid path.
    let model = SelectionModel::new();
    model.set_source(Some(nested_source(2, 2, 1)));
    assert_eq!(
        model.select_range(&path(&[1]), &path(&[2])),
        Err(SelectionError::IndexOutOfRange)
    );
}

#[test]
fn verify_custom_property_provider_members() {
    use ferroui_base::metadata::MarkupTyped;
    let metadata = <SelectionModel as MarkupTyped>::MARKUP;
    assert_eq!(
        metadata.full_name(),
        "FerroUI.Controls.Repeater.SelectionModel"
    );
    assert!(metadata.find_property("NotAProperty").is_none());
    assert!(metadata
        .find_property("SelectedItem")
        .unwrap()
        .set
        .is_none());
    let (model, _) = flat();
    model.select(3).unwrap();
    let boxed: BoxedValue = Rc::new(model);
    let property = metadata.find_property("SelectedItem").unwrap();
    let value = (property.get.unwrap())(&[Some(boxed)]).unwrap().unwrap();
    assert_eq!(value.downcast_ref::<i32>(), Some(&3));
}

#[test]
fn validate_property_changed_event_is_raised() {
    let (model, _) = flat();
    let names = Rc::new(RefCell::new(Vec::new()));
    let seen = names.clone();
    model.property_changed().add(Rc::new(move |name| {
        seen.borrow_mut().push(name.to_string())
    }));
    model.select(3).unwrap();
    assert_eq!(
        *names.borrow(),
        [
            "AnchorIndex",
            "SelectedIndex",
            "SelectedIndices",
            "SelectedItem",
            "SelectedItems"
        ]
    );
}

#[test]
fn already_selected_does_not_raise_event() {
    for single in [true, false] {
        let (model, _) = flat();
        model.set_single_select(single);
        model.select_at(&path(&[3])).unwrap();
        let calls = count_events(&model);
        model.select_at(&path(&[3])).unwrap();
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn already_deselected_does_not_raise_event() {
    for single in [true, false] {
        let (model, _) = flat();
        model.set_single_select(single);
        let calls = count_events(&model);
        model.deselect_at(&path(&[3])).unwrap();
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn validate_selection_mode_change_from_multiple_to_single() {
    let (model, _) = flat();
    model.select(3).unwrap();
    model.select(1).unwrap();
    model.set_single_select(true);
    assert_eq!(indices(&model), ["R.1"]);
}

#[test]
fn validate_selection_mode_change_from_multiple_to_single_selection_changed_event() {
    for selected_count in 0..3 {
        let (model, _) = flat();
        for index in 0..selected_count {
            model.select(index).unwrap();
        }
        let calls = count_events(&model);
        model.set_single_select(true);
        assert_eq!(calls.get(), usize::from(selected_count > 1));
    }
}

#[test]
fn verify_select_all_on_flat_source() {
    let (model, _) = flat();
    model.select_all().unwrap();
    assert_eq!(model.selected_indices().count(), 10);
    for i in 0..10 {
        assert_eq!(model.is_selected(i), Some(true));
    }
}

#[test]
fn verify_select_all_on_nested_source() {
    let (model, _) = three_levels();
    model.select_all().unwrap();
    assert_eq!(model.is_selected_at(&path(&[])).unwrap(), Some(true));
    for i in 0..2 {
        for j in 0..2 {
            for k in 0..3 {
                assert_eq!(model.is_selected_at(&path(&[i, j, k])).unwrap(), Some(true));
            }
        }
    }
}

#[test]
fn verify_is_selected_with_group_index() {
    let (model, _) = nested();
    assert_eq!(model.is_selected_with_group(1, 2), Some(false));
    model.select_with_group(1, 2).unwrap();
    assert_eq!(model.is_selected_with_group(1, 2), Some(true));
    assert_eq!(model.is_selected_with_group(1, 0), Some(false));
}

#[test]
fn verify_single_select_with_group_index_replaces_selection() {
    let (model, _) = nested();
    model.set_single_select(true);
    model.select_with_group(0, 1).unwrap();
    model.select_with_group(2, 2).unwrap();
    assert_eq!(indices(&model), ["R.2.2"]);
}

#[test]
fn verify_deselect_range() {
    let (model, _) = flat();
    model.select_range(&path(&[0]), &path(&[9])).unwrap();
    let calls = count_events(&model);
    model.deselect_range(&path(&[3]), &path(&[5])).unwrap();
    assert_eq!(
        indices(&model),
        ["R.0", "R.1", "R.2", "R.6", "R.7", "R.8", "R.9"]
    );
    assert_eq!(calls.get(), 1);
    model.deselect_range(&path(&[8]), &path(&[7])).unwrap();
    assert_eq!(indices(&model), ["R.0", "R.1", "R.2", "R.6", "R.9"]);
    assert_eq!(calls.get(), 2);
}

#[test]
fn verify_property_changed_handler_removal() {
    let (model, _) = flat();
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let token = model
        .property_changed()
        .add(Rc::new(move |_| called.set(called.get() + 1)));
    model.property_changed().remove(token);
    model.select(1).unwrap();
    assert_eq!(calls.get(), 0);
}

#[test]
fn verify_children_requested_args_only_valid_during_handler() {
    let (model, _) = nested();
    let captured = Rc::new(RefCell::new(None));
    let seen = captured.clone();
    model.children_requested(move |args| {
        assert_eq!(args.source_index().unwrap().to_string(), "R.1");
        args.set_children(
            args.source()
                .unwrap()
                .as_ref()
                .and_then(SelectionModel::source_from_item),
        );
        *seen.borrow_mut() = Some(args.clone());
    });
    model.select_with_group(1, 2).unwrap();
    let args = captured.borrow().clone().unwrap();
    assert!(matches!(
        args.source(),
        Err(SelectionError::EventArgsExpired)
    ));
    assert_eq!(args.source_index(), Err(SelectionError::EventArgsExpired));
    assert!(args.children().is_none());
}

#[test]
fn verify_children_requested_handler_removal() {
    let (model, _) = nested();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let subscription = model.children_requested(move |args| {
        seen.set(seen.get() + 1);
        args.set_children(
            args.source()
                .unwrap()
                .as_ref()
                .and_then(SelectionModel::source_from_item),
        );
    });
    model.select_with_group(1, 2).unwrap();
    assert_eq!(calls.get(), 1);
    subscription.dispose();
    model.clear_selection();
    model.select_with_group(2, 1).unwrap();
    assert_eq!(calls.get(), 1);
}

// FerroUI additions: synchronous source callbacks must not install stale child identities.
#[test]
fn children_requested_rejects_same_source_collection_changes() {
    for mutation in ["insert", "remove", "replace", "move", "reset"] {
        let first = FerroList::from_items([10, 11]);
        let second = FerroList::from_items([20, 21]);
        let groups = FerroList::from_items([
            ItemsSource::from(Rc::new(first.clone())),
            ItemsSource::from(Rc::new(second.clone())),
        ]);
        let model = SelectionModel::new();
        model.set_source(Some(Rc::new(groups.clone()).into()));
        let edited = groups.clone();
        let subscription = model.children_requested(move |args| {
            args.set_children(
                args.source().unwrap().as_ref().and_then(SelectionModel::source_from_item),
            );
            match mutation {
                "insert" => edited.insert(0, ItemsSource::from_values([30, 31])),
                "remove" => { edited.remove_at(0); }
                "replace" => edited.set(0, ItemsSource::from_values([30, 31])),
                "move" => edited.move_item(0, 1),
                "reset" => edited.clear(),
                _ => unreachable!(),
            }
        });

        assert_eq!(model.select_with_group(0, 0), Err(SelectionError::SelectionChanged), "{mutation}");
        assert!(model.selected_indices().is_empty(), "{mutation}");
        assert!(!first.has_collection_changed_subscribers(), "{mutation}");
        assert!(!second.has_collection_changed_subscribers(), "{mutation}");

        subscription.dispose();
        if groups.count() > 0 {
            model.select_with_group(0, 0).unwrap();
            let expected = groups.get(0).get_at(0).unwrap();
            assert_eq!(model.selected_item().unwrap().downcast_ref::<i32>(), expected.downcast_ref::<i32>());
        }
    }
}

#[test]
fn children_requested_rejects_discarded_ancestor_nodes() {
    for clear_source in [false, true] {
        let leaves = FerroList::from_items([10, 11]);
        let groups = FerroList::from_items([ItemsSource::from_values([
            ItemsSource::from(Rc::new(leaves.clone())),
        ])]);
        let model = SelectionModel::new();
        model.set_source(Some(Rc::new(groups.clone()).into()));
        let weak = Rc::downgrade(&model);
        let edited = groups.clone();
        let subscription = model.children_requested(move |args| {
            args.set_children(
                args.source().unwrap().as_ref().and_then(SelectionModel::source_from_item),
            );
            if args.source_index().unwrap() == path(&[0, 0]) {
                if clear_source { edited.clear(); }
                else { weak.upgrade().unwrap().clear_selection(); }
            }
        });

        assert_eq!(model.select_at(&path(&[0, 0, 1])), Err(SelectionError::SelectionChanged));
        assert!(model.selected_indices().is_empty());
        assert!(!leaves.has_collection_changed_subscribers());
        subscription.dispose();
        if !clear_source {
            model.select_at(&path(&[0, 0, 1])).unwrap();
            assert_eq!(indices(&model), ["R.0.0.1"]);
        }
    }
}

#[test]
fn children_requested_keeps_a_child_realized_by_a_nested_request() {
    let leaves = FerroList::from_items([10, 11]);
    let model = SelectionModel::new();
    model.set_source(Some(ItemsSource::from_values([
        ItemsSource::from(Rc::new(leaves.clone())),
    ])));
    let weak = Rc::downgrade(&model);
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    model.children_requested(move |args| {
        let call = called.get();
        called.set(call + 1);
        args.set_children(
            args.source().unwrap().as_ref().and_then(SelectionModel::source_from_item),
        );
        if call == 0 { weak.upgrade().unwrap().select_with_group(0, 1).unwrap(); }
    });

    model.select_with_group(0, 0).unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(indices(&model), ["R.0.0", "R.0.1"]);
    leaves.insert(0, 9);
    assert_eq!(indices(&model), ["R.0.1", "R.0.2"]);
    model.clear_selection();
    assert!(!leaves.has_collection_changed_subscribers());
}

#[test]
fn child_source_read_rejects_reentrant_tree_reset() {
    struct Source {
        model: std::rc::Weak<SelectionModel>,
        child: ItemsSource,
        reset: Cell<bool>,
    }
    impl IItemsList for Source {
        fn count(&self) -> usize { 1 }
        fn get_at(&self, _: usize) -> Option<BoxedValue> {
            if self.reset.replace(false) { self.model.upgrade().unwrap().clear_selection(); }
            Some(Rc::new(self.child.clone()))
        }
        fn as_any(&self) -> &dyn std::any::Any { self }
    }
    let model = SelectionModel::new();
    let leaves = FerroList::from_items([10, 11]);
    model.set_source(Some(ItemsSource::new(Rc::new(Source {
        model: Rc::downgrade(&model),
        child: Rc::new(leaves.clone()).into(),
        reset: Cell::new(true),
    }))));

    assert_eq!(model.select_with_group(0, 0), Err(SelectionError::SelectionChanged));
    assert!(model.selected_indices().is_empty());
    assert!(!leaves.has_collection_changed_subscribers());
    model.select_with_group(0, 1).unwrap();
    assert_eq!(indices(&model), ["R.0.1"]);
}

#[test]
fn verify_selected_views_fail_after_selection_tree_is_discarded() {
    let (model, _) = nested();
    model.select_with_group(1, 2).unwrap();
    let items = model.selected_items();
    let paths = model.selected_indices();
    assert_eq!(paths.get_at(0).unwrap(), path(&[1, 2]));
    model.set_source(Some(ItemsSource::from_values(0..3)));
    assert!(matches!(
        items.get_at(0),
        Err(SelectionError::SelectionChanged)
    ));
    assert_eq!(paths.get_at(0), Err(SelectionError::SelectionChanged));
    assert!(model.selected_items().is_empty());
}

#[test]
fn verify_insert_inside_selected_range_splits_range() {
    let (model, source) = flat();
    model.set_anchor_index_flat(2).unwrap();
    model.select_range_from_anchor(5).unwrap();
    model.set_anchor_index_flat(5).unwrap();
    let calls = count_events(&model);
    source.insert(4, 100);
    assert_eq!(indices(&model), ["R.2", "R.3", "R.5", "R.6"]);
    assert_eq!(model.anchor_index(), Some(path(&[6])));
    assert_eq!(calls.get(), 1);
}

#[test]
fn verify_adding_item_to_selected_group_makes_ancestors_partial() {
    let (model, groups) = three_levels();
    model.select_all().unwrap();
    let calls = count_events(&model);
    groups[0][1].add(1000);
    assert_eq!(
        model.is_selected_at(&path(&[0, 1, 3])).unwrap(),
        Some(false)
    );
    assert_eq!(model.is_selected_at(&path(&[0, 1, 2])).unwrap(), Some(true));
    assert_eq!(model.is_selected_at(&path(&[0, 1])).unwrap(), None);
    assert_eq!(model.is_selected_at(&path(&[0])).unwrap(), None);
    assert_eq!(model.is_selected_at(&path(&[1])).unwrap(), Some(true));
    assert!(calls.get() > 0);
}

#[test]
fn verify_removing_unselected_item_makes_ancestors_selected() {
    let (model, groups) = three_levels();
    model.select_all().unwrap();
    model.deselect_at(&path(&[0, 1, 2])).unwrap();
    let calls = count_events(&model);
    groups[0][1].remove_at(2);
    assert_eq!(model.is_selected_at(&path(&[0, 1])).unwrap(), Some(true));
    assert_eq!(model.is_selected_at(&path(&[0])).unwrap(), Some(true));
    assert!(calls.get() > 0);
}

#[test]
fn verify_removing_item_under_unselected_ancestors_does_not_raise_selection_changed() {
    let (model, groups) = three_levels();
    model.select_at(&path(&[0, 1, 0])).unwrap();
    model.deselect_at(&path(&[0, 1, 0])).unwrap();
    let calls = count_events(&model);
    groups[0][1].remove_at(2);
    assert_eq!(model.is_selected_at(&path(&[0, 1])).unwrap(), Some(false));
    assert_eq!(model.is_selected_at(&path(&[0])).unwrap(), Some(false));
    assert_eq!(calls.get(), 0);
}

// FerroUI addition: proves flat select-all is lazy for a data-virtualized million-item source.
#[test]
fn select_all_flat_does_not_read_source_items() {
    struct Source {
        reads: Cell<usize>,
    }
    impl IItemsList for Source {
        fn count(&self) -> usize {
            1_000_000
        }
        fn get_at(&self, index: usize) -> Option<BoxedValue> {
            self.reads.set(self.reads.get() + 1);
            Some(Rc::new(index))
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    let source = Rc::new(Source {
        reads: Cell::new(0),
    });
    let model = SelectionModel::new();
    model.set_source(Some(ItemsSource::new(source.clone())));
    model.select_all_flat();
    assert_eq!(model.selected_indices().count(), 1_000_000);
    assert_eq!(source.reads.get(), 0);
    assert_eq!(
        model.selected_indices().get_at(999_999).unwrap(),
        path(&[999_999])
    );
    assert_eq!(source.reads.get(), 0);
    model.selected_items().get_at(999_999).unwrap();
    assert_eq!(source.reads.get(), 1);
}

// FerroUI addition: subscriptions must neither leak the model nor keep discarded child sources alive.
#[test]
fn collection_subscriptions_are_revoked_when_model_is_dropped() {
    let (model, source) = flat();
    let weak = Rc::downgrade(&model);
    assert!(source.has_collection_changed_subscribers());
    drop(model);
    assert!(weak.upgrade().is_none());
    assert!(!source.has_collection_changed_subscribers());
}

// The pinned upstream ignores its desired-once regression; this asserts its current behavior.
#[test]
fn singleton_range_preserves_upstream_double_notification() {
    let (model, _) = flat();
    let calls = count_events(&model);
    model.select_range(&path(&[4]), &path(&[4])).unwrap();
    assert_eq!(calls.get(), 2);
    model.deselect_range(&path(&[4]), &path(&[4])).unwrap();
    assert_eq!(calls.get(), 4);
}

// FerroUI addition: stable views do not hold source data until indexed.
#[test]
fn selection_views_are_shared_and_iterate_in_index_order() {
    let (model, _) = flat();
    model.select(8).unwrap();
    model.select(3).unwrap();
    let view = model.selected_indices();
    assert!(view == model.selected_indices());
    assert_eq!(view.iter().len(), 2);
    assert_eq!(indices(&model), ["R.3", "R.8"]);
    assert_eq!(
        model.selected_item().unwrap().downcast_ref::<i32>(),
        Some(&3)
    );
}

// FerroUI addition: a single selection that a failed select clears is announced.
#[test]
fn a_failed_single_select_announces_the_cleared_selection() {
    let (model, _source) = flat();
    model.set_single_select(true);
    model.select(1).unwrap();
    let raised = Rc::new(Cell::new(0));
    let count = raised.clone();
    let _subscription = model.selection_changed(move |_| count.set(count.get() + 1));
    assert!(model.select(100).is_err());
    assert_eq!(model.selected_index(), None);
    assert_eq!(raised.get(), 1);
}

// FerroUI addition: the selected-items view is an items source a selecting control can
// search, and a stale index has no item instead of a panic.
#[test]
fn the_selected_items_view_answers_index_of_and_stale_reads() {
    let (model, _source) = flat();
    model.select(2).unwrap();
    model.select(5).unwrap();
    let view = model.selected_items();
    let fifth: Option<BoxedValue> = Some(Rc::new(5i32));
    assert_eq!(IItemsList::index_of(&view, &fifth), 1);
    assert_eq!(IItemsList::index_of(&view, &Some(Rc::new(7i32) as BoxedValue)), -1);
    assert!(IItemsList::get_at(&view, 10).is_none());
}
