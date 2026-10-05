use crate::media::{Geometry, ImmediateDrawingContext, IntersectionResult};
use crate::{Point, Rect, Ref};

/// Represents a custom draw operation in the low-level scene graph.
pub trait ICustomDrawOperation: 'static {
    /// The bounds of the visible content in the node in global coordinates.
    fn bounds(&self) -> Rect;

    /// Hit tests the node against a point, in global coordinates.
    ///
    /// The implementation is responsible for checking transforms and
    /// geometry clips if necessary.
    fn hit_test(&self, p: Point) -> bool;

    /// Hit tests the node against a geometry.
    fn hit_test_geometry(&self, _geometry: &Ref<Geometry>) -> IntersectionResult {
        IntersectionResult::Empty
    }

    /// Renders the node to a drawing context.
    fn render(&self, context: &mut ImmediateDrawingContext<'_>);

    /// Value equality between operations (C# `IEquatable<ICustomDrawOperation>`).
    fn equals(&self, other: &dyn ICustomDrawOperation) -> bool;

    /// Releases the operation's resources.
    fn dispose(&self);
}
