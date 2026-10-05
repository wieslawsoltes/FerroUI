//! Exercises the on/off-curve contour walk of the glyph geometry builder
//! against hand-built simple glyphs, pinning the TrueType decomposition rules:
//! an implied on-curve midpoint between consecutive off-curve points, and the
//! figure-start selection when the first point is off-curve (the last point
//! when that one is on-curve, otherwise the implied midpoint between last and
//! first).

use std::cell::{Cell, RefCell};

use crate::media::fonts::tables::LocaTable;
use crate::media::{FillRule, SweepDirection};
use crate::platform::IGeometryContext;
use crate::utilities::ReadOnlyMemory;
use crate::{Matrix, Point, Size};

use super::{GlyfTable, GlyphFlag};

#[test]
fn all_off_curve_contour_emits_a_quadratic_per_point() {
    // A "quadratic circle": every point is an off-curve control; each consecutive pair
    // implies an on-curve midpoint, so the contour decomposes into exactly one
    // quadratic segment per input point, closing back to the figure start.
    let glyf = build_single_glyph_table(&[(100, 0, false), (0, 100, false), (-100, 0, false), (0, -100, false)]);

    let mut context = SegmentRecordingContext::default();

    assert!(glyf.try_build_glyph_geometry(0, Matrix::IDENTITY, &mut context));

    assert_eq!(context.figure_start.get(), Point::new(50.0, -50.0));
    assert!(context.lines.borrow().is_empty());

    let quadratics = context.quadratics.borrow();

    assert_eq!(quadratics.len(), 4);
    assert_eq!(quadratics[0], (Point::new(100.0, 0.0), Point::new(50.0, 50.0)));
    assert_eq!(quadratics[1], (Point::new(0.0, 100.0), Point::new(-50.0, 50.0)));
    assert_eq!(quadratics[2], (Point::new(-100.0, 0.0), Point::new(-50.0, -50.0)));
    // The trailing off-curve point closes the contour back to the figure start.
    assert_eq!(quadratics[3], (Point::new(0.0, -100.0), Point::new(50.0, -50.0)));

    // TrueType outlines use the non-zero winding rule.
    assert_eq!(context.fill_rule.get(), Some(FillRule::NonZero));
}

#[test]
fn off_curve_first_point_starts_the_figure_at_the_on_curve_last_point() {
    let glyf = build_single_glyph_table(&[(100, 100, false), (0, 0, true), (200, 0, true)]);

    let mut context = SegmentRecordingContext::default();

    assert!(glyf.try_build_glyph_geometry(0, Matrix::IDENTITY, &mut context));

    // The last point is on-curve, so the contour starts there — an implied midpoint
    // between an off-curve and an on-curve point is not a point of the curve and
    // would add a spurious vertex.
    assert_eq!(context.figure_start.get(), Point::new(200.0, 0.0));

    let quadratics = context.quadratics.borrow();

    assert_eq!(quadratics.len(), 1);
    assert_eq!(quadratics[0], (Point::new(100.0, 100.0), Point::new(0.0, 0.0)));

    // The two on-curve points connect with the closing line.
    assert_eq!(*context.lines.borrow(), [Point::new(200.0, 0.0)]);
}

#[test]
fn on_curve_start_with_off_curve_run_splits_at_implied_midpoints() {
    // Pins the already-correct on-curve-start walk so the unified walker cannot drift.
    let glyf = build_single_glyph_table(&[(0, 0, true), (100, 0, false), (100, 100, false), (0, 100, true)]);

    let mut context = SegmentRecordingContext::default();

    assert!(glyf.try_build_glyph_geometry(0, Matrix::IDENTITY, &mut context));

    assert_eq!(context.figure_start.get(), Point::new(0.0, 0.0));

    let quadratics = context.quadratics.borrow();

    assert_eq!(quadratics.len(), 2);
    assert_eq!(quadratics[0], (Point::new(100.0, 0.0), Point::new(100.0, 50.0)));
    assert_eq!(quadratics[1], (Point::new(100.0, 100.0), Point::new(0.0, 100.0)));

    assert_eq!(*context.lines.borrow(), [Point::new(0.0, 0.0)]);
}

// --- synthetic font construction -------------------------------------------------------

fn build_single_glyph_table(points: &[(i16, i16, bool)]) -> GlyfTable {
    let mut data = Vec::new();

    // Glyph header; the bounding box is not consumed by the walk.
    write_i16(&mut data, 1);
    write_i16(&mut data, 0);
    write_i16(&mut data, 0);
    write_i16(&mut data, 0);
    write_i16(&mut data, 0);

    write_u16(&mut data, (points.len() - 1) as u16); // endPtsOfContours[0]
    write_u16(&mut data, 0); // instructionLength

    // One flag per point, coordinates encoded as int16 deltas
    // (XShortVector/YShortVector clear, *IsSame* clear).
    for &(_, _, on_curve) in points {
        data.push(if on_curve { GlyphFlag::OnCurvePoint.bits() } else { GlyphFlag::None.bits() });
    }

    let mut previous = 0i16;

    for &(x, _, _) in points {
        write_i16(&mut data, x - previous);
        previous = x;
    }

    previous = 0;

    for &(_, y, _) in points {
        write_i16(&mut data, y - previous);
        previous = y;
    }

    // Short 'loca' stores offset / 2, so pad the glyph to an even length.
    if (data.len() & 1) != 0 {
        data.push(0);
    }

    let mut loca = Vec::new();
    write_u16(&mut loca, 0);
    write_u16(&mut loca, (data.len() / 2) as u16);

    create_glyf_table(data, loca, 1)
}

fn write_u16(data: &mut Vec<u8>, value: u16) {
    data.extend_from_slice(&value.to_be_bytes());
}

fn write_i16(data: &mut Vec<u8>, value: i16) {
    data.extend_from_slice(&value.to_be_bytes());
}

/// Builds a [`GlyfTable`] directly from raw 'glyf'/'loca' bytes, bypassing the
/// full font-load path (which would require a complete font).
fn create_glyf_table(glyf_data: Vec<u8>, loca_data: Vec<u8>, glyph_count: i32) -> GlyfTable {
    let loca = LocaTable::new(ReadOnlyMemory::from_vec(loca_data), glyph_count, /* is_short_format */ true);

    GlyfTable::new(ReadOnlyMemory::from_vec(glyf_data), loca)
}

#[derive(Default)]
struct SegmentRecordingContext {
    figure_start: Cell<Point>,
    lines: RefCell<Vec<Point>>,
    quadratics: RefCell<Vec<(Point, Point)>>,
    fill_rule: Cell<Option<FillRule>>,
}

impl IGeometryContext for SegmentRecordingContext {
    fn begin_figure(&mut self, start_point: Point, _is_filled: bool) {
        self.figure_start.set(start_point);
    }

    fn line_to(&mut self, point: Point, _is_stroked: bool) {
        self.lines.borrow_mut().push(point);
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, _is_stroked: bool) {
        self.quadratics.borrow_mut().push((control_point, end_point));
    }

    fn cubic_bezier_to(&mut self, _control_point1: Point, _control_point2: Point, _end_point: Point, _is_stroked: bool) {}

    fn arc_to(
        &mut self,
        _point: Point,
        _size: Size,
        _rotation_angle: f64,
        _is_large_arc: bool,
        _sweep_direction: SweepDirection,
        _is_stroked: bool,
    ) {
    }

    fn end_figure(&mut self, _is_closed: bool) {}

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.fill_rule.set(Some(fill_rule));
    }

    fn dispose(&mut self) {}
}
