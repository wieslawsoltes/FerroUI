//! `SourceTests.cs`: the flat and hierarchical sources, their columns, rows, sorting, expansion, moves and
//! selection, without a presentation.

use super::support::{
    array, assert_panics, assert_same, assert_same_items, header, row_model, row_models, same_rc, single, some_items,
    ResettingCollection,
};
use crate::accessor::{Accessor, TreeModel};
use crate::model_collection::{list, observable, ModelCollection, Models};
use crate::models::{
    as_hierarchical_row, ColumnList, ColumnRef, HierarchicalExpanderColumn, HierarchicalRow, IColumn, IRows,
    ListSortDirection, RowEventArgs, RowRef, RowsChangedArgs, TextColumn, ValueColumn,
};
use crate::selection::{IReadOnlyList, TreeSelectionModelBase, TreeSelectionModelSourceResetEventArgs};
use crate::{
    index_path, FlatTreeDataGridSource, HierarchicalTreeDataGridSource, ITreeDataGridSource, IndexPath, RowDropPosition,
    RowEventHandler, RowMoveEffects, TreeDataGridItemsSourceView,
};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model, ModelTypeBuilder};
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn existing_delegate_is_used_directly_for_values_sorting_and_edits() {
    let getter = |x: &Rc<Node>| x.name();
    let column = ValueColumn::<Node, String>::from_delegate(
        header("Name"),
        getter,
        Some("Name"),
        Some(Rc::new(|x: &Rc<Node>, value: String| x.set_name(value))),
        None,
        None,
        None,
    );
    let source = FlatTreeDataGridSource::new(array(vec![Node::new("B"), Node::new("A")]));
    source.columns().add(column.clone() as ColumnRef<Node>);
    // `Assert.Same(getter, column.Getter)`: a closure has no identity to compare, so the column's getter is
    // checked to read what the delegate reads.
    let probe = Node::new("Probe");
    assert_eq!(getter(&probe), (column.accessor().getter())(&probe));
    assert_eq!(Some("Name"), column.property_name());
    // `Assert.Null(column.GetterExpression)`: a delegate has no path below the model to observe.
    assert!(column.accessor().links().is_empty());
    source.sort_by(Some(&*column as &dyn IColumn), ListSortDirection::Ascending);
    let first = row_model::<Node>(&source.rows().get(0));
    assert_eq!("A", first.name());
    column.set_value(&first, "Changed".to_string());
    assert_eq!("Changed", column.get_value(&first));
}

#[test]
fn flat_sort_edits_and_selection_work_without_a_ui_runtime() {
    let items = observable(vec![Node::new("B"), Node::new("A"), Node::new("C")]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);
    let name = editable_name_column("Name");
    source.columns().add(name.clone() as ColumnRef<Node>);
    let selection = flat_selection(&source);
    selection.set_selected_index(IndexPath::new(0));
    assert!(source.sort_by(Some(&*name as &dyn IColumn), ListSortDirection::Ascending));
    assert_eq!(vec!["A", "B", "C"], names(&*source.rows()));
    assert_same(&items.get(0), &selection.selected_item().unwrap());
    let changes = counter();
    {
        let changes = changes.clone();
        source.rows().add_collection_changed(Rc::new(move |_e: &RowsChangedArgs<'_>| bump(&changes)));
    }
    items.insert(0, Node::new("D"));
    assert_eq!(IndexPath::new(1), selection.selected_index());
    assert_eq!(1, changes.get());
    name.set_value(&items.get(1), "Z".to_string());
    source.sort_by(Some(&*name as &dyn IColumn), ListSortDirection::Ascending);
    let rows = source.rows();
    assert_eq!("Z", row_model::<Node>(&rows.get(rows.count() - 1)).name());
}

#[test]
fn flat_clear_sort_restores_source_order_and_column_state() {
    let items = observable(vec![Node::new("B"), Node::new("A"), Node::new("C")]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);
    let column = name_column("Name");
    source.columns().add(column.clone() as ColumnRef<Node>);
    source.sort_by(Some(&*column as &dyn IColumn), ListSortDirection::Ascending);

    ITreeDataGridSource::clear_sort(&*source);

    assert!(!source.is_sorted());
    assert_eq!(None, column.sort_direction());
    assert_eq!(vec!["B", "A", "C"], names(&*source.rows()));
}

#[test]
fn flat_move_preserves_selected_rows() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let third = Node::new("Third");
    let items = observable(vec![first.clone(), second.clone(), third.clone()]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);
    let selection = flat_selection(&source);
    selection.set_single_select(false);
    selection.select(&IndexPath::new(0));
    selection.select(&IndexPath::new(1));

    source.move_rows(
        &*source,
        &[IndexPath::new(0), IndexPath::new(1)],
        &IndexPath::new(2),
        RowDropPosition::After,
        RowMoveEffects::MOVE,
    );

    assert_same_items(&[third, first.clone(), second.clone()], &items.to_vec());
    assert_same_items(&[first, second], &some_items(selection.selected_items().to_vec()));
    assert_eq!(vec![IndexPath::new(1), IndexPath::new(2)], selection.selected_indexes().to_vec());
}

#[test]
fn flat_move_tracks_selected_duplicate_occurrences() {
    let shared = Node::new("Shared");
    let other = Node::new("Other");
    let items = observable(vec![shared.clone(), shared.clone(), other.clone()]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);
    let selection = flat_selection(&source);
    selection.set_selected_index(IndexPath::new(1));

    source.move_rows(&*source, &[IndexPath::new(1)], &IndexPath::new(2), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_same_items(&[shared.clone(), other, shared], &items.to_vec());
    assert_eq!(IndexPath::new(2), selection.selected_index());
}

#[test]
fn flat_move_rejects_duplicate_source_indexes_before_mutation() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);

    assert_panics(|| {
        source.move_rows(
            &*source,
            &[IndexPath::new(0), IndexPath::new(0)],
            &IndexPath::new(1),
            RowDropPosition::After,
            RowMoveEffects::MOVE,
        )
    });

    assert_same_items(&[first, second], &items.to_vec());
}

#[test]
fn flat_move_rejects_an_invalid_target_before_mutation() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);

    assert_panics(|| {
        source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(5), RowDropPosition::Before, RowMoveEffects::MOVE)
    });

    assert_same_items(&[first, second], &items.to_vec());
}

#[test]
fn flat_move_rejects_an_invalid_source_before_mutation() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);

    assert_panics(|| {
        source.move_rows(&*source, &[IndexPath::new(5)], &IndexPath::new(1), RowDropPosition::Before, RowMoveEffects::MOVE)
    });

    assert_same_items(&[first, second], &items.to_vec());
}

#[test]
fn flat_move_rejects_a_read_only_list_before_mutation() {
    let items = array(vec![Node::new("First"), Node::new("Second")]);
    let source = FlatTreeDataGridSource::new(items.clone());

    assert_panics(|| {
        source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE)
    });

    assert_eq!(vec!["First", "Second"], model_names(&items));
}

#[test]
fn flat_move_none_is_a_no_op_for_a_read_only_list() {
    let items = array(vec![Node::new("First"), Node::new("Second")]);
    let source = FlatTreeDataGridSource::new(items.clone());

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::None, RowMoveEffects::MOVE);

    assert_eq!(vec!["First", "Second"], model_names(&items));
}

#[test]
fn flat_move_refreshes_materialized_rows_for_a_plain_list() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let items = list(vec![first.clone(), second.clone()]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);
    assert_same_items(&[first.clone(), second.clone()], &row_models::<Node>(&*source.rows()));

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_same_items(&[second.clone(), first.clone()], &items.borrow());
    assert_same_items(&[second, first], &row_models::<Node>(&*source.rows()));
}

#[test]
fn flat_move_preserves_selection_anchors() {
    // [InlineData(false)], [InlineData(true)]
    for notifying in [false, true] {
        let first = Node::new("First");
        let second = Node::new("Second");
        let items: Models<Node> = if notifying {
            observable(vec![first.clone(), second.clone()]) as Models<Node>
        } else {
            list(vec![first.clone(), second.clone()]) as Models<Node>
        };
        let source = FlatTreeDataGridSource::new(items);
        let selection = flat_selection(&source);
        selection.set_selected_index(IndexPath::new(0));

        source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);

        assert_eq!(IndexPath::new(1), selection.anchor_index(), "notifying: {notifying}");
        assert_eq!(IndexPath::new(1), selection.range_anchor_index(), "notifying: {notifying}");
    }
}

#[test]
fn flat_move_preserves_empty_selection_anchors() {
    let items = observable(vec![Node::new("First"), Node::new("Second")]);
    let source = FlatTreeDataGridSource::new(items as Models<Node>);
    let selection = flat_selection(&source);
    selection.set_single_select(false);
    selection.set_selected_index(IndexPath::new(0));
    selection.select(&IndexPath::new(1));
    selection.set_anchor_index(IndexPath::UNSELECTED);
    selection.set_range_anchor_index(IndexPath::UNSELECTED);
    assert_eq!(IndexPath::UNSELECTED, selection.anchor_index());
    assert_eq!(IndexPath::UNSELECTED, selection.range_anchor_index());

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_eq!(IndexPath::UNSELECTED, selection.anchor_index());
    assert_eq!(IndexPath::UNSELECTED, selection.range_anchor_index());
}

#[test]
fn hierarchy_expansion_sorting_and_incremental_updates_share_model_indexes() {
    let parent = Node::with_children("Parent", vec![Node::new("B"), Node::new("A")]);
    let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
    let column = node_expander(name_column("Name"));
    source.columns().add(column.clone() as ColumnRef<Node>);
    assert_eq!(1, source.rows().count());
    source.expand(&IndexPath::new(0));
    assert_eq!(3, source.rows().count());
    let selection = hierarchical_selection(&source);
    selection.set_selected_index(index_path![0, 0]);
    source.sort_by(Some(&*column as &dyn IColumn), ListSortDirection::Ascending);
    assert_eq!(index_path![0, 1], source.rows().row_index_to_model_index(1));
    parent.children().insert(0, Node::new("C"));
    assert_eq!(index_path![0, 1], selection.selected_index());
    assert_eq!(4, source.rows().count());
    source.collapse(&IndexPath::new(0));
    assert_eq!(1, source.rows().count());
    assert_eq!("B", selection.selected_item().unwrap().name());
    source.expand(&IndexPath::new(0));
    assert_eq!(4, source.rows().count());
}

#[test]
fn hierarchical_items_replacement_raises_property_changed() {
    let source = HierarchicalTreeDataGridSource::new(array(vec![Node::new("Old")]));
    let property_name: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    {
        let property_name = property_name.clone();
        source.property_changed().add(Rc::new(move |name: &str| *property_name.borrow_mut() = Some(name.to_string())));
    }

    source.set_items(array(vec![Node::new("New")]));

    assert_eq!(Some("Items".to_string()), *property_name.borrow());
}

#[test]
fn unequal_range_replace_resynchronizes_unsorted_row_projection() {
    // [InlineData(1)], [InlineData(3)]
    for replacement_count in [1, 3] {
        let children = ResettingCollection::new(vec![
            SharedNode::new("Old 1", None),
            SharedNode::new("Old 2", None),
            SharedNode::new("Tail", None),
        ]);
        let source = HierarchicalTreeDataGridSource::new(array(vec![SharedNode::new(
            "Parent",
            Some(children.clone() as Models<SharedNode>),
        )]));
        add_shared_expander(&source);
        source.expand(&IndexPath::new(0));
        let changes: Rc<RefCell<Vec<NotifyCollectionChangedAction>>> = Rc::new(RefCell::new(Vec::new()));
        {
            let changes = changes.clone();
            source.rows().add_collection_changed(Rc::new(move |e: &RowsChangedArgs<'_>| changes.borrow_mut().push(e.action)));
        }

        let replacements: Vec<Rc<SharedNode>> =
            (1..=replacement_count).map(|x| SharedNode::new(&format!("New {x}"), None)).collect();
        children.replace_range(0, 2, replacements.clone());

        let mut expected = vec!["Parent".to_string()];
        expected.extend(replacements.iter().map(|x| x.name.clone()));
        expected.push("Tail".to_string());
        assert_eq!(expected, shared_names(&*source.rows()), "replacement count: {replacement_count}");
        assert_eq!(
            vec![NotifyCollectionChangedAction::Remove, NotifyCollectionChangedAction::Add],
            *changes.borrow(),
            "replacement count: {replacement_count}"
        );
    }
}

#[test]
fn bound_expansion_is_initialized_and_changes_without_a_view() {
    let parent = BoundNode::create(true, vec![BoundNode::new()]);
    let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
    source.columns().add(bound_expander(true));
    assert_eq!(2, source.rows().count());
    parent.set_is_expanded(false);
    assert_eq!(1, source.rows().count());
    source.expand(&IndexPath::new(0));
    assert!(parent.is_expanded());
    assert_eq!(2, source.rows().count());
}

#[test]
fn expanded_row_rebinds_when_its_child_collection_is_replaced() {
    // [InlineData(false)], [InlineData(true)]
    for bound_expansion in [false, true] {
        let old_children = observable(vec![BoundNode::new()]);
        let new_children = observable(vec![BoundNode::new(), BoundNode::new()]);
        let parent = BoundNode::with_children(true, old_children.clone());
        let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
        source.columns().add(bound_expander(bound_expansion));
        if !bound_expansion {
            source.expand(&IndexPath::new(0));
        }
        assert_eq!(2, source.rows().count(), "bound expansion: {bound_expansion}");
        let lifecycle_events = count_lifecycle_events(&source);

        parent.set_children(new_children.clone());

        assert_eq!(3, source.rows().count(), "bound expansion: {bound_expansion}");
        assert_eq!(0, lifecycle_events.get(), "bound expansion: {bound_expansion}");
        assert_same(&new_children.get(0), &row_model::<BoundNode>(&source.rows().get(1)));
        old_children.add(BoundNode::new());
        assert_eq!(3, source.rows().count(), "bound expansion: {bound_expansion}");
        new_children.add(BoundNode::new());
        assert_eq!(4, source.rows().count(), "bound expansion: {bound_expansion}");
    }
}

#[test]
fn fresh_child_enumerable_wrapper_does_not_reset_rows_or_selection() {
    let child = BoundNode::new();
    let parent = BoundNode::create(false, vec![child.clone()]);
    let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
    // `x => x.Children.Where(_ => true)`: a fresh sequence that does not notify, on every call.
    source.columns().add(HierarchicalExpanderColumn::new(
        expanded_text_column(),
        |x: &Rc<BoundNode>| Some(Rc::new(x.children().to_vec()) as Models<BoundNode>),
        None,
        None,
        None,
    ) as ColumnRef<BoundNode>);
    source.expand(&IndexPath::new(0));
    let selection = hierarchical_selection(&source);
    selection.set_selected_index(index_path![0, 0]);
    let child_row = source.rows().get(1);
    let row_changes = counter();
    {
        let row_changes = row_changes.clone();
        source.rows().add_collection_changed(Rc::new(move |_e: &RowsChangedArgs<'_>| bump(&row_changes)));
    }

    parent.notify_unrelated_property_changed();

    assert_eq!(0, row_changes.get());
    assert_same(&child_row, &source.rows().get(1));
    assert_eq!(index_path![0, 0], selection.selected_index());
    assert_same(&child, &selection.selected_item().unwrap());
}

#[test]
fn empty_rebound_child_collection_restores_expander_visibility_when_populated() {
    let parent = BoundNode::with_children(true, observable(vec![BoundNode::new()]));
    let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
    source.columns().add(bound_expander(true));
    let row = hierarchical_row::<BoundNode>(&source.rows().get(0));
    let replacement = observable(Vec::<Rc<BoundNode>>::new());

    parent.set_children(replacement.clone());

    assert!(!row.is_expanded());
    assert!(!row.show_expander());
    replacement.add(BoundNode::new());
    assert!(row.show_expander());
    assert_eq!(1, source.rows().count());
}

#[test]
fn empty_rebound_child_collection_observes_a_later_collection_replacement() {
    let parent = BoundNode::with_children(false, observable(vec![BoundNode::new()]));
    let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
    source.columns().add(bound_expander(false));
    source.expand(&IndexPath::new(0));
    let row = hierarchical_row::<BoundNode>(&source.rows().get(0));

    parent.set_children(observable(Vec::new()));
    assert!(!row.show_expander());

    parent.set_children(observable(vec![BoundNode::new()]));

    assert!(row.show_expander());
    assert!(row.is_expanded());
    assert_eq!(2, source.rows().count());
}

#[test]
fn failed_expansion_observes_empty_children_without_lifecycle_events() {
    let parent = BoundNode::new();
    let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
    source.columns().add(HierarchicalExpanderColumn::new(
        expanded_text_column(),
        |x: &Rc<BoundNode>| Some(x.children() as Models<BoundNode>),
        Some(Accessor::new(|_x: &Rc<BoundNode>| true)),
        None,
        None,
    ) as ColumnRef<BoundNode>);
    let row = hierarchical_row::<BoundNode>(&source.rows().get(0));
    let lifecycle_events = count_lifecycle_events(&source);

    source.expand(&IndexPath::new(0));

    assert!(!row.is_expanded());
    assert!(!row.show_expander());
    assert_eq!(0, lifecycle_events.get());
    parent.children().add(BoundNode::new());
    assert!(row.show_expander());
    assert!(!row.is_expanded());
    assert_eq!(1, source.rows().count());
}

#[test]
fn replacing_a_child_collection_resets_its_selection_subtree() {
    let old_child = BoundNode::new();
    let new_child = BoundNode::new();
    let old_children = observable(vec![old_child]);
    let parent = BoundNode::with_children(false, old_children.clone());
    let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
    source.columns().add(bound_expander(false));
    source.expand(&IndexPath::new(0));
    let selection = hierarchical_selection(&source);
    selection.set_selected_index(index_path![0, 0]);
    let resets = counter();
    {
        let resets = resets.clone();
        selection.add_source_reset(Rc::new(move |e: &TreeSelectionModelSourceResetEventArgs| {
            assert_eq!(IndexPath::new(0), *e.parent_index());
            bump(&resets);
        }));
    }

    parent.set_children(observable(vec![new_child.clone()]));

    assert_eq!(1, resets.get());
    assert_eq!(IndexPath::UNSELECTED, selection.selected_index());
    assert!(selection.selected_item().is_none());
    assert!(selection.selected_indexes().to_vec().is_empty());
    assert_same(&new_child, &row_model::<BoundNode>(&source.rows().get(1)));
    old_children.add(BoundNode::new());
    assert_eq!(2, source.rows().count());
    assert_eq!(1, resets.get());
    assert!(selection.selected_indexes().to_vec().is_empty());
}

#[test]
fn replacing_a_parent_item_clears_its_descendant_selection_subtree() {
    let old_child = BoundNode::new();
    let new_child = BoundNode::new();
    let old_parent = BoundNode::create(true, vec![old_child]);
    let new_parent = BoundNode::create(true, vec![new_child.clone()]);
    let items = observable(vec![old_parent.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<BoundNode>);
    source.columns().add(bound_expander(true));
    let selection = hierarchical_selection(&source);
    selection.set_selected_index(index_path![0, 0]);

    items.set(0, new_parent.clone());

    assert_eq!(IndexPath::UNSELECTED, selection.selected_index());
    assert!(selection.selected_item().is_none());
    assert!(selection.selected_indexes().to_vec().is_empty());
    assert_same(&new_parent, &row_model::<BoundNode>(&source.rows().get(0)));
    assert_same(&new_child, &row_model::<BoundNode>(&source.rows().get(1)));
    old_parent.children().add(BoundNode::new());
    assert_eq!(2, source.rows().count());
}

#[test]
fn unequal_parent_replace_shifts_descendant_selection_and_anchors() {
    let selected_child = BoundNode::new();
    let selected_parent = BoundNode::create(true, vec![selected_child.clone()]);
    let items = ResettingCollection::new(vec![BoundNode::new(), selected_parent.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<BoundNode>);
    source.columns().add(bound_expander(true));
    let selection = hierarchical_selection(&source);
    selection.set_selected_index(index_path![1, 0]);

    items.replace_range(0, 1, vec![BoundNode::new(), BoundNode::new()]);

    assert_eq!(index_path![2, 0], selection.selected_index());
    assert_eq!(index_path![2, 0], selection.anchor_index());
    assert_eq!(index_path![2, 0], selection.range_anchor_index());
    assert_same(&selected_child, &selection.selected_item().unwrap());
    let parent_rows: Vec<Rc<BoundNode>> = row_models::<BoundNode>(&*source.rows())
        .into_iter()
        .filter(|x| same_rc(x, &selected_parent))
        .collect();
    assert_same(&selected_parent, &single(parent_rows));
}

#[test]
fn unequal_replace_clears_anchors_inside_the_replaced_range() {
    let items = ResettingCollection::new(vec![Node::new("Old 1"), Node::new("Old 2"), Node::new("Tail")]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);
    source.columns().add(name_column("Name") as ColumnRef<Node>);
    let selection = flat_selection(&source);
    selection.set_anchor_index(IndexPath::new(1));
    selection.set_range_anchor_index(IndexPath::new(1));

    items.replace_range(0, 2, vec![Node::new("Replacement")]);

    assert_eq!(IndexPath::UNSELECTED, selection.anchor_index());
    assert_eq!(IndexPath::UNSELECTED, selection.range_anchor_index());
}

#[test]
fn equal_size_replace_clears_anchors_inside_the_replaced_range() {
    let items = ResettingCollection::new(vec![Node::new("Old 1"), Node::new("Old 2"), Node::new("Tail")]);
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);
    source.columns().add(name_column("Name") as ColumnRef<Node>);
    let selection = flat_selection(&source);
    selection.set_anchor_index(IndexPath::new(1));
    selection.set_range_anchor_index(IndexPath::new(1));

    items.replace_range(0, 2, vec![Node::new("Replacement 1"), Node::new("Replacement 2")]);

    assert_eq!(IndexPath::UNSELECTED, selection.anchor_index());
    assert_eq!(IndexPath::UNSELECTED, selection.range_anchor_index());
}

#[test]
fn indexless_collection_changes_reset_rows_and_selection() {
    verify(|items, replacement| items.add_without_index(replacement), &["A", "B", "C"]);
    verify(|items, _| items.remove_at_without_index(0), &["B"]);
    verify(|items, replacement| items.replace_without_index(0, replacement), &["C", "B"]);

    fn verify(change: impl Fn(&ResettingCollection<Node>, Rc<Node>), expected: &[&str]) {
        let items = ResettingCollection::new(vec![Node::new("A"), Node::new("B")]);
        let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);
        source.columns().add(name_column("Name") as ColumnRef<Node>);
        assert_eq!(vec!["A", "B"], names(&*source.rows()));
        let selection = flat_selection(&source);
        selection.set_selected_index(IndexPath::new(1));

        change(&items, Node::new("C"));

        assert_eq!(expected.to_vec(), names(&*source.rows()));
        assert_eq!(IndexPath::UNSELECTED, selection.selected_index());
        assert!(selection.selected_item().is_none());
    }
}

#[test]
fn moving_the_expander_column_preserves_rows_and_expansion() {
    let source = HierarchicalTreeDataGridSource::new(array(vec![Node::with_children("Root", vec![Node::new("Child")])]));
    let expander = node_expander(name_column("Name"));
    source.columns().add(expander.clone() as ColumnRef<Node>);
    source.columns().add(name_column("Other") as ColumnRef<Node>);
    source.expand(&IndexPath::new(0));
    let rows = source.rows();
    source.columns().move_column(0, 1);
    assert_same(&expander, &source.columns().get(1));
    assert_same(&rows, &source.rows());
    assert_eq!(2, rows.count());
}

#[test]
fn items_source_view_can_be_disposed_without_a_registered_listener() {
    let items = observable(Vec::<Rc<Node>>::new());
    let view = TreeDataGridItemsSourceView::new(items as Models<Node>);
    // The reference removes a handler that was never added: here, a token that was never handed out.
    view.remove_collection_changed(u64::MAX);
    view.dispose();
    view.dispose();
}

#[test]
fn replacing_a_hierarchical_item_disposes_the_replaced_row() {
    let replaced = BoundNode::create(false, vec![BoundNode::new()]);
    let items = observable(vec![replaced.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<BoundNode>);
    source.columns().add(bound_expander(true));
    let replaced_row = hierarchical_row::<BoundNode>(&source.rows().get(0));

    items.set(0, BoundNode::new());
    replaced.set_is_expanded(true);

    assert!(!replaced_row.is_expanded());
}

#[test]
fn hierarchical_move_with_none_position_is_a_no_op() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    add_node_expander(&source);

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::None, RowMoveEffects::MOVE);

    assert_same(&first, &items.get(0));
    assert_same(&second, &items.get(1));
}

#[test]
fn column_replacement_reports_new_and_old_items_in_the_correct_fields() {
    let columns = ColumnList::<Node>::new();
    let original = name_column("Original");
    let replacement = name_column("Replacement");
    let change: Rc<RefCell<Option<(Vec<ColumnRef<Node>>, Vec<ColumnRef<Node>>)>>> = Rc::new(RefCell::new(None));
    columns.add(original.clone() as ColumnRef<Node>);
    {
        let change = change.clone();
        columns.list().add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, ColumnRef<Node>>| {
            *change.borrow_mut() = Some((e.new_items.to_vec(), e.old_items.to_vec()));
        }));
    }

    columns.set(0, replacement.clone() as ColumnRef<Node>);

    let (new_items, old_items) = change.borrow().clone().expect("a change of the columns");
    assert_same(&replacement, &new_items[0]);
    assert_same(&original, &old_items[0]);
}

#[test]
fn column_list_rejects_duplicate_instances_before_mutation() {
    let columns = ColumnList::<Node>::new();
    let column = name_column("Name") as ColumnRef<Node>;
    columns.add(column.clone());
    let changes = counter();
    {
        let changes = changes.clone();
        columns
            .list()
            .add_collection_changed(Rc::new(move |_e: &NotifyCollectionChangedEventArgs<'_, ColumnRef<Node>>| bump(&changes)));
    }

    assert_panics(|| columns.add(column.clone()));
    assert_panics(|| columns.insert_range(1, |add| add(column.clone())));
    assert_panics(|| columns.reset(|items| items.push(column.clone())));

    assert_eq!(1, columns.count());
    assert_same(&column, &columns.get(0));
    assert_eq!(0, changes.get());
}

#[test]
fn replacing_the_expander_column_is_rejected_before_mutation() {
    let source = HierarchicalTreeDataGridSource::new(array(vec![Node::new("Root")]));
    let expander = node_expander(name_column("Name"));
    source.columns().add(expander.clone() as ColumnRef<Node>);
    let rows = source.rows();

    assert_panics(|| source.columns().set(0, name_column("Other") as ColumnRef<Node>));

    assert_same(&expander, &source.columns().get(0));
    assert_same(&rows, &source.rows());
}

#[test]
fn adding_or_removing_an_expander_is_rejected_before_mutation() {
    let source = HierarchicalTreeDataGridSource::new(array(vec![Node::new("Root")]));
    let expander = node_expander(name_column("Name"));
    source.columns().add(expander.clone() as ColumnRef<Node>);

    assert_panics(|| source.columns().add(node_expander(name_column("Other")) as ColumnRef<Node>));
    assert_eq!(1, source.columns().count());
    assert_panics(|| {
        source.columns().remove(&(expander.clone() as ColumnRef<Node>));
    });
    assert_eq!(1, source.columns().count());
    assert_same(&expander, &source.columns().get(0));
}

#[test]
fn column_reset_is_staged_and_retains_the_expander() {
    let source = HierarchicalTreeDataGridSource::new(array(vec![Node::new("Root")]));
    let expander = node_expander(name_column("Name"));
    let original = name_column("Original");
    let replacement = name_column("Replacement");
    source.columns().add(expander.clone() as ColumnRef<Node>);
    source.columns().add(original.clone() as ColumnRef<Node>);

    source.columns().reset(|columns| {
        columns.retain(|c| !same_rc(c, &original));
        columns.push(replacement.clone() as ColumnRef<Node>);
    });

    let expected = [expander.clone() as ColumnRef<Node>, replacement.clone() as ColumnRef<Node>];
    assert_same_items(&expected, &source.columns().to_vec());
    assert_panics(|| source.columns().reset(|columns| columns.retain(|c| !same_rc(c, &expander))));
    assert_same_items(&expected, &source.columns().to_vec());
}

#[test]
fn batch_column_changes_validate_the_expander_before_mutation() {
    let source = HierarchicalTreeDataGridSource::new(array(vec![Node::new("Root")]));
    let expander = node_expander(name_column("Name"));
    source.columns().add(expander.clone() as ColumnRef<Node>);

    assert_panics(|| {
        source.columns().insert_range(1, |add| {
            add(name_column("Ordinary") as ColumnRef<Node>);
            add(node_expander(name_column("Other")) as ColumnRef<Node>);
        })
    });
    assert_eq!(1, source.columns().count());
    assert_same(&expander, &source.columns().get(0));
    assert_panics(|| source.columns().remove_range(0, 1));
    assert_eq!(1, source.columns().count());
    assert_same(&expander, &source.columns().get(0));
}

#[test]
fn hierarchical_move_preserves_order_across_parent_groups() {
    let first = Node::with_children("First", vec![Node::new("A")]);
    let second = Node::with_children("Second", vec![Node::new("B")]);
    let target = Node::new("Target");
    let items = observable(vec![first, second, target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<Node>);
    add_node_expander(&source);

    source.move_rows(
        &*source,
        &[index_path![0, 0], index_path![1, 0]],
        &IndexPath::new(2),
        RowDropPosition::Inside,
        RowMoveEffects::MOVE,
    );

    assert_eq!(vec!["A", "B"], target.children().to_vec().iter().map(|x| x.name()).collect::<Vec<_>>());
}

#[test]
fn hierarchical_move_uses_the_original_after_target_offset() {
    let first = Node::new("First");
    let middle = Node::new("Middle");
    let target = Node::new("Target");
    let items = observable(vec![first.clone(), middle.clone(), target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);

    source.move_rows(
        &*source,
        &[IndexPath::new(0), IndexPath::new(2)],
        &IndexPath::new(2),
        RowDropPosition::After,
        RowMoveEffects::MOVE,
    );

    assert_same_items(&[middle, first, target], &items.to_vec());
}

#[test]
fn hierarchical_move_resolves_all_source_paths_before_removal() {
    let first = Node::new("First");
    let moved_child = Node::new("Child");
    let second = Node::with_children("Second", vec![moved_child.clone()]);
    let target = Node::new("Target");
    let items = observable(vec![first.clone(), second.clone(), target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    add_node_expander(&source);

    source.move_rows(
        &*source,
        &[IndexPath::new(0), index_path![1, 0]],
        &IndexPath::new(2),
        RowDropPosition::Inside,
        RowMoveEffects::MOVE,
    );

    assert_same_items(&[second, target.clone()], &items.to_vec());
    assert_same_items(&[first, moved_child], &target.children().to_vec());
}

#[test]
fn hierarchical_move_preserves_selected_rows() {
    let first = Node::with_children("First", vec![Node::new("Child")]);
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    add_node_expander(&source);
    let selection = hierarchical_selection(&source);
    selection.set_single_select(false);
    selection.select(&IndexPath::new(0));
    selection.select(&index_path![0, 0]);

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_same_items(&[second, first], &items.to_vec());
    assert_eq!(vec![IndexPath::new(1), index_path![1, 0]], selection.selected_indexes().to_vec());
}

#[test]
fn hierarchical_move_preserves_the_primary_selection() {
    let first = Node::new("First");
    let primary = Node::new("Primary");
    let target = Node::new("Target");
    let items = observable(vec![first.clone(), primary.clone(), target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    add_node_expander(&source);
    let selection = hierarchical_selection(&source);
    selection.set_single_select(false);
    selection.select(&IndexPath::new(1));
    selection.select(&IndexPath::new(0));

    source.move_rows(
        &*source,
        &[IndexPath::new(0), IndexPath::new(1)],
        &IndexPath::new(2),
        RowDropPosition::After,
        RowMoveEffects::MOVE,
    );

    assert_same_items(&[target, first, primary.clone()], &items.to_vec());
    assert_eq!(IndexPath::new(2), selection.selected_index());
    assert_same(&primary, &selection.selected_item().unwrap());
    assert_eq!(vec![IndexPath::new(1), IndexPath::new(2)], selection.selected_indexes().to_vec());
}

#[test]
fn hierarchical_move_on_plain_list_preserves_moved_primary_and_retained_selection() {
    let primary = Node::new("Primary");
    let retained = Node::new("Retained");
    let target = Node::new("Target");
    let items = list(vec![primary.clone(), retained.clone(), target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    add_node_expander(&source);
    let selection = hierarchical_selection(&source);
    selection.set_single_select(false);
    selection.select(&IndexPath::new(0));
    selection.select(&IndexPath::new(1));

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(2), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_same_items(&[retained, target, primary.clone()], &items.borrow());
    assert_eq!(IndexPath::new(2), selection.selected_index());
    assert_same(&primary, &selection.selected_item().unwrap());
    assert_eq!(vec![IndexPath::new(0), IndexPath::new(2)], selection.selected_indexes().to_vec());
}

#[test]
fn hierarchical_move_on_plain_list_remaps_an_unmoved_selection() {
    let moved = Node::new("Moved");
    let selected = Node::new("Selected");
    let target = Node::new("Target");
    let items = list(vec![moved.clone(), selected.clone(), target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    let selection = hierarchical_selection(&source);
    selection.set_selected_index(IndexPath::new(1));

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(2), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_same_items(&[selected.clone(), target, moved], &items.borrow());
    assert_eq!(IndexPath::new(0), selection.selected_index());
    assert_same(&selected, &selection.selected_item().unwrap());
}

#[test]
fn hierarchical_move_prefers_exact_selected_source_paths() {
    let child = Node::new("Child");
    let parent = Node::with_children("Parent", vec![child.clone()]);
    let target = Node::new("Target");
    let items = observable(vec![parent.clone(), target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<Node>);
    add_node_expander(&source);
    let selection = hierarchical_selection(&source);
    selection.set_single_select(false);
    selection.select(&IndexPath::new(0));
    selection.select(&index_path![0, 0]);

    source.move_rows(
        &*source,
        &[IndexPath::new(0), index_path![0, 0]],
        &IndexPath::new(1),
        RowDropPosition::Inside,
        RowMoveEffects::MOVE,
    );

    assert_same_items(&[parent, child], &target.children().to_vec());
    assert_eq!(vec![index_path![0, 0], index_path![0, 1]], selection.selected_indexes().to_vec());
}

#[test]
fn hierarchical_move_maps_selections_through_the_deepest_moved_ancestor() {
    let grandchild = Node::new("Grandchild");
    let extracted = Node::with_children("Extracted", vec![grandchild.clone()]);
    let retained = Node::new("Retained");
    let parent = Node::with_children("Parent", vec![extracted.clone(), retained.clone()]);
    let target = Node::new("Target");
    let items = observable(vec![parent.clone(), target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<Node>);
    add_node_expander(&source);
    let selection = hierarchical_selection(&source);
    selection.set_single_select(false);
    selection.select(&index_path![0, 0, 0]);
    selection.select(&index_path![0, 1]);

    source.move_rows(
        &*source,
        &[IndexPath::new(0), index_path![0, 0]],
        &IndexPath::new(1),
        RowDropPosition::Inside,
        RowMoveEffects::MOVE,
    );

    assert_same_items(&[parent.clone(), extracted], &target.children().to_vec());
    assert_same(&retained, &single(parent.children().to_vec()));
    assert_eq!(vec![index_path![0, 0, 0], index_path![0, 1, 0]], selection.selected_indexes().to_vec());
    assert_same_items(&[retained, grandchild], &some_items(selection.selected_items().to_vec()));
}

#[test]
fn hierarchical_move_rejects_targets_in_the_moved_subtree() {
    // [InlineData(RowDropPosition.Inside, 0)], [InlineData(RowDropPosition.Before, 1)],
    // [InlineData(RowDropPosition.After, 1)]
    for (position, target_depth) in [(RowDropPosition::Inside, 0), (RowDropPosition::Before, 1), (RowDropPosition::After, 1)] {
        let child = Node::new("Child");
        let parent = Node::with_children("Parent", vec![child.clone()]);
        let items = observable(vec![parent.clone()]);
        let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
        add_node_expander(&source);
        let target = if target_depth == 0 { IndexPath::new(0) } else { index_path![0, 0] };

        assert_panics(|| source.move_rows(&*source, &[IndexPath::new(0)], &target, position, RowMoveEffects::MOVE));

        assert_same(&parent, &single(items.to_vec()));
        assert_same(&child, &single(parent.children().to_vec()));
    }
}

#[test]
fn hierarchical_move_rejects_duplicate_source_paths_before_mutation() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);

    assert_panics(|| {
        source.move_rows(
            &*source,
            &[IndexPath::new(0), IndexPath::new(0)],
            &IndexPath::new(1),
            RowDropPosition::After,
            RowMoveEffects::MOVE,
        )
    });

    assert_same_items(&[first, second], &items.to_vec());
}

#[test]
fn empty_moves_do_not_rebuild_materialized_plain_list_rows() {
    let flat_items = list(vec![Node::new("Flat")]);
    let flat = FlatTreeDataGridSource::new(flat_items as Models<Node>);
    let flat_row = flat.rows().get(0);
    let flat_changes = counter();
    {
        let flat_changes = flat_changes.clone();
        flat.rows().add_collection_changed(Rc::new(move |_e: &RowsChangedArgs<'_>| bump(&flat_changes)));
    }

    flat.move_rows(&*flat, &[], &IndexPath::new(0), RowDropPosition::Before, RowMoveEffects::MOVE);

    assert_same(&flat_row, &flat.rows().get(0));
    assert_eq!(0, flat_changes.get());

    let hierarchical_items = list(vec![Node::new("Hierarchical")]);
    let hierarchical = HierarchicalTreeDataGridSource::new(hierarchical_items as Models<Node>);
    add_node_expander(&hierarchical);
    let hierarchical_row = hierarchical.rows().get(0);
    let hierarchical_changes = counter();
    {
        let hierarchical_changes = hierarchical_changes.clone();
        hierarchical.rows().add_collection_changed(Rc::new(move |_e: &RowsChangedArgs<'_>| bump(&hierarchical_changes)));
    }

    hierarchical.move_rows(&*hierarchical, &[], &IndexPath::new(0), RowDropPosition::Before, RowMoveEffects::MOVE);

    assert_same(&hierarchical_row, &hierarchical.rows().get(0));
    assert_eq!(0, hierarchical_changes.get());
}

#[test]
fn hierarchical_move_rejects_an_invalid_target_before_mutation() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);

    assert_panics(|| {
        source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(5), RowDropPosition::Before, RowMoveEffects::MOVE)
    });

    assert_same_items(&[first, second], &items.to_vec());
}

#[test]
fn hierarchical_move_rejects_an_invalid_source_before_mutation() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);

    assert_panics(|| {
        source.move_rows(&*source, &[IndexPath::new(5)], &IndexPath::new(1), RowDropPosition::Before, RowMoveEffects::MOVE)
    });

    assert_same_items(&[first, second], &items.to_vec());
}

#[test]
fn hierarchical_move_uses_the_requested_path_for_a_shared_target_collection() {
    let shared_children = observable(Vec::<Rc<SharedNode>>::new());
    let moved = SharedNode::new("Moved", None);
    let first_parent = SharedNode::new("First", Some(shared_children.clone() as Models<SharedNode>));
    let second_parent = SharedNode::new("Second", Some(shared_children.clone() as Models<SharedNode>));
    let items = observable(vec![moved.clone(), first_parent, second_parent]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<SharedNode>);
    add_shared_expander(&source);
    let selection = hierarchical_selection(&source);
    selection.set_selected_index(IndexPath::new(0));

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(2), RowDropPosition::Inside, RowMoveEffects::MOVE);

    assert_same(&moved, &single(shared_children.to_vec()));
    assert_eq!(index_path![1, 0], selection.selected_index());
}

#[test]
fn hierarchical_move_groups_aliased_source_collections_by_identity() {
    let first = SharedNode::new("A", None);
    let second = SharedNode::new("B", None);
    let third = SharedNode::new("C", None);
    let shared_children = observable(vec![first.clone(), second.clone(), third.clone()]);
    let first_parent = SharedNode::new("First", Some(shared_children.clone() as Models<SharedNode>));
    let second_parent = SharedNode::new("Second", Some(shared_children.clone() as Models<SharedNode>));
    let target = SharedNode::new("Target", None);
    let items = observable(vec![first_parent, second_parent, target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<SharedNode>);
    add_shared_expander(&source);

    source.move_rows(
        &*source,
        &[index_path![0, 0], index_path![1, 1]],
        &IndexPath::new(2),
        RowDropPosition::Inside,
        RowMoveEffects::MOVE,
    );

    assert_same_items(&[third], &shared_children.to_vec());
    assert_same_items(&[first, second], &target.children.to_vec());
}

#[test]
fn hierarchical_move_rejects_duplicate_physical_source_offsets() {
    let shared = SharedNode::new("Shared", None);
    let shared_children = observable(vec![shared.clone()]);
    let first_parent = SharedNode::new("First", Some(shared_children.clone() as Models<SharedNode>));
    let second_parent = SharedNode::new("Second", Some(shared_children.clone() as Models<SharedNode>));
    let target = SharedNode::new("Target", None);
    let items = observable(vec![first_parent, second_parent, target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<SharedNode>);
    add_shared_expander(&source);

    assert_panics(|| {
        source.move_rows(
            &*source,
            &[index_path![0, 0], index_path![1, 0]],
            &IndexPath::new(2),
            RowDropPosition::Inside,
            RowMoveEffects::MOVE,
        )
    });

    assert_same(&shared, &single(shared_children.to_vec()));
    assert_eq!(0, target.children.count());
}

#[test]
fn hierarchical_move_rejects_an_aliased_self_drop() {
    let shared = SharedNode::new("Shared", None);
    let shared_children = observable(vec![shared.clone()]);
    let first_parent = SharedNode::new("First", Some(shared_children.clone() as Models<SharedNode>));
    let second_parent = SharedNode::new("Second", Some(shared_children.clone() as Models<SharedNode>));
    let items = observable(vec![first_parent, second_parent]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<SharedNode>);
    add_shared_expander(&source);

    assert_panics(|| {
        source.move_rows(&*source, &[index_path![0, 0]], &index_path![1, 0], RowDropPosition::Inside, RowMoveEffects::MOVE)
    });

    assert_same(&shared, &single(shared_children.to_vec()));
    assert_eq!(0, shared.children.count());
}

#[test]
fn hierarchical_move_rejects_a_target_collection_aliased_into_the_moved_subtree() {
    let shared_children = observable(Vec::<Rc<SharedNode>>::new());
    let first = SharedNode::new("First", Some(shared_children.clone() as Models<SharedNode>));
    let second = SharedNode::new("Second", Some(shared_children.clone() as Models<SharedNode>));
    let items = observable(vec![first.clone(), second.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<SharedNode>);
    add_shared_expander(&source);

    assert_panics(|| {
        source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::Inside, RowMoveEffects::MOVE)
    });

    assert_same_items(&[first, second], &items.to_vec());
    assert_eq!(0, shared_children.count());
}

#[test]
fn hierarchical_move_does_not_scan_subtrees_for_a_same_collection_reorder() {
    let first = Node::with_children("First", vec![Node::new("Child")]);
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    let child_selector_calls = counter();
    {
        let child_selector_calls = child_selector_calls.clone();
        source.columns().add(HierarchicalExpanderColumn::new(
            name_column("Name") as ColumnRef<Node>,
            move |x: &Rc<Node>| {
                bump(&child_selector_calls);
                Some(x.children() as Models<Node>)
            },
            None,
            None,
            None,
        ) as ColumnRef<Node>);
    }

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_eq!(0, child_selector_calls.get());
    assert_same_items(&[second, first], &items.to_vec());
}

#[test]
fn hierarchical_move_refreshes_materialized_rows_for_a_plain_list() {
    let first = Node::new("First");
    let second = Node::new("Second");
    let items = list(vec![first.clone(), second.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    add_node_expander(&source);
    assert_same_items(&[first.clone(), second.clone()], &row_models::<Node>(&*source.rows()));

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_same_items(&[second.clone(), first.clone()], &items.borrow());
    assert_same_items(&[second, first], &row_models::<Node>(&*source.rows()));
}

#[test]
fn hierarchical_move_refreshes_an_expanded_plain_child_list() {
    let moved = SharedNode::new("Moved", None);
    let existing = SharedNode::new("Existing", None);
    let children = list(vec![existing.clone()]);
    let target = SharedNode::new("Target", Some(children.clone() as Models<SharedNode>));
    let items = observable(vec![moved.clone(), target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<SharedNode>);
    add_shared_expander(&source);
    source.expand(&IndexPath::new(1));
    assert_same_items(&[moved.clone(), target.clone(), existing.clone()], &row_models::<SharedNode>(&*source.rows()));

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::Inside, RowMoveEffects::MOVE);

    assert_same_items(&[existing.clone(), moved.clone()], &children.borrow());
    assert_same_items(&[target, existing, moved], &row_models::<SharedNode>(&*source.rows()));
}

#[test]
fn hierarchical_move_preserves_source_owned_expansion() {
    let expanded = Node::with_children("Expanded", vec![Node::new("Child")]);
    let sibling = Node::new("Sibling");
    let items = observable(vec![expanded.clone(), sibling.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<Node>);
    add_node_expander(&source);
    source.expand(&IndexPath::new(0));
    assert_eq!(3, source.rows().count());

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_same_items(&[sibling, expanded.clone(), expanded.children().get(0)], &row_models::<Node>(&*source.rows()));
    assert!(is_expanded(&source.rows().get(1)));
}

#[test]
fn hierarchical_move_preserves_selection_anchors() {
    // [InlineData(false)], [InlineData(true)]
    for notifying in [false, true] {
        let first = Node::new("First");
        let second = Node::new("Second");
        let items: Models<Node> = if notifying {
            observable(vec![first.clone(), second.clone()]) as Models<Node>
        } else {
            list(vec![first.clone(), second.clone()]) as Models<Node>
        };
        let source = HierarchicalTreeDataGridSource::new(items);
        let selection = hierarchical_selection(&source);
        selection.set_selected_index(IndexPath::new(0));

        source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);

        assert_eq!(IndexPath::new(1), selection.anchor_index(), "notifying: {notifying}");
        assert_eq!(IndexPath::new(1), selection.range_anchor_index(), "notifying: {notifying}");
    }
}

#[test]
fn hierarchical_move_preserves_empty_selection_anchors() {
    let items = observable(vec![Node::new("First"), Node::new("Second")]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<Node>);
    let selection = hierarchical_selection(&source);
    selection.set_single_select(false);
    selection.set_selected_index(IndexPath::new(0));
    selection.select(&IndexPath::new(1));
    selection.set_anchor_index(IndexPath::UNSELECTED);
    selection.set_range_anchor_index(IndexPath::UNSELECTED);
    assert_eq!(IndexPath::UNSELECTED, selection.anchor_index());
    assert_eq!(IndexPath::UNSELECTED, selection.range_anchor_index());

    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);

    assert_eq!(IndexPath::UNSELECTED, selection.anchor_index());
    assert_eq!(IndexPath::UNSELECTED, selection.range_anchor_index());
}

#[test]
fn hierarchical_reset_clears_a_stale_range_anchor_before_a_move() {
    let survivor = Node::new("Survivor");
    let removed = Node::with_children("Removed", vec![Node::new("Child")]);
    let other = Node::new("Other");
    let items = ResettingCollection::new(vec![survivor.clone(), removed, other.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    add_node_expander(&source);
    let selection = hierarchical_selection(&source);
    selection.set_range_anchor_index(index_path![1, 0]);
    assert_eq!(index_path![1, 0], selection.range_anchor_index());

    items.reset(vec![survivor.clone(), other.clone()]);

    assert_eq!(IndexPath::UNSELECTED, selection.range_anchor_index());
    source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::After, RowMoveEffects::MOVE);
    assert_same_items(&[other, survivor], &items.to_vec());
}

#[test]
fn hierarchical_move_restores_a_selection_reached_through_an_alias() {
    let moved = SharedNode::new("Moved", None);
    let remaining = SharedNode::new("Remaining", None);
    let shared_children = observable(vec![moved.clone(), remaining.clone()]);
    let first_parent = SharedNode::new("First", Some(shared_children.clone() as Models<SharedNode>));
    let second_parent = SharedNode::new("Second", Some(shared_children.clone() as Models<SharedNode>));
    let target = SharedNode::new("Target", None);
    let items = observable(vec![first_parent, second_parent, target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<SharedNode>);
    add_shared_expander(&source);
    let selection = hierarchical_selection(&source);
    selection.set_selected_index(index_path![1, 0]);

    source.move_rows(&*source, &[index_path![0, 0]], &IndexPath::new(2), RowDropPosition::Inside, RowMoveEffects::MOVE);

    assert_same(&remaining, &single(shared_children.to_vec()));
    assert_same(&moved, &single(target.children.to_vec()));
    assert_eq!(index_path![2, 0], selection.selected_index());
}

#[test]
fn hierarchical_move_remaps_retained_selection_through_an_aliased_target() {
    let moved = SharedNode::new("Moved", None);
    let first = SharedNode::new("A", None);
    let selected = SharedNode::new("B", None);
    let shared_children = observable(vec![first.clone(), selected.clone()]);
    let first_parent = SharedNode::new("First", Some(shared_children.clone() as Models<SharedNode>));
    let second_parent = SharedNode::new("Second", Some(shared_children.clone() as Models<SharedNode>));
    let items = observable(vec![moved.clone(), first_parent, second_parent]);
    let source = HierarchicalTreeDataGridSource::new(items as Models<SharedNode>);
    add_shared_expander(&source);
    let selection = hierarchical_selection(&source);
    selection.set_selected_index(index_path![1, 1]);

    source.move_rows(&*source, &[IndexPath::new(0)], &index_path![2, 1], RowDropPosition::Before, RowMoveEffects::MOVE);

    assert_same_items(&[first, moved, selected], &shared_children.to_vec());
    assert_eq!(index_path![0, 2], selection.selected_index());
}

#[test]
fn hierarchical_move_rejects_a_read_only_target_before_mutation() {
    let moved = SharedNode::new("Moved", None);
    // `ReadOnlyCollection<SharedNode>`: a fixed sequence, which is not a mutable list.
    let read_only_children = array(Vec::<Rc<SharedNode>>::new());
    let target = SharedNode::new("Target", Some(read_only_children.clone()));
    let items = observable(vec![moved.clone(), target.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<SharedNode>);

    assert_panics(|| {
        source.move_rows(&*source, &[IndexPath::new(0)], &IndexPath::new(1), RowDropPosition::Inside, RowMoveEffects::MOVE)
    });

    assert_same_items(&[moved, target], &items.to_vec());
    assert_eq!(0, read_only_children.count());
}

#[test]
fn moving_an_expanded_root_recomputes_the_flattened_destination() {
    let first = Node::with_children("First", vec![Node::new("Child")]);
    let second = Node::new("Second");
    let items = observable(vec![first.clone(), second.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    add_node_expander(&source);
    source.expand(&IndexPath::new(0));
    let expanded_row = source.rows().get(0);

    items.move_item(0, 1);

    assert_same_items(&[second, first], &items.to_vec());
    assert_eq!(vec!["Second", "First", "Child"], names(&*source.rows()));
    assert_same(&expanded_row, &source.rows().get(1));
    assert!(is_expanded(&source.rows().get(1)));
}

#[test]
fn moving_an_expanded_root_preserves_its_row_while_sorted() {
    let expanded = Node::with_children("Same", vec![Node::new("Child")]);
    let sibling = Node::new("Same");
    let items = observable(vec![expanded.clone(), sibling.clone()]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    let column = node_expander(name_column("Name"));
    source.columns().add(column.clone() as ColumnRef<Node>);
    source.sort_by(Some(&*column as &dyn IColumn), ListSortDirection::Ascending);
    source.expand(&IndexPath::new(0));
    let expanded_row = source.rows().get(0);

    items.move_item(0, 1);

    assert_same_items(&[sibling.clone(), expanded.clone()], &items.to_vec());
    assert_same_items(&[sibling, expanded.clone(), expanded.children().get(0)], &row_models::<Node>(&*source.rows()));
    assert_same(&expanded_row, &source.rows().get(1));
    assert!(is_expanded(&source.rows().get(1)));
}

#[test]
fn sorted_row_removal_reports_the_removed_row_not_its_model() {
    let first = Node::new("B");
    let second = Node::new("A");
    let items = observable(vec![first.clone(), second]);
    let source = HierarchicalTreeDataGridSource::new(items.clone() as Models<Node>);
    let column = node_expander(name_column("Name"));
    source.columns().add(column.clone() as ColumnRef<Node>);
    source.sort_by(Some(&*column as &dyn IColumn), ListSortDirection::Ascending);
    let removed_row = single(
        source.rows().to_vec().into_iter().filter(|x| same_rc(&row_model::<Node>(x), &first)).collect::<Vec<RowRef>>(),
    );
    let change: Rc<RefCell<Option<Vec<RowRef>>>> = Rc::new(RefCell::new(None));
    {
        let change = change.clone();
        source.rows().add_collection_changed(Rc::new(move |e: &RowsChangedArgs<'_>| {
            *change.borrow_mut() = Some(e.old_items.to_vec());
        }));
    }

    items.remove(&first);

    let old_items = change.borrow().clone().expect("a change of the rows");
    assert_same(&removed_row, &old_items[0]);
}

#[test]
fn nested_expansion_binding_tracks_leaf_and_intermediate_changes() {
    let child = NestedBoundNode::new(vec![NestedBoundNode::new(Vec::new())]);
    let parent = NestedBoundNode::new(vec![child.clone()]);
    let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
    source.columns().add(HierarchicalExpanderColumn::new(
        TextColumn::new(header("Expanded"), nested_is_expanded(), None, None, None) as ColumnRef<NestedBoundNode>,
        |x: &Rc<NestedBoundNode>| Some(x.children.clone() as Models<NestedBoundNode>),
        None,
        // `isExpandedSelector: x => x.Expansion.IsExpanded`, with the setter the reference synthesizes from it.
        Some(nested_is_expanded().with_setter(|x: &Rc<NestedBoundNode>, value: bool| x.expansion().set_is_expanded(value))),
        None,
    ) as ColumnRef<NestedBoundNode>);

    assert_eq!(1, source.rows().count());
    parent.expansion().set_is_expanded(true);
    assert_eq!(2, source.rows().count());
    child.expansion().set_is_expanded(true);
    assert_eq!(3, source.rows().count());
    child.set_expansion(ExpansionState::new(false));
    assert_eq!(2, source.rows().count());
    parent.set_expansion(ExpansionState::new(false));
    assert_eq!(1, source.rows().count());
    parent.set_expansion(ExpansionState::new(true));
    assert_eq!(2, source.rows().count());
    child.set_expansion(ExpansionState::new(true));
    assert_eq!(3, source.rows().count());
}

#[test]
fn show_expander_reflects_child_collection_changes_without_a_presentation() {
    let parent = Node::new("Parent");
    let source = HierarchicalTreeDataGridSource::new(array(vec![parent.clone()]));
    add_node_expander(&source);
    let row = hierarchical_row::<Node>(&source.rows().get(0));

    assert!(!row.show_expander());
    parent.children().add(Node::new("Child"));
    assert!(row.show_expander());
    parent.children().clear();
    assert!(!row.show_expander());
}

#[test]
fn core_has_no_ui_framework_assembly_references() {
    // The reference checks that the core references no assembly of its UI framework (the reference's test
    // name names the framework); the port's equivalent: the crate depends on the base library only, never on
    // the controls (a crate outside the framework, such as a hasher, is not a UI framework).
    let manifest = include_str!("../Cargo.toml");
    let dependencies = manifest.split("[dependencies]").nth(1).expect("a dependencies table");
    let dependencies = dependencies.split("\n[").next().unwrap_or_default();
    for line in dependencies.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')) {
        if line.starts_with("ferroui") {
            assert!(line.starts_with("ferroui-base"), "an unexpected dependency: {line}");
        }
    }
}

#[test]
fn equal_single_index_paths_have_equal_hash_codes() {
    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashSet;
    use std::hash::{Hash, Hasher};

    fn hash_code(path: &IndexPath) -> u64 {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        hasher.finish()
    }

    let encoded = IndexPath::new(0);
    let from_array = IndexPath::from_indexes(&[0]);
    let mut paths = HashSet::new();
    paths.insert(encoded.clone());

    assert_eq!(encoded, from_array);
    assert_eq!(hash_code(&encoded), hash_code(&from_array));
    assert!(paths.contains(&from_array));
}

fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

fn bump(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

/// `source.RowSelection!` of a flat source.
fn flat_selection<M: TreeModel>(source: &FlatTreeDataGridSource<M>) -> Rc<TreeSelectionModelBase<M>> {
    source.row_selection().expect("a row selection").base().clone()
}

/// `source.RowSelection!` of a hierarchical source.
fn hierarchical_selection<M: TreeModel>(source: &HierarchicalTreeDataGridSource<M>) -> Rc<TreeSelectionModelBase<M>> {
    source.row_selection().expect("a row selection").base().clone()
}

/// `Assert.IsType<HierarchicalRow<M>>(row)`.
fn hierarchical_row<M: TreeModel>(row: &RowRef) -> Rc<HierarchicalRow<M>> {
    as_hierarchical_row::<M>(row).expect("a hierarchical row")
}

/// `((IExpanderRow<M>)row).IsExpanded`.
fn is_expanded(row: &RowRef) -> bool {
    row.as_expander().expect("an expander row").is_expanded()
}

/// The names of the models of the rows (`source.Rows.Select(x => ((Node)x.Model!).Name)`).
fn names(rows: &dyn IRows) -> Vec<String> {
    row_models::<Node>(rows).iter().map(|x| x.name()).collect()
}

fn shared_names(rows: &dyn IRows) -> Vec<String> {
    row_models::<SharedNode>(rows).iter().map(|x| x.name.clone()).collect()
}

/// The names of a collection of models (`items.Select(x => x.Name)`).
fn model_names(items: &Models<Node>) -> Vec<String> {
    ModelCollection::to_vec(&**items).iter().map(|x| x.name()).collect()
}

/// `new TextColumn<Node, string>(header, x => x.Name)`.
fn name_column(text: &str) -> Rc<ValueColumn<Node, String>> {
    TextColumn::new(header(text), Accessor::member("Name", |x: &Rc<Node>| x.name()), None, None, None)
}

/// `new TextColumn<Node, string>(header, x => x.Name, (x, value) => x.Name = value)`.
fn editable_name_column(text: &str) -> Rc<ValueColumn<Node, String>> {
    TextColumn::new(
        header(text),
        Accessor::member("Name", |x: &Rc<Node>| x.name()).with_setter(|x: &Rc<Node>, value: String| x.set_name(value)),
        None,
        None,
        None,
    )
}

/// `new HierarchicalExpanderColumn<Node>(inner, x => x.Children)`.
fn node_expander(inner: Rc<ValueColumn<Node, String>>) -> Rc<HierarchicalExpanderColumn<Node>> {
    HierarchicalExpanderColumn::new(inner as ColumnRef<Node>, |x: &Rc<Node>| Some(x.children() as Models<Node>), None, None, None)
}

fn add_node_expander(source: &HierarchicalTreeDataGridSource<Node>) -> Rc<HierarchicalExpanderColumn<Node>> {
    let column = node_expander(name_column("Name"));
    source.columns().add(column.clone() as ColumnRef<Node>);
    column
}

fn add_shared_expander(source: &HierarchicalTreeDataGridSource<SharedNode>) {
    source.columns().add(HierarchicalExpanderColumn::new(
        TextColumn::new(header("Name"), Accessor::member("Name", |x: &Rc<SharedNode>| x.name.clone()), None, None, None)
            as ColumnRef<SharedNode>,
        |x: &Rc<SharedNode>| Some(x.children.clone()),
        None,
        None,
        None,
    ) as ColumnRef<SharedNode>);
}

/// `new TextColumn<BoundNode, bool>("Expanded", x => x.IsExpanded)`.
fn expanded_text_column() -> ColumnRef<BoundNode> {
    TextColumn::new(header("Expanded"), Accessor::member("IsExpanded", |x: &Rc<BoundNode>| x.is_expanded()), None, None, None)
        as ColumnRef<BoundNode>
}

/// The expander column of `BoundNode`, its expansion bound (`isExpandedSelector: x => x.IsExpanded`, with the
/// setter the reference synthesizes from it) or not.
fn bound_expander(bound_expansion: bool) -> ColumnRef<BoundNode> {
    let is_expanded_selector = bound_expansion.then(|| {
        Accessor::member("IsExpanded", |x: &Rc<BoundNode>| x.is_expanded())
            .with_setter(|x: &Rc<BoundNode>, value: bool| x.set_is_expanded(value))
    });
    HierarchicalExpanderColumn::new(
        expanded_text_column(),
        |x: &Rc<BoundNode>| Some(x.children() as Models<BoundNode>),
        None,
        is_expanded_selector,
        None,
    ) as ColumnRef<BoundNode>
}

/// `RowExpanding`, `RowExpanded`, `RowCollapsing` and `RowCollapsed` counted together.
fn count_lifecycle_events(source: &HierarchicalTreeDataGridSource<BoundNode>) -> Rc<Cell<i32>> {
    let events = counter();
    let handler: Rc<RowEventHandler<BoundNode>> = {
        let events = events.clone();
        Rc::new(move |_e: &RowEventArgs<Rc<HierarchicalRow<BoundNode>>>| bump(&events))
    };
    source.add_row_expanding(handler.clone());
    source.add_row_expanded(handler.clone());
    source.add_row_collapsing(handler.clone());
    source.add_row_collapsed(handler);
    events
}

/// `x => x.Expansion.IsExpanded`: read from the expansion state, observed on it.
fn nested_is_expanded() -> Accessor<NestedBoundNode, bool> {
    Accessor::member("Expansion", |x: &Rc<NestedBoundNode>| x.expansion().is_expanded())
        .through(|x: &Rc<NestedBoundNode>| Some(x.expansion() as BoxedValue), "IsExpanded")
}

struct Node {
    name: RefCell<String>,
    children: Rc<FerroList<Rc<Node>>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Node {
    fn new(name: &str) -> Rc<Self> {
        Self::with_children(name, Vec::new())
    }

    /// `new Node(name) { Children = { children } }`.
    fn with_children(name: &str, children: Vec<Rc<Node>>) -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(name.to_string()), children: observable(children) })
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: String) {
        *self.name.borrow_mut() = value;
    }

    fn children(&self) -> Rc<FerroList<Rc<Node>>> {
        self.children.clone()
    }
}

struct SharedNode {
    name: String,
    children: Models<SharedNode>,
}

impl PartialEq for SharedNode {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl SharedNode {
    /// `new SharedNode(name, children)`: an observable collection when no children are given.
    fn new(name: &str, children: Option<Models<SharedNode>>) -> Rc<Self> {
        Rc::new(Self {
            name: name.to_string(),
            children: children.unwrap_or_else(|| observable(Vec::<Rc<SharedNode>>::new()) as Models<SharedNode>),
        })
    }
}

struct BoundNode {
    expanded: Cell<bool>,
    children: RefCell<Rc<FerroList<Rc<BoundNode>>>>,
    changed: Event<str>,
}

impl PartialEq for BoundNode {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for BoundNode {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl Model for BoundNode {
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
        builder.notify_property_changed()
    }
}

impl BoundNode {
    fn new() -> Rc<Self> {
        Self::create(false, Vec::new())
    }

    /// `new BoundNode { IsExpanded = expanded, Children = { children } }`.
    fn create(expanded: bool, children: Vec<Rc<BoundNode>>) -> Rc<Self> {
        Self::with_children(expanded, observable(children))
    }

    /// `new BoundNode { IsExpanded = expanded, Children = children }`.
    fn with_children(expanded: bool, children: Rc<FerroList<Rc<BoundNode>>>) -> Rc<Self> {
        Self::register();
        Rc::new(Self { expanded: Cell::new(expanded), children: RefCell::new(children), changed: Event::new() })
    }

    fn is_expanded(&self) -> bool {
        self.expanded.get()
    }

    fn set_is_expanded(&self, value: bool) {
        if self.expanded.get() == value {
            return;
        }
        self.expanded.set(value);
        self.changed.raise("IsExpanded");
    }

    fn children(&self) -> Rc<FerroList<Rc<BoundNode>>> {
        self.children.borrow().clone()
    }

    fn set_children(&self, value: Rc<FerroList<Rc<BoundNode>>>) {
        if Rc::ptr_eq(&self.children.borrow(), &value) {
            return;
        }
        *self.children.borrow_mut() = value;
        self.changed.raise("Children");
    }

    fn notify_unrelated_property_changed(&self) {
        self.changed.raise("NotifyUnrelatedPropertyChanged");
    }
}

struct NestedBoundNode {
    expansion: RefCell<Rc<ExpansionState>>,
    children: Rc<FerroList<Rc<NestedBoundNode>>>,
    changed: Event<str>,
}

impl PartialEq for NestedBoundNode {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for NestedBoundNode {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl Model for NestedBoundNode {
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
        builder.notify_property_changed()
    }
}

impl NestedBoundNode {
    /// `new NestedBoundNode { Children = { children } }`.
    fn new(children: Vec<Rc<NestedBoundNode>>) -> Rc<Self> {
        Self::register();
        Rc::new(Self {
            expansion: RefCell::new(ExpansionState::new(false)),
            children: observable(children),
            changed: Event::new(),
        })
    }

    fn expansion(&self) -> Rc<ExpansionState> {
        self.expansion.borrow().clone()
    }

    fn set_expansion(&self, value: Rc<ExpansionState>) {
        if Rc::ptr_eq(&self.expansion.borrow(), &value) {
            return;
        }
        *self.expansion.borrow_mut() = value;
        self.changed.raise("Expansion");
    }
}

struct ExpansionState {
    expanded: Cell<bool>,
    changed: Event<str>,
}

impl PartialEq for ExpansionState {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ExpansionState {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl Model for ExpansionState {
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self> {
        builder.notify_property_changed()
    }
}

impl ExpansionState {
    /// `new ExpansionState { IsExpanded = expanded }`.
    fn new(expanded: bool) -> Rc<Self> {
        Self::register();
        Rc::new(Self { expanded: Cell::new(expanded), changed: Event::new() })
    }

    fn is_expanded(&self) -> bool {
        self.expanded.get()
    }

    fn set_is_expanded(&self, value: bool) {
        if self.expanded.get() == value {
            return;
        }
        self.expanded.set(value);
        self.changed.raise("IsExpanded");
    }
}
