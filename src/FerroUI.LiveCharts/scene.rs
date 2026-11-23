//! Backend-independent measured chart drawing and point interaction.

use crate::paint::BrushSpace;
use ferroui_base::{
    Point, Rect, Size,
    media::{Color, IBrush, PenLineCap, PenLineJoin},
};
use std::fmt;
use std::rc::Rc;

/// Invalid chart data is reported before drawing, never as a rendering panic.
#[derive(Clone, Debug, PartialEq)]
pub enum ChartError {
    InvalidSize,
    InvalidAxis(String),
    InvalidData(String),
}
impl fmt::Display for ChartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize => f.write_str("chart size must be finite and nonnegative"),
            Self::InvalidAxis(value) | Self::InvalidData(value) => f.write_str(value),
        }
    }
}
impl std::error::Error for ChartError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PointId {
    pub series: usize,
    pub point: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaintStyle {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    /// Native brushes override the corresponding color convenience value.
    pub fill_brush: Option<Rc<dyn IBrush>>,
    pub stroke_brush: Option<Rc<dyn IBrush>>,
    pub brush_space: BrushSpace,
    pub stroke_width: f64,
    pub stroke_cap: PenLineCap,
    pub stroke_join: PenLineJoin,
    pub stroke_miter: f64,
    pub opacity: f64,
    /// Absolute device-independent lengths, matching source DashEffect.
    pub dashes: Vec<f64>,
    pub dash_offset: f64,
}
impl Default for PaintStyle {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            fill_brush: None,
            stroke_brush: None,
            brush_space: BrushSpace::Clip,
            stroke_width: 1.0,
            stroke_cap: PenLineCap::Flat,
            stroke_join: PenLineJoin::Miter,
            stroke_miter: 0.0,
            opacity: 1.0,
            dashes: Vec::new(),
            dash_offset: 0.0,
        }
    }
}
impl PaintStyle {
    pub fn fill(color: Color) -> Self {
        Self {
            fill: Some(color),
            ..Self::default()
        }
    }
    pub fn stroke(color: Color, width: f64) -> Self {
        Self {
            stroke: Some(color),
            stroke_width: width,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    CubicTo {
        control1: Point,
        control2: Point,
        end: Point,
    },
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Line {
        start: Point,
        end: Point,
    },
    Polyline(Vec<Point>),
    Polygon(Vec<Point>),
    Rectangle {
        rect: Rect,
        radius: f64,
    },
    Ellipse(Rect),
    Path(Vec<PathCommand>),
    Text {
        origin: Point,
        text: String,
        size: f64,
        rotation: f64,
        anchor: TextAnchor,
    },
}

/// Horizontal anchoring is resolved using the host text shaper's measured width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextAnchor {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Primitive {
    pub geometry: Geometry,
    pub style: PaintStyle,
    pub clip: Option<Rect>,
    pub id: Option<PointId>,
    /// Stable host entity identity. This survives index changes and distinguishes
    /// an exiting point from its replacement at the same event-facing PointId.
    pub entity_key: Option<(u64, u64)>,
}
impl Primitive {
    pub fn new(geometry: Geometry, style: PaintStyle) -> Self {
        Self {
            geometry,
            style,
            clip: None,
            id: None,
            entity_key: None,
        }
    }
    pub fn clipped(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }
    pub fn identified(mut self, id: PointId) -> Self {
        self.id = Some(id);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum HitShape {
    Rectangle(Rect),
    Circle {
        center: Point,
        radius: f64,
    },
    AnnularSector {
        center: Point,
        inner_radius: f64,
        outer_radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    },
    Polygon(Vec<Point>),
    CompoundPolygon(Vec<Vec<Point>>),
    /// Exact geometry may differ from the category slot used for hover discovery.
    /// This is how a thin column and both ends of a range remain discoverable.
    WithHoverArea {
        shape: Box<HitShape>,
        bounds: Rect,
    },
    Compound(Vec<HitShape>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct HitRegion {
    pub id: PointId,
    pub shape: HitShape,
    pub center: Point,
    pub value: f64,
    pub label: String,
}
impl HitRegion {
    pub fn contains(&self, point: Point) -> bool {
        self.shape.contains(point)
    }
}

impl HitShape {
    pub fn contains(&self, point: Point) -> bool {
        match self {
            // `RectangleHoverArea.IsPointerOver`: the edges are outside, every side at least 1 px.
            HitShape::Rectangle(r) => {
                hover_span(r.x, r.width, point.x, 0.0) && hover_span(r.y, r.height, point.y, 0.0)
            }
            HitShape::Circle { center, radius } => {
                (point.x - center.x).hypot(point.y - center.y) <= *radius
            }
            HitShape::AnnularSector {
                center,
                inner_radius,
                outer_radius,
                start_angle,
                sweep_angle,
            } => {
                let dx = point.x - center.x;
                let dy = point.y - center.y;
                let r = dx.hypot(dy);
                if r < *inner_radius || r > *outer_radius {
                    return false;
                }
                if sweep_angle.abs() >= 360.0 {
                    return true;
                }
                let a = dy.atan2(dx).to_degrees();
                let distance = if *sweep_angle >= 0.0 {
                    (a - start_angle).rem_euclid(360.0)
                } else {
                    (start_angle - a).rem_euclid(360.0)
                };
                distance <= sweep_angle.abs()
            }
            HitShape::CompoundPolygon(rings) => {
                rings
                    .iter()
                    .filter(|points| polygon_contains(points, point))
                    .count()
                    % 2
                    == 1
            }
            HitShape::Polygon(points) => polygon_contains(points, point),
            HitShape::WithHoverArea { shape, .. } => shape.contains(point),
            HitShape::Compound(shapes) => shapes.iter().any(|shape| shape.contains(point)),
        }
    }

    /// Axis-aligned hover extent, in control-local units. These bounds may be
    /// intentionally wider than the painted geometry, as in the upstream bar series.
    pub fn hover_bounds(&self) -> Rect {
        match self {
            Self::Rectangle(rect) | Self::WithHoverArea { bounds: rect, .. } => Rect::new(
                rect.x.min(rect.x + rect.width),
                rect.y.min(rect.y + rect.height),
                rect.width.abs(),
                rect.height.abs(),
            ),
            Self::Circle { center, radius } => Rect::new(
                center.x - radius,
                center.y - radius,
                radius * 2.0,
                radius * 2.0,
            ),
            Self::AnnularSector {
                center,
                outer_radius,
                ..
            } => Rect::new(
                center.x - outer_radius,
                center.y - outer_radius,
                outer_radius * 2.0,
                outer_radius * 2.0,
            ),
            Self::Polygon(points) => points_bounds(points.iter().copied()),
            Self::CompoundPolygon(rings) => points_bounds(rings.iter().flatten().copied()),
            Self::Compound(shapes) => points_bounds(shapes.iter().flat_map(|shape| {
                let rect = shape.hover_bounds();
                [
                    Point::new(rect.x, rect.y),
                    Point::new(rect.x + rect.width, rect.y + rect.height),
                ]
            })),
        }
    }

    /// The point distances are measured from: the centre of the hover extent, and for an
    /// annular sector the middle of the sector itself, not the centre of its circle (which is
    /// outside of a ring).
    pub fn hover_center(&self) -> Point {
        match self {
            Self::AnnularSector {
                center,
                inner_radius,
                outer_radius,
                start_angle,
                sweep_angle,
            } => {
                let (sin, cos) = (start_angle + sweep_angle * 0.5).to_radians().sin_cos();
                let radius = (inner_radius + outer_radius) * 0.5;
                Point::new(center.x + cos * radius, center.y + sin * radius)
            }
            _ => {
                let bounds = self.hover_bounds();
                Point::new(bounds.x + bounds.width * 0.5, bounds.y + bounds.height * 0.5)
            }
        }
    }

    fn contains_hover(&self, point: Point) -> bool {
        match self {
            Self::WithHoverArea { bounds, .. } => Self::Rectangle(*bounds).contains(point),
            _ => self.contains(point),
        }
    }

    /// The rectangle of a rectangular hover area (`RectangleHoverArea` of the source).
    fn hover_rectangle(&self) -> Option<Rect> {
        match self {
            Self::Rectangle(rect) | Self::WithHoverArea { bounds: rect, .. } => Some(*rect),
            _ => None,
        }
    }

    /// Whether the point is in the column (`x`) or the row of the hover extent, `tolerance`
    /// wider on each side. A rectangular area takes the rule of `RectangleHoverArea.IsPointerOver`.
    fn contains_hover_along(&self, x: bool, point: Point, tolerance: f64) -> bool {
        let value = if x { point.x } else { point.y };
        let span = |rect: Rect| {
            if x {
                (rect.x, rect.width)
            } else {
                (rect.y, rect.height)
            }
        };
        if let Some(rect) = self.hover_rectangle() {
            let (start, length) = span(rect);
            return hover_span(start, length, value, tolerance);
        }
        let (start, length) = span(self.hover_bounds());
        value >= start - tolerance && value <= start + length + tolerance
    }
}

/// `RectangleHoverArea.IsPointerOver` of the source along one axis: the span from `start` over
/// `length` (either sign) is at least 1 px long, and a value exactly on its edge is outside.
/// `tolerance` widens the span on each side.
fn hover_span(start: f64, length: f64, value: f64, tolerance: f64) -> bool {
    let min = start.min(start + length);
    let max = start.max(start + length);
    let max = if max - min < 1.0 { min + 1.0 } else { max };
    value > min - tolerance && value < max + tolerance
}

fn points_bounds(points: impl Iterator<Item = Point>) -> Rect {
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for point in points {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    if min.x > max.x || min.y > max.y {
        Rect::new(0.0, 0.0, 0.0, 0.0)
    } else {
        Rect::new(min.x, min.y, max.x - min.x, max.y - min.y)
    }
}

fn polygon_contains(points: &[Point], point: Point) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = points[points.len() - 1];
    for current in points {
        if (current.y > point.y) != (previous.y > point.y)
            && point.x
                < (previous.x - current.x) * (point.y - current.y) / (previous.y - current.y)
                    + current.x
        {
            inside = !inside;
        }
        previous = *current;
    }
    inside
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FindingStrategy {
    #[default]
    Automatic,
    CompareAll,
    CompareOnlyX,
    CompareOnlyY,
    CompareAllTakeClosest,
    CompareOnlyXTakeClosest,
    CompareOnlyYTakeClosest,
    ExactMatch,
    ExactMatchTakeClosest,
}
ferroui_base::ferro_markup_enum!(FindingStrategy {
    Automatic,
    CompareAll,
    CompareOnlyX,
    CompareOnlyY,
    CompareAllTakeClosest,
    CompareOnlyXTakeClosest,
    CompareOnlyYTakeClosest,
    ExactMatch,
    ExactMatchTakeClosest
});

#[derive(Clone, Debug, PartialEq)]
pub struct LegendItem {
    pub series: usize,
    pub name: String,
    pub color: Color,
}

/// A complete frame in control-local device independent coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub size: Size,
    pub plot_bounds: Rect,
    pub primitives: Vec<Primitive>,
    pub hits: Vec<HitRegion>,
    pub legend: Vec<LegendItem>,
    /// Stable model identities supplied by a host. Event indexes stay in PointId;
    /// transitions can follow the same model when a collection is reordered.
    pub entity_keys: std::collections::HashMap<PointId, (u64, u64)>,
    /// Resolved from the visible series by the engine, as in GetFindingStrategy.
    pub automatic_finding_strategy: FindingStrategy,
}
impl Scene {
    pub fn new(size: Size, plot_bounds: Rect) -> Self {
        Self {
            size,
            plot_bounds,
            primitives: Vec::new(),
            hits: Vec::new(),
            legend: Vec::new(),
            entity_keys: std::collections::HashMap::new(),
            automatic_finding_strategy: FindingStrategy::CompareAllTakeClosest,
        }
    }
    pub fn hit_test(
        &self,
        point: Point,
        strategy: FindingStrategy,
        tolerance: f64,
    ) -> Vec<&HitRegion> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Vec::new();
        }
        let bounds = self.plot_bounds;
        if bounds.width <= 0.0
            || bounds.height <= 0.0
            || point.x < bounds.x
            || point.x > bounds.x + bounds.width
            || point.y < bounds.y
            || point.y > bounds.y + bounds.height
        {
            return Vec::new();
        }
        let tolerance = if tolerance.is_finite() {
            tolerance.max(0.0)
        } else {
            0.0
        };
        let strategy = if strategy == FindingStrategy::Automatic {
            self.automatic_finding_strategy
        } else {
            strategy
        };
        let closest = matches!(
            strategy,
            FindingStrategy::CompareAllTakeClosest
                | FindingStrategy::CompareOnlyXTakeClosest
                | FindingStrategy::CompareOnlyYTakeClosest
                | FindingStrategy::ExactMatchTakeClosest
        );
        let distance = |hit: &&HitRegion| {
            let center = hit.shape.hover_center();
            match strategy {
                FindingStrategy::CompareOnlyX | FindingStrategy::CompareOnlyXTakeClosest => {
                    (point.x - center.x).abs()
                }
                FindingStrategy::CompareOnlyY | FindingStrategy::CompareOnlyYTakeClosest => {
                    (point.y - center.y).abs()
                }
                _ => (point.x - center.x).hypot(point.y - center.y),
            }
        };
        let mut result: Vec<_> = self
            .hits
            .iter()
            .filter(|hit| match strategy {
                FindingStrategy::ExactMatch | FindingStrategy::ExactMatchTakeClosest => {
                    hit.contains(point)
                }
                FindingStrategy::CompareOnlyX | FindingStrategy::CompareOnlyXTakeClosest => {
                    hit.shape.contains_hover_along(true, point, tolerance)
                }
                FindingStrategy::CompareOnlyY | FindingStrategy::CompareOnlyYTakeClosest => {
                    hit.shape.contains_hover_along(false, point, tolerance)
                }
                _ => hit.shape.contains_hover(point) || distance(hit) <= tolerance,
            })
            .collect();
        if closest {
            result.sort_by(|a, b| {
                a.id.series
                    .cmp(&b.id.series)
                    .then_with(|| distance(a).total_cmp(&distance(b)))
            });
            result.dedup_by_key(|hit| hit.id.series);
        }
        result
    }
}

pub(crate) fn validate_size(size: Size) -> Result<(), ChartError> {
    if !size.width.is_finite() || !size.height.is_finite() || size.width < 0.0 || size.height < 0.0
    {
        Err(ChartError::InvalidSize)
    } else {
        Ok(())
    }
}

/// The series colours of the source's default (light) theme: `ColorPalletes.MaterialDesign500`,
/// taken in series order (`Theme.GetSeriesColor`).
pub const PALETTE: [Color; 9] = [
    Color::from_rgb(33, 150, 243),
    Color::from_rgb(244, 67, 54),
    Color::from_rgb(139, 195, 74),
    Color::from_rgb(0, 188, 212),
    Color::from_rgb(63, 81, 181),
    Color::from_rgb(255, 193, 7),
    Color::from_rgb(0, 150, 136),
    Color::from_rgb(233, 30, 99),
    Color::from_rgb(96, 125, 139),
];

pub fn palette_color(index: usize) -> Color {
    PALETTE[index % PALETTE.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn annular_hit_respects_hole_direction_and_angle_wrap() {
        let hit = HitRegion {
            id: PointId {
                series: 0,
                point: 0,
            },
            shape: HitShape::AnnularSector {
                center: Point::new(0.0, 0.0),
                inner_radius: 2.0,
                outer_radius: 10.0,
                start_angle: 350.0,
                sweep_angle: 30.0,
            },
            center: Point::new(0.0, 0.0),
            value: 1.0,
            label: String::new(),
        };
        assert!(hit.contains(Point::new(5.0, 0.0)));
        assert!(!hit.contains(Point::new(0.0, 0.0)));
        assert!(!hit.contains(Point::new(-5.0, 0.0)));
        assert!(!hit.contains(Point::new(11.0, 0.0)));
    }

    /// A scene of one hit region, in a plot that holds every probe of the rectangle tests.
    fn single_hit(shape: HitShape) -> Scene {
        let mut scene = Scene::new(
            Size::new(1000.0, 1000.0),
            Rect::new(-1000.0, -1000.0, 3000.0, 3000.0),
        );
        scene.hits.push(HitRegion {
            id: PointId {
                series: 0,
                point: 0,
            },
            center: shape.hover_center(),
            shape,
            value: 0.0,
            label: String::new(),
        });
        scene
    }

    /// `RectangleHoverArea.IsPointerOver` of the source, through the hit test of a scene.
    fn is_pointer_over(area: Rect, x: f64, y: f64, strategy: FindingStrategy) -> bool {
        !single_hit(HitShape::Rectangle(area))
            .hit_test(Point::new(x, y), strategy, 0.0)
            .is_empty()
    }

    /// CoreObjectsTests/RectangleHoverAreaTesting.IsPointerOver_ZeroHeight_Tooltipable_OnXOnlyStrategy:
    /// a column collapsed to zero height is found anywhere in its column by X.
    #[test]
    fn source_zero_height_is_tooltipable_on_x_only_strategy() {
        let area = Rect::new(100.0, 200.0, 50.0, 0.0);
        let strategy = FindingStrategy::CompareOnlyXTakeClosest;
        for y in [50.0, 200.0, 350.0] {
            assert!(is_pointer_over(area, 125.0, y, strategy), "in the column at {y}");
        }
        for x in [50.0, 180.0] {
            assert!(!is_pointer_over(area, x, 200.0, strategy), "beside the column at {x}");
        }
    }

    /// RectangleHoverAreaTesting.IsPointerOver_ZeroHeight_NotTooltipable_OnYOnlyStrategy: by Y
    /// the zero-height area is found only in the row of 1 px its minimum size gives it.
    #[test]
    fn source_zero_height_is_not_tooltipable_on_y_only_strategy() {
        let area = Rect::new(100.0, 200.0, 50.0, 0.0);
        let strategy = FindingStrategy::CompareOnlyYTakeClosest;
        assert!(!is_pointer_over(area, 125.0, 50.0, strategy));
        assert!(!is_pointer_over(area, 125.0, 350.0, strategy));
        assert!(is_pointer_over(area, 125.0, 200.5, strategy));
    }

    /// SeriesTests/SeriesHitTests.RectangleHoverArea_OnEdge_Misses: a pointer exactly on an edge
    /// is outside.
    #[test]
    fn source_rectangle_hover_area_on_edge_misses() {
        let area = Rect::new(10.0, 20.0, 30.0, 40.0);
        for (x, y) in [(10.0, 30.0), (40.0, 30.0), (20.0, 20.0), (20.0, 60.0)] {
            assert!(!is_pointer_over(area, x, y, FindingStrategy::CompareAll), "edge {x}, {y}");
            assert!(!HitShape::Rectangle(area).contains(Point::new(x, y)));
        }
        assert!(is_pointer_over(area, 20.0, 30.0, FindingStrategy::CompareAll));
    }

    /// CoreObjectsTests/MeasureTesting.RectangleHoverAreaIsPointerOverCompareAll,
    /// RectangleHoverAreaIsPointerOverCompareOnlyX and RectangleHoverAreaIsPointerOverCompareOnlyY.
    #[test]
    fn source_rectangle_hover_area_is_pointer_over_by_strategy() {
        let area = Rect::new(10.0, 20.0, 100.0, 50.0);
        let all = FindingStrategy::CompareAll;
        assert!(is_pointer_over(area, 50.0, 40.0, all));
        for (x, y) in [(5.0, 40.0), (50.0, 5.0), (200.0, 40.0), (50.0, 200.0)] {
            assert!(!is_pointer_over(area, x, y, all), "outside at {x}, {y}");
        }
        assert!(is_pointer_over(area, 50.0, 999.0, FindingStrategy::CompareOnlyX));
        assert!(!is_pointer_over(area, 5.0, 30.0, FindingStrategy::CompareOnlyX));
        assert!(is_pointer_over(area, 999.0, 40.0, FindingStrategy::CompareOnlyY));
        assert!(!is_pointer_over(area, 50.0, 5.0, FindingStrategy::CompareOnlyY));
    }

    /// MeasureTesting.RectangleHoverAreaMinWidthAndHeight: an area of no size is 1 px wide and
    /// high.
    #[test]
    fn source_rectangle_hover_area_min_width_and_height() {
        let area = Rect::new(50.0, 50.0, 0.0, 0.0);
        assert!(is_pointer_over(area, 50.5, 50.5, FindingStrategy::CompareAll));
    }

    /// MeasureTesting.RectangleHoverAreaIsPointerOverNegativeHeight, ...NegativeWidth and
    /// ...NegativeWidthAndHeight (issue 2165 of the source): the spans are unordered.
    #[test]
    fn source_rectangle_hover_area_is_pointer_over_negative_dimensions() {
        let all = FindingStrategy::CompareAll;
        let height = Rect::new(10.0, 70.0, 100.0, -50.0);
        assert!(is_pointer_over(height, 50.0, 40.0, all));
        assert!(is_pointer_over(height, 50.0, 40.0, FindingStrategy::CompareAllTakeClosest));
        assert!(is_pointer_over(height, 50.0, 40.0, FindingStrategy::CompareOnlyY));
        assert!(!is_pointer_over(height, 50.0, 10.0, all));
        assert!(!is_pointer_over(height, 50.0, 80.0, all));
        let width = Rect::new(110.0, 20.0, -100.0, 50.0);
        assert!(is_pointer_over(width, 50.0, 40.0, all));
        assert!(is_pointer_over(width, 50.0, 40.0, FindingStrategy::CompareOnlyX));
        assert!(!is_pointer_over(width, 5.0, 40.0, all));
        assert!(!is_pointer_over(width, 120.0, 40.0, all));
        let both = Rect::new(110.0, 70.0, -100.0, -50.0);
        assert!(is_pointer_over(both, 50.0, 40.0, all));
        assert!(!is_pointer_over(both, 5.0, 40.0, all));
        assert!(!is_pointer_over(both, 50.0, 80.0, all));
    }

    /// The hover area of a category (the bounds of `WithHoverArea`, a `RectangleHoverArea` in
    /// the source) takes the same rule for every strategy; the native tolerance widens it on
    /// each side and keeps its edge outside.
    #[test]
    fn category_hover_area_excludes_its_edges_and_widens_by_the_tolerance() {
        let scene = single_hit(HitShape::WithHoverArea {
            shape: Box::new(HitShape::Circle {
                center: Point::new(25.0, 40.0),
                radius: 2.0,
            }),
            bounds: Rect::new(10.0, 20.0, 30.0, 40.0),
        });
        let found = |x: f64, y: f64, strategy: FindingStrategy, tolerance: f64| {
            scene.hit_test(Point::new(x, y), strategy, tolerance).len()
        };
        for strategy in [FindingStrategy::CompareAll, FindingStrategy::CompareOnlyX] {
            assert_eq!(found(10.0, 30.0, strategy, 0.0), 0);
            assert_eq!(found(40.0, 30.0, strategy, 0.0), 0);
            assert_eq!(found(10.5, 30.0, strategy, 0.0), 1);
        }
        assert_eq!(found(30.0, 60.0, FindingStrategy::CompareOnlyY, 0.0), 0);
        assert_eq!(found(30.0, 59.5, FindingStrategy::CompareOnlyY, 0.0), 1);
        assert_eq!(found(5.0, 30.0, FindingStrategy::CompareOnlyX, 5.0), 0);
        assert_eq!(found(5.5, 30.0, FindingStrategy::CompareOnlyX, 5.0), 1);
        assert_eq!(found(45.5, 30.0, FindingStrategy::CompareOnlyX, 5.0), 0);
        assert_eq!(found(44.5, 30.0, FindingStrategy::CompareOnlyX, 5.0), 1);
    }
}
