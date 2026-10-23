//! Pie, doughnut and radial gauge measurement.

use crate::axis::format_number;
use crate::scene::*;
use crate::series::{PolarLabelsPosition, RadialAlignment, Series};
use ferroui_base::{Point, Rect, Size};
use std::collections::BTreeMap;
use std::f64::consts::FRAC_PI_2;

#[derive(Clone, Debug, PartialEq)]
pub struct PieOptions {
    pub draw_margin: Option<Rect>,
    pub initial_rotation: f64,
    pub max_angle: f64,
    pub min_value: f64,
    pub max_value: Option<f64>,
    pub clockwise: bool,
}
impl Default for PieOptions {
    fn default() -> Self {
        Self {
            draw_margin: None,
            initial_rotation: 0.0,
            max_angle: 360.0,
            min_value: 0.0,
            max_value: None,
            clockwise: true,
        }
    }
}

pub struct PieChartEngine;
impl PieChartEngine {
    pub fn measure(
        size: Size,
        series: &[Series],
        options: &PieOptions,
    ) -> Result<Scene, ChartError> {
        Self::measure_with_labels(size, series, options, |text, size| {
            Size::new(text.chars().count() as f64 * size * 0.6, size * 1.2)
        })
    }

    /// Native hosts supply their text shaper so outer label reservation and
    /// centered labels use the same font metrics as the final drawing.
    pub fn measure_with_labels(
        size: Size,
        series: &[Series],
        options: &PieOptions,
        mut measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Result<Scene, ChartError> {
        validate_size(size)?;
        if ![
            options.initial_rotation,
            options.max_angle,
            options.min_value,
        ]
        .iter()
        .all(|v| v.is_finite())
            || options.max_angle < 0.0
            || options.max_angle > 360.0
            || options
                .max_value
                .is_some_and(|v| !v.is_finite() || v <= options.min_value)
        {
            return Err(ChartError::InvalidData(
                "pie angle must be in [0,360] and gauge limits must be ordered and finite".into(),
            ));
        }
        let plot = options.draw_margin.unwrap_or(Rect::from_size(size));
        if [plot.x, plot.y, plot.width, plot.height]
            .iter()
            .any(|v| !v.is_finite())
            || plot.width < 0.0
            || plot.height < 0.0
        {
            return Err(ChartError::InvalidData("invalid pie draw margin".into()));
        }
        let mut scene = Scene::new(size, plot);
        scene.automatic_finding_strategy = FindingStrategy::CompareAll;
        let center = Point::new(plot.x + plot.width * 0.5, plot.y + plot.height * 0.5);
        let mut max_push: f64 = 0.0;
        let mut label_reservation: f64 = 0.0;
        let mut totals = BTreeMap::<(u32, u64), f64>::new();
        let mut label_sizes = Vec::with_capacity(series.len());
        for (si, s) in series.iter().enumerate() {
            let mut sizes = Vec::new();
            if !s.visible {
                label_sizes.push(sizes);
                continue;
            }
            if [
                s.inner_radius,
                s.outer_radius_offset,
                s.push_out,
                s.hover_push_out,
                s.relative_inner_radius,
                s.relative_outer_radius,
                s.max_radial_column_width,
                s.corner_radius,
                s.stroke_width,
                s.opacity,
                s.data_labels_size,
                s.data_labels_rotation,
            ]
            .iter()
            .any(|v| !v.is_finite())
                || s.max_radial_column_width < 0.0
            {
                return Err(ChartError::InvalidData("pie dimensions and paint values must be finite and maximum radial width must be nonnegative".into()));
            }
            if !s.is_gauge && s.values.iter().any(|c| c.is_finite()) {
                max_push = max_push.max(s.push_out.max(s.hover_push_out).max(0.0));
            }
            if !s.is_fill_series {
                scene.legend.push(LegendItem {
                    series: si,
                    name: if s.name.is_empty() {
                        format!("Series {}", si + 1)
                    } else {
                        s.name.clone()
                    },
                    color: s.color(si),
                });
            }
            for c in &s.values {
                if c.is_finite() {
                    if c.primary_value < 0.0 {
                        return Err(ChartError::InvalidData(
                            "pie values must be nonnegative".into(),
                        ));
                    }
                    let key = pie_key(s, c.secondary_value);
                    let total = totals.entry(key).or_default();
                    *total += c.primary_value;
                    if !total.is_finite() && options.max_value.is_none() {
                        return Err(ChartError::InvalidData(
                            "pie total exceeds the finite numerical range".into(),
                        ));
                    }
                }
                let measured = if s.data_labels && !s.is_fill_series && c.is_finite() {
                    measure_text(&format_number(c.primary_value), s.data_labels_size)
                } else {
                    Size::default()
                };
                if ![measured.width, measured.height]
                    .iter()
                    .all(|v| v.is_finite() && *v >= 0.0)
                {
                    return Err(ChartError::InvalidData(
                        "text measurement must return finite nonnegative dimensions".into(),
                    ));
                }
                if s.polar_labels_position == PolarLabelsPosition::Outer {
                    label_reservation =
                        label_reservation.max(1.5 * measured.width.hypot(measured.height));
                }
                sizes.push(measured);
            }
            label_sizes.push(sizes);
        }
        let mut stacks = BTreeMap::<(u32, u64), f64>::new();
        let mut slices = Vec::new();
        for (si, s) in series.iter().enumerate().filter(|(_, s)| s.visible) {
            // The source applies OuterRadiusOffset to the diameter. Negative
            // offsets intentionally enlarge a gauge background (issue #2131).
            let stroke = if s.stroke.is_some() || s.stroke_brush.is_some() {
                s.stroke_width.max(0.0)
            } else {
                0.0
            };
            let outer = ((plot.width.min(plot.height)
                - 2.0 * stroke
                - 2.0 * max_push
                - label_reservation
                - s.outer_radius_offset)
                * 0.5)
                .max(0.0);
            let minimum = s.inner_radius.max(0.0).min(outer);
            let count = s.values.len().max(1) as f64;
            let step = (outer - minimum) / count;
            let (mut relative_inner, mut relative_outer) =
                (s.relative_inner_radius, s.relative_outer_radius);
            // CorePieSeries.BeginMeasure clamps the first radial slot, then
            // applies the resulting offsets to every concentric value.
            if step - relative_outer > s.max_radial_column_width {
                let difference = step - relative_outer - s.max_radial_column_width;
                match s.radial_align {
                    RadialAlignment::Outer => {
                        relative_outer = 0.0;
                        relative_inner = difference;
                    }
                    RadialAlignment::Center => {
                        relative_outer = difference * 0.5;
                        relative_inner = difference * 0.5;
                    }
                    RadialAlignment::Inner => {
                        relative_outer = difference;
                        relative_inner = 0.0;
                    }
                }
            }
            for (index, c) in s.values.iter().enumerate() {
                if !c.is_finite() {
                    continue;
                }
                let key = pie_key(s, c.secondary_value);
                let total = options
                    .max_value
                    .unwrap_or_else(|| totals.get(&key).copied().unwrap_or_default());
                let stack = stacks.entry(key).or_default();
                let range = (total - options.min_value).max(0.0);
                let (mut start, mut sweep) = if total <= 0.0 || range <= 0.0 {
                    (0.0, 0.0)
                } else if s.is_relative_to_min_value {
                    let start = stack.min(range) / range * options.max_angle;
                    let top = (*stack + c.primary_value).min(range);
                    (start, (top / range * options.max_angle - start).max(0.0))
                } else {
                    let start = stack.min(total) / total * options.max_angle;
                    let top = (*stack + c.primary_value).min(total);
                    (
                        start,
                        ((top - options.min_value) / range * options.max_angle - start).max(0.0),
                    )
                };
                *stack += c.primary_value;
                sweep = sweep.min(options.max_angle);
                if !options.clockwise {
                    start = options.max_angle - start - sweep;
                }
                if s.is_fill_series {
                    start = 0.0;
                    sweep = (options.max_angle - 0.1).max(0.0);
                }
                if sweep <= 0.0 {
                    continue;
                }
                start += options.initial_rotation;
                // Empty values still reserve their radial slot: index, not the
                // number of nonempty points, controls the concentric position.
                let inner = (minimum + index as f64 * step + relative_inner).max(0.0);
                let radius = (minimum + (index + 1) as f64 * step - relative_outer).max(0.0);
                if inner >= radius {
                    continue;
                }
                let middle = (start + sweep * 0.5).to_radians();
                let displaced = Point::new(
                    center.x + middle.cos() * s.push_out.max(0.0),
                    center.y + middle.sin() * s.push_out.max(0.0),
                );
                slices.push(PieSlice {
                    series: si,
                    point: index,
                    center: displaced,
                    inner,
                    outer: radius,
                    label_outer: radius + relative_outer,
                    start,
                    sweep,
                });
            }
        }
        // Gauge backgrounds have source ZIndex -1 and must stay behind their
        // values even when the builder appends them at the end of the collection.
        slices.sort_by_key(|slice| !series[slice.series].is_fill_series);
        for slice in slices {
            let si = slice.series;
            let s = &series[si];
            let c = s.values[slice.point];
            let id = PointId {
                series: si,
                point: slice.point,
            };
            let path = rounded_annular_sector_path(
                slice.center,
                slice.inner,
                slice.outer,
                slice.start,
                slice.sweep,
                s.corner_radius,
                s.inverted_corner_radius,
            );
            let mut style = s.style(si, true);
            if s.stroke.is_none() && s.stroke_brush.is_none() {
                style.stroke = None;
            }
            scene.primitives.push(
                Primitive::new(Geometry::Path(path), style)
                    .clipped(plot)
                    .identified(id),
            );
            if s.is_fill_series {
                continue;
            }
            let middle = (slice.start + slice.sweep * 0.5).to_radians();
            let hit_center = Point::new(
                slice.center.x + middle.cos() * (slice.inner + slice.outer) * 0.5,
                slice.center.y + middle.sin() * (slice.inner + slice.outer) * 0.5,
            );
            scene.hits.push(HitRegion {
                id,
                shape: HitShape::AnnularSector {
                    center: slice.center,
                    inner_radius: slice.inner,
                    outer_radius: slice.outer,
                    start_angle: slice.start,
                    sweep_angle: slice.sweep,
                },
                center: hit_center,
                value: c.primary_value,
                label: format!(
                    "{}{}{}",
                    s.name,
                    if s.name.is_empty() { "" } else { ": " },
                    format_number(c.primary_value)
                ),
            });
            if s.data_labels {
                let measured = label_sizes[si][slice.point];
                let (location, anchor, rotation) = label_position(center, &slice, s, measured);
                scene.primitives.push(
                    Primitive::new(
                        Geometry::Text {
                            origin: Point::new(location.x, location.y - measured.height * 0.5),
                            text: format_number(c.primary_value),
                            size: s.data_labels_size,
                            rotation,
                            anchor,
                        },
                        PaintStyle::fill(s.data_labels_color.unwrap_or_else(|| s.color(si))),
                    )
                    .identified(id),
                );
            }
        }
        Ok(scene)
    }
}

#[derive(Clone, Copy)]
struct PieSlice {
    series: usize,
    point: usize,
    center: Point,
    inner: f64,
    outer: f64,
    label_outer: f64,
    start: f64,
    sweep: f64,
}
fn pie_key(series: &Series, category: f64) -> (u32, u64) {
    (
        series.stack_group,
        if category == 0.0 {
            0
        } else {
            category.to_bits()
        },
    )
}
fn label_position(
    center: Point,
    slice: &PieSlice,
    s: &Series,
    size: Size,
) -> (Point, TextAnchor, f64) {
    let middle = slice.start + slice.sweep * 0.5;
    let (angle, radius) = match s.polar_labels_position {
        PolarLabelsPosition::ChartCenter => {
            return (center, TextAnchor::Center, s.data_labels_rotation)
        }
        PolarLabelsPosition::Start => (slice.start, (slice.inner + slice.label_outer) * 0.5),
        PolarLabelsPosition::End => (
            slice.start + slice.sweep,
            (slice.inner + slice.label_outer) * 0.5,
        ),
        PolarLabelsPosition::Outer => (
            middle,
            slice.label_outer + 0.45 * size.width.hypot(size.height),
        ),
        PolarLabelsPosition::Middle => (
            middle,
            slice.inner + (slice.label_outer - slice.inner) * if s.is_gauge { 0.5 } else { 0.65 },
        ),
    };
    let (anchor, rotation) = match s.polar_labels_position {
        PolarLabelsPosition::Start | PolarLabelsPosition::End => {
            let angle = angle.rem_euclid(360.0);
            let lower = angle > 180.0;
            let at_end = s.polar_labels_position == PolarLabelsPosition::End;
            (
                if lower == at_end {
                    TextAnchor::Start
                } else {
                    TextAnchor::End
                },
                angle - if lower { -90.0 } else { 90.0 },
            )
        }
        _ => (TextAnchor::Center, s.data_labels_rotation),
    };
    let radians = angle.to_radians();
    (
        Point::new(
            center.x + radians.cos() * radius,
            center.y + radians.sin() * radius,
        ),
        anchor,
        rotation,
    )
}

/// Cubics split at quarter turns represent full circles without degenerate arc endpoints.
pub fn annular_sector_path(
    center: Point,
    inner: f64,
    outer: f64,
    start_degrees: f64,
    sweep_degrees: f64,
) -> Vec<PathCommand> {
    let start = start_degrees.to_radians();
    let sweep = sweep_degrees.to_radians();
    let point = |radius: f64, angle: f64| {
        Point::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        )
    };
    let mut path = Vec::new();
    path.push(PathCommand::MoveTo(point(outer, start)));
    append_arc(&mut path, center, outer, start, sweep);
    if inner > 0.0 {
        path.push(PathCommand::LineTo(point(inner, start + sweep)));
        append_arc(&mut path, center, inner, start + sweep, -sweep);
    } else {
        path.push(PathCommand::LineTo(center));
    }
    path.push(PathCommand::Close);
    path
}

/// Rounded radial corners adapted from DoughnutGeometry.RoundedDraw. FerroUI
/// receives cubic arcs instead of a backend-specific ArcTo command. Full rings
/// remain exactly closed, and narrow wedges cap the corner before it can cross
/// the opposite radial edge.
pub fn rounded_annular_sector_path(
    center: Point,
    inner: f64,
    outer: f64,
    start_degrees: f64,
    sweep_degrees: f64,
    corner_radius: f64,
    inverted: bool,
) -> Vec<PathCommand> {
    if corner_radius <= 0.0 || sweep_degrees.abs() >= 360.0 || outer <= inner {
        return annular_sector_path(center, inner, outer, start_degrees, sweep_degrees);
    }
    let start = start_degrees.to_radians();
    let sweep = sweep_degrees.to_radians();
    let end = start + sweep;
    let mut corner = corner_radius.min((outer - inner) * 0.5).max(0.0);
    if sweep.abs() < std::f64::consts::PI {
        let tangent = (sweep.abs() * 0.5).tan();
        corner = corner.min(outer * tangent / (1.0 + tangent));
    }
    if corner <= f64::EPSILON {
        return annular_sector_path(center, inner, outer, start_degrees, sweep_degrees);
    }
    let direction = sweep.signum() * if inverted { -1.0 } else { 1.0 };
    let outer_angle = direction * (corner / (outer - corner)).atan();
    let inner_angle = direction * (corner / (inner + corner)).atan();
    let at = |radius: f64, angle: f64| {
        Point::new(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        )
    };
    let mut path = vec![PathCommand::MoveTo(at(inner + corner, start))];
    path.push(PathCommand::LineTo(at(outer - corner, start)));
    append_tangent_arc(
        &mut path,
        at(outer, start),
        at(outer, start + outer_angle),
        corner,
    );
    path.push(PathCommand::LineTo(at(outer, start + outer_angle)));
    append_arc(
        &mut path,
        center,
        outer,
        start + outer_angle,
        sweep - 2.0 * outer_angle,
    );
    append_tangent_arc(&mut path, at(outer, end), at(outer - corner, end), corner);
    path.push(PathCommand::LineTo(at(inner + corner, end)));
    if corner < inner {
        append_tangent_arc(
            &mut path,
            at(inner, end),
            at(inner, end - inner_angle),
            corner,
        );
        path.push(PathCommand::LineTo(at(inner, end - inner_angle)));
        append_arc(
            &mut path,
            center,
            inner,
            end - inner_angle,
            -(sweep - 2.0 * inner_angle),
        );
        append_tangent_arc(
            &mut path,
            at(inner, start),
            at(inner + corner, start),
            corner,
        );
    } else {
        append_tangent_arc(&mut path, center, at(inner + corner, start), inner);
    }
    path.push(PathCommand::Close);
    path
}

fn append_tangent_arc(path: &mut Vec<PathCommand>, corner: Point, toward: Point, radius: f64) {
    let Some(previous) = path.iter().rev().find_map(|command| match command {
        PathCommand::MoveTo(point) | PathCommand::LineTo(point) => Some(*point),
        PathCommand::CubicTo { end, .. } => Some(*end),
        PathCommand::Close => None,
    }) else {
        return;
    };
    let (ux, uy) = (previous.x - corner.x, previous.y - corner.y);
    let (vx, vy) = (toward.x - corner.x, toward.y - corner.y);
    let (u_len, v_len) = (ux.hypot(uy), vx.hypot(vy));
    if radius <= 0.0 || u_len <= f64::EPSILON || v_len <= f64::EPSILON {
        path.push(PathCommand::LineTo(corner));
        return;
    }
    let (ux, uy, vx, vy) = (ux / u_len, uy / u_len, vx / v_len, vy / v_len);
    let half_angle = ((ux * vx + uy * vy).clamp(-1.0, 1.0)).acos() * 0.5;
    let bisector_length = (ux + vx).hypot(uy + vy);
    if half_angle.sin().abs() < 1e-12 || half_angle.cos().abs() < 1e-12 || bisector_length < 1e-12 {
        path.push(PathCommand::LineTo(corner));
        return;
    }
    let distance = radius / half_angle.tan();
    let first = Point::new(corner.x + ux * distance, corner.y + uy * distance);
    let last = Point::new(corner.x + vx * distance, corner.y + vy * distance);
    let center_distance = radius / half_angle.sin();
    let center = Point::new(
        corner.x + (ux + vx) / bisector_length * center_distance,
        corner.y + (uy + vy) / bisector_length * center_distance,
    );
    let start = (first.y - center.y).atan2(first.x - center.x);
    let end = (last.y - center.y).atan2(last.x - center.x);
    let sweep = if uy * vx - ux * vy > 0.0 {
        (end - start).rem_euclid(std::f64::consts::TAU)
    } else {
        -(start - end).rem_euclid(std::f64::consts::TAU)
    };
    path.push(PathCommand::LineTo(first));
    append_arc(path, center, radius, start, sweep);
}

fn append_arc(path: &mut Vec<PathCommand>, center: Point, radius: f64, start: f64, sweep: f64) {
    let segments = (sweep.abs() / FRAC_PI_2).ceil().max(1.0) as usize;
    let delta = sweep / segments as f64;
    for index in 0..segments {
        let a = start + index as f64 * delta;
        let b = a + delta;
        let k = 4.0 / 3.0 * (delta / 4.0).tan();
        let p1 = Point::new(
            center.x + radius * (a.cos() - k * a.sin()),
            center.y + radius * (a.sin() + k * a.cos()),
        );
        let p2 = Point::new(
            center.x + radius * (b.cos() + k * b.sin()),
            center.y + radius * (b.sin() - k * b.cos()),
        );
        let end = Point::new(center.x + radius * b.cos(), center.y + radius * b.sin());
        path.push(PathCommand::CubicTo {
            control1: p1,
            control2: p2,
            end,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Coordinate, SeriesKind};
    fn series(v: f64) -> Series {
        Series::from_values(SeriesKind::Pie, [v])
    }
    fn sector(hit: &HitRegion) -> (Point, f64, f64, f64, f64) {
        let HitShape::AnnularSector {
            center,
            inner_radius,
            outer_radius,
            start_angle,
            sweep_angle,
        } = hit.shape
        else {
            panic!("pie sector");
        };
        (center, inner_radius, outer_radius, start_angle, sweep_angle)
    }
    #[test]
    fn source_should_scale_preserves_slice_angles_center_and_hover_reservation() {
        // CoreTests/SeriesTests/PieSeriesTest.cs::ShouldScale, same four values,
        // 1000x1000 size and HoverPushout=50; verifies engine geometry directly.
        let values = [100.0, 50.0, 25.0, 25.0];
        let series: Vec<_> = values
            .into_iter()
            .map(|value| {
                let mut s = series(value);
                s.hover_push_out = 50.0;
                s
            })
            .collect();
        let measured =
            PieChartEngine::measure(Size::new(1000.0, 1000.0), &series, &PieOptions::default())
                .unwrap();
        let mut angle = 0.0;
        for (value, hit) in values.into_iter().zip(&measured.hits) {
            let (center, _, radius, start, sweep) = sector(hit);
            assert_eq!(center, Point::new(500.0, 500.0));
            assert_eq!(radius * 2.0, 900.0);
            assert!((sweep / 360.0 - value / 200.0).abs() < 0.001);
            assert!((start - angle).abs() < 0.001);
            angle += sweep;
        }
    }
    #[test]
    fn minimum_relative_gauge_uses_the_effective_range_denominator() {
        // CorePieSeries.MeasurePieLayout has separate absolute and relative
        // branches. The #2131 overflow fixture uses the same effective-range cap.
        let options = PieOptions {
            min_value: 30.0,
            max_value: Some(100.0),
            ..PieOptions::default()
        };
        let absolute =
            PieChartEngine::measure(Size::new(300.0, 300.0), &[series(35.0)], &options).unwrap();
        let mut relative = series(35.0);
        relative.is_relative_to_min_value = true;
        let relative_scene =
            PieChartEngine::measure(Size::new(300.0, 300.0), &[relative.clone()], &options)
                .unwrap();
        assert!((sector(&absolute.hits[0]).4 - 360.0 * 5.0 / 70.0).abs() < 1e-10);
        assert_eq!(sector(&relative_scene.hits[0]).4, 180.0);
        relative.values[0] = Coordinate::new(0.0, 150.0);
        let saturated =
            PieChartEngine::measure(Size::new(300.0, 300.0), &[relative], &options).unwrap();
        assert_eq!(sector(&saturated.hits[0]).4, 360.0);
    }
    #[test]
    fn gauge_background_draws_first_without_hover_or_legend_entry() {
        let value = series(30.0);
        let mut background = series(0.0);
        background.is_fill_series = true;
        background.is_gauge = true;
        let options = PieOptions {
            max_angle: 270.0,
            max_value: Some(100.0),
            ..PieOptions::default()
        };
        let measured =
            PieChartEngine::measure(Size::new(300.0, 300.0), &[value, background], &options)
                .unwrap();
        assert_eq!(
            measured.primitives[0].id,
            Some(PointId {
                series: 1,
                point: 0
            })
        );
        assert_eq!(measured.hits.len(), 1);
        assert_eq!(measured.legend.len(), 1);
        assert_eq!(sector(&measured.hits[0]).4, 81.0);
    }
    #[test]
    fn radial_width_alignment_keeps_the_requested_edge() {
        let expected = [
            (RadialAlignment::Outer, 90.0, 120.0),
            (RadialAlignment::Center, 55.0, 85.0),
            (RadialAlignment::Inner, 20.0, 50.0),
        ];
        for (align, inner, outer) in expected {
            let mut s = series(1.0);
            s.is_gauge = true;
            s.inner_radius = 20.0;
            s.max_radial_column_width = 30.0;
            s.radial_align = align;
            let measured =
                PieChartEngine::measure(Size::new(240.0, 240.0), &[s], &PieOptions::default())
                    .unwrap();
            let actual = sector(&measured.hits[0]);
            assert_eq!((actual.1, actual.2), (inner, outer));
        }
    }
    #[test]
    fn tight_gauge_preserves_negative_offsets_and_does_not_reserve_hover_pushout() {
        // SnapshotTests/PieChartTests.cs::GaugeInTightBounds (#2131), numerical
        // layout adaptation; this does not claim PNG equivalence.
        let mut value = series(100.0);
        value.is_gauge = true;
        value.inner_radius = 38.0;
        value.relative_inner_radius = -4.0;
        value.max_radial_column_width = 5.0;
        let mut background = series(100.0);
        background.is_gauge = true;
        background.inner_radius = 40.0;
        background.relative_inner_radius = 4.0;
        background.outer_radius_offset = -20.0;
        background.max_radial_column_width = 5.0;
        let options = PieOptions {
            initial_rotation: 270.0,
            max_value: Some(100.0),
            ..PieOptions::default()
        };
        let first = PieChartEngine::measure(Size::new(240.0, 100.0), &[value], &options).unwrap();
        let second =
            PieChartEngine::measure(Size::new(240.0, 100.0), &[background], &options).unwrap();
        let a = sector(&first.hits[0]);
        let b = sector(&second.hits[0]);
        assert_eq!((a.1, a.2), (45.0, 50.0));
        assert_eq!((b.1, b.2), (55.0, 60.0));
    }
    #[test]
    fn relative_radii_keep_the_radial_slot_of_an_empty_value() {
        let mut s = Series::from_values(SeriesKind::Pie, [0.0, 10.0]);
        s.values[0] = Coordinate::EMPTY;
        s.is_gauge = true;
        s.inner_radius = 20.0;
        s.relative_inner_radius = 8.0;
        s.relative_outer_radius = 6.0;
        let measured =
            PieChartEngine::measure(Size::new(400.0, 400.0), &[s], &PieOptions::default()).unwrap();
        assert_eq!(measured.hits[0].id.point, 1);
        let hit = sector(&measured.hits[0]);
        assert_eq!((hit.1, hit.2), (118.0, 194.0));
    }
    #[test]
    fn source_label_positions_follow_slice_angles_and_native_text_metrics() {
        // PieSeriesTest.ShouldPlaceDataLabel uses the same five positions. This
        // fixture supplies known native metrics and checks the label anchor.
        for position in [
            PolarLabelsPosition::ChartCenter,
            PolarLabelsPosition::Middle,
            PolarLabelsPosition::Start,
            PolarLabelsPosition::End,
            PolarLabelsPosition::Outer,
        ] {
            let mut s = series(1.0);
            s.data_labels = true;
            s.polar_labels_position = position;
            s.hover_push_out = 0.0;
            let options = PieOptions {
                initial_rotation: 30.0,
                max_angle: 120.0,
                ..PieOptions::default()
            };
            let measured = PieChartEngine::measure_with_labels(
                Size::new(500.0, 500.0),
                &[s],
                &options,
                |_, _| Size::new(20.0, 12.0),
            )
            .unwrap();
            let text = measured
                .primitives
                .iter()
                .find_map(|p| {
                    if let Geometry::Text { origin, .. } = p.geometry {
                        Some(origin)
                    } else {
                        None
                    }
                })
                .unwrap();
            let center = Point::new(text.x, text.y + 6.0);
            let dx = center.x - 250.0;
            let dy = center.y - 250.0;
            if position == PolarLabelsPosition::ChartCenter {
                assert_eq!(center, Point::new(250.0, 250.0));
                continue;
            }
            let expected = match position {
                PolarLabelsPosition::Start => 30.0,
                PolarLabelsPosition::End => 150.0,
                _ => 90.0,
            };
            assert!((dy.atan2(dx).to_degrees() - expected).abs() < 1e-8);
            if position == PolarLabelsPosition::Outer {
                assert!(dx.hypot(dy) > sector(&measured.hits[0]).2);
            }
        }
    }
    #[test]
    fn rounded_and_inverted_corners_stay_finite_for_narrow_and_full_slices() {
        let center = Point::new(100.0, 100.0);
        let normal = rounded_annular_sector_path(center, 50.0, 90.0, 20.0, 80.0, 8.0, false);
        let inverted = rounded_annular_sector_path(center, 50.0, 90.0, 20.0, 80.0, 8.0, true);
        assert_ne!(normal, inverted);
        for sweep in [0.00001, 0.1, 80.0, 350.0, 360.0] {
            for inverted in [false, true] {
                let path =
                    rounded_annular_sector_path(center, 0.0, 90.0, 20.0, sweep, 80.0, inverted);
                assert_eq!(path.last(), Some(&PathCommand::Close));
                for command in path {
                    match command {
                        PathCommand::MoveTo(p) | PathCommand::LineTo(p) => {
                            assert!(p.x.is_finite() && p.y.is_finite())
                        }
                        PathCommand::CubicTo {
                            control1,
                            control2,
                            end,
                        } => assert!([control1, control2, end]
                            .iter()
                            .all(|p| p.x.is_finite() && p.y.is_finite())),
                        PathCommand::Close => {}
                    }
                }
            }
        }
    }
    #[test]
    fn pie_slices_sum_to_full_circle_and_ids_remain_stable() {
        let result = PieChartEngine::measure(
            Size::new(400.0, 400.0),
            &[series(2.0), series(3.0), series(5.0)],
            &PieOptions::default(),
        )
        .unwrap();
        assert_eq!(result.hits.len(), 3);
        let sweeps: Vec<_> = result
            .hits
            .iter()
            .map(|h| match h.shape {
                HitShape::AnnularSector { sweep_angle, .. } => sweep_angle,
                _ => 0.0,
            })
            .collect();
        assert_eq!(sweeps, vec![72.0, 108.0, 180.0]);
    }
    #[test]
    fn single_slice_has_closed_full_circle_geometry() {
        let result = PieChartEngine::measure(
            Size::new(400.0, 400.0),
            &[series(5.0)],
            &PieOptions::default(),
        )
        .unwrap();
        let Geometry::Path(p) = &result.primitives[0].geometry else {
            panic!()
        };
        assert_eq!(
            p.iter()
                .filter(|c| matches!(c, PathCommand::CubicTo { .. }))
                .count(),
            4
        );
        assert_eq!(p.last(), Some(&PathCommand::Close));
    }
    #[test]
    fn gauge_overflow_is_clamped_to_maximum_angle() {
        let result = PieChartEngine::measure(
            Size::new(400.0, 400.0),
            &[series(500.0)],
            &PieOptions {
                max_value: Some(100.0),
                max_angle: 180.0,
                ..PieOptions::default()
            },
        )
        .unwrap();
        let HitShape::AnnularSector { sweep_angle, .. } = result.hits[0].shape else {
            panic!()
        };
        assert_eq!(sweep_angle, 180.0);
    }
    #[test]
    fn doughnut_hole_has_no_hover() {
        let mut s = series(2.0);
        s.inner_radius = 60.0;
        let result =
            PieChartEngine::measure(Size::new(400.0, 400.0), &[s], &PieOptions::default()).unwrap();
        assert!(!result.hits[0].contains(Point::new(200.0, 200.0)));
    }
    #[test]
    fn zero_and_empty_values_do_not_divide_by_zero() {
        let mut s = series(0.0);
        s.values.push(Coordinate::EMPTY);
        let result =
            PieChartEngine::measure(Size::new(400.0, 400.0), &[s], &PieOptions::default()).unwrap();
        assert!(result.hits.is_empty());
    }
    #[test]
    fn nested_pie_values_become_independent_rings() {
        let a = Series::from_values(SeriesKind::Pie, [1.0, 3.0]);
        let b = Series::from_values(SeriesKind::Pie, [1.0, 1.0]);
        let result =
            PieChartEngine::measure(Size::new(400.0, 400.0), &[a, b], &PieOptions::default())
                .unwrap();
        assert_eq!(result.hits.len(), 4);
        for hit in &result.hits {
            let (_, inner, _, _, sweep) = sector(hit);
            if hit.id.point == 0 {
                assert_eq!(inner, 0.0);
                assert_eq!(sweep, 180.0);
            } else {
                assert!(inner > 0.0);
                assert_eq!(sweep, if hit.id.series == 0 { 270.0 } else { 90.0 });
            }
        }
    }
}
