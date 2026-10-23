//! Cartesian measurement adapted from the pinned chart and series implementations.
//! Pure calculation produces a retained FerroUI drawing scene; native controls own scheduling.

use crate::axis::{format_number, Axis, AxisPosition, AxisScale};
use crate::kernel::Coordinate;
use crate::measure::{AxisOrientation, Bounds};
use crate::scene::*;
use crate::series::{Series, SeriesKind};
use ferroui_base::{media::Color, Point, Rect, Size};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub x_axis: usize,
    pub y_axis: usize,
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f64,
}
impl Default for Section {
    fn default() -> Self {
        Self {
            x_axis: 0,
            y_axis: 0,
            x_min: None,
            x_max: None,
            y_min: None,
            y_max: None,
            fill: Some(Color::from_argb(35, 66, 99, 235)),
            stroke: None,
            stroke_width: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CartesianOptions {
    /// Explicit control-local plot rectangle. None computes margins for visible axes.
    pub draw_margin: Option<Rect>,
    pub sections: Vec<Section>,
}
impl Default for CartesianOptions {
    fn default() -> Self {
        Self {
            draw_margin: None,
            sections: Vec::new(),
        }
    }
}

/// Calculated axes can also drive crosshairs and zoom/pan without reinterpreting pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct CartesianMeasurement {
    pub scene: Scene,
    pub x_scales: Vec<AxisScale>,
    pub y_scales: Vec<AxisScale>,
    /// Per-series data extrema and source padding/geometry requests, in model order.
    pub series_bounds: Vec<Option<CartesianSeriesBounds>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CartesianSeriesBounds {
    pub x: Bounds,
    pub y: Bounds,
}

pub struct CartesianChartEngine;
impl CartesianChartEngine {
    pub fn measure(
        size: Size,
        series: &[Series],
        x_axes: &[Axis],
        y_axes: &[Axis],
        options: &CartesianOptions,
    ) -> Result<Scene, ChartError> {
        Self::measure_with_axes(size, series, x_axes, y_axes, options).map(|m| m.scene)
    }
    pub fn measure_with_axes(
        size: Size,
        series: &[Series],
        x_axes: &[Axis],
        y_axes: &[Axis],
        options: &CartesianOptions,
    ) -> Result<CartesianMeasurement, ChartError> {
        Self::measure_with_axes_and_labels(size, series, x_axes, y_axes, options, |text, size| {
            Size::new(text.chars().count() as f64 * size * 0.6, size * 1.2)
        })
    }
    pub fn measure_with_axes_and_labels(
        size: Size,
        series: &[Series],
        x_axes: &[Axis],
        y_axes: &[Axis],
        options: &CartesianOptions,
        mut measure_text: impl FnMut(&str, f64) -> Size,
    ) -> Result<CartesianMeasurement, ChartError> {
        validate_size(size)?;
        let defaults = [Axis::default()];
        let x_axes = if x_axes.is_empty() {
            &defaults[..]
        } else {
            x_axes
        };
        let y_axes = if y_axes.is_empty() {
            &defaults[..]
        } else {
            y_axes
        };
        let plot = plot_bounds(size, x_axes, y_axes, options.draw_margin)?;
        let mut scene = Scene::new(size, plot);
        // Kernel.Extensions.GetFindingStrategy chooses one policy for all visible
        // series: category X, category Y, or two-dimensional comparison.
        let mut all_x = true;
        let mut all_y = true;
        for s in series.iter().filter(|s| s.visible) {
            all_x &= !s.kind.is_row() && !matches!(s.kind, SeriesKind::Scatter | SeriesKind::Heat);
            all_y &= s.kind.is_row();
        }
        scene.automatic_finding_strategy = if all_x {
            FindingStrategy::CompareOnlyXTakeClosest
        } else if all_y {
            FindingStrategy::CompareOnlyYTakeClosest
        } else {
            FindingStrategy::CompareAllTakeClosest
        };
        let mut x_bounds = vec![(f64::INFINITY, f64::NEG_INFINITY); x_axes.len()];
        let mut y_bounds = vec![(f64::INFINITY, f64::NEG_INFINITY); y_axes.len()];
        let mut totals: BTreeMap<StackKey, (f64, f64)> = BTreeMap::new();
        for (si, s) in series.iter().enumerate().filter(|(_, s)| s.visible) {
            validate_series(s, x_axes.len(), y_axes.len())?;
            scene.legend.push(LegendItem {
                series: si,
                name: if s.name.is_empty() {
                    format!("Series {}", si + 1)
                } else {
                    s.name.clone()
                },
                color: s.color(si),
            });
            if s.kind.is_stacked() && s.stack_100_percent {
                for c in s.values.iter().filter(|c| valid_point(s, c)) {
                    let t = totals.entry(stack_key(s, c)).or_default();
                    if c.primary_value >= 0.0 {
                        t.0 += c.primary_value;
                    } else {
                        t.1 -= c.primary_value;
                    }
                }
            }
        }
        let mut stacks: BTreeMap<StackKey, (f64, f64)> = BTreeMap::new();
        let mut measured: Vec<Vec<Option<MeasuredPoint>>> = Vec::with_capacity(series.len());
        let mut series_bounds = Vec::with_capacity(series.len());
        for s in series {
            let mut points = Vec::with_capacity(s.values.len());
            if !s.visible {
                measured.push(points);
                series_bounds.push(None);
                continue;
            }
            let mut own_x = (f64::INFINITY, f64::NEG_INFINITY);
            let mut own_y = (f64::INFINITY, f64::NEG_INFINITY);
            let category_half = if s.kind.is_bar() {
                if s.kind.is_row() {
                    y_axes[s.y_axis].unit_width
                } else {
                    x_axes[s.x_axis].unit_width
                }
                .abs()
                    * 0.5
            } else {
                0.0
            };
            for (index, c) in s.values.iter().copied().enumerate() {
                if !valid_point(s, &c) {
                    points.push(None);
                    continue;
                }
                let mut top = c.primary_value;
                let mut bottom = match s.kind {
                    SeriesKind::RangeColumn | SeriesKind::RangeRow | SeriesKind::RangeLine => {
                        c.tertiary_value
                    }
                    SeriesKind::Financial | SeriesKind::Box => c.quinary_value,
                    _ => s.pivot,
                };
                if s.kind.is_stacked() {
                    let key = stack_key(s, &c);
                    if s.stack_100_percent {
                        let total = totals.get(&key).copied().unwrap_or_default();
                        let denominator = if top >= 0.0 { total.0 } else { total.1 };
                        top = if denominator == 0.0 {
                            0.0
                        } else {
                            top / denominator * 100.0
                        };
                    }
                    let stack = stacks.entry(key).or_insert((s.pivot, s.pivot));
                    let offset = if top >= 0.0 {
                        &mut stack.0
                    } else {
                        &mut stack.1
                    };
                    bottom = *offset;
                    *offset += top;
                    top = *offset;
                }
                let (mut xmin, mut xmax, mut ymin, mut ymax) = if s.kind.is_row() {
                    (
                        top,
                        top,
                        c.secondary_value - category_half,
                        c.secondary_value + category_half,
                    )
                } else {
                    (
                        c.secondary_value - category_half,
                        c.secondary_value + category_half,
                        top,
                        top,
                    )
                };
                if s.kind.is_bar() || s.kind.is_area() || s.fill.is_some() || s.fill_brush.is_some()
                {
                    if s.kind.is_row() {
                        xmin = xmin.min(bottom);
                        xmax = xmax.max(bottom);
                    } else {
                        ymin = ymin.min(bottom);
                        ymax = ymax.max(bottom);
                    }
                }
                if !c.point_error.is_empty {
                    let (cx, cy) = if s.kind.is_row() {
                        (top, c.secondary_value)
                    } else {
                        (c.secondary_value, top)
                    };
                    xmin = xmin.min(cx - c.point_error.xi.abs());
                    xmax = xmax.max(cx + c.point_error.xj.abs());
                    ymin = ymin.min(cy - c.point_error.yj.abs());
                    ymax = ymax.max(cy + c.point_error.yi.abs());
                }
                for value in [
                    xmin,
                    xmax,
                    if s.kind.is_row() {
                        top
                    } else {
                        c.secondary_value
                    },
                ] {
                    include_axis(&mut x_bounds[s.x_axis], value, &x_axes[s.x_axis]);
                    include_axis(&mut own_x, value, &x_axes[s.x_axis]);
                }
                for value in [
                    ymin,
                    ymax,
                    if s.kind.is_row() {
                        c.secondary_value
                    } else {
                        top
                    },
                ] {
                    include_axis(&mut y_bounds[s.y_axis], value, &y_axes[s.y_axis]);
                    include_axis(&mut own_y, value, &y_axes[s.y_axis]);
                }
                points.push(Some(MeasuredPoint {
                    index,
                    coordinate: c,
                    bottom,
                    top,
                }));
            }
            series_bounds.push(if own_x.0.is_finite() && own_y.0.is_finite() {
                Some(CartesianSeriesBounds {
                    x: data_bounds(own_x),
                    y: data_bounds(own_y),
                })
            } else {
                None
            });
            measured.push(points);
        }
        // Heat cells represent a data interval, not a pixel sized glyph. Infer irregular
        // date/number spacing rather than assuming the default axis unit of one.
        for (si, s) in series
            .iter()
            .enumerate()
            .filter(|(_, s)| s.visible && s.kind == SeriesKind::Heat)
        {
            let dx = minimum_delta(
                s.values
                    .iter()
                    .filter(|c| valid_point(s, c))
                    .map(|c| c.secondary_value),
            )
            .unwrap_or(x_axes[s.x_axis].unit_width.abs());
            let dy = minimum_delta(
                s.values
                    .iter()
                    .filter(|c| valid_point(s, c))
                    .map(|c| c.primary_value),
            )
            .unwrap_or(y_axes[s.y_axis].unit_width.abs());
            // Expand this series' cells, not the aggregate bounds. Otherwise two
            // heat series on the same axes add the padding twice, and a distant
            // line series is padded by unrelated heat-cell dimensions.
            for c in s.values.iter().filter(|c| valid_point(s, c)) {
                for value in [c.secondary_value - dx * 0.5, c.secondary_value + dx * 0.5] {
                    include_axis(&mut x_bounds[s.x_axis], value, &x_axes[s.x_axis]);
                    if value.is_finite()
                        && (x_axes[s.x_axis].logarithmic_base.is_none() || value > 0.0)
                    {
                        if let Some(bounds) = &mut series_bounds[si] {
                            bounds.x.append_value(value);
                        }
                    }
                }
                for value in [c.primary_value - dy * 0.5, c.primary_value + dy * 0.5] {
                    include_axis(&mut y_bounds[s.y_axis], value, &y_axes[s.y_axis]);
                    if value.is_finite()
                        && (y_axes[s.y_axis].logarithmic_base.is_none() || value > 0.0)
                    {
                        if let Some(bounds) = &mut series_bounds[si] {
                            bounds.y.append_value(value);
                        }
                    }
                }
            }
        }
        let x_label_sizes = x_axes
            .iter()
            .zip(&x_bounds)
            .map(|(axis, bounds)| maximum_label_size(axis, *bounds, &mut measure_text))
            .collect::<Vec<_>>();
        let y_label_sizes = y_axes
            .iter()
            .zip(&y_bounds)
            .map(|(axis, bounds)| maximum_label_size(axis, *bounds, &mut measure_text))
            .collect::<Vec<_>>();
        let mut x_requests = vec![Bounds::default(); x_axes.len()];
        let mut y_requests = vec![Bounds::default(); y_axes.len()];
        for (si, s) in series.iter().enumerate().filter(|(_, s)| s.visible) {
            let Some(bounds) = &mut series_bounds[si] else {
                continue;
            };
            let geometry_size = if matches!(
                s.kind,
                SeriesKind::Line
                    | SeriesKind::StepLine
                    | SeriesKind::Area
                    | SeriesKind::StackedArea
                    | SeriesKind::StackedStepArea
                    | SeriesKind::RangeLine
            ) {
                (s.geometry_size.max(0.0)
                    + if s.geometry_size > 0.0 {
                        s.stroke_width.max(0.0)
                    } else {
                        0.0
                    })
                    * 0.5
            } else {
                0.0
            };
            set_padding_request(
                &mut bounds.x,
                &x_axes[s.x_axis],
                size,
                AxisOrientation::X,
                x_label_sizes[s.x_axis],
                s.data_padding.x,
                geometry_size,
            )?;
            set_padding_request(
                &mut bounds.y,
                &y_axes[s.y_axis],
                size,
                AxisOrientation::Y,
                y_label_sizes[s.y_axis],
                s.data_padding.y,
                geometry_size,
            )?;
            bounds.x.min_delta = minimum_delta(measured[si].iter().flatten().map(|p| {
                if s.kind.is_row() {
                    p.top
                } else {
                    p.coordinate.secondary_value
                }
            }))
            .unwrap_or(f64::MAX);
            bounds.y.min_delta = minimum_delta(measured[si].iter().flatten().map(|p| {
                if s.kind.is_row() {
                    p.coordinate.secondary_value
                } else {
                    p.top
                }
            }))
            .unwrap_or(f64::MAX);
            x_requests[s.x_axis].append_bounds(&bounds.x);
            y_requests[s.y_axis].append_bounds(&bounds.y);
        }
        let all_columns = series
            .iter()
            .filter(|s| s.visible)
            .all(|s| matches!(s.kind, SeriesKind::Column | SeriesKind::StackedColumn));
        let x_scales = x_axes
            .iter()
            .zip(x_bounds.iter())
            .zip(&x_requests)
            .map(|((a, &(min, max)), request)| {
                padded_scale(a, min, max, plot.x, plot.x + plot.width, request, false)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let y_scales = y_axes
            .iter()
            .zip(y_bounds.iter())
            .zip(&y_requests)
            .map(|((a, &(min, max)), request)| {
                padded_scale(
                    a,
                    min,
                    max,
                    plot.y + plot.height,
                    plot.y,
                    request,
                    all_columns,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        draw_sections(&mut scene, &options.sections, &x_scales, &y_scales)?;
        draw_axes(
            &mut scene,
            x_axes,
            y_axes,
            &x_scales,
            &y_scales,
            &x_label_sizes,
            &y_label_sizes,
        );
        for (si, s) in series.iter().enumerate().filter(|(_, s)| s.visible) {
            let xs = x_scales[s.x_axis];
            let ys = y_scales[s.y_axis];
            let points = &measured[si];
            match s.kind {
                SeriesKind::Line
                | SeriesKind::StepLine
                | SeriesKind::Area
                | SeriesKind::StackedArea
                | SeriesKind::StackedStepArea
                | SeriesKind::RangeLine => draw_line_series(
                    &mut scene,
                    si,
                    s,
                    points,
                    xs,
                    ys,
                    x_axes[s.x_axis].unit_width,
                ),
                SeriesKind::Column
                | SeriesKind::Row
                | SeriesKind::StackedColumn
                | SeriesKind::StackedRow
                | SeriesKind::RangeColumn
                | SeriesKind::RangeRow => {
                    draw_bars(&mut scene, si, s, series, points, xs, ys, x_axes, y_axes)
                }
                SeriesKind::Scatter => draw_scatter(&mut scene, si, s, points, xs, ys),
                SeriesKind::Heat => draw_heat(&mut scene, si, s, points, xs, ys, x_axes, y_axes),
                SeriesKind::Financial | SeriesKind::Box => {
                    draw_financial(&mut scene, si, s, points, xs, ys, x_axes)
                }
                SeriesKind::Pie | SeriesKind::PolarLine => {
                    return Err(ChartError::InvalidData(
                        "a pie or polar series requires its matching chart engine".into(),
                    ));
                }
            }
        }
        Ok(CartesianMeasurement {
            scene,
            x_scales,
            y_scales,
            series_bounds,
        })
    }
}

fn data_bounds((min, max): (f64, f64)) -> Bounds {
    Bounds {
        min,
        max,
        is_empty: false,
        ..Bounds::default()
    }
}
fn maximum_label_size(
    axis: &Axis,
    bounds: (f64, f64),
    measure: &mut impl FnMut(&str, f64) -> Size,
) -> Size {
    let min = axis
        .min_limit
        .unwrap_or(if bounds.0.is_finite() { bounds.0 } else { 0.0 });
    let max = axis
        .max_limit
        .unwrap_or(if bounds.1.is_finite() { bounds.1 } else { 10.0 });
    let mut maximum = Size::new(0.0, 0.0);
    // Source CoreAxis.GetPossibleMaxLabelSize samples 25 intervals before ticks.
    for i in 0..=25 {
        let fraction = i as f64 / 25.0;
        let value = min * (1.0 - fraction) + max * fraction;
        let measured = measure(&axis.label(value), axis.text_size);
        if measured.width.is_finite() && measured.height.is_finite() {
            maximum.width = maximum
                .width
                .max(measured.width + axis.labels_padding * 2.0);
            maximum.height = maximum
                .height
                .max(measured.height + axis.labels_padding * 2.0);
        }
    }
    maximum
}
fn set_padding_request(
    bounds: &mut Bounds,
    axis: &Axis,
    size: Size,
    orientation: AxisOrientation,
    labels: Size,
    padding: f64,
    geometry: f64,
) -> Result<(), ChartError> {
    let mut axis = axis.clone();
    axis.padding = 0.0;
    let scale = AxisScale::new(&axis, bounds.min, bounds.max, 0.0, 1.0)?;
    let request = scale
        .tick_with_metrics(&axis, size, orientation, labels)
        .value
        * padding;
    bounds.padding_min = if request.is_finite() {
        request.max(0.0)
    } else {
        0.0
    };
    bounds.padding_max = bounds.padding_min;
    bounds.requested_geometry_size = geometry;
    Ok(())
}
fn padded_scale(
    axis: &Axis,
    raw_min: f64,
    raw_max: f64,
    start: f64,
    end: f64,
    request: &Bounds,
    columns: bool,
) -> Result<AxisScale, ChartError> {
    let mut plain = axis.clone();
    plain.padding = 0.0;
    let initial = AxisScale::new(&plain, raw_min, raw_max, start, end)?;
    if axis.logarithmic_base.is_some() {
        return Ok(initial);
    }
    let (mut min, mut max) = (initial.min, initial.max);
    let percent = ((max - min) * axis.padding.max(0.0)).max(0.0);
    let percent = if percent.is_finite() { percent } else { 0.0 };
    let geometry_padding = |scale: AxisScale| {
        if start == end {
            return 0.0;
        }
        let value = (scale.to_chart_values(start + request.requested_geometry_size)
            - scale.to_chart_values(start))
        .abs();
        if value.is_finite() {
            value
        } else {
            0.0
        }
    };
    if axis.min_limit.is_none() {
        let amount = geometry_padding(initial)
            .max(request.padding_min)
            .max(percent);
        if (min - amount).is_finite() {
            min -= amount;
        }
        if columns && raw_min >= 0.0 {
            min = min.max(0.0);
        }
    }
    if axis.max_limit.is_none() {
        // The source re-creates its scaler after applying the lower padding.
        let after_min = AxisScale::new(&plain, min, max, start, end)?;
        let amount = geometry_padding(after_min)
            .max(request.padding_max)
            .max(percent);
        if (max + amount).is_finite() {
            max += amount;
        }
        if columns && raw_max <= 0.0 {
            max = max.min(0.0);
        }
    }
    AxisScale::new(&plain, min, max, start, end)
}

#[derive(Clone, Copy)]
struct MeasuredPoint {
    index: usize,
    coordinate: Coordinate,
    bottom: f64,
    top: f64,
}
type StackKey = (usize, usize, u8, u32, u64);
fn stack_key(s: &Series, c: &Coordinate) -> StackKey {
    (
        s.x_axis,
        s.y_axis,
        if s.kind.is_row() {
            1
        } else if s.kind.is_bar() {
            2
        } else {
            3
        },
        s.stack_group,
        if c.secondary_value == 0.0 {
            0
        } else {
            c.secondary_value.to_bits()
        },
    )
}
fn include_axis(bounds: &mut (f64, f64), value: f64, axis: &Axis) {
    if value.is_finite() && (axis.logarithmic_base.is_none() || value > 0.0) {
        bounds.0 = bounds.0.min(value);
        bounds.1 = bounds.1.max(value);
    }
}
fn baseline(scale: AxisScale, value: f64) -> f64 {
    scale.to_pixels(if scale.logarithmic_base.is_some() && value <= 0.0 {
        scale.min
    } else {
        value
    })
}
fn valid_point(s: &Series, c: &Coordinate) -> bool {
    if c.is_empty || !c.primary_value.is_finite() || !c.secondary_value.is_finite() {
        return false;
    }
    match s.kind {
        SeriesKind::RangeColumn
        | SeriesKind::RangeRow
        | SeriesKind::RangeLine
        | SeriesKind::Scatter
        | SeriesKind::Heat => c.tertiary_value.is_finite(),
        SeriesKind::Financial => {
            c.tertiary_value.is_finite()
                && c.quaternary_value.is_finite()
                && c.quinary_value.is_finite()
        }
        SeriesKind::Box => {
            c.tertiary_value.is_finite()
                && c.quaternary_value.is_finite()
                && c.quinary_value.is_finite()
                && c.senary_value.is_finite()
        }
        _ => true,
    }
}
fn validate_series(s: &Series, x: usize, y: usize) -> Result<(), ChartError> {
    if s.x_axis >= x || s.y_axis >= y {
        return Err(ChartError::InvalidAxis(
            "series refers to a missing axis".into(),
        ));
    }
    if [
        s.pivot,
        s.geometry_size,
        s.min_geometry_size,
        s.line_smoothness,
        s.stroke_width,
        s.opacity,
        s.padding,
        s.corner_radius,
        s.max_bar_width,
        s.data_labels_size,
        s.data_padding.x,
        s.data_padding.y,
    ]
    .iter()
    .any(|v| !v.is_finite())
        || s.dashes.iter().any(|d| !d.is_finite() || *d < 0.0)
        || s.data_padding.x < 0.0
        || s.data_padding.y < 0.0
    {
        return Err(ChartError::InvalidData(
            "series dimensions and paint values must be finite".into(),
        ));
    }
    Ok(())
}
fn plot_bounds(
    size: Size,
    x: &[Axis],
    y: &[Axis],
    explicit: Option<Rect>,
) -> Result<Rect, ChartError> {
    if let Some(r) = explicit {
        if [r.x, r.y, r.width, r.height].iter().any(|v| !v.is_finite())
            || r.width < 0.0
            || r.height < 0.0
        {
            return Err(ChartError::InvalidData(
                "draw margin must be finite with nonnegative size".into(),
            ));
        }
        return Ok(r);
    }
    let left = y
        .iter()
        .filter(|a| a.visible && a.position == AxisPosition::Start)
        .count() as f64
        * 58.0
        + 8.0;
    let right = y
        .iter()
        .filter(|a| a.visible && a.position == AxisPosition::End)
        .count() as f64
        * 58.0
        + 12.0;
    let top = x
        .iter()
        .filter(|a| a.visible && a.position == AxisPosition::End)
        .count() as f64
        * 40.0
        + 12.0;
    let bottom = x
        .iter()
        .filter(|a| a.visible && a.position == AxisPosition::Start)
        .count() as f64
        * 40.0
        + 8.0;
    let sx = (size.width / (left + right).max(1.0)).min(1.0);
    let sy = (size.height / (top + bottom).max(1.0)).min(1.0);
    Ok(Rect::new(
        left * sx,
        top * sy,
        (size.width - (left + right) * sx).max(0.0),
        (size.height - (top + bottom) * sy).max(0.0),
    ))
}
fn draw_axes(
    scene: &mut Scene,
    x: &[Axis],
    y: &[Axis],
    xs: &[AxisScale],
    ys: &[AxisScale],
    x_label_sizes: &[Size],
    y_label_sizes: &[Size],
) {
    let r = scene.plot_bounds;
    let mut x_offsets = [0.0, 0.0];
    let mut y_offsets = [0.0, 0.0];
    for ((axis, scale), labels) in x
        .iter()
        .zip(xs)
        .zip(x_label_sizes)
        .filter(|((a, _), _)| a.visible)
    {
        let side = usize::from(axis.position == AxisPosition::End);
        let baseline = if side == 0 {
            r.y + r.height + x_offsets[side]
        } else {
            r.y - x_offsets[side]
        };
        for value in scale.ticks_with_metrics(axis, scene.size, AxisOrientation::X, *labels) {
            let px = scale.to_pixels(value);
            if !px.is_finite() {
                continue;
            }
            if axis.show_separator_lines {
                scene.primitives.push(
                    Primitive::new(
                        Geometry::Line {
                            start: Point::new(px, r.y),
                            end: Point::new(px, r.y + r.height),
                        },
                        PaintStyle::stroke(axis.separators_color, 1.0),
                    )
                    .clipped(r),
                );
            }
            scene.primitives.push(Primitive::new(
                Geometry::Text {
                    origin: Point::new(
                        px,
                        if side == 0 {
                            baseline + 6.0
                        } else {
                            baseline - axis.text_size - 6.0
                        },
                    ),
                    text: axis.label(value),
                    size: axis.text_size,
                    rotation: axis.labels_rotation,
                    anchor: TextAnchor::Center,
                },
                PaintStyle::fill(axis.labels_color),
            ));
        }
        if !axis.name.is_empty() {
            scene.primitives.push(Primitive::new(
                Geometry::Text {
                    origin: Point::new(
                        r.x + r.width * 0.5,
                        if side == 0 {
                            baseline + 23.0
                        } else {
                            baseline - 30.0
                        },
                    ),
                    text: axis.name.clone(),
                    size: axis.text_size,
                    rotation: 0.0,
                    anchor: TextAnchor::Center,
                },
                PaintStyle::fill(axis.labels_color),
            ));
        }
        x_offsets[side] += 40.0;
    }
    for ((axis, scale), labels) in y
        .iter()
        .zip(ys)
        .zip(y_label_sizes)
        .filter(|((a, _), _)| a.visible)
    {
        let side = usize::from(axis.position == AxisPosition::End);
        let baseline = if side == 0 {
            r.x - y_offsets[side]
        } else {
            r.x + r.width + y_offsets[side]
        };
        for value in scale.ticks_with_metrics(axis, scene.size, AxisOrientation::Y, *labels) {
            let py = scale.to_pixels(value);
            if !py.is_finite() {
                continue;
            }
            if axis.show_separator_lines {
                scene.primitives.push(
                    Primitive::new(
                        Geometry::Line {
                            start: Point::new(r.x, py),
                            end: Point::new(r.x + r.width, py),
                        },
                        PaintStyle::stroke(axis.separators_color, 1.0),
                    )
                    .clipped(r),
                );
            }
            scene.primitives.push(Primitive::new(
                Geometry::Text {
                    origin: Point::new(
                        if side == 0 {
                            baseline - 8.0
                        } else {
                            baseline + 8.0
                        },
                        py - axis.text_size * 0.5,
                    ),
                    text: axis.label(value),
                    size: axis.text_size,
                    rotation: axis.labels_rotation,
                    anchor: if side == 0 {
                        TextAnchor::End
                    } else {
                        TextAnchor::Start
                    },
                },
                PaintStyle::fill(axis.labels_color),
            ));
        }
        if !axis.name.is_empty() {
            scene.primitives.push(Primitive::new(
                Geometry::Text {
                    origin: Point::new(
                        if side == 0 {
                            baseline - 55.0
                        } else {
                            baseline + 48.0
                        },
                        r.y + r.height * 0.5,
                    ),
                    text: axis.name.clone(),
                    size: axis.text_size,
                    rotation: -90.0,
                    anchor: TextAnchor::Center,
                },
                PaintStyle::fill(axis.labels_color),
            ));
        }
        y_offsets[side] += 58.0;
    }
}
fn draw_sections(
    scene: &mut Scene,
    sections: &[Section],
    xs: &[AxisScale],
    ys: &[AxisScale],
) -> Result<(), ChartError> {
    let plot = scene.plot_bounds;
    for section in sections {
        let Some(x) = xs.get(section.x_axis) else {
            return Err(ChartError::InvalidAxis(
                "section refers to a missing X axis".into(),
            ));
        };
        let Some(y) = ys.get(section.y_axis) else {
            return Err(ChartError::InvalidAxis(
                "section refers to a missing Y axis".into(),
            ));
        };
        let left = section.x_min.map(|v| x.to_pixels(v)).unwrap_or(plot.x);
        let right = section
            .x_max
            .map(|v| x.to_pixels(v))
            .unwrap_or(plot.x + plot.width);
        let top = section.y_max.map(|v| y.to_pixels(v)).unwrap_or(plot.y);
        let bottom = section
            .y_min
            .map(|v| y.to_pixels(v))
            .unwrap_or(plot.y + plot.height);
        if [left, right, top, bottom, section.stroke_width]
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err(ChartError::InvalidData(
                "section endpoints must be finite and valid for their axes".into(),
            ));
        }
        let style = PaintStyle {
            fill: section.fill,
            stroke: section.stroke,
            stroke_width: section.stroke_width,
            ..PaintStyle::default()
        };
        let geometry = if left == right || top == bottom {
            Geometry::Line {
                start: Point::new(left, top),
                end: Point::new(right, bottom),
            }
        } else {
            Geometry::Rectangle {
                rect: Rect::new(
                    left.min(right),
                    top.min(bottom),
                    (right - left).abs(),
                    (bottom - top).abs(),
                ),
                radius: 0.0,
            }
        };
        scene
            .primitives
            .push(Primitive::new(geometry, style).clipped(plot));
    }
    Ok(())
}
fn finite_point(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}
fn location(p: &MeasuredPoint, x: AxisScale, y: AxisScale) -> Point {
    Point::new(
        x.to_pixels(p.coordinate.secondary_value),
        y.to_pixels(p.top),
    )
}
fn append_hit(
    scene: &mut Scene,
    si: usize,
    s: &Series,
    p: &MeasuredPoint,
    shape: HitShape,
    center: Point,
) {
    let value = if s.kind == SeriesKind::Heat {
        p.coordinate.tertiary_value
    } else {
        p.coordinate.primary_value
    };
    let text = match s.kind {
        SeriesKind::RangeColumn | SeriesKind::RangeRow | SeriesKind::RangeLine => format!(
            "{} – {}",
            format_number(p.coordinate.tertiary_value),
            format_number(p.coordinate.primary_value)
        ),
        SeriesKind::Financial => format!(
            "O {} H {} L {} C {}",
            format_number(p.coordinate.tertiary_value),
            format_number(p.coordinate.primary_value),
            format_number(p.coordinate.quinary_value),
            format_number(p.coordinate.quaternary_value)
        ),
        SeriesKind::Box => format!(
            "Min {} Q1 {} Median {} Q3 {} Max {}",
            format_number(p.coordinate.quinary_value),
            format_number(p.coordinate.quaternary_value),
            format_number(p.coordinate.senary_value),
            format_number(p.coordinate.tertiary_value),
            format_number(p.coordinate.primary_value)
        ),
        _ => format_number(value),
    };
    scene.hits.push(HitRegion {
        id: PointId {
            series: si,
            point: p.index,
        },
        shape,
        center,
        value,
        label: format!(
            "{}{}{}",
            s.name,
            if s.name.is_empty() { "" } else { ": " },
            text
        ),
    });
    if s.data_labels {
        scene.primitives.push(
            Primitive::new(
                Geometry::Text {
                    origin: Point::new(center.x + 4.0, center.y - s.data_labels_size - 4.0),
                    text,
                    size: s.data_labels_size,
                    rotation: 0.0,
                    anchor: TextAnchor::Start,
                },
                PaintStyle::fill(s.data_labels_color.unwrap_or_else(|| s.color(si))),
            )
            .clipped(scene.plot_bounds)
            .identified(PointId {
                series: si,
                point: p.index,
            }),
        );
    }
}

pub(crate) fn line_path(points: &[Point], smoothness: f64, step: bool) -> Vec<PathCommand> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut path = Vec::with_capacity(points.len() * if step { 2 } else { 1 });
    path.push(PathCommand::MoveTo(points[0]));
    for i in 1..points.len() {
        let current = points[i - 1];
        let next = points[i];
        if step {
            path.push(PathCommand::LineTo(Point::new(next.x, current.y)));
            path.push(PathCommand::LineTo(next));
        } else if smoothness <= 0.0 {
            path.push(PathCommand::LineTo(next));
        } else {
            let previous = points[i.saturating_sub(2)];
            let after = points[(i + 1).min(points.len() - 1)];
            let curve = crate::kernel::line_spline_math::control_points(
                previous, current, next, after, smoothness,
            );
            path.push(PathCommand::CubicTo {
                control1: curve.control1(),
                control2: curve.control2(),
                end: curve.end(),
            });
        }
    }
    path
}
fn draw_line_series(
    scene: &mut Scene,
    si: usize,
    s: &Series,
    points: &[Option<MeasuredPoint>],
    x: AxisScale,
    y: AxisScale,
    unit_width: f64,
) {
    let mut run = Vec::new();
    let flush = |scene: &mut Scene, run: &mut Vec<&MeasuredPoint>| {
        if run.is_empty() {
            return;
        }
        let top: Vec<_> = run.iter().map(|p| location(p, x, y)).collect();
        let step = matches!(s.kind, SeriesKind::StepLine | SeriesKind::StackedStepArea);
        let smooth = s.line_smoothness.clamp(0.0, 1.0);
        let mut path = line_path(&top, smooth, step);
        if s.kind.is_area() || s.fill.is_some() || s.fill_brush.is_some() {
            let bottom: Vec<_> = run
                .iter()
                .rev()
                .map(|p| {
                    Point::new(
                        x.to_pixels(p.coordinate.secondary_value),
                        baseline(y, p.bottom),
                    )
                })
                .collect();
            if let Some(first) = bottom.first() {
                path.push(PathCommand::LineTo(*first));
            }
            path.extend(line_path(&bottom, smooth, step).into_iter().skip(1));
            path.push(PathCommand::Close);
            let mut style = s.style(si, true);
            if s.fill.is_none() && s.fill_brush.is_none() {
                style.opacity *= 0.35;
            }
            style.stroke = None;
            style.stroke_brush = None;
            scene.primitives.push(
                Primitive::new(Geometry::Path(path), style)
                    .clipped(scene.plot_bounds)
                    .identified(PointId {
                        series: si,
                        point: usize::MAX,
                    }),
            );
        }
        let mut style = s.style(si, false);
        style.fill = None;
        style.fill_brush = None;
        scene.primitives.push(
            Primitive::new(Geometry::Path(line_path(&top, smooth, step)), style.clone())
                .clipped(scene.plot_bounds)
                .identified(PointId {
                    series: si,
                    point: usize::MAX,
                }),
        );
        if s.kind == SeriesKind::RangeLine {
            let low: Vec<_> = run
                .iter()
                .map(|p| {
                    Point::new(
                        x.to_pixels(p.coordinate.secondary_value),
                        baseline(y, p.bottom),
                    )
                })
                .collect();
            scene.primitives.push(
                Primitive::new(Geometry::Path(line_path(&low, smooth, false)), style)
                    .clipped(scene.plot_bounds)
                    .identified(PointId {
                        series: si,
                        point: usize::MAX,
                    }),
            );
        }
        run.clear();
    };
    for point in points {
        let Some(p) = point else {
            if s.enable_null_splitting {
                flush(scene, &mut run);
            }
            continue;
        };
        let center = location(p, x, y);
        if !finite_point(center) {
            flush(scene, &mut run);
            continue;
        }
        run.push(p);
    }
    flush(scene, &mut run);

    // Draw markers, error bars and labels over all fills and strokes. Emitting
    // them while building a run lets its subsequent area fill obscure the points.
    for p in points.iter().flatten() {
        let center = location(p, x, y);
        if !finite_point(center) {
            continue;
        }
        let low = Point::new(center.x, baseline(y, p.bottom));
        let range = s.kind == SeriesKind::RangeLine;
        if range && !finite_point(low) {
            continue;
        }
        let marker_centers = [center, low];
        for marker in &marker_centers[..if range { 2 } else { 1 }] {
            if s.geometry_size > 0.0 {
                scene.primitives.push(
                    Primitive::new(
                        Geometry::Ellipse(Rect::new(
                            marker.x - s.geometry_size / 2.0,
                            marker.y - s.geometry_size / 2.0,
                            s.geometry_size,
                            s.geometry_size,
                        )),
                        s.style(si, true),
                    )
                    .clipped(scene.plot_bounds)
                    .identified(PointId {
                        series: si,
                        point: p.index,
                    }),
                );
            }
        }
        let marker_radius = (s.geometry_size / 2.0).max(0.5);
        let marker_shape = |center| HitShape::Circle {
            center,
            radius: marker_radius,
        };
        let exact = if range {
            HitShape::Compound(vec![marker_shape(center), marker_shape(low)])
        } else {
            marker_shape(center)
        };
        let slot = category_width(x, p.coordinate.secondary_value, unit_width).max(1.0);
        let hover_bounds = if range {
            Rect::new(
                center.x - slot * 0.5,
                center.y.min(low.y),
                slot,
                (center.y - low.y).abs().max(1.0),
            )
        } else {
            Rect::new(
                center.x - slot * 0.5,
                center.y - marker_radius,
                slot,
                marker_radius * 2.0,
            )
        };
        append_hit(
            scene,
            si,
            s,
            p,
            HitShape::WithHoverArea {
                shape: Box::new(exact),
                bounds: hover_bounds,
            },
            if range {
                Point::new(center.x, (center.y + low.y) * 0.5)
            } else {
                center
            },
        );
        if !range {
            draw_errors(scene, si, s, p, x, y);
        }
    }
}
fn category_width(scale: AxisScale, category: f64, unit: f64) -> f64 {
    (scale.to_pixels(category + unit.abs() * 0.5) - scale.to_pixels(category - unit.abs() * 0.5))
        .abs()
}
fn bar_group(si: usize, s: &Series, all: &[Series]) -> (usize, usize) {
    let mut keys = Vec::new();
    let mut own = 0;
    for (i, candidate) in all.iter().enumerate().filter(|(_, a)| {
        a.visible
            && a.kind.is_bar()
            && !matches!(a.kind, SeriesKind::Financial | SeriesKind::Box)
            && a.kind.is_row() == s.kind.is_row()
            && a.x_axis == s.x_axis
            && a.y_axis == s.y_axis
    }) {
        let key = if candidate.kind.is_stacked() {
            (true, candidate.stack_group as usize)
        } else {
            (false, i)
        };
        let pos = if let Some(pos) = keys.iter().position(|k| *k == key) {
            pos
        } else {
            keys.push(key);
            keys.len() - 1
        };
        if i == si {
            own = pos;
        }
    }
    (
        if s.ignores_bar_position {
            1
        } else {
            keys.len().max(1)
        },
        own,
    )
}
fn bar_layout(
    s: &Series,
    scale: AxisScale,
    category: f64,
    unit: f64,
    group: (usize, usize),
) -> (f64, f64) {
    let (count, own) = group;
    let raw = category_width(scale, category, unit);
    let group_width = (raw - s.padding.max(0.0)).max(1.0);
    let slot = (group_width / count as f64).min(s.max_bar_width.max(1.0));
    let width = (slot - s.padding.max(0.0)).max(1.0);
    let offset = if s.ignores_bar_position {
        0.0
    } else {
        (own as f64 - (count as f64 - 1.0) * 0.5) * slot
    };
    (width, offset)
}
fn draw_bars(
    scene: &mut Scene,
    si: usize,
    s: &Series,
    all: &[Series],
    points: &[Option<MeasuredPoint>],
    x: AxisScale,
    y: AxisScale,
    xa: &[Axis],
    ya: &[Axis],
) {
    let group = bar_group(si, s, all);
    for p in points.iter().flatten() {
        let row = s.kind.is_row();
        let category = p.coordinate.secondary_value;
        let scale = if row { y } else { x };
        let unit = if row {
            ya[s.y_axis].unit_width
        } else {
            xa[s.x_axis].unit_width
        };
        let (width, offset) = bar_layout(s, scale, category, unit, group);
        let center_category = scale.to_pixels(category) + offset;
        let value_scale = if row { x } else { y };
        let a = baseline(value_scale, p.bottom);
        let b = value_scale.to_pixels(p.top);
        if ![width, center_category, a, b].iter().all(|v| v.is_finite()) {
            continue;
        }
        let rect = if row {
            Rect::new(
                a.min(b),
                center_category - width / 2.0,
                (b - a).abs(),
                width,
            )
        } else {
            Rect::new(
                center_category - width / 2.0,
                a.min(b),
                width,
                (b - a).abs(),
            )
        };
        scene.primitives.push(
            Primitive::new(
                Geometry::Rectangle {
                    rect,
                    radius: s.corner_radius.max(0.0),
                },
                s.style(si, true),
            )
            .clipped(scene.plot_bounds)
            .identified(PointId {
                series: si,
                point: p.index,
            }),
        );
        let center = if row {
            Point::new(b, center_category)
        } else {
            Point::new(center_category, b)
        };
        let slot = category_width(scale, category, unit) / group.0 as f64;
        let hover_bounds = if row {
            Rect::new(
                rect.x,
                center_category - slot * 0.5,
                rect.width.max(1.0),
                slot,
            )
        } else {
            Rect::new(
                center_category - slot * 0.5,
                rect.y,
                slot,
                rect.height.max(1.0),
            )
        };
        append_hit(
            scene,
            si,
            s,
            p,
            HitShape::WithHoverArea {
                shape: Box::new(HitShape::Rectangle(rect)),
                bounds: hover_bounds,
            },
            center,
        );
        draw_errors(scene, si, s, p, x, y);
    }
}
fn draw_scatter(
    scene: &mut Scene,
    si: usize,
    s: &Series,
    points: &[Option<MeasuredPoint>],
    x: AxisScale,
    y: AxisScale,
) {
    let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in points.iter().flatten() {
        min = min.min(p.coordinate.tertiary_value);
        max = max.max(p.coordinate.tertiary_value);
    }
    for p in points.iter().flatten() {
        let center = location(p, x, y);
        if !finite_point(center) {
            continue;
        }
        let t = if max == min {
            1.0
        } else {
            (p.coordinate.tertiary_value - min) / (max - min)
        };
        let size = (s.min_geometry_size + (s.geometry_size - s.min_geometry_size) * t).max(0.0);
        scene.primitives.push(
            Primitive::new(
                Geometry::Ellipse(Rect::new(
                    center.x - size * 0.5,
                    center.y - size * 0.5,
                    size,
                    size,
                )),
                s.style(si, true),
            )
            .clipped(scene.plot_bounds)
            .identified(PointId {
                series: si,
                point: p.index,
            }),
        );
        append_hit(
            scene,
            si,
            s,
            p,
            HitShape::Circle {
                center,
                radius: (size * 0.5).max(5.0),
            },
            center,
        );
        draw_errors(scene, si, s, p, x, y);
    }
}
fn draw_heat(
    scene: &mut Scene,
    si: usize,
    s: &Series,
    points: &[Option<MeasuredPoint>],
    x: AxisScale,
    y: AxisScale,
    xa: &[Axis],
    ya: &[Axis],
) {
    let dx = minimum_delta(
        points
            .iter()
            .flatten()
            .map(|p| p.coordinate.secondary_value),
    )
    .unwrap_or(xa[s.x_axis].unit_width);
    let dy = minimum_delta(points.iter().flatten().map(|p| p.coordinate.primary_value))
        .unwrap_or(ya[s.y_axis].unit_width);
    let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in points.iter().flatten() {
        min = min.min(p.coordinate.tertiary_value);
        max = max.max(p.coordinate.tertiary_value);
    }
    for p in points.iter().flatten() {
        let c = p.coordinate;
        let a = Point::new(
            x.to_pixels(c.secondary_value - dx / 2.0),
            y.to_pixels(c.primary_value - dy / 2.0),
        );
        let b = Point::new(
            x.to_pixels(c.secondary_value + dx / 2.0),
            y.to_pixels(c.primary_value + dy / 2.0),
        );
        if !finite_point(a) || !finite_point(b) {
            continue;
        }
        let rect = Rect::new(
            a.x.min(b.x),
            a.y.min(b.y),
            (b.x - a.x).abs(),
            (b.y - a.y).abs(),
        );
        let t = if max == min {
            0.0
        } else {
            (c.tertiary_value - min) / (max - min)
        };
        let color = heat_color(&s.heat_map, t);
        scene.primitives.push(
            Primitive::new(
                Geometry::Rectangle { rect, radius: 0.0 },
                PaintStyle {
                    opacity: s.opacity,
                    ..PaintStyle::fill(color)
                },
            )
            .clipped(scene.plot_bounds)
            .identified(PointId {
                series: si,
                point: p.index,
            }),
        );
        append_hit(
            scene,
            si,
            s,
            p,
            HitShape::Rectangle(rect),
            Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5),
        );
    }
}
pub(crate) fn heat_color(colors: &[Color], t: f64) -> Color {
    if colors.is_empty() {
        return Color::from_argb(0, 0, 0, 0);
    }
    if colors.len() == 1 {
        return colors[0];
    }
    let pos = t.clamp(0.0, 1.0) * (colors.len() - 1) as f64;
    let i = (pos.floor() as usize).min(colors.len() - 2);
    let f = (pos - i as f64).clamp(0.0, 1.0);
    let (a, b) = (colors[i], colors[i + 1]);
    let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * f) as u8;
    Color::from_argb(mix(a.a, b.a), mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b))
}
fn draw_financial(
    scene: &mut Scene,
    si: usize,
    s: &Series,
    points: &[Option<MeasuredPoint>],
    x: AxisScale,
    y: AxisScale,
    xa: &[Axis],
) {
    for p in points.iter().flatten() {
        let c = p.coordinate;
        let center = x.to_pixels(c.secondary_value);
        let high = y.to_pixels(c.primary_value);
        let low = y.to_pixels(c.quinary_value);
        let open = y.to_pixels(c.tertiary_value);
        let close = y.to_pixels(c.quaternary_value);
        let category_width = category_width(x, c.secondary_value, xa[s.x_axis].unit_width);
        let width = category_width.min(s.max_bar_width.max(1.0));
        if ![center, high, low, open, close, width]
            .iter()
            .all(|v| v.is_finite())
        {
            continue;
        }
        let mut style = s.style(si, true);
        if s.kind == SeriesKind::Financial && s.fill.is_none() && s.fill_brush.is_none() {
            let color = if c.quaternary_value >= c.tertiary_value {
                Color::from_rgb(22, 163, 74)
            } else {
                Color::from_rgb(220, 38, 38)
            };
            style.fill = Some(color);
            style.stroke = Some(color);
        }
        let line_style = PaintStyle {
            fill: None,
            fill_brush: None,
            ..style.clone()
        };
        scene.primitives.push(
            Primitive::new(
                Geometry::Line {
                    start: Point::new(center, high),
                    end: Point::new(center, low),
                },
                line_style.clone(),
            )
            .clipped(scene.plot_bounds)
            .identified(PointId {
                series: si,
                point: p.index,
            }),
        );
        let body = Rect::new(
            center - width / 2.0,
            open.min(close),
            width,
            (close - open).abs().max(1.0),
        );
        scene.primitives.push(
            Primitive::new(
                Geometry::Rectangle {
                    rect: body,
                    radius: s.corner_radius.max(0.0),
                },
                style,
            )
            .clipped(scene.plot_bounds)
            .identified(PointId {
                series: si,
                point: p.index,
            }),
        );
        if s.kind == SeriesKind::Box {
            for value in [high, low, y.to_pixels(c.senary_value)] {
                scene.primitives.push(
                    Primitive::new(
                        Geometry::Line {
                            start: Point::new(center - width / 2.0, value),
                            end: Point::new(center + width / 2.0, value),
                        },
                        line_style.clone(),
                    )
                    .clipped(scene.plot_bounds)
                    .identified(PointId {
                        series: si,
                        point: p.index,
                    }),
                );
            }
        }
        let hit = Rect::new(
            center - width / 2.0,
            high.min(low),
            width,
            (low - high).abs(),
        );
        append_hit(
            scene,
            si,
            s,
            p,
            HitShape::WithHoverArea {
                shape: Box::new(HitShape::Rectangle(hit)),
                bounds: Rect::new(
                    center - category_width * 0.5,
                    hit.y,
                    category_width,
                    hit.height.max(1.0),
                ),
            },
            Point::new(center, (high + low) * 0.5),
        );
    }
}
fn draw_errors(
    scene: &mut Scene,
    si: usize,
    s: &Series,
    p: &MeasuredPoint,
    x: AxisScale,
    y: AxisScale,
) {
    let c = p.coordinate;
    let error = c.point_error;
    if error.is_empty {
        return;
    }
    let (vx, vy) = if s.kind.is_row() {
        (p.top, c.secondary_value)
    } else {
        (c.secondary_value, p.top)
    };
    let center = Point::new(x.to_pixels(vx), y.to_pixels(vy));
    let cap = 4.0;
    let style = PaintStyle::stroke(s.color(si), s.stroke_width.max(1.0));
    let left = x.to_pixels(vx - error.xi.abs());
    let right = x.to_pixels(vx + error.xj.abs());
    let top = y.to_pixels(vy + error.yi.abs());
    let bottom = y.to_pixels(vy - error.yj.abs());
    if ![left, right, top, bottom].iter().all(|v| v.is_finite()) {
        return;
    }
    for (a, b) in [
        (Point::new(left, center.y), Point::new(right, center.y)),
        (
            Point::new(left, center.y - cap),
            Point::new(left, center.y + cap),
        ),
        (
            Point::new(right, center.y - cap),
            Point::new(right, center.y + cap),
        ),
        (Point::new(center.x, top), Point::new(center.x, bottom)),
        (
            Point::new(center.x - cap, top),
            Point::new(center.x + cap, top),
        ),
        (
            Point::new(center.x - cap, bottom),
            Point::new(center.x + cap, bottom),
        ),
    ] {
        scene.primitives.push(
            Primitive::new(Geometry::Line { start: a, end: b }, style.clone())
                .clipped(scene.plot_bounds)
                .identified(PointId {
                    series: si,
                    point: p.index,
                }),
        );
    }
}
fn minimum_delta(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut values: Vec<_> = values.filter(|v| v.is_finite()).collect();
    values.sort_by(f64::total_cmp);
    values.dedup();
    values
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|v| v.is_finite() && *v > 0.0)
        .min_by(f64::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene(series: &[Series]) -> Scene {
        CartesianChartEngine::measure(
            Size::new(640.0, 400.0),
            series,
            &[],
            &[],
            &CartesianOptions::default(),
        )
        .unwrap()
    }
    #[test]
    fn line_nulls_split_paths_and_keep_original_point_indices() {
        let mut s = Series::from_values(SeriesKind::Line, [1.0, 2.0, 3.0, 4.0]);
        s.values[1] = Coordinate::EMPTY;
        s.geometry_size = 0.0;
        let result = scene(&[s]);
        assert_eq!(
            result.hits.iter().map(|h| h.id.point).collect::<Vec<_>>(),
            vec![0, 2, 3]
        );
        assert_eq!(
            result
                .primitives
                .iter()
                .filter(|p| matches!(p.geometry, Geometry::Path(_)))
                .count(),
            2
        );
    }
    #[test]
    fn line_can_connect_across_nulls() {
        let mut s = Series::from_values(SeriesKind::Line, [1.0, 2.0, 3.0]);
        s.values[1] = Coordinate::EMPTY;
        s.enable_null_splitting = false;
        s.geometry_size = 0.0;
        assert_eq!(
            scene(&[s])
                .primitives
                .iter()
                .filter(|p| matches!(p.geometry, Geometry::Path(_)))
                .count(),
            1
        );
    }
    #[test]
    fn range_bounds_include_both_endpoints() {
        let mut s = Series::new(SeriesKind::RangeColumn);
        s.values = vec![Coordinate::range(0.0, 10.0, -50.0)];
        let m = CartesianChartEngine::measure_with_axes(
            Size::new(640.0, 400.0),
            &[s],
            &[],
            &[],
            &CartesianOptions::default(),
        )
        .unwrap();
        assert!(m.y_scales[0].min <= -50.0);
        assert!(m.y_scales[0].max >= 10.0);
    }
    #[test]
    fn stacked_positive_and_negative_bounds_are_independent() {
        let a = Series::from_values(SeriesKind::StackedColumn, [3.0, -2.0]);
        let b = Series::from_values(SeriesKind::StackedColumn, [4.0, -5.0]);
        let m = CartesianChartEngine::measure_with_axes(
            Size::new(640.0, 400.0),
            &[a, b],
            &[],
            &[],
            &CartesianOptions::default(),
        )
        .unwrap();
        assert!(m.y_scales[0].min <= -7.0);
        assert!(m.y_scales[0].max >= 7.0);
        assert_eq!(m.scene.hits.len(), 4);
    }
    #[test]
    fn stacked_hundred_percent_normalizes_categories() {
        let mut a = Series::from_values(SeriesKind::StackedColumn, [3.0, 1.0]);
        a.stack_100_percent = true;
        let mut b = Series::from_values(SeriesKind::StackedColumn, [1.0, 4.0]);
        b.stack_100_percent = true;
        let m = CartesianChartEngine::measure_with_axes(
            Size::new(640.0, 400.0),
            &[a, b],
            &[],
            &[],
            &CartesianOptions::default(),
        )
        .unwrap();
        assert!(m.y_scales[0].max >= 100.0 && m.y_scales[0].max < 110.0);
    }
    #[test]
    fn nonfinite_points_are_gaps_not_renderer_inputs() {
        let result = scene(&[Series::from_values(
            SeriesKind::Line,
            [f64::NAN, 1.0, f64::INFINITY, 2.0],
        )]);
        assert_eq!(result.hits.len(), 2);
    }
    #[test]
    fn missing_axis_is_an_error() {
        let mut s = Series::default();
        s.y_axis = 2;
        assert!(CartesianChartEngine::measure(
            Size::new(640.0, 400.0),
            &[s],
            &[],
            &[],
            &CartesianOptions::default()
        )
        .is_err());
    }
    #[test]
    fn grouped_bars_have_distinct_centers_and_shared_category() {
        let result = scene(&[
            Series::from_values(SeriesKind::Column, [3.0]),
            Series::from_values(SeriesKind::Column, [4.0]),
        ]);
        assert_ne!(result.hits[0].center.x, result.hits[1].center.x);
    }
    #[test]
    fn all_cartesian_series_emit_measured_hits() {
        for kind in [
            SeriesKind::Line,
            SeriesKind::StepLine,
            SeriesKind::Area,
            SeriesKind::StackedArea,
            SeriesKind::StackedStepArea,
            SeriesKind::Column,
            SeriesKind::Row,
            SeriesKind::StackedColumn,
            SeriesKind::StackedRow,
            SeriesKind::Scatter,
            SeriesKind::Heat,
            SeriesKind::RangeColumn,
            SeriesKind::RangeRow,
            SeriesKind::RangeLine,
            SeriesKind::Financial,
            SeriesKind::Box,
        ] {
            let mut s = Series::new(kind);
            s.values = vec![
                Coordinate::box_value(0.0, 10.0, 7.0, 3.0, 1.0, 5.0),
                Coordinate::box_value(1.0, 12.0, 8.0, 4.0, 2.0, 6.0),
            ];
            assert_eq!(scene(&[s]).hits.len(), 2, "{kind:?}");
        }
    }
    #[test]
    fn heat_cells_infer_datetime_spacing() {
        let mut s = Series::new(SeriesKind::Heat);
        s.values = vec![
            Coordinate::weighted(1_000_000.0, 1.0, 3.0),
            Coordinate::weighted(1_100_000.0, 2.0, 4.0),
        ];
        let result = scene(&[s]);
        let HitShape::Rectangle(rect) = result.hits[0].shape else {
            panic!("expected heat rect")
        };
        assert!(rect.width > 100.0);
    }
    #[test]
    fn numeric_extremes_scale_without_overflow() {
        let s = Series::from_values(SeriesKind::Line, [-1e308, 1e308]);
        let result = scene(&[s]);
        assert_eq!(result.hits.len(), 2);
        assert!(result.hits.iter().all(|h| finite_point(h.center)));
    }
    #[test]
    fn logarithmic_axes_round_trip() {
        let axis = Axis {
            min_limit: Some(1.0),
            max_limit: Some(1000.0),
            logarithmic_base: Some(10.0),
            ..Axis::default()
        };
        let s = AxisScale::new(&axis, 1.0, 1000.0, 0.0, 300.0).unwrap();
        assert!((s.to_pixels(10.0) - 100.0).abs() < 1e-8);
        assert!((s.to_chart_values(200.0) - 100.0).abs() < 1e-8);
    }
    #[test]
    fn inverted_axis_round_trips() {
        let axis = Axis {
            inverted: true,
            ..Axis::default()
        };
        let s = AxisScale::new(&axis, 0.0, 10.0, 0.0, 100.0).unwrap();
        assert!(s.to_pixels(0.0) > s.to_pixels(10.0));
        assert!((s.to_chart_values(s.to_pixels(3.0)) - 3.0).abs() < 1e-10);
    }
    #[test]
    fn automatic_hover_compares_the_shared_category_axis() {
        // Kernel.Extensions.GetFindingStrategy: vertical series prefer X, rows Y,
        // and mixing orientations (or including scatter) compares both dimensions.
        let result = scene(&[Series::from_values(SeriesKind::Line, [2.0, 8.0])]);
        let x = result.hits[0].center.x;
        let pointer = Point::new(x, result.plot_bounds.y + result.plot_bounds.height * 0.5);
        assert_eq!(
            result
                .hit_test(pointer, FindingStrategy::Automatic, 0.0)
                .len(),
            1
        );
        assert!(result
            .hit_test(pointer, FindingStrategy::CompareAll, 0.0)
            .is_empty());
        let rows = scene(&[Series::from_values(SeriesKind::Row, [2.0, 8.0])]);
        assert_eq!(
            rows.automatic_finding_strategy,
            FindingStrategy::CompareOnlyYTakeClosest
        );
        let mixed = scene(&[
            Series::from_values(SeriesKind::Column, [2.0]),
            Series::from_values(SeriesKind::Scatter, [4.0]),
        ]);
        assert_eq!(
            mixed.automatic_finding_strategy,
            FindingStrategy::CompareAllTakeClosest
        );
    }
    #[test]
    fn range_hover_covers_band_but_exact_match_targets_either_marker() {
        // CoreRangeLineSeries.Invalidate gives one band hover area, while its
        // FindPointsInPosition override tests both high and low marker visuals.
        let mut range = Series::new(SeriesKind::RangeLine);
        range.values = vec![Coordinate::range(0.0, 10.0, 2.0)];
        let result = scene(&[range]);
        let hit = &result.hits[0];
        let HitShape::WithHoverArea { shape, .. } = &hit.shape else {
            panic!("range hover");
        };
        let HitShape::Compound(markers) = shape.as_ref() else {
            panic!("range endpoints");
        };
        let HitShape::Circle { center: low, .. } = markers[1] else {
            panic!("low endpoint");
        };
        assert_eq!(
            result.hit_test(low, FindingStrategy::ExactMatch, 0.0).len(),
            1
        );
        assert_eq!(
            result
                .hit_test(hit.center, FindingStrategy::CompareAll, 0.0)
                .len(),
            1
        );
        assert!(result
            .hit_test(hit.center, FindingStrategy::ExactMatch, 0.0)
            .is_empty());
        assert!(hit.label.contains('2') && hit.label.contains("10"));
    }
    #[test]
    fn narrow_column_hover_uses_category_slot_and_exact_match_uses_visual() {
        // BarSeries separates full category hover width from MaxBarWidth.
        let mut column = Series::from_values(SeriesKind::Column, [4.0]);
        column.max_bar_width = 5.0;
        let result = scene(&[column]);
        let hit = &result.hits[0];
        let bounds = hit.shape.hover_bounds();
        let pointer = Point::new(
            bounds.x + bounds.width * 0.25,
            bounds.y + bounds.height * 0.5,
        );
        assert_eq!(
            result
                .hit_test(pointer, FindingStrategy::Automatic, 0.0)
                .len(),
            1
        );
        assert!(result
            .hit_test(pointer, FindingStrategy::ExactMatch, 0.0)
            .is_empty());
    }
    #[test]
    fn line_fill_is_behind_markers_and_labels() {
        let mut area = Series::from_values(SeriesKind::Area, [1.0, 3.0, 2.0]);
        area.fill = Some(Color::from_rgb(255, 0, 0));
        area.data_labels = true;
        let result = scene(&[area]);
        let last_path = result
            .primitives
            .iter()
            .rposition(|p| matches!(p.geometry, Geometry::Path(_)))
            .unwrap();
        let first_marker = result
            .primitives
            .iter()
            .position(|p| matches!(p.geometry, Geometry::Ellipse(_)))
            .unwrap();
        assert!(last_path < first_marker);
    }
    #[test]
    fn duplicate_heat_series_do_not_expand_shared_bounds_twice() {
        let mut heat = Series::new(SeriesKind::Heat);
        heat.values = vec![
            Coordinate::weighted(0.0, 1.0, 42.0),
            Coordinate::weighted(2.0, 3.0, 7.0),
        ];
        let measure = |series: &[Series]| {
            CartesianChartEngine::measure_with_axes(
                Size::new(640.0, 400.0),
                series,
                &[],
                &[],
                &CartesianOptions::default(),
            )
            .unwrap()
        };
        let single = measure(&[heat.clone()]);
        let duplicate = measure(&[heat.clone(), heat]);
        assert_eq!(single.x_scales, duplicate.x_scales);
        assert_eq!(single.y_scales, duplicate.y_scales);
        assert_eq!(single.scene.hits[0].value, 42.0);
        assert_eq!(single.scene.hits[0].label, "42");
    }
}
