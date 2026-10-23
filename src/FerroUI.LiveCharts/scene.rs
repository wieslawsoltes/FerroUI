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
            HitShape::Rectangle(r) => {
                point.x >= r.x.min(r.x + r.width)
                    && point.x <= r.x.max(r.x + r.width)
                    && point.y >= r.y.min(r.y + r.height)
                    && point.y <= r.y.max(r.y + r.height)
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

    fn contains_hover(&self, point: Point) -> bool {
        match self {
            Self::WithHoverArea { bounds, .. } => Self::Rectangle(*bounds).contains(point),
            _ => self.contains(point),
        }
    }
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
            let bounds = hit.shape.hover_bounds();
            let center = Point::new(
                bounds.x + bounds.width * 0.5,
                bounds.y + bounds.height * 0.5,
            );
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
                    let bounds = hit.shape.hover_bounds();
                    point.x >= bounds.x - tolerance
                        && point.x <= bounds.x + bounds.width + tolerance
                }
                FindingStrategy::CompareOnlyY | FindingStrategy::CompareOnlyYTakeClosest => {
                    let bounds = hit.shape.hover_bounds();
                    point.y >= bounds.y - tolerance
                        && point.y <= bounds.y + bounds.height + tolerance
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

pub fn palette_color(index: usize) -> Color {
    const COLORS: [u32; 10] = [
        0xff4263eb, 0xff15aabf, 0xfff59f00, 0xffe64980, 0xff40c057, 0xff7950f2, 0xffff922b,
        0xff228be6, 0xff82c91e, 0xffbe4bdb,
    ];
    let c = COLORS[index % COLORS.len()];
    Color::from_argb((c >> 24) as u8, (c >> 16) as u8, (c >> 8) as u8, c as u8)
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
}
