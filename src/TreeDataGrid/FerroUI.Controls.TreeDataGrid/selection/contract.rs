//! `Selection/ITreeDataGridSelectionInteraction.cs`: how a selection handles the input of the
//! control.

use crate::TreeDataGrid;
use ferroui_base::input::{KeyEventArgs, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, TextInputEventArgs};
use ferroui_treedatagrid_core::models::RowRef;
use std::rc::Rc;

/// The interaction of a selection with the control (`ITreeDataGridSelectionInteraction`).
pub trait ITreeDataGridSelectionInteraction {
    /// `SelectionChanged`: the cells or rows shown as selected changed.
    fn add_selection_changed(&self, handler: Rc<dyn Fn()>) -> u64;
    fn remove_selection_changed(&self, token: u64);
    fn is_cell_selected(&self, _column_index: i32, _row_index: i32) -> bool {
        false
    }
    fn is_row_selected(&self, _row: &RowRef) -> bool {
        false
    }
    fn is_row_selected_at(&self, _row_index: i32) -> bool {
        false
    }
    fn on_key_down(&self, _sender: &TreeDataGrid, _e: &KeyEventArgs) {}
    fn on_preview_key_down(&self, _sender: &TreeDataGrid, _e: &KeyEventArgs) {}
    fn on_key_up(&self, _sender: &TreeDataGrid, _e: &KeyEventArgs) {}
    fn on_text_input(&self, _sender: &TreeDataGrid, _e: &TextInputEventArgs) {}
    fn on_pointer_pressed(&self, _sender: &TreeDataGrid, _e: &PointerPressedEventArgs) {}
    fn on_pointer_moved(&self, _sender: &TreeDataGrid, _e: &PointerEventArgs) {}
    fn on_pointer_released(&self, _sender: &TreeDataGrid, _e: &PointerReleasedEventArgs) {}
}
