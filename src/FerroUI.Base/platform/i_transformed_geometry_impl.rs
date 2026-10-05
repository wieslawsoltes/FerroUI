use crate::platform::IGeometryImpl;
use crate::Matrix;
use std::rc::Rc;

/// Represents a geometry with a transform applied.
///
/// A transformed geometry transforms its source geometry and exposes the
/// result through the members of [`IGeometryImpl`].
pub trait ITransformedGeometryImpl: IGeometryImpl {
    /// The source geometry that the transform is applied to.
    fn source_geometry(&self) -> Rc<dyn IGeometryImpl>;

    /// The applied transform.
    fn transform(&self) -> Matrix;
}
