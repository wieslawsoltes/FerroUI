//! The test doubles of the scene graph tests: what the upstream tests set up
//! with `Mock<IGeometryImpl>`, `Mock<IBitmapImpl>` and
//! `Mock<ICustomDrawOperation>`, and the recording drawing context that
//! stands for `Mock<IDrawingContextImpl>` with its verifications.

use crate::media::imaging::BitmapEncoderOptions;
use crate::media::immutable::ImmutablePen;
use crate::media::{Brushes, IBrush, IImmutableBrush, IPen, ImmediateDrawingContext, IntersectionResult};
use crate::platform::{IBitmapImpl, IGeometryImpl, ITransformedGeometryImpl};
use crate::rendering::composition::drawing::{RenderDataResource, RenderDataStream};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl, MockGeometryImpl};
use crate::{Matrix, PixelSize, Point, Rect, RoundedRect, Vector};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// `Brushes.Black`.
pub(crate) fn black() -> Rc<dyn IBrush> {
    Brushes::black()
}

/// `new ImmutablePen(Brushes.Black, thickness)`.
pub(crate) fn black_pen(thickness: f64) -> Rc<dyn IPen> {
    let brush: Rc<dyn IImmutableBrush> = Brushes::black();
    Rc::new(ImmutablePen::with_brush(Some(brush), thickness))
}

/// A brush as the stream records it without a compositor.
pub(crate) fn b(brush: &Rc<dyn IBrush>) -> Option<RenderDataResource> {
    Some(RenderDataResource::Brush(brush.clone()))
}

/// A pen as the stream records it without a compositor.
pub(crate) fn p(pen: &Rc<dyn IPen>) -> Option<RenderDataResource> {
    Some(RenderDataResource::Pen(pen.clone()))
}

/// `new RoundedRect(new Rect(x, y, width, height))`.
pub(crate) fn rrect(x: f64, y: f64, width: f64, height: f64) -> RoundedRect {
    RoundedRect::from_rect(Rect::new(x, y, width, height))
}

/// Replays a stream on a recording drawing context and returns what was
/// called on it.
pub(crate) fn replay(stream: &RenderDataStream) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.log_transforms = false;
    stream.replay(&mut context);
    log.entries()
}

/// `Mock<IGeometryImpl>`: answers `FillContains` and `StrokeContains` with
/// true for the points it was set up with (and, for `StrokeContains`, the
/// pen, when one was given) and with false otherwise; every other member
/// returns the default value, as a loose mock does.
#[derive(Default)]
pub(crate) struct TestGeometryImpl {
    pub fill: Vec<Point>,
    pub stroke: Vec<Point>,
    pub stroke_pen: Option<TestPen>,
}

/// The pen a [`TestGeometryImpl`] expects. The tests run on one thread; the
/// wrapper only satisfies the thread-safety bound of the geometry contract.
pub(crate) struct TestPen(Rc<dyn IPen>);

// SAFETY: test geometries are created and used on the thread of the test.
unsafe impl Send for TestPen {}
unsafe impl Sync for TestPen {}

impl TestGeometryImpl {
    pub(crate) fn new() -> Arc<TestGeometryImpl> {
        Arc::new(TestGeometryImpl::default())
    }

    pub(crate) fn with_fill(points: &[Point]) -> Arc<TestGeometryImpl> {
        Arc::new(TestGeometryImpl { fill: points.to_vec(), ..Default::default() })
    }

    /// `StrokeContains(pen, point)` set up for the given pen only.
    pub(crate) fn with_stroke_pen(pen: &Rc<dyn IPen>, points: &[Point]) -> Arc<TestGeometryImpl> {
        Arc::new(TestGeometryImpl { stroke: points.to_vec(), stroke_pen: Some(TestPen(pen.clone())), ..Default::default() })
    }
}

impl IGeometryImpl for TestGeometryImpl {
    fn bounds(&self) -> Rect {
        Rect::default()
    }
    fn contour_length(&self) -> f64 {
        0.0
    }
    fn get_render_bounds(&self, _pen: Option<&dyn IPen>) -> Rect {
        Rect::default()
    }
    fn get_widened_geometry(&self, _pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
        MockGeometryImpl::new(Rect::default())
    }
    fn fill_contains(&self, point: Point) -> bool {
        self.fill.contains(&point)
    }
    fn get_fill_intersection_result(&self, _geometry: &dyn IGeometryImpl) -> IntersectionResult {
        IntersectionResult::NotCalculated
    }
    fn intersect(&self, _geometry: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
        None
    }
    fn stroke_contains(&self, pen: Option<&dyn IPen>, point: Point) -> bool {
        let pen_matches = match (&self.stroke_pen, pen) {
            (None, _) => true,
            (Some(expected), Some(pen)) => *expected.0 == *pen,
            (Some(_), None) => false,
        };
        pen_matches && self.stroke.contains(&point)
    }
    fn with_transform(&self, transform: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
        MockGeometryImpl::new(Rect::default()).with_transform(transform)
    }
    fn try_get_point_at_distance(&self, _distance: f64) -> Option<Point> {
        None
    }
    fn try_get_point_and_tangent_at_distance(&self, _distance: f64) -> Option<(Point, Point)> {
        None
    }
    fn try_get_segment(
        &self,
        _start_distance: f64,
        _stop_distance: f64,
        _start_on_begin_figure: bool,
    ) -> Option<Arc<dyn IGeometryImpl>> {
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `Mock.Of<IBitmapImpl>()`, held as `RefCountable.Create(..)` is: the
/// reference count is the strong count of the handle.
#[derive(Default)]
pub(crate) struct TestBitmapImpl;

impl IBitmapImpl for TestBitmapImpl {
    fn dpi(&self) -> Vector {
        Vector::default()
    }
    fn pixel_size(&self) -> PixelSize {
        PixelSize::default()
    }
    fn version(&self) -> i32 {
        0
    }
    fn save(&self, _stream: &mut dyn std::io::Write, _options: &BitmapEncoderOptions) -> std::io::Result<()> {
        Ok(())
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `Mock<ICustomDrawOperation>`: the bounds and the hit points it was set up
/// with, and the counts of its `Render` and `Dispose` calls.
/// A counter of a test operation: operations are shared between threads,
/// so it counts atomically behind the interface of a cell.
#[derive(Default)]
pub(crate) struct Counter(std::sync::atomic::AtomicI32);

impl Counter {
    pub(crate) fn new(value: i32) -> Self {
        Self(std::sync::atomic::AtomicI32::new(value))
    }

    pub(crate) fn get(&self) -> i32 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub(crate) fn set(&self, value: i32) {
        self.0.store(value, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Default)]
pub(crate) struct TestCustomOperation {
    pub bounds: Rect,
    pub hits: Vec<Point>,
    pub render_count: Counter,
    pub dispose_count: Counter,
}

impl TestCustomOperation {
    pub(crate) fn new() -> Arc<TestCustomOperation> {
        Arc::new(TestCustomOperation::default())
    }

    pub(crate) fn with_bounds(bounds: Rect) -> Arc<TestCustomOperation> {
        Arc::new(TestCustomOperation { bounds, ..Default::default() })
    }

    pub(crate) fn with_hits(hits: &[Point]) -> Arc<TestCustomOperation> {
        Arc::new(TestCustomOperation { hits: hits.to_vec(), ..Default::default() })
    }
}

impl ICustomDrawOperation for TestCustomOperation {
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn hit_test(&self, p: Point) -> bool {
        self.hits.contains(&p)
    }
    fn render(&self, _context: &mut ImmediateDrawingContext<'_>) {
        self.render_count.set(self.render_count.get() + 1);
    }
    fn equals(&self, other: &dyn ICustomDrawOperation) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const dyn ICustomDrawOperation)
    }
    fn dispose(&self) {
        self.dispose_count.set(self.dispose_count.get() + 1);
    }
}
