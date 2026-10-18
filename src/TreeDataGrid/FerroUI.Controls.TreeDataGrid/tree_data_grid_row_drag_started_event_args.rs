//! `TreeDataGridRowDragStartedEventArgs.cs`: the arguments of the `RowDragStarted` event.

use crate::TreeDataGrid;
use ferroui_base::ferro_routed_event_args;
use ferroui_base::input::DragDropEffects;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::BoxedValue;
use std::cell::Cell;
use std::rc::Rc;

/// Provides data for the `RowDragStarted` event of the grid (`TreeDataGridRowDragStartedEventArgs`).
#[derive(Clone)]
pub struct TreeDataGridRowDragStartedEventArgs {
    base: RoutedEventArgs,
    models: Rc<[Option<BoxedValue>]>,
    // Copies of the arguments share the effects, as the reference's single object does.
    allowed_effects: Rc<Cell<DragDropEffects>>,
}

ferro_routed_event_args!(TreeDataGridRowDragStartedEventArgs: RoutedEventArgs);

impl TreeDataGridRowDragStartedEventArgs {
    /// The arguments of a drag of the rows of `models`.
    pub fn new(models: Vec<Option<BoxedValue>>) -> Self {
        Self {
            base: RoutedEventArgs::with_event(TreeDataGrid::row_drag_started_event()),
            models: models.into(),
            allowed_effects: Rc::new(Cell::new(DragDropEffects::NONE)),
        }
    }

    /// The effects the drag allows.
    pub fn allowed_effects(&self) -> DragDropEffects {
        self.allowed_effects.get()
    }

    /// Sets [`allowed_effects`](Self::allowed_effects).
    pub fn set_allowed_effects(&self, value: DragDropEffects) {
        self.allowed_effects.set(value);
    }

    /// The models of the dragged rows.
    pub fn models(&self) -> &[Option<BoxedValue>] {
        &self.models
    }
}
