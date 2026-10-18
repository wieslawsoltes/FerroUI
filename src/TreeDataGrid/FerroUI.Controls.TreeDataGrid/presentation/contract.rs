//! `Presentation/TreeDataGridPresentation.cs`: what the control reads of a source, the columns it
//! lays out, the rows that realize cells, the selection and the sorting.

use crate::models::{ColumnRef, IColumns, ITreeDataGridRows};
use crate::selection::ITreeDataGridSelectionInteraction;
use crate::{TreeDataGridRowDropPosition, TreeDataGridSelectionChangedEventArgs, TreeDataGridSelectionMode};
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::input::DragDropEffects;
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::models::ListSortDirection;
use ferroui_treedatagrid_core::IndexPath;
use std::any::Any;
use std::rc::Rc;

/// The presentation of a source to the control (`TreeDataGridPresentation`).
pub trait TreeDataGridPresentation: INotifyPropertyChanged + Any {
    /// The object the presentation presents (the source), to tell whether two presentations show
    /// the same source.
    fn source_identity(&self) -> Rc<dyn Any>;
    fn can_select_multiple(&self) -> bool;
    fn select(&self, index: &IndexPath, replace: bool);
    fn deselect(&self, index: &IndexPath);
    fn is_selected(&self, index: &IndexPath) -> bool;
    fn columns(&self) -> Rc<dyn IColumns>;
    fn rows(&self) -> Rc<dyn ITreeDataGridRows>;
    fn is_hierarchical(&self) -> bool;
    fn is_sorted(&self) -> bool;
    fn selection_interaction(&self) -> Option<Rc<dyn ITreeDataGridSelectionInteraction>>;
    fn selected_items(&self) -> Option<Vec<Option<BoxedValue>>>;
    fn selected_indexes(&self) -> Option<Vec<IndexPath>>;
    /// `Sorted`.
    fn add_sorted(&self, handler: Rc<dyn Fn()>) -> u64;
    fn remove_sorted(&self, token: u64);
    fn sort_by(&self, column: &ColumnRef, direction: ListSortDirection) -> bool;
    fn move_rows(&self, indexes: &[IndexPath], target: &IndexPath, position: TreeDataGridRowDropPosition, effects: DragDropEffects);
    fn dispose(&self);
    /// Stops following the source while the control is detached.
    fn suspend(&self) {}
    fn resume(&self) {}
    /// `NativeSelectionChanged`: the selection of a source whose selection the control does not
    /// interact with directly changed.
    fn add_native_selection_changed(&self, _handler: Rc<dyn Fn(&TreeDataGridSelectionChangedEventArgs)>) -> u64 {
        0
    }
    fn remove_native_selection_changed(&self, _token: u64) {}
    /// Applies the selection mode of the control (`ApplySelectionMode`).
    fn apply_selection_mode(&self, _mode: TreeDataGridSelectionMode) {}
    fn as_any(&self) -> &dyn Any;
}

/// A shared presentation.
pub type PresentationRef = Rc<dyn TreeDataGridPresentation>;
