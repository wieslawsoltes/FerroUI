use super::raw::IRawInputEventArgs;
use super::IPointerDevice;
use std::any::Any;

/// Represents an input device: the object that turns the raw input events
/// of the platform into routed events.
pub trait IInputDevice: std::any::Any {
    /// Processes raw event. Is called after preprocessing by the input
    /// manager.
    fn process_raw_event(&self, ev: &dyn IRawInputEventArgs);


    /// The device as a pointer device, if it is one.
    fn as_pointer_device(&self) -> Option<&dyn IPointerDevice> {
        None
    }
}

impl dyn IInputDevice {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
