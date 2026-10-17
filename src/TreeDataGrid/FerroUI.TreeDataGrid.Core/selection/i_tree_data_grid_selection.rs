//! `Selection/ITreeDataGridSelection.cs`: a selection whose source the grid assigns.

use super::i_tree_selection_model::UntypedModels;

/// A selection of a tree data grid: the grid assigns the items of its source to it.
pub trait ITreeDataGridSelection {
    /// Gets the source of the selection.
    fn source(&self) -> Option<UntypedModels>;

    /// Sets the source of the selection.
    fn set_source(&self, value: Option<UntypedModels>);
}
