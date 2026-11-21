use crate::input::PlatformWheelDelta;
use super::raw_input_event_args::raw_input_event_args;
use super::{RawPointerEventArgs, RawPointerEventType};
use crate::input::{IInputDevice, IInputRoot, RawInputModifiers};
use crate::{Point, Vector};
use std::rc::Rc;

/// A raw mouse wheel event.
pub struct RawMouseWheelEventArgs {
    base: RawPointerEventArgs,
    delta: Vector,
    platform_delta: Option<PlatformWheelDelta>,
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
            platform_delta: None,
        }
    }

    /// The amount the wheel was scrolled by.
    #[inline]
    pub fn delta(&self) -> Vector {
        self.delta
    }

    /// Original platform wheel data, when the backend supplies it.
    pub fn platform_delta(&self) -> Option<PlatformWheelDelta> {
        self.platform_delta
    }

    /// Adds original platform wheel data without changing the normalized delta.
    pub fn with_platform_delta(mut self, delta: PlatformWheelDelta) -> Self {
        self.platform_delta = Some(delta);
        self
    }

}
