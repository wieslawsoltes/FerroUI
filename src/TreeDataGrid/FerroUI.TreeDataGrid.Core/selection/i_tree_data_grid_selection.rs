//! `Selection/ITreeDataGridSelection.cs`: a selection whose source the grid assigns.

use super::i_tree_selection_model::{ITreeSelectionModel, UntypedModels};
use std::any::Any;
use std::rc::Rc;

/// A selection of a tree data grid: the grid assigns the items of its source to it.
pub trait ITreeDataGridSelection: Any {
    /// Gets the source of the selection.
    fn source(&self) -> Option<UntypedModels>;

    /// Sets the source of the selection.
    fn set_source(&self, value: Option<UntypedModels>);

    /// The selection of rows of the selection (`as ITreeSelectionModel`, or the row selection of a
    /// cell selection), which a move of rows keeps on the moved rows.
    fn row_selection_model(&self) -> Option<Rc<dyn ITreeSelectionModel>> {
        None
    }
}

/// The selection as the concrete selection type `T`.
pub fn downcast_selection<T: 'static>(selection: &Rc<dyn ITreeDataGridSelection>) -> Option<Rc<T>> {
    let any: Rc<dyn Any> = selection.clone();
    any.downcast::<T>().ok()
}
