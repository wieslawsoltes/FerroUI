//! Exercises the composite point-matching path (ARGS_ARE_XY_VALUES clear) using
//! a hand-built 'glyf'/'loca' blob. Real-world fonts that use point matching
//! are rare, so the synthetic fixture is the only reliable way to cover this
//! branch.

use std::cell::{Cell, RefCell};

use crate::media::fonts::tables::LocaTable;
use crate::media::{FillRule, SweepDirection};
use crate::platform::IGeometryContext;
use crate::utilities::ReadOnlyMemory;
use crate::{Matrix, Point, Size};

use super::{CompositeFlags, GlyfTable, GlyphFlag};

// Glyph layout in the synthetic font:
//   glyph 0: 100x100 square at the origin            -> the "base"
//   glyph 1: 10x10   square at the origin            -> the point-matched "accent"
//   glyph 2: composite { base by x/y offset (0,0), accent by point matching }
const BASE_GLYPH: u16 = 0;
const ACCENT_GLYPH: u16 = 1;
const COMPOSITE_GLYPH: i32 = 2;

#[test]
fn point_matched_composite_aligns_component_point_onto_parent_point() {
    // Match the accent's point 0 (its local origin) onto the base's point 2 (100, 100).
    let glyf = build_glyf(2, 0);

    let mut context = FigureRecordingContext::default();

    assert!(glyf.try_build_glyph_geometry(COMPOSITE_GLYPH, Matrix::IDENTITY, &mut context));

    let figures = context.figures.borrow();
    let all_points = context.all_points.borrow();

    // One contour from the base, one from the accent.
    assert_eq!(figures.len(), 2);
    assert!(context.all_closed());

    // The base contour is emitted first, unmoved.
    assert!(contains(&figures[0], 0.0, 0.0));
    assert!(contains(&figures[0], 100.0, 100.0));

    // The accent contour is translated by (parentPoint - componentPoint) = (100,100) - (0,0).
    // Its figure therefore starts where the matched parent point is.
    assert!(close(figures[1][0], 100.0, 100.0));

    // The translated accent reaches (110, 110); those coordinates are impossible for the
    // base (which spans 0..100) so they prove the component actually moved.
    assert!(contains(&all_points, 110.0, 110.0));
    assert!(contains(&all_points, 110.0, 100.0));
    assert!(contains(&all_points, 100.0, 110.0));

    // If point matching were (incorrectly) treated as a zero offset, the accent would stay
    // at the origin and its far corner (10, 10) would appear instead.
    assert!(!contains(&all_points, 10.0, 10.0));
}

#[test]
fn point_matched_composite_matching_different_parent_point_moves_accent_there() {
    // Match the accent's origin onto the base's point 1 (100, 0) instead.
    let glyf = build_glyf(1, 0);

    let mut context = FigureRecordingContext::default();

    assert!(glyf.try_build_glyph_geometry(COMPOSITE_GLYPH, Matrix::IDENTITY, &mut context));

    let figures = context.figures.borrow();

    assert_eq!(figures.len(), 2);
    assert!(close(figures[1][0], 100.0, 0.0));
    assert!(contains(&context.all_points.borrow(), 110.0, 10.0));
}

#[test]
fn point_matched_composite_out_of_range_component_point_bails_out() {
    // The accent only has 4 points (0..3); point 99 is out of range.
    let glyf = build_glyf(2, 99);

    let mut context = FigureRecordingContext::default();

    assert!(!glyf.try_build_glyph_geometry(COMPOSITE_GLYPH, Matrix::IDENTITY, &mut context));

    // Nothing is emitted: the materialised outline is discarded before reaching the context.
    assert!(context.figures.borrow().is_empty());
}

#[test]
fn point_matched_composite_out_of_range_parent_point_bails_out() {
    // The base contributes 4 parent points (0..3); point 50 is out of range.
    let glyf = build_glyf(50, 0);

    let mut context = FigureRecordingContext::default();

    assert!(!glyf.try_build_glyph_geometry(COMPOSITE_GLYPH, Matrix::IDENTITY, &mut context));

    assert!(context.figures.borrow().is_empty());
}

// --- synthetic font construction -------------------------------------------------------

fn build_glyf(parent_point: u8, component_point: u8) -> GlyfTable {
    let glyph0 = pad_to_even(build_simple_square(100));
    let glyph1 = pad_to_even(build_simple_square(10));
    let glyph2 = pad_to_even(build_point_matched_composite(parent_point, component_point));

    let mut glyf = Vec::new();
    glyf.extend_from_slice(&glyph0);
    glyf.extend_from_slice(&glyph1);
    glyf.extend_from_slice(&glyph2);

    // Short 'loca' stores offset / 2 (which is why glyphs are padded to an even length).
    let offsets = [
        0,
        glyph0.len(),
        glyph0.len() + glyph1.len(),
        glyph0.len() + glyph1.len() + glyph2.len(),
    ];

    let mut loca = Vec::new();

    for offset in offsets {
        write_u16(&mut loca, (offset / 2) as u16);
    }

    create_glyf_table(glyf, loca, 3)
}

/// A simple, single-contour square with on-curve corners (0,0), (size,0),
/// (size,size), (0,size), encoded with int16 coordinate deltas.
fn build_simple_square(size: i16) -> Vec<u8> {
    let mut data = Vec::new();

    // Glyph header.
    write_i16(&mut data, 1); // numberOfContours (> 0 => simple)
    write_i16(&mut data, 0); // xMin
    write_i16(&mut data, 0); // yMin
    write_i16(&mut data, size); // xMax
    write_i16(&mut data, size); // yMax

    // Simple glyph body.
    write_u16(&mut data, 3); // endPtsOfContours[0] => 4 points
    write_u16(&mut data, 0); // instructionLength

    // Flags: 4 on-curve points, coordinates encoded as int16 deltas
    // (XShortVector/YShortVector clear, *IsSame* clear).
    for _ in 0..4 {
        data.push(GlyphFlag::OnCurvePoint.bits());
    }

    // X deltas: 0, +size, 0, -size.
    write_i16(&mut data, 0);
    write_i16(&mut data, size);
    write_i16(&mut data, 0);
    write_i16(&mut data, -size);

    // Y deltas: 0, 0, +size, 0.
    write_i16(&mut data, 0);
    write_i16(&mut data, 0);
    write_i16(&mut data, size);
    write_i16(&mut data, 0);

    data
}

/// A composite glyph with two components: the base placed by an x/y offset of
/// (0,0), and the accent placed by point matching (ARGS_ARE_XY_VALUES clear).
fn build_point_matched_composite(parent_point: u8, component_point: u8) -> Vec<u8> {
    let mut data = Vec::new();

    // Glyph header (numberOfContours < 0 => composite).
    write_i16(&mut data, -1);
    write_i16(&mut data, 0);
    write_i16(&mut data, 0);
    write_i16(&mut data, 110);
    write_i16(&mut data, 110);

    // Component 0: the base, placed by x/y offset (0,0). Byte args.
    write_u16(&mut data, (CompositeFlags::ArgsAreXYValues | CompositeFlags::MoreComponents).bits());
    write_u16(&mut data, BASE_GLYPH);
    data.push(0); // arg1 = x offset 0
    data.push(0); // arg2 = y offset 0

    // Component 1: the accent, placed by point matching. Byte args, no more components.
    write_u16(&mut data, 0); // flags: ArgsAreWords clear, ArgsAreXYValues clear, MoreComponents clear
    write_u16(&mut data, ACCENT_GLYPH);
    data.push(parent_point); // arg1 = point of the already-assembled glyph
    data.push(component_point); // arg2 = point of this component

    data
}

fn pad_to_even(mut data: Vec<u8>) -> Vec<u8> {
    if (data.len() & 1) != 0 {
        data.push(0);
    }

    data
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

fn close(point: Point, x: f64, y: f64) -> bool {
    (point.x - x).abs() < 0.001 && (point.y - y).abs() < 0.001
}

fn contains(points: &[Point], x: f64, y: f64) -> bool {
    points.iter().any(|&point| close(point, x, y))
}

#[derive(Default)]
struct FigureRecordingContext {
    figures: RefCell<Vec<Vec<Point>>>,
    all_points: RefCell<Vec<Point>>,
    any_open: Cell<bool>,
    in_figure: Cell<bool>,
}

impl FigureRecordingContext {
    fn all_closed(&self) -> bool {
        !self.any_open.get()
    }

    fn add(&self, point: Point) {
        if self.in_figure.get() {
            if let Some(current) = self.figures.borrow_mut().last_mut() {
                current.push(point);
            }
        }

        self.all_points.borrow_mut().push(point);
    }
}

impl IGeometryContext for FigureRecordingContext {
    fn begin_figure(&mut self, start_point: Point, _is_filled: bool) {
        self.figures.borrow_mut().push(vec![start_point]);
        self.in_figure.set(true);
        self.all_points.borrow_mut().push(start_point);
    }

    fn line_to(&mut self, point: Point, _is_stroked: bool) {
        self.add(point);
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, _is_stroked: bool) {
        self.add(control_point);
        self.add(end_point);
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, _is_stroked: bool) {
        self.add(control_point1);
        self.add(control_point2);
        self.add(end_point);
    }

    fn arc_to(
        &mut self,
        point: Point,
        _size: Size,
        _rotation_angle: f64,
        _is_large_arc: bool,
        _sweep_direction: SweepDirection,
        _is_stroked: bool,
    ) {
        self.add(point);
    }

    fn end_figure(&mut self, is_closed: bool) {
        if !is_closed {
            self.any_open.set(true);
        }

        self.in_figure.set(false);
    }

    fn set_fill_rule(&mut self, _fill_rule: FillRule) {}

    fn dispose(&mut self) {}
}
