//! Polar/radar measurement, independent angular/radial axes and source spline geometry.
//!
//! The angular origin follows LiveCharts2: bounds determine the angular interval
//! length, while zero always points at `initial_rotation`. The inverse uses atan2
//! and deliberately corrects the source's quadrant and nonzero-minimum defects.

use crate::axis::{format_number, Axis, AxisScale};
use crate::kernel::line_spline_math::control_points;
use crate::measure::AxisTick;
use crate::scene::*;
use crate::series::{PolarLabelsPosition, Series};
use ferroui_base::{Point, Rect, Size};
use std::collections::BTreeMap;

/// Source-compatible modifier packed into an axis/series label rotation.
pub const TANGENT_ANGLE: f64 = (1_u32 << 25) as f64;
/// Source-compatible modifier packed into an axis/series label rotation.
pub const COTANGENT_ANGLE: f64 = (1_u32 << 26) as f64;

#[derive(Clone, Debug, PartialEq)]
pub struct PolarOptions {
    pub draw_margin: Option<Rect>,
    pub inner_radius: f64,
    pub initial_rotation: f64,
    pub total_angle: f64,
    /// Enlarge and translate the plot using the source data-extents fit algorithm.
    pub fit_to_bounds: bool,
}
impl Default for PolarOptions {
    fn default() -> Self {
        Self {
            draw_margin: None,
            inner_radius: 0.0,
            initial_rotation: -90.0,
            total_angle: 360.0,
            fit_to_bounds: false,
        }
    }
}

/// One resolved pair of polar axes. All distances use FerroUI logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolarScale {
    pub center: Point,
    /// Maps angular values to degrees, including rotation and inversion.
    pub angle: AxisScale,
    /// Maps radial values to pixel distances from `center`.
    pub radius: AxisScale,
    pub initial_rotation: f64,
}
impl PolarScale {
    pub fn get_angle(self, angle: f64) -> f64 {
        self.angle.to_pixels(angle)
    }
    pub fn to_pixels(self, angle: f64, radius: f64) -> Point {
        self.to_pixels_with_angle_in_degrees(self.get_angle(angle), radius)
    }
    pub fn to_pixels_with_angle_in_degrees(self, angle: f64, radius: f64) -> Point {
        at_radius(self.center, angle, self.radius.to_pixels(radius))
    }
    /// Angular values are normalized to a revolution starting at InitialRotation.
    pub fn to_chart_values(self, point: Point) -> Point {
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        let relative = dy.atan2(dx).to_degrees() - self.initial_rotation;
        let relative = if self.angle.pixel_end >= self.angle.pixel_start {
            relative.rem_euclid(360.0)
        } else {
            -(-relative).rem_euclid(360.0)
        };
        Point::new(
            self.angle.to_chart_values(self.initial_rotation + relative),
            self.radius.to_chart_values(dx.hypot(dy)),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolarMeasurement {
    pub scene: Scene,
    pub angle_scales: Vec<AxisScale>,
    pub radius_scales: Vec<AxisScale>,
    pub center: Point,
    pub initial_rotation: f64,
}
impl PolarMeasurement {
    pub fn scaler(&self, angle_axis: usize, radius_axis: usize) -> Option<PolarScale> {
        Some(PolarScale {
            center: self.center,
            angle: *self.angle_scales.get(angle_axis)?,
            radius: *self.radius_scales.get(radius_axis)?,
            initial_rotation: self.initial_rotation,
        })
    }
}

pub struct PolarChartEngine;
impl PolarChartEngine {
    /// Convenience entry point for a single angle/radius axis pair.
    pub fn measure(
        size: Size,
        series: &[Series],
        angle_axis: &Axis,
        radius_axis: &Axis,
        options: &PolarOptions,
    ) -> Result<Scene, ChartError> {
        Self::measure_with_axes(
            size,
            series,
            std::slice::from_ref(angle_axis),
            std::slice::from_ref(radius_axis),
            options,
        )
        .map(|m| m.scene)
    }
    pub fn measure_with_axes(
        size: Size,
        series: &[Series],
        angle_axes: &[Axis],
        radius_axes: &[Axis],
        options: &PolarOptions,
    ) -> Result<PolarMeasurement, ChartError> {
        Self::measure_with_axes_and_labels(
            size,
            series,
            angle_axes,
            radius_axes,
            options,
            |text, size| Size::new(text.chars().count() as f64 * size * 0.6, size * 1.2),
        )
    }
    /// Native hosts supply FormattedText metrics here so axis margins and labels
    /// use the same typeface and font backend as the rendered text.
    pub fn measure_with_axes_and_labels(
        size: Size,
        series: &[Series],
        angle_axes: &[Axis],
        radius_axes: &[Axis],
        options: &PolarOptions,
        mut measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Result<PolarMeasurement, ChartError> {
        validate_size(size)?;
        validate_options(options)?;
        let defaults = [Axis::default()];
        let angle_axes = if angle_axes.is_empty() {
            &defaults[..]
        } else {
            angle_axes
        };
        let radius_axes = if radius_axes.is_empty() {
            &defaults[..]
        } else {
            radius_axes
        };
        for a in angle_axes.iter().chain(radius_axes.iter()) {
            validate_axis(a)?;
        }
        let mut angle_bounds = vec![DataBounds::default(); angle_axes.len()];
        let mut radius_bounds = vec![DataBounds::default(); radius_axes.len()];
        let mut radial_padding = vec![0.0_f64; radius_axes.len()];
        let mut angular_padding = vec![false; angle_axes.len()];
        let mut totals: BTreeMap<StackKey, (f64, f64)> = BTreeMap::new();
        for s in series.iter().filter(|s| s.visible) {
            validate_series(s)?;
            if s.polar_stacked && s.stack_100_percent {
                let ai = axis_index(s.x_axis, angle_axes.len());
                let ri = axis_index(s.y_axis, radius_axes.len());
                for c in s.values.iter().filter(|c| c.is_finite()) {
                    let e = totals
                        .entry(stack_key(s, ai, ri, c.secondary_value))
                        .or_default();
                    if c.primary_value >= 0.0 {
                        e.0 += c.primary_value;
                    } else {
                        e.1 += -c.primary_value;
                    }
                }
            }
        }
        let mut stacks: BTreeMap<StackKey, (f64, f64)> = BTreeMap::new();
        let mut radii: Vec<Vec<Option<f64>>> =
            series.iter().map(|s| vec![None; s.values.len()]).collect();
        for (si, s) in series.iter().enumerate().filter(|(_, s)| s.visible) {
            let ai = axis_index(s.x_axis, angle_axes.len());
            let ri = axis_index(s.y_axis, radius_axes.len());
            let mut own = DataBounds::default();
            for (pi, c) in s.values.iter().enumerate().filter(|(_, c)| c.is_finite()) {
                let mut value = c.primary_value;
                if s.polar_stacked {
                    let key = stack_key(s, ai, ri, c.secondary_value);
                    if s.stack_100_percent {
                        let total = totals.get(&key).copied().unwrap_or_default();
                        let denominator = if value >= 0.0 { total.0 } else { total.1 };
                        value = if denominator > 0.0 {
                            value / denominator * 100.0
                        } else {
                            0.0
                        };
                    }
                    let stack = stacks.entry(key).or_default();
                    let track = if value >= 0.0 {
                        &mut stack.0
                    } else {
                        &mut stack.1
                    };
                    own.append(*track);
                    *track += value;
                    value = *track;
                }
                if !value.is_finite()
                    || angle_axes[ai].logarithmic_base.is_some() && c.secondary_value <= 0.0
                    || radius_axes[ri].logarithmic_base.is_some() && value <= 0.0
                {
                    continue;
                }
                radii[si][pi] = Some(value);
                angle_bounds[ai].append(c.secondary_value);
                own.append(value);
                angular_padding[ai] = true;
            }
            radius_bounds[ri].append_bounds(own);
            if own.has_values() && radius_axes[ri].logarithmic_base.is_none() {
                let a = &radius_axes[ri];
                let padding = if own.min == own.max {
                    0.1 * if own.min == 0.0 { 1.0 } else { own.min.abs() } * s.data_padding.y
                } else {
                    AxisTick::polar(
                        size,
                        a.min_limit.unwrap_or(own.min),
                        a.max_limit.unwrap_or(own.max),
                        options.inner_radius,
                        options.total_angle,
                        false,
                        a.min_separators,
                        a.force_step,
                    )
                    .value
                        * s.data_padding.y
                };
                radial_padding[ri] = radial_padding[ri].max(padding.max(0.0));
            }
        }
        for (i, a) in angle_axes.iter().enumerate() {
            if !a.labels.is_empty() && !angle_bounds[i].has_values() {
                angle_bounds[i] = DataBounds {
                    min: 0.0,
                    max: a.labels.len().saturating_sub(1) as f64,
                };
                angular_padding[i] = true;
            }
            angle_bounds[i].ensure(a);
        }
        for (i, a) in radius_axes.iter().enumerate() {
            radius_bounds[i].ensure(a);
        }
        let full_plot = Rect::new(0.0, 0.0, size.width, size.height);
        let raw_angles = build_angles(angle_axes, &angle_bounds, options)?;
        let raw_radii = build_radii(radius_axes, &radius_bounds, full_plot, options)?;
        let mut plot = if options.fit_to_bounds {
            fit_plot(size, series, &radii, &raw_angles, &raw_radii, options)
        } else if let Some(plot) = options.draw_margin {
            plot
        } else {
            let mut margin = 0.0;
            for (i, axis) in angle_axes.iter().enumerate().filter(|(_, a)| a.visible) {
                let mut radius = 0.0_f64;
                for value in polar_ticks(raw_angles[i], axis, size, options, true) {
                    let m = text_size(&mut measure_text, &axis.label(value), axis.text_size);
                    radius = radius.max(
                        (m.width * 0.5 + axis.labels_padding)
                            .hypot(m.height * 0.5 + axis.labels_padding),
                    );
                }
                margin += radius;
            }
            margin = margin.min(size.width.min(size.height) * 0.5);
            Rect::new(
                margin,
                margin,
                (size.width - margin * 2.0).max(0.0),
                (size.height - margin * 2.0).max(0.0),
            )
        };
        // A collapsed explicit rectangle is a valid hidden plot, not an invalid frame.
        plot.width = plot.width.max(0.0);
        plot.height = plot.height.max(0.0);
        for (i, b) in angle_bounds.iter_mut().enumerate() {
            if angular_padding[i]
                && angle_axes[i].max_limit.is_none()
                && angle_axes[i].logarithmic_base.is_none()
            {
                b.max += 1.0;
            }
        }
        for (i, b) in radius_bounds.iter_mut().enumerate() {
            if radius_axes[i].min_limit.is_none() {
                b.min -= radial_padding[i];
            }
            if radius_axes[i].max_limit.is_none() {
                b.max += radial_padding[i];
            }
        }
        let angle_scales = build_angles(angle_axes, &angle_bounds, options)?;
        let radius_scales = build_radii(radius_axes, &radius_bounds, plot, options)?;
        let center = Point::new(plot.x + plot.width * 0.5, plot.y + plot.height * 0.5);
        let mut measured = PolarMeasurement {
            scene: Scene::new(size, plot),
            angle_scales,
            radius_scales,
            center,
            initial_rotation: options.initial_rotation,
        };
        measured.scene.automatic_finding_strategy = FindingStrategy::CompareAll;
        if plot.width <= 0.0 || plot.height <= 0.0 {
            return Ok(measured);
        }
        draw_axes(
            &mut measured,
            angle_axes,
            radius_axes,
            options,
            &mut measure_text,
        );
        for (si, s) in series.iter().enumerate().filter(|(_, s)| s.visible) {
            measured.scene.legend.push(LegendItem {
                series: si,
                name: if s.name.is_empty() {
                    format!("Series {}", si + 1)
                } else {
                    s.name.clone()
                },
                color: s.color(si),
            });
        }
        // A later stacked radial fill sits behind earlier, smaller cumulative fills.
        let mut order: Vec<_> = series
            .iter()
            .enumerate()
            .filter(|(_, s)| s.visible)
            .collect();
        order.sort_by_key(|(i, s)| {
            if s.polar_stacked {
                (0, usize::MAX - i)
            } else {
                (1, *i)
            }
        });
        for (si, s) in order {
            let ai = axis_index(s.x_axis, angle_axes.len());
            let ri = axis_index(s.y_axis, radius_axes.len());
            let scaler = measured.scaler(ai, ri).unwrap();
            let mut runs: Vec<Vec<Point>> = vec![Vec::new()];
            let mut markers = Vec::new();
            for (index, c) in s.values.iter().enumerate() {
                let Some(radius) = radii[si][index] else {
                    if s.enable_null_splitting && !runs.last().unwrap().is_empty() {
                        runs.push(Vec::new());
                    }
                    continue;
                };
                let p = scaler.to_pixels(c.secondary_value, radius);
                if !p.x.is_finite() || !p.y.is_finite() {
                    continue;
                }
                runs.last_mut().unwrap().push(p);
                let id = PointId {
                    series: si,
                    point: index,
                };
                if s.geometry_size > 0.0 {
                    markers.push(
                        Primitive::new(
                            Geometry::Ellipse(Rect::new(
                                p.x - s.geometry_size * 0.5,
                                p.y - s.geometry_size * 0.5,
                                s.geometry_size,
                                s.geometry_size,
                            )),
                            s.style(si, true),
                        )
                        .identified(id),
                    );
                }
                let hover_size = s.geometry_size.max(16.0);
                let angular_label = if angle_axes[ai].labels.is_empty() {
                    String::new()
                } else {
                    angle_axes[ai].label(c.secondary_value)
                };
                let radial_label = radius_axes[ri].label(c.primary_value);
                let label = [
                    s.name.as_str(),
                    angular_label.as_str(),
                    radial_label.as_str(),
                ]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(": ");
                measured.scene.hits.push(HitRegion {
                    id,
                    shape: HitShape::WithHoverArea {
                        shape: Box::new(HitShape::Circle {
                            center: p,
                            radius: s.geometry_size.max(0.0) * 0.5,
                        }),
                        bounds: Rect::new(
                            p.x - hover_size * 0.5,
                            p.y - hover_size * 0.5,
                            hover_size,
                            hover_size,
                        ),
                    },
                    center: p,
                    value: c.primary_value,
                    label,
                });
                if s.data_labels {
                    let text = format_number(c.primary_value);
                    let m = text_size(&mut measure_text, &text, s.data_labels_size);
                    let angle = scaler.get_angle(c.secondary_value);
                    let label_position = polar_label_position(
                        center,
                        p,
                        angle,
                        m,
                        s.geometry_size,
                        s.polar_labels_position,
                    );
                    markers.push(
                        Primitive::new(
                            Geometry::Text {
                                origin: Point::new(
                                    label_position.x,
                                    label_position.y - m.height * 0.5,
                                ),
                                text,
                                size: s.data_labels_size,
                                rotation: polar_label_rotation(
                                    s.data_labels_rotation,
                                    angle,
                                    false,
                                ),
                                anchor: TextAnchor::Center,
                            },
                            PaintStyle::fill(s.data_labels_color.unwrap_or_else(|| s.color(si))),
                        )
                        .identified(id),
                    );
                }
            }
            for run in runs.into_iter().filter(|r| !r.is_empty()) {
                let closed = s.polar_closed && run.len() > 2;
                let path = polar_line_path(&run, s.line_smoothness.clamp(0.0, 1.0), closed);
                measured.scene.primitives.push(
                    Primitive::new(
                        Geometry::Path(path),
                        s.style(si, s.fill.is_some() || s.fill_brush.is_some()),
                    )
                    .identified(PointId {
                        series: si,
                        point: usize::MAX,
                    }),
                );
            }
            measured.scene.primitives.extend(markers);
        }
        Ok(measured)
    }
}

#[derive(Clone, Copy)]
struct DataBounds {
    min: f64,
    max: f64,
}
impl Default for DataBounds {
    fn default() -> Self {
        Self {
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }
}
impl DataBounds {
    fn append(&mut self, v: f64) {
        if v.is_finite() {
            self.min = self.min.min(v);
            self.max = self.max.max(v);
        }
    }
    fn append_bounds(&mut self, b: Self) {
        self.append(b.min);
        self.append(b.max);
    }
    fn has_values(self) -> bool {
        self.min.is_finite() && self.max.is_finite()
    }
    fn ensure(&mut self, axis: &Axis) {
        if !self.has_values() {
            self.min = 0.0;
            self.max = 10.0 * axis.unit_width;
        }
        if self.min == self.max {
            self.min -= axis.unit_width * 0.5;
            self.max += axis.unit_width * 0.5;
        }
    }
}
type StackKey = (usize, usize, u32, u64);
fn stack_key(s: &Series, ai: usize, ri: usize, angle: f64) -> StackKey {
    (
        ai,
        ri,
        s.stack_group,
        if angle == 0.0 { 0 } else { angle.to_bits() },
    )
}
fn axis_index(index: usize, count: usize) -> usize {
    if index < count {
        index
    } else {
        0
    }
}
fn validate_options(o: &PolarOptions) -> Result<(), ChartError> {
    if [o.inner_radius, o.initial_rotation, o.total_angle]
        .iter()
        .any(|v| !v.is_finite())
        || o.inner_radius < 0.0
        || o.total_angle <= 0.0
        || o.total_angle > 360.0
    {
        return Err(ChartError::InvalidData(
            "polar dimensions and angles must be finite with total angle in (0,360]".into(),
        ));
    }
    if let Some(p) = o.draw_margin {
        if [p.x, p.y, p.width, p.height].iter().any(|v| !v.is_finite())
            || p.width < 0.0
            || p.height < 0.0
        {
            return Err(ChartError::InvalidData("invalid polar draw margin".into()));
        }
    }
    Ok(())
}
fn validate_axis(a: &Axis) -> Result<(), ChartError> {
    if [
        a.unit_width,
        a.min_step,
        a.text_size,
        a.labels_rotation,
        a.labels_angle,
        a.labels_padding,
    ]
    .iter()
    .any(|v| !v.is_finite())
        || a.unit_width <= 0.0
        || a.min_step < 0.0
        || a.text_size < 0.0
        || a.labels_padding < 0.0
        || a.custom_separators.iter().any(|v| !v.is_finite())
    {
        return Err(ChartError::InvalidAxis(
            "polar units, label dimensions and separators must be finite and valid".into(),
        ));
    }
    Ok(())
}
fn validate_series(s: &Series) -> Result<(), ChartError> {
    if [
        s.geometry_size,
        s.line_smoothness,
        s.stroke_width,
        s.opacity,
        s.data_labels_size,
        s.data_labels_rotation,
        s.data_padding.x,
        s.data_padding.y,
    ]
    .iter()
    .any(|v| !v.is_finite())
        || s.geometry_size < 0.0
        || s.data_labels_size < 0.0
        || s.data_padding.x < 0.0
        || s.data_padding.y < 0.0
    {
        return Err(ChartError::InvalidData(
            "polar series dimensions and paint values must be finite and valid".into(),
        ));
    }
    Ok(())
}
fn build_angles(
    axes: &[Axis],
    bounds: &[DataBounds],
    o: &PolarOptions,
) -> Result<Vec<AxisScale>, ChartError> {
    axes.iter()
        .zip(bounds)
        .map(|(axis, b)| {
            let mut a = axis.clone();
            a.padding = 0.0;
            let mut scale = AxisScale::new(&a, b.min, b.max, 0.0, 1.0)?;
            let (min, max) = if let Some(base) = scale.logarithmic_base {
                (scale.min.log(base), scale.max.log(base))
            } else {
                (scale.min, scale.max)
            };
            let magnitude = min.abs().max(max.abs()).max(1.0);
            let span = max / magnitude - min / magnitude;
            let direction = if axis.inverted { -1.0 } else { 1.0 };
            scale.pixel_start =
                o.initial_rotation + direction * o.total_angle * (min / magnitude) / span;
            scale.pixel_end =
                o.initial_rotation + direction * o.total_angle * (max / magnitude) / span;
            if !scale.pixel_start.is_finite() || !scale.pixel_end.is_finite() {
                return Err(ChartError::InvalidAxis(
                    "polar angle span cannot be represented".into(),
                ));
            }
            Ok(scale)
        })
        .collect()
}
fn build_radii(
    axes: &[Axis],
    bounds: &[DataBounds],
    plot: Rect,
    o: &PolarOptions,
) -> Result<Vec<AxisScale>, ChartError> {
    let outer = plot.width.min(plot.height) * 0.5;
    let inner = o.inner_radius.min(outer);
    axes.iter()
        .zip(bounds)
        .map(|(axis, b)| {
            let mut a = axis.clone();
            a.padding = 0.0;
            AxisScale::new(&a, b.min, b.max, inner, outer)
        })
        .collect()
}
fn polar_ticks(
    scale: AxisScale,
    axis: &Axis,
    size: Size,
    o: &PolarOptions,
    is_angle: bool,
) -> Vec<f64> {
    if !axis.custom_separators.is_empty() || axis.logarithmic_base.is_some() {
        return scale.ticks(axis, size.width.min(size.height));
    }
    let step = AxisTick::polar(
        size,
        scale.min,
        scale.max,
        o.inner_radius,
        o.total_angle,
        is_angle,
        axis.min_separators,
        axis.force_step,
    )
    .value;
    let minimum = axis
        .min_step
        .max(if axis.labels.is_empty() { 0.0 } else { 1.0 });
    let step = if axis.force_step && minimum > 0.0 {
        minimum
    } else {
        step.max(minimum)
    };
    if !step.is_finite() || step <= 0.0 {
        return vec![];
    }
    let first = (scale.min / step).ceil() * step;
    (0..1024)
        .map(|i| first + i as f64 * step)
        .take_while(|v| v.is_finite() && *v <= scale.max + step * 1e-9)
        .collect()
}
fn fit_plot(
    size: Size,
    series: &[Series],
    radii: &[Vec<Option<f64>>],
    angles: &[AxisScale],
    radial: &[AxisScale],
    o: &PolarOptions,
) -> Rect {
    let center = Point::new(size.width * 0.5, size.height * 0.5);
    let (mut left, mut right, mut top, mut bottom) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    let mut has_point = false;
    for (si, s) in series.iter().enumerate().filter(|(_, s)| s.visible) {
        let scaler = PolarScale {
            center,
            angle: angles[axis_index(s.x_axis, angles.len())],
            radius: radial[axis_index(s.y_axis, radial.len())],
            initial_rotation: o.initial_rotation,
        };
        for (i, c) in s.values.iter().enumerate() {
            let Some(r) = radii[si][i] else {
                continue;
            };
            let p = scaler.to_pixels(c.secondary_value, r);
            if !p.x.is_finite() || !p.y.is_finite() {
                continue;
            }
            has_point = true;
            left = left.max(center.x - p.x);
            right = right.max(p.x - center.x);
            top = top.max(center.y - p.y);
            bottom = bottom.max(p.y - center.y);
        }
    }
    if !has_point {
        return Rect::new(0.0, 0.0, size.width, size.height);
    }
    let (dl, dr, dt, db) = (
        center.x - left,
        center.x - right,
        center.y - top,
        center.y - bottom,
    );
    Rect::new(
        -dl,
        -dt,
        (size.width + dl + dr).max(0.0),
        (size.height + dt + db).max(0.0),
    )
}
fn draw_axes(
    m: &mut PolarMeasurement,
    angle_axes: &[Axis],
    radius_axes: &[Axis],
    o: &PolarOptions,
    measure_text: &mut impl FnMut(&str, f64) -> Size,
) {
    let outer = m.scene.plot_bounds.width.min(m.scene.plot_bounds.height) * 0.5;
    let inner = o.inner_radius.min(outer);
    for (i, axis) in radius_axes.iter().enumerate().filter(|(_, a)| a.visible) {
        let scaler = m.scaler(0, i).unwrap();
        for value in polar_ticks(scaler.radius, axis, m.scene.size, o, false) {
            let radius = scaler.radius.to_pixels(value);
            if !radius.is_finite() || radius < 0.0 {
                continue;
            }
            if axis.show_separator_lines {
                m.scene.primitives.push(Primitive::new(
                    Geometry::Ellipse(Rect::new(
                        m.center.x - radius,
                        m.center.y - radius,
                        radius * 2.0,
                        radius * 2.0,
                    )),
                    PaintStyle::stroke(axis.separators_color, 1.0),
                ));
            }
            let text = axis.label(value);
            if text.trim().is_empty() {
                continue;
            }
            let size = text_size(measure_text, &text, axis.text_size);
            let position = at_radius(m.center, axis.labels_angle, radius);
            m.scene.primitives.push(Primitive::new(
                Geometry::Text {
                    origin: Point::new(position.x, position.y - size.height * 0.5),
                    text,
                    size: axis.text_size,
                    rotation: polar_label_rotation(
                        axis.labels_rotation,
                        scaler.get_angle(value),
                        false,
                    ),
                    anchor: TextAnchor::Center,
                },
                PaintStyle::fill(axis.labels_color),
            ));
        }
    }
    for (i, axis) in angle_axes.iter().enumerate().filter(|(_, a)| a.visible) {
        let scaler = m.scaler(i, 0).unwrap();
        for value in polar_ticks(scaler.angle, axis, m.scene.size, o, true) {
            let angle = scaler.get_angle(value);
            let start = at_radius(m.center, angle, inner);
            let end = at_radius(m.center, angle, outer);
            if axis.show_separator_lines {
                m.scene.primitives.push(Primitive::new(
                    Geometry::Line { start, end },
                    PaintStyle::stroke(axis.separators_color, 1.0),
                ));
            }
            let text = axis.label(value);
            if text.trim().is_empty() {
                continue;
            }
            let size = text_size(measure_text, &text, axis.text_size);
            m.scene.primitives.push(Primitive::new(
                Geometry::Text {
                    origin: Point::new(end.x, end.y - size.height * 0.5),
                    text,
                    size: axis.text_size,
                    rotation: polar_label_rotation(axis.labels_rotation, angle, true),
                    anchor: TextAnchor::Center,
                },
                PaintStyle::fill(axis.labels_color),
            ));
        }
    }
}
fn text_size(measure: &mut impl FnMut(&str, f64) -> Size, text: &str, size: f64) -> Size {
    let measured = measure(text, size);
    if measured.width.is_finite()
        && measured.height.is_finite()
        && measured.width >= 0.0
        && measured.height >= 0.0
    {
        measured
    } else {
        Size::new(0.0, 0.0)
    }
}
fn at_radius(center: Point, degrees: f64, radius: f64) -> Point {
    let radians = degrees.to_radians();
    Point::new(
        center.x + radians.cos() * radius,
        center.y + radians.sin() * radius,
    )
}
fn polar_label_position(
    center: Point,
    point: Point,
    angle: f64,
    label: Size,
    geometry: f64,
    position: PolarLabelsPosition,
) -> Point {
    let mut radius = (point.x - center.x).hypot(point.y - center.y);
    let offset = (label.width + geometry * 0.5).hypot(label.height + geometry * 0.5) * 0.5;
    match position {
        PolarLabelsPosition::ChartCenter => return center,
        PolarLabelsPosition::End => radius += offset,
        PolarLabelsPosition::Start => radius -= offset,
        PolarLabelsPosition::Outer => radius *= 2.0,
        PolarLabelsPosition::Middle => {}
    }
    at_radius(center, angle, radius)
}
/// Decode the source bit-packed rotation while keeping ordinary negative degrees valid.
pub(crate) fn polar_label_rotation(rotation: f64, angle: f64, always_upright: bool) -> f64 {
    let bits = if rotation >= TANGENT_ANGLE {
        rotation as u64
    } else {
        0
    };
    let tangent = bits & (1 << 25) != 0;
    let cotangent = bits & (1 << 26) != 0;
    let mut value = rotation;
    if tangent {
        value -= TANGENT_ANGLE;
        value += angle - 90.0;
    }
    if cotangent {
        value -= COTANGENT_ANGLE;
        value += angle;
    }
    if always_upright || tangent || cotangent {
        value = value.rem_euclid(360.0);
        if (value + 90.0).rem_euclid(360.0) > 180.0 {
            value += 180.0;
        }
    }
    value
}
fn polar_line_path(points: &[Point], smoothness: f64, closed: bool) -> Vec<PathCommand> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut path = vec![PathCommand::MoveTo(points[0])];
    let n = points.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    for i in 0..count {
        let previous = if i == 0 {
            if closed {
                points[n - 1]
            } else {
                points[0]
            }
        } else {
            points[i - 1]
        };
        let next = if i + 1 < n { points[i + 1] } else { points[0] };
        let after = if i + 2 < n {
            points[i + 2]
        } else if closed {
            points[(i + 2) % n]
        } else {
            points[n - 1]
        };
        if smoothness == 0.0 {
            path.push(PathCommand::LineTo(next));
            continue;
        }
        let segment = control_points(previous, points[i], next, after, smoothness);
        path.push(PathCommand::CubicTo {
            control1: if i == 0 {
                points[i]
            } else {
                segment.control1()
            },
            control2: segment.control2(),
            end: segment.end(),
        });
    }
    if closed {
        path.push(PathCommand::Close);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{kernel::Coordinate, SeriesKind};
    #[test]
    fn named_radar_categories_have_distinct_positions() {
        let axis = Axis {
            labels: vec!["A".into(), "B".into(), "C".into(), "D".into()],
            ..Axis::default()
        };
        let s = Series::from_values(SeriesKind::PolarLine, [5.0; 4]);
        let scene = PolarChartEngine::measure(
            Size::new(400.0, 400.0),
            &[s],
            &axis,
            &Axis::default(),
            &PolarOptions::default(),
        )
        .unwrap();
        assert_eq!(scene.hits.len(), 4);
        assert_ne!(scene.hits[0].center, scene.hits[3].center);
        assert!((scene.hits[0].center.x - 200.0).abs() < 1e-8);
    }
    #[test]
    fn polar_nulls_do_not_close_across_missing_values() {
        let mut s = Series::from_values(SeriesKind::PolarLine, [1.0, 2.0, 3.0, 4.0]);
        s.values[1] = Coordinate::EMPTY;
        let scene = PolarChartEngine::measure(
            Size::new(400.0, 400.0),
            &[s],
            &Axis::default(),
            &Axis::default(),
            &PolarOptions::default(),
        )
        .unwrap();
        assert_eq!(scene.hits.len(), 3);
        assert!(scene
            .primitives
            .iter()
            .all(|p| !matches!(&p.geometry,Geometry::Path(v) if v.contains(&PathCommand::Close))));
    }
    #[test]
    fn constant_polar_values_have_finite_positions() {
        let s = Series::from_values(SeriesKind::PolarLine, [0.0; 4]);
        let scene = PolarChartEngine::measure(
            Size::new(400.0, 400.0),
            &[s],
            &Axis::default(),
            &Axis::default(),
            &PolarOptions::default(),
        )
        .unwrap();
        assert!(scene
            .hits
            .iter()
            .all(|h| h.center.x.is_finite() && h.center.y.is_finite()));
    }
    fn hidden_axis(min: f64, max: f64) -> Axis {
        Axis {
            min_limit: Some(min),
            max_limit: Some(max),
            visible: false,
            ..Axis::default()
        }
    }
    fn full_options() -> PolarOptions {
        PolarOptions {
            initial_rotation: 0.0,
            draw_margin: Some(Rect::new(0.0, 0.0, 300.0, 300.0)),
            ..PolarOptions::default()
        }
    }
    fn assert_point(actual: Point, expected: Point) {
        assert!(
            (actual.x - expected.x).abs() < 1e-8,
            "x {} != {}",
            actual.x,
            expected.x
        );
        assert!(
            (actual.y - expected.y).abs() < 1e-8,
            "y {} != {}",
            actual.y,
            expected.y
        );
    }
    /// Direct port: CoreTests/SeriesTests/PolarLineSeriesTest.ShouldScaleProperly.
    #[test]
    fn source_should_scale_properly() {
        let mut s = Series::new(SeriesKind::PolarLine);
        s.values = (1..=10)
            .map(|i| Coordinate::new(36.0 * i as f64, i as f64))
            .collect();
        s.geometry_size = 0.0;
        s.polar_closed = false;
        let m = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            &[s],
            &[hidden_axis(0.0, 360.0)],
            &[hidden_axis(0.0, 10.0)],
            &PolarOptions::default(),
        )
        .unwrap();
        assert_eq!(m.scene.plot_bounds, Rect::new(0.0, 0.0, 300.0, 300.0));
        let distance = |p: Point| (p.x - 150.0).hypot(p.y - 150.0);
        let unit = distance(m.scene.hits[0].center);
        assert!((unit - 15.0).abs() < 1e-9);
        for (i, hit) in m.scene.hits.iter().enumerate() {
            assert!((unit * (i + 1) as f64 - distance(hit.center)).abs() < 0.0001);
        }
    }
    /// Direct port: CoreTests/ChartTests/PolarDrawMarginTests.ExplicitDrawMargin_IsHonored.
    #[test]
    fn source_explicit_draw_margin_is_honored() {
        let s = Series::from_values(SeriesKind::PolarLine, [1.0, 2.0, 3.0]);
        let options = PolarOptions {
            draw_margin: Some(Rect::new(40.0, 50.0, 200.0, 180.0)),
            ..PolarOptions::default()
        };
        let m =
            PolarChartEngine::measure_with_axes(Size::new(300.0, 300.0), &[s], &[], &[], &options)
                .unwrap();
        assert_eq!(m.scene.plot_bounds, Rect::new(40.0, 50.0, 200.0, 180.0));
        assert_eq!(m.center, Point::new(140.0, 140.0));
    }
    #[test]
    fn multiple_axis_pairs_scale_and_inverse_independently() {
        let mut a = Series::new(SeriesKind::PolarLine);
        a.values = vec![Coordinate::new(90.0, 5.0)];
        let mut b = Series::new(SeriesKind::PolarLine);
        b.values = vec![Coordinate::new(45.0, 50.0)];
        b.x_axis = 1;
        b.y_axis = 1;
        let m = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            &[a, b],
            &[hidden_axis(0.0, 360.0), hidden_axis(0.0, 180.0)],
            &[hidden_axis(0.0, 10.0), hidden_axis(0.0, 100.0)],
            &full_options(),
        )
        .unwrap();
        for hit in &m.scene.hits {
            assert_point(hit.center, Point::new(150.0, 225.0));
        }
        assert_point(
            m.scaler(0, 0)
                .unwrap()
                .to_chart_values(Point::new(150.0, 225.0)),
            Point::new(90.0, 5.0),
        );
        assert_point(
            m.scaler(1, 1)
                .unwrap()
                .to_chart_values(Point::new(150.0, 225.0)),
            Point::new(45.0, 50.0),
        );
        assert!(m.scaler(2, 0).is_none());
    }
    #[test]
    fn angular_origin_matches_source_for_nonzero_limits_and_inversion() {
        let mut a = hidden_axis(10.0, 370.0);
        let mut r = hidden_axis(0.0, 10.0);
        for inverted in [false, true] {
            a.inverted = inverted;
            r.inverted = inverted;
            let m = PolarChartEngine::measure_with_axes(
                Size::new(300.0, 300.0),
                &[],
                &[a.clone()],
                &[r.clone()],
                &full_options(),
            )
            .unwrap();
            let scaler = m.scaler(0, 0).unwrap();
            let point = scaler.to_pixels(90.0, 2.0);
            assert_point(
                point,
                if inverted {
                    Point::new(150.0, 30.0)
                } else {
                    Point::new(150.0, 180.0)
                },
            );
            assert_point(scaler.to_chart_values(point), Point::new(90.0, 2.0));
        }
    }
    #[test]
    fn automatic_bounds_are_per_axis_and_reserve_a_final_angular_slot() {
        let mut a = Series::from_values(SeriesKind::PolarLine, [1.0, 2.0, 3.0, 4.0]);
        a.data_padding = Point::new(0.0, 0.0);
        let mut b = Series::from_values(SeriesKind::PolarLine, [1000.0, 2000.0, 3000.0, 4000.0]);
        b.data_padding = Point::new(0.0, 0.0);
        b.x_axis = 1;
        b.y_axis = 1;
        let axes = [Axis::default(), Axis::default()];
        let m = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            &[a, b],
            &axes,
            &axes,
            &full_options(),
        )
        .unwrap();
        assert_eq!((m.angle_scales[0].min, m.angle_scales[0].max), (0.0, 4.0));
        assert_eq!((m.radius_scales[0].min, m.radius_scales[0].max), (1.0, 4.0));
        assert_eq!(
            (m.radius_scales[1].min, m.radius_scales[1].max),
            (1000.0, 4000.0)
        );
        assert_eq!(m.scaler(0, 0).unwrap().get_angle(3.0), 270.0);
    }
    #[test]
    fn source_data_padding_and_hot_reload_axis_fallback() {
        let mut s = Series::from_values(SeriesKind::PolarLine, [1.0, 2.0, 3.0, 4.0]);
        s.x_axis = 50;
        s.y_axis = 100;
        let m = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            &[s],
            &[],
            &[],
            &full_options(),
        )
        .unwrap();
        assert_eq!(
            (m.radius_scales[0].min, m.radius_scales[0].max),
            (-0.5, 5.5)
        );
        assert_eq!(m.scene.hits.len(), 4);
    }
    #[test]
    fn fit_to_bounds_uses_source_negative_margin_data_extents() {
        let mut s = Series::new(SeriesKind::PolarLine);
        s.values = vec![Coordinate::new(0.0, 10.0), Coordinate::new(90.0, 10.0)];
        let options = PolarOptions {
            fit_to_bounds: true,
            ..full_options()
        };
        let m = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            &[s],
            &[hidden_axis(0.0, 360.0)],
            &[hidden_axis(0.0, 10.0)],
            &options,
        )
        .unwrap();
        assert_eq!(m.scene.plot_bounds, Rect::new(-150.0, -150.0, 450.0, 450.0));
        assert_point(m.scene.hits[0].center, Point::new(300.0, 75.0));
        assert_point(m.scene.hits[1].center, Point::new(75.0, 300.0));
    }
    #[test]
    fn radial_labels_use_absolute_labels_angle_and_custom_separators() {
        let mut r = hidden_axis(0.0, 10.0);
        r.visible = true;
        r.labels_angle = -60.0;
        r.custom_separators = vec![5.0];
        let m = PolarChartEngine::measure_with_axes_and_labels(
            Size::new(300.0, 300.0),
            &[],
            &[hidden_axis(0.0, 360.0)],
            &[r],
            &full_options(),
            |_, _| Size::new(20.0, 10.0),
        )
        .unwrap();
        let label = m
            .scene
            .primitives
            .iter()
            .find_map(|p| match &p.geometry {
                Geometry::Text { origin, text, .. } if text == "5" => Some(*origin),
                _ => None,
            })
            .unwrap();
        assert_point(
            label,
            Point::new(187.5, 150.0 - 75.0 * (3.0_f64).sqrt() * 0.5 - 5.0),
        );
        assert_eq!(
            m.scene
                .primitives
                .iter()
                .filter(|p| matches!(p.geometry, Geometry::Ellipse(_)))
                .count(),
            1
        );
    }
    #[test]
    fn native_text_metrics_control_angular_auto_margin() {
        let a = Axis {
            labels: vec!["A".into(), "B".into()],
            min_limit: Some(0.0),
            max_limit: Some(2.0),
            labels_padding: 3.0,
            ..Axis::default()
        };
        let m = PolarChartEngine::measure_with_axes_and_labels(
            Size::new(300.0, 300.0),
            &[],
            &[a],
            &[hidden_axis(0.0, 10.0)],
            &PolarOptions::default(),
            |text, _| {
                if text.is_empty() {
                    Size::default()
                } else {
                    Size::new(100.0, 20.0)
                }
            },
        )
        .unwrap();
        let margin = 53.0_f64.hypot(13.0);
        assert!((m.scene.plot_bounds.x - margin).abs() < 1e-9);
        assert!((m.scene.plot_bounds.width - (300.0 - margin * 2.0)).abs() < 1e-9);
    }
    #[test]
    fn source_hover_minimum_keeps_exact_marker_geometry() {
        let mut s = Series::new(SeriesKind::PolarLine);
        s.geometry_size = 2.0;
        s.values = vec![Coordinate::new(0.0, 5.0)];
        let m = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            &[s],
            &[hidden_axis(0.0, 360.0)],
            &[hidden_axis(0.0, 10.0)],
            &full_options(),
        )
        .unwrap();
        assert_eq!(
            m.scene.automatic_finding_strategy,
            FindingStrategy::CompareAll
        );
        assert_eq!(
            m.scene
                .hit_test(Point::new(232.0, 157.0), FindingStrategy::Automatic, 0.0)
                .len(),
            1
        );
        assert!(m
            .scene
            .hit_test(Point::new(232.0, 157.0), FindingStrategy::ExactMatch, 0.0)
            .is_empty());
    }
    #[test]
    fn polar_labels_follow_each_source_position_and_rotation_flag() {
        for (position, factor) in [
            (PolarLabelsPosition::ChartCenter, 0.0),
            (PolarLabelsPosition::Middle, 75.0),
            (PolarLabelsPosition::Outer, 150.0),
            (
                PolarLabelsPosition::Start,
                75.0 - 25.0_f64.hypot(15.0) * 0.5,
            ),
            (PolarLabelsPosition::End, 75.0 + 25.0_f64.hypot(15.0) * 0.5),
        ] {
            let mut s = Series::new(SeriesKind::PolarLine);
            s.values = vec![Coordinate::new(0.0, 5.0)];
            s.geometry_size = 10.0;
            s.data_labels = true;
            s.polar_labels_position = position;
            let m = PolarChartEngine::measure_with_axes_and_labels(
                Size::new(300.0, 300.0),
                &[s],
                &[hidden_axis(0.0, 360.0)],
                &[hidden_axis(0.0, 10.0)],
                &full_options(),
                |_, _| Size::new(20.0, 10.0),
            )
            .unwrap();
            let origin = m
                .scene
                .primitives
                .iter()
                .find_map(|p| match p.geometry {
                    Geometry::Text { origin, .. } => Some(origin),
                    _ => None,
                })
                .unwrap();
            assert_point(origin, Point::new(150.0 + factor, 145.0));
        }
        assert_eq!(polar_label_rotation(TANGENT_ANGLE, 90.0, false), 0.0);
        assert_eq!(
            polar_label_rotation(COTANGENT_ANGLE, 180.0, false).rem_euclid(360.0),
            0.0
        );
        assert_eq!(polar_label_rotation(-45.0, 180.0, false), -45.0);
    }
    #[test]
    fn stacked_radii_share_groups_without_moving_tooltip_values() {
        let mut a = Series::new(SeriesKind::PolarLine);
        a.polar_stacked = true;
        a.values = vec![Coordinate::new(0.0, 2.0)];
        let mut b = a.clone();
        b.values[0].primary_value = 3.0;
        let mut c = b.clone();
        c.stack_group = 1;
        let m = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            &[a, b, c],
            &[hidden_axis(0.0, 360.0)],
            &[hidden_axis(0.0, 10.0)],
            &full_options(),
        )
        .unwrap();
        for (si, x, value) in [(0, 180.0, 2.0), (1, 225.0, 3.0), (2, 195.0, 3.0)] {
            let hit = m.scene.hits.iter().find(|h| h.id.series == si).unwrap();
            assert_point(hit.center, Point::new(x, 150.0));
            assert_eq!(hit.value, value);
        }
    }
    #[test]
    fn filled_paths_draw_before_markers_and_close_each_valid_run() {
        let mut s = Series::from_values(SeriesKind::PolarLine, [2.0; 7]);
        s.values[3] = Coordinate::EMPTY;
        s.fill = Some(ferroui_base::media::Color::from_rgb(10, 20, 30));
        let m = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            &[s],
            &[hidden_axis(0.0, 7.0)],
            &[hidden_axis(0.0, 10.0)],
            &full_options(),
        )
        .unwrap();
        let paths: Vec<_> = m
            .scene
            .primitives
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(&p.geometry, Geometry::Path(_)))
            .collect();
        assert_eq!(paths.len(), 2);
        for (_, primitive) in &paths {
            let Geometry::Path(commands) = &primitive.geometry else {
                unreachable!()
            };
            assert_eq!(
                commands
                    .iter()
                    .filter(|c| matches!(c, PathCommand::CubicTo { .. }))
                    .count(),
                3
            );
            assert_eq!(commands.last(), Some(&PathCommand::Close));
        }
        let marker = m
            .scene
            .primitives
            .iter()
            .position(|p| p.id.is_some_and(|id| id.point != usize::MAX))
            .unwrap();
        assert!(paths.iter().all(|(index, _)| *index < marker));
    }
    #[test]
    fn collapsed_plot_hides_geometry_and_remeasures_after_resize() {
        let s = Series::from_values(SeriesKind::PolarLine, [1.0, 2.0, 3.0]);
        let options = PolarOptions {
            draw_margin: Some(Rect::new(0.0, 0.0, 0.0, 100.0)),
            ..PolarOptions::default()
        };
        let empty = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            std::slice::from_ref(&s),
            &[],
            &[],
            &options,
        )
        .unwrap();
        assert!(empty.scene.primitives.is_empty() && empty.scene.hits.is_empty());
        let restored = PolarChartEngine::measure_with_axes(
            Size::new(300.0, 300.0),
            &[s],
            &[],
            &[],
            &full_options(),
        )
        .unwrap();
        assert_eq!(restored.scene.hits.len(), 3);
    }
}
