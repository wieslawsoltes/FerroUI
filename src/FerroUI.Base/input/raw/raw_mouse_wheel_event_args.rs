use super::raw_input_event_args::raw_input_event_args;
use super::{RawPointerEventArgs, RawPointerEventType};
use crate::input::{IInputDevice, IInputRoot, RawInputModifiers};
use crate::{Point, Vector};
use std::rc::Rc;

/// A raw mouse wheel event.
pub struct RawMouseWheelEventArgs {
    base: RawPointerEventArgs,
    delta: Vector,
}

raw_input_event_args!(RawMouseWheelEventArgs: RawPointerEventArgs);

impl RawMouseWheelEventArgs {
    /// Creates raw mouse wheel event args.
    pub fn new(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        position: Point,
        delta: Vector,
        input_modifiers: RawInputModifiers,
    ) -> Self {
        Self {
            base: RawPointerEventArgs::new(device, timestamp, root, RawPointerEventType::Wheel, position, input_modifiers),
            delta,
        }
    }

    /// The amount the wheel was scrolled by.
    #[inline]
    pub fn delta(&self) -> Vector {
        self.delta
    }
}
