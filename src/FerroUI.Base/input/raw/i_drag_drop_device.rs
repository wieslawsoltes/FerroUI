use crate::input::IInputDevice;

/// The input device that turns the raw drag events of the platform into
/// the routed drag-and-drop events.
///
/// This is an implementation detail of the platform backends.
pub trait IDragDropDevice: IInputDevice {}

impl dyn IDragDropDevice {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
