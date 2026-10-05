//! `GlyfTable` tests.
//!
//! The reference tests run against a real static TrueType font; here the same
//! cases run against a small hand-built font (`head` + `maxp` + long-format
//! `loca` + `glyf`) served through the synthetic font harness.
//!
//! Glyphs: 0 = empty (space), 1 = a 100x200 box with an off-curve corner ("letter"),
//! 2 = composite of glyph 1 twice (the second scaled by 0.5 and offset),
//! 3 = composite referring to itself (cycle), 4 = truncated header.

use std::cell::{Cell, RefCell};

use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};
use crate::media::fonts::tables::{HeadTable, MaxpTable};
use crate::media::{FillRule, GlyphBounds, SweepDirection};
use crate::platform::IGeometryContext;
use crate::{Matrix, Point, Size};

use super::{CompositeFlags, GlyfTable, GlyphDescriptor, GlyphFlag};

const SPACE_GLYPH: i32 = 0;
const LETTER_GLYPH: i32 = 1;
const COMPOSITE_GLYPH: i32 = 2;
const CYCLIC_GLYPH: i32 = 3;
const TRUNCATED_GLYPH: i32 = 4;
const GLYPH_COUNT: u16 = 5;

fn build_letter() -> Vec<u8> {
    let mut glyph = BigEndianBuffer::new();

    glyph.int16(1).int16(10).int16(-20).int16(110).int16(180); // header
    glyph.uint16(3).uint16(0); // endPtsOfContours[0], instructionLength

    let on = GlyphFlag::OnCurvePoint.bits() as i32;

    glyph.uint8(on).uint8(on).uint8(0).uint8(on);

    // Points (10,-20), (110,-20), (110,180) [off-curve], (10,180).
    glyph.int16(10).int16(100).int16(0).int16(-100);
    glyph.int16(-20).int16(0).int16(200).int16(0);

    glyph.to_array()
}

fn build_composite() -> Vec<u8> {
    let mut glyph = BigEndianBuffer::new();

    glyph.int16(-1).int16(10).int16(-20).int16(260).int16(180);

    // Component 0: glyph 1 at offset (0, 0), byte args.
    glyph
        .uint16((CompositeFlags::ArgsAreXYValues | CompositeFlags::MoreComponents).bits() as i32)
        .uint16(LETTER_GLYPH)
        .int8(0)
        .int8(0);

    // Component 1: glyph 1 scaled by 0.5 at offset (200, -10), word args.
    glyph
        .uint16((CompositeFlags::ArgsAreXYValues | CompositeFlags::ArgsAreWords | CompositeFlags::WeHaveAScale).bits() as i32)
        .uint16(LETTER_GLYPH)
        .int16(200)
        .int16(-10)
        .f2dot14(0.5);

    glyph.to_array()
}

fn build_cyclic() -> Vec<u8> {
    let mut glyph = BigEndianBuffer::new();

    glyph.int16(-1).zeros(8);
    glyph.uint16(CompositeFlags::ArgsAreXYValues.bits() as i32).uint16(CYCLIC_GLYPH).int8(0).int8(0);

    glyph.to_array()
}

fn build_font() -> SyntheticFont {
    let glyphs = [Vec::new(), build_letter(), build_composite(), build_cyclic(), vec![0u8, 1, 0, 0]];

    let mut glyf = Vec::new();
    let mut loca = BigEndianBuffer::new();

    for glyph in &glyphs {
        loca.uint32(glyf.len() as u32);
        glyf.extend_from_slice(glyph);
    }

    loca.uint32(glyf.len() as u32);

    let mut head = BigEndianBuffer::new();

    head.uint32(0x0001_0000)
        .uint32(0x0001_0000)
        .uint32(0)
        .uint32(0x5F0F_3CF5)
        .uint16(0)
        .uint16(1000)
        .zeros(16) // created, modified
        .zeros(8) // bounding box
        .uint16(0)
        .uint16(8)
        .int16(2)
        .int16(1) // indexToLocFormat: long
        .int16(0);

    let mut maxp = BigEndianBuffer::new();

    maxp.uint32(0x0000_5000).uint16(GLYPH_COUNT as i32);

    let mut font = SyntheticFont::new();

    font.replace("head", head.to_array())
        .replace("maxp", maxp.to_array())
        .replace("loca", loca.to_array())
        .replace("glyf", glyf);

    font
}

fn load_glyf() -> GlyfTable {
    let font = build_font();

    let head = HeadTable::try_load(&font).unwrap().unwrap();
    let maxp = MaxpTable::load(&font).unwrap();

    GlyfTable::try_load(&font, &head, &maxp).unwrap()
}

#[test]
fn try_load_succeeds() {
    let glyf = load_glyf();

    assert_eq!(glyf.glyph_count(), GLYPH_COUNT as i32);
}

#[test]
fn try_load_needs_both_glyf_and_loca() {
    let mut font = build_font();
    let head = HeadTable::try_load(&font).unwrap().unwrap();
    let maxp = MaxpTable::load(&font).unwrap();

    font.remove("loca");
    assert!(GlyfTable::try_load(&font, &head, &maxp).is_none());

    let mut font = build_font();
    font.remove("glyf");
    assert!(GlyfTable::try_load(&font, &head, &maxp).is_none());
}

#[test]
fn try_get_glyph_data_returns_false_for_out_of_range() {
    let glyf = load_glyf();

    assert!(glyf.try_get_glyph_data(-1).is_none());
    assert!(glyf.try_get_glyph_data(i32::MAX).is_none());
    assert!(glyf.try_get_glyph_data(GLYPH_COUNT as i32).is_none());
}

#[test]
fn try_get_glyph_data_returns_empty_for_space_glyph() {
    let glyf = load_glyf();

    // An empty glyph is a valid glyph with no outline data.
    let data = glyf.try_get_glyph_data(SPACE_GLYPH).unwrap();

    assert!(data.is_empty());
}

#[test]
fn try_get_glyph_data_returns_data_for_letter() {
    let glyf = load_glyf();

    let data = glyf.try_get_glyph_data(LETTER_GLYPH).unwrap();

    assert!(!data.is_empty());
    // At minimum a glyph carries its 10-byte header (numberOfContours + bbox).
    assert!(data.len() >= 10);
    assert_eq!(data.to_vec(), build_letter());
}

#[test]
fn try_get_glyph_data_rejects_offsets_outside_of_the_glyf_table() {
    let mut font = build_font();
    let head = HeadTable::try_load(&font).unwrap().unwrap();
    let maxp = MaxpTable::load(&font).unwrap();

    // The end offset of glyph 1 points far past the table; glyph 2 then starts there.
    font.patch_uint32("loca", 8, 0x00FF_FFFF);

    let glyf = GlyfTable::try_load(&font, &head, &maxp).unwrap();

    assert!(glyf.try_get_glyph_data(LETTER_GLYPH).is_none());
    assert!(glyf.try_get_glyph_data(COMPOSITE_GLYPH).is_none());
    assert!(glyf.try_get_glyph_bounds(LETTER_GLYPH).is_none());

    let mut context = RecordingGeometryContext::default();

    assert!(!glyf.try_build_glyph_geometry(LETTER_GLYPH, Matrix::IDENTITY, &mut context));

    let mut bounds = [GlyphBounds::new(1, 1, 1, 1); 2];
    glyf.get_glyph_bounds(&[LETTER_GLYPH as u16, COMPOSITE_GLYPH as u16], &mut bounds);

    assert_eq!(bounds, [GlyphBounds::default(); 2]);
}

#[test]
fn try_build_glyph_geometry_builds_closed_figures_for_letter() {
    let glyf = load_glyf();

    let mut context = RecordingGeometryContext::default();

    assert!(glyf.try_build_glyph_geometry(LETTER_GLYPH, Matrix::IDENTITY, &mut context));

    assert!(context.begin_figure_count.get() >= 1, "Expected at least one contour.");
    assert_eq!(context.begin_figure_count.get(), context.end_figure_count.get());
    assert!(context.all_figures_closed(), "Glyph contours must be closed.");
    assert_eq!(
        *context.points.borrow(),
        [
            Point::new(10.0, -20.0),  // begin
            Point::new(110.0, -20.0), // line
            Point::new(110.0, 180.0), // quadratic control
            Point::new(10.0, 180.0),  // quadratic end
            Point::new(10.0, -20.0),  // closing line
        ]
    );
    assert_eq!(context.fill_rule.get(), Some(FillRule::NonZero));
}

#[test]
fn try_build_glyph_geometry_returns_false_for_empty_glyph() {
    let glyf = load_glyf();

    let mut context = RecordingGeometryContext::default();

    // Nothing to draw for an empty glyph.
    assert!(!glyf.try_build_glyph_geometry(SPACE_GLYPH, Matrix::IDENTITY, &mut context));
    assert_eq!(context.begin_figure_count.get(), 0);
}

#[test]
fn try_build_glyph_geometry_returns_false_for_out_of_range() {
    let glyf = load_glyf();

    for glyph_index in [-1, i32::MAX] {
        let mut context = RecordingGeometryContext::default();

        assert!(!glyf.try_build_glyph_geometry(glyph_index, Matrix::IDENTITY, &mut context));
        assert_eq!(context.begin_figure_count.get(), 0);
    }
}

#[test]
fn try_build_glyph_geometry_applies_transform() {
    let glyf = load_glyf();

    let mut identity = RecordingGeometryContext::default();
    assert!(glyf.try_build_glyph_geometry(LETTER_GLYPH, Matrix::IDENTITY, &mut identity));

    let mut scaled = RecordingGeometryContext::default();
    assert!(glyf.try_build_glyph_geometry(LETTER_GLYPH, Matrix::create_scale(2.0, 2.0), &mut scaled));

    // Same glyph, same decode order: scaling the transform by 2 must scale every
    // emitted point by 2.
    let identity_points = identity.points.borrow();
    let scaled_points = scaled.points.borrow();

    assert_eq!(identity_points.len(), scaled_points.len());

    let mut saw_non_zero = false;

    for (a, b) in identity_points.iter().zip(scaled_points.iter()) {
        assert!((a.x * 2.0 - b.x).abs() < 0.0005);
        assert!((a.y * 2.0 - b.y).abs() < 0.0005);

        if a.x.abs() > 0.001 || a.y.abs() > 0.001 {
            saw_non_zero = true;
        }
    }

    assert!(saw_non_zero, "Expected the glyph to emit non-zero coordinates.");
}

#[test]
fn try_build_glyph_geometry_builds_composite_glyph() {
    let glyf = load_glyf();

    assert_eq!(find_composite_glyph(&glyf), COMPOSITE_GLYPH);

    let mut context = RecordingGeometryContext::default();

    assert!(glyf.try_build_glyph_geometry(COMPOSITE_GLYPH, Matrix::create_scale(2.0, 2.0), &mut context));

    // A composite expands into its components' contours.
    assert_eq!(context.begin_figure_count.get(), 2);
    assert_eq!(context.begin_figure_count.get(), context.end_figure_count.get());
    assert!(context.all_figures_closed());

    let points = context.points.borrow();

    // First component: unmoved, then the outer transform.
    assert_eq!(points[0], Point::new(20.0, -40.0));
    assert_eq!(points[2], Point::new(220.0, 360.0));
    // Second component: scaled by 0.5, offset by (200, -10), then the outer transform.
    assert_eq!(points[5], Point::new((10.0 * 0.5 + 200.0) * 2.0, (-20.0 * 0.5 - 10.0) * 2.0));
    assert_eq!(points[7], Point::new((110.0 * 0.5 + 200.0) * 2.0, (180.0 * 0.5 - 10.0) * 2.0));
}

#[test]
fn try_build_glyph_geometry_stops_on_component_cycles_and_malformed_glyphs() {
    let glyf = load_glyf();

    let mut context = RecordingGeometryContext::default();

    assert!(!glyf.try_build_glyph_geometry(CYCLIC_GLYPH, Matrix::IDENTITY, &mut context));
    assert_eq!(context.begin_figure_count.get(), 0);

    // A glyph too short for its header.
    assert!(!glyf.try_build_glyph_geometry(TRUNCATED_GLYPH, Matrix::IDENTITY, &mut context));
    assert_eq!(context.begin_figure_count.get(), 0);

    // The pooled cycle guard is clean again afterwards.
    assert!(glyf.try_build_glyph_geometry(LETTER_GLYPH, Matrix::IDENTITY, &mut context));
}

#[test]
fn try_get_glyph_bounds_returns_false_for_out_of_range() {
    let glyf = load_glyf();

    assert!(glyf.try_get_glyph_bounds(-1).is_none());
    assert!(glyf.try_get_glyph_bounds(i32::MAX).is_none());
    // Too short to contain a header.
    assert!(glyf.try_get_glyph_bounds(TRUNCATED_GLYPH).is_none());
}

#[test]
fn try_get_glyph_bounds_returns_zero_for_empty_glyph() {
    let glyf = load_glyf();

    // Empty glyph is valid but carries no box.
    assert_eq!(glyf.try_get_glyph_bounds(SPACE_GLYPH), Some((0, 0, 0, 0)));
}

#[test]
fn try_get_glyph_bounds_returns_non_empty_box_for_letter() {
    let glyf = load_glyf();

    let (x_min, y_min, x_max, y_max) = glyf.try_get_glyph_bounds(LETTER_GLYPH).unwrap();

    assert!(x_max > x_min, "Letter glyph should have a positive-width box.");
    assert!(y_max > y_min, "Letter glyph should have a positive-height box.");
    assert_eq!((x_min, y_min, x_max, y_max), (10, -20, 110, 180));
}

#[test]
fn try_get_glyph_bounds_matches_glyph_descriptor_header() {
    let glyf = load_glyf();

    let data = glyf.try_get_glyph_data(LETTER_GLYPH).unwrap();
    let descriptor = GlyphDescriptor::new(data).unwrap();

    let (x_min, y_min, x_max, y_max) = glyf.try_get_glyph_bounds(LETTER_GLYPH).unwrap();

    // The header-only read must agree with the descriptor's parsed bounds.
    let bounds = descriptor.conservative_bounds();

    assert_eq!(bounds.x, x_min as f64);
    assert_eq!(bounds.y, y_min as f64);
    assert_eq!(bounds.width, (x_max - x_min) as f64);
    assert_eq!(bounds.height, (y_max - y_min) as f64);
}

#[test]
fn get_glyph_bounds_reads_a_batch() {
    let glyf = load_glyf();

    let mut bounds = [GlyphBounds::new(9, 9, 9, 9); 5];

    glyf.get_glyph_bounds(&[1, 0, 2, 4, 500], &mut bounds);

    assert_eq!(
        bounds,
        [
            GlyphBounds::new(10, -20, 110, 180),
            GlyphBounds::default(), // empty glyph
            GlyphBounds::new(10, -20, 260, 180),
            GlyphBounds::default(), // too short for a header
            GlyphBounds::default(), // out of range
        ]
    );
}

fn find_composite_glyph(glyf: &GlyfTable) -> i32 {
    for i in 0..glyf.glyph_count() {
        if let Some(data) = glyf.try_get_glyph_data(i) {
            if data.len() >= 2 {
                let number_of_contours = i16::from_be_bytes([data.span()[0], data.span()[1]]);

                // Negative contour count marks a composite glyph.
                if number_of_contours < 0 {
                    return i;
                }
            }
        }
    }

    -1
}

#[derive(Default)]
struct RecordingGeometryContext {
    begin_figure_count: Cell<i32>,
    end_figure_count: Cell<i32>,
    any_figure_open: Cell<bool>,
    points: RefCell<Vec<Point>>,
    fill_rule: Cell<Option<FillRule>>,
}

impl RecordingGeometryContext {
    fn all_figures_closed(&self) -> bool {
        !self.any_figure_open.get()
    }
}

impl IGeometryContext for RecordingGeometryContext {
    fn begin_figure(&mut self, start_point: Point, _is_filled: bool) {
        self.begin_figure_count.set(self.begin_figure_count.get() + 1);
        self.points.borrow_mut().push(start_point);
    }

    fn line_to(&mut self, point: Point, _is_stroked: bool) {
        self.points.borrow_mut().push(point);
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, _is_stroked: bool) {
        self.points.borrow_mut().extend([control_point, end_point]);
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, _is_stroked: bool) {
        self.points.borrow_mut().extend([control_point1, control_point2, end_point]);
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
        self.points.borrow_mut().push(point);
    }

    fn end_figure(&mut self, is_closed: bool) {
        self.end_figure_count.set(self.end_figure_count.get() + 1);

        if !is_closed {
            self.any_figure_open.set(true);
        }
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.fill_rule.set(Some(fill_rule));
    }

    fn dispose(&mut self) {}
}
