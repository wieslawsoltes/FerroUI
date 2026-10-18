//! `TreeDataGridRowEventArgs.cs`: the arguments of the events about a row of the grid
//! (`RowPrepared`, `RowClearing`).

use crate::primitives::TreeDataGridRow;
use ferroui_base::Ref;

/// Provides data for the row events of the grid (`TreeDataGridRowEventArgs`).
#[derive(Clone)]
pub struct TreeDataGridRowEventArgs {
    row: Ref<TreeDataGridRow>,
    row_index: i32,
}

impl TreeDataGridRowEventArgs {
    pub fn new(row: Ref<TreeDataGridRow>, row_index: i32) -> Self {
        Self { row, row_index }
    }

    /// The row.
    pub fn row(&self) -> &Ref<TreeDataGridRow> {
        &self.row
    }

    /// The index of the row.
    pub fn row_index(&self) -> i32 {
        self.row_index
    }
}
