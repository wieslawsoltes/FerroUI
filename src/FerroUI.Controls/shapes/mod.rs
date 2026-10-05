//! Shapes: controls that draw a geometry with a fill and a stroke.

mod arc;
mod ellipse;
mod line;
mod path;
mod polygon;
mod polyline;
mod rectangle;
mod sector;
mod shape;

pub use arc::Arc;
pub use ellipse::Ellipse;
pub use line::Line;
pub use path::Path;
pub use polygon::Polygon;
pub use polyline::Polyline;
pub use rectangle::Rectangle;
pub use sector::Sector;
pub use shape::{Shape, ShapeImpl, ShapeImplExt, ShapeVTable};

#[cfg(test)]
mod ellipse_tests;
#[cfg(test)]
mod path_tests;
#[cfg(test)]
mod polygon_tests;
#[cfg(test)]
mod polyline_tests;
#[cfg(test)]
mod rectangle_tests;
#[cfg(test)]
mod shape_tests;
