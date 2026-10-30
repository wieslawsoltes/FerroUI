//! `TreeDataGridRowDragStartedEventArgs.cs`: the arguments of the `RowDragStarted` event.

use crate::TreeDataGrid;
use ferroui_base::ferro_routed_event_args;
use ferroui_base::input::{DataTransfer, DragDropEffects, KeyModifiers};
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
    data_transfer: Rc<DataTransfer>,
    key_modifiers: KeyModifiers,
}

ferro_routed_event_args!(TreeDataGridRowDragStartedEventArgs: RoutedEventArgs);

impl TreeDataGridRowDragStartedEventArgs {
    /// The arguments of a drag of the rows of `models`.
    pub fn new(models: Vec<Option<BoxedValue>>) -> Self {
        Self::with_modifiers(models, KeyModifiers::NONE)
    }

    /// Creates arguments with the modifiers captured by the original pointer press.
    /// Applications can use explicit gestures to distinguish a row move from a reference drag.
    pub fn with_modifiers(models: Vec<Option<BoxedValue>>, key_modifiers: KeyModifiers) -> Self {
        Self {
            base: RoutedEventArgs::with_event(TreeDataGrid::row_drag_started_event()),
            models: models.into(),
            allowed_effects: Rc::new(Cell::new(DragDropEffects::NONE)),
            data_transfer: DataTransfer::new(),
            key_modifiers,
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

    /// Application payloads contributed to the native drag. The grid retains its own row
    /// identity payload, so adding data does not disable normal row reordering. Clones of
    /// these event arguments share this collection, just as they share allowed effects.
    /// The internal row token is set on the first item before the native drag starts so
    /// single-item platform backends retain both row identity and application formats.
    pub fn data_transfer(&self) -> &Rc<DataTransfer> {
        &self.data_transfer
    }

    /// Modifiers at the pointer press that initiated the drag, before selection can change.
    pub fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }
}
