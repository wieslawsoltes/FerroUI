//! `Selection/TreeSelectionModelIndexesChangedEventArgs.cs`.

use crate::IndexPath;

/// Holds data for the indexes changed event of a tree selection model.
#[derive(Clone, Debug)]
pub struct TreeSelectionModelIndexesChangedEventArgs {
    parent_index: IndexPath,
    start_index: i32,
    end_index: i32,
    delta: i32,
}

impl TreeSelectionModelIndexesChangedEventArgs {
    pub fn new(parent_index: IndexPath, start_index: i32, end_index: i32, delta: i32) -> Self {
        Self { parent_index, start_index, end_index, delta }
    }

    /// Gets the index of the parent item.
    pub fn parent_index(&self) -> &IndexPath {
        &self.parent_index
    }

    /// Gets the inclusive start index of the range of indexes that changed.
    pub fn start_index(&self) -> i32 {
        self.start_index
    }

    /// Gets the exclusive end index of the range of indexes that changed.
    pub fn end_index(&self) -> i32 {
        self.end_index
    }

    /// Gets the delta of the change; i.e. the number of indexes added or removed.
    pub fn delta(&self) -> i32 {
        self.delta
    }
}
