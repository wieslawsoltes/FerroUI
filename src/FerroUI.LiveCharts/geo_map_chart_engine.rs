//! Geographic map measurement adapted from the pinned `GeoMapChart`, `MapFactory`,
//! `CoreHeatLandSeries`, `LandAreaGeometry` and `MapViewportTransform`: the projection fitted
//! into the draw margin and clipped to it, the lands of the visible layers by descending process
//! index (the chart's fill, then its stroke over every land), the heat colour of the lands of the
//! heat land series, the orthographic globe with its horizon walked along the rim of the disc,
//! the pan and zoom viewport over the projected map, hover areas with the source's tooltip
//! values and anchor, and markers at geographic coordinates. Pure calculation produces a retained
//! scene that the native controls draw like the other chart scenes.

use crate::geo::{
    DrawnMap, GeographicBounds, LandDefinition, MapProjection, MapProjector, Maps,
    OrthographicProjector,
};
use crate::heat_functions::HeatFunctions;
use crate::measure::Bounds;
use crate::scene::*;
use crate::series::{Series, SeriesKind};
use ferroui_base::{media::Color, Point, Rect, Size};
use std::collections::HashMap;
use std::f64::consts::{PI, TAU};

/// The chart's default land stroke (`Stroke` of the source map view).
pub const MAP_STROKE: Color = Color::from_rgb(255, 255, 255);
/// The chart's default land fill, also the background of the orthographic globe (`Fill`).
pub const MAP_FILL: Color = Color::from_rgb(240, 240, 240);
/// The heat map of a heat land series of the source (`HeatLandSeries`).
pub const HEAT_LAND_MAP: [Color; 2] =
    [Color::from_rgb(179, 229, 252), Color::from_rgb(2, 136, 209)];

/// A weighted land of a heat land series (`HeatLand`, `IWeigthedMapLand`): the key of a land of
/// the map (its short name) and its value.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatLand {
    pub name: String,
    pub value: f64,
}
impl HeatLand {
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// A heat land series (`CoreHeatLandSeries`): the name, visibility and heat map of its
/// [`Series`] (the source's two blues by default), its lands, the offsets of the heat map and
/// the bounds that override the weights of the lands.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatLandSeries {
    pub series: Series,
    pub lands: Vec<HeatLand>,
    pub color_stops: Option<Vec<f64>>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}
impl Default for HeatLandSeries {
    fn default() -> Self {
        Self {
            series: Series::new(SeriesKind::HeatLand),
            lands: Vec::new(),
            color_stops: None,
            min_value: None,
            max_value: None,
        }
    }
}
impl HeatLandSeries {
    pub fn new(lands: Vec<HeatLand>) -> Self {
        Self {
            lands,
            ..Self::default()
        }
    }

    /// The bounds of the weights (`WeightBounds`): the values of the lands, with `min_value` and
    /// `max_value` in place of their extremes when they are set.
    pub fn weight_bounds(&self) -> Bounds {
        let mut bounds = Bounds::default();
        for land in &self.lands {
            bounds.append_value(land.value);
        }
        if let Some(min) = self.min_value {
            bounds.min = min;
        }
        if let Some(max) = self.max_value {
            bounds.max = max;
        }
        bounds
    }

    /// The value of the land of the given short name, ignoring case (`TryGetValue`).
    pub fn try_get_value(&self, land_short_name: &str) -> Option<f64> {
        let wanted = land_short_name.to_lowercase();
        self.lands
            .iter()
            .find(|land| land.name.to_lowercase() == wanted)
            .map(|land| land.value)
    }
}

/// A circle at a geographic coordinate, drawn over the map: the native counterpart of a
/// `GeoVisualElement` around a `GeometryVisual<CircleGeometry>` (the markers of the source's
/// snapshot suite). It is centred on its coordinate, follows pan, zoom and rotation at a constant
/// size, and is hidden on the far side of a globe.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoMarker {
    pub longitude: f64,
    pub latitude: f64,
    /// The diameter.
    pub size: f64,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f64,
}
impl Default for GeoMarker {
    /// The city markers of the source's `MapsTests`: 14 pixels of orange red with a white
    /// outline of 2.
    fn default() -> Self {
        Self {
            longitude: 0.0,
            latitude: 0.0,
            size: 14.0,
            fill: Some(Color::from_rgb(255, 87, 51)),
            stroke: Some(Color::from_rgb(255, 255, 255)),
            stroke_width: 2.0,
        }
    }
}
impl GeoMarker {
    pub fn new(longitude: f64, latitude: f64) -> Self {
        Self {
            longitude,
            latitude,
            ..Self::default()
        }
    }
}

/// The direction of a zoom step (`ZoomDirection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZoomDirection {
    ZoomIn,
    ZoomOut,
}

/// The pan and zoom of a map (`MapViewportTransform`, `GeoMapChart.ZoomLevel` and `PanOffset`):
/// a projected position `p` is drawn at `p * zoom + center * (1 - zoom) + pan`, where `center`
/// is the centre of the control. The stroke of the lands scales with the zoom, as the source's
/// canvas matrix scales it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapViewport {
    pub zoom: f64,
    pub pan: Point,
}
impl Default for MapViewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Point::new(0.0, 0.0),
        }
    }
}
impl MapViewport {
    pub fn new(zoom: f64, pan: Point) -> Self {
        Self { zoom, pan }
    }

    /// Whether the viewport changes nothing (the opposite of `IsActive`).
    pub fn is_identity(&self) -> bool {
        const EPSILON: f64 = 1e-5;
        (self.zoom - 1.0).abs() < EPSILON
            && self.pan.x.abs() < EPSILON
            && self.pan.y.abs() < EPSILON
    }

    /// The screen position of a projected position.
    pub fn transform(&self, point: Point, center: Point) -> Point {
        Point::new(
            point.x * self.zoom + center.x * (1.0 - self.zoom) + self.pan.x,
            point.y * self.zoom + center.y * (1.0 - self.zoom) + self.pan.y,
        )
    }

    /// The projected position of a screen position (`InverseViewportTransform`); a zoom of 0 is
    /// taken as 1.
    pub fn inverse(&self, point: Point, center: Point) -> Point {
        let zoom = if self.zoom == 0.0 { 1.0 } else { self.zoom };
        Point::new(
            (point.x - center.x * (1.0 - self.zoom) - self.pan.x) / zoom,
            (point.y - center.y * (1.0 - self.zoom) - self.pan.y) / zoom,
        )
    }

    /// The viewport moved by a screen distance (`MapFactory.Pan`).
    pub fn panned(&self, delta: Point) -> Self {
        Self::new(
            self.zoom,
            Point::new(self.pan.x + delta.x, self.pan.y + delta.y),
        )
    }

    /// One zoom step about a screen position, which stays in place (`MapFactory.Zoom`). The
    /// speed is clamped to [0.1, 0.95]; a step in multiplies the zoom by `1 / (1 - speed)`, a step
    /// out by `1 - speed`. The zoom stays at most `max_zoom` and at least `0.8 * min_zoom`, the
    /// overshoot that the bounce back of a release takes back.
    pub fn zoomed(
        &self,
        pivot: Point,
        direction: ZoomDirection,
        speed: f64,
        min_zoom: f64,
        max_zoom: f64,
        center: Point,
    ) -> Self {
        let speed = 1.0 - if speed.is_nan() { 0.1 } else { speed.clamp(0.1, 0.95) };
        let old_zoom = if self.zoom > 0.0 { self.zoom } else { 1.0 };
        let mut new_zoom = match direction {
            ZoomDirection::ZoomIn => old_zoom / speed,
            ZoomDirection::ZoomOut => old_zoom * speed,
        };
        if new_zoom < min_zoom * 0.8 {
            new_zoom = min_zoom * 0.8;
        }
        if new_zoom > max_zoom {
            new_zoom = max_zoom;
        }
        let tx_old = center.x * (1.0 - old_zoom) + self.pan.x;
        let ty_old = center.y * (1.0 - old_zoom) + self.pan.y;
        let tx_new = pivot.x - (pivot.x - tx_old) * new_zoom / old_zoom;
        let ty_new = pivot.y - (pivot.y - ty_old) * new_zoom / old_zoom;
        Self::new(
            new_zoom,
            Point::new(
                tx_new - center.x * (1.0 - new_zoom),
                ty_new - center.y * (1.0 - new_zoom),
            ),
        )
    }

    /// Where a release settles the viewport (`BounceBack`): the zoom back to at least
    /// `min_zoom` and the map, the size of the control at that zoom, panned to cover the control.
    /// `None` when the viewport is already there.
    pub fn bounce_back_target(&self, control: Size, min_zoom: f64) -> Option<Self> {
        if !(control.width > 0.0) || !(control.height > 0.0) {
            return None;
        }
        let (cx, cy) = (control.width * 0.5, control.height * 0.5);
        let zoom = self.zoom;
        let target_zoom = if zoom < min_zoom { min_zoom } else { zoom };
        let map_w = control.width * target_zoom;
        let map_h = control.height * target_zoom;
        let mut target = self.pan;
        if (target_zoom - zoom).abs() > 1e-6 && zoom != 0.0 {
            target = Point::new(
                self.pan.x * target_zoom / zoom,
                self.pan.y * target_zoom / zoom,
            );
        }
        let mut tx = cx * (1.0 - target_zoom) + target.x;
        let mut ty = cy * (1.0 - target_zoom) + target.y;
        if tx > 0.0 {
            tx = 0.0;
        }
        if tx + map_w < control.width {
            tx = control.width - map_w;
        }
        if ty > 0.0 {
            ty = 0.0;
        }
        if ty + map_h < control.height {
            ty = control.height - map_h;
        }
        let target = Point::new(
            tx - cx * (1.0 - target_zoom),
            ty - cy * (1.0 - target_zoom),
        );
        if (target.x - self.pan.x).abs() < 0.5
            && (target.y - self.pan.y).abs() < 0.5
            && (target_zoom - zoom).abs() < 0.001
        {
            return None;
        }
        Some(Self::new(target_zoom, target))
    }

    /// The viewport `t` of the way from this one to `to`.
    pub fn interpolate(&self, to: &Self, t: f64) -> Self {
        Self::new(
            self.zoom + (to.zoom - self.zoom) * t,
            Point::new(
                self.pan.x + (to.pan.x - self.pan.x) * t,
                self.pan.y + (to.pan.y - self.pan.y) * t,
            ),
        )
    }
}

/// The longitude to rotate a globe to from `from`, the short way round (`RotateTo`): a
/// difference of more than 180 degrees goes the other way.
pub fn shortest_rotation(from: f64, to: f64) -> f64 {
    let delta = to - from;
    if delta > 180.0 {
        from + (delta - 360.0)
    } else if delta < -180.0 {
        from + (delta + 360.0)
    } else {
        to
    }
}

/// The settings of a map chart (the projection, bounds and paints of `IGeoMapView`, and the
/// centre and viewport of `GeoMapChart`).
#[derive(Clone, Debug, PartialEq)]
pub struct GeoMapOptions {
    /// An explicit draw margin; `None` takes the whole control.
    pub draw_margin: Option<Rect>,
    pub projection: MapProjection,
    pub bounds: GeographicBounds,
    /// The longitude and latitude an orthographic globe faces (`CenterLongitude`,
    /// `CenterLatitude`).
    pub center_longitude: f64,
    pub center_latitude: f64,
    pub viewport: MapViewport,
    /// The stroke of every land; `None` takes the stroke of its layer.
    pub stroke: Option<Color>,
    /// The fill of a land without heat and of the globe; `None` takes the fill of its layer.
    pub fill: Option<Color>,
    pub stroke_width: f64,
}
impl Default for GeoMapOptions {
    fn default() -> Self {
        Self {
            draw_margin: None,
            projection: MapProjection::Default,
            bounds: GeographicBounds::default(),
            center_longitude: 0.0,
            center_latitude: 0.0,
            viewport: MapViewport::default(),
            stroke: Some(MAP_STROKE),
            fill: Some(MAP_FILL),
            stroke_width: 1.0,
        }
    }
}

/// The value a series has for a land (`GeoTooltipValue`).
#[derive(Clone, Debug, PartialEq)]
pub struct GeoTooltipValue {
    /// The index of the series.
    pub series: usize,
    pub series_name: String,
    pub value: f64,
}

/// A land as hovered or clicked (`GeoTooltipPoint`, and the result of `FindLandAt`): its
/// identity in the scene, its names, the values of the series that have one in the order of the
/// series, the anchor of its tooltip (the middle of its largest visible ring) and its screen
/// bounds, the hover area of its chart point.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoLand {
    /// The layer (in the order of the map) and the land (in the order of its layer).
    pub id: PointId,
    pub short_name: String,
    pub name: String,
    pub values: Vec<GeoTooltipValue>,
    pub center: Point,
    pub bounds: Rect,
}
impl GeoLand {
    /// The value of the first series that has one, or 0 (`GeoTooltipPoint.Value`).
    pub fn value(&self) -> f64 {
        self.values.first().map_or(0.0, |value| value.value)
    }
}

/// A measured map: the scene, the projector and viewport it was drawn with, and its lands.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoMapMeasurement {
    pub scene: Scene,
    pub projector: MapProjector,
    pub viewport: MapViewport,
    /// The centre of the control, about which the viewport zooms.
    pub viewport_center: Point,
    pub lands: Vec<GeoLand>,
}
impl GeoMapMeasurement {
    /// The screen position of a coordinate (`GeoMapChart.Project`), `None` on the far side of a
    /// globe.
    pub fn project(&self, longitude: f64, latitude: f64) -> Option<Point> {
        if !longitude.is_finite()
            || !latitude.is_finite()
            || !self.projector.is_visible(longitude, latitude)
        {
            return None;
        }
        let point = self.viewport.transform(
            self.projector.to_map(longitude, latitude),
            self.viewport_center,
        );
        (point.x.is_finite() && point.y.is_finite()).then_some(point)
    }

    /// The coordinate of a screen position (`GeoMapChart.Unproject`), `None` outside the disc of
    /// a globe.
    pub fn unproject(&self, point: Point) -> Option<(f64, f64)> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        self.projector
            .to_coordinates(self.viewport.inverse(point, self.viewport_center))
    }

    /// The land of a hit region.
    pub fn land(&self, id: PointId) -> Option<&GeoLand> {
        self.lands.iter().find(|land| land.id == id)
    }

    /// The first land whose shape contains the screen position (`FindLandAt`).
    pub fn find_land_at(&self, point: Point) -> Option<&GeoLand> {
        let hit = self
            .scene
            .hit_test(point, FindingStrategy::ExactMatch, 0.0)
            .into_iter()
            .next()?;
        self.land(hit.id)
    }
}

pub struct GeoMapChartEngine;

impl GeoMapChartEngine {
    /// Measures a map of the given size. The projection is fitted into the draw margin (the
    /// whole control without one) and the lands are clipped to its area, which is the plot of
    /// the scene; a collapsed draw margin hides the map. A heat map of fewer than two colours, or
    /// colour stops of another count, is an error.
    pub fn measure(
        size: Size,
        map: &DrawnMap,
        series: &[HeatLandSeries],
        markers: &[GeoMarker],
        options: &GeoMapOptions,
    ) -> Result<GeoMapMeasurement, ChartError> {
        validate_size(size)?;
        validate_options(options)?;
        let plot = match options.draw_margin {
            Some(margin) => {
                if [margin.x, margin.y, margin.width, margin.height]
                    .iter()
                    .any(|v| !v.is_finite())
                {
                    return Err(ChartError::InvalidData("invalid map draw margin".into()));
                }
                Rect::new(
                    margin.x,
                    margin.y,
                    margin.width.max(0.0),
                    margin.height.max(0.0),
                )
            }
            None => Rect::new(0.0, 0.0, size.width, size.height),
        };
        let viewport = options.viewport;
        let viewport_center = Point::new(size.width * 0.5, size.height * 0.5);
        let center = (options.center_longitude, options.center_latitude);
        // A map too small to draw hides its plot (HidePlotZones); its points still project with
        // a projector of the whole control, as the source's GetOrBuildProjector before a measure.
        if plot.width <= 0.0 || plot.height <= 0.0 {
            return Ok(GeoMapMeasurement {
                scene: Scene::new(size, plot),
                projector: Maps::build_projector(
                    options.projection,
                    [size.width, size.height],
                    [0.0, 0.0],
                    center,
                    options.bounds,
                ),
                viewport,
                viewport_center,
                lands: Vec::new(),
            });
        }
        let projector = Maps::build_projector(
            options.projection,
            [plot.width, plot.height],
            [plot.x, plot.y],
            center,
            options.bounds,
        );
        let clip = projector.map_rect();
        let mut scene = Scene::new(size, clip);
        scene.automatic_finding_strategy = FindingStrategy::ExactMatchTakeClosest;
        let visible: Vec<_> = series
            .iter()
            .enumerate()
            .filter(|(_, s)| s.series.visible)
            .collect();
        let heat = heat_colors(map, &visible)?;
        let to_screen = |point: Point| viewport.transform(point, viewport_center);
        let ortho = projector.as_orthographic();
        // The globe behind an orthographic map, in the chart's fill.
        if let (Some(globe), Some(fill)) = (ortho, options.fill) {
            let middle = to_screen(Point::new(globe.screen_center_x(), globe.screen_center_y()));
            let radius = globe.radius() * viewport.zoom;
            scene.primitives.push(
                Primitive::new(
                    Geometry::Ellipse(Rect::new(
                        middle.x - radius,
                        middle.y - radius,
                        radius * 2.0,
                        radius * 2.0,
                    )),
                    PaintStyle::fill(fill),
                )
                .clipped(clip),
            );
        }
        let stroke_width = options.stroke_width * viewport.zoom;
        let mut layer_order: Vec<_> = (0..map.layers().len())
            .filter(|&i| map.layers()[i].is_visible)
            .collect();
        layer_order.sort_by_key(|&i| std::cmp::Reverse(map.layers()[i].process_index));
        let mut strokes = Vec::new();
        let mut lands = Vec::new();
        for layer_index in layer_order {
            let layer = &map.layers()[layer_index];
            let stroke = options.stroke.or(layer.stroke);
            let fill = options.fill.or(layer.fill);
            // Only the lands of the default layer take heat (FindLand).
            let heated = layer.name == DrawnMap::DEFAULT_LAYER;
            let mut outline = Vec::new();
            for (land_index, (key, land)) in layer.lands().enumerate() {
                let id = PointId {
                    series: layer_index,
                    point: land_index,
                };
                let land_fill = heated
                    .then(|| heat.get(key).copied())
                    .flatten()
                    .or(fill);
                let mut rings = Vec::new();
                for data in &land.data {
                    let projected = match ortho {
                        Some(globe) => match orthographic_ring(globe, &data.coordinates) {
                            Some(ring) => ring,
                            None => continue,
                        },
                        None => data
                            .coordinates
                            .iter()
                            .map(|c| projector.to_map(c.x, c.y))
                            .collect(),
                    };
                    let ring: Vec<_> = projected.into_iter().map(to_screen).collect();
                    let Some(&first) = ring.first() else {
                        continue;
                    };
                    if let Some(color) = land_fill {
                        scene.primitives.push(
                            Primitive::new(Geometry::Polygon(ring.clone()), PaintStyle::fill(color))
                                .clipped(clip)
                                .identified(id),
                        );
                    }
                    outline.push(PathCommand::MoveTo(first));
                    outline.extend(ring[1..].iter().copied().map(PathCommand::LineTo));
                    if ortho.is_some() {
                        outline.push(PathCommand::Close);
                    }
                    rings.push(HitShape::Polygon(ring));
                }
                if rings.is_empty() {
                    continue;
                }
                let bounds = land_screen_bounds(land, &projector, to_screen);
                let center = land_screen_center(land, &projector, to_screen).unwrap_or(Point::new(
                    bounds.x + bounds.width * 0.5,
                    bounds.y + bounds.height * 0.5,
                ));
                let values: Vec<_> = visible
                    .iter()
                    .filter_map(|&(si, s)| {
                        s.try_get_value(&land.short_name)
                            .map(|value| GeoTooltipValue {
                                series: si,
                                series_name: s.series.name.clone(),
                                value,
                            })
                    })
                    .collect();
                let geo_land = GeoLand {
                    id,
                    short_name: land.short_name.clone(),
                    name: land.name.clone(),
                    values,
                    center,
                    bounds,
                };
                scene.hits.push(HitRegion {
                    id,
                    shape: HitShape::WithHoverArea {
                        shape: Box::new(HitShape::Compound(rings)),
                        bounds,
                    },
                    center,
                    value: geo_land.value(),
                    label: tooltip_label(&geo_land),
                });
                lands.push(geo_land);
            }
            if let (Some(color), false) = (stroke, outline.is_empty()) {
                strokes.push(
                    Primitive::new(
                        Geometry::Path(outline),
                        PaintStyle::stroke(color, stroke_width),
                    )
                    .clipped(clip),
                );
            }
        }
        // The fills are drawn before the stroke paint (GeoMapStrokeZIndex).
        scene.primitives.extend(strokes);
        // The markers are visual elements, drawn above the map and not clipped to it.
        for marker in markers {
            if !marker.size.is_finite() || marker.size <= 0.0 || !marker.stroke_width.is_finite()
            {
                continue;
            }
            let Some(at) = project(&projector, viewport, viewport_center, marker) else {
                continue;
            };
            let half = marker.size * 0.5;
            scene.primitives.push(Primitive::new(
                Geometry::Ellipse(Rect::new(at.x - half, at.y - half, marker.size, marker.size)),
                PaintStyle {
                    fill: marker.fill,
                    stroke: marker.stroke,
                    stroke_width: marker.stroke_width.max(0.0),
                    ..PaintStyle::default()
                },
            ));
        }
        for &(si, s) in &visible {
            scene.legend.push(LegendItem {
                series: si,
                name: s.series.name.clone(),
                color: s
                    .series
                    .heat_map
                    .last()
                    .copied()
                    .unwrap_or_else(|| palette_color(si)),
            });
        }
        Ok(GeoMapMeasurement {
            scene,
            projector,
            viewport,
            viewport_center,
            lands,
        })
    }
}

fn validate_options(options: &GeoMapOptions) -> Result<(), ChartError> {
    let viewport = options.viewport;
    if !options.center_longitude.is_finite()
        || !options.center_latitude.is_finite()
        || !viewport.zoom.is_finite()
        || viewport.zoom <= 0.0
        || !viewport.pan.x.is_finite()
        || !viewport.pan.y.is_finite()
        || !options.stroke_width.is_finite()
        || options.stroke_width < 0.0
    {
        return Err(ChartError::InvalidData(
            "the map centre, zoom, pan and stroke width must be finite; the zoom positive".into(),
        ));
    }
    Ok(())
}

/// The heat colour of each land of the default layer that a visible series weighs
/// (`CoreHeatLandSeries.Measure`); a later series paints over an earlier one. A land without a
/// finite value is not painted.
fn heat_colors(
    map: &DrawnMap,
    series: &[(usize, &HeatLandSeries)],
) -> Result<HashMap<String, Color>, ChartError> {
    let mut colors = HashMap::new();
    for &(_, s) in series {
        let heat_map = &s.series.heat_map;
        let stops = HeatFunctions::build_color_stops(heat_map, s.color_stops.as_deref())?;
        let bounds = s.weight_bounds();
        for land in &s.lands {
            if !land.value.is_finite() || map.find_land(&land.name).is_none() {
                continue;
            }
            let color = HeatFunctions::interpolate_color(land.value, &bounds, heat_map, &stops);
            colors.insert(land.name.clone(), color);
        }
    }
    Ok(colors)
}

fn project(
    projector: &MapProjector,
    viewport: MapViewport,
    center: Point,
    marker: &GeoMarker,
) -> Option<Point> {
    let (longitude, latitude) = (marker.longitude, marker.latitude);
    if !longitude.is_finite() || !latitude.is_finite() || !projector.is_visible(longitude, latitude)
    {
        return None;
    }
    let point = viewport.transform(projector.to_map(longitude, latitude), center);
    (point.x.is_finite() && point.y.is_finite()).then_some(point)
}

/// The screen bounds of the visible coordinates of every ring of a land
/// (`ComputeLandScreenBounds`), or an empty rectangle at the origin without one.
fn land_screen_bounds(
    land: &LandDefinition,
    projector: &MapProjector,
    to_screen: impl Fn(Point) -> Point,
) -> Rect {
    let points = land
        .data
        .iter()
        .flat_map(|data| data.coordinates.iter())
        .filter(|c| projector.is_visible(c.x, c.y))
        .map(|c| projector.to_map(c.x, c.y));
    match extent(points) {
        Some((min, max)) => {
            let (a, b) = (to_screen(min), to_screen(max));
            Rect::new(a.x, a.y, b.x - a.x, b.y - a.y)
        }
        None => Rect::new(0.0, 0.0, 0.0, 0.0),
    }
}

/// The tooltip anchor of a land (`ComputeLandScreenCenter`): the middle of the bounds of its
/// visible ring with the largest bounds, so that a land across the antimeridian is anchored on
/// its mainland, not between its parts.
fn land_screen_center(
    land: &LandDefinition,
    projector: &MapProjector,
    to_screen: impl Fn(Point) -> Point,
) -> Option<Point> {
    let mut best: Option<(f64, Point, Point)> = None;
    for data in &land.data {
        let points = data
            .coordinates
            .iter()
            .filter(|c| projector.is_visible(c.x, c.y))
            .map(|c| projector.to_map(c.x, c.y));
        let Some((min, max)) = extent(points) else {
            continue;
        };
        let area = (max.x - min.x) * (max.y - min.y);
        if best.is_none_or(|(best_area, _, _)| area > best_area) {
            best = Some((area, min, max));
        }
    }
    let (_, min, max) = best?;
    Some(to_screen(Point::new(
        (min.x + max.x) / 2.0,
        (min.y + max.y) / 2.0,
    )))
}

fn extent(points: impl Iterator<Item = Point>) -> Option<(Point, Point)> {
    let mut result: Option<(Point, Point)> = None;
    for point in points {
        let (min, max) = result.get_or_insert((point, point));
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    result
}

/// The text of the source's default geo tooltip (`SKDefaultGeoTooltip`): the name of the land in
/// title case, then a line for each series value, `{name}: {value:N2}` (the value alone for a
/// series without a name).
pub fn tooltip_label(land: &GeoLand) -> String {
    let mut label = title_case(&land.name);
    for value in &land.values {
        label.push('\n');
        if !value.series_name.is_empty() {
            label.push_str(&value.series_name);
            label.push_str(": ");
        }
        label.push_str(&format_n2(value.value));
    }
    label
}

/// `TextInfo.ToTitleCase` of the invariant culture for lower-case text: the first letter of every
/// word in upper case.
fn title_case(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_word = false;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            if in_word {
                result.push(ch);
            } else {
                result.extend(ch.to_uppercase());
            }
            in_word = true;
        } else {
            result.push(ch);
            in_word = ch.is_numeric() || ch == '\'';
        }
    }
    result
}

/// The `N2` format of the invariant culture: two decimals and grouped thousands.
fn format_n2(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let text = format!("{:.2}", value.abs());
    let (whole, fraction) = text.split_once('.').unwrap_or((text.as_str(), "00"));
    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    let sign = if value < 0.0 { "-" } else { "" };
    format!("{sign}{grouped}.{fraction}")
}

/// The ring of a land on a globe (`MapFactory.BuildOrthographicPath`), `None` when it is all on
/// the far side. The ring follows its visible coordinates; where it crosses the horizon the
/// crossing is found by bisection, and the ring walks the rim of the disc from where it leaves
/// the visible side to where it comes back, instead of a chord through the disc.
pub(crate) fn orthographic_ring(
    globe: &OrthographicProjector,
    coordinates: &[Point],
) -> Option<Vec<Point>> {
    if !coordinates.iter().any(|c| globe.is_visible(c.x, c.y)) {
        return None;
    }
    let mut path = Vec::with_capacity(coordinates.len());
    let mut first = None;
    let mut pending_exit = None;
    for (i, &current) in coordinates.iter().enumerate() {
        let next = coordinates[(i + 1) % coordinates.len()];
        let current_visible = globe.is_visible(current.x, current.y);
        let next_visible = globe.is_visible(next.x, next.y);
        if current_visible {
            let point = globe.to_map(current.x, current.y);
            first.get_or_insert(point);
            path.push(point);
            if !next_visible {
                // Visible to hidden: the ring leaves at the horizon.
                let horizon = horizon_point(globe, current, next);
                let exit = globe.to_map(horizon.x, horizon.y);
                path.push(exit);
                pending_exit = Some(exit);
            }
        } else if next_visible {
            // Hidden to visible: the ring comes back at the horizon.
            let horizon = horizon_point(globe, next, current);
            let entry = globe.to_map(horizon.x, horizon.y);
            if let Some(exit) = pending_exit.take() {
                horizon_arc(globe, &mut path, exit, entry);
            }
            first.get_or_insert(entry);
            path.push(entry);
        }
    }
    if let (Some(exit), Some(first)) = (pending_exit, first) {
        // The ring ends hidden: it closes along the rim back to where it started.
        horizon_arc(globe, &mut path, exit, first);
    }
    first.map(|_| path)
}

/// The horizon between a visible and a hidden coordinate (`FindHorizonPoint`): 15 bisections of
/// the straight line between them in degrees.
fn horizon_point(globe: &OrthographicProjector, visible: Point, hidden: Point) -> Point {
    let (mut a, mut b) = (visible, hidden);
    for _ in 0..15 {
        let middle = Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
        if globe.is_visible(middle.x, middle.y) {
            a = middle;
        } else {
            b = middle;
        }
    }
    Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

/// The rim of the disc from one screen position to another, the short way round, in steps of at
/// most 3 degrees (`EmitHorizonArc`); the start is not repeated.
fn horizon_arc(globe: &OrthographicProjector, path: &mut Vec<Point>, from: Point, to: Point) {
    let (cx, cy, r) = (globe.screen_center_x(), globe.screen_center_y(), globe.radius());
    let a0 = (from.y - cy).atan2(from.x - cx);
    let a1 = (to.y - cy).atan2(to.x - cx);
    let mut delta = a1 - a0;
    if delta > PI {
        delta -= TAU;
    } else if delta < -PI {
        delta += TAU;
    }
    let steps = ((delta.abs() / 3.0_f64.to_radians()).ceil() as usize).max(1);
    let step = delta / steps as f64;
    for i in 1..=steps {
        let angle = a0 + step * i as f64;
        path.push(Point::new(cx + r * angle.cos(), cy + r * angle.sin()));
    }
}

#[cfg(test)]
#[path = "geo_map_chart_engine_tests.rs"]
mod tests;
