use crate::{require, Result, ToolError};
use ferroui_designer_core::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn validate(self) -> Result<()> {
        require(
            self.x.is_finite() && self.y.is_finite(),
            "Coordinates must be finite",
        )
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn right(self) -> f64 {
        self.x + self.width
    }
    pub fn bottom(self) -> f64 {
        self.y + self.height
    }
    pub fn center(self) -> Point {
        Point {
            x: self.x + self.width * 0.5,
            y: self.y + self.height * 0.5,
        }
    }
    pub fn validate(self) -> Result<()> {
        require(
            [
                self.x,
                self.y,
                self.width,
                self.height,
                self.right(),
                self.bottom(),
            ]
            .iter()
            .all(|v| v.is_finite())
                && self.width >= 0.0
                && self.height >= 0.0,
            "Bounds must have finite coordinates and nonnegative dimensions",
        )
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x <= self.right() && p.y <= self.bottom()
    }
    pub fn intersects(self, other: Self) -> bool {
        self.x <= other.right()
            && self.right() >= other.x
            && self.y <= other.bottom()
            && self.bottom() >= other.y
    }
    pub fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Self {
            x,
            y,
            width: self.right().max(other.right()) - x,
            height: self.bottom().max(other.bottom()) - y,
        }
    }
    pub fn translated(self, delta: Point) -> Result<Self> {
        delta.validate()?;
        let result = Self {
            x: self.x + delta.x,
            y: self.y + delta.y,
            ..self
        };
        result.validate()?;
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    Left,
    HorizontalCenter,
    Right,
    Top,
    VerticalCenter,
    Bottom,
}

/// Row-vector affine transform: x'=x*m11+y*m21+dx, y'=x*m12+y*m22+dy.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Affine {
    pub m11: f64,
    pub m12: f64,
    pub m21: f64,
    pub m22: f64,
    pub dx: f64,
    pub dy: f64,
}
impl Default for Affine {
    fn default() -> Self {
        Self::IDENTITY
    }
}
impl Affine {
    pub const IDENTITY: Self = Self {
        m11: 1.0,
        m12: 0.0,
        m21: 0.0,
        m22: 1.0,
        dx: 0.0,
        dy: 0.0,
    };
    pub fn validate(self) -> Result<()> {
        require(
            [self.m11, self.m12, self.m21, self.m22, self.dx, self.dy]
                .iter()
                .all(|v| v.is_finite()),
            "Transform must be finite",
        )
    }
    pub fn transform(self, point: Point) -> Result<Point> {
        self.validate()?;
        point.validate()?;
        let result = Point {
            x: point.x * self.m11 + point.y * self.m21 + self.dx,
            y: point.x * self.m12 + point.y * self.m22 + self.dy,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn inverse(self) -> Result<Self> {
        self.validate()?;
        let d = self.m11 * self.m22 - self.m12 * self.m21;
        require(d.is_finite() && d != 0.0, "Transform is singular")?;
        let result = Self {
            m11: self.m22 / d,
            m12: -self.m12 / d,
            m21: -self.m21 / d,
            m22: self.m11 / d,
            dx: (self.m21 * self.dy - self.m22 * self.dx) / d,
            dy: (self.m12 * self.dx - self.m11 * self.dy) / d,
        };
        result.validate()?;
        Ok(result)
    }
    /// Apply self, then next.
    pub fn then(self, next: Self) -> Result<Self> {
        self.validate()?;
        next.validate()?;
        let result = Self {
            m11: self.m11 * next.m11 + self.m12 * next.m21,
            m12: self.m11 * next.m12 + self.m12 * next.m22,
            m21: self.m21 * next.m11 + self.m22 * next.m21,
            m22: self.m21 * next.m12 + self.m22 * next.m22,
            dx: self.dx * next.m11 + self.dy * next.m21 + next.dx,
            dy: self.dx * next.m12 + self.dy * next.m22 + next.dy,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn rotation(degrees: f64, pivot: Point) -> Result<Self> {
        require(degrees.is_finite(), "Angle must be finite")?;
        pivot.validate()?;
        let (s, c) = degrees.to_radians().sin_cos();
        let result = Self {
            m11: c,
            m12: s,
            m21: -s,
            m22: c,
            dx: pivot.x - pivot.x * c + pivot.y * s,
            dy: pivot.y - pivot.x * s - pivot.y * c,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn scale(x: f64, y: f64, pivot: Point) -> Result<Self> {
        require(x.is_finite() && y.is_finite(), "Scale must be finite")?;
        pivot.validate()?;
        let result = Self {
            m11: x,
            m22: y,
            dx: pivot.x * (1.0 - x),
            dy: pivot.y * (1.0 - y),
            ..Self::IDENTITY
        };
        result.validate()?;
        Ok(result)
    }
    pub fn bounds(self, rect: Rect) -> Result<Rect> {
        rect.validate()?;
        let points = [
            Point::new(rect.x, rect.y),
            Point::new(rect.right(), rect.y),
            Point::new(rect.right(), rect.bottom()),
            Point::new(rect.x, rect.bottom()),
        ]
        .map(|p| self.transform(p));
        let mut result = None;
        for p in points {
            let p = p?;
            let r = Rect::new(p.x, p.y, 0.0, 0.0);
            result = Some(result.map_or(r, |b: Rect| b.union(r)));
        }
        Ok(result.unwrap())
    }
    pub fn to_xaml(self) -> Result<String> {
        self.validate()?;
        Ok(format!(
            "matrix({},{},{},{},{},{})",
            self.m11, self.m12, self.m21, self.m22, self.dx, self.dy
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapSettings {
    pub grid: Option<f64>,
    pub tolerance: f64,
    pub guides_x: Vec<f64>,
    pub guides_y: Vec<f64>,
}
impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            grid: Some(8.0),
            tolerance: 6.0,
            guides_x: vec![],
            guides_y: vec![],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapGuide {
    pub axis: Axis,
    pub position: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapResult {
    pub delta: Point,
    pub bounds: Rect,
    pub guides: Vec<SnapGuide>,
}
impl SnapSettings {
    /// All geometry is in document units. Convert a desired screen tolerance by the zoom factor.
    pub fn snap(&self, moving: Rect, others: &[Rect]) -> Result<SnapResult> {
        moving.validate()?;
        require(
            self.tolerance.is_finite() && self.tolerance >= 0.0,
            "Snap tolerance must be finite and nonnegative",
        )?;
        if let Some(grid) = self.grid {
            require(
                grid.is_finite() && grid > 0.0,
                "Grid size must be finite and positive",
            )?;
        }
        require(
            self.guides_x
                .iter()
                .chain(&self.guides_y)
                .all(|v| v.is_finite()),
            "Guides must be finite",
        )?;
        let mut x = self.guides_x.clone();
        let mut y = self.guides_y.clone();
        for rect in others {
            rect.validate()?;
            x.extend([rect.x, rect.center().x, rect.right()]);
            y.extend([rect.y, rect.center().y, rect.bottom()]);
        }
        let (dx, gx) = snap_axis(
            [moving.x, moving.center().x, moving.right()],
            &x,
            self.grid,
            self.tolerance,
        );
        let (dy, gy) = snap_axis(
            [moving.y, moving.center().y, moving.bottom()],
            &y,
            self.grid,
            self.tolerance,
        );
        let delta = Point { x: dx, y: dy };
        let bounds = moving.translated(delta)?;
        let mut guides = vec![];
        if let Some(position) = gx {
            guides.push(SnapGuide {
                axis: Axis::Horizontal,
                position,
            });
        }
        if let Some(position) = gy {
            guides.push(SnapGuide {
                axis: Axis::Vertical,
                position,
            });
        }
        Ok(SnapResult {
            delta,
            bounds,
            guides,
        })
    }
}
fn snap_axis(
    anchors: [f64; 3],
    guides: &[f64],
    grid: Option<f64>,
    tolerance: f64,
) -> (f64, Option<f64>) {
    let mut best: Option<(f64, f64)> = None;
    for anchor in anchors {
        for guide in guides {
            let delta = *guide - anchor;
            if delta.abs() <= tolerance
                && best.is_none_or(|(previous, position)| {
                    delta.abs() < previous.abs()
                        || (delta.abs() == previous.abs() && *guide < position)
                })
            {
                best = Some((delta, *guide));
            }
        }
    }
    // Explicit/peer guides win ties; the grid snaps the leading edge, preserving dimensions.
    if let Some(grid) = grid {
        let guide = (anchors[0] / grid).round() * grid;
        let delta = guide - anchors[0];
        if delta.is_finite()
            && delta.abs() <= tolerance
            && best.is_none_or(|(previous, _)| delta.abs() < previous.abs())
        {
            best = Some((delta, guide));
        }
    }
    best.map_or((0.0, None), |(delta, guide)| (delta, Some(guide)))
}
fn validate_rects(rects: &[(NodeId, Rect)]) -> Result<Rect> {
    require(!rects.is_empty(), "Select at least one object")?;
    let mut ids = BTreeSet::new();
    let mut bounds = None;
    for (id, rect) in rects {
        require(ids.insert(*id), "Selection contains duplicate IDs")?;
        rect.validate()?;
        bounds = Some(bounds.map_or(*rect, |previous: Rect| previous.union(*rect)));
    }
    Ok(bounds.unwrap())
}
pub fn align(rects: &[(NodeId, Rect)], alignment: Alignment) -> Result<BTreeMap<NodeId, Point>> {
    let bounds = validate_rects(rects)?;
    let mut result = BTreeMap::new();
    for (id, rect) in rects {
        let delta = match alignment {
            Alignment::Left => Point::new(bounds.x - rect.x, 0.0),
            Alignment::HorizontalCenter => Point::new(bounds.center().x - rect.center().x, 0.0),
            Alignment::Right => Point::new(bounds.right() - rect.right(), 0.0),
            Alignment::Top => Point::new(0.0, bounds.y - rect.y),
            Alignment::VerticalCenter => Point::new(0.0, bounds.center().y - rect.center().y),
            Alignment::Bottom => Point::new(0.0, bounds.bottom() - rect.bottom()),
        };
        delta.validate()?;
        result.insert(*id, delta);
    }
    Ok(result)
}
pub fn distribute(rects: &[(NodeId, Rect)], axis: Axis) -> Result<BTreeMap<NodeId, Point>> {
    validate_rects(rects)?;
    require(
        rects.len() >= 3,
        "Distribution requires at least three objects",
    )?;
    let mut sorted = rects.to_vec();
    let start = |r: Rect| match axis {
        Axis::Horizontal => r.x,
        Axis::Vertical => r.y,
    };
    let length = |r: Rect| match axis {
        Axis::Horizontal => r.width,
        Axis::Vertical => r.height,
    };
    sorted.sort_by(|a, b| start(a.1).total_cmp(&start(b.1)).then(a.0.cmp(&b.0)));
    let last = sorted
        .last()
        .ok_or_else(|| ToolError("Empty selection".into()))?
        .1;
    let total = sorted.iter().map(|(_, r)| length(*r)).sum::<f64>();
    let gap = (start(last) + length(last) - start(sorted[0].1) - total) / (sorted.len() - 1) as f64;
    require(gap.is_finite(), "Distribution overflow")?;
    let mut position = start(sorted[0].1);
    let mut result = BTreeMap::new();
    for (id, rect) in sorted {
        let amount = position - start(rect);
        let delta = match axis {
            Axis::Horizontal => Point::new(amount, 0.0),
            Axis::Vertical => Point::new(0.0, amount),
        };
        delta.validate()?;
        result.insert(id, delta);
        position += length(rect) + gap;
    }
    Ok(result)
}
