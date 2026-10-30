use crate::media::FillRule;
use crate::Point;

/// An owned, backend-neutral Bezier path. Coordinates include the geometry's
/// transform; no sampling or polygon approximation is performed by this API.
#[derive(Clone, Debug, PartialEq)]
pub struct GeometryPath {
    pub fill_rule: FillRule,
    pub segments: Vec<GeometryPathSegment>,
}

/// Curve commands in a [`GeometryPath`]. An open figure remains open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeometryPathSegment {
    MoveTo(Point),
    LineTo(Point),
    QuadraticTo {
        control: Point,
        point: Point,
    },
    CubicTo {
        control1: Point,
        control2: Point,
        point: Point,
    },
    Close,
}

/// The independently rendered fill and stroke paths of a geometry.
///
/// Some geometries intentionally fill and stroke different paths (for example
/// a figure whose closing edge is not stroked). An absent path is empty, while
/// `IGeometryImpl::try_get_path_data` returning `None` means that the backend
/// does not support extraction. Consumers must preserve this distinction.
#[derive(Clone, Debug, PartialEq)]
pub struct GeometryPathData {
    pub fill: Option<GeometryPath>,
    pub stroke: Option<GeometryPath>,
}
