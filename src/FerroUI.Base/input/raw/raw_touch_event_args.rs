use super::raw_input_event_args::raw_input_event_args;
use super::{RawPointerEventArgs, RawPointerEventType, RawPointerPoint};
use crate::input::{IInputDevice, IInputRoot, RawInputModifiers};
use crate::Point;
use std::rc::Rc;

/// A raw touch event.
pub struct RawTouchEventArgs {
    base: RawPointerEventArgs,
}

raw_input_event_args!(RawTouchEventArgs: RawPointerEventArgs);

impl RawTouchEventArgs {
    /// Creates raw touch event args for a position in client coordinates.
    pub fn new(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        type_: RawPointerEventType,
        position: Point,
        input_modifiers: RawInputModifiers,
        raw_pointer_id: i64,
    ) -> Self {
        let base = RawPointerEventArgs::new(device, timestamp, root, type_, position, input_modifiers);
        base.set_raw_pointer_id(raw_pointer_id);
        Self { base }
    }

    /// Creates raw touch event args for a raw pointer point.
    pub fn with_point(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        type_: RawPointerEventType,
        point: RawPointerPoint,
        input_modifiers: RawInputModifiers,
        raw_pointer_id: i64,
    ) -> Self {
        let base = RawPointerEventArgs::with_point(device, timestamp, root, type_, point, input_modifiers);
        base.set_raw_pointer_id(raw_pointer_id);
        Self { base }
    }
}
