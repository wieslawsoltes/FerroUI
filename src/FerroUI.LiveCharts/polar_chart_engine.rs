//! Polar/radar lines and filled polygons with angular and radial axes.

use crate::axis::{Axis, AxisScale, format_number};
use crate::cartesian_chart_engine::line_path;
use crate::scene::*;
use crate::series::Series;
use ferroui_base::{Point, Rect, Size};

#[derive(Clone, Debug, PartialEq)]
pub struct PolarOptions {
    pub draw_margin: Option<Rect>,
    pub inner_radius: f64,
    pub initial_rotation: f64,
    pub total_angle: f64,
}
impl Default for PolarOptions {
    fn default() -> Self {
        Self {
            draw_margin: None,
            inner_radius: 0.0,
            initial_rotation: -90.0,
            total_angle: 360.0,
        }
    }
}

pub struct PolarChartEngine;
impl PolarChartEngine {
    pub fn measure(
        size: Size,
        series: &[Series],
        angle_axis: &Axis,
        radius_axis: &Axis,
        options: &PolarOptions,
    ) -> Result<Scene, ChartError> {
        validate_size(size)?;
        if [
            options.inner_radius,
            options.initial_rotation,
            options.total_angle,
        ]
        .iter()
        .any(|v| !v.is_finite())
            || options.inner_radius < 0.0
            || options.total_angle <= 0.0
            || options.total_angle > 360.0
        {
            return Err(ChartError::InvalidData(
                "polar dimensions and angles must be finite with total angle in (0,360]".into(),
            ));
        }
        let margin = 36.0_f64.min(size.width.min(size.height) * 0.5);
        let plot = options.draw_margin.unwrap_or(Rect::new(
            margin,
            margin,
            (size.width - margin * 2.0).max(0.0),
            (size.height - margin * 2.0).max(0.0),
        ));
        if [plot.x, plot.y, plot.width, plot.height]
            .iter()
            .any(|v| !v.is_finite())
            || plot.width < 0.0
            || plot.height < 0.0
        {
            return Err(ChartError::InvalidData("invalid polar draw margin".into()));
        }
        let center = Point::new(plot.x + plot.width * 0.5, plot.y + plot.height * 0.5);
        let outer = plot.width.min(plot.height) * 0.5;
        let inner = options.inner_radius.min(outer);
        let (mut amin, mut amax, mut rmin, mut rmax) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for s in series.iter().filter(|s| s.visible) {
            for c in s.values.iter().filter(|c| c.is_finite()) {
                amin = amin.min(c.secondary_value);
                amax = amax.max(c.secondary_value);
                rmin = rmin.min(c.primary_value);
                rmax = rmax.max(c.primary_value);
            }
        }
        if !amin.is_finite() {
            amin = 0.0;
            amax = 360.0;
        }
        if !rmin.is_finite() {
            rmin = 0.0;
            rmax = 10.0;
        }
        // Named categories occupy equal angular slots. A closed polygon does not
        // collapse its last category onto the first category.
        let category_count = angle_axis.labels.len();
        if category_count > 0 {
            amin = 0.0;
            amax = category_count as f64;
        } else if amin == amax {
            amax = amin + 1.0;
        }
        let mut aa = angle_axis.clone();
        aa.padding = 0.0;
        let mut ra = radius_axis.clone();
        ra.padding = 0.0;
        if ra.min_limit.is_none() {
            rmin = rmin.min(0.0);
        }
        let angle = AxisScale::new(
            &aa,
            amin,
            amax,
            options.initial_rotation,
            options.initial_rotation + options.total_angle,
        )?;
        let radial = AxisScale::new(&ra, rmin, rmax, inner, outer)?;
        let point = |av: f64, rv: f64| {
            let a = angle.to_pixels(av).to_radians();
            let r = radial.to_pixels(rv);
            Point::new(center.x + a.cos() * r, center.y + a.sin() * r)
        };
        let mut scene = Scene::new(size, plot);
        if radius_axis.visible {
            for value in radial.ticks(radius_axis, outer - inner) {
                let r = radial.to_pixels(value);
                if !r.is_finite() || r < 0.0 {
                    continue;
                }
                if radius_axis.show_separator_lines {
                    scene.primitives.push(Primitive::new(
                        Geometry::Ellipse(Rect::new(center.x - r, center.y - r, r * 2.0, r * 2.0)),
                        PaintStyle::stroke(radius_axis.separators_color, 1.0),
                    ));
                }
                let origin = point(angle.min, value);
                scene.primitives.push(Primitive::new(
                    Geometry::Text {
                        origin,
                        text: radius_axis.label(value),
                        size: radius_axis.text_size,
                        rotation: radius_axis.labels_rotation,
                        anchor: TextAnchor::Start,
                    },
                    PaintStyle::fill(radius_axis.labels_color),
                ));
            }
        }
        if angle_axis.visible {
            let mut ticks = angle.ticks(angle_axis, outer * std::f64::consts::TAU);
            if category_count > 0 {
                ticks = (0..category_count).map(|i| i as f64).collect();
            }
            for value in ticks {
                let a = angle.to_pixels(value).to_radians();
                let start = Point::new(center.x + a.cos() * inner, center.y + a.sin() * inner);
                let end = Point::new(center.x + a.cos() * outer, center.y + a.sin() * outer);
                if angle_axis.show_separator_lines {
                    scene.primitives.push(Primitive::new(
                        Geometry::Line { start, end },
                        PaintStyle::stroke(angle_axis.separators_color, 1.0),
                    ));
                }
                scene.primitives.push(Primitive::new(
                    Geometry::Text {
                        origin: Point::new(
                            center.x + a.cos() * (outer + 12.0),
                            center.y + a.sin() * (outer + 12.0),
                        ),
                        text: angle_axis.label(value),
                        size: angle_axis.text_size,
                        rotation: angle_axis.labels_rotation,
                        anchor: if a.cos() > 0.2 {
                            TextAnchor::Start
                        } else if a.cos() < -0.2 {
                            TextAnchor::End
                        } else {
                            TextAnchor::Center
                        },
                    },
                    PaintStyle::fill(angle_axis.labels_color),
                ));
            }
        }
        for (si, s) in series.iter().enumerate().filter(|(_, s)| s.visible) {
            if [
                s.geometry_size,
                s.line_smoothness,
                s.stroke_width,
                s.opacity,
                s.data_labels_size,
            ]
            .iter()
            .any(|v| !v.is_finite())
            {
                return Err(ChartError::InvalidData(
                    "polar series dimensions and paint values must be finite".into(),
                ));
            }
            scene.legend.push(LegendItem {
                series: si,
                name: if s.name.is_empty() {
                    format!("Series {}", si + 1)
                } else {
                    s.name.clone()
                },
                color: s.color(si),
            });
            let mut runs: Vec<Vec<Point>> = vec![Vec::new()];
            let mut has_gap = false;
            for (index, c) in s.values.iter().enumerate() {
                if !c.is_finite() {
                    if s.enable_null_splitting && !runs.last().unwrap().is_empty() {
                        runs.push(Vec::new());
                        has_gap = true;
                    }
                    continue;
                }
                let p = point(c.secondary_value, c.primary_value);
                if !p.x.is_finite() || !p.y.is_finite() {
                    continue;
                }
                runs.last_mut().unwrap().push(p);
                let id = PointId {
                    series: si,
                    point: index,
                };
                if s.geometry_size > 0.0 {
                    scene.primitives.push(
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
                scene.hits.push(HitRegion {
                    id,
                    shape: HitShape::Circle {
                        center: p,
                        radius: (s.geometry_size * 0.5).max(5.0),
                    },
                    center: p,
                    value: c.primary_value,
                    label: format!(
                        "{}{}{}",
                        s.name,
                        if s.name.is_empty() { "" } else { ": " },
                        format_number(c.primary_value)
                    ),
                });
                if s.data_labels {
                    scene.primitives.push(
                        Primitive::new(
                            Geometry::Text {
                                origin: Point::new(p.x + 4.0, p.y - s.data_labels_size),
                                text: format_number(c.primary_value),
                                size: s.data_labels_size,
                                rotation: 0.0,
                                anchor: TextAnchor::Start,
                            },
                            PaintStyle::fill(s.data_labels_color.unwrap_or_else(|| s.color(si))),
                        )
                        .identified(id),
                    );
                }
            }
            for mut run in runs.into_iter().filter(|r| !r.is_empty()) {
                if s.polar_closed && !has_gap && run.len() > 2 {
                    run.push(run[0]);
                }
                let mut path = line_path(&run, s.line_smoothness.clamp(0.0, 1.0), false);
                let closed = s.polar_closed && !has_gap && run.len() > 3;
                if closed {
                    path.push(PathCommand::Close);
                }
                let mut style = s.style(si, closed && (s.fill.is_some() || s.fill_brush.is_some()));
                if !closed {
                    style.fill = None;
                    style.fill_brush = None;
                }
                scene.primitives.push(
                    Primitive::new(Geometry::Path(path), style)
                        .clipped(plot)
                        .identified(PointId {
                            series: si,
                            point: usize::MAX,
                        }),
                );
            }
        }
        Ok(scene)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SeriesKind, kernel::Coordinate};
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
        assert!(
            scene.primitives.iter().all(
                |p| !matches!(&p.geometry,Geometry::Path(v) if v.contains(&PathCommand::Close))
            )
        );
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
        assert!(
            scene
                .hits
                .iter()
                .all(|h| h.center.x.is_finite() && h.center.y.is_finite())
        );
    }
}
