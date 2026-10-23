//! Geographic chart layout and interaction over the native drawing scene.
// Sources: LiveChartsCore/GeoMapChart.cs; SkiaSharp/MapFactory.cs (projection math only).
// Copyright (c) 2021 Alberto Rodriguez Orozco & LiveCharts Contributors. MIT.

pub use crate::geo::{DrawnMap, GeoCoordinate, GeographicBounds, MapProjection};
use crate::geo::{GeoPolygon, MapProjector, OrthographicProjector, Projector};
use crate::heat_functions::HeatPalette;
use crate::scene::{
    validate_size, ChartError, Geometry, HitRegion, HitShape, PaintStyle, PathCommand, PointId,
    Primitive, Scene, TextAnchor,
};
use ferroui_base::{media::Color, Point, Rect, Size};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct HeatLand {
    pub name: String,
    pub value: f64,
    pub color: Option<Color>,
}
impl HeatLand {
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            color: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoMarker {
    pub coordinate: GeoCoordinate,
    pub radius: f64,
    pub label: String,
    pub value: f64,
    pub color: Color,
    pub show_label: bool,
}
impl GeoMarker {
    pub fn new(coordinate: GeoCoordinate, label: impl Into<String>) -> Self {
        Self {
            coordinate,
            radius: 5.0,
            label: label.into(),
            value: 0.0,
            color: Color::from_rgb(66, 99, 235),
            show_label: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GeoData {
    pub map: DrawnMap,
    pub heat: Vec<HeatLand>,
    pub markers: Vec<GeoMarker>,
}
impl GeoData {
    pub fn from_geojson(json: &str) -> Result<Self, ChartError> {
        Ok(Self {
            map: DrawnMap::from_geojson(json)?,
            ..Self::default()
        })
    }
    pub fn world() -> Result<Self, ChartError> {
        Ok(Self {
            map: DrawnMap::world()?,
            ..Self::default()
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoMapOptions {
    pub projection: MapProjection,
    pub geographic_bounds: GeographicBounds,
    pub center: GeoCoordinate,
    pub zoom: f64,
    pub pan: Point,
    pub min_zoom: f64,
    pub max_zoom: f64,
    pub heat_colors: Vec<Color>,
    pub heat_stops: Option<Vec<f64>>,
    pub min_heat: Option<f64>,
    pub max_heat: Option<f64>,
    pub land_fill: Color,
    pub land_stroke: Color,
    pub stroke_width: f64,
    pub globe_fill: Option<Color>,
    pub marker_label_size: f64,
    pub marker_label_color: Color,
}
impl Default for GeoMapOptions {
    fn default() -> Self {
        Self {
            projection: MapProjection::Default,
            geographic_bounds: GeographicBounds::default(),
            center: GeoCoordinate::default(),
            zoom: 1.0,
            pan: Point::new(0.0, 0.0),
            min_zoom: 1.0,
            max_zoom: 100.0,
            heat_colors: vec![Color::from_rgb(225, 237, 255), Color::from_rgb(30, 64, 175)],
            heat_stops: None,
            min_heat: None,
            max_heat: None,
            land_fill: Color::from_rgb(221, 226, 234),
            land_stroke: Color::from_rgb(255, 255, 255),
            stroke_width: 0.75,
            globe_fill: Some(Color::from_rgb(239, 246, 255)),
            marker_label_size: 12.0,
            marker_label_color: Color::from_rgb(31, 41, 55),
        }
    }
}
impl GeoMapOptions {
    pub fn reset_viewport(&mut self) {
        self.zoom = 1.0;
        self.pan = Point::new(0.0, 0.0);
    }

    pub fn pan_by(&mut self, delta: Point) -> Result<(), ChartError> {
        let x = self.pan.x + delta.x;
        let y = self.pan.y + delta.y;
        if !x.is_finite() || !y.is_finite() {
            return Err(ChartError::InvalidData("Map pan must remain finite".into()));
        }
        self.pan = Point::new(x, y);
        Ok(())
    }

    /// Scales around a screen pivot, keeping the geographic point under the pointer fixed.
    pub fn zoom_at(&mut self, pivot: Point, factor: f64, size: Size) -> Result<(), ChartError> {
        validate_size(size)?;
        validate_options(self)?;
        if !factor.is_finite() || factor <= 0.0 || !pivot.x.is_finite() || !pivot.y.is_finite() {
            return Err(ChartError::InvalidData(
                "Map zoom factor must be finite and positive and its pivot finite".into(),
            ));
        }
        let next = (self.zoom * factor).clamp(self.min_zoom, self.max_zoom);
        let ratio = next / self.zoom;
        let center = Point::new(size.width * 0.5, size.height * 0.5);
        self.pan = Point::new(
            ratio * self.pan.x + (1.0 - ratio) * (pivot.x - center.x),
            ratio * self.pan.y + (1.0 - ratio) * (pivot.y - center.y),
        );
        self.zoom = next;
        Ok(())
    }

    /// Computes the settled viewport used after a pan gesture. Animation is supplied by the host.
    pub fn constrain_viewport(&mut self, size: Size) -> Result<(), ChartError> {
        validate_size(size)?;
        validate_options(self)?;
        let next = self.zoom.clamp(self.min_zoom, self.max_zoom);
        let ratio = next / self.zoom;
        let max_x = (size.width * (next - 1.0) * 0.5).max(0.0);
        let max_y = (size.height * (next - 1.0) * 0.5).max(0.0);
        self.pan = Point::new(
            (self.pan.x * ratio).clamp(-max_x, max_x),
            (self.pan.y * ratio).clamp(-max_y, max_y),
        );
        self.zoom = next;
        Ok(())
    }
}

/// A reusable transform for cached map paths and inverse pointer coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapViewportTransform {
    pub zoom: f64,
    pub pan: Point,
    pub center: Point,
}
impl MapViewportTransform {
    pub fn transform(self, point: Point) -> Point {
        Point::new(
            (point.x - self.center.x) * self.zoom + self.center.x + self.pan.x,
            (point.y - self.center.y) * self.zoom + self.center.y + self.pan.y,
        )
    }
    pub fn inverse(self, point: Point) -> Option<Point> {
        if !self.zoom.is_finite() || self.zoom <= 0.0 {
            return None;
        }
        let point = Point::new(
            (point.x - self.center.x - self.pan.x) / self.zoom + self.center.x,
            (point.y - self.center.y - self.pan.y) / self.zoom + self.center.y,
        );
        (point.x.is_finite() && point.y.is_finite()).then_some(point)
    }
}

/// Maps a requested longitude onto the shortest rotational path from `from`.
pub fn shortest_rotation_target(from: GeoCoordinate, to: GeoCoordinate) -> GeoCoordinate {
    let delta = (to.longitude - from.longitude + 180.0).rem_euclid(360.0) - 180.0;
    GeoCoordinate::new(from.longitude + delta, to.latitude)
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoMapLayout {
    pub scene: Scene,
    pub projector: Projector,
    pub viewport: MapViewportTransform,
}
impl GeoMapLayout {
    pub fn project(&self, coordinate: GeoCoordinate) -> Option<Point> {
        if !coordinate.is_finite()
            || !self
                .projector
                .is_visible(coordinate.longitude, coordinate.latitude)
        {
            return None;
        }
        let result = self.viewport.transform(
            self.projector
                .to_map(coordinate.longitude, coordinate.latitude),
        );
        (result.x.is_finite() && result.y.is_finite()).then_some(result)
    }
    pub fn unproject(&self, point: Point) -> Option<GeoCoordinate> {
        self.projector.to_coordinates(self.viewport.inverse(point)?)
    }
}

pub struct GeoMapChartEngine;
impl GeoMapChartEngine {
    pub fn measure(
        data: &GeoData,
        size: Size,
        options: &GeoMapOptions,
    ) -> Result<Scene, ChartError> {
        Ok(Self::layout(data, size, options)?.scene)
    }

    pub fn layout(
        data: &GeoData,
        size: Size,
        options: &GeoMapOptions,
    ) -> Result<GeoMapLayout, ChartError> {
        validate_size(size)?;
        validate_options(options)?;
        let control_bounds = Rect::new(0.0, 0.0, size.width, size.height);
        let projector = Projector::fit(
            options.projection,
            control_bounds,
            options.geographic_bounds,
            options.center,
        );
        let viewport = MapViewportTransform {
            zoom: options.zoom,
            pan: options.pan,
            center: Point::new(size.width * 0.5, size.height * 0.5),
        };
        let mut scene = Scene::new(size, control_bounds);
        if size.width == 0.0 || size.height == 0.0 {
            return Ok(GeoMapLayout {
                scene,
                projector,
                viewport,
            });
        }
        let palette = HeatPalette::new(&options.heat_colors, options.heat_stops.as_deref())
            .map_err(|e| ChartError::InvalidData(e.to_string()))?;
        let heat: HashMap<_, _> = data
            .heat
            .iter()
            .filter(|entry| entry.value.is_finite())
            .map(|entry| (entry.name.to_lowercase(), entry))
            .collect();
        let auto_min = heat
            .values()
            .map(|value| value.value)
            .fold(f64::INFINITY, f64::min);
        let auto_max = heat
            .values()
            .map(|value| value.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let min = options
            .min_heat
            .unwrap_or(if auto_min.is_finite() { auto_min } else { 0.0 });
        let max = options
            .max_heat
            .unwrap_or(if auto_max.is_finite() { auto_max } else { min });
        if min > max {
            return Err(ChartError::InvalidData(
                "Map heat minimum must not exceed its maximum".into(),
            ));
        }
        let base_bounds = projector.map_bounds();
        // Source map geometry is clipped to the projection's actual fitted rectangle.
        let transformed = viewport.transform(Point::new(base_bounds.x, base_bounds.y));
        let clip = intersection(
            control_bounds,
            Rect::new(
                transformed.x,
                transformed.y,
                base_bounds.width * options.zoom,
                base_bounds.height * options.zoom,
            ),
        );
        if let (Projector::Orthographic(ortho), Some(color)) = (&projector, options.globe_fill) {
            let center = viewport.transform(ortho.screen_center());
            let radius = ortho.radius() * viewport.zoom;
            scene.primitives.push(
                Primitive::new(
                    Geometry::Ellipse(Rect::new(
                        center.x - radius,
                        center.y - radius,
                        radius * 2.0,
                        radius * 2.0,
                    )),
                    PaintStyle::fill(color),
                )
                .clipped(control_bounds),
            );
        }
        let mut layer_indexes: Vec<_> = (0..data.map.layers.len()).collect();
        // Upstream processes larger indexes first; lower layers are subsequently painted on top.
        layer_indexes.sort_by_key(|i| std::cmp::Reverse(data.map.layers[*i].process_index));
        for layer_index in layer_indexes {
            let layer = &data.map.layers[layer_index];
            if !layer.is_visible {
                continue;
            }
            if !layer.stroke_width.is_finite() || layer.stroke_width < 0.0 {
                return Err(ChartError::InvalidData(
                    "Map layer stroke width must be finite and nonnegative".into(),
                ));
            }
            for (land_index, (key, land)) in layer.lands.iter().enumerate() {
                let heat = heat.get(key).or_else(|| heat.get(&land.short_name));
                let value = heat.map_or(0.0, |entry| entry.value);
                let fill = heat
                    .map(|entry| {
                        entry
                            .color
                            .unwrap_or_else(|| palette.sample(heat_progress(entry.value, min, max)))
                    })
                    .unwrap_or(layer.fill.unwrap_or(options.land_fill));
                let style = PaintStyle {
                    fill: Some(fill),
                    stroke: Some(layer.stroke.unwrap_or(options.land_stroke)),
                    stroke_width: if layer.stroke.is_some() {
                        layer.stroke_width
                    } else {
                        options.stroke_width
                    },
                    ..PaintStyle::default()
                };
                let id = PointId {
                    series: layer_index,
                    point: land_index,
                };
                let mut all_rings = Vec::new();
                let mut commands = Vec::new();
                for polygon in &land.polygons {
                    let projected = project_polygon(polygon, &projector);
                    for (ring_index, ring) in projected.into_iter().enumerate() {
                        let screen: Vec<_> = ring
                            .into_iter()
                            .map(|point| viewport.transform(point))
                            .collect();
                        let mut ring = clip_ring(&screen, clip);
                        if ring.len() < 3 {
                            continue;
                        }
                        // FerroUI's native nonzero fill rule preserves holes with opposite winding.
                        let positive = signed_area(&ring) > 0.0;
                        if positive != (ring_index == 0) {
                            ring.reverse();
                        }
                        commands.push(PathCommand::MoveTo(ring[0]));
                        commands.extend(ring.iter().skip(1).copied().map(PathCommand::LineTo));
                        commands.push(PathCommand::Close);
                        all_rings.push(ring);
                    }
                }
                if all_rings.is_empty() {
                    continue;
                }
                let center = polygon_center(&all_rings[0]);
                scene.primitives.push(
                    Primitive::new(Geometry::Path(commands), style)
                        .clipped(clip)
                        .identified(id),
                );
                scene.hits.push(HitRegion {
                    id,
                    shape: HitShape::CompoundPolygon(all_rings),
                    center,
                    value,
                    label: if heat.is_some() {
                        format!("{}: {value}", land.name)
                    } else {
                        land.name.clone()
                    },
                });
            }
        }
        let marker_series = data.map.layers.len();
        for (index, marker) in data.markers.iter().enumerate() {
            if !marker.coordinate.is_finite()
                || !marker.value.is_finite()
                || !projector.is_visible(marker.coordinate.longitude, marker.coordinate.latitude)
            {
                continue;
            }
            if !marker.radius.is_finite() || marker.radius < 0.0 {
                return Err(ChartError::InvalidData(
                    "Geographic marker radius must be finite and nonnegative".into(),
                ));
            }
            let center = viewport.transform(
                projector.to_map(marker.coordinate.longitude, marker.coordinate.latitude),
            );
            if !contains(clip, center) {
                continue;
            }
            let radius = marker.radius;
            let id = PointId {
                series: marker_series,
                point: index,
            };
            scene.primitives.push(
                Primitive::new(
                    Geometry::Ellipse(Rect::new(
                        center.x - radius,
                        center.y - radius,
                        radius * 2.0,
                        radius * 2.0,
                    )),
                    PaintStyle::fill(marker.color),
                )
                .clipped(clip)
                .identified(id),
            );
            scene.hits.push(HitRegion {
                id,
                shape: HitShape::Circle { center, radius },
                center,
                value: marker.value,
                label: marker.label.clone(),
            });
            if marker.show_label && !marker.label.is_empty() {
                scene.primitives.push(
                    Primitive::new(
                        Geometry::Text {
                            origin: Point::new(
                                center.x,
                                center.y - radius - options.marker_label_size * 1.2,
                            ),
                            text: marker.label.clone(),
                            size: options.marker_label_size,
                            rotation: 0.0,
                            anchor: TextAnchor::Center,
                        },
                        PaintStyle::fill(options.marker_label_color),
                    )
                    .clipped(control_bounds),
                );
            }
        }
        Ok(GeoMapLayout {
            scene,
            projector,
            viewport,
        })
    }
}

fn validate_options(options: &GeoMapOptions) -> Result<(), ChartError> {
    if !options.zoom.is_finite()
        || options.zoom <= 0.0
        || !options.min_zoom.is_finite()
        || options.min_zoom <= 0.0
        || !options.max_zoom.is_finite()
        || options.max_zoom < options.min_zoom
        || !options.pan.x.is_finite()
        || !options.pan.y.is_finite()
        || !options.center.is_finite()
        || !options.stroke_width.is_finite()
        || options.stroke_width < 0.0
        || !options.marker_label_size.is_finite()
        || options.marker_label_size < 0.0
        || options.min_heat.is_some_and(|v| !v.is_finite())
        || options.max_heat.is_some_and(|v| !v.is_finite())
    {
        return Err(ChartError::InvalidData("Map viewport, projection center and style values must be finite; zoom must be positive".into()));
    }
    Ok(())
}

fn heat_progress(value: f64, min: f64, max: f64) -> f64 {
    if min == max {
        return 0.0;
    }
    let range = max - min;
    if range.is_finite() {
        (value - min) / range
    } else {
        (value * 0.5 - min * 0.5) / (max * 0.5 - min * 0.5)
    }
}

fn project_polygon(polygon: &GeoPolygon, projector: &Projector) -> Vec<Vec<Point>> {
    polygon
        .rings
        .iter()
        .map(|ring| match projector {
            Projector::Orthographic(ortho) => orthographic_ring(ring, ortho),
            _ => ring
                .iter()
                .map(|point| projector.to_map(point.longitude, point.latitude))
                .filter(|point| point.x.is_finite() && point.y.is_finite())
                .collect(),
        })
        .collect()
}

/// Matches the upstream visible-disc clipping: exits and entries join along the
/// shorter horizon arc, rather than an artificial chord through the globe.
fn orthographic_ring(
    coordinates: &[GeoCoordinate],
    projector: &OrthographicProjector,
) -> Vec<Point> {
    if coordinates.len() < 3
        || !coordinates
            .iter()
            .any(|p| projector.is_visible(p.longitude, p.latitude))
    {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut pending_exit = None;
    for index in 0..coordinates.len() {
        let current = coordinates[index];
        let next = coordinates[(index + 1) % coordinates.len()];
        let current_visible = projector.is_visible(current.longitude, current.latitude);
        let next_visible = projector.is_visible(next.longitude, next.latitude);
        if current_visible {
            result.push(projector.to_map(current.longitude, current.latitude));
            if !next_visible {
                let horizon = horizon_point(projector, current, next);
                let point = projector.to_map(horizon.longitude, horizon.latitude);
                result.push(point);
                pending_exit = Some(point);
            }
        } else if next_visible {
            let horizon = horizon_point(projector, next, current);
            let entry = projector.to_map(horizon.longitude, horizon.latitude);
            if let Some(exit) = pending_exit.take() {
                append_horizon_arc(&mut result, projector, exit, entry);
            }
            result.push(entry);
        }
    }
    if let (Some(exit), Some(&first)) = (pending_exit, result.first()) {
        append_horizon_arc(&mut result, projector, exit, first);
    }
    result
}

fn horizon_point(
    projector: &OrthographicProjector,
    visible: GeoCoordinate,
    invisible: GeoCoordinate,
) -> GeoCoordinate {
    let mut a = visible;
    let mut b = invisible;
    // The upstream bisection is retained at double precision. Longitude is kept
    // on the same meridian branch so crossing the dateline takes the short edge.
    b.longitude = a.longitude + (b.longitude - a.longitude + 180.0).rem_euclid(360.0) - 180.0;
    for _ in 0..48 {
        let middle = GeoCoordinate::new(
            a.longitude * 0.5 + b.longitude * 0.5,
            a.latitude * 0.5 + b.latitude * 0.5,
        );
        if projector.is_visible(middle.longitude, middle.latitude) {
            a = middle;
        } else {
            b = middle;
        }
    }
    GeoCoordinate::new(
        a.longitude * 0.5 + b.longitude * 0.5,
        a.latitude * 0.5 + b.latitude * 0.5,
    )
}

fn append_horizon_arc(
    points: &mut Vec<Point>,
    projector: &OrthographicProjector,
    from: Point,
    to: Point,
) {
    let center = projector.screen_center();
    let start = (from.y - center.y).atan2(from.x - center.x);
    let end = (to.y - center.y).atan2(to.x - center.x);
    let delta = (end - start + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    let steps = (delta.abs() / 3.0_f64.to_radians()).ceil().max(1.0) as usize;
    for step in 1..=steps {
        let angle = start + delta * step as f64 / steps as f64;
        points.push(Point::new(
            center.x + projector.radius() * angle.cos(),
            center.y + projector.radius() * angle.sin(),
        ));
    }
}

fn intersection(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Rect::new(
        x,
        y,
        (a.x + a.width).min(b.x + b.width).max(x) - x,
        (a.y + a.height).min(b.y + b.height).max(y) - y,
    )
}
fn contains(rect: Rect, point: Point) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

fn clip_ring(ring: &[Point], rect: Rect) -> Vec<Point> {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Vec::new();
    }
    let mut output = ring.to_vec();
    // Sutherland-Hodgman retains holes and makes hit geometry match the drawing clip.
    for edge in 0..4 {
        let input = std::mem::take(&mut output);
        if input.is_empty() {
            break;
        }
        let inside = |p: Point| match edge {
            0 => p.x >= rect.x,
            1 => p.x <= rect.x + rect.width,
            2 => p.y >= rect.y,
            _ => p.y <= rect.y + rect.height,
        };
        let cut = |a: Point, b: Point| {
            if edge < 2 {
                let x = if edge == 0 {
                    rect.x
                } else {
                    rect.x + rect.width
                };
                let fraction = (x - a.x) / (b.x - a.x);
                Point::new(x, a.y + (b.y - a.y) * fraction)
            } else {
                let y = if edge == 2 {
                    rect.y
                } else {
                    rect.y + rect.height
                };
                let fraction = (y - a.y) / (b.y - a.y);
                Point::new(a.x + (b.x - a.x) * fraction, y)
            }
        };
        let mut previous = *input.last().expect("nonempty ring checked above");
        let mut previous_inside = inside(previous);
        for point in input {
            let current_inside = inside(point);
            if current_inside != previous_inside {
                output.push(cut(previous, point));
            }
            if current_inside {
                output.push(point);
            }
            previous = point;
            previous_inside = current_inside;
        }
    }
    output
}

fn signed_area(points: &[Point]) -> f64 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        area += a.x * b.y - b.x * a.y;
    }
    area * 0.5
}
fn polygon_center(points: &[Point]) -> Point {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    Point::new(min_x * 0.5 + max_x * 0.5, min_y * 0.5 + max_y * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn square_data() -> GeoData {
        GeoData::from_geojson(r#"{"features":[{"properties":{"name":"Land"},"geometry":{"type":"Polygon","coordinates":[[[-20,-20],[20,-20],[20,20],[-20,20],[-20,-20]],[[-5,-5],[5,-5],[5,5],[-5,5],[-5,-5]]]}}]}"#).unwrap()
    }
    #[test]
    fn polygon_holes_are_not_drawn_or_hit() {
        let layout = GeoMapChartEngine::layout(
            &square_data(),
            Size::new(720.0, 360.0),
            &GeoMapOptions::default(),
        )
        .unwrap();
        let hole = layout.project(GeoCoordinate::new(0.0, 0.0)).unwrap();
        let land = layout.project(GeoCoordinate::new(10.0, 10.0)).unwrap();
        assert_eq!(layout.scene.hits.len(), 1);
        assert!(!layout.scene.hits[0].contains(hole));
        assert!(layout.scene.hits[0].contains(land));
        if let Geometry::Path(commands) = &layout.scene.primitives[0].geometry {
            assert_eq!(
                commands
                    .iter()
                    .filter(|c| matches!(c, PathCommand::Close))
                    .count(),
                2
            );
        } else {
            panic!("expected compound path");
        }
    }
    #[test]
    fn zoom_keeps_pivot_coordinate_fixed_and_pan_inverts() {
        let size = Size::new(800.0, 600.0);
        let data = square_data();
        let mut options = GeoMapOptions::default();
        options.pan_by(Point::new(30.0, -20.0)).unwrap();
        let before = GeoMapChartEngine::layout(&data, size, &options).unwrap();
        let pivot = Point::new(450.0, 270.0);
        let coordinate = before.unproject(pivot).unwrap();
        options.zoom_at(pivot, 2.0, size).unwrap();
        let after = GeoMapChartEngine::layout(&data, size, &options).unwrap();
        let projected = after.project(coordinate).unwrap();
        assert!((projected.x - pivot.x).abs() < 1e-9 && (projected.y - pivot.y).abs() < 1e-9);
        let roundtrip = after.unproject(projected).unwrap();
        assert!((roundtrip.longitude - coordinate.longitude).abs() < 1e-9);
        assert!((roundtrip.latitude - coordinate.latitude).abs() < 1e-9);
    }
    #[test]
    fn orthographic_far_side_and_inverse_disc_rejection() {
        let options = GeoMapOptions {
            projection: MapProjection::Orthographic,
            ..GeoMapOptions::default()
        };
        let data=GeoData::from_geojson(r#"{"features":[{"properties":{"name":"hidden"},"geometry":{"type":"Polygon","coordinates":[[[150,-10],[170,-10],[170,10],[150,10],[150,-10]]]}}]}"#).unwrap();
        let layout = GeoMapChartEngine::layout(&data, Size::new(500.0, 500.0), &options).unwrap();
        assert!(layout.scene.hits.is_empty());
        assert!(layout.project(GeoCoordinate::new(180.0, 0.0)).is_none());
        assert!(layout.unproject(Point::new(0.0, 0.0)).is_none());
    }
    #[test]
    fn horizon_closes_along_globe_boundary_instead_of_cutting_a_chord() {
        let projector =
            OrthographicProjector::new(Rect::new(0.0, 0.0, 500.0, 500.0), GeoCoordinate::default());
        let ring = [
            GeoCoordinate::new(60.0, -40.0),
            GeoCoordinate::new(120.0, -40.0),
            GeoCoordinate::new(120.0, 40.0),
            GeoCoordinate::new(60.0, 40.0),
        ];
        let result = orthographic_ring(&ring, &projector);
        let horizon_points = result
            .iter()
            .filter(|p| ((p.x - 250.0).hypot(p.y - 250.0) - 250.0).abs() < 1e-5)
            .count();
        assert!(horizon_points > 20);
        assert!(result
            .iter()
            .all(|p| (p.x - 250.0).hypot(p.y - 250.0) <= 250.0 + 1e-9));
    }
    #[test]
    fn heat_color_and_marker_ids_are_stable_across_projection_changes() {
        let mut data = square_data();
        data.heat.push(HeatLand::new("LAND", 10.0));
        let mut marker = GeoMarker::new(GeoCoordinate::new(10.0, 10.0), "City");
        marker.show_label = true;
        data.markers.push(marker);
        let options = GeoMapOptions {
            min_heat: Some(0.0),
            max_heat: Some(10.0),
            ..GeoMapOptions::default()
        };
        let scene = GeoMapChartEngine::measure(&data, Size::new(720.0, 360.0), &options).unwrap();
        assert_eq!(scene.hits.len(), 2);
        assert_eq!(scene.hits[0].value, 10.0);
        assert_eq!(
            scene.primitives[0].style.fill,
            options.heat_colors.last().copied()
        );
        assert_eq!(
            scene.hits[1].id,
            PointId {
                series: 1,
                point: 0
            }
        );
        assert!(scene
            .primitives
            .iter()
            .any(|p| matches!(&p.geometry,Geometry::Text {text,..} if text=="City")));
    }
    #[test]
    fn world_map_builds_finite_scenes_in_all_projections() {
        let data = GeoData::world().unwrap();
        for projection in [
            MapProjection::Default,
            MapProjection::Mercator,
            MapProjection::Orthographic,
        ] {
            let options = GeoMapOptions {
                projection,
                ..GeoMapOptions::default()
            };
            let scene =
                GeoMapChartEngine::measure(&data, Size::new(900.0, 600.0), &options).unwrap();
            assert!(scene.hits.len() > 30);
            for hit in &scene.hits {
                assert!(hit.center.x.is_finite() && hit.center.y.is_finite());
            }
            for primitive in &scene.primitives {
                if let Geometry::Path(commands) = &primitive.geometry {
                    for command in commands {
                        if let PathCommand::MoveTo(point) | PathCommand::LineTo(point) = command {
                            assert!(point.x.is_finite() && point.y.is_finite());
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn viewport_constraints_and_shortest_rotation_target() {
        let mut options = GeoMapOptions {
            zoom: 2.0,
            pan: Point::new(999.0, -999.0),
            ..GeoMapOptions::default()
        };
        options.constrain_viewport(Size::new(400.0, 300.0)).unwrap();
        assert_eq!(options.pan, Point::new(200.0, -150.0));
        options.reset_viewport();
        assert_eq!(options.zoom, 1.0);
        assert_eq!(options.pan, Point::new(0.0, 0.0));
        assert_eq!(
            shortest_rotation_target(
                GeoCoordinate::new(170.0, 0.0),
                GeoCoordinate::new(-170.0, 15.0)
            ),
            GeoCoordinate::new(190.0, 15.0)
        );
    }
}
