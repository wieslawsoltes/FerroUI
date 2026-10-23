//! The factories of the WinUI shapes, geometries, view boxes and geometric
//! clips (`Compositor.Create*` of `Microsoft.UI.Composition`).

use super::{
    CompositionContainerShape, CompositionEllipseGeometry, CompositionGeometry, CompositionLineGeometry,
    CompositionPath, CompositionPathGeometry, CompositionRectangleGeometry, CompositionRoundedRectangleGeometry,
    CompositionSpriteShape, CompositionViewBox, Compositor, GeometricClip, ShapeVisual,
};
use std::rc::Rc;

impl Compositor {
    /// `CreateShapeVisual`: a visual that draws shapes.
    pub fn create_shape_visual(&self) -> ShapeVisual {
        ShapeVisual::new(&self.this_handle())
    }

    /// `CreateSpriteShape`: a shape without a geometry.
    pub fn create_sprite_shape(&self) -> CompositionSpriteShape {
        CompositionSpriteShape::new(&self.this_handle())
    }

    /// `CreateSpriteShape(geometry)`: a shape that draws `geometry`.
    pub fn create_sprite_shape_with_geometry(&self, geometry: Option<Rc<CompositionGeometry>>) -> CompositionSpriteShape {
        CompositionSpriteShape::with_geometry(&self.this_handle(), geometry)
    }

    /// `CreateContainerShape`: a shape that groups shapes.
    pub fn create_container_shape(&self) -> CompositionContainerShape {
        CompositionContainerShape::new(&self.this_handle())
    }

    /// `CreateRectangleGeometry`: an empty rectangle at the origin.
    pub fn create_rectangle_geometry(&self) -> CompositionRectangleGeometry {
        CompositionRectangleGeometry::new(&self.this_handle())
    }

    /// `CreateRoundedRectangleGeometry`: an empty rounded rectangle at the
    /// origin.
    pub fn create_rounded_rectangle_geometry(&self) -> CompositionRoundedRectangleGeometry {
        CompositionRoundedRectangleGeometry::new(&self.this_handle())
    }

    /// `CreateEllipseGeometry`: an ellipse with zero radii at the origin.
    pub fn create_ellipse_geometry(&self) -> CompositionEllipseGeometry {
        CompositionEllipseGeometry::new(&self.this_handle())
    }

    /// `CreateLineGeometry`: a line from the origin to the origin.
    pub fn create_line_geometry(&self) -> CompositionLineGeometry {
        CompositionLineGeometry::new(&self.this_handle())
    }

    /// `CreatePathGeometry`: a geometry without a path.
    pub fn create_path_geometry(&self) -> CompositionPathGeometry {
        CompositionPathGeometry::new(&self.this_handle())
    }

    /// `CreatePathGeometry(path)`: a geometry that shows `path`.
    pub fn create_path_geometry_with_path(&self, path: Option<CompositionPath>) -> CompositionPathGeometry {
        CompositionPathGeometry::with_path(&self.this_handle(), path)
    }

    /// `CreateViewBox`: a view box with a zero size.
    pub fn create_view_box(&self) -> Rc<CompositionViewBox> {
        CompositionViewBox::new(&self.this_handle())
    }

    /// `CreateGeometricClip`: a clip without a geometry.
    pub fn create_geometric_clip(&self) -> GeometricClip {
        GeometricClip::new(&self.this_handle())
    }

    /// `CreateGeometricClip(geometry)`: a clip to the inside of `geometry`.
    pub fn create_geometric_clip_with_geometry(&self, geometry: Option<Rc<CompositionGeometry>>) -> GeometricClip {
        GeometricClip::with_geometry(&self.this_handle(), geometry)
    }
}
