use crate::media::{IPen, IntersectionResult};
use crate::platform::{IStreamGeometryImpl, ITransformedGeometryImpl};
use crate::{Matrix, Point, Rect};
use std::rc::Rc;

/// Defines the platform-specific interface for a geometry.
pub trait IGeometryImpl: 'static {
    /// The geometry's bounding rectangle.
    fn bounds(&self) -> Rect;

    /// The geometry's total length as if all its contours are placed in a
    /// straight line.
    fn contour_length(&self) -> f64;

    /// Gets the geometry's bounding rectangle with the specified pen.
    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect;

    /// Gets a geometry that is the shape defined by the stroke on the
    /// geometry produced by the specified pen.
    fn get_widened_geometry(&self, pen: &dyn IPen) -> Rc<dyn IGeometryImpl>;

    /// Indicates whether the geometry's fill contains the specified point.
    fn fill_contains(&self, point: Point) -> bool;

    /// Gets the relation between this geometry's fill and another one's.
    fn get_fill_intersection_result(&self, geometry: &dyn IGeometryImpl) -> IntersectionResult;

    /// Intersects the geometry with another geometry.
    ///
    /// Returns a new geometry object representing the intersection or `None`
    /// when the operation failed.
    fn intersect(&self, geometry: &dyn IGeometryImpl) -> Option<Rc<dyn IGeometryImpl>>;

    /// Indicates whether the geometry's stroke contains the specified point.
    fn stroke_contains(&self, pen: Option<&dyn IPen>, point: Point) -> bool;

    /// Makes a clone of the geometry with the specified transform.
    fn with_transform(&self, transform: Matrix) -> Rc<dyn ITransformedGeometryImpl>;

    /// Attempts to get the corresponding point at the specified distance.
    fn try_get_point_at_distance(&self, distance: f64) -> Option<Point>;

    /// Attempts to get the corresponding point and tangent from the
    /// specified distance along the contour of the geometry.
    fn try_get_point_and_tangent_at_distance(&self, distance: f64) -> Option<(Point, Point)>;

    /// Attempts to get the corresponding path segment given by the two
    /// distances specified. Imagine it like snipping a part of the current
    /// geometry.
    ///
    /// When `start_on_begin_figure` is true, the resulting snipped path
    /// begins with a figure start.
    fn try_get_segment(
        &self,
        start_distance: f64,
        stop_distance: f64,
        start_on_begin_figure: bool,
    ) -> Option<Rc<dyn IGeometryImpl>>;

    /// Lets the backend recover its concrete type.
    fn as_any(&self) -> &dyn std::any::Any;

    /// The geometry viewed as [`ITransformedGeometryImpl`], when it is one.
    fn as_transformed_geometry(&self) -> Option<&dyn ITransformedGeometryImpl> {
        None
    }

    /// The geometry viewed as [`IStreamGeometryImpl`], when it is one.
    fn as_stream_geometry(&self) -> Option<&dyn IStreamGeometryImpl> {
        None
    }
}
