//! `CellSelectionTests.cs`: the selection of cells of flat and hierarchical sources.

use super::support::{array, header, single};
use crate::accessor::Accessor;
use crate::model_collection::{observable, Models};
use crate::models::{ColumnRef, HierarchicalExpanderColumn, IColumn, ListSortDirection, TextColumn};
use crate::selection::{ITreeDataGridSelection, TreeDataGridCellSelectionChangedEventArgs, TreeDataGridCellSelectionModel};
use crate::{
    index_path, CellIndex, FlatTreeDataGridSource, HierarchicalTreeDataGridSource, ITreeDataGridSource, ITreeDataGridSourceOf,
    IndexPath, RowDropPosition, RowMoveEffects,
};
use ferroui_base::collections::FerroList;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn range_selects_columns_and_rows_in_one_notification() {
    let source = flat(None);
    let selection = cell_selection(&source);
    selection.set_single_select(false);
    source.set_selection(Some(selection.clone() as Rc<dyn ITreeDataGridSelection>));
    let typed = Rc::new(Cell::new(0));
    let untyped = Rc::new(Cell::new(0));
    {
        let typed = typed.clone();
        let model = selection.clone();
        selection.add_selection_changed(Rc::new(move |_e: &TreeDataGridCellSelectionChangedEventArgs| {
            typed.set(typed.get() + 1);
            assert_eq!(4, model.count());
        }));
    }
    {
        // The typed and the untyped `SelectionChanged` of the reference are one event here.
        let untyped = untyped.clone();
        selection.add_selection_changed(Rc::new(move |_e: &TreeDataGridCellSelectionChangedEventArgs| {
            untyped.set(untyped.get() + 1);
        }));
    }
    selection.set_selected_range(&CellIndex::new(0, 0), 2, 2);
    assert_eq!(1, typed.get());
    assert_eq!(1, untyped.get());
    assert_eq!(
        vec![CellIndex::new(0, 0), CellIndex::new(1, 0), CellIndex::new(0, 1), CellIndex::new(1, 1)],
        selection.selected_indexes()
    );
    assert_eq!(CellIndex::new(0, 0), selection.anchor_index());
    assert_eq!(CellIndex::new(1, 1), selection.range_anchor_index());
}

#[test]
fn sorted_range_uses_display_order_and_keeps_model_indexes() {
    let source = flat(None);
    source.sort_by_column(&source.columns().get(0), ListSortDirection::Ascending);
    let selection = cell_selection(&source);
    selection.set_single_select(false);
    selection.set_selected_range(&CellIndex::new(1, source.rows().row_index_to_model_index(0)), 1, 2);
    assert!(selection.is_selected_at(1, &IndexPath::new(1))); // A
    assert!(selection.is_selected_at(1, &IndexPath::new(2))); // B
    assert!(!selection.is_selected_at(1, &IndexPath::new(0))); // C
    assert_eq!(CellIndex::new(1, 1), selection.selected_index());
    source.clear_sort();
    assert!(selection.is_selected_at(1, &IndexPath::new(1)));
}

#[test]
fn backwards_and_single_cell_ranges_have_valid_endpoints() {
    let source = flat(None);
    let selection = cell_selection(&source);
    selection.set_single_select(false);
    selection.set_selected_range(&CellIndex::new(1, 2), -2, -3);
    assert_eq!(6, selection.count());
    assert_eq!(CellIndex::new(1, 2), selection.anchor_index());
    assert_eq!(CellIndex::new(0, 0), selection.range_anchor_index());
    selection.set_single_select(true);
    assert_eq!(1, selection.selected_indexes().len());
    selection.set_selected_range(&CellIndex::new(0, 0), 2, 3);
    assert_eq!(CellIndex::new(0, 0), single(selection.selected_indexes()));
    selection.clear();
    assert!(selection.selected_indexes().is_empty());
}

#[test]
fn insert_remove_move_and_reset_track_selected_models_and_notify() {
    let items = observable(vec![Node::new("A"), Node::new("B"), Node::new("C")]);
    let source = flat(Some(items.clone()));
    let selection = cell_selection(&source);
    source.set_selection(Some(selection.clone() as Rc<dyn ITreeDataGridSelection>));
    selection.set_selected_index(CellIndex::new(1, 1));
    let events = Rc::new(Cell::new(0));
    {
        let events = events.clone();
        selection.add_selection_changed(Rc::new(move |_e: &TreeDataGridCellSelectionChangedEventArgs| {
            events.set(events.get() + 1);
        }));
    }
    items.insert(0, Node::new("New"));
    assert_eq!(CellIndex::new(1, 2), selection.selected_index());
    assert!(events.get() > 0);
    items.move_item(2, 0);
    assert_eq!(CellIndex::new(1, 0), selection.selected_index());
    items.remove_at(0);
    assert!(selection.selected_indexes().is_empty());
    selection.set_selected_index(CellIndex::new(0, 0));
    source.set_items(array(vec![Node::new("Replacement")]));
    assert!(selection.selected_indexes().is_empty());
}

#[test]
fn column_insert_remove_and_hidden_state_preserve_source_identity() {
    let source = flat(None);
    let selection = cell_selection(&source);
    selection.set_selected_index(CellIndex::new(1, 0));
    source.columns().get(0).set_is_visible(false);
    assert_eq!(CellIndex::new(1, 0), selection.selected_index());
    source.columns().insert(0, name_column("New") as ColumnRef<Node>);
    assert_eq!(CellIndex::new(2, 0), selection.selected_index());
    source.columns().remove_at(2);
    assert!(selection.selected_indexes().is_empty());
}

#[test]
fn hierarchical_range_uses_nested_model_paths() {
    let source = HierarchicalTreeDataGridSource::new(array(vec![Node::with_children(
        "Root",
        vec![Node::new("A"), Node::new("B")],
    )]));
    source.columns().add(expander(name_column("Name")) as ColumnRef<Node>);
    source.expand(&IndexPath::new(0));
    let selection = cell_selection(&source);
    selection.set_single_select(false);
    selection.set_selected_range(&CellIndex::new(0, index_path![0, 0]), 1, 2);
    assert_eq!(
        vec![CellIndex::new(0, index_path![0, 0]), CellIndex::new(0, index_path![0, 1])],
        selection.selected_indexes()
    );
    source.collapse(&IndexPath::new(0));
    assert_eq!(2, selection.count());
    source.expand(&IndexPath::new(0));
    assert!(selection.is_selected(&CellIndex::new(0, index_path![0, 1])));
}

#[test]
fn source_move_preserves_cells_on_the_moved_rows() {
    let source = flat(None);
    let selection = cell_selection(&source);
    selection.set_single_select(false);
    source.set_selection(Some(selection.clone() as Rc<dyn ITreeDataGridSelection>));
    selection.set_selected_range(&CellIndex::new(1, 0), 1, 2);
    source.move_rows(
        &*source,
        &[IndexPath::new(0), IndexPath::new(1)],
        &IndexPath::new(2),
        RowDropPosition::After,
        RowMoveEffects::MOVE,
    );
    assert_eq!(vec![CellIndex::new(1, 1), CellIndex::new(1, 2)], selection.selected_indexes());
}

#[test]
fn hierarchical_source_move_preserves_selected_child_cells() {
    let source = HierarchicalTreeDataGridSource::new(observable(vec![
        Node::with_children("Root", vec![Node::new("Child")]),
        Node::new("Target"),
    ]) as Models<Node>);
    source.columns().add(expander(name_column("Name")) as ColumnRef<Node>);
    source.expand(&IndexPath::new(0));
    let selection = cell_selection(&source);
    source.set_selection(Some(selection.clone() as Rc<dyn ITreeDataGridSelection>));
    selection.set_selected_index(CellIndex::new(0, index_path![0, 0]));
    source.move_rows(&*source, &[index_path![0, 0]], &IndexPath::new(1), RowDropPosition::Inside, RowMoveEffects::MOVE);
    assert_eq!(CellIndex::new(0, index_path![1, 0]), selection.selected_index());
}

#[test]
fn invalid_or_empty_range_clears_selection() {
    // [InlineData(-1, 0, 1, 1)], [InlineData(0, 10, 1, 1)], [InlineData(0, 0, 0, 1)], [InlineData(0, 0, 1, 0)]
    for (column, row, columns, rows) in [(-1, 0, 1, 1), (0, 10, 1, 1), (0, 0, 0, 1), (0, 0, 1, 0)] {
        let source = flat(None);
        let selection = cell_selection(&source);
        selection.set_selected_index(CellIndex::new(0, 0));
        selection.set_selected_range(&CellIndex::new(column, row), columns, rows);
        assert!(selection.selected_indexes().is_empty(), "({column}, {row}, {columns}, {rows})");
    }
}

#[test]
fn disposed_selection_stops_observing_collections() {
    let items = observable(vec![Node::new("A")]);
    let source = flat(Some(items.clone()));
    let selection = cell_selection(&source);
    selection.set_selected_index(CellIndex::new(0, 0));
    selection.dispose();
    let events = Rc::new(Cell::new(0));
    {
        let events = events.clone();
        selection.add_selection_changed(Rc::new(move |_e: &TreeDataGridCellSelectionChangedEventArgs| {
            events.set(events.get() + 1);
        }));
    }
    items.clear();
    source.columns().clear();
    assert_eq!(0, events.get());
    assert!(selection.selected_indexes().is_empty());
}

fn flat(items: Option<Rc<FerroList<Rc<Node>>>>) -> Rc<FlatTreeDataGridSource<Node>> {
    let items = items.unwrap_or_else(|| observable(vec![Node::new("C"), Node::new("A"), Node::new("B")]));
    FlatTreeDataGridSource::new(items as Models<Node>)
        .with_column(name_column("Name") as ColumnRef<Node>)
        .with_column(name_column("Other") as ColumnRef<Node>)
}

/// `new TreeDataGridCellSelectionModel<Node>(source)`.
fn cell_selection<S: ITreeDataGridSourceOf<Node> + 'static>(source: &Rc<S>) -> Rc<TreeDataGridCellSelectionModel<Node>> {
    TreeDataGridCellSelectionModel::new(&(source.clone() as Rc<dyn ITreeDataGridSourceOf<Node>>))
}

/// `new TextColumn<Node, string>(header, x => x.Name)`.
fn name_column(text: &str) -> Rc<crate::models::ValueColumn<Node, String>> {
    TextColumn::new(header(text), Accessor::member("Name", |x: &Rc<Node>| x.name.clone()), None, None, None)
}

/// `new HierarchicalExpanderColumn<Node>(inner, x => x.Children)`.
fn expander(inner: Rc<crate::models::ValueColumn<Node, String>>) -> Rc<HierarchicalExpanderColumn<Node>> {
    HierarchicalExpanderColumn::new(
        inner as ColumnRef<Node>,
        |x: &Rc<Node>| Some(x.children.clone() as Models<Node>),
        None,
        None,
        None,
    )
}

/// `sealed record Node(string Name)` with its `Children`: equal by value, as a record is.
struct Node {
    name: String,
    children: Rc<FerroList<Rc<Node>>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && Rc::ptr_eq(&self.children, &other.children)
    }
}

impl Node {
    fn new(name: &str) -> Rc<Self> {
        Self::with_children(name, Vec::new())
    }

    fn with_children(name: &str, children: Vec<Rc<Node>>) -> Rc<Self> {
        Rc::new(Self { name: name.to_string(), children: observable(children) })
    }
}
