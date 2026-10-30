use super::ink_drawing_attributes::{
    validate_affine, InkDrawingAttributes, InkDrawingAttributesKind, PenTipShape,
};
use super::ink_point::{valid_position, InkError, InkPoint};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{Matrix, Point, Rect};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

struct StrokeIds {
    next: u32,
    live: HashMap<u32, usize>,
}

ferroui_base::ferro_thread_local! {
    static STROKE_IDS: RefCell<StrokeIds> = RefCell::new(StrokeIds {
        next: 1, live: HashMap::new(),
    });
}

fn allocate_id() -> Result<u32, InkError> {
    STROKE_IDS.with(|ids| {
        let mut ids = ids.borrow_mut();
        // At most live.len() consecutive identities can be occupied. In
        // particular an imported high ID cannot exhaust future allocations.
        for _ in 0..=ids.live.len() {
            let candidate = ids.next;
            ids.next = candidate.checked_add(1).unwrap_or(1);
            if let std::collections::hash_map::Entry::Vacant(entry) = ids.live.entry(candidate) {
                entry.insert(1);
                return Ok(candidate);
            }
        }
        Err(InkError::StrokeIdExhausted)
    })
}

/// One editable stroke identity with immutable raw device samples.
/// Attributes are copied on construction. Geometry is cached until attributes
/// or the point transform change; selection does not invalidate tessellation.
pub struct InkStroke {
    id: u32,
    points: Box<[InkPoint]>,
    attributes: Cell<InkDrawingAttributes>,
    selected: Cell<bool>,
    point_transform: Cell<Matrix>,
    rendering_points: RefCell<Option<Rc<Vec<InkPoint>>>>,
    outlines: RefCell<Option<Rc<Vec<Vec<Point>>>>>,
    bounds: Cell<Option<Rect>>,
    changed: Rc<HandlerList<dyn Fn()>>,
    geometry_vertex_bound: usize,
}

impl PartialEq for InkStroke {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl std::fmt::Debug for InkStroke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InkStroke")
            .field("id", &self.id)
            .field("points", &self.points.len())
            .field("selected", &self.selected.get())
            .finish()
    }
}

impl InkStroke {
    /// Conservative maximum contour vertices reserved by one stroke. This
    /// accounts for curve subdivision and the most detailed supported nib.
    pub const MAX_RENDERED_VERTICES: usize = 1_048_576;

    pub fn new(
        points: Vec<InkPoint>,
        attributes: InkDrawingAttributes,
    ) -> Result<Rc<Self>, InkError> {
        let geometry_vertex_bound = Self::validate(&points, attributes)?;
        let id = allocate_id()?;
        Ok(Self::allocate(
            id,
            points,
            attributes,
            Matrix::IDENTITY,
            false,
            geometry_vertex_bound,
        ))
    }

    fn allocate(
        id: u32,
        points: Vec<InkPoint>,
        attributes: InkDrawingAttributes,
        transform: Matrix,
        selected: bool,
        geometry_vertex_bound: usize,
    ) -> Rc<Self> {
        Rc::new(Self {
            id,
            points: points.into_boxed_slice(),
            attributes: Cell::new(attributes),
            selected: Cell::new(selected),
            point_transform: Cell::new(transform),
            rendering_points: RefCell::new(None),
            outlines: RefCell::new(None),
            bounds: Cell::new(None),
            changed: Rc::new(HandlerList::new()),
            geometry_vertex_bound,
        })
    }

    pub(crate) fn restore(
        id: u32,
        points: Vec<InkPoint>,
        attributes: InkDrawingAttributes,
        transform: Matrix,
        selected: bool,
    ) -> Result<Rc<Self>, InkError> {
        let geometry_vertex_bound = Self::validate(&points, attributes)?;
        validate_affine(transform)?;
        validate_transformed_points(&points, transform)?;
        if id == 0 {
            return Err(InkError::InvalidData("invalid stroke ID"));
        }
        // Restored documents preserve their identities. Counts permit two
        // documents to contain the same serialized ID while new ink still
        // avoids every live identity on this UI thread.
        STROKE_IDS.with(|ids| {
            *ids.borrow_mut().live.entry(id).or_default() += 1;
        });
        Ok(Self::allocate(
            id,
            points,
            attributes,
            transform,
            selected,
            geometry_vertex_bound,
        ))
    }

    fn validate(points: &[InkPoint], attributes: InkDrawingAttributes) -> Result<usize, InkError> {
        if points.is_empty() {
            return Err(InkError::EmptyStroke);
        }
        if points.len() > super::persistence::InkPersistenceLimits::DEFAULT.max_points_per_stroke {
            return Err(InkError::LimitExceeded);
        }
        attributes.validate()?;
        for p in points {
            p.validate()?;
        }
        if points.windows(2).any(|p| p[1].timestamp < p[0].timestamp) {
            return Err(InkError::InvalidInput(
                "Ink timestamps must be nondecreasing",
            ));
        }
        geometry_vertex_bound(points)
    }

    pub(crate) fn geometry_vertex_bound(&self) -> usize {
        self.geometry_vertex_bound
    }

    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn ink_points(&self) -> &[InkPoint] {
        &self.points
    }
    pub fn get_ink_points(&self) -> &[InkPoint] {
        self.ink_points()
    }
    pub fn drawing_attributes(&self) -> InkDrawingAttributes {
        self.attributes.get()
    }
    pub fn set_drawing_attributes(&self, value: InkDrawingAttributes) -> Result<(), InkError> {
        value.validate()?;
        if self.attributes.replace(value) != value {
            self.invalidate_geometry();
        }
        Ok(())
    }
    pub fn selected(&self) -> bool {
        self.selected.get()
    }
    pub fn set_selected(&self, value: bool) {
        if self.selected.replace(value) != value {
            self.notify();
        }
    }
    pub fn point_transform(&self) -> Matrix {
        self.point_transform.get()
    }
    pub fn set_point_transform(&self, value: Matrix) -> Result<(), InkError> {
        validate_affine(value)?;
        validate_transformed_points(&self.points, value)?;
        if self.point_transform.replace(value) != value {
            self.invalidate_geometry();
        }
        Ok(())
    }
    /// A new stroke identity, with the same samples, attributes and transform.
    pub fn clone_stroke(&self) -> Result<Rc<Self>, InkError> {
        let result = Self::new(self.points.to_vec(), self.attributes.get())?;
        result.point_transform.set(self.point_transform.get());
        result.selected.set(self.selected.get());
        Ok(result)
    }
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.changed);
        Disposable::create(move || {
            if let Some(list) = weak.upgrade() {
                list.remove(token);
            }
        })
    }
    fn notify(&self) {
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }
    fn invalidate_geometry(&self) {
        self.rendering_points.borrow_mut().take();
        self.outlines.borrow_mut().take();
        self.bounds.set(None);
        self.notify();
    }

    /// Smoothed local-space samples, cached across redraws. Cubic Hermite
    /// interpolation is limited to the local convex range to avoid overshoot
    /// around sharp corners and repeated samples. Input telemetry is retained.
    pub fn rendering_points(&self) -> Rc<Vec<InkPoint>> {
        if let Some(points) = self.rendering_points.borrow().as_ref() {
            return points.clone();
        }
        let points = if self.attributes.get().fit_to_curve && self.points.len() >= 3 {
            let capacity = 1 + self
                .points
                .windows(2)
                .map(|pair| {
                    ((Point::distance(pair[0].position, pair[1].position) / 4.0).ceil() as usize)
                        .clamp(1, 8)
                })
                .sum::<usize>();
            let mut result = Vec::with_capacity(capacity);
            result.push(self.points[0]);
            for i in 0..self.points.len() - 1 {
                let p0 = self.points[i.saturating_sub(1)].position;
                let p1 = self.points[i].position;
                let p2 = self.points[i + 1].position;
                let p3 = self.points[(i + 2).min(self.points.len() - 1)].position;
                let distance = Point::distance(p1, p2);
                let steps = ((distance / 4.0).ceil() as usize).clamp(1, 8);
                for step in 1..=steps {
                    if step == steps {
                        result.push(self.points[i + 1]);
                        continue;
                    }
                    let t = step as f64 / steps as f64;
                    let t2 = t * t;
                    let t3 = t2 * t;
                    let mut sample = InkPoint::interpolate(self.points[i], self.points[i + 1], t);
                    let coordinate = |a: f64, b: f64, c: f64, d: f64| {
                        let v = 0.5
                            * (2.0 * b
                                + (c - a) * t
                                + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                                + (3.0 * b - a - 3.0 * c + d) * t3);
                        v.clamp(b.min(c), b.max(c))
                    };
                    sample.position = Point::new(
                        coordinate(p0.x, p1.x, p2.x, p3.x),
                        coordinate(p0.y, p1.y, p2.y, p3.y),
                    );
                    result.push(sample);
                }
            }
            result
        } else {
            self.points.to_vec()
        };
        let points = Rc::new(points);
        *self.rendering_points.borrow_mut() = Some(points.clone());
        points
    }

    /// All contours of a stroke, in transformed coordinates and consistently
    /// wound. Fill them together using nonzero winding in ONE geometry draw.
    /// This unions overlapping segments before compositing, which is essential
    /// for highlighters. Elliptic nibs are polygonized to 32 vertices.
    pub fn outline_polygons(&self) -> Rc<Vec<Vec<Point>>> {
        if let Some(outlines) = self.outlines.borrow().as_ref() {
            return outlines.clone();
        }
        let points = self.rendering_points();
        let mut outlines = Vec::with_capacity((self.geometry_vertex_bound - 32) / 64 + 1);
        let mut previous: Option<Vec<Point>> = None;
        let mut previous_sample: Option<InkPoint> = None;
        for point in points.iter() {
            // Elapsed time alone does not alter the nib. Preserve every raw
            // timestamp while avoiding identical overlapping contours.
            if previous_sample.is_some_and(|before| same_geometry_sample(before, *point)) {
                continue;
            }
            let nib = self.nib_polygon(*point);
            if let Some(before) = previous.as_ref() {
                let mut both = Vec::with_capacity(before.len() + nib.len());
                both.extend_from_slice(before);
                both.extend_from_slice(&nib);
                let hull = convex_hull(both);
                if hull.len() >= 3 {
                    outlines.push(hull);
                }
            } else if nib.len() >= 3 {
                outlines.push(nib.clone());
            }
            previous = Some(nib);
            previous_sample = Some(*point);
        }
        let outlines = Rc::new(outlines);
        *self.outlines.borrow_mut() = Some(outlines.clone());
        outlines
    }

    fn nib_basis(&self, sample: InkPoint) -> (Point, Point, Point) {
        let a = self.attributes.get();
        let scale = a.pressure_scale(sample.pressure);
        let mut nib = Matrix::create_scale(a.size.width * scale / 2.0, a.size.height * scale / 2.0);
        if a.kind == InkDrawingAttributesKind::Pencil && !a.ignore_tilt {
            // A portable, deterministic tilt response. Native pencil grain is
            // a renderer/platform feature, not encoded in the stroke samples.
            let tilt = f64::from(sample.tilt_x)
                .hypot(f64::from(sample.tilt_y))
                .min(90.0);
            nib = nib
                * Matrix::create_scale(1.0 + tilt / 30.0, 1.0)
                * Matrix::create_rotation(f64::from(sample.tilt_y).atan2(f64::from(sample.tilt_x)));
        }
        nib = nib
            * Matrix::create_rotation(f64::from(sample.twist).to_radians())
            * a.pen_tip_transform
            * self.point_transform.get();
        let center = sample.position * self.point_transform.get();
        (
            center,
            Point::new(nib.m11, nib.m12),
            Point::new(nib.m21, nib.m22),
        )
    }

    fn nib_polygon(&self, sample: InkPoint) -> Vec<Point> {
        let (c, u, v) = self.nib_basis(sample);
        if u.x == 0.0 && u.y == 0.0 && v.x == 0.0 && v.y == 0.0 {
            return vec![c];
        }
        let points = match self.attributes.get().pen_tip {
            PenTipShape::Rectangle => vec![c - u - v, c + u - v, c + u + v, c - u + v],
            PenTipShape::Circle => (0..32)
                .map(|i| {
                    let t = i as f64 * std::f64::consts::TAU / 32.0;
                    c + u * t.cos() + v * t.sin()
                })
                .collect(),
        };
        convex_hull(points)
    }

    /// Analytic axis-aligned nib bounds, including rotation, shear, pressure,
    /// pencil tilt and stroke transforms. Bounds enclose the flattened curve.
    pub fn bounding_rect(&self) -> Rect {
        if let Some(bounds) = self.bounds.get() {
            return bounds;
        }
        let mut bounds: Option<Rect> = None;
        let mut visible = false;
        for p in self.rendering_points().iter() {
            let (c, u, v) = self.nib_basis(*p);
            let (x, y) = match self.attributes.get().pen_tip {
                PenTipShape::Circle => (u.x.hypot(v.x), u.y.hypot(v.y)),
                PenTipShape::Rectangle => (u.x.abs() + v.x.abs(), u.y.abs() + v.y.abs()),
            };
            visible |= x != 0.0 || y != 0.0;
            let r = Rect::new(c.x - x, c.y - y, 2.0 * x, 2.0 * y);
            bounds = Some(bounds.map_or(r, |b| union_rect(b, r)));
        }
        let bounds = if visible {
            bounds.unwrap_or_default()
        } else {
            Rect::default()
        };
        self.bounds.set(Some(bounds));
        bounds
    }

    /// Swept circular eraser against every stroke segment (including sparse
    /// samples), rather than testing only the sampled input points.
    pub fn hit_test_segment(&self, from: Point, to: Point, radius: f64) -> bool {
        if !valid_position(from) || !valid_position(to) || !radius.is_finite() || radius < 0.0 {
            return false;
        }
        let bounds = self.bounding_rect().inflate(radius);
        if !rect_intersects_segment(bounds, from, to) {
            return false;
        }
        self.outline_polygons().iter().any(|polygon| {
            point_in_polygon(from, polygon)
                || point_in_polygon(to, polygon)
                || polygon_edges(polygon).any(|(a, b)| {
                    segment_distance_squared(from, to, a, b) <= radius * radius + 1.0e-12
                })
        })
    }

    pub(crate) fn contained_in_lasso(&self, polygon: &[Point]) -> bool {
        if polygon.len() < 3 {
            return false;
        }
        let outlines = self.outline_polygons();
        !outlines.is_empty()
            && outlines.iter().all(|nib| {
                nib.iter().all(|p| point_in_polygon(*p, polygon))
                    && polygon_edges(nib).all(|(a, b)| {
                        point_in_polygon((a + b) * 0.5, polygon)
                            && !polygon_edges(polygon)
                                .any(|(c, d)| segments_properly_cross(a, b, c, d))
                    })
            })
    }
}

fn same_geometry_sample(a: InkPoint, b: InkPoint) -> bool {
    a.position == b.position
        && a.pressure == b.pressure
        && a.tilt_x == b.tilt_x
        && a.tilt_y == b.tilt_y
        && a.twist == b.twist
}

/// Reserves the maximum curve-fitted contour cost between validated samples.
/// Input collection uses this incrementally, adding 32 vertices per fragment
/// and enforcing the stroke budget before accepting a sample. Equal geometry
/// samples still retain their timestamps but need no additional contour.
pub(crate) fn geometry_segment_vertex_bound(a: InkPoint, b: InkPoint) -> usize {
    if same_geometry_sample(a, b) {
        return 0;
    }
    let subdivisions =
        ((Point::distance(a.position, b.position) / 4.0).ceil() as usize).clamp(1, 8);
    subdivisions * 64
}

fn geometry_vertex_bound(points: &[InkPoint]) -> Result<usize, InkError> {
    let mut vertices = 32usize;
    for pair in points.windows(2) {
        if same_geometry_sample(pair[0], pair[1]) {
            continue;
        }
        let cost = if points.len() >= 3 {
            geometry_segment_vertex_bound(pair[0], pair[1])
        } else {
            64
        };
        vertices = vertices.checked_add(cost).ok_or(InkError::LimitExceeded)?;
        if vertices > InkStroke::MAX_RENDERED_VERTICES {
            return Err(InkError::LimitExceeded);
        }
    }
    Ok(vertices)
}

impl Drop for InkStroke {
    fn drop(&mut self) {
        let _ = STROKE_IDS.try_with(|ids| {
            let mut ids = ids.borrow_mut();
            if let Some(count) = ids.live.get_mut(&self.id) {
                *count -= 1;
                if *count == 0 {
                    ids.live.remove(&self.id);
                }
            }
        });
    }
}

fn validate_transformed_points(points: &[InkPoint], matrix: Matrix) -> Result<(), InkError> {
    if points.iter().any(|p| !valid_position(p.position * matrix)) {
        return Err(InkError::InvalidTransform);
    }
    Ok(())
}

pub(crate) fn union_rect(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect::new(
        x,
        y,
        a.right().max(b.right()) - x,
        a.bottom().max(b.bottom()) - y,
    )
}

fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}
fn convex_hull(mut points: Vec<Point>) -> Vec<Point> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup();
    if points.len() <= 2 {
        return points;
    }
    let mut result = Vec::with_capacity(points.len() + 1);
    for p in &points {
        while result.len() >= 2
            && cross(result[result.len() - 2], result[result.len() - 1], *p) <= 0.0
        {
            result.pop();
        }
        result.push(*p);
    }
    let lower_len = result.len();
    for p in points[..points.len() - 1].iter().rev() {
        while result.len() > lower_len
            && cross(result[result.len() - 2], result[result.len() - 1], *p) <= 0.0
        {
            result.pop();
        }
        result.push(*p);
    }
    result.pop();
    result
}
fn polygon_edges(points: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
        .take(points.len())
}
fn point_in_polygon(p: Point, polygon: &[Point]) -> bool {
    if polygon.is_empty() {
        return false;
    }
    let mut inside = false;
    for (a, b) in polygon_edges(polygon) {
        if point_segment_distance_squared(p, a, b) <= 1.0e-20 {
            return true;
        }
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}
fn point_segment_distance_squared(p: Point, a: Point, b: Point) -> f64 {
    let d = b - a;
    let length = d.x * d.x + d.y * d.y;
    let t = if length == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * d.x + (p.y - a.y) * d.y) / length).clamp(0.0, 1.0)
    };
    let q = a + d * t;
    (p.x - q.x).powi(2) + (p.y - q.y).powi(2)
}
fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    segments_properly_cross(a, b, c, d)
        || point_segment_distance_squared(a, c, d) <= 1.0e-20
        || point_segment_distance_squared(b, c, d) <= 1.0e-20
        || point_segment_distance_squared(c, a, b) <= 1.0e-20
        || point_segment_distance_squared(d, a, b) <= 1.0e-20
}
fn segments_properly_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let ab_c = cross(a, b, c);
    let ab_d = cross(a, b, d);
    let cd_a = cross(c, d, a);
    let cd_b = cross(c, d, b);
    (ab_c > 0.0 && ab_d < 0.0 || ab_c < 0.0 && ab_d > 0.0)
        && (cd_a > 0.0 && cd_b < 0.0 || cd_a < 0.0 && cd_b > 0.0)
}
fn segment_distance_squared(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if segments_intersect(a, b, c, d) {
        0.0
    } else {
        point_segment_distance_squared(a, c, d)
            .min(point_segment_distance_squared(b, c, d))
            .min(point_segment_distance_squared(c, a, b))
            .min(point_segment_distance_squared(d, a, b))
    }
}
fn rect_intersects_segment(r: Rect, a: Point, b: Point) -> bool {
    if r.contains(a) || r.contains(b) {
        return true;
    }
    let corners = [
        r.top_left(),
        r.top_right(),
        r.bottom_right(),
        r.bottom_left(),
    ];
    let intersects = polygon_edges(&corners).any(|(c, d)| segments_intersect(a, b, c, d));
    intersects
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stroke(points: &[(f64, f64)]) -> Rc<InkStroke> {
        InkStroke::new(
            points
                .iter()
                // The nominal pressure, which draws the configured size.
                .map(|&(x, y)| InkPoint::new(Point::new(x, y), 0.5).unwrap())
                .collect(),
            InkDrawingAttributes {
                fit_to_curve: false,
                ..Default::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn dot_bounds_and_zero_pressure() {
        let s = stroke(&[(10.0, 20.0)]);
        assert_eq!(s.bounding_rect(), Rect::new(9.0, 19.0, 2.0, 2.0));
        assert!(s.hit_test_segment(Point::new(10.0, 20.0), Point::new(10.0, 20.0), 0.0));
        let zero = InkStroke::new(
            vec![InkPoint::new(Point::default(), 0.0).unwrap()],
            Default::default(),
        )
        .unwrap();
        assert!(zero.outline_polygons().is_empty());
        assert!(!zero.hit_test_segment(Point::default(), Point::default(), 1.0));
    }
    #[test]
    fn erasing_hits_between_sparse_samples() {
        let s = stroke(&[(0.0, 0.0), (100.0, 0.0)]);
        assert!(s.hit_test_segment(Point::new(50.0, -20.0), Point::new(50.0, 20.0), 0.1));
        assert!(!s.hit_test_segment(Point::new(50.0, 5.0), Point::new(50.0, 6.0), 1.0));
    }
    #[test]
    fn rotated_rectangle_has_analytic_nib_bounds() {
        let a = InkDrawingAttributes {
            size: ferroui_base::Size::new(4.0, 2.0),
            pen_tip: PenTipShape::Rectangle,
            pen_tip_transform: Matrix::create_rotation(std::f64::consts::FRAC_PI_4),
            ..Default::default()
        };
        let s = InkStroke::new(vec![InkPoint::new(Point::default(), 0.5).unwrap()], a).unwrap();
        assert!((s.bounding_rect().width - 3.0 * 2.0_f64.sqrt()).abs() < 1.0e-9);
        s.set_point_transform(Matrix::create_scale(2.0, 3.0))
            .unwrap();
        assert!((s.bounding_rect().width - 6.0 * 2.0_f64.sqrt()).abs() < 1.0e-9);
    }
    #[test]
    fn geometry_cache_survives_selection_but_not_transform() {
        let s = stroke(&[(0.0, 0.0), (10.0, 10.0)]);
        let geometry = s.outline_polygons();
        s.set_selected(true);
        assert!(Rc::ptr_eq(&geometry, &s.outline_polygons()));
        s.set_point_transform(Matrix::create_translation(5.0, 2.0))
            .unwrap();
        assert!(!Rc::ptr_eq(&geometry, &s.outline_polygons()));
        assert_eq!(s.ink_points()[0].position, Point::default());
    }
    #[test]
    fn lasso_requires_entire_stroke_containment() {
        let s = stroke(&[(-100.0, 0.0), (100.0, 0.0)]);
        let lasso = [
            Point::new(-1.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(1.0, 1.0),
            Point::new(-1.0, 1.0),
        ];
        assert!(!s.contained_in_lasso(&lasso));
        let inside = stroke(&[(0.0, 0.0)]);
        assert!(inside.contained_in_lasso(&lasso));
    }
    #[test]
    fn cloned_strokes_have_distinct_identity() {
        let a = stroke(&[(0.0, 0.0)]);
        let b = a.clone_stroke().unwrap();
        assert_ne!(a.id(), b.id());
        assert_eq!(a.ink_points(), b.ink_points());
    }
    #[test]
    fn geometry_budget_accounts_for_subdivision_and_repeated_samples() {
        let samples = |spacing: f64| {
            (0..2100)
                .map(|index| InkPoint::new(Point::new(index as f64 * spacing, 0.0), 1.0).unwrap())
                .collect()
        };
        assert!(InkStroke::new(samples(1.0), Default::default()).is_ok());
        assert!(matches!(
            InkStroke::new(samples(32.0), Default::default()),
            Err(InkError::LimitExceeded)
        ));
        // Keep the complete timed device history, but render one stationary
        // nib instead of thousands of identical overlapping contours.
        let repeated = (0..65_536)
            .map(|timestamp| {
                let mut point = InkPoint::new(Point::default(), 1.0).unwrap();
                point.timestamp = timestamp;
                point
            })
            .collect();
        let stroke = InkStroke::new(repeated, Default::default()).unwrap();
        assert_eq!(stroke.ink_points().len(), 65_536);
        assert_eq!(stroke.geometry_vertex_bound(), 32);
        assert_eq!(stroke.outline_polygons().len(), 1);
    }
    #[test]
    fn smoothing_preserves_exact_device_endpoints_at_large_coordinates() {
        let mut endpoint = InkPoint::new(Point::new(0.001, 0.003), 1.0e-8).unwrap();
        endpoint.timestamp = (1u64 << 54) + 3;
        let points = vec![
            InkPoint::new(Point::new(1.0e12, -1.0e12), 1.0).unwrap(),
            endpoint,
            InkPoint {
                position: Point::new(0.002, 0.004),
                ..endpoint
            },
        ];
        let stroke = InkStroke::new(points.clone(), Default::default()).unwrap();
        let rendered = stroke.rendering_points();
        for original in points {
            assert!(rendered.contains(&original));
        }
    }
    #[test]
    fn imported_ids_do_not_exhaust_new_strokes_or_collide() {
        let p = InkPoint::new(Point::default(), 1.0).unwrap();
        let high = InkStroke::restore(
            u32::MAX,
            vec![p],
            Default::default(),
            Matrix::IDENTITY,
            false,
        )
        .unwrap();
        let next = STROKE_IDS.with(|ids| ids.borrow().next);
        let reserved =
            InkStroke::restore(next, vec![p], Default::default(), Matrix::IDENTITY, false).unwrap();
        let fresh = stroke(&[(0.0, 0.0)]);
        assert_ne!(fresh.id(), high.id());
        assert_ne!(fresh.id(), reserved.id());
    }
    #[test]
    fn drawing_attributes_are_snapshots() {
        let mut a = InkDrawingAttributes::default();
        let s = InkStroke::new(vec![InkPoint::new(Point::default(), 1.0).unwrap()], a).unwrap();
        a.size.width = 30.0;
        assert_eq!(s.drawing_attributes().size.width, 2.0);
    }
}
