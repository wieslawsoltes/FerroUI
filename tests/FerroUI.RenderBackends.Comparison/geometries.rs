//! The geometry queries of the contracts, compared in numbers.
//!
//! The same shapes are created with the render interface of two backends
//! and asked the same questions: the bounds, the render bounds of pens, the
//! length of the contour and points along it, and whether the fill and the
//! stroke contain the points of a grid.

use crate::Backend;
use ferroui_base::media::immutable::{ImmutableDashStyle, ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{Colors, FillRule, GeometryCombineMode, IPen, PenLineCap, PenLineJoin, SweepDirection};
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::{Matrix, Point, Rect, Size};
use std::rc::Rc;
use std::sync::Arc;

/// A shape: its name in the table and how a backend creates it.
pub struct Shape {
    pub name: &'static str,
    pub create: fn(&Backend) -> Arc<dyn IGeometryImpl>,
    /// Whether the contour is the same figure from the same start in both
    /// backends, so that lengths and points along it can be compared. A
    /// combined geometry is not: the boolean operations of the two backends
    /// give the same area as figures in another order.
    pub same_contour: bool,
    /// Whether the shape is the outline of a stroke, which each backend
    /// builds from curves to its own tolerance: its bounds and its fill
    /// agree as far as strokes do.
    pub outline_of_a_stroke: bool,
}

fn rectangle(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    backend.interface.create_rectangle_geometry(Rect::new(12.5, 20.0, 150.0, 90.25))
}

fn ellipse(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    backend.interface.create_ellipse_geometry(Rect::new(20.0, 30.0, 160.0, 110.0))
}

fn line(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    backend.interface.create_line_geometry(Point::new(170.0, 25.0), Point::new(30.0, 160.0))
}

fn curves(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    let geometry = backend.interface.create_stream_geometry();
    let mut context = geometry.open();
    context.begin_figure(Point::new(30.0, 40.0), true);
    context.line_to(Point::new(120.0, 25.0), true);
    context.quadratic_bezier_to(Point::new(185.0, 60.0), Point::new(150.0, 110.0), true);
    context.cubic_bezier_to(Point::new(120.0, 150.0), Point::new(190.0, 160.0), Point::new(140.0, 185.0), true);
    context.end_figure(true);
    context.dispose();
    geometry
}

fn arc(backend: &Backend, is_large_arc: bool, sweep_direction: SweepDirection, rotation: f64) -> Arc<dyn IGeometryImpl> {
    let geometry = backend.interface.create_stream_geometry();
    let mut context = geometry.open();
    context.begin_figure(Point::new(60.0, 110.0), true);
    context.arc_to(Point::new(140.0, 90.0), Size::new(70.0, 45.0), rotation, is_large_arc, sweep_direction, true);
    context.end_figure(false);
    context.dispose();
    geometry
}

fn arc_small_clockwise(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    arc(backend, false, SweepDirection::Clockwise, 0.0)
}

fn arc_large_clockwise(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    arc(backend, true, SweepDirection::Clockwise, 0.0)
}

fn arc_small_counter_clockwise(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    arc(backend, false, SweepDirection::CounterClockwise, 0.0)
}

fn arc_large_rotated(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    arc(backend, true, SweepDirection::CounterClockwise, 35.0)
}

/// An arc whose radii are too small to reach its end point: they are scaled
/// up.
fn arc_radii_too_small(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    let geometry = backend.interface.create_stream_geometry();
    let mut context = geometry.open();
    context.begin_figure(Point::new(40.0, 100.0), true);
    context.arc_to(Point::new(160.0, 100.0), Size::new(20.0, 10.0), 0.0, false, SweepDirection::Clockwise, true);
    context.end_figure(false);
    context.dispose();
    geometry
}

fn star_even_odd(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    let geometry = backend.interface.create_stream_geometry();
    let mut context = geometry.open();
    context.set_fill_rule(FillRule::EvenOdd);
    let point = |index: usize| {
        let angle = -std::f64::consts::FRAC_PI_2 + index as f64 * 4.0 * std::f64::consts::PI / 5.0;
        Point::new(100.0 + 85.0 * angle.cos(), 105.0 + 85.0 * angle.sin())
    };
    context.begin_figure(point(0), true);
    for index in 1..5 {
        context.line_to(point(index), true);
    }
    context.end_figure(true);
    context.dispose();
    geometry
}

fn transformed(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    let transform = Matrix::create_rotation(0.4) * Matrix::create_scale(0.8, 1.2) * Matrix::create_translation(60.0, -20.0);
    curves(backend).with_transform(transform)
}

fn group(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    let children = vec![rectangle(backend), ellipse(backend), line(backend)];
    backend.interface.create_geometry_group(FillRule::EvenOdd, &children)
}

fn combined(backend: &Backend, mode: GeometryCombineMode) -> Arc<dyn IGeometryImpl> {
    backend.interface.create_combined_geometry(mode, rectangle(backend), ellipse(backend))
}

fn combined_union(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    combined(backend, GeometryCombineMode::Union)
}

fn combined_intersect(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    combined(backend, GeometryCombineMode::Intersect)
}

fn combined_xor(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    combined(backend, GeometryCombineMode::Xor)
}

fn combined_exclude(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    combined(backend, GeometryCombineMode::Exclude)
}

fn widened(backend: &Backend) -> Arc<dyn IGeometryImpl> {
    curves(backend).get_widened_geometry(&pens()[1].1)
}

/// The shapes.
pub fn shapes() -> Vec<Shape> {
    macro_rules! shape {
        ($create:ident, $same_contour:expr) => {
            Shape {
                name: stringify!($create),
                create: $create,
                same_contour: $same_contour,
                outline_of_a_stroke: stringify!($create) == "widened",
            }
        };
    }

    vec![
        shape!(rectangle, true),
        shape!(ellipse, true),
        shape!(line, true),
        shape!(curves, true),
        shape!(arc_small_clockwise, true),
        shape!(arc_large_clockwise, true),
        shape!(arc_small_counter_clockwise, true),
        shape!(arc_large_rotated, true),
        shape!(arc_radii_too_small, true),
        shape!(star_even_odd, true),
        shape!(transformed, true),
        shape!(group, true),
        shape!(combined_union, false),
        shape!(combined_intersect, false),
        shape!(combined_xor, false),
        shape!(combined_exclude, false),
        shape!(widened, false),
    ]
}

/// The pens the render bounds and the stroke hit tests are asked with.
pub fn pens() -> Vec<(&'static str, ImmutablePen)> {
    let brush = || Some(Rc::new(ImmutableSolidColorBrush::new(Colors::BLACK)) as Rc<dyn ferroui_base::media::IImmutableBrush>);

    vec![
        ("thin flat miter", ImmutablePen::new(brush(), 1.0, None, PenLineCap::Flat, PenLineJoin::Miter, 10.0)),
        ("thick round round", ImmutablePen::new(brush(), 12.0, None, PenLineCap::Round, PenLineJoin::Round, 10.0)),
        ("thick square bevel", ImmutablePen::new(brush(), 9.0, None, PenLineCap::Square, PenLineJoin::Bevel, 10.0)),
        (
            "dashed",
            ImmutablePen::new(
                brush(),
                6.0,
                Some(Rc::new(ImmutableDashStyle::new(Some(&[2.0, 1.5]), 0.5))),
                PenLineCap::Flat,
                PenLineJoin::Miter,
                10.0,
            ),
        ),
    ]
}

/// How far the answers of two backends about one shape are apart.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShapeDifference {
    /// The largest difference of an edge of the bounds.
    pub bounds: f64,
    /// The largest difference of an edge of the render bounds of a pen.
    pub render_bounds: f64,
    /// The difference of the contour lengths, or `None` when the contours
    /// are not comparable.
    pub contour_length: Option<f64>,
    /// The largest distance of the points at the same distance along the
    /// contour, or `None` when the contours are not comparable.
    pub point_at_distance: Option<f64>,
    /// The points of the grid the fills disagree about, of [`GRID_POINTS`].
    pub fill_contains: usize,
    /// The points of the grid the strokes of each pen disagree about, of
    /// [`GRID_POINTS`], in the order of [`pens`].
    pub stroke_contains: [usize; 4],
}

/// The points of the hit test grid: 67 by 67 over the 200 units the shapes
/// lie in, at coordinates no edge of a shape lies on.
pub const GRID_POINTS: usize = 67 * 67;

fn rect_difference(a: Rect, b: Rect) -> f64 {
    [(a.x - b.x).abs(), (a.y - b.y).abs(), (a.right() - b.right()).abs(), (a.bottom() - b.bottom()).abs()]
        .into_iter()
        .fold(0.0, f64::max)
}

/// Asks two backends the same questions about a shape.
pub fn compare_shape(shape: &Shape, reference: &Backend, tested: &Backend) -> ShapeDifference {
    let (a, b) = ((shape.create)(reference), (shape.create)(tested));
    let pens = pens();

    let mut difference = ShapeDifference { bounds: rect_difference(a.bounds(), b.bounds()), ..Default::default() };

    for (_, pen) in &pens {
        let pen: &dyn IPen = pen;
        difference.render_bounds =
            difference.render_bounds.max(rect_difference(a.get_render_bounds(Some(pen)), b.get_render_bounds(Some(pen))));
    }

    if shape.same_contour {
        let (length_a, length_b) = (a.contour_length(), b.contour_length());
        difference.contour_length = Some((length_a - length_b).abs());

        let mut largest = 0.0f64;
        // Twenty distances along the contour, none of them at a corner of
        // a shape: there the tangent is that of either side.
        for step in 0..20 {
            let distance = length_a * (step as f64 + 0.37) / 20.0;
            match (a.try_get_point_and_tangent_at_distance(distance), b.try_get_point_and_tangent_at_distance(distance)) {
                (Some((point_a, tangent_a)), Some((point_b, tangent_b))) => {
                    largest = largest.max((point_a.x - point_b.x).hypot(point_a.y - point_b.y));
                    // A tangent is a unit vector: its difference counts in
                    // the same number, scaled to the units of the shape.
                    largest = largest.max((tangent_a.x - tangent_b.x).hypot(tangent_a.y - tangent_b.y));
                }
                (None, None) => {}
                _ => largest = f64::INFINITY,
            }
        }
        difference.point_at_distance = Some(largest);
    }

    for row in 0..67 {
        for column in 0..67 {
            let point = Point::new(1.37 + 3.0 * column as f64, 0.61 + 3.0 * row as f64);
            if a.fill_contains(point) != b.fill_contains(point) {
                difference.fill_contains += 1;
            }
            for (index, (_, pen)) in pens.iter().enumerate() {
                let pen: &dyn IPen = pen;
                if a.stroke_contains(Some(pen), point) != b.stroke_contains(Some(pen), point) {
                    difference.stroke_contains[index] += 1;
                }
            }
        }
    }

    difference
}
