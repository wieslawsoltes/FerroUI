use super::raw_input_event_args::raw_input_event_args;
use super::{IDragDropDevice, RawDragEventType, RawInputEventArgs};
use crate::input::{DragDropEffects, IDataTransfer, IInputRoot, KeyModifiers, RawInputModifiers};
use crate::Point;
use std::cell::Cell;
use std::rc::Rc;

/// A raw drag event: a drag-and-drop operation of the platform entering,
/// moving over, leaving or dropping on an input root.
///
/// This is an implementation detail of the platform backends.
pub struct RawDragEvent {
    base: RawInputEventArgs,
    location: Cell<Point>,
    data_transfer: Rc<dyn IDataTransfer>,
    effects: Cell<DragDropEffects>,
    type_: RawDragEventType,
    key_modifiers: KeyModifiers,
}

raw_input_event_args!(RawDragEvent: RawInputEventArgs);

impl RawDragEvent {
    /// Creates a raw drag event.
    pub fn new(
        input_device: Rc<dyn IDragDropDevice>,
        type_: RawDragEventType,
        root: Rc<dyn IInputRoot>,
        location: Point,
        data_transfer: Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: RawInputModifiers,
    ) -> Self {
        Self {
            base: RawInputEventArgs::new(input_device, 0, root),
            location: Cell::new(location),
            data_transfer,
            effects: Cell::new(effects),
            type_,
            key_modifiers: modifiers.to_key_modifiers(),
        }
    }

    /// The location of the pointer, in the coordinates of the root.
    #[inline]
    pub fn location(&self) -> Point {
        self.location.get()
    }

    /// Sets the location of the pointer.
    #[inline]
    pub fn set_location(&self, value: Point) {
        self.location.set(value)
    }

    /// The data being dragged.
    #[inline]
    pub fn data_transfer(&self) -> &Rc<dyn IDataTransfer> {
        &self.data_transfer
    }

    /// The effects of the operation: the allowed effects when the event is
    /// sent, the effects accepted by the target once it has been processed.
    #[inline]
    pub fn effects(&self) -> DragDropEffects {
        self.effects.get()
    }

    /// Sets the effects of the operation.
    #[inline]
    pub fn set_effects(&self, value: DragDropEffects) {
        self.effects.set(value)
    }

    /// The type of the event.
    #[inline]
    pub fn type_(&self) -> RawDragEventType {
        self.type_
    }

    /// The key modifiers held down during the event.
    #[inline]
    pub fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }
}
