//! Bounded intersection preparation for native region authoring.
//!
//! Curves remain Bézier curves. Subdivision isolates transverse intersections;
//! it is never a polyline approximation. A candidate is accepted only after the
//! ordinary face builder certifies its complete planar embedding. Coincident
//! runs, tangencies and numerically unresolved contacts are deliberately errors.

use super::*;

const MAX_EDGES: usize = 32_768;
const MAX_WORK: usize = 1_000_000;
const MAX_DEPTH: u8 = 40;
// These bounds are in normalized geometry / parameter space, not screen pixels.
// They limit round-off when two independently evaluated curves share a vertex.
const ROUND_OFF: f64 = 256.0 * f64::EPSILON;
const PARAMETER_ROUND_OFF: f64 = 128.0 * f64::EPSILON;

pub(super) struct Preparation {
    original: VectorNetwork,
    prepared: VectorNetwork,
    replacements: BTreeMap<VectorEdgeId, Vec<VectorEdgeId>>,
}

fn geometry(network: &VectorNetwork) -> VectorNetwork {
    VectorNetwork { vertices: network.vertices.clone(), edges: network.edges.clone(),
        regions: BTreeMap::new(), fill_rule: network.fill_rule, next_id: network.next_id }
}

impl Preparation {
    pub(super) fn matches(&self, network: &VectorNetwork) -> bool {
        self.original.vertices == network.vertices && self.original.edges == network.edges
            && self.original.fill_rule == network.fill_rule && self.original.next_id == network.next_id
    }

    /// Paint stacks are deliberately absent from the cache. Apply the edge
    /// expansion to the caller's current regions, preserving their IDs/order.
    pub(super) fn apply(&self, network: &mut VectorNetwork) -> Result<(), NetworkFaceError> {
        if !self.matches(network) { return Err(NetworkFaceError::StaleFaceMap); }
        let mut candidate = self.prepared.clone();
        candidate.regions = network.regions.clone();
        let mut work = Work(MAX_WORK);
        for region in candidate.regions.values_mut() {
            for contour in &mut region.contours {
                let mut expanded = Vec::new();
                for reference in contour.iter() {
                    work.take()?;
                    if let Some(edges) = self.replacements.get(&reference.edge) {
                        work.0 = work.0.checked_sub(edges.len()).ok_or(NetworkFaceError::ComplexityLimit)?;
                        if reference.reversed {
                            expanded.extend(edges.iter().rev().copied().map(NetworkEdgeRef::reverse));
                        } else {
                            expanded.extend(edges.iter().copied().map(NetworkEdgeRef::forward));
                        }
                    } else { expanded.push(*reference); }
                }
                *contour = expanded;
            }
        }
        candidate.validate().map_err(NetworkFaceError::InvalidNetwork)?;
        *network = candidate;
        Ok(())
    }

    /// Builds a candidate only. NetworkFaceMap::prepare performs the final
    /// embedding/face certification before this plan can escape to a caller.
    pub(super) fn build(network: &VectorNetwork) -> Result<Self, NetworkFaceError> {
        network.validate().map_err(NetworkFaceError::InvalidNetwork)?;
        if network.edges.len() > MAX_EDGES { return Err(NetworkFaceError::ComplexityLimit); }
        let bounds = network.edges.keys().filter_map(|&id| network.edge_curve(id).map(Curve::hull_bounds))
            .reduce(Rect::union).ok_or(NetworkFaceError::IndeterminateBoundary)?;
        let scale = bounds.width.max(bounds.height);
        if !scale.is_finite() || scale <= 0.0 || !(1.0 / scale).is_finite() {
            return Err(NetworkFaceError::IndeterminateBoundary);
        }
        let origin = Point::new(bounds.x, bounds.y);
        let mut edges = Vec::with_capacity(network.edges.len());
        for (&id, edge) in &network.edges {
            let mut curve = network.edge_curve(id).unwrap();
            curve.points = curve.points.map(|p| (p - origin) * (1.0 / scale));
            if curve.points[..=curve.degree].iter().any(|p| !finite(*p)) {
                return Err(NetworkFaceError::IndeterminateBoundary);
            }
            edges.push(Edge { id, start: edge.start, end: edge.end, curve, bounds: curve.hull_bounds() });
        }
        let mut work = Work(MAX_WORK);
        let mut hits = Vec::new();
        for edge in &edges { self_intersections(*edge, &mut hits, &mut work)?; }
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
                for (id, point) in [(a.start, a.curve.points[0]), (a.end, a.curve.points[a.curve.degree])] {
                    if id == b.start || id == b.end { allowed.push(point); }
                }
                intersections(a, Piece::whole(a.curve), b, Piece::whole(b.curve), &allowed, &mut hits, &mut work)?;
            }
        }
        assemble(network, &edges, hits, origin, scale, &mut work)
    }
}

impl VectorNetwork {
    /// Split supported crossings and weld exact endpoint contacts atomically.
    /// Original first-subedge IDs, stroke flags and region paints survive.
    /// Unchanged networks allocate nothing; ambiguous geometry changes nothing.
    pub fn planarize_intersections(&mut self) -> Result<bool, NetworkFaceError> {
        NetworkFaceMap::prepare(self)?.apply_preparation(self)
    }
}

#[derive(Clone, Copy)]
struct Edge { id: VectorEdgeId, start: VectorVertexId, end: VectorVertexId, curve: Curve, bounds: Rect }

#[derive(Clone, Copy)]
struct Piece { curve: Curve, from: f64, to: f64 }

impl Piece {
    fn whole(curve: Curve) -> Self { Self { curve, from: 0.0, to: 1.0 } }
    fn parameter(self, t: f64) -> f64 { self.from + (self.to - self.from) * t }
    fn split(self) -> (Self, Self) {
        let (a, b) = split(self.curve, 0.5);
        let middle = self.parameter(0.5);
        (Self { curve: a, from: self.from, to: middle }, Self { curve: b, from: middle, to: self.to })
    }
}

#[derive(Clone, Copy)]
struct Hit { a: VectorEdgeId, ta: f64, b: VectorEdgeId, tb: f64, point: Point }

struct Work(usize);
impl Work {
    fn take(&mut self) -> Result<(), NetworkFaceError> {
        self.0 = self.0.checked_sub(1).ok_or(NetworkFaceError::ComplexityLimit)?;
        Ok(())
    }
}

fn cross(a: Point, b: Point) -> f64 { a.x * b.y - a.y * b.x }
fn dot(a: Point, b: Point) -> f64 { a.x * b.x + a.y * b.y }
fn near(a: Point, b: Point) -> bool { (a.x - b.x).abs().max((a.y - b.y).abs()) <= ROUND_OFF }
fn error(a: Edge, b: Edge) -> NetworkFaceError { NetworkFaceError::UnsplitIntersection(a.id, b.id) }

fn split(curve: Curve, t: f64) -> (Curve, Curve) {
    let mut current = curve.points;
    let mut a = [curve.points[0]; 4];
    let mut b = [curve.points[curve.degree]; 4];
    for level in (1..=curve.degree).rev() {
        for i in 0..level { current[i] = interpolate(current[i], current[i + 1], t); }
        a[curve.degree - level + 1] = current[0];
        b[level - 1] = current[level - 1];
    }
    (Curve { points: a, degree: curve.degree }, Curve { points: b, degree: curve.degree })
}

fn derivative(curve: Curve, t: f64) -> Point {
    let mut points = [Point::default(); 4];
    for i in 0..curve.degree { points[i] = (curve.points[i + 1] - curve.points[i]) * curve.degree as f64; }
    Curve { points, degree: curve.degree - 1 }.evaluate(t)
}

fn transverse(a: Point, b: Point) -> bool {
    cross(a, b).abs() > (a.x.abs() * b.y.abs() + a.y.abs() * b.x.abs()) * (1024.0 * f64::EPSILON)
        && cross(a, b) != 0.0
}

fn add_hit(a: Edge, ta: f64, b: Edge, tb: f64, hits: &mut Vec<Hit>, work: &mut Work) -> Result<(), NetworkFaceError> {
    work.take()?;
    if hits.len() >= MAX_EDGES { return Err(NetworkFaceError::ComplexityLimit); }
    if !(0.0..=1.0).contains(&ta) || !(0.0..=1.0).contains(&tb) { return Err(error(a, b)); }
    // Only the identical parameter denotes a subdivision boundary. Distinct
    // roots inside the numerical ambiguity window may enclose a real loop.
    if a.id == b.id && ta == tb { return Ok(()); }
    if a.id == b.id && (ta - tb).abs() <= PARAMETER_ROUND_OFF { return Err(error(a, b)); }
    let pa = a.curve.evaluate(ta);
    let pb = b.curve.evaluate(tb);
    if !near(pa, pb) { return Err(error(a, b)); }
    let endpoint = |edge: Edge, t| if t == 0.0 { Some(edge.start) } else if t == 1.0 { Some(edge.end) } else { None };
    if endpoint(a, ta).is_some_and(|id| endpoint(b, tb) == Some(id)) { return Ok(()); }
    let separated_endpoint = endpoint(a, ta).is_some() && endpoint(b, tb).is_some() && pa == pb
        && faces::separated_hulls(a.curve, b.curve, &[pa]);
    if !separated_endpoint && !transverse(derivative(a.curve, ta), derivative(b.curve, tb)) { return Err(error(a, b)); }
    hits.push(Hit { a: a.id, ta, b: b.id, tb, point: pa.lerp(pb, 0.5) });
    Ok(())
}

fn line_line(a: Edge, b: Edge, hits: &mut Vec<Hit>, work: &mut Work) -> Result<(), NetworkFaceError> {
    let p = a.curve.points[0];
    let q = b.curve.points[0];
    let r = a.curve.points[1] - p;
    let s = b.curve.points[1] - q;
    if !transverse(r, s) { return Err(error(a, b)); }
    let t = cross(q - p, s) / cross(r, s);
    let u = cross(q - p, r) / cross(r, s);
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) { add_hit(a, t, b, u, hits, work)?; }
    Ok(())
}

fn line_cubic(line: Edge, cubic: Edge, hits: &mut Vec<Hit>, work: &mut Work) -> Result<(), NetworkFaceError> {
    let start = line.curve.points[0];
    let direction = line.curve.points[1] - start;
    let length = dot(direction, direction);
    if length == 0.0 { return Err(NetworkFaceError::DegenerateEdge(line.id)); }
    let mut relative = cubic.curve;
    relative.points = relative.points.map(|p| p - start);
    let coefficients: Vec<_> = relative.power_coefficients().iter().map(|p| cross(*p, direction)).collect();
    if coefficients.iter().all(|&c| c == 0.0) { return Err(error(line, cubic)); }
    for mut t in unit_polynomial_roots(&coefficients) {
        work.take()?;
        // Polish the bracketed polynomial result against de Casteljau itself.
        // This avoids cancellation in a power polynomial near an endpoint.
        for _ in 0..6 {
            let residual = cross(relative.evaluate(t), direction);
            let slope = cross(derivative(relative, t), direction);
            if slope == 0.0 { break; }
            let next = t - residual / slope;
            if !(0.0..=1.0).contains(&next) || next == t { break; }
            t = next;
        }
        let point = relative.evaluate(t);
        let u = dot(point, direction) / length;
        if (0.0..=1.0).contains(&u) { add_hit(line, u, cubic, t, hits, work)?; }
    }
    Ok(())
}

/// A common projection makes each piece a monotone graph; disjoint derivative
/// slope intervals prove at most one transverse intersection in this pair.
fn unique_crossing(a: Curve, b: Curve) -> bool {
    let da = derivative(a, 0.5);
    let mut db = derivative(b, 0.5);
    let reverse = dot(da, db) < 0.0;
    if reverse { db = db * -1.0; }
    let la = da.x.hypot(da.y);
    let lb = db.x.hypot(db.y);
    if la == 0.0 || lb == 0.0 { return false; }
    let axis = da * (1.0 / la) + db * (1.0 / lb);
    let range = |curve: Curve, reverse: bool| -> Option<(f64, f64)> {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for i in 0..curve.degree {
            let mut d = curve.points[i + 1] - curve.points[i];
            if reverse { d = d * -1.0; }
            let projection = dot(d, axis);
            if projection <= d.x.hypot(d.y) * (1024.0 * f64::EPSILON) { return None; }
            let slope = cross(axis, d) / projection;
            min = min.min(slope); max = max.max(slope);
        }
        Some((min, max))
    };
    match (range(a, false), range(b, reverse)) {
        (Some(a), Some(b)) => a.1 + ROUND_OFF < b.0 || b.1 + ROUND_OFF < a.0,
        _ => false,
    }
}

fn solve_pair(a: Curve, b: Curve) -> Option<(f64, f64)> {
    // Exact endpoint coincidences also cover roots at a subdivision boundary.
    for ta in [0.0, 1.0] { for tb in [0.0, 1.0] {
        if a.evaluate(ta) == b.evaluate(tb) { return Some((ta, tb)); }
    } }
    let (mut ta, mut tb) = (0.5, 0.5);
    for _ in 0..24 {
        let delta = a.evaluate(ta) - b.evaluate(tb);
        let da = derivative(a, ta);
        let db = derivative(b, tb);
        if !transverse(da, db) { return None; }
        let next_a = ta - cross(delta, db) / cross(da, db);
        let next_b = tb - cross(delta, da) / cross(da, db);
        if !(0.0..=1.0).contains(&next_a) || !(0.0..=1.0).contains(&next_b) { return None; }
        if next_a == ta && next_b == tb { break; }
        ta = next_a; tb = next_b;
    }
    near(a.evaluate(ta), b.evaluate(tb)).then_some((ta, tb))
}

fn intersections(a: Edge, pa: Piece, b: Edge, pb: Piece, allowed: &[Point], hits: &mut Vec<Hit>, work: &mut Work) -> Result<(), NetworkFaceError> {
    if faces::separated_hulls(pa.curve, pb.curve, allowed) { return Ok(()); }
    if a.id != b.id {
        // Exact end-to-end contacts can be certified without a transverse
        // derivative (for example two collinear continuations). Never snap
        // nearby endpoints or accept an overlapping run through this path.
        let mut contacts = Vec::new();
        let mut endpoints = allowed.to_vec();
        for ta in [0.0, 1.0] { for tb in [0.0, 1.0] {
            if a.curve.evaluate(ta) == b.curve.evaluate(tb) {
                contacts.push((ta, tb)); endpoints.push(a.curve.evaluate(ta));
            }
        } }
        if !contacts.is_empty() && faces::separated_hulls(a.curve, b.curve, &endpoints) {
            for (ta, tb) in contacts { add_hit(a, ta, b, tb, hits, work)?; }
            return Ok(());
        }
        if a.curve.degree == 1 && b.curve.degree == 1 { return line_line(a, b, hits, work); }
        if a.curve.degree == 1 { return line_cubic(a, b, hits, work); }
        if b.curve.degree == 1 { return line_cubic(b, a, hits, work); }
    }
    let mut pending = vec![(pa, pb, 0u8)];
    while let Some((pa, pb, depth)) = pending.pop() {
        work.take()?;
        if faces::separated_hulls(pa.curve, pb.curve, allowed) { continue; }
        if unique_crossing(pa.curve, pb.curve) {
            if let Some((ta, tb)) = solve_pair(pa.curve, pb.curve) {
                add_hit(a, pa.parameter(ta), b, pb.parameter(tb), hits, work)?;
                continue;
            }
        }
        if depth == MAX_DEPTH { return Err(error(a, b)); }
        let ab = pa.curve.hull_bounds();
        let bb = pb.curve.hull_bounds();
        if ab.width + ab.height >= bb.width + bb.height {
            let (first, second) = pa.split();
            pending.push((second, pb, depth + 1)); pending.push((first, pb, depth + 1));
        } else {
            let (first, second) = pb.split();
            pending.push((pa, second, depth + 1)); pending.push((pa, first, depth + 1));
        }
    }
    Ok(())
}

fn self_intersections(edge: Edge, hits: &mut Vec<Hit>, work: &mut Work) -> Result<(), NetworkFaceError> {
    if faces::monotone(edge.curve) { return Ok(()); }
    let mut pending = vec![(Piece::whole(edge.curve), 0u8)];
    let mut pieces = Vec::new();
    while let Some((piece, depth)) = pending.pop() {
        work.take()?;
        if faces::monotone(piece.curve) { pieces.push(piece); continue; }
        if depth == MAX_DEPTH { return Err(NetworkFaceError::DegenerateEdge(edge.id)); }
        let (a, b) = piece.split();
        pending.push((b, depth + 1)); pending.push((a, depth + 1));
    }
    for (i, &a) in pieces.iter().enumerate() {
        for (j, &b) in pieces.iter().enumerate().skip(i + 1) {
            work.take()?;
            let mut allowed = Vec::new();
            if j == i + 1 { allowed.push(a.curve.points[a.curve.degree]); }
            if i == 0 && j == pieces.len() - 1 && edge.start == edge.end { allowed.push(edge.curve.points[0]); }
            intersections(edge, a, edge, b, &allowed, hits, work)?;
        }
    }
    Ok(())
}

fn root(parents: &mut [usize], mut id: usize) -> usize {
    while parents[id] != id { parents[id] = parents[parents[id]]; id = parents[id]; }
    id
}

fn join(parents: &mut [usize], a: usize, b: usize) {
    let a = root(parents, a); let b = root(parents, b);
    parents[a.max(b)] = a.min(b);
}

fn portion(curve: Curve, from: f64, to: f64) -> Curve {
    let left = if to == 1.0 { curve } else { split(curve, to).0 };
    if from == 0.0 { left } else { split(left, from / to).1 }
}

fn ordered_hit(hit: Hit) -> (VectorEdgeId, f64, VectorEdgeId, f64) {
    if hit.a < hit.b || (hit.a == hit.b && hit.ta <= hit.tb) {
        (hit.a, hit.ta, hit.b, hit.tb)
    } else { (hit.b, hit.tb, hit.a, hit.ta) }
}

/// Repeated Newton solves on adjacent subdivision pieces can differ by a few
/// ULPs. Proximity only selects events to inspect: a derivative-cone certificate
/// must prove that their common parameter rectangle has at most one crossing.
/// Events involving three different lines instead need exact concurrence on
/// the ORIGINAL coordinates; normalized evaluation equality is insufficient.
fn same_intersection(network: &VectorNetwork, edges: &[Edge], common: VectorEdgeId,
    first: Hit, second: Hit, work: &mut Work) -> Result<bool, NetworkFaceError>
{
    if !near(first.point, second.point) { return Ok(false); }
    let a = ordered_hit(first);
    let b = ordered_hit(second);
    if a.0 == b.0 && a.2 == b.2 {
        if (a.1 - b.1).abs() > PARAMETER_ROUND_OFF || (a.3 - b.3).abs() > PARAMETER_ROUND_OFF { return Ok(false); }
        let curve = |id| edges[edges.binary_search_by_key(&id, |edge| edge.id).unwrap()].curve;
        let (ca, cb) = (curve(a.0), curve(a.2));
        let mut padding = 0.25;
        for _ in 0..24 {
            work.take()?;
            let pa = portion(ca, (a.1.min(b.1) - padding).max(0.0), (a.1.max(b.1) + padding).min(1.0));
            let pb = portion(cb, (a.3.min(b.3) - padding).max(0.0), (a.3.max(b.3) + padding).min(1.0));
            if unique_crossing(pa, pb) { return Ok(true); }
            padding *= 0.5;
        }
        return Ok(false);
    }
    let other = |hit: Hit| if hit.a == common { hit.b } else { hit.a };
    let curves = [common, other(first), other(second)].map(|id| network.edge_curve(id).unwrap());
    if curves.iter().any(|curve| curve.degree != 1) { return Ok(false); }
    // The predicate is small, fixed-degree exact arithmetic, but still charges
    // work so a large nearly concurrent input cannot escape the global bound.
    work.0 = work.0.checked_sub(128).ok_or(NetworkFaceError::ComplexityLimit)?;
    Ok(exact_line_concurrence(curves))
}

/// Nonoverlapping floating-point expansion, using error-free two-sum and FMA
/// product residuals. The concurrence caller first scales by a power of two
/// and bounds input exponents. Its degree-four determinant therefore cannot
/// overflow or underflow, including the least significant expansion terms.
struct Exact(Vec<f64>);
impl Exact {
    fn number(value: f64) -> Self { Self(if value == 0.0 { Vec::new() } else { vec![value] }) }
    fn term(&mut self, mut value: f64) {
        let mut result = Vec::with_capacity(self.0.len() + 1);
        for &part in &self.0 {
            let sum = value + part;
            let virtual_part = sum - value;
            let error = (value - (sum - virtual_part)) + (part - virtual_part);
            if error != 0.0 { result.push(error); }
            value = sum;
        }
        if value != 0.0 { result.push(value); }
        self.0 = result;
    }
    fn add(mut self, other: Self) -> Self { for part in other.0 { self.term(part); } self }
    fn sub(mut self, other: Self) -> Self { for part in other.0 { self.term(-part); } self }
    fn mul(&self, other: &Self) -> Self {
        let mut result = Self(Vec::new());
        for &a in &self.0 { for &b in &other.0 {
            let product = a * b;
            result.term(a.mul_add(b, -product));
            result.term(product);
        } }
        result
    }
    fn zero(&self) -> bool { self.0.is_empty() }
}

fn exact_line_concurrence(curves: [Curve; 3]) -> bool {
    let magnitude = curves.iter().flat_map(|curve| curve.points[..2].iter())
        .flat_map(|point| [point.x.abs(), point.y.abs()]).fold(0.0, f64::max);
    if magnitude == 0.0 || !magnitude.is_finite() { return false; }
    let scale = 2.0_f64.powi(-(magnitude.log2().floor() as i32));
    if !scale.is_finite() || scale == 0.0 { return false; }
    let minimum = 2.0_f64.powi(-128);
    let mut lines = Vec::with_capacity(3);
    for curve in curves {
        let (p, q) = (curve.points[0] * scale, curve.points[1] * scale);
        if [p.x, p.y, q.x, q.y].iter().any(|value| !value.is_finite() || (*value != 0.0 && value.abs() < minimum)) { return false; }
        // Exact power-of-two scaling must not erase a nonzero input.
        if [curve.points[0].x, curve.points[0].y, curve.points[1].x, curve.points[1].y]
            .into_iter().zip([p.x, p.y, q.x, q.y]).any(|(before, after)| before != 0.0 && after == 0.0) { return false; }
        let e = Exact::number;
        lines.push([e(p.y).sub(e(q.y)), e(q.x).sub(e(p.x)), e(p.x).mul(&e(q.y)).sub(e(p.y).mul(&e(q.x)))]);
    }
    let (a, b, c) = (&lines[0], &lines[1], &lines[2]);
    let x = a[1].mul(&b[2]).sub(a[2].mul(&b[1]));
    let y = a[2].mul(&b[0]).sub(a[0].mul(&b[2]));
    let w = a[0].mul(&b[1]).sub(a[1].mul(&b[0]));
    !w.zero() && c[0].mul(&x).add(c[1].mul(&y)).add(c[2].mul(&w)).zero()
}

fn assemble(network: &VectorNetwork, edges: &[Edge], hits: Vec<Hit>, origin: Point, scale: f64, work: &mut Work) -> Result<Preparation, NetworkFaceError> {
    let mut cuts = BTreeMap::<VectorEdgeId, Vec<(f64, usize)>>::new();
    for (index, hit) in hits.iter().enumerate() {
        cuts.entry(hit.a).or_default().push((hit.ta, index));
        cuts.entry(hit.b).or_default().push((hit.tb, index));
    }
    let mut parents: Vec<_> = (0..hits.len()).collect();
    let mut endpoint_events = BTreeMap::new();
    for (&edge, points) in &mut cuts {
        points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for pair in points.windows(2) {
            work.take()?;
            if (pair[0].0 - pair[1].0).abs() <= PARAMETER_ROUND_OFF {
                // A tolerance bounds numerical uncertainty; it must not weld
                // distinct roots. Three nearby pairwise crossings can bound
                // a real face even though all their coordinates are close.
                // Final embedding certification cannot detect that face once
                // it has already been collapsed into a common junction.
                if pair[0].1 != pair[1].1 && !same_intersection(network, edges, edge, hits[pair[0].1], hits[pair[1].1], work)? {
                    return Err(NetworkFaceError::UnsplitIntersection(edge, edge));
                }
                join(&mut parents, pair[0].1, pair[1].1);
            }
        }
        let edge = &network.edges[&edge];
        for &(t, index) in points.iter() {
            let endpoint = if t == 0.0 { Some(edge.start) } else if t == 1.0 { Some(edge.end) } else { None };
            if let Some(vertex) = endpoint {
                if let Some(previous) = endpoint_events.insert(vertex, index) { join(&mut parents, previous, index); }
            }
        }
    }
    let mut groups = BTreeMap::<usize, BTreeSet<VectorVertexId>>::new();
    for index in 0..hits.len() { groups.entry(root(&mut parents, index)).or_default(); }
    for (vertex, event) in endpoint_events { groups.entry(root(&mut parents, event)).or_default().insert(vertex); }
    for points in cuts.values_mut() {
        for (_, index) in points.iter_mut() { *index = root(&mut parents, *index); }
        points.dedup_by(|a, b| {
            if a.1 == b.1 && (a.0 - b.0).abs() <= PARAMETER_ROUND_OFF {
                // These events were independently proved to name one root.
                // Retain an authored endpoint rather than a nearby estimate.
                if a.0 == 0.0 || a.0 == 1.0 { b.0 = a.0; }
                true
            } else { false }
        });
    }
    let added_edges: usize = cuts.values().map(|points| points.iter().filter(|(t, _)| *t > 0.0 && *t < 1.0).count()).sum();
    if network.edges.len().checked_add(added_edges).is_none_or(|count| count > MAX_EDGES) { return Err(NetworkFaceError::ComplexityLimit); }
    let added_vertices = groups.values().filter(|vertices| vertices.is_empty()).count();
    let (mut next, end) = network.reserve_ids((added_edges + added_vertices) as u64).ok_or(NetworkFaceError::IdExhausted)?;
    let mut prepared = geometry(network);
    let mut group_vertices = BTreeMap::new();
    let mut welded = BTreeMap::new();
    for (&index, vertices) in &groups {
        let vertex = if let Some(&vertex) = vertices.first() {
            let position = network.vertices[&vertex].position;
            // Distinct existing coordinates are never snapped together.
            if vertices.iter().any(|id| network.vertices[id].position != position) {
                return Err(NetworkFaceError::UnsplitIntersection(hits[index].a, hits[index].b));
            }
            for &other in vertices {
                welded.insert(other, vertex);
                if other != vertex { prepared.vertices.remove(&other); }
            }
            vertex
        } else {
            let hit = hits[index];
            let a = network.edge_curve(hit.a).unwrap().evaluate(hit.ta);
            let b = network.edge_curve(hit.b).unwrap().evaluate(hit.tb);
            let position = a.lerp(b, 0.5);
            if !finite(position) || !near((position - origin) * (1.0 / scale), hit.point) {
                return Err(NetworkFaceError::UnsplitIntersection(hit.a, hit.b));
            }
            let vertex = next; next += 1;
            prepared.vertices.insert(vertex, NetworkVertex { position, mode: AnchorMode::Corner });
            vertex
        };
        prepared.vertices.get_mut(&vertex).unwrap().mode = AnchorMode::Corner;
        group_vertices.insert(index, vertex);
    }
    for (index, hit) in hits.iter().enumerate() {
        let vertex = group_vertices[&root(&mut parents, index)];
        let position = (prepared.vertices[&vertex].position - origin) * (1.0 / scale);
        if !near(position, hit.point) { return Err(NetworkFaceError::UnsplitIntersection(hit.a, hit.b)); }
    }
    let mut replacements = BTreeMap::new();
    for edge in edges {
        let original = &network.edges[&edge.id];
        let start = welded.get(&original.start).copied().unwrap_or(original.start);
        let end_vertex = welded.get(&original.end).copied().unwrap_or(original.end);
        let interior: Vec<_> = cuts.get(&edge.id).into_iter().flatten().copied().filter(|(t, _)| *t > 0.0 && *t < 1.0).collect();
        let mut curve = network.edge_curve(edge.id).unwrap();
        let mut previous = 0.0;
        let mut from = start;
        let mut ids = Vec::with_capacity(interior.len() + 1);
        for (index, (t, to)) in interior.iter().map(|&(t, group)| (t, group_vertices[&group]))
            .chain(std::iter::once((1.0, end_vertex))).enumerate()
        {
            work.take()?;
            let part = if t == 1.0 { curve } else {
                let (part, rest) = split(curve, (t - previous) / (1.0 - previous));
                curve = rest; part
            };
            let id = if index == 0 { edge.id } else { let id = next; next += 1; id };
            let curve = if part.degree == 1 { NetworkCurve::Line }
                else { NetworkCurve::Cubic { control1: part.points[1], control2: part.points[2] } };
            prepared.edges.insert(id, NetworkEdge { start: from, end: to, curve, stroke: original.stroke });
            ids.push(id); from = to; previous = t;
        }
        if ids.len() > 1 { replacements.insert(edge.id, ids); }
    }
    debug_assert_eq!(next, end);
    prepared.next_id = end;
    // Subdivision changes handle lengths; preserve Smooth where valid and
    // release only constraints made invalid by the new junction topology.
    let adjacency = prepared.adjacency(true);
    for &vertex in network.vertices.keys() {
        let Some(mode) = prepared.vertices.get(&vertex).map(|v| v.mode) else { continue; };
        if mode != AnchorMode::Corner && !prepared.vertex_mode_is_valid(vertex, &adjacency[&vertex], mode) {
            let mode = if prepared.vertex_mode_is_valid(vertex, &adjacency[&vertex], AnchorMode::Smooth) {
                AnchorMode::Smooth
            } else { AnchorMode::Corner };
            prepared.vertices.get_mut(&vertex).unwrap().mode = mode;
        }
    }
    Ok(Preparation { original: geometry(network), prepared, replacements })
}

#[cfg(test)]
#[path = "vector_planarize_tests.rs"]
mod tests;
