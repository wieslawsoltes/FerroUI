use crate::media::fonts::tables::big_endian_binary_reader::FontTableError;
use crate::media::fonts::tables::decycler::DecyclerException;
use crate::media::fonts::tables::head_table::HeadTable;
use crate::media::fonts::tables::loca_table::LocaTable;
use crate::media::fonts::tables::maxp_table::MaxpTable;
use crate::media::fonts::OpenTypeTag;
use crate::media::{FillRule, GlyphBounds, IFontMemory, SweepDirection};
use crate::platform::IGeometryContext;
use crate::utilities::ReadOnlyMemory;
use crate::{Matrix, Point, Size, Vector};

use super::composite_flags::CompositeFlags;
use super::composite_glyph::CompositeGlyph;
use super::glyph_component::GlyphComponent;
use super::glyph_decycler::GlyphDecycler;
use super::glyph_descriptor::GlyphDescriptor;
use super::glyph_flag::GlyphFlag;
use super::simple_glyph::SimpleGlyph;

/// Building a glyph outline stopped: a component cycle, too deep a component
/// nesting or malformed glyph data. Every case makes
/// [`GlyfTable::try_build_glyph_geometry`] return `false`.
struct BuildError;

impl From<DecyclerException> for BuildError {
    fn from(_: DecyclerException) -> Self {
        BuildError
    }
}

impl From<FontTableError> for BuildError {
    fn from(_: FontTableError) -> Self {
        BuildError
    }
}

/// Reader for the 'glyf' table. Provides on-demand access to individual glyph
/// data using the 'loca' index. Designed for high-performance lookups on the
/// hot path.
#[derive(Clone, Debug)]
pub struct GlyfTable {
    glyf_data: ReadOnlyMemory<u8>,
    loca_table: LocaTable,
}

impl GlyfTable {
    pub const TABLE_NAME: &'static str = "glyf";
    pub const TAG: OpenTypeTag = OpenTypeTag::from_chars('g', 'l', 'y', 'f');

    pub(crate) fn new(glyf_data: ReadOnlyMemory<u8>, loca_table: LocaTable) -> Self {
        Self { glyf_data, loca_table }
    }

    /// Gets the total number of glyphs defined in the font.
    pub fn glyph_count(&self) -> i32 {
        self.loca_table.glyph_count()
    }

    /// Attempts to load the 'glyf' table from the specified font data.
    ///
    /// Never fails: `None` when the font has no 'glyf' or no 'loca' table.
    ///
    /// * `font` - The font from which to retrieve the 'glyf' table.
    /// * `head` - The 'head' table containing font header information required
    ///   for loading the 'glyf' table.
    /// * `maxp` - The 'maxp' table providing maximum profile information needed
    ///   to interpret the 'glyf' table.
    pub fn try_load(font: &dyn IFontMemory, head: &HeadTable, maxp: &MaxpTable) -> Option<GlyfTable> {
        let glyf_table_data = font.try_get_table(Self::TAG)?;

        let loca_table = LocaTable::load(font, head, maxp)?;

        Some(GlyfTable::new(glyf_table_data, loca_table))
    }

    /// Attempts to retrieve the raw glyph data for the specified glyph index.
    ///
    /// If the glyph exists but has no data (for example, a missing or empty
    /// glyph), the method returns an empty memory region. If the glyph index
    /// is invalid or out of range, the method returns `None`.
    pub fn try_get_glyph_data(&self, glyph_index: i32) -> Option<ReadOnlyMemory<u8>> {
        let (start, end) = self.loca_table.try_get_offsets(glyph_index)?;

        if start == end {
            return Some(ReadOnlyMemory::empty());
        }

        // Additional safety check for glyf table bounds
        if start < 0 || end as i64 > self.glyf_data.len() as i64 || start > end {
            return None;
        }

        Some(self.glyf_data.slice(start as usize, (end - start) as usize))
    }

    /// Reads a glyph's bounding box `(x_min, y_min, x_max, y_max)` from its
    /// 'glyf' header without parsing contours.
    ///
    /// The values are the control-point bounding box stored in the glyph header
    /// (the min/max of all on- and off-curve points), in font design units.
    /// This is a slight superset of the rendered ink bounds for glyphs with
    /// off-curve points. Composite glyphs carry their overall bounding box in
    /// the header too, so no recursion is needed. Returns all-zero bounds for
    /// empty glyphs (e.g. whitespace); returns `None` when the glyph index is
    /// out of range or the glyph data is too short to contain a header.
    pub fn try_get_glyph_bounds(&self, glyph_index: i32) -> Option<(i16, i16, i16, i16)> {
        // Out of range gives None.
        let data = self.try_get_glyph_data(glyph_index)?;

        if data.is_empty() {
            // Empty glyph (e.g. whitespace): valid, zero bounds.
            return Some((0, 0, 0, 0));
        }

        let span = data.span();

        // Glyph header: int16 numberOfContours, then int16 xMin, yMin, xMax, yMax.
        if span.len() < 10 {
            return None;
        }

        Some((read_int16(span, 2), read_int16(span, 4), read_int16(span, 6), read_int16(span, 8)))
    }

    /// Reads bounding boxes for a batch of glyphs into `bounds`.
    ///
    /// The hot path for ink-bounds computation. The `glyf` and `loca` spans are
    /// fetched once for the whole batch (not per glyph), and offsets and
    /// headers are read directly. Out-of-range, empty, or malformed glyphs are
    /// written as the default (zero) box.
    ///
    /// Panics when `bounds` is shorter than `glyph_indices`.
    #[allow(dead_code)] // used by the glyph typeface's batch bounds path, which is ported separately
    pub(crate) fn get_glyph_bounds(&self, glyph_indices: &[u16], bounds: &mut [GlyphBounds]) {
        let glyf = self.glyf_data.span();
        let loca = self.loca_table.raw_data();
        let short_format = self.loca_table.is_short_format();
        let glyph_count = self.loca_table.glyph_count();
        let entry_size: usize = if short_format { 2 } else { 4 };

        for (i, &glyph_index) in glyph_indices.iter().enumerate() {
            bounds[i] = GlyphBounds::default();

            let gid = glyph_index as i32;

            if gid >= glyph_count {
                continue;
            }

            let loca_offset = gid as usize * entry_size;

            // Need both loca[gid] and loca[gid + 1].
            let Some(entries) = loca.get(loca_offset..loca_offset + 2 * entry_size) else {
                continue;
            };

            let (start, end): (i32, i32) = if short_format {
                // Short format: uint16 values stored divided by 2
                (
                    u16::from_be_bytes([entries[0], entries[1]]) as i32 * 2,
                    u16::from_be_bytes([entries[2], entries[3]]) as i32 * 2,
                )
            } else {
                (
                    u32::from_be_bytes([entries[0], entries[1], entries[2], entries[3]]) as i32,
                    u32::from_be_bytes([entries[4], entries[5], entries[6], entries[7]]) as i32,
                )
            };

            // Empty (start == end) or malformed glyph → leave the zero box.
            if end.wrapping_sub(start) < 10 || start < 0 || end as u32 as u64 > glyf.len() as u64 {
                continue;
            }

            let Some(header) = glyf.get(start as usize..start as usize + 10) else {
                continue;
            };

            bounds[i] = GlyphBounds::new(
                read_int16(header, 2),
                read_int16(header, 4),
                read_int16(header, 6),
                read_int16(header, 8),
            );
        }
    }

    /// Builds the glyph outline into the provided geometry context. Returns
    /// false for empty glyphs. Coordinates are in font design units. Composite
    /// glyphs are supported.
    pub fn try_build_glyph_geometry(&self, glyph_index: i32, transform: Matrix, context: &mut dyn IGeometryContext) -> bool {
        // TrueType outlines use the non-zero winding rule. The default geometry fill
        // rule is EvenOdd, which would XOR overlapping contours (e.g. the
        // crossbar and diagonal strokes of 'A', or composites where an accent overlaps
        // its base glyph) and leave gaps where they intersect.
        context.set_fill_rule(FillRule::NonZero);

        let decycler = GlyphDecycler::rent();

        // A cycle, too deep a nesting or malformed glyph data all give "no outline".
        let result = self
            .try_build_glyph_geometry_internal(glyph_index, context, transform, &decycler)
            .unwrap_or(false);

        GlyphDecycler::return_to_pool(decycler);

        result
    }

    /// Builds the geometry for a simple glyph by processing its contours and
    /// converting them into geometry commands.
    fn build_simple_glyph_geometry(simple_glyph: SimpleGlyph<'_>, context: &mut dyn IGeometryContext, transform: Matrix) -> bool {
        let end_pts_of_contours = simple_glyph.end_pts_of_contours();

        if end_pts_of_contours.is_empty() {
            return false;
        }

        let flags = simple_glyph.flags();
        let x_coords = simple_glyph.x_coordinates();
        let y_coords = simple_glyph.y_coordinates();
        let point_count = flags.len();

        // Materialise the points once so every contour goes through the single
        // emit_contour walker shared with the composite point-matching path.
        let mut points = Vec::with_capacity(point_count);
        let mut on_curve = Vec::with_capacity(point_count);

        for i in 0..point_count {
            points.push(Point::new(x_coords[i] as f64, y_coords[i] as f64));
            on_curve.push(flags[i].contains(GlyphFlag::OnCurvePoint));
        }

        let mut start_point_index = 0usize;

        for &end_point_index in end_pts_of_contours {
            // The parser guarantees strictly increasing endpoints below the point count.
            let end_point_index = end_point_index as usize;

            if let (Some(contour_points), Some(contour_on_curve)) =
                (points.get(start_point_index..=end_point_index), on_curve.get(start_point_index..=end_point_index))
            {
                Self::emit_contour(contour_points, contour_on_curve, transform, context);
            }

            start_point_index = end_point_index + 1;
        }

        true
    }

    /// Emits one contour's segments to the geometry context, applying `transform`.
    ///
    /// Implements the TrueType on/off-curve walk once for every caller: the
    /// figure starts at the first point when it is on-curve, at the last point
    /// when only that one is on-curve, and at their implied midpoint when both
    /// are off-curve; consecutive off-curve points imply an on-curve midpoint
    /// between them; the contour closes back to the start through a trailing
    /// off-curve control point when one is pending.
    fn emit_contour(points: &[Point], on_curve: &[bool], transform: Matrix, context: &mut dyn IGeometryContext) {
        let point_count = points.len();

        if point_count == 0 {
            return;
        }

        let figure_start;
        let walk_start;
        let walk_count;

        if on_curve[0] {
            figure_start = points[0];
            walk_start = 1;
            walk_count = point_count - 1;
        } else if on_curve[point_count - 1] {
            // The last point is consumed as the start; the walk covers the rest.
            figure_start = points[point_count - 1];
            walk_start = 0;
            walk_count = point_count - 1;
        } else {
            let first = points[0];
            let last = points[point_count - 1];
            figure_start = Point::new((first.x + last.x) / 2.0, (first.y + last.y) / 2.0);
            walk_start = 0;
            walk_count = point_count;
        }

        context.begin_figure(transform.transform(figure_start), true);

        let mut pending_control = Point::default();
        let mut has_pending_control = false;

        for i in 0..walk_count {
            let index = walk_start + i;
            let point = points[index];

            if on_curve[index] {
                if has_pending_control {
                    context.quadratic_bezier_to(transform.transform(pending_control), transform.transform(point), true);
                    has_pending_control = false;
                } else {
                    context.line_to(transform.transform(point), true);
                }
            } else {
                if has_pending_control {
                    // Two consecutive off-curve points -> implied on-curve midpoint.
                    let implied =
                        Point::new((pending_control.x + point.x) / 2.0, (pending_control.y + point.y) / 2.0);
                    context.quadratic_bezier_to(transform.transform(pending_control), transform.transform(implied), true);
                }

                pending_control = point;
                has_pending_control = true;
            }
        }

        // Close back to the start: through the trailing control point when one is pending,
        // with an explicit line otherwise (end_figure's implicit close is then zero-length).
        if has_pending_control {
            context.quadratic_bezier_to(transform.transform(pending_control), transform.transform(figure_start), true);
        } else {
            context.line_to(transform.transform(figure_start), true);
        }

        context.end_figure(true);
    }

    /// Creates a transformation matrix for a composite glyph component based
    /// on its flags and transformation parameters.
    fn create_component_transform(component: &GlyphComponent) -> Matrix {
        let flags = component.flags;

        let (mut tx, mut ty) = (0.0, 0.0);

        if flags.contains(CompositeFlags::ArgsAreXYValues) {
            tx = component.arg1 as f64;
            ty = component.arg2 as f64;
        }

        let scale = Self::create_component_scale(component);

        Matrix::new(scale.m11, scale.m12, scale.m21, scale.m22, tx, ty)
    }

    /// Attempts to build the geometry for the specified glyph and adds it to
    /// the provided geometry context.
    ///
    /// This method processes both simple and composite glyphs. For composite
    /// glyphs, recursion is used and the decycler prevents cycles. `Ok(false)`
    /// when the glyph is empty or invalid.
    fn try_build_glyph_geometry_internal(
        &self,
        glyph_index: i32,
        context: &mut dyn IGeometryContext,
        transform: Matrix,
        decycler: &GlyphDecycler,
    ) -> Result<bool, BuildError> {
        let _guard = decycler.enter(glyph_index)?;

        let glyph_data = match self.try_get_glyph_data(glyph_index) {
            Some(glyph_data) if !glyph_data.is_empty() => glyph_data,
            _ => return Ok(false),
        };

        let descriptor = GlyphDescriptor::new(glyph_data)?;

        if descriptor.is_simple_glyph() {
            Ok(Self::build_simple_glyph_geometry(descriptor.simple_glyph(), context, transform))
        } else {
            self.build_composite_glyph_geometry(descriptor.composite_glyph()?, context, transform, decycler)
        }
    }

    /// Builds the geometry for a composite glyph by recursively processing its
    /// components. `Ok(true)` if at least one component was successfully
    /// processed.
    fn build_composite_glyph_geometry(
        &self,
        composite_glyph: CompositeGlyph<'_>,
        context: &mut dyn IGeometryContext,
        transform: Matrix,
        decycler: &GlyphDecycler,
    ) -> Result<bool, BuildError> {
        let components = composite_glyph.components();

        if components.is_empty() {
            return Ok(false);
        }

        // When ARGS_ARE_XY_VALUES is clear, arg1/arg2 are point numbers: the component
        // is placed by making one of its points coincide with a point in the
        // already-assembled glyph (point matching), not by an x/y offset. The streaming
        // loop below doesn't retain points, so route those composites through the
        // materialising path instead. The flag is computed once while parsing.
        if composite_glyph.uses_point_matching() {
            return self.build_point_matched_composite(components, context, transform, decycler);
        }

        let mut has_geometry = false;

        for component in components {
            let component_transform = Self::create_component_transform(component);
            let combined_transform = component_transform * transform;

            let mut wrapped_context = TransformingGeometryContext::new(&mut *context, combined_transform);

            if self.try_build_glyph_geometry_internal(
                component.glyph_index as i32,
                &mut wrapped_context,
                Matrix::IDENTITY,
                decycler,
            )? {
                has_geometry = true;
            }
        }

        Ok(has_geometry)
    }

    /// Builds a composite glyph in which at least one component is placed by
    /// point matching.
    ///
    /// Unlike the streaming fast path, this materialises every component's
    /// transformed points into a single buffer so a point-matched component
    /// can be aligned to a point of the already-assembled glyph, then emits the
    /// assembled contours. Only reached for the rare composites that actually
    /// use point matching. Returns `Ok(false)` (no outline) rather than an
    /// incorrect one for cases not yet supported: a component that is itself
    /// composite, or a point index that is out of range or refers to a phantom
    /// point (which this reader does not materialise).
    fn build_point_matched_composite(
        &self,
        components: &[GlyphComponent],
        context: &mut dyn IGeometryContext,
        transform: Matrix,
        decycler: &GlyphDecycler,
    ) -> Result<bool, BuildError> {
        let mut outline = ResolvedOutline::new(64);

        for component in components {
            let component_start = outline.point_count();

            // Resolve the component's points into composite space with its 2x2 scale
            // applied (but not the placement offset, which is computed next).
            if !self.try_resolve_simple_glyph_points(
                component.glyph_index as i32,
                Self::create_component_scale(component),
                decycler,
                &mut outline,
            )? {
                return Ok(false);
            }

            let offset = if component.flags.contains(CompositeFlags::ArgsAreXYValues) {
                // Signed x/y offset (the unscaled-offset default).
                Vector::new(component.arg1 as f64, component.arg2 as f64)
            } else {
                // Point matching: arg1 is a point already placed by an earlier component,
                // arg2 is a point of this component. They are unsigned point numbers, but
                // CompositeGlyph parses the raw bytes/words as signed, so reinterpret to
                // unsigned here (two's-complement round-trip).
                let args_are_words = component.flags.contains(CompositeFlags::ArgsAreWords);
                let (parent_point, component_point) = if args_are_words {
                    (component.arg1 as u16 as usize, component.arg2 as u16 as usize)
                } else {
                    (component.arg1 as u8 as usize, component.arg2 as u8 as usize)
                };

                let component_point_count = outline.point_count() - component_start;

                if parent_point >= component_start || component_point >= component_point_count {
                    // Out of range, or references a phantom point this reader does not
                    // materialise — bail rather than place the component incorrectly.
                    return Ok(false);
                }

                let difference = outline.get_point(parent_point) - outline.get_point(component_start + component_point);

                Vector::new(difference.x, difference.y)
            };

            let point_count = outline.point_count();

            outline.translate_range(component_start, point_count, offset);
        }

        if outline.point_count() == 0 {
            return Ok(false);
        }

        Self::emit_resolved_outline(&outline, transform, context);

        Ok(true)
    }

    /// Appends a simple glyph's points (transformed by `transform`) and
    /// contour boundaries to `outline`. `Ok(false)` for a component that is
    /// itself composite (nested point matching is not supported yet); an empty
    /// glyph contributes no points and gives `Ok(true)`.
    fn try_resolve_simple_glyph_points(
        &self,
        glyph_index: i32,
        transform: Matrix,
        decycler: &GlyphDecycler,
        outline: &mut ResolvedOutline,
    ) -> Result<bool, BuildError> {
        let _guard = decycler.enter(glyph_index)?;

        let glyph_data = match self.try_get_glyph_data(glyph_index) {
            Some(glyph_data) if !glyph_data.is_empty() => glyph_data,
            _ => return Ok(true),
        };

        let descriptor = GlyphDescriptor::new(glyph_data)?;

        if !descriptor.is_simple_glyph() {
            return Ok(false);
        }

        let simple_glyph = descriptor.simple_glyph();

        let flags = simple_glyph.flags();
        let x_coords = simple_glyph.x_coordinates();
        let y_coords = simple_glyph.y_coordinates();

        let mut start = 0usize;

        for &end in simple_glyph.end_pts_of_contours() {
            let end = end as usize;

            for i in start..=end {
                let point = transform.transform(Point::new(x_coords[i] as f64, y_coords[i] as f64));
                outline.add_point(point, flags[i].contains(GlyphFlag::OnCurvePoint));
            }

            outline.end_contour();
            start = end + 1;
        }

        Ok(true)
    }

    /// Builds the 2x2 scale/transform of a composite component (without any
    /// translation; the placement offset is applied separately).
    fn create_component_scale(component: &GlyphComponent) -> Matrix {
        let flags = component.flags;

        let (m11, m12, m21, m22);

        if flags.contains(CompositeFlags::WeHaveAScale) {
            m11 = component.scale as f64;
            m22 = component.scale as f64;
            m12 = 0.0;
            m21 = 0.0;
        } else if flags.contains(CompositeFlags::WeHaveAnXAndYScale) {
            m11 = component.scale_x as f64;
            m22 = component.scale_y as f64;
            m12 = 0.0;
            m21 = 0.0;
        } else if flags.contains(CompositeFlags::WeHaveATwoByTwo) {
            m11 = component.scale_x as f64;
            m12 = component.scale01 as f64;
            m21 = component.scale10 as f64;
            m22 = component.scale_y as f64;
        } else {
            m11 = 1.0;
            m22 = 1.0;
            m12 = 0.0;
            m21 = 0.0;
        }

        Matrix::new(m11, m12, m21, m22, 0.0, 0.0)
    }

    /// Emits the contours of a materialised outline to the geometry context,
    /// applying `transform`, via the shared `emit_contour` walker.
    fn emit_resolved_outline(outline: &ResolvedOutline, transform: Matrix, context: &mut dyn IGeometryContext) {
        let points = outline.points();
        let on_curve = outline.on_curve();

        let mut start_point_index = 0isize;

        for &end_point_index in outline.contour_ends() {
            let point_count = end_point_index - start_point_index + 1;

            if point_count > 0 {
                let range = start_point_index as usize..=end_point_index as usize;

                Self::emit_contour(&points[range.clone()], &on_curve[range], transform, context);
            }

            start_point_index = end_point_index + 1;
        }
    }
}

#[inline]
fn read_int16(span: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes([span[offset], span[offset + 1]])
}

/// A growable buffer of resolved (transformed) outline points used only by the
/// point-matching composite path.
struct ResolvedOutline {
    points: Vec<Point>,
    on_curve: Vec<bool>,
    /// The index of the last point of each contour.
    contour_ends: Vec<isize>,
}

impl ResolvedOutline {
    fn new(capacity: usize) -> Self {
        Self {
            points: Vec::with_capacity(capacity),
            on_curve: Vec::with_capacity(capacity),
            contour_ends: Vec::with_capacity(16),
        }
    }

    fn point_count(&self) -> usize {
        self.points.len()
    }

    fn points(&self) -> &[Point] {
        &self.points
    }

    fn on_curve(&self) -> &[bool] {
        &self.on_curve
    }

    fn contour_ends(&self) -> &[isize] {
        &self.contour_ends
    }

    fn get_point(&self, index: usize) -> Point {
        self.points[index]
    }

    fn add_point(&mut self, point: Point, on_curve: bool) {
        self.points.push(point);
        self.on_curve.push(on_curve);
    }

    fn end_contour(&mut self) {
        self.contour_ends.push(self.points.len() as isize - 1);
    }

    fn translate_range(&mut self, from_inclusive: usize, to_exclusive: usize, offset: Vector) {
        for point in &mut self.points[from_inclusive..to_exclusive] {
            *point += offset;
        }
    }
}

/// Wrapper that applies a matrix transform to coordinates before delegating to
/// the real context.
struct TransformingGeometryContext<'a> {
    inner: &'a mut dyn IGeometryContext,
    matrix: Matrix,
}

impl<'a> TransformingGeometryContext<'a> {
    fn new(inner: &'a mut dyn IGeometryContext, matrix: Matrix) -> Self {
        Self { inner, matrix }
    }
}

impl IGeometryContext for TransformingGeometryContext<'_> {
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    ) {
        self.inner.arc_to(
            self.matrix.transform(point),
            size,
            rotation_angle,
            is_large_arc,
            sweep_direction,
            is_stroked,
        );
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        self.inner.begin_figure(self.matrix.transform(start_point), is_filled);
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        self.inner.cubic_bezier_to(
            self.matrix.transform(control_point1),
            self.matrix.transform(control_point2),
            self.matrix.transform(end_point),
            is_stroked,
        );
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        self.inner
            .quadratic_bezier_to(self.matrix.transform(control_point), self.matrix.transform(end_point), is_stroked);
    }

    fn line_to(&mut self, point: Point, is_stroked: bool) {
        self.inner.line_to(self.matrix.transform(point), is_stroked);
    }

    fn end_figure(&mut self, is_closed: bool) {
        self.inner.end_figure(is_closed);
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.inner.set_fill_rule(fill_rule);
    }

    fn dispose(&mut self) {}
}
