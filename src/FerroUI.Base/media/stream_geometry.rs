use crate::media::{FillRule, Geometry, GeometryImpl, PathMarkupParser, StreamGeometryContext, SweepDirection};
use crate::platform::{self, IGeometryContext, IGeometryImpl, IStreamGeometryBuilder};
use crate::utilities::FormatError;
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, Size};
use std::cell::RefCell;
use std::sync::Arc;

/// Represents the geometry of an arbitrarily complex shape.
#[repr(C)]
pub struct StreamGeometry {
    base: Geometry,
    impl_: RefCell<Option<Arc<dyn IGeometryImpl>>>,
}

ferro_class! {
    StreamGeometry: Geometry, virtuals StreamGeometryImpl: GeometryImpl {
        /// Creates the context that [`StreamGeometry::open`] wraps.
        fn open_core(this) -> Box<dyn IGeometryContext>;
    }
}
crate::ferro_class_info!(StreamGeometry { new: StreamGeometry::new });
ferro_impl_classes!(StreamGeometry: FerroObjectImpl);

impl GeometryImpl for StreamGeometry {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        let result = StreamGeometry::with_impl(this.create_defining_geometry());
        result.set_transform(this.transform());
        result.upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Arc<dyn IGeometryImpl>> {
        let mut impl_ = this.impl_.borrow_mut();
        if impl_.is_none() {
            let mut builder = Self::create_builder(None);
            *impl_ = Some(builder.to_geometry());
            builder.dispose();
        }

        impl_.clone()
    }
}

impl StreamGeometryImpl for StreamGeometry {
    fn open_core(this: &Self) -> Box<dyn IGeometryContext> {
        let source = this.impl_.borrow().clone();
        Box::new(BuilderWrapper { builder: Self::create_builder(source.as_deref()), owner: this.to_ref() })
    }
}

impl StreamGeometry {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Geometry::construct(), impl_: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a stream geometry over a platform implementation.
    fn with_impl(impl_: Option<Arc<dyn IGeometryImpl>>) -> Ref<Self> {
        instantiate(Self { base: Geometry::construct(), impl_: RefCell::new(impl_) })
    }

    /// Creates a stream geometry from a string of path data.
    pub fn parse(s: &str) -> Result<Ref<StreamGeometry>, FormatError> {
        let stream_geometry = StreamGeometry::new();

        let mut builder = Self::create_builder(None);
        {
            let mut parser = PathMarkupParser::new(&mut *builder);
            let result = parser.parse(s);
            parser.dispose();
            result?;
        }

        *stream_geometry.impl_.borrow_mut() = Some(builder.to_geometry());

        Ok(stream_geometry)
    }

    /// Opens the geometry to start defining it.
    ///
    /// Returns a [`StreamGeometryContext`] which can be used to define the
    /// geometry.
    ///
    /// The new figures are added to the existing ones. The geometry is
    /// updated when the context is disposed.
    pub fn open(&self) -> StreamGeometryContext {
        StreamGeometryContext::from_context(self.open_core())
    }

    fn set_impl(&self, impl_: Arc<dyn IGeometryImpl>) {
        *self.impl_.borrow_mut() = Some(impl_);
        self.invalidate_geometry();
    }

    fn create_builder(source: Option<&dyn IGeometryImpl>) -> Box<dyn IStreamGeometryBuilder> {
        let factory = platform::render_interface();
        factory.create_stream_geometry_builder(source)
    }
}

/// Wraps an [`IStreamGeometryBuilder`] and sets the [`StreamGeometry`]
/// geometry when disposed.
struct BuilderWrapper {
    builder: Box<dyn IStreamGeometryBuilder>,
    owner: Ref<StreamGeometry>,
}

impl IGeometryContext for BuilderWrapper {
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    ) {
        self.builder.arc_to(point, size, rotation_angle, is_large_arc, sweep_direction, is_stroked);
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        self.builder.begin_figure(start_point, is_filled);
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        self.builder.cubic_bezier_to(control_point1, control_point2, end_point, is_stroked);
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        self.builder.quadratic_bezier_to(control_point, end_point, is_stroked);
    }

    fn line_to(&mut self, point: Point, is_stroked: bool) {
        self.builder.line_to(point, is_stroked);
    }

    fn end_figure(&mut self, is_closed: bool) {
        self.builder.end_figure(is_closed);
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.builder.set_fill_rule(fill_rule);
    }

    fn dispose(&mut self) {
        self.owner.set_impl(self.builder.to_geometry());
        self.builder.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::geometry::tests::with_mock_render_interface;
    use crate::media::TranslateTransform;
    use std::cell::Cell;
    use std::rc::Rc;

    fn count_changed(target: &StreamGeometry) -> Rc<Cell<i32>> {
        let changed = Rc::new(Cell::new(0));
        let c = changed.clone();
        target.changed(move || c.set(c.get() + 1)).detach();
        changed
    }

    #[test]
    fn changed_is_raised_when_the_drawing_is_finished() {
        with_mock_render_interface(|_| {
            let target = StreamGeometry::new();
            let changed = count_changed(&target);

            {
                let mut context = target.open();
                assert_eq!(0, changed.get());

                context.begin_figure(Point::new(0.0, 0.0), true);
                context.line_to(Point::new(10.0, 0.0), true);
                context.end_figure(true);

                assert_eq!(0, changed.get());
            }

            assert_eq!(1, changed.get());
        });
    }

    #[test]
    fn changed_is_raised_when_the_drawing_is_finished_and_a_transform_is_set() {
        with_mock_render_interface(|_| {
            let target = StreamGeometry::new();
            target.set_transform(TranslateTransform::with_offset(10.0, 20.0));
            let changed = count_changed(&target);

            {
                let mut context = target.open();
                assert_eq!(0, changed.get());

                context.begin_figure(Point::new(0.0, 0.0), true);
                context.line_to(Point::new(10.0, 0.0), true);
                context.end_figure(true);

                assert_eq!(0, changed.get());
            }

            assert_eq!(1, changed.get());
        });
    }
}
