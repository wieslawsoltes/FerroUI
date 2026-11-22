#![allow(deprecated)]

use crate::platform::{IGeometryImpl, IStreamGeometryContextImpl};
use std::sync::Arc;

/// Defines the platform-specific interface for a stream geometry.
#[deprecated(note = "This interface is unused and will be removed.")]
pub trait IStreamGeometryImpl: IGeometryImpl {
    /// Clones the geometry.
    #[deprecated(note = "This method is unused and will be removed.")]
    fn clone_geometry(&self) -> Arc<dyn IStreamGeometryImpl>;

    /// Opens the geometry to start defining it.
    ///
    /// Returns a context which can be used to define the geometry.
    #[deprecated(note = "This method is unused and will be removed.")]
    fn open(&self) -> Box<dyn IStreamGeometryContextImpl>;
}

impl dyn IStreamGeometryImpl {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
