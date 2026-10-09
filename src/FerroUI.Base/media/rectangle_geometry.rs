use crate::media::{Geometry, GeometryImpl, GeometryBuilder, StreamGeometryContext};
use crate::platform::{self, IGeometryImpl};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Rect, Ref,
    StyledProperty,
};
use std::sync::Arc;

/// Represents the geometry of a rectangle.
#[repr(C)]
pub struct RectangleGeometry {
    base: Geometry,
}

ferro_class!(RectangleGeometry: Geometry);
crate::ferro_class_info!(RectangleGeometry { new: RectangleGeometry::new });

crate::ferro_impl_classes!(RectangleGeometry: FerroObjectImpl);

impl GeometryImpl for RectangleGeometry {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        RectangleGeometry::with_rect_and_radii(this.rect(), this.radius_x(), this.radius_y()).upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Arc<dyn IGeometryImpl>> {
        let radius_x = this.radius_x();
        let radius_y = this.radius_y();
        let factory = platform::render_interface();

        if radius_x == 0.0 && radius_y == 0.0 {
            // Optimization when there are no corner radii
            Some(factory.create_rectangle_geometry(this.rect()))
        } else {
            let geometry = factory.create_stream_geometry();
            {
                let mut ctx = StreamGeometryContext::new(geometry.open());
                GeometryBuilder::draw_rounded_corners_rectangle(&mut ctx, this.rect(), radius_x, radius_y);
            }
            Some(geometry)
        }
    }
}

crate::ferro_properties! { impl RectangleGeometry {
    ferro_property!(pub fn radius_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<RectangleGeometry, _>("RadiusX", 0.0)
    });

    ferro_property!(pub fn radius_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<RectangleGeometry, _>("RadiusY", 0.0)
    });

    ferro_property!(pub fn rect_property() -> StyledProperty<Rect> {
        FerroProperty::register::<RectangleGeometry, _>("Rect", Rect::default())
    });
} }

impl RectangleGeometry {
    fn static_constructor() {
        Geometry::affects_geometry(&[
            Self::radius_x_property().as_property(),
            Self::radius_y_property().as_property(),
            Self::rect_property().as_property(),
        ]);
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Geometry::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a rectangle geometry with the given bounds.
    pub fn with_rect(rect: Rect) -> Ref<Self> {
        let result = Self::new();
        result.set_rect(rect);
        result
    }

    /// Creates a rectangle geometry with the given bounds and corner radii.
    pub fn with_rect_and_radii(rect: Rect, radius_x: f64, radius_y: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_rect(rect);
        result.set_radius_x(radius_x);
        result.set_radius_y(radius_y);
        result
    }

    /// The x-axis radius of the ellipse that is used to round the corners of the rectangle. Corner radii are represented by an ellipse so this is the horizontal axis width of the ellipse.
    pub fn radius_x(&self) -> f64 {
        self.get_value(Self::radius_x_property())
    }

    pub fn set_radius_x(&self, value: f64) {
        self.set_value(Self::radius_x_property(), value)
    }

    /// The y-axis radius of the ellipse that is used to round the corners of the rectangle. Corner radii are represented by an ellipse so this is the vertical axis height of the ellipse.
    pub fn radius_y(&self) -> f64 {
        self.get_value(Self::radius_y_property())
    }

    pub fn set_radius_y(&self, value: f64) {
        self.set_value(Self::radius_y_property(), value)
    }

    /// The bounds of the rectangle.
    pub fn rect(&self) -> Rect {
        self.get_value(Self::rect_property())
    }

    pub fn set_rect(&self, value: Rect) {
        self.set_value(Self::rect_property(), value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::geometry::tests::with_mock_render_interface;
    use crate::media::RotateTransform;

    #[test]
    fn rectangle_with_transform_can_be_changed() {
        with_mock_render_interface(|_| {
            let target = RectangleGeometry::new();
            target.set_rect(Rect::new(0.0, 0.0, 100.0, 100.0));
            target.set_transform(RotateTransform::with_angle(45.0));

            target.set_rect(Rect::new(50.0, 50.0, 150.0, 150.0));

            assert!(target.platform_impl().unwrap().as_transformed_geometry().is_some());
        });
    }

    #[test]
    fn rounded_rectangle_is_drawn_into_a_stream_geometry() {
        with_mock_render_interface(|factory| {
            let target = RectangleGeometry::with_rect_and_radii(Rect::new(0.0, 0.0, 100.0, 50.0), 10.0, 5.0);
            assert!(target.platform_impl().is_some());
            assert_eq!(vec!["stream"], *factory.created.borrow());
            assert_eq!(
                vec![
                    "begin 10, 0 true",
                    "line 90, 0 true",
                    "arc 100, 5 10, 5 0 false Clockwise true",
                    "line 100, 45 true",
                    "arc 90, 50 10, 5 0 false Clockwise true",
                    "line 10, 50 true",
                    "arc 0, 45 10, 5 0 false Clockwise true",
                    "line 0, 5 true",
                    "arc 10, 0 10, 5 0 false Clockwise true",
                    "end true",
                    "dispose",
                ],
                *factory.streams.borrow()[0].lock().unwrap()
            );

            let clone = target.clone_geometry().cast::<RectangleGeometry>().unwrap();
            assert_eq!((Rect::new(0.0, 0.0, 100.0, 50.0), 10.0, 5.0), (clone.rect(), clone.radius_x(), clone.radius_y()));

            target.set_radius_x(0.0);
            target.set_radius_y(0.0);
            assert!(target.platform_impl().is_some());
            assert_eq!(vec!["stream", "rectangle 0, 0, 100, 50"], *factory.created.borrow());
        });
    }
}
