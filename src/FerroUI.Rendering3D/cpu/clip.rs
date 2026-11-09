//! Clipping in homogeneous coordinates: a triangle of clip-space vertices against the near
//! plane (`z >= 0`), the far plane (`z <= w`), a plane just in front of the eye (`w >= epsilon`,
//! for the matrices of a `MatrixCamera`) and a guard band around the viewport, so that the
//! fixed-point coordinates of the rasterizer stay small. Sutherland-Hodgman, plane by plane, with
//! the attributes interpolated linearly in clip space (where they are linear).

/// The number of attributes a vertex carries: the world position (3), the world normal (3) and
/// the texture coordinate (2).
pub(crate) const ATTRIBUTES: usize = 8;

/// The half width of the guard band, in multiples of the viewport (`|x| <= GUARD * w`): what is
/// beyond is never on the screen, and nothing within it is clipped, so the band only bounds the
/// coordinates.
pub(crate) const GUARD: f32 = 4.0;

/// The smallest `w` that is kept: a point at or behind the eye has no projection.
pub(crate) const W_EPSILON: f32 = 1.0e-6;

/// The number of planes.
const PLANES: usize = 7;

/// The most vertices a polygon can have: a triangle gains at most one vertex per plane.
pub(crate) const MAX_POLYGON: usize = 3 + PLANES;

/// A vertex after the vertex stage.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct ClipVertex {
    /// The position in clip space.
    pub clip: [f32; 4],
    /// The attributes ([`ATTRIBUTES`]).
    pub attributes: [f32; ATTRIBUTES],
}

/// The signed distance of `vertex` to plane `plane`: inside where it is at least zero.
#[inline]
fn distance(plane: usize, vertex: &ClipVertex) -> f32 {
    let [x, y, z, w] = vertex.clip;
    match plane {
        0 => z,
        1 => w - z,
        2 => w - W_EPSILON,
        3 => GUARD * w - x,
        4 => GUARD * w + x,
        5 => GUARD * w - y,
        _ => GUARD * w + y,
    }
}

/// The point of the segment from `a` to `b` at `t`.
#[inline]
fn lerp(a: &ClipVertex, b: &ClipVertex, t: f32) -> ClipVertex {
    let mut result = ClipVertex::default();
    for i in 0..4 {
        result.clip[i] = a.clip[i] + (b.clip[i] - a.clip[i]) * t;
    }
    for i in 0..ATTRIBUTES {
        result.attributes[i] = a.attributes[i] + (b.attributes[i] - a.attributes[i]) * t;
    }
    result
}

/// A convex polygon of at most [`MAX_POLYGON`] vertices, without allocation.
#[derive(Clone, Copy)]
pub(crate) struct Polygon {
    pub vertices: [ClipVertex; MAX_POLYGON],
    pub len: usize,
}

impl Polygon {
    fn empty() -> Polygon {
        Polygon { vertices: [ClipVertex::default(); MAX_POLYGON], len: 0 }
    }

    fn push(&mut self, vertex: ClipVertex) {
        if self.len < MAX_POLYGON {
            self.vertices[self.len] = vertex;
            self.len += 1;
        }
    }

    /// The vertices.
    pub fn as_slice(&self) -> &[ClipVertex] {
        &self.vertices[..self.len]
    }
}

/// What clipping a triangle gave.
pub(crate) enum Clipped {
    /// The triangle is inside every plane: it is drawn as it is.
    Inside,
    /// The triangle is outside a plane, or degenerate: nothing is drawn.
    Outside,
    /// The part of the triangle inside the planes, a convex polygon (drawn as a fan).
    Polygon(Polygon),
}

/// Clips the triangle `triangle` against the planes.
pub(crate) fn clip_triangle(triangle: &[ClipVertex; 3]) -> Clipped {
    if triangle.iter().any(|vertex| vertex.clip.iter().any(|value| !value.is_finite())) {
        return Clipped::Outside;
    }
    let mut outside_any = false;
    for plane in 0..PLANES {
        let inside = triangle.iter().filter(|vertex| distance(plane, vertex) >= 0.0).count();
        if inside == 0 {
            return Clipped::Outside;
        }
        if inside < 3 {
            outside_any = true;
        }
    }
    if !outside_any {
        return Clipped::Inside;
    }

    let mut current = Polygon::empty();
    for vertex in triangle {
        current.push(*vertex);
    }
    let mut next = Polygon::empty();
    for plane in 0..PLANES {
        next.len = 0;
        let count = current.len;
        for index in 0..count {
            let a = &current.vertices[index];
            let b = &current.vertices[(index + 1) % count];
            let (da, db) = (distance(plane, a), distance(plane, b));
            let (a_inside, b_inside) = (da >= 0.0, db >= 0.0);
            if a_inside {
                next.push(*a);
            }
            if a_inside != b_inside {
                next.push(lerp(a, b, da / (da - db)));
            }
        }
        std::mem::swap(&mut current, &mut next);
        if current.len < 3 {
            return Clipped::Outside;
        }
    }
    Clipped::Polygon(current)
}
