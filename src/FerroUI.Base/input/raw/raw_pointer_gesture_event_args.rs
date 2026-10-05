use super::raw_input_event_args::raw_input_event_args;
use super::{RawPointerEventArgs, RawPointerEventType};
use crate::input::{IInputDevice, IInputRoot, RawInputModifiers};
use crate::{Point, Vector};
use std::rc::Rc;

/// A raw touchpad gesture event (magnify, rotate or swipe).
pub struct RawPointerGestureEventArgs {
    base: RawPointerEventArgs,
    delta: Vector,
}

raw_input_event_args!(RawPointerGestureEventArgs: RawPointerEventArgs);

impl RawPointerGestureEventArgs {
    /// Creates raw pointer gesture event args.
    pub fn new(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        gesture_type: RawPointerEventType,
        position: Point,
        delta: Vector,
        input_modifiers: RawInputModifiers,
    ) -> Self {
        Self {
            base: RawPointerEventArgs::new(device, timestamp, root, gesture_type, position, input_modifiers),
            delta,
        }
    }

    /// The amount of the gesture.
    #[inline]
    pub fn delta(&self) -> Vector {
        self.delta
    }
}
