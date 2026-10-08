use crate::platform::IGeometryImpl;
use std::sync::Arc;

/// A geometry as referenced by render data: something that resolves to a
/// platform geometry when the data is replayed.
pub trait IRenderDataGeometry: 'static {
    /// The platform geometry to draw, if there is one right now.
    fn geometry_impl(&self) -> Option<Arc<dyn IGeometryImpl>>;
}
