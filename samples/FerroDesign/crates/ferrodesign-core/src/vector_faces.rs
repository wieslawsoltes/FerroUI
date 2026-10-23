//! Checked planar embedding and exact, edge-owned bounded faces.
//!
//! Face boundaries retain authored Bézier commands. Subdivision is used only
//! to certify that control hulls do not intersect, never to approximate output.
//! The operation deliberately declines ambiguous/touching geometry: it neither
//! invents junctions at crossings nor mutates the graph while discovering faces.

use super::*;
use std::fmt;

const MAX_EDGES: usize = 32_768;
const MAX_GEOMETRY_WORK: usize = 1_000_000;
const MAX_SUBDIVISION: u8 = 40;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkFaceError {
    InvalidNetwork(String),
    InvalidRegionFills(String),
    MissingRegion(VectorRegionId),
    AmbiguousRegionReset(VectorRegionId),
    NotEditable,
    DegenerateEdge(VectorEdgeId),
    AmbiguousJunction(VectorVertexId),
    UnsplitIntersection(VectorEdgeId, VectorEdgeId),
    IndeterminateBoundary,
    ComplexityLimit,
    IdExhausted,
    StaleFaceMap,
}

impl fmt::Display for NetworkFaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNetwork(reason) => write!(f, "Invalid vector network: {reason}"),
            Self::InvalidRegionFills(reason) => write!(f, "Invalid region paints: {reason}"),
            Self::MissingRegion(region) => write!(f, "Vector region {region} no longer exists."),
            Self::AmbiguousRegionReset(region) => write!(f, "Region {region} only partly overlaps earlier fill geometry. Resetting its paint would change that coverage; keep an explicit paint stack for this composite region."),
            Self::NotEditable => f.write_str("Select an unlocked vector network."),
            Self::DegenerateEdge(edge) => write!(f, "Edge {edge} has degenerate or singular geometry; remove or split it before finding faces."),
            Self::AmbiguousJunction(vertex) => write!(f, "Vertex {vertex} has coincident outgoing tangents; separate or split those edges before finding faces."),
            Self::UnsplitIntersection(a, b) => write!(f, "Edges {a} and {b} cross, overlap, touch without a shared vertex, or cannot be separated reliably. Split and weld their intersections before finding faces."),
            Self::IndeterminateBoundary => f.write_str("A face has an indeterminate orientation or repeated boundary; simplify its geometry before finding faces."),
            Self::ComplexityLimit => f.write_str("Face discovery exceeded its geometry work limit; split this network into smaller layers."),
            Self::IdExhausted => f.write_str("The vector network has exhausted its stable ID space."),
            Self::StaleFaceMap => f.write_str("The vector network changed during painting. Start a new paint gesture."),
        }
    }
}

impl std::error::Error for NetworkFaceError {}

/// Immutable, certified elementary faces for interactive paint picking.
///
/// Intersection certification runs once at construction. Point queries prune
/// a balanced bounds tree and evaluate the original Bézier winding, retaining
/// narrow faces and holes without a screen-dependent flattening tolerance.
/// Paint changes do not invalidate this map; vertex/edge changes do. No stable
/// region IDs are allocated by preparation or hovering. `prepare` may retain a
/// checked subdivision plan, but does not apply it to the authored network.
pub struct NetworkFaceMap {
    source: VectorNetwork,
    preparation: Option<super::planarize::Preparation>,
    faces: Vec<NetworkRegion>,
    paths: Vec<VectorPath>,
    bounds: Vec<Rect>,
    tree: Vec<FaceBoundsNode>,
    root: Option<usize>,
}

struct FaceBoundsNode {
    bounds: Rect,
    children: Option<(usize, usize)>,
    face: usize,
}

impl NetworkFaceMap {
    pub fn new(network: &VectorNetwork) -> Result<Self, NetworkFaceError> {
        let faces = network.detect_faces()?;
        let paths: Vec<_> = faces.iter().map(|face| {
            let mut path = VectorPath { fill_rule: FillRule::NonZero, ..VectorPath::default() };
            for contour in &face.contours { path.segments.extend(contour_path(network, contour).segments); }
            path
        }).collect();
        let bounds = faces.iter().map(|face| face.contours.iter().flatten()
            .filter_map(|reference| network.edge_curve(reference.edge).map(Curve::hull_bounds))
            .reduce(Rect::union).ok_or(NetworkFaceError::IndeterminateBoundary))
            .collect::<Result<Vec<_>, _>>()?;
        // Authored regions are not part of the checked embedding. Avoid copying
        // their potentially large independent paint/image stacks into the cache.
        let source = VectorNetwork { vertices: network.vertices.clone(), edges: network.edges.clone(),
            regions: BTreeMap::new(), fill_rule: network.fill_rule, next_id: network.next_id };
        let mut result = Self { source, preparation: None, faces, paths, bounds, tree: Vec::new(), root: None };
        let mut indices: Vec<_> = (0..result.faces.len()).collect();
        if !indices.is_empty() { result.root = Some(result.build_tree(&mut indices)); }
        Ok(result)
    }

    /// Prepare common line/cubic crossings on an immutable clone, then certify
    /// the entire resulting embedding with the same strict face builder. This
    /// does not reserve IDs in the caller, change paints, or create history.
    pub fn prepare(network: &VectorNetwork) -> Result<Self, NetworkFaceError> {
        match Self::new(network) {
            Ok(map) => return Ok(map),
            Err(NetworkFaceError::UnsplitIntersection(..) | NetworkFaceError::AmbiguousJunction(..)) => {},
            Err(error) => return Err(error),
        }
        let preparation = super::planarize::Preparation::build(network)?;
        let mut candidate = network.clone();
        preparation.apply(&mut candidate)?;
        let mut map = Self::new(&candidate)?;
        map.preparation = Some(preparation);
        Ok(map)
    }

    pub fn len(&self) -> usize { self.faces.len() }
    pub fn is_empty(&self) -> bool { self.faces.is_empty() }
    pub fn path(&self, face: usize) -> Option<&VectorPath> { self.paths.get(face) }

    pub fn matches_geometry(&self, network: &VectorNetwork) -> bool {
        self.source.vertices == network.vertices && self.source.edges == network.edges
            && self.source.fill_rule == network.fill_rule
    }

    /// A prepared map accepts either its original unmodified allocator/graph
    /// or its applied geometry. Paint-only changes do not copy stale paints
    /// back into the document; a newly allocated region forces re-preparation.
    pub fn matches_source(&self, network: &VectorNetwork) -> bool {
        self.matches_geometry(network) || self.preparation.as_ref().is_some_and(|plan| plan.matches(network))
    }

    pub(super) fn apply_preparation(&self, network: &mut VectorNetwork) -> Result<bool, NetworkFaceError> {
        if self.matches_geometry(network) { return Ok(false); }
        self.preparation.as_ref().ok_or(NetworkFaceError::StaleFaceMap)?.apply(network)?;
        Ok(true)
    }

    pub fn face_at(&self, point: Point) -> Option<usize> {
        if !finite(point) { return None; }
        self.root.and_then(|root| self.find_face(root, point))
    }

    /// Faces crossed by a straight pointer segment, including faces narrower
    /// than the distance between delivered pointer events. Samples are taken
    /// between polynomial curve/segment crossings, not at fixed pixel steps.
    /// Returns one interior sample per face in traversal order.
    pub fn faces_along_segment(&self, start: Point, end: Point) -> Result<Vec<(usize, Point)>, NetworkFaceError> {
        if !finite(start) || !finite(end) { return Ok(Vec::new()); }
        let mut hits = Vec::new();
        let mut seen = BTreeSet::new();
        let mut add = |point: Point, work: &mut Work| -> Result<(), NetworkFaceError> {
            if let Some(face) = self.root.map(|root| self.find_face_checked(root, point, work)).transpose()?.flatten() {
                if seen.insert(face) { hits.push((face, point)); }
            }
            Ok(())
        };
        let mut work = Work(MAX_GEOMETRY_WORK);
        add(start, &mut work)?;
        let delta = end - start;
        let scale = delta.x.abs().max(delta.y.abs());
        if scale == 0.0 { return Ok(hits); }
        if !scale.is_finite() { return Err(NetworkFaceError::IndeterminateBoundary); }
        let direction = delta * (1.0 / scale);
        let length = direction.x * direction.x + direction.y * direction.y;
        let mut candidates = BTreeSet::new();
        if let Some(root) = self.root {
            self.segment_edges(root, Rect::from_points(start, end), &mut candidates, &mut work)?;
        }
        let mut breaks = vec![0.0, 1.0];
        for edge in candidates {
            work.take()?;
            let mut curve = self.source.edge_curve(edge).unwrap();
            curve.points = curve.points.map(|point| (point - start) * (1.0 / scale));
            if curve.points[..=curve.degree].iter().any(|point| !finite(*point)) { return Err(NetworkFaceError::IndeterminateBoundary); }
            let coefficients = curve.power_coefficients();
            let normal: Vec<_> = coefficients[..=curve.degree].iter()
                .map(|point| point.x * direction.y - point.y * direction.x).collect();
            for t in unit_polynomial_roots(&normal) {
                let point = curve.evaluate(t);
                let distance = (point.x * direction.x + point.y * direction.y) / length;
                if distance > 0.0 && distance < 1.0 { breaks.push(distance); }
            }
        }
        breaks.sort_by(f64::total_cmp);
        breaks.dedup();
        for interval in breaks.windows(2) {
            work.take()?;
            add(start.lerp(end, interval[0] + (interval[1] - interval[0]) / 2.0), &mut work)?;
        }
        add(end, &mut work)?;
        Ok(hits)
    }

    fn segment_edges(&self, index: usize, bounds: Rect, result: &mut BTreeSet<VectorEdgeId>, work: &mut Work) -> Result<(), NetworkFaceError> {
        work.take()?;
        let node = &self.tree[index];
        if !node.bounds.intersects(bounds) { return Ok(()); }
        if let Some((left, right)) = node.children {
            self.segment_edges(left, bounds, result, work)?;
            self.segment_edges(right, bounds, result, work)?;
        } else {
            for reference in self.faces[node.face].contours.iter().flatten() {
                work.take()?;
                if self.source.edge_curve(reference.edge).unwrap().hull_bounds().intersects(bounds) {
                    result.insert(reference.edge);
                }
            }
        }
        Ok(())
    }

    fn find_face(&self, index: usize, point: Point) -> Option<usize> {
        let node = &self.tree[index];
        if !node.bounds.contains(point) { return None; }
        if let Some((left, right)) = node.children {
            self.find_face(left, point).or_else(|| self.find_face(right, point))
        } else {
            (self.faces[node.face].contours.iter()
                .map(|contour| contour_winding(&self.source, contour, point)).sum::<i32>() != 0)
                .then_some(node.face)
        }
    }

    fn find_face_checked(&self, index: usize, point: Point, work: &mut Work) -> Result<Option<usize>, NetworkFaceError> {
        work.take()?;
        let node = &self.tree[index];
        if !node.bounds.contains(point) { return Ok(None); }
        if let Some((left, right)) = node.children {
            match self.find_face_checked(left, point, work)? {
                Some(face) => Ok(Some(face)),
                None => self.find_face_checked(right, point, work),
            }
        } else {
            let mut winding = 0;
            for contour in &self.faces[node.face].contours {
                work.0 = work.0.checked_sub(contour.len()).ok_or(NetworkFaceError::ComplexityLimit)?;
                winding += contour_winding(&self.source, contour, point);
            }
            Ok((winding != 0).then_some(node.face))
        }
    }

    fn build_tree(&mut self, indices: &mut [usize]) -> usize {
        let bounds = indices.iter().map(|&index| self.bounds[index]).reduce(Rect::union).unwrap();
        let index = self.tree.len();
        self.tree.push(FaceBoundsNode { bounds, children: None, face: indices[0] });
        if indices.len() > 1 {
            let horizontal = bounds.width >= bounds.height;
            let middle = indices.len() / 2;
            indices.select_nth_unstable_by(middle, |&a, &b| {
                let center = |r: Rect| if horizontal { r.x + r.width / 2.0 } else { r.y + r.height / 2.0 };
                center(self.bounds[a]).total_cmp(&center(self.bounds[b])).then(a.cmp(&b))
            });
            let (left, right) = indices.split_at_mut(middle);
            let children = (self.build_tree(left), self.build_tree(right));
            self.tree[index].children = Some(children);
        }
        index
    }
}

struct Work(usize);

impl Work {
    fn take(&mut self) -> Result<(), NetworkFaceError> {
        self.0 = self.0.checked_sub(1).ok_or(NetworkFaceError::ComplexityLimit)?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct EmbeddedEdge {
    id: VectorEdgeId,
    start: usize,
    end: usize,
    curve: Curve,
    bounds: Rect,
}

struct Boundary {
    contour: Vec<NetworkEdgeRef>,
    area: f64,
    bounds: Rect,
}

fn winding_is_filled(rule: FillRule, winding: i64) -> bool {
    match rule {
        FillRule::NonZero => winding != 0,
        FillRule::EvenOdd => winding % 2 != 0,
    }
}

fn fills_have_paint(fills: &[crate::Fill]) -> bool {
    fills.iter().any(|fill| fill.visible && fill.opacity > 0.0 && match &fill.paint {
        crate::Paint::Solid(color) => color.a != 0,
        crate::Paint::LinearGradient { stops, .. }
        | crate::Paint::RadialGradient { stops, .. }
        | crate::Paint::AngularGradient { stops, .. } => stops.iter().any(|stop| stop.color.a != 0),
        crate::Paint::Image { .. } => true,
    })
}

/// The dual graph gives exact integer winding values inside every elementary
/// face. No point samples, flattening tolerance or tiny inward offsets are
/// required, including for narrow curved annuli and disconnected components.
struct FaceTopology {
    adjacency: Vec<Vec<(usize, VectorEdgeId, i64)>>,
    edges: BTreeSet<VectorEdgeId>,
    exterior: usize,
}

impl FaceTopology {
    fn new(faces: &[NetworkRegion], work: &mut Work) -> Result<Self, NetworkFaceError> {
        let mut sides = BTreeMap::<VectorEdgeId, [Option<usize>; 2]>::new();
        for (face, region) in faces.iter().enumerate() {
            for reference in region.contours.iter().flatten() {
                work.take()?;
                let side = &mut sides.entry(reference.edge).or_default()[usize::from(reference.reversed)];
                if side.replace(face).is_some() { return Err(NetworkFaceError::IndeterminateBoundary); }
            }
        }
        let exterior = faces.len();
        let mut adjacency = vec![Vec::new(); exterior + 1];
        let edges = sides.keys().copied().collect();
        for (edge, [left, right]) in sides {
            let left = left.unwrap_or(exterior);
            let right = right.unwrap_or(exterior);
            if left == right { return Err(NetworkFaceError::IndeterminateBoundary); }
            // A forward oriented boundary adds one to the winding on its left.
            adjacency[right].push((left, edge, 1));
            adjacency[left].push((right, edge, -1));
        }
        Ok(Self { adjacency, edges, exterior })
    }

    fn windings<'a>(
        &self,
        regions: impl IntoIterator<Item = &'a NetworkRegion>,
        work: &mut Work,
    ) -> Result<Vec<i64>, NetworkFaceError> {
        let mut weights = BTreeMap::<VectorEdgeId, i64>::new();
        for region in regions {
            for reference in region.contours.iter().flatten() {
                work.take()?;
                if !self.edges.contains(&reference.edge) { return Err(NetworkFaceError::IndeterminateBoundary); }
                let weight = weights.entry(reference.edge).or_default();
                *weight = weight.checked_add(if reference.reversed { -1 } else { 1 })
                    .ok_or(NetworkFaceError::ComplexityLimit)?;
            }
        }
        let mut windings = vec![None; self.adjacency.len()];
        windings[self.exterior] = Some(0i64);
        let mut pending = std::collections::VecDeque::from([self.exterior]);
        while let Some(face) = pending.pop_front() {
            for &(neighbor, edge, direction) in &self.adjacency[face] {
                work.take()?;
                let value = windings[face].unwrap().checked_add(direction * weights.get(&edge).copied().unwrap_or(0))
                    .ok_or(NetworkFaceError::ComplexityLimit)?;
                if let Some(previous) = windings[neighbor] {
                    if previous != value { return Err(NetworkFaceError::IndeterminateBoundary); }
                } else {
                    windings[neighbor] = Some(value);
                    pending.push_back(neighbor);
                }
            }
        }
        windings.truncate(self.exterior);
        windings.into_iter().collect::<Option<Vec<_>>>().ok_or(NetworkFaceError::IndeterminateBoundary)
    }
}

impl VectorNetwork {
    fn region_reset_removes_overlay(&self, id: VectorRegionId) -> Result<bool, NetworkFaceError> {
        let faces = self.detect_faces()?;
        let mut work = Work(MAX_GEOMETRY_WORK);
        let topology = FaceTopology::new(&faces, &mut work)?;
        let target = topology.windings(std::iter::once(&self.regions[&id]), &mut work)?;
        let inherited = topology.windings(self.regions.values().filter(|region| region.fills.is_none()), &mut work)?;
        let mut covered: Vec<_> = inherited.into_iter().map(|winding| winding_is_filled(self.fill_rule, winding)).collect();
        for (_, region) in self.regions.range(..id).filter(|(_, region)| region.fills.is_some()) {
            let windings = topology.windings(std::iter::once(region), &mut work)?;
            for (covered, winding) in covered.iter_mut().zip(windings) { *covered |= winding_is_filled(self.fill_rule, winding); }
        }
        let mut covered_count = 0;
        let mut uncovered_count = 0;
        for (winding, covered) in target.into_iter().zip(covered) {
            if winding_is_filled(self.fill_rule, winding) {
                if covered { covered_count += 1; } else { uncovered_count += 1; }
            }
        }
        match (covered_count > 0, uncovered_count > 0) {
            (true, false) => Ok(true),
            (false, true) => Ok(false),
            (true, true) => Err(NetworkFaceError::AmbiguousRegionReset(id)),
            (false, false) => Err(NetworkFaceError::IndeterminateBoundary),
        }
    }

    /// Finds every bounded face of a checked planar line/cubic embedding.
    /// Open branches are excluded from boundaries. Nested disconnected cycles
    /// become an outer face with oriented holes and separate inner faces.
    /// Unsplit intersections, coincident tangents and numerically unresolved
    /// curve contacts return an error without changing regions or allocating IDs.
    pub fn detect_faces(&self) -> Result<Vec<NetworkRegion>, NetworkFaceError> {
        self.validate().map_err(NetworkFaceError::InvalidNetwork)?;
        if self.edges.is_empty() {
            return Ok(Vec::new());
        }
        if self.edges.len() > MAX_EDGES {
            return Err(NetworkFaceError::ComplexityLimit);
        }
        let mut work = Work(MAX_GEOMETRY_WORK);
        let vertices: Vec<_> = self.vertices.keys().copied().collect();
        let vertex_index: BTreeMap<_, _> = vertices.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        // Normalize only the geometric certification. Original controls and
        // stable edge references remain authoritative in returned contours.
        let bounds = self.edges.keys().filter_map(|&id| self.edge_curve(id).map(Curve::hull_bounds))
            .reduce(Rect::union).ok_or(NetworkFaceError::IndeterminateBoundary)?;
        let scale = bounds.width.max(bounds.height);
        if !scale.is_finite() || scale <= 0.0 {
            return Err(NetworkFaceError::DegenerateEdge(*self.edges.keys().next().unwrap()));
        }
        let origin = Point::new(bounds.x, bounds.y);
        let mut edges = Vec::with_capacity(self.edges.len());
        let mut adjacency = vec![Vec::<usize>::new(); vertices.len()];
        for (&id, edge) in &self.edges {
            let mut curve = self.edge_curve(id).ok_or(NetworkFaceError::DegenerateEdge(id))?;
            curve.points = curve.points.map(|point| (point - origin) * (1.0 / scale));
            if curve.points[..=curve.degree].iter().any(|point| !finite(*point))
                || curve.points[..=curve.degree].iter().all(|point| *point == curve.points[0])
            {
                return Err(NetworkFaceError::DegenerateEdge(id));
            }
            certify_simple_curve(curve, id, &mut work)?;
            let index = edges.len();
            let start = vertex_index[&edge.start];
            let end = vertex_index[&edge.end];
            adjacency[start].push(index);
            adjacency[end].push(index);
            edges.push(EmbeddedEdge { id, start, end, curve, bounds: curve.hull_bounds() });
        }
        certify_embedding(&edges, &mut work)?;
        let bridges = find_bridges(&edges, &adjacency);
        let mut outgoing = vec![Vec::<usize>::new(); vertices.len()];
        for (index, edge) in edges.iter().enumerate() {
            if bridges[index] { continue; }
            outgoing[edge.start].push(index * 2);
            outgoing[edge.end].push(index * 2 + 1);
        }
        let mut angles = vec![0.0; edges.len() * 2];
        for (vertex, refs) in outgoing.iter_mut().enumerate() {
            for &reference in refs.iter() {
                let direction = outgoing_direction(edges[reference / 2].curve, reference % 2 == 1)
                    .ok_or(NetworkFaceError::DegenerateEdge(edges[reference / 2].id))?;
                angles[reference] = direction.y.atan2(direction.x);
            }
            refs.sort_by(|a, b| angles[*a].total_cmp(&angles[*b]).then(a.cmp(b)));
            if refs.len() > 1 {
                for (&a, &b) in refs.iter().zip(refs.iter().cycle().skip(1)) {
                    let delta = (angles[a] - angles[b]).abs();
                    if delta.min(std::f64::consts::TAU - delta) <= 128.0 * f64::EPSILON {
                        return Err(NetworkFaceError::AmbiguousJunction(vertices[vertex]));
                    }
                }
            }
        }
        let mut next = vec![usize::MAX; edges.len() * 2];
        for refs in &outgoing {
            for (index, &reference) in refs.iter().enumerate() {
                next[reference ^ 1] = refs[(index + refs.len() - 1) % refs.len()];
            }
        }
        let mut visited = vec![false; next.len()];
        let mut positive = Vec::<Boundary>::new();
        let mut negative = Vec::<Boundary>::new();
        for start in 0..next.len() {
            if visited[start] || next[start] == usize::MAX { continue; }
            let mut current = start;
            let mut contour = Vec::new();
            let mut seen_edges = BTreeSet::new();
            loop {
                if visited[current] {
                    if current != start { return Err(NetworkFaceError::IndeterminateBoundary); }
                    break;
                }
                visited[current] = true;
                let edge = edges[current / 2];
                if !seen_edges.insert(edge.id) { return Err(NetworkFaceError::IndeterminateBoundary); }
                contour.push(NetworkEdgeRef { edge: edge.id, reversed: current % 2 == 1 });
                current = next[current];
                if current == usize::MAX { return Err(NetworkFaceError::IndeterminateBoundary); }
            }
            canonicalize_contour(&mut contour);
            let first = &edges[edges.binary_search_by_key(&contour[0].edge, |edge| edge.id).unwrap()];
            let local_origin = if contour[0].reversed { first.curve.points[first.curve.degree] } else { first.curve.points[0] };
            let mut area = 0.0;
            let mut magnitude = 0.0;
            // Every edge index is located once through the monotonic ID list.
            for reference in &contour {
                let index = edges.binary_search_by_key(&reference.edge, |edge| edge.id).unwrap();
                let (value, uncertainty_scale) = curve_area(edges[index].curve, local_origin);
                area += if reference.reversed { -value } else { value };
                magnitude += uncertainty_scale;
            }
            if !area.is_finite() || area.abs() <= magnitude * (256.0 * f64::EPSILON) {
                return Err(NetworkFaceError::IndeterminateBoundary);
            }
            let path = contour_path(self, &contour);
            let bounds = path.tight_bounds().ok_or(NetworkFaceError::IndeterminateBoundary)?;
            let boundary = Boundary { contour, area, bounds };
            if area > 0.0 { positive.push(boundary); } else { negative.push(boundary); }
        }
        let mut faces: Vec<_> = positive.iter().map(|boundary| NetworkRegion { contours: vec![boundary.contour.clone()], fills: None }).collect();
        let mut containers = crate::SpatialIndex::new(scale / (positive.len().max(1) as f64).sqrt());
        for (index, boundary) in positive.iter().enumerate() { containers.insert(index as u64, boundary.bounds); }
        for hole in negative {
            let reference = hole.contour[0];
            let point = self.edge_curve(reference.edge).unwrap().evaluate(0.5);
            let hole_edges: BTreeSet<_> = hole.contour.iter().map(|r| r.edge).collect();
            let mut parent: Option<usize> = None;
            for index in containers.query(Rect::new(point.x, point.y, 0.0, 0.0)) {
                work.take()?;
                let index = index as usize;
                let outer = &positive[index];
                if !outer.contour.iter().any(|r| hole_edges.contains(&r.edge)) && contour_winding(self, &outer.contour, point) != 0
                    && parent.is_none_or(|previous| outer.area < positive[previous].area)
                { parent = Some(index); }
            }
            if let Some(index) = parent { faces[index].contours.push(hole.contour); }
        }
        for face in &mut faces {
            face.contours.sort();
            if !self.region_is_valid(face) { return Err(NetworkFaceError::IndeterminateBoundary); }
        }
        faces.sort_by_key(region_key);
        Ok(faces)
    }

    /// Returns the elementary face containing a node-local point. The complete
    /// checked embedding is reused for exact curve containment, including holes.
    pub fn face_at(&self, point: Point) -> Result<Option<NetworkRegion>, NetworkFaceError> {
        if !finite(point) { return Ok(None); }
        Ok(self.detect_faces()?.into_iter().find(|face| face.contours.iter()
            .map(|contour| contour_winding(self, contour, point)).sum::<i32>() != 0))
    }

    /// Exact containment of one stored region, independent of paint visibility.
    /// This is also the replacement footprint of an explicitly empty stack.
    pub fn region_contains(&self, id: VectorRegionId, point: Point) -> bool {
        finite(point) && self.regions.get(&id).is_some_and(|region| {
            self.region_is_valid(region) && winding_is_filled(self.fill_rule, region.contours.iter()
                .map(|contour| i64::from(contour_winding(self, contour, point))).sum())
        })
    }

    /// The last explicit region owns the point, even if its paint stack is
    /// empty or invisible. Its footprint suppresses every earlier paint.
    pub fn explicit_region_at(&self, point: Point) -> Option<VectorRegionId> {
        if !finite(point) { return None; }
        self.regions.iter().rev().find_map(|(&id, region)| {
            (region.fills.is_some() && self.region_contains(id, point)).then_some(id)
        })
    }

    /// Select the replacement owner first, including a transparent override.
    /// Otherwise select a containing inherited region only when the combined
    /// inherited geometry fills this point under the network's fill rule.
    pub fn region_at(&self, point: Point) -> Option<VectorRegionId> {
        if !finite(point) { return None; }
        if let Some(id) = self.explicit_region_at(point) { return Some(id); }
        let winding = self.regions.values().filter(|region| region.fills.is_none() && self.region_is_valid(region))
            .flat_map(|region| &region.contours)
            .map(|contour| i64::from(contour_winding(self, contour, point))).sum();
        if !winding_is_filled(self.fill_rule, winding) { return None; }
        self.regions.iter().rev().find_map(|(&id, region)| {
            (region.fills.is_none() && self.region_contains(id, point)).then_some(id)
        })
    }

    /// Paint-aware network picking with the same sparse replacement order as
    /// the renderer. Inherited contours retain their *combined* fill rule.
    /// Image alpha and the local alpha of gradient stops remain raster concerns.
    pub fn contains_fill_with_paints(&self, point: Point, inherited: &[crate::Fill]) -> bool {
        if !finite(point) { return false; }
        if let Some(id) = self.explicit_region_at(point) {
            return fills_have_paint(self.regions[&id].fills.as_deref().unwrap());
        }
        fills_have_paint(inherited) && winding_is_filled(self.fill_rule,
            self.regions.values().filter(|region| region.fills.is_none() && self.region_is_valid(region))
                .flat_map(|region| &region.contours)
                .map(|contour| i64::from(contour_winding(self, contour, point))).sum())
    }

    pub fn hit_test_with_fills(&self, point: Point, stroke_tolerance: f64, inherited: &[crate::Fill]) -> bool {
        self.contains_fill_with_paints(point, inherited)
            || (stroke_tolerance.is_finite() && stroke_tolerance >= 0.0
                && self.nearest_stroke(point).is_some_and(|hit| hit.distance <= stroke_tolerance))
    }

    /// Set a complete region stack. An explicit empty stack is a transparent
    /// replacement, not a request to remove its topological region.
    ///
    /// Resetting to None removes a redundant overlay where earlier geometry
    /// already supplies the entire region, or restores inheritance when no
    /// earlier region covers it. Mixed composite coverage is rejected instead
    /// of changing an EvenOdd hole or duplicating inherited alpha.
    pub fn set_region_fills(
        &mut self,
        id: VectorRegionId,
        fills: Option<Vec<crate::Fill>>,
    ) -> Result<bool, NetworkFaceError> {
        if let Some(fills) = &fills { validate_region_fills(fills).map_err(NetworkFaceError::InvalidRegionFills)?; }
        self.validate().map_err(NetworkFaceError::InvalidNetwork)?;
        let previous = &self.regions.get(&id).ok_or(NetworkFaceError::MissingRegion(id))?.fills;
        if previous == &fills { return Ok(false); }
        if fills.is_none() && self.region_reset_removes_overlay(id)? {
            self.regions.remove(&id);
        } else {
            self.regions.get_mut(&id).unwrap().fills = fills;
        }
        Ok(true)
    }

    /// Paint one checked elementary face without rewriting unrelated regions.
    /// A face over a legacy larger inherited region becomes a sparse override;
    /// replacement rendering prevents a second alpha application underneath it.
    /// Existing topmost matching regions keep their identity and contour order.
    /// Outside hits, repeated values and failed preflight preserve the allocator.
    pub fn paint_face_at(
        &mut self,
        point: Point,
        fills: Option<Vec<crate::Fill>>,
    ) -> Result<bool, NetworkFaceError> {
        if let Some(fills) = &fills { validate_region_fills(fills).map_err(NetworkFaceError::InvalidRegionFills)?; }
        let Some(face) = self.face_at(point)? else { return Ok(false); };
        self.paint_detected_face_at(face, point, fills)
    }

    /// Paint a face from an already certified embedding. The geometry guard
    /// makes a retained map safe even after undo, import or topology replacement.
    pub fn paint_mapped_face_at(
        &mut self,
        map: &NetworkFaceMap,
        point: Point,
        fills: Vec<crate::Fill>,
    ) -> Result<bool, NetworkFaceError> {
        validate_region_fills(&fills).map_err(NetworkFaceError::InvalidRegionFills)?;
        if !map.matches_source(self) { return Err(NetworkFaceError::StaleFaceMap); }
        self.validate().map_err(NetworkFaceError::InvalidNetwork)?;
        let Some(face) = map.face_at(point) else { return Ok(false); };
        // The subdivision and the paint are one atomic edit, including ID
        // exhaustion after topology allocation. A no-op never applies a plan.
        let mut candidate = self.clone();
        map.apply_preparation(&mut candidate)?;
        if !candidate.paint_detected_face_at(map.faces[face].clone(), point, Some(fills))? { return Ok(false); }
        *self = candidate;
        Ok(true)
    }

    /// Native point authoring uses the checked preparation path. The strict
    /// `paint_face_at` API remains available for already planar callers.
    pub fn paint_prepared_face_at(&mut self, point: Point, fills: Option<Vec<crate::Fill>>) -> Result<bool, NetworkFaceError> {
        if let Some(fills) = &fills { validate_region_fills(fills).map_err(NetworkFaceError::InvalidRegionFills)?; }
        if !finite(point) { return Ok(false); }
        let map = NetworkFaceMap::prepare(self)?;
        let Some(face) = map.face_at(point) else { return Ok(false); };
        let mut candidate = self.clone();
        map.apply_preparation(&mut candidate)?;
        if !candidate.paint_detected_face_at(map.faces[face].clone(), point, fills)? { return Ok(false); }
        *self = candidate;
        Ok(true)
    }

    fn paint_detected_face_at(
        &mut self, mut face: NetworkRegion, point: Point, fills: Option<Vec<crate::Fill>>,
    ) -> Result<bool, NetworkFaceError> {
        let key = face_key(&face);
        let explicit = self.explicit_region_at(point);
        let existing = if let Some(id) = explicit {
            (face_key(&self.regions[&id]) == key).then_some(id)
        } else {
            self.regions.iter().rev().find_map(|(&id, region)| (face_key(region) == key).then_some(id))
        };
        if let Some(id) = existing {
            // Both entry points have already validated these explicit paints
            // and the graph. Only inheritance reset requires coverage analysis.
            if fills.is_some() {
                let previous = &mut self.regions.get_mut(&id).unwrap().fills;
                if previous == &fills { return Ok(false); }
                *previous = fills;
                return Ok(true);
            }
            return self.set_region_fills(id, fills);
        }
        // There is no elementary override to reset. Creating an inherited face
        // here could cancel an existing EvenOdd region or sit below an explicit
        // larger replacement; it would not reset that replacement.
        if fills.is_none() { return Ok(false); }
        let (id, next) = self.reserve_ids(1).ok_or(NetworkFaceError::IdExhausted)?;
        face.fills = fills;
        self.regions.insert(id, face);
        self.next_id = next;
        Ok(true)
    }

    /// Replaces inherited fill regions with all bounded elementary faces while
    /// retaining explicit region replacements, including composite footprints.
    /// Matching regions keep IDs, paint and serialized contour order; vertex/
    /// edge IDs, geometry, fill rule and stroke ownership never change. Preflight
    /// errors, exhausted IDs and repeated calls leave the entire graph untouched.
    pub fn fill_all_faces(&mut self) -> Result<bool, NetworkFaceError> {
        let faces = self.detect_faces()?;
        self.fill_detected_faces(faces)
    }

    /// Atomically prepare crossings and fill the resulting bounded faces.
    /// An open graph with no bounded face is not rewritten merely by filling.
    pub fn fill_prepared_faces(&mut self) -> Result<bool, NetworkFaceError> {
        let map = NetworkFaceMap::prepare(self)?;
        if map.is_empty() && map.preparation.is_some() { return Ok(false); }
        let mut candidate = self.clone();
        let prepared = map.apply_preparation(&mut candidate)?;
        let changed = candidate.fill_detected_faces(map.faces)?;
        if !prepared && !changed { return Ok(false); }
        *self = candidate;
        Ok(true)
    }

    fn fill_detected_faces(&mut self, faces: Vec<NetworkRegion>) -> Result<bool, NetworkFaceError> {
        let mut existing: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (&id, region) in &self.regions { existing.entry(face_key(region)).or_default().push(id); }
        let mut regions = BTreeMap::new();
        let mut pending = Vec::new();
        for face in faces {
            if let Some(id) = existing.get_mut(&face_key(&face)).and_then(|ids| {
                if ids.is_empty() { None } else { Some(ids.remove(0)) }
            }) {
                regions.insert(id, self.regions[&id].clone());
            } else {
                pending.push(face);
            }
        }
        // A sparse override may span several elementary faces. Filling the
        // underlying topology must not discard its explicit paint, transparent
        // cutout or priority relative to other existing overrides.
        for (&id, region) in self.regions.iter().filter(|(_, region)| region.fills.is_some()) {
            regions.entry(id).or_insert_with(|| region.clone());
        }
        let (first, next) = self.reserve_ids(pending.len() as u64).ok_or(NetworkFaceError::IdExhausted)?;
        for (index, face) in pending.into_iter().enumerate() { regions.insert(first + index as u64, face); }
        if regions == self.regions { return Ok(false); }
        self.regions = regions;
        self.next_id = next;
        Ok(true)
    }
}

impl Editor {
    /// One scoped, reversible topology/fill edit. World transforms and bounds
    /// do not change; preparation errors preserve selection/history/redo.
    pub fn fill_network_faces(&mut self, id: NodeId) -> Result<bool, NetworkFaceError> {
        if self.document.effective_locked(id) { return Err(NetworkFaceError::NotEditable); }
        let mut node = self.document.nodes.get(&id).cloned().ok_or(NetworkFaceError::NotEditable)?;
        if node.kind != NodeKind::Vector { return Err(NetworkFaceError::NotEditable); }
        let network = node.vector_network.as_mut().ok_or(NetworkFaceError::NotEditable)?;
        let changed = network.fill_prepared_faces()?;
        let needs_fill = node.fills.is_empty() && network.regions.values().any(|region| region.fills.is_none());
        if !changed && !needs_fill { return Ok(false); }
        if needs_fill { node.fills.push(crate::Fill::solid(crate::Color::rgb(217, 217, 217))); }
        if node.component_source.is_some() {
            if changed { node.overridden_properties.insert("path".into()); }
            if needs_fill { node.overridden_properties.insert("fills".into()); }
        }
        // The ordinary property update boundary detaches a conflicting empty
        // fill style when authoring the default fill, within the same history.
        self.update_node(id, "Fill network faces", |existing| *existing = node);
        Ok(true)
    }

    /// Point is in the node's local coordinate space, so callers can preserve
    /// their native affine picking and camera behavior. No bounds normalization
    /// is needed for a paint-only edit.
    pub fn paint_network_face(
        &mut self,
        id: NodeId,
        point: Point,
        fills: Option<Vec<crate::Fill>>,
    ) -> Result<bool, NetworkFaceError> {
        self.edit_network_region_paints(id, "Paint vector region", |network| network.paint_prepared_face_at(point, fills))
    }

    pub fn set_network_region_fills(
        &mut self,
        id: NodeId,
        region: VectorRegionId,
        fills: Option<Vec<crate::Fill>>,
    ) -> Result<bool, NetworkFaceError> {
        self.edit_network_region_paints(id, "Edit vector region paints", |network| network.set_region_fills(region, fills))
    }

    pub fn paint_mapped_network_face(
        &mut self, id: NodeId, map: &NetworkFaceMap, point: Point, fills: Vec<crate::Fill>,
    ) -> Result<bool, NetworkFaceError> {
        self.paint_mapped_network_faces(id, map, &[point], &fills)
    }

    /// Batch a pointer segment's faces into one retained node update. Validation,
    /// history capture and invalidation occur once; any failed face leaves the
    /// entire segment unchanged, including its stable-ID allocator.
    pub fn paint_mapped_network_faces(
        &mut self, id: NodeId, map: &NetworkFaceMap, points: &[Point], fills: &[crate::Fill],
    ) -> Result<bool, NetworkFaceError> {
        validate_region_fills(fills).map_err(NetworkFaceError::InvalidRegionFills)?;
        if points.len() > MAX_EDGES { return Err(NetworkFaceError::ComplexityLimit); }
        self.edit_network_region_paints(id, "Paint vector regions", |network| {
            if !map.matches_source(network) { return Err(NetworkFaceError::StaleFaceMap); }
            network.validate().map_err(NetworkFaceError::InvalidNetwork)?;
            let mut changed = false;
            let mut seen = BTreeSet::new();
            let mut work = Work(MAX_GEOMETRY_WORK);
            for &point in points {
                if !finite(point) { continue; }
                let Some(face) = map.root.map(|root| map.find_face_checked(root, point, &mut work)).transpose()?.flatten() else { continue; };
                if seen.insert(face) {
                    map.apply_preparation(network)?;
                    changed |= network.paint_detected_face_at(map.faces[face].clone(), point, Some(fills.to_vec()))?;
                }
            }
            Ok(changed)
        })
    }

    fn edit_network_region_paints(
        &mut self,
        id: NodeId,
        label: &str,
        operation: impl FnOnce(&mut VectorNetwork) -> Result<bool, NetworkFaceError>,
    ) -> Result<bool, NetworkFaceError> {
        if self.document.effective_locked(id) { return Err(NetworkFaceError::NotEditable); }
        let mut node = self.document.nodes.get(&id).cloned().ok_or(NetworkFaceError::NotEditable)?;
        if node.kind != NodeKind::Vector { return Err(NetworkFaceError::NotEditable); }
        if !operation(node.vector_network.as_mut().ok_or(NetworkFaceError::NotEditable)?)? { return Ok(false); }
        // Component reconciliation already preserves the authoritative network
        // under the path override. The layer's local fill style/variables remain
        // linked: only this region's explicit value has been authored.
        if node.component_source.is_some() { node.overridden_properties.insert("path".into()); }
        self.update_node(id, label, |existing| *existing = node);
        Ok(true)
    }
}

fn contour_path(network: &VectorNetwork, contour: &[NetworkEdgeRef]) -> VectorPath {
    let mut path = VectorPath { fill_rule: FillRule::NonZero, ..VectorPath::default() };
    if let Some((start, _, _)) = contour.first().and_then(|reference| network.oriented_edge(*reference)) {
        path.segments.push(PathSegment::MoveTo { point: network.vertices[&start].position });
        path.segments.extend(contour.iter().filter_map(|reference| network.oriented_edge(*reference).map(|(_, _, command)| command)));
        path.segments.push(PathSegment::Close);
    }
    path
}

/// Polynomial ray crossings, independent of the document's flattened hit-test
/// path. Derivative roots partition every curve into y-monotone intervals. The
/// half-open y test counts a shared vertex once and cancels local extrema. A
/// bracketed root search then evaluates x on the actual Bézier, including for
/// arbitrarily small artwork and narrow curved annuli.
fn contour_winding(network: &VectorNetwork, contour: &[NetworkEdgeRef], point: Point) -> i32 {
    let mut winding = 0;
    for reference in contour {
        let mut curve = network.edge_curve(reference.edge).unwrap();
        // Translating before multiplication avoids squaring world coordinates;
        // query-relative scaling keeps tiny and translated artwork equivalent.
        curve.points = curve.points.map(|position| position - point);
        let scale = curve.points[..=curve.degree].iter()
            .map(|position| position.x.abs().max(position.y.abs())).fold(0.0, f64::max);
        if scale <= 0.0 || !scale.is_finite() { continue; }
        curve.points = curve.points.map(|position| position * (1.0 / scale));
        let coefficients = curve.power_coefficients();
        let mut derivative = [0.0; 3];
        for index in 1..=curve.degree { derivative[index - 1] = coefficients[index].y * index as f64; }
        let mut breaks = vec![0.0];
        breaks.extend(unit_polynomial_roots(&derivative[..curve.degree]).into_iter().filter(|&t| t > 0.0 && t < 1.0));
        breaks.push(1.0);
        breaks.sort_by(f64::total_cmp);
        breaks.dedup();
        for interval in breaks.windows(2) {
            let (mut low, mut high) = (interval[0], interval[1]);
            let (a, b) = (curve.evaluate(low).y, curve.evaluate(high).y);
            let direction = if a <= 0.0 && b > 0.0 { 1 }
                else if b <= 0.0 && a > 0.0 { -1 }
                else { continue; };
            let t = if a == 0.0 { low } else if b == 0.0 { high } else {
                for _ in 0..64 {
                    let middle = low + (high - low) * 0.5;
                    if middle == low || middle == high { break; }
                    let y = curve.evaluate(middle).y;
                    if y == 0.0 { low = middle; high = middle; break; }
                    if (y < 0.0) == (direction > 0) { low = middle; } else { high = middle; }
                }
                low + (high - low) * 0.5
            };
            if curve.evaluate(t).x > 0.0 { winding += if reference.reversed { -direction } else { direction }; }
        }
    }
    winding
}

fn canonicalize_contour(contour: &mut [NetworkEdgeRef]) {
    if let Some((first, _)) = contour.iter().enumerate().min_by_key(|(_, reference)| **reference) { contour.rotate_left(first); }
}

fn face_key(region: &NetworkRegion) -> Vec<Vec<NetworkEdgeRef>> {
    let forward = region_key(region);
    let reverse = region_key(&NetworkRegion { contours: region.contours.iter().map(|contour| {
        contour.iter().rev().map(|reference| NetworkEdgeRef { edge: reference.edge, reversed: !reference.reversed }).collect()
    }).collect(), fills: None });
    forward.min(reverse)
}

fn curve_area(mut curve: Curve, origin: Point) -> (f64, f64) {
    curve.points = curve.points.map(|point| point - origin);
    let coefficients = curve.power_coefficients();
    let mut area = 0.0;
    let mut magnitude = 0.0;
    for i in 0..=curve.degree {
        for j in 1..=curve.degree {
            let value = (coefficients[i].x * coefficients[j].y - coefficients[i].y * coefficients[j].x)
                * (j as f64 / (i + j) as f64) * 0.5;
            area += value;
            magnitude += value.abs();
        }
    }
    (area, magnitude)
}

fn outgoing_direction(curve: Curve, reversed: bool) -> Option<Point> {
    let origin = if reversed { curve.points[curve.degree] } else { curve.points[0] };
    for index in 1..=curve.degree {
        let point = curve.points[if reversed { curve.degree - index } else { index }] - origin;
        let length = point.x.hypot(point.y);
        if length > 0.0 && length.is_finite() { return Some(point * (1.0 / length)); }
    }
    None
}

// Iterative Tarjan bridge discovery keeps large open networks off the call
// stack. Parallel edges use edge identity, so a two-edge curved lens is a cycle.
fn find_bridges(edges: &[EmbeddedEdge], adjacency: &[Vec<usize>]) -> Vec<bool> {
    let mut bridges = vec![false; edges.len()];
    let mut discovered = vec![usize::MAX; adjacency.len()];
    let mut low = vec![0; adjacency.len()];
    let mut clock = 0;
    for root in 0..adjacency.len() {
        if discovered[root] != usize::MAX { continue; }
        discovered[root] = clock; low[root] = clock; clock += 1;
        let mut stack = vec![(root, usize::MAX, 0)];
        while let Some((vertex, parent_edge, position)) = stack.last_mut() {
            if *position < adjacency[*vertex].len() {
                let edge_index = adjacency[*vertex][*position];
                *position += 1;
                if edge_index == *parent_edge { continue; }
                let edge = edges[edge_index];
                let other = if edge.start == *vertex { edge.end } else { edge.start };
                if discovered[other] == usize::MAX {
                    discovered[other] = clock; low[other] = clock; clock += 1;
                    stack.push((other, edge_index, 0));
                } else {
                    low[*vertex] = low[*vertex].min(discovered[other]);
                }
            } else {
                let (vertex, parent_edge, _) = stack.pop().unwrap();
                if let Some(&(parent, _, _)) = stack.last() {
                    bridges[parent_edge] = low[vertex] > discovered[parent];
                    low[parent] = low[parent].min(low[vertex]);
                }
            }
        }
    }
    bridges
}

fn split_curve(curve: Curve) -> (Curve, Curve) {
    let mut current = curve.points;
    let mut a = [curve.points[0]; 4];
    let mut b = [curve.points[curve.degree]; 4];
    for level in (1..=curve.degree).rev() {
        for i in 0..level { current[i] = interpolate(current[i], current[i + 1], 0.5); }
        a[curve.degree - level + 1] = current[0];
        b[level - 1] = current[level - 1];
    }
    (Curve { points: a, degree: curve.degree }, Curve { points: b, degree: curve.degree })
}

pub(super) fn monotone(curve: Curve) -> bool {
    [false, true].into_iter().any(|y| {
        let mut differences = [0.0; 3];
        for i in 1..=curve.degree {
            differences[i - 1] = if y { curve.points[i].y - curve.points[i - 1].y } else { curve.points[i].x - curve.points[i - 1].x };
        }
        let differences = &differences[..curve.degree];
        differences.iter().any(|d| *d != 0.0)
            && (differences.iter().all(|d| *d >= 0.0) || differences.iter().all(|d| *d <= 0.0))
    })
}

fn certify_simple_curve(curve: Curve, edge: VectorEdgeId, work: &mut Work) -> Result<(), NetworkFaceError> {
    if monotone(curve) { return Ok(()); }
    let mut pending = vec![(curve, 0u8)];
    let mut pieces = Vec::new();
    while let Some((piece, depth)) = pending.pop() {
        work.take()?;
        if monotone(piece) { pieces.push(piece); continue; }
        if depth == MAX_SUBDIVISION { return Err(NetworkFaceError::DegenerateEdge(edge)); }
        let (a, b) = split_curve(piece);
        pending.push((b, depth + 1)); pending.push((a, depth + 1));
    }
    for (i, &a) in pieces.iter().enumerate() {
        for (j, &b) in pieces.iter().enumerate().skip(i + 1) {
            let mut allowed = Vec::new();
            if j == i + 1 { allowed.push(a.points[a.degree]); }
            if i == 0 && j == pieces.len() - 1 && curve.points[0] == curve.points[curve.degree] { allowed.push(curve.points[0]); }
            certify_disjoint(a, b, &allowed, edge, edge, work)?;
        }
    }
    Ok(())
}

fn certify_embedding(edges: &[EmbeddedEdge], work: &mut Work) -> Result<(), NetworkFaceError> {
    let mut sweep: Vec<_> = (0..edges.len()).collect();
    sweep.sort_by(|&a, &b| edges[a].bounds.x.total_cmp(&edges[b].bounds.x).then(a.cmp(&b)));
    for (i, &a) in sweep.iter().enumerate() {
        let a = edges[a];
        for &b in sweep.iter().skip(i + 1) {
            let b = edges[b];
            if b.bounds.x > a.bounds.x + a.bounds.width { break; }
            work.take()?;
            if a.bounds.y + a.bounds.height < b.bounds.y || b.bounds.y + b.bounds.height < a.bounds.y { continue; }
            let mut allowed = Vec::new();
            for (vertex, point) in [(a.start, a.curve.points[0]), (a.end, a.curve.points[a.curve.degree])] {
                if (vertex == b.start || vertex == b.end) && !allowed.contains(&point) { allowed.push(point); }
            }
            certify_disjoint(a.curve, b.curve, &allowed, a.id, b.id, work)?;
        }
    }
    Ok(())
}

fn certify_disjoint(a: Curve, b: Curve, allowed: &[Point], a_id: VectorEdgeId, b_id: VectorEdgeId, work: &mut Work) -> Result<(), NetworkFaceError> {
    let mut pending = vec![(a, b, 0u8)];
    while let Some((a, b, depth)) = pending.pop() {
        work.take()?;
        if separated_hulls(a, b, allowed) { continue; }
        if (a.degree == 1 && b.degree == 1) || depth == MAX_SUBDIVISION {
            return Err(NetworkFaceError::UnsplitIntersection(a_id, b_id));
        }
        let a_bounds = a.hull_bounds();
        let b_bounds = b.hull_bounds();
        if (a_bounds.width + a_bounds.height >= b_bounds.width + b_bounds.height && a.degree != 1) || b.degree == 1 {
            let (first, second) = split_curve(a);
            pending.push((second, b, depth + 1)); pending.push((first, b, depth + 1));
        } else {
            let (first, second) = split_curve(b);
            pending.push((a, second, depth + 1)); pending.push((a, first, depth + 1));
        }
    }
    Ok(())
}

pub(super) fn separated_hulls(a: Curve, b: Curve, allowed: &[Point]) -> bool {
    if separating_axis(a, b, Point::new(1.0, 0.0), allowed)
        || separating_axis(a, b, Point::new(0.0, 1.0), allowed)
    { return true; }
    for curve in [a, b] {
        for i in 0..=curve.degree {
            for j in i + 1..=curve.degree {
                let direction = curve.points[j] - curve.points[i];
                if direction == Point::default() { continue; }
                if separating_axis(a, b, Point::new(-direction.y, direction.x), allowed)
                    || separating_axis(a, b, direction, allowed)
                { return true; }
            }
        }
    }
    false
}

fn separating_axis(a: Curve, b: Curve, axis: Point, allowed: &[Point]) -> bool {
    let project = |point: Point| point.x * axis.x + point.y * axis.y;
    let range = |curve: Curve| {
        curve.points[..=curve.degree].iter().map(|&point| project(point))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), value| (lo.min(value), hi.max(value)))
    };
    let (a_min, a_max) = range(a);
    let (b_min, b_max) = range(b);
    if a_max < b_min || b_max < a_min { return true; }
    let plane = if a_max == b_min { a_max } else if b_max == a_min { b_max } else { return false; };
    // A nonconstant Bézier projection has a strict sign in its interior when
    // all controls lie on one side. Its endpoints on the separator must be
    // explicitly allowed shared vertices; coincident whole curves are declined.
    let a_off = a_min != a_max;
    let b_off = b_min != b_max;
    if !a_off && !b_off { return false; }
    [a, b].into_iter().zip([a_off, b_off]).all(|(curve, off)| {
        !off || [curve.points[0], curve.points[curve.degree]].into_iter()
            .all(|point| project(point) != plane || allowed.contains(&point))
    })
}

#[cfg(test)]
#[path = "vector_faces_tests.rs"]
mod tests;
