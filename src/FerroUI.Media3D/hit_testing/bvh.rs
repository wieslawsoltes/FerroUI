//! The bounding-volume hierarchy of a mesh: the triangles of one version of a
//! `MeshGeometry3D` in a binary tree of axis-aligned boxes, built once per `(id, version)` and
//! kept by the mesh, and the intersection of a ray with them (Moller-Trumbore, with barycentric
//! weights).
//!
//! The tree is made of the positions in double precision (the values of the mesh's
//! `Positions`, so the points and weights of a hit are those of the mesh) and the triangles of
//! the mesh's [`MeshData`] (the indices that can be drawn: a triangle naming a position that does
//! not exist is left out, as rendering leaves it out). A node is split at the median of the
//! centres of its triangles along the longest axis of their bounds; a leaf holds at most
//! [`LEAF_SIZE`] triangles. The nodes are in one vector, a node's first child right after it.

use crate::{Point3D, Vector3D};
use ferroui_base::rendering::scene3d::MeshData;
use std::cell::Cell;
use std::sync::Arc;

/// The most triangles of a leaf.
const LEAF_SIZE: usize = 4;

/// Below this share of the product of the lengths of the edges and the direction, a triangle is
/// parallel to the ray or degenerate (no area): it is not hit.
const PARALLEL_EPSILON: f64 = 1e-12;

ferroui_base::ferro_thread_local! {
    /// The hierarchies built on this thread: what the tests of the cache count.
    static BUILT: Cell<u64> = const { Cell::new(0) };
}

/// How many hierarchies of meshes were built on this thread.
pub(crate) fn hierarchies_built() -> u64 {
    BUILT.with(Cell::get)
}

/// An axis-aligned box.
#[derive(Clone, Copy, Debug)]
struct Bounds {
    min: [f64; 3],
    max: [f64; 3],
}

impl Bounds {
    const EMPTY: Bounds = Bounds { min: [f64::INFINITY; 3], max: [f64::NEG_INFINITY; 3] };

    fn add_point(&mut self, point: [f64; 3]) {
        for axis in 0..3 {
            // `min` and `max` keep the other value for a NaN coordinate.
            self.min[axis] = self.min[axis].min(point[axis]);
            self.max[axis] = self.max[axis].max(point[axis]);
        }
    }

    fn add(&mut self, other: &Bounds) {
        self.add_point(other.min);
        self.add_point(other.max);
    }

    fn centre(&self) -> [f64; 3] {
        [(self.min[0] + self.max[0]) * 0.5, (self.min[1] + self.max[1]) * 0.5, (self.min[2] + self.max[2]) * 0.5]
    }

    /// Whether the ray `origin + t * direction`, `t >= 0`, passes through the box (the slab test;
    /// `inverse` is one over each component of the direction).
    fn hit_by(&self, origin: [f64; 3], inverse: [f64; 3]) -> bool {
        let mut near = 0.0f64;
        let mut far = f64::INFINITY;
        for axis in 0..3 {
            let t1 = (self.min[axis] - origin[axis]) * inverse[axis];
            let t2 = (self.max[axis] - origin[axis]) * inverse[axis];
            // A NaN (a ray in the plane of a face of the box) leaves the interval as it is.
            near = near.max(t1.min(t2));
            far = far.min(t1.max(t2));
        }
        near <= far
    }
}

/// A node of the tree: a leaf names `count` triangles of [`MeshBvh::triangles`] from `first`; an
/// inner node (`count` 0) has its first child right after it and its second at `first`.
#[derive(Clone, Copy, Debug)]
struct Node {
    bounds: Bounds,
    first: u32,
    count: u32,
}

/// The hierarchy of one version of a mesh; see the documentation of the module.
pub(crate) struct MeshBvh {
    /// The positions of the mesh, in its coordinates.
    positions: Vec<[f64; 3]>,
    /// The triangles that can be drawn (their indices into the positions).
    mesh: Arc<MeshData>,
    nodes: Vec<Node>,
    /// The triangles (their numbers in `mesh.indices`, three indices each) in the order of the
    /// leaves.
    triangles: Vec<u32>,
}

/// A triangle hit by a ray, in the coordinates of the mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TriangleHit {
    /// The parameter of the ray at the hit (`origin + t * direction`).
    pub t: f64,
    /// The indices of the three vertices (into `Positions`).
    pub vertices: [u32; 3],
    /// The barycentric weights of the three vertices, summing to 1.
    pub weights: [f64; 3],
    /// The point hit.
    pub point: Point3D,
    /// Whether the ray hit the front face of the triangle (its vertices counterclockwise seen
    /// from where the ray comes from).
    pub front: bool,
}

impl MeshBvh {
    /// Builds the hierarchy of `positions` (the mesh's own) and the triangles of `mesh`.
    pub(crate) fn build(positions: Vec<[f64; 3]>, mesh: Arc<MeshData>) -> MeshBvh {
        BUILT.with(|built| built.set(built.get() + 1));
        let triangle_count = mesh.triangle_count();
        let mut boxes = Vec::with_capacity(triangle_count);
        for triangle in mesh.indices.chunks_exact(3) {
            let mut bounds = Bounds::EMPTY;
            for &index in triangle {
                // `MeshData` names only positions that exist; a mesh changed since cannot be
                // read here (the hierarchy is made of one version), but stay safe.
                bounds.add_point(positions.get(index as usize).copied().unwrap_or([f64::NAN; 3]));
            }
            boxes.push(bounds);
        }
        let mut triangles: Vec<u32> = (0..triangle_count as u32).collect();
        let mut nodes = Vec::with_capacity(if triangle_count == 0 { 0 } else { 2 * triangle_count / LEAF_SIZE + 1 });
        if triangle_count > 0 {
            build_node(&mut nodes, &mut triangles, 0, &boxes);
        }
        MeshBvh { positions, mesh, nodes, triangles }
    }

    /// The number of triangles.
    pub(crate) fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Calls `hit` for every triangle the ray `origin + t * direction`, `t >= 0`, passes through
    /// on a side in `sides` (`(front, back)`), and returns how many triangles were tested.
    pub(crate) fn intersect(
        &self,
        origin: Point3D,
        direction: Vector3D,
        sides: (bool, bool),
        hit: &mut dyn FnMut(TriangleHit),
    ) -> usize {
        if self.nodes.is_empty() || !(sides.0 || sides.1) {
            return 0;
        }
        let o = [origin.x, origin.y, origin.z];
        let d = [direction.x, direction.y, direction.z];
        let inverse = [1.0 / d[0], 1.0 / d[1], 1.0 / d[2]];
        let mut tested = 0;
        let mut stack: Vec<u32> = Vec::with_capacity(64);
        stack.push(0);
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index as usize];
            if !node.bounds.hit_by(o, inverse) {
                continue;
            }
            if node.count == 0 {
                stack.push(node.first);
                stack.push(index + 1);
                continue;
            }
            for &triangle in &self.triangles[node.first as usize..(node.first + node.count) as usize] {
                tested += 1;
                let base = triangle as usize * 3;
                let vertices = [self.mesh.indices[base], self.mesh.indices[base + 1], self.mesh.indices[base + 2]];
                if let Some(result) = self.intersect_triangle(o, d, vertices, sides) {
                    hit(result);
                }
            }
        }
        tested
    }

    /// Moller-Trumbore: the hit of the ray on one triangle, if it is on a side in `sides`.
    fn intersect_triangle(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        vertices: [u32; 3],
        sides: (bool, bool),
    ) -> Option<TriangleHit> {
        let p0 = *self.positions.get(vertices[0] as usize)?;
        let p1 = *self.positions.get(vertices[1] as usize)?;
        let p2 = *self.positions.get(vertices[2] as usize)?;
        let e1 = sub(p1, p0);
        let e2 = sub(p2, p0);
        let p = cross(direction, e2);
        let determinant = dot(e1, p);
        let scale = length(e1) * length(e2) * length(direction);
        // Not a number, parallel to the ray, or without area.
        if !(determinant.abs() > PARALLEL_EPSILON * scale) {
            return None;
        }
        // The determinant is -(e1 x e2) . direction: positive when the ray meets the side the
        // counterclockwise normal points to, the front.
        let front = determinant > 0.0;
        if (front && !sides.0) || (!front && !sides.1) {
            return None;
        }
        let inverse = 1.0 / determinant;
        let s = sub(origin, p0);
        let u = dot(s, p) * inverse;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = cross(s, e1);
        let v = dot(direction, q) * inverse;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = dot(e2, q) * inverse;
        if !(t >= 0.0) {
            return None;
        }
        let w0 = 1.0 - u - v;
        let point = Point3D::new(
            w0 * p0[0] + u * p1[0] + v * p2[0],
            w0 * p0[1] + u * p1[1] + v * p2[1],
            w0 * p0[2] + u * p1[2] + v * p2[2],
        );
        Some(TriangleHit { t, vertices, weights: [w0, u, v], point, front })
    }
}

/// Builds the node of `triangles` (a range of the whole list starting at `offset`) and its
/// descendants; returns its index.
fn build_node(nodes: &mut Vec<Node>, triangles: &mut [u32], offset: usize, boxes: &[Bounds]) -> u32 {
    let mut bounds = Bounds::EMPTY;
    let mut centres = Bounds::EMPTY;
    for &triangle in triangles.iter() {
        let triangle_bounds = &boxes[triangle as usize];
        bounds.add(triangle_bounds);
        centres.add_point(triangle_bounds.centre());
    }
    let index = nodes.len() as u32;
    nodes.push(Node { bounds, first: offset as u32, count: triangles.len() as u32 });
    if triangles.len() <= LEAF_SIZE {
        return index;
    }
    let extent = [centres.max[0] - centres.min[0], centres.max[1] - centres.min[1], centres.max[2] - centres.min[2]];
    let axis = if extent[0] >= extent[1] && extent[0] >= extent[2] {
        0
    } else if extent[1] >= extent[2] {
        1
    } else {
        2
    };
    // Every centre in one point (or not a number): no split separates the triangles.
    if !(extent[axis] > 0.0) {
        return index;
    }
    let middle = triangles.len() / 2;
    triangles.select_nth_unstable_by(middle, |a, b| {
        boxes[*a as usize].centre()[axis].total_cmp(&boxes[*b as usize].centre()[axis])
    });
    let (left, right) = triangles.split_at_mut(middle);
    build_node(nodes, left, offset, boxes);
    let second = build_node(nodes, right, offset + middle, boxes);
    let node = &mut nodes[index as usize];
    node.first = second;
    node.count = 0;
    index
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
