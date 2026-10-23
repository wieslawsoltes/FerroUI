use serde::{Deserialize, Serialize};
pub const GEOMETRY_EPSILON: f64 = 1e-8;
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn distance(self, p: Self) -> f64 {
        (self.x - p.x).hypot(self.y - p.y)
    }
    pub fn lerp(self, p: Self, t: f64) -> Self {
        self + (p - self) * t
    }
}
impl std::ops::Add for Point {
    type Output = Self;
    fn add(self, p: Self) -> Self {
        Self::new(self.x + p.x, self.y + p.y)
    }
}
impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, p: Self) -> Self {
        Self::new(self.x - p.x, self.y - p.y)
    }
}
impl std::ops::Mul<f64> for Point {
    type Output = Self;
    fn mul(self, t: f64) -> Self {
        Self::new(self.x * t, self.y * t)
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
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn from_points(a: Point, b: Point) -> Self {
        Self::new(
            a.x.min(b.x),
            a.y.min(b.y),
            (a.x - b.x).abs(),
            (a.y - b.y).abs(),
        )
    }
    pub fn right(self) -> f64 {
        self.x + self.width
    }
    pub fn bottom(self) -> f64 {
        self.y + self.height
    }
    pub fn center(self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x <= self.right() && p.y <= self.bottom()
    }
    pub fn contains_rect(self, r: Self) -> bool {
        r.x >= self.x && r.y >= self.y && r.right() <= self.right() && r.bottom() <= self.bottom()
    }
    pub fn intersects(self, r: Self) -> bool {
        self.x <= r.right() && self.right() >= r.x && self.y <= r.bottom() && self.bottom() >= r.y
    }
    pub fn union(self, r: Self) -> Self {
        let x = self.x.min(r.x);
        let y = self.y.min(r.y);
        Self::new(
            x,
            y,
            self.right().max(r.right()) - x,
            self.bottom().max(r.bottom()) - y,
        )
    }
    pub fn intersection(self, r: Self) -> Option<Self> {
        let x = self.x.max(r.x);
        let y = self.y.max(r.y);
        let w = self.right().min(r.right()) - x;
        let h = self.bottom().min(r.bottom()) - y;
        if w >= 0.0 && h >= 0.0 {
            Some(Self::new(x, y, w, h))
        } else {
            None
        }
    }
    pub fn inflate(self, d: f64) -> Self {
        Self::new(
            self.x - d,
            self.y - d,
            self.width + 2.0 * d,
            self.height + 2.0 * d,
        )
    }
    pub fn corners(self) -> [Point; 4] {
        [
            Point::new(self.x, self.y),
            Point::new(self.right(), self.y),
            Point::new(self.right(), self.bottom()),
            Point::new(self.x, self.bottom()),
        ]
    }
    pub fn is_finite(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
    }
}
/// Column vector affine matrix: x'=a*x+c*y+e, y'=b*x+d*y+f.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}
impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}
impl Transform {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    pub const fn translation(x: f64, y: f64) -> Self {
        Self {
            e: x,
            f: y,
            ..Self::IDENTITY
        }
    }
    pub const fn scale(x: f64, y: f64) -> Self {
        Self {
            a: x,
            d: y,
            b: 0.0,
            c: 0.0,
            e: 0.0,
            f: 0.0,
        }
    }
    pub fn rotation(degrees: f64) -> Self {
        let (s, c) = degrees.to_radians().sin_cos();
        Self {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }
    pub fn then(self, next: Self) -> Self {
        next * self
    }
    pub fn apply(self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }
    pub fn apply_vector(self, p: Point) -> Point {
        Point::new(self.a * p.x + self.c * p.y, self.b * p.x + self.d * p.y)
    }
    pub fn inverse(self) -> Option<Self> {
        let det = self.a * self.d - self.b * self.c;
        if !det.is_finite() || det.abs() < GEOMETRY_EPSILON {
            return None;
        }
        let inv = 1.0 / det;
        Some(Self {
            a: self.d * inv,
            b: -self.b * inv,
            c: -self.c * inv,
            d: self.a * inv,
            e: (self.c * self.f - self.d * self.e) * inv,
            f: (self.b * self.e - self.a * self.f) * inv,
        })
    }
    pub fn transform_rect(self, r: Rect) -> Rect {
        r.corners()
            .into_iter()
            .map(|p| self.apply(p))
            .map(|p| Rect::new(p.x, p.y, 0.0, 0.0))
            .reduce(|a, b| a.union(b))
            .unwrap_or_default()
    }
}
impl std::ops::Mul for Transform {
    type Output = Self;
    fn mul(self, r: Self) -> Self {
        Self {
            a: self.a * r.a + self.c * r.b,
            b: self.b * r.a + self.d * r.b,
            c: self.a * r.c + self.c * r.d,
            d: self.b * r.c + self.d * r.d,
            e: self.a * r.e + self.c * r.f + self.e,
            f: self.b * r.e + self.d * r.f + self.f,
        }
    }
}
pub fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let d = b - a;
    let len = d.x * d.x + d.y * d.y;
    if len < GEOMETRY_EPSILON {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * d.x + (p.y - a.y) * d.y) / len).clamp(0.0, 1.0);
    p.distance(a.lerp(b, t))
}
pub fn point_in_polygon(p: Point, points: &[Point]) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[j];
        if distance_to_segment(p, a, b) < GEOMETRY_EPSILON {
            return true;
        }
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside
        }
        j = i
    }
    inside
}
/// Filled-path containment shared by direct paths and retained picking geometry.
/// The caller chooses its flattening tolerance and can reuse the contours.
pub(crate) fn path_contours_contain(rule: crate::FillRule, contours: &[Vec<Point>], p: Point) -> bool {
    if rule == crate::FillRule::EvenOdd {
        return contours.iter().filter(|points| point_in_polygon(p, points)).count() % 2 == 1;
    }
    let mut winding = 0i32;
    for points in contours {
        if points.len() < 3 { continue; }
        for i in 0..points.len() {
            let a = points[i]; let b = points[(i + 1) % points.len()];
            if distance_to_segment(p, a, b) < GEOMETRY_EPSILON { return true; }
            let cross = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
            if a.y <= p.y {
                if b.y > p.y && cross > 0.0 { winding += 1; }
            } else if b.y <= p.y && cross < 0.0 { winding -= 1; }
        }
    }
    winding != 0
}

pub fn flatten_cubic(
    p0: Point,
    p1: Point,
    p2: Point,
    p3: Point,
    tolerance: f64,
    out: &mut Vec<Point>,
) {
    fn rec(a: Point, b: Point, c: Point, d: Point, t: f64, depth: u8, out: &mut Vec<Point>) {
        if depth >= 16 || distance_to_segment(b, a, d).max(distance_to_segment(c, a, d)) <= t {
            out.push(d);
            return;
        }
        let ab = a.lerp(b, 0.5);
        let bc = b.lerp(c, 0.5);
        let cd = c.lerp(d, 0.5);
        let abc = ab.lerp(bc, 0.5);
        let bcd = bc.lerp(cd, 0.5);
        let m = abc.lerp(bcd, 0.5);
        rec(a, ab, abc, m, t, depth + 1, out);
        rec(m, bcd, cd, d, t, depth + 1, out)
    }
    rec(p0, p1, p2, p3, tolerance.max(0.001), 0, out)
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub pan: Point,
    pub zoom: f64,
    pub width: f64,
    pub height: f64,
}
impl Default for Viewport {
    fn default() -> Self {
        Self {
            pan: Point::new(80.0, 80.0),
            zoom: 0.8,
            width: 1200.0,
            height: 800.0,
        }
    }
}
impl Viewport {
    pub fn screen_to_world(self, p: Point) -> Point {
        Point::new(
            (p.x - self.pan.x) / self.zoom,
            (p.y - self.pan.y) / self.zoom,
        )
    }
    pub fn world_to_screen(self, p: Point) -> Point {
        p * self.zoom + self.pan
    }
    pub fn visible_rect(self) -> Rect {
        let p = self.screen_to_world(Point::default());
        Rect::new(p.x, p.y, self.width / self.zoom, self.height / self.zoom)
    }
    pub fn zoom_at(&mut self, screen: Point, zoom: f64) {
        let world = self.screen_to_world(screen);
        self.zoom = zoom.clamp(0.02, 64.0);
        self.pan = screen - world * self.zoom
    }
    pub fn fit(&mut self, b: Rect, padding: f64) {
        self.zoom = (((self.width - 2.0 * padding).max(1.0) / b.width.max(1.0))
            .min((self.height - 2.0 * padding).max(1.0) / b.height.max(1.0)))
        .clamp(0.02, 64.0);
        self.pan = Point::new(
            self.width / 2.0 - b.center().x * self.zoom,
            self.height / 2.0 - b.center().y * self.zoom,
        )
    }
}
