//! `CellIndex.cs`.

use crate::IndexPath;

/// A cell of a TreeDataGrid source: the index of its column in the source's columns and the
/// hierarchical index of its row's model in the data.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CellIndex {
    pub column_index: i32,
    pub row_index: IndexPath,
}

impl CellIndex {
    pub fn new(column_index: i32, row_index: impl Into<IndexPath>) -> Self {
        Self { column_index, row_index: row_index.into() }
    }
}
