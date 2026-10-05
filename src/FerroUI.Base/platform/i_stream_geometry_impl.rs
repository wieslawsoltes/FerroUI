use crate::platform::{IGeometryImpl, IStreamGeometryContextImpl};
use std::rc::Rc;

/// Defines the platform-specific interface for a stream geometry.
pub trait IStreamGeometryImpl: IGeometryImpl {
    /// Clones the geometry.
    fn clone_geometry(&self) -> Rc<dyn IStreamGeometryImpl>;

    /// Opens the geometry to start defining it.
    ///
    /// Returns a context which can be used to define the geometry.
    fn open(&self) -> Box<dyn IStreamGeometryContextImpl>;
}
