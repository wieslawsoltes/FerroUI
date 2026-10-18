//! `TreeDataGridCellEventArgs.cs`: the arguments of the events about a cell of the grid
//! (`CellPrepared`, `CellClearing`, `CellValueChanged`).

use ferroui_base::Ref;
use ferroui_controls::Control;

/// Provides data for the cell events of the grid (`TreeDataGridCellEventArgs`).
#[derive(Clone)]
pub struct TreeDataGridCellEventArgs {
    cell: Ref<Control>,
    column_index: i32,
    row_index: i32,
}

impl TreeDataGridCellEventArgs {
    pub fn new(cell: Ref<Control>, column_index: i32, row_index: i32) -> Self {
        Self { cell, column_index, row_index }
    }

    /// The cell.
    pub fn cell(&self) -> &Ref<Control> {
        &self.cell
    }

    /// The index of the column of the cell.
    pub fn column_index(&self) -> i32 {
        self.column_index
    }

    /// The index of the row of the cell.
    pub fn row_index(&self) -> i32 {
        self.row_index
    }
}
