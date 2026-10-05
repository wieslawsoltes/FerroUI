use std::any::Any;

/// Represents a platform implementation of a mouse cursor.
pub trait ICursorImpl {
    /// Releases the platform resources of the cursor.
    fn dispose(&self);

    /// Lets the backend that created the cursor recover its concrete type
    /// (the reference backends cast the cursor they are handed back).
    fn as_any(&self) -> &dyn Any;
}
