//! `TreeDataGridRowDragEventArgs.cs`: where a dragged row is dropped, and the arguments of the
//! `RowDragOver` and `RowDrop` events.

use crate::primitives::TreeDataGridRow;
use ferroui_base::ferro_routed_event_args;
use ferroui_base::input::DragEventArgs;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::Ref;
use ferroui_treedatagrid_core::RowDropPosition;
use std::cell::Cell;
use std::rc::Rc;

/// Where dragged rows are dropped relative to the target row (`TreeDataGridRowDropPosition`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TreeDataGridRowDropPosition {
    #[default]
    None,
    Before,
    After,
    Inside,
}

impl From<TreeDataGridRowDropPosition> for RowDropPosition {
    fn from(value: TreeDataGridRowDropPosition) -> Self {
        match value {
            TreeDataGridRowDropPosition::None => RowDropPosition::None,
            TreeDataGridRowDropPosition::Before => RowDropPosition::Before,
            TreeDataGridRowDropPosition::After => RowDropPosition::After,
            TreeDataGridRowDropPosition::Inside => RowDropPosition::Inside,
        }
    }
}

impl From<RowDropPosition> for TreeDataGridRowDropPosition {
    fn from(value: RowDropPosition) -> Self {
        match value {
            RowDropPosition::None => TreeDataGridRowDropPosition::None,
            RowDropPosition::Before => TreeDataGridRowDropPosition::Before,
            RowDropPosition::After => TreeDataGridRowDropPosition::After,
            RowDropPosition::Inside => TreeDataGridRowDropPosition::Inside,
        }
    }
}

/// Provides data for the `RowDragOver` and `RowDrop` events of the grid (`TreeDataGridRowDragEventArgs`).
#[derive(Clone)]
pub struct TreeDataGridRowDragEventArgs {
    base: RoutedEventArgs,
    inner: DragEventArgs,
    target_row: Option<Ref<TreeDataGridRow>>,
    // Copies of the arguments share the position, as the reference's single object does.
    position: Rc<Cell<TreeDataGridRowDropPosition>>,
}

ferro_routed_event_args!(TreeDataGridRowDragEventArgs: RoutedEventArgs);

impl TreeDataGridRowDragEventArgs {
    /// The arguments of `routed_event` for a drag over `row`, described by `inner`.
    pub fn new(routed_event: &RoutedEvent<TreeDataGridRowDragEventArgs>, row: Option<Ref<TreeDataGridRow>>, inner: DragEventArgs) -> Self {
        Self {
            base: RoutedEventArgs::with_event(routed_event),
            inner,
            target_row: row,
            position: Rc::new(Cell::new(TreeDataGridRowDropPosition::None)),
        }
    }

    /// The arguments of the drag and drop operation.
    pub fn inner(&self) -> &DragEventArgs {
        &self.inner
    }

    /// The row being dragged over.
    pub fn target_row(&self) -> Option<&Ref<TreeDataGridRow>> {
        self.target_row.as_ref()
    }

    /// How the data should be dropped into the target row.
    ///
    /// For drag operations, the value of this property controls the adorner displayed when
    /// dragging. For drop operations, controls the final location of the drop.
    pub fn position(&self) -> TreeDataGridRowDropPosition {
        self.position.get()
    }

    /// Sets [`position`](Self::position).
    pub fn set_position(&self, value: TreeDataGridRowDropPosition) {
        self.position.set(value);
    }
}
