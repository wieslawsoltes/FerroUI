//! `Selection/TreeSelectionModelSourceResetEventArgs.cs`.

use crate::IndexPath;

/// Holds data for the source reset event of a tree selection model.
#[derive(Clone, Debug)]
pub struct TreeSelectionModelSourceResetEventArgs {
    parent_index: IndexPath,
}

impl TreeSelectionModelSourceResetEventArgs {
    pub fn new(parent_index: IndexPath) -> Self {
        Self { parent_index }
    }

    /// Gets the index of the parent of the collection that was reset.
    pub fn parent_index(&self) -> &IndexPath {
        &self.parent_index
    }
}
