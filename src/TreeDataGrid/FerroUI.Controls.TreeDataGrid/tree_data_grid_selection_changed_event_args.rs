//! `TreeDataGridSelectionChangedEventArgs.cs`: the arguments of the selection changed event of the
//! grid, for rows and for cells.

use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::selection::{
    box_model, IReadOnlyList, ReadOnlyList, TreeSelectionModelSelectionChangedEventArgs,
};
use ferroui_treedatagrid_core::{CellIndex, IndexPath};
use std::rc::Rc;

/// The arguments of [`TreeDataGrid`](crate::TreeDataGrid)'s `SelectionChanged`
/// (`TreeDataGridSelectionChangedEventArgs`): the rows deselected and selected, with their models,
/// and for a cell selection the cells deselected and selected.
#[derive(Clone)]
pub struct TreeDataGridSelectionChangedEventArgs {
    deselected_indexes: Rc<ReadOnlyList<IndexPath>>,
    selected_indexes: Rc<ReadOnlyList<IndexPath>>,
    deselected_items: Rc<ReadOnlyList<Option<BoxedValue>>>,
    selected_items: Rc<ReadOnlyList<Option<BoxedValue>>>,
    deselected_cell_indexes: Rc<[CellIndex]>,
    selected_cell_indexes: Rc<[CellIndex]>,
}

impl TreeDataGridSelectionChangedEventArgs {
    /// The arguments; a list that is `None` is empty.
    pub fn new(
        deselected_indexes: Option<Vec<IndexPath>>,
        selected_indexes: Option<Vec<IndexPath>>,
        deselected_items: Option<Vec<Option<BoxedValue>>>,
        selected_items: Option<Vec<Option<BoxedValue>>>,
        deselected_cell_indexes: Option<Vec<CellIndex>>,
        selected_cell_indexes: Option<Vec<CellIndex>>,
    ) -> Self {
        Self {
            deselected_indexes: Rc::new(ReadOnlyList::new(deselected_indexes.unwrap_or_default())),
            selected_indexes: Rc::new(ReadOnlyList::new(selected_indexes.unwrap_or_default())),
            deselected_items: Rc::new(ReadOnlyList::new(deselected_items.unwrap_or_default())),
            selected_items: Rc::new(ReadOnlyList::new(selected_items.unwrap_or_default())),
            deselected_cell_indexes: deselected_cell_indexes.unwrap_or_default().into(),
            selected_cell_indexes: selected_cell_indexes.unwrap_or_default().into(),
        }
    }

    /// The arguments of a change of rows, from the arguments of the change of a tree selection.
    pub fn from_rows(e: &dyn TreeSelectionModelSelectionChangedEventArgs) -> Self {
        Self::new(
            Some(e.deselected_indexes().to_vec()),
            Some(e.selected_indexes().to_vec()),
            Some(e.deselected_items().to_vec()),
            Some(e.selected_items().to_vec()),
            None,
            None,
        )
    }

    /// The arguments of a change of cells (`deselectedCellIndexes:`, `selectedCellIndexes:`).
    pub fn from_cells(deselected: Vec<CellIndex>, selected: Vec<CellIndex>) -> Self {
        Self::new(None, None, None, None, Some(deselected), Some(selected))
    }

    /// The indexes of the rows that were removed from the selection.
    pub fn deselected_indexes(&self) -> Vec<IndexPath> {
        self.deselected_indexes.to_vec()
    }

    /// The indexes of the rows that were added to the selection.
    pub fn selected_indexes(&self) -> Vec<IndexPath> {
        self.selected_indexes.to_vec()
    }

    /// The models of the rows that were removed from the selection.
    pub fn deselected_items(&self) -> Vec<Option<BoxedValue>> {
        self.deselected_items.to_vec()
    }

    /// The models of the rows that were added to the selection.
    pub fn selected_items(&self) -> Vec<Option<BoxedValue>> {
        self.selected_items.to_vec()
    }

    /// The cells that were removed from the selection.
    pub fn deselected_cell_indexes(&self) -> &[CellIndex] {
        &self.deselected_cell_indexes
    }

    /// The cells that were added to the selection.
    pub fn selected_cell_indexes(&self) -> &[CellIndex] {
        &self.selected_cell_indexes
    }
}

impl Default for TreeDataGridSelectionChangedEventArgs {
    fn default() -> Self {
        Self::new(None, None, None, None, None, None)
    }
}

impl TreeSelectionModelSelectionChangedEventArgs for TreeDataGridSelectionChangedEventArgs {
    fn deselected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>> {
        self.deselected_indexes.clone()
    }

    fn selected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>> {
        self.selected_indexes.clone()
    }

    fn deselected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>> {
        self.deselected_items.clone()
    }

    fn selected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>> {
        self.selected_items.clone()
    }
}

/// The typed arguments of the selection changed event (`TreeDataGridSelectionChangedEventArgs<TModel>`):
/// the rows with their models of type `M`, and the cells.
#[derive(Clone)]
pub struct TreeDataGridSelectionChangedEventArgsOf<M: TreeModel> {
    deselected_indexes: Vec<IndexPath>,
    selected_indexes: Vec<IndexPath>,
    deselected_items: Vec<Option<Rc<M>>>,
    selected_items: Vec<Option<Rc<M>>>,
    deselected_cell_indexes: Vec<CellIndex>,
    selected_cell_indexes: Vec<CellIndex>,
}

impl<M: TreeModel> TreeDataGridSelectionChangedEventArgsOf<M> {
    /// The arguments; a list that is `None` is empty.
    pub fn new(
        deselected_indexes: Option<Vec<IndexPath>>,
        selected_indexes: Option<Vec<IndexPath>>,
        deselected_items: Option<Vec<Option<Rc<M>>>>,
        selected_items: Option<Vec<Option<Rc<M>>>>,
        deselected_cell_indexes: Option<Vec<CellIndex>>,
        selected_cell_indexes: Option<Vec<CellIndex>>,
    ) -> Self {
        Self {
            deselected_indexes: deselected_indexes.unwrap_or_default(),
            selected_indexes: selected_indexes.unwrap_or_default(),
            deselected_items: deselected_items.unwrap_or_default(),
            selected_items: selected_items.unwrap_or_default(),
            deselected_cell_indexes: deselected_cell_indexes.unwrap_or_default(),
            selected_cell_indexes: selected_cell_indexes.unwrap_or_default(),
        }
    }

    pub fn deselected_indexes(&self) -> &[IndexPath] {
        &self.deselected_indexes
    }

    pub fn selected_indexes(&self) -> &[IndexPath] {
        &self.selected_indexes
    }

    pub fn deselected_items(&self) -> &[Option<Rc<M>>] {
        &self.deselected_items
    }

    pub fn selected_items(&self) -> &[Option<Rc<M>>] {
        &self.selected_items
    }

    pub fn deselected_cell_indexes(&self) -> &[CellIndex] {
        &self.deselected_cell_indexes
    }

    pub fn selected_cell_indexes(&self) -> &[CellIndex] {
        &self.selected_cell_indexes
    }

    /// The untyped arguments.
    pub fn untyped(&self) -> TreeDataGridSelectionChangedEventArgs {
        TreeDataGridSelectionChangedEventArgs::new(
            Some(self.deselected_indexes.clone()),
            Some(self.selected_indexes.clone()),
            Some(self.deselected_items.iter().map(box_model).collect()),
            Some(self.selected_items.iter().map(box_model).collect()),
            Some(self.deselected_cell_indexes.clone()),
            Some(self.selected_cell_indexes.clone()),
        )
    }
}
