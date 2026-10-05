use super::{DragDropEffects, IDataTransfer, IKeyModifiersEventArgs, KeyModifiers};
use crate::interactivity::{Interactive, RoutedEvent, RoutedEventArgs};
use crate::{ferro_routed_event_args, Point, Ref, Visual};
use std::cell::Cell;
use std::rc::Rc;

/// Provides data for the drag-and-drop events.
#[derive(Clone)]
pub struct DragEventArgs {
    base: RoutedEventArgs,
    target: Ref<Interactive>,
    target_location: Point,
    drag_effects: Rc<Cell<DragDropEffects>>,
    data_transfer: Rc<dyn IDataTransfer>,
    key_modifiers: KeyModifiers,
}

ferro_routed_event_args!(DragEventArgs: RoutedEventArgs);

impl DragEventArgs {
    /// Creates drag event args. `target_location` is the location of the
    /// pointer in the coordinates of `target`.
    pub fn new(
        routed_event: Option<&RoutedEvent<DragEventArgs>>,
        data_transfer: Rc<dyn IDataTransfer>,
        target: Ref<Interactive>,
        target_location: Point,
        key_modifiers: KeyModifiers,
    ) -> Self {
        let base = RoutedEventArgs::new();
        base.set_routed_event(routed_event);
        Self {
            base,
            target,
            target_location,
            drag_effects: Rc::new(Cell::new(DragDropEffects::NONE)),
            data_transfer,
            key_modifiers,
        }
    }

    /// The effects of the operation. A handler sets them to what the
    /// target accepts.
    #[inline]
    pub fn drag_effects(&self) -> DragDropEffects {
        self.drag_effects.get()
    }

    /// Sets the effects of the operation.
    #[inline]
    pub fn set_drag_effects(&self, value: DragDropEffects) {
        self.drag_effects.set(value)
    }

    /// The data being dragged.
    #[inline]
    pub fn data_transfer(&self) -> &Rc<dyn IDataTransfer> {
        &self.data_transfer
    }

    /// The key modifiers held down during the event.
    #[inline]
    pub fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }

    /// Gets the position of the pointer relative to a visual.
    pub fn get_position(&self, relative_to: &Visual) -> Point {
        self.target.translate_point(self.target_location, relative_to).unwrap_or(Point::new(0.0, 0.0))
    }
}

impl IKeyModifiersEventArgs for DragEventArgs {
    fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }
}
