//! File selection and byte storage contracts without UI dependencies.
use ferrodock_model::core::SerializationResult;
use std::{future::Future, pin::Pin};

/// A local asynchronous layout file operation.
pub type LayoutFileFuture<T> = Pin<Box<dyn Future<Output = SerializationResult<T>>>>;

/// Selected layout contents and a display name, without a platform path assumption.
pub struct LayoutFileContents {
    /// Name displayed in operation feedback.
    pub name: String,
    /// Original file bytes for the standard serializer to decode.
    pub bytes: Vec<u8>,
}

/// Selects and reads/writes layout files using the platform's storage facilities.
pub trait ILayoutFileStorage {
    /// Returns None on cancellation; errors retain their original cause.
    fn open_layout(&self) -> LayoutFileFuture<Option<LayoutFileContents>>;
    /// Returns the saved name, or None on cancellation, after writing and flushing.
    fn save_layout(&self, bytes: Vec<u8>) -> LayoutFileFuture<Option<String>>;
}
