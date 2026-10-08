use crate::media::{IPen, IntersectionResult};
use crate::platform::{IGeometryImpl, ITransformedGeometryImpl};
use crate::{Matrix, Point, Rect};
use std::rc::Rc;

/// An immutable [`IGeometryImpl`] that wraps a built platform geometry and exposes only
/// its read-only surface.
///
/// Unlike a stream geometry this cannot be re-opened or mutated (it is not a stream
/// geometry: [`IGeometryImpl::as_stream_geometry`] answers `None`, and
/// [`IGeometryImpl::as_any`] gives the wrapper, so it cannot be down-cast and re-opened),
/// which makes a single instance safe to cache and share: for example the glyph outline
/// returned by `GlyphTypeface::get_glyph_outline`. It is also not an object of the
/// property system, so it carries none of the styling overhead.
pub(crate) struct ImmutableGeometryImpl {
    inner: Rc<dyn IGeometryImpl>,
}

impl ImmutableGeometryImpl {
    pub(crate) fn new(inner: Rc<dyn IGeometryImpl>) -> ImmutableGeometryImpl {
        ImmutableGeometryImpl { inner }
    }
}

impl IGeometryImpl for ImmutableGeometryImpl {
    fn bounds(&self) -> Rect {
        self.inner.bounds()
    }

    fn contour_length(&self) -> f64 {
        self.inner.contour_length()
    }

    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect {
        self.inner.get_render_bounds(pen)
    }

    fn get_widened_geometry(&self, pen: &dyn IPen) -> Rc<dyn IGeometryImpl> {
        self.inner.get_widened_geometry(pen)
    }

    fn fill_contains(&self, point: Point) -> bool {
        self.inner.fill_contains(point)
    }

    fn intersect(&self, geometry: &dyn IGeometryImpl) -> Option<Rc<dyn IGeometryImpl>> {
        self.inner.intersect(geometry)
    }

    fn get_fill_intersection_result(&self, geometry: &dyn IGeometryImpl) -> IntersectionResult {
        self.inner.get_fill_intersection_result(geometry)
    }

    fn stroke_contains(&self, pen: Option<&dyn IPen>, point: Point) -> bool {
        self.inner.stroke_contains(pen, point)
    }

    fn with_transform(&self, transform: Matrix) -> Rc<dyn ITransformedGeometryImpl> {
        self.inner.with_transform(transform)
    }

    fn try_get_point_at_distance(&self, distance: f64) -> Option<Point> {
        self.inner.try_get_point_at_distance(distance)
    }

    fn try_get_point_and_tangent_at_distance(&self, distance: f64) -> Option<(Point, Point)> {
        self.inner.try_get_point_and_tangent_at_distance(distance)
    }

    fn try_get_segment(
        &self,
        start_distance: f64,
        stop_distance: f64,
        start_on_begin_figure: bool,
    ) -> Option<Rc<dyn IGeometryImpl>> {
        self.inner.try_get_segment(start_distance, stop_distance, start_on_begin_figure)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Not upstream tests: the class is only covered by render tests there.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::geometry::tests::with_mock_render_interface;
    use crate::platform::IPlatformRenderInterface;

    #[test]
    fn the_wrapper_of_a_stream_geometry_cannot_be_reopened() {
        with_mock_render_interface(|factory| {
            let stream = factory.create_stream_geometry();
            assert!(stream.as_stream_geometry().is_some());

            let immutable = ImmutableGeometryImpl::new(stream);

            assert!(immutable.as_stream_geometry().is_none());
            assert!(immutable.as_any().downcast_ref::<ImmutableGeometryImpl>().is_some());
        });
    }

    #[test]
    fn the_wrapper_answers_as_the_wrapped_geometry() {
        with_mock_render_interface(|factory| {
            let stream = factory.create_stream_geometry();
            let bounds = stream.bounds();
            let contour_length = stream.contour_length();

            let immutable = ImmutableGeometryImpl::new(stream);

            assert_eq!(bounds, immutable.bounds());
            assert_eq!(contour_length, immutable.contour_length());
            assert!(!immutable.fill_contains(Point::new(1.0, 1.0)));
            assert!(immutable.try_get_point_at_distance(1.0).is_none());
        });
    }
}
