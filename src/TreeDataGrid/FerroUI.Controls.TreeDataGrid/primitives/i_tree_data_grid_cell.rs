//! `Primitives/ITreeDataGridCell.cs`: what an expander cell needs of the cell it hosts.

use super::TreeDataGridElementFactory;
use crate::models::CellRef;
use crate::selection::ITreeDataGridSelectionInteraction;
use std::rc::Rc;

/// A cell control that can be bound to a cell model (`ITreeDataGridCell`). Implemented by
/// [`TreeDataGridCell`](super::TreeDataGridCell); a control is viewed as one with
/// [`Control::cast`](ferroui_base::Ref::cast) to `TreeDataGridCell`.
pub trait ITreeDataGridCell {
    fn column_index(&self) -> i32;

    fn realize(
        &self,
        factory: &Rc<TreeDataGridElementFactory>,
        selection: Option<&dyn ITreeDataGridSelectionInteraction>,
        model: &CellRef,
        column_index: i32,
        row_index: i32,
    );

    fn unrealize(&self);
}
