use super::IPointerDevice;

/// Represents a mouse device.
pub trait IMouseDevice: IPointerDevice {}

impl dyn IMouseDevice {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
