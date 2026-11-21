use std::any::Any;

/// Represents a platform implementation of a mouse cursor.
pub trait ICursorImpl: std::any::Any {
    /// Releases the platform resources of the cursor.
    fn dispose(&self);

}

impl dyn ICursorImpl {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
