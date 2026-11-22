use crate::platform::IGeometryImpl;
use crate::Matrix;
use std::sync::Arc;

/// Represents a geometry with a transform applied.
///
/// A transformed geometry transforms its source geometry and exposes the
/// result through the members of [`IGeometryImpl`].
pub trait ITransformedGeometryImpl: IGeometryImpl {
    /// The source geometry that the transform is applied to.
    fn source_geometry(&self) -> Arc<dyn IGeometryImpl>;

    /// The applied transform.
    fn transform(&self) -> Matrix;
}

impl dyn ITransformedGeometryImpl {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
