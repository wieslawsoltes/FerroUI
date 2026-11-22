use super::IPointerDevice;

/// Represents a pen/stylus device.
pub trait IPenDevice: IPointerDevice {}

impl dyn IPenDevice {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
