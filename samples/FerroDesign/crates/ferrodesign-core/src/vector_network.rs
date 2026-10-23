//! Editable vector networks with stable vertices, edges and fill regions.
//!
//! The graph is authoritative. Rendering/export receive derived SVG-style
//! paths: stroke trails contain each stroked edge once, while explicit region
//! boundaries determine fill geometry. No graphics backend is involved.

use super::{
    distance_to_rect, endpoint, finite, interpolate, unit,
    unit_polynomial_roots, AnchorMode, Curve, Node, NodeKind, PathSegment, Point, Rect,
    Topology, Transform, VectorPath, GEOMETRY_EPSILON,
};
use crate::{Editor, FillRule, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[path = "vector_faces.rs"]
mod faces;
pub use faces::{NetworkFaceError, NetworkFaceMap};

#[path = "vector_planarize.rs"]
mod planarize;

pub type VectorVertexId = u64;
pub type VectorEdgeId = u64;
pub type VectorRegionId = u64;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkVertex {
    pub position: Point,
    pub mode: AnchorMode,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NetworkCurve {
    #[default]
    Line,
    Cubic { control1: Point, control2: Point },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkEdge {
    pub start: VectorVertexId,
    pub end: VectorVertexId,
    pub curve: NetworkCurve,
    /// False is useful for an open SVG contour's implicit fill closing edge.
    /// It contributes to explicit regions but does not add a painted stroke.
    #[serde(default = "default_stroke")]
    pub stroke: bool,
}

fn default_stroke() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NetworkEdgeRef {
    pub edge: VectorEdgeId,
    #[serde(default)]
    pub reversed: bool,
}

impl NetworkEdgeRef {
    pub const fn forward(edge: VectorEdgeId) -> Self {
        Self { edge, reversed: false }
    }

    pub const fn reverse(edge: VectorEdgeId) -> Self {
        Self { edge, reversed: true }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkRegion {
    /// Every boundary is an oriented, continuous, closed walk. Multiple walks
    /// describe holes or disconnected pieces according to the network fill rule.
    pub contours: Vec<Vec<NetworkEdgeRef>>,
    /// None participates in the legacy compound layer-fill geometry. An
    /// explicit stack replaces inherited paint inside this region, including
    /// an empty stack which cuts a transparent hole. Later stable region IDs
    /// replace earlier explicit regions where their interiors overlap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fills: Option<Vec<crate::Fill>>,
}

impl NetworkRegion {
    pub fn effective_fills<'a>(&'a self, inherited: &'a [crate::Fill]) -> &'a [crate::Fill] {
        self.fills.as_deref().unwrap_or(inherited)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VectorNetwork {
    pub vertices: BTreeMap<VectorVertexId, NetworkVertex>,
    pub edges: BTreeMap<VectorEdgeId, NetworkEdge>,
    pub regions: BTreeMap<VectorRegionId, NetworkRegion>,
    pub fill_rule: FillRule,
    /// IDs share a monotonically increasing space and are never renumbered or
    /// reused after deletion. The counter is serialized for durable undo/import.
    pub next_id: u64,
}

impl Default for VectorNetwork {
    fn default() -> Self {
        Self {
            vertices: BTreeMap::new(),
            edges: BTreeMap::new(),
            regions: BTreeMap::new(),
            fill_rule: FillRule::default(),
            next_id: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum NetworkPointId {
    Vertex(VectorVertexId),
    Control1(VectorEdgeId),
    Control2(VectorEdgeId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetworkEditPoint {
    pub id: NetworkPointId,
    pub position: Point,
    pub anchor: Point,
    pub vertex: VectorVertexId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetworkSegmentHit {
    pub edge: VectorEdgeId,
    pub t: f64,
    pub position: Point,
    pub distance: f64,
}

impl VectorNetwork {
    fn reserve_ids(&self, count: u64) -> Option<(u64, u64)> {
        let next = self.next_id.checked_add(count)?;
        if self.next_id == 0
            || (self.next_id..next).any(|id| {
                self.vertices.contains_key(&id)
                    || self.edges.contains_key(&id)
                    || self.regions.contains_key(&id)
            })
        {
            return None;
        }
        Some((self.next_id, next))
    }

    pub fn add_vertex(&mut self, position: Point) -> Option<VectorVertexId> {
        if !finite(position) {
            return None;
        }
        let (id, next) = self.reserve_ids(1)?;
        self.vertices.insert(id, NetworkVertex { position, mode: AnchorMode::Corner });
        self.next_id = next;
        Some(id)
    }

    pub fn connect(&mut self, start: VectorVertexId, end: VectorVertexId) -> Option<VectorEdgeId> {
        self.connect_curve(start, end, NetworkCurve::Line)
    }

    /// Adds a branch or connection. A changed endpoint topology releases its
    /// tangent mode to Corner while retaining every existing control position.
    /// Parallel curves and self-loops are supported; identical edges are ignored.
    pub fn connect_curve(
        &mut self,
        start: VectorVertexId,
        end: VectorVertexId,
        curve: NetworkCurve,
    ) -> Option<VectorEdgeId> {
        if !self.vertices.contains_key(&start)
            || !self.vertices.contains_key(&end)
            || !curve_is_finite(&curve)
            || self.edges.values().any(|edge| {
                edge.stroke
                    && ((edge.start == start && edge.end == end && edge.curve == curve)
                        || (edge.start == end
                            && edge.end == start
                            && reverse_curve(&edge.curve) == curve))
            })
        {
            return None;
        }
        let (id, next) = self.reserve_ids(1)?;
        self.edges.insert(id, NetworkEdge { start, end, curve, stroke: true });
        self.next_id = next;
        self.vertices.get_mut(&start)?.mode = AnchorMode::Corner;
        self.vertices.get_mut(&end)?.mode = AnchorMode::Corner;
        Some(id)
    }

    pub fn add_region(&mut self, contours: Vec<Vec<NetworkEdgeRef>>) -> Option<VectorRegionId> {
        let region = NetworkRegion { contours, fills: None };
        if !self.region_is_valid(&region) {
            return None;
        }
        let key = region_key(&region);
        if self.regions.values().any(|existing| region_key(existing) == key) {
            return None;
        }
        let (id, next) = self.reserve_ids(1)?;
        self.regions.insert(id, region);
        self.next_id = next;
        Some(id)
    }

    pub fn remove_region(&mut self, id: VectorRegionId) -> bool {
        self.regions.remove(&id).is_some()
    }

    /// Removing a boundary edge removes regions which used it. Unrelated
    /// regions, shared vertices, edge IDs and open branches remain intact.
    pub fn remove_edge(&mut self, id: VectorEdgeId) -> bool {
        if self.edges.remove(&id).is_none() {
            return false;
        }
        self.regions.retain(|_, region| {
            !region.contours.iter().flatten().any(|reference| reference.edge == id)
        });
        true
    }

    pub fn remove_vertex(&mut self, id: VectorVertexId) -> bool {
        if !self.vertices.contains_key(&id) {
            return false;
        }
        let removed: BTreeSet<_> = self.edges.iter()
            .filter(|(_, edge)| edge.start == id || edge.end == id)
            .map(|(&edge, _)| edge)
            .collect();
        self.edges.retain(|edge, _| !removed.contains(edge));
        self.regions.retain(|_, region| {
            !region.contours.iter().flatten().any(|reference| removed.contains(&reference.edge))
        });
        self.vertices.remove(&id);
        true
    }

    fn adjacency(&self, stroked_only: bool) -> BTreeMap<VectorVertexId, Vec<NetworkEdgeRef>> {
        let mut result: BTreeMap<_, Vec<_>> = self.vertices.keys()
            .map(|&vertex| (vertex, Vec::new()))
            .collect();
        for (&id, edge) in &self.edges {
            if stroked_only && !edge.stroke {
                continue;
            }
            if let Some(edges) = result.get_mut(&edge.start) {
                edges.push(NetworkEdgeRef::forward(id));
            }
            if let Some(edges) = result.get_mut(&edge.end) {
                edges.push(NetworkEdgeRef::reverse(id));
            }
        }
        result
    }

    fn edge_curve(&self, id: VectorEdgeId) -> Option<Curve> {
        let edge = self.edges.get(&id)?;
        let start = self.vertices.get(&edge.start)?.position;
        let end = self.vertices.get(&edge.end)?.position;
        match edge.curve {
            NetworkCurve::Line => Some(Curve::line(start, end)),
            NetworkCurve::Cubic { control1, control2 } => Curve::from_segment(
                start,
                &PathSegment::CubicTo { control1, control2, point: end },
            ),
        }
    }

    fn oriented_edge(&self, reference: NetworkEdgeRef) -> Option<(VectorVertexId, VectorVertexId, PathSegment)> {
        let edge = self.edges.get(&reference.edge)?;
        let (start, end) = if reference.reversed { (edge.end, edge.start) } else { (edge.start, edge.end) };
        let point = self.vertices.get(&end)?.position;
        let command = match (&edge.curve, reference.reversed) {
            (NetworkCurve::Line, _) => PathSegment::LineTo { point },
            (NetworkCurve::Cubic { control1, control2 }, false) => PathSegment::CubicTo {
                control1: *control1, control2: *control2, point,
            },
            (NetworkCurve::Cubic { control1, control2 }, true) => PathSegment::CubicTo {
                control1: *control2, control2: *control1, point,
            },
        };
        self.vertices.get(&start)?;
        Some((start, end, command))
    }

    fn region_is_valid(&self, region: &NetworkRegion) -> bool {
        if region.contours.is_empty() {
            return false;
        }
        let mut seen = BTreeSet::new();
        region.contours.iter().all(|contour| {
            let Some(first) = contour.first().and_then(|r| self.oriented_edge(*r)) else {
                return false;
            };
            let origin = first.0;
            let mut current = origin;
            for reference in contour {
                let Some((start, end, _)) = self.oriented_edge(*reference) else {
                    return false;
                };
                if start != current || !seen.insert(reference.edge) {
                    return false;
                }
                current = end;
            }
            current == origin
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        for id in self.vertices.keys().chain(self.edges.keys()).chain(self.regions.keys()) {
            if *id == 0 || !ids.insert(*id) || *id >= self.next_id {
                return Err("Network IDs must be unique, nonzero and below next_id".into());
            }
        }
        if self.next_id == 0 || self.vertices.values().any(|vertex| !finite(vertex.position)) {
            return Err("Invalid network vertex geometry or ID counter".into());
        }
        for edge in self.edges.values() {
            if !self.vertices.contains_key(&edge.start)
                || !self.vertices.contains_key(&edge.end)
                || !curve_is_finite(&edge.curve)
            {
                return Err("Network edge has a missing vertex or invalid control".into());
            }
        }
        let adjacency = self.adjacency(true);
        for (&id, vertex) in &self.vertices {
            if vertex.mode != AnchorMode::Corner
                && !self.vertex_mode_is_valid(id, &adjacency[&id], vertex.mode)
            {
                return Err("A network tangent constraint requires aligned cubic stroke handles".into());
            }
        }
        if self.regions.values().any(|region| !self.region_is_valid(region)) {
            return Err("Network fill boundaries must be continuous closed walks of existing edges".into());
        }
        for region in self.regions.values() {
            if let Some(fills) = &region.fills { validate_region_fills(fills)?; }
        }
        Ok(())
    }

    fn stroke_refs(&self, vertex: VectorVertexId) -> Vec<NetworkEdgeRef> {
        let mut refs = Vec::with_capacity(2);
        for (&id, edge) in &self.edges {
            if !edge.stroke {
                continue;
            }
            if edge.start == vertex {
                refs.push(NetworkEdgeRef::forward(id));
            }
            if edge.end == vertex {
                refs.push(NetworkEdgeRef::reverse(id));
            }
        }
        refs
    }

    fn control_at(&self, reference: NetworkEdgeRef) -> Option<(NetworkPointId, Point)> {
        match self.edges.get(&reference.edge)?.curve {
            NetworkCurve::Cubic { control1, control2 } => Some(if reference.reversed {
                (NetworkPointId::Control2(reference.edge), control2)
            } else {
                (NetworkPointId::Control1(reference.edge), control1)
            }),
            NetworkCurve::Line => None,
        }
    }

    fn vertex_mode_is_valid(
        &self,
        vertex: VectorVertexId,
        refs: &[NetworkEdgeRef],
        mode: AnchorMode,
    ) -> bool {
        if mode == AnchorMode::Corner {
            return true;
        }
        if refs.len() > 2 {
            return false;
        }
        let Some(anchor) = self.vertices.get(&vertex) else {
            return false;
        };
        let handles: Option<Vec<_>> = refs.iter().map(|reference| self.control_at(*reference)).collect();
        let Some(handles) = handles else {
            return false;
        };
        handles.len() < 2
            || network_tangents_match(handles[0].1, handles[1].1, anchor.position, mode)
    }

    pub fn point_position(&self, id: NetworkPointId) -> Option<Point> {
        match id {
            NetworkPointId::Vertex(vertex) => self.vertices.get(&vertex).map(|v| v.position),
            NetworkPointId::Control1(edge) => self.control_at(NetworkEdgeRef::forward(edge)).map(|p| p.1),
            NetworkPointId::Control2(edge) => self.control_at(NetworkEdgeRef::reverse(edge)).map(|p| p.1),
        }
    }

    pub fn point_vertex(&self, id: NetworkPointId) -> Option<VectorVertexId> {
        match id {
            NetworkPointId::Vertex(vertex) => self.vertices.contains_key(&vertex).then_some(vertex),
            NetworkPointId::Control1(edge) => self.edges.get(&edge).map(|edge| edge.start),
            NetworkPointId::Control2(edge) => self.edges.get(&edge).map(|edge| edge.end),
        }
    }

    pub fn edit_points(&self) -> Vec<NetworkEditPoint> {
        let mut result = Vec::with_capacity(self.vertices.len() + self.edges.len() * 2);
        for (&vertex, value) in &self.vertices {
            result.push(NetworkEditPoint {
                id: NetworkPointId::Vertex(vertex),
                position: value.position,
                anchor: value.position,
                vertex,
            });
        }
        for (&edge, value) in &self.edges {
            let NetworkCurve::Cubic { control1, control2 } = value.curve else {
                continue;
            };
            for (id, position, vertex) in [
                (NetworkPointId::Control1(edge), control1, value.start),
                (NetworkPointId::Control2(edge), control2, value.end),
            ] {
                if let Some(anchor) = self.vertices.get(&vertex) {
                    result.push(NetworkEditPoint { id, position, anchor: anchor.position, vertex });
                }
            }
        }
        result
    }

    fn assign_control(&mut self, id: NetworkPointId, position: Point) -> bool {
        let (edge, first) = match id {
            NetworkPointId::Control1(edge) => (edge, true),
            NetworkPointId::Control2(edge) => (edge, false),
            NetworkPointId::Vertex(_) => return false,
        };
        match self.edges.get_mut(&edge).map(|edge| &mut edge.curve) {
            Some(NetworkCurve::Cubic { control1, control2 }) => {
                if first { *control1 = position; } else { *control2 = position; }
                true
            }
            _ => false,
        }
    }

    /// Shared-vertex dragging translates every attached handle once, including
    /// both handles of self-loops and any implicit fill boundary controls.
    /// Handle dragging respects the endpoint's degree-two stroke constraint.
    pub fn move_edit_point(&mut self, id: NetworkPointId, position: Point) -> bool {
        let Some(previous) = self.point_position(id) else {
            return false;
        };
        if !finite(position) || position == previous {
            return false;
        }
        if let NetworkPointId::Vertex(vertex) = id {
            let delta = position - previous;
            if !finite(delta) {
                return false;
            }
            let mut updates = Vec::new();
            for (&edge, value) in &self.edges {
                if let NetworkCurve::Cubic { control1, control2 } = value.curve {
                    if value.start == vertex {
                        updates.push((NetworkPointId::Control1(edge), control1 + delta));
                    }
                    if value.end == vertex {
                        updates.push((NetworkPointId::Control2(edge), control2 + delta));
                    }
                }
            }
            if updates.iter().any(|(_, position)| !finite(*position)) {
                return false;
            }
            self.vertices.get_mut(&vertex).unwrap().position = position;
            for (control, position) in updates {
                self.assign_control(control, position);
            }
            return true;
        }
        let Some(vertex) = self.point_vertex(id) else {
            return false;
        };
        let Some(anchor) = self.vertices.get(&vertex) else {
            return false;
        };
        let edge_id = match id {
            NetworkPointId::Control1(edge) | NetworkPointId::Control2(edge) => edge,
            _ => return false,
        };
        let mut opposite = None;
        if self.edges[&edge_id].stroke && anchor.mode != AnchorMode::Corner {
            let refs = self.stroke_refs(vertex);
            if refs.len() > 2 {
                return false;
            }
            let handles: Option<Vec<_>> = refs.iter().map(|reference| self.control_at(*reference)).collect();
            let Some(handles) = handles else {
                return false;
            };
            if let Some(&(other, old_position)) = handles.iter().find(|(other, _)| *other != id) {
                let direction = position - anchor.position;
                let next = if anchor.mode == AnchorMode::Symmetric {
                    anchor.position - direction
                } else if let Some(direction) = unit(direction) {
                    anchor.position - direction * old_position.distance(anchor.position)
                } else {
                    old_position
                };
                if !finite(next) {
                    return false;
                }
                opposite = Some((other, next));
            }
        }
        self.assign_control(id, position);
        if let Some((other, position)) = opposite {
            self.assign_control(other, position);
        }
        true
    }

    /// Corner releases the linkage without retracting existing network handles.
    /// Smooth and Symmetric modes are available at vertices with at most two
    /// stroked incident edges; branch vertices retain independent tangents.
    pub fn set_vertex_mode(&mut self, vertex: VectorVertexId, mode: AnchorMode) -> bool {
        let Some(value) = self.vertices.get(&vertex) else {
            return false;
        };
        let refs = self.stroke_refs(vertex);
        if value.mode == mode && self.vertex_mode_is_valid(vertex, &refs, mode) {
            return false;
        }
        if mode == AnchorMode::Corner {
            self.vertices.get_mut(&vertex).unwrap().mode = mode;
            return true;
        }
        if refs.len() > 2 {
            return false;
        }
        let anchor = value.position;
        let mut candidate = self.clone();
        for reference in &refs {
            let Some(curve) = candidate.edge_curve(reference.edge) else {
                return false;
            };
            let (control1, control2, _) = curve.as_cubic();
            candidate.edges.get_mut(&reference.edge).unwrap().curve = NetworkCurve::Cubic {
                control1,
                control2,
            };
        }
        let handles: Vec<_> = refs.iter().filter_map(|reference| candidate.control_at(*reference)).collect();
        let mut direction = None;
        let mut lengths = Vec::with_capacity(handles.len());
        for (index, (_, position)) in handles.iter().enumerate() {
            let length = position.distance(anchor);
            let reference = refs[index];
            let Some((_, other, _)) = candidate.oriented_edge(reference) else {
                return false;
            };
            let other_position = candidate.vertices[&other].position;
            lengths.push(if length <= GEOMETRY_EPSILON {
                anchor.distance(other_position) / 3.0
            } else { length });
            if direction.is_none() {
                let sign = if index == 0 { 1.0 } else { -1.0 };
                direction = unit((*position - anchor) * sign)
                    .or_else(|| unit((other_position - anchor) * sign));
            }
        }
        let direction = direction.unwrap_or(Point::new(1.0, 0.0));
        if mode == AnchorMode::Symmetric && lengths.len() == 2 {
            let average = lengths[0] * 0.5 + lengths[1] * 0.5;
            lengths[0] = average;
            lengths[1] = average;
        }
        for (index, ((id, _), length)) in handles.iter().zip(lengths).enumerate() {
            let sign = if index == 0 { 1.0 } else { -1.0 };
            let position = anchor + direction * (length * sign);
            if !finite(position) {
                return false;
            }
            candidate.assign_control(*id, position);
        }
        candidate.vertices.get_mut(&vertex).unwrap().mode = mode;
        *self = candidate;
        true
    }

    /// Welds a vertex into another while retaining all edge and region IDs.
    /// Source handles move with their endpoint; the resulting junction uses
    /// independent tangents so branches never acquire an ambiguous linkage.
    pub fn merge_vertices(&mut self, source: VectorVertexId, target: VectorVertexId) -> bool {
        if source == target {
            return false;
        }
        let (Some(source_value), Some(target_value)) = (self.vertices.get(&source), self.vertices.get(&target)) else {
            return false;
        };
        let mut candidate = self.clone();
        if source_value.position != target_value.position
            && !candidate.move_edit_point(NetworkPointId::Vertex(source), target_value.position)
        {
            return false;
        }
        for edge in candidate.edges.values_mut() {
            if edge.start == source { edge.start = target; }
            if edge.end == source { edge.end = target; }
        }
        candidate.vertices.remove(&source);
        candidate.vertices.get_mut(&target).unwrap().mode = AnchorMode::Corner;
        if candidate.validate().is_err() {
            return false;
        }
        *self = candidate;
        true
    }

    pub fn transformed(&self, transform: Transform) -> Self {
        let mut result = self.clone();
        for vertex in result.vertices.values_mut() {
            vertex.position = transform.apply(vertex.position);
        }
        for edge in result.edges.values_mut() {
            if let NetworkCurve::Cubic { control1, control2 } = &mut edge.curve {
                *control1 = transform.apply(*control1);
                *control2 = transform.apply(*control2);
            }
        }
        result
    }
}

impl VectorNetwork {
    /// Converts SVG-style contours into shared-vertex topology. Explicit closed
    /// endpoint aliases are welded using the path editor's existing tolerance;
    /// coincident points in unrelated contours stay separate. Quadratic curves
    /// are degree elevated exactly. An open contour receives an unstroked fill
    /// closing edge, preserving its fill without introducing a visible stroke.
    pub fn from_path(path: &VectorPath) -> Option<Self> {
        let topology = Topology::new(path)?;
        if !path.validate_anchor_modes() {
            return None;
        }
        let mut result = Self { fill_rule: path.fill_rule, ..Self::default() };
        let mut vertices = BTreeMap::new();
        for (index, alias) in topology.aliases.iter().enumerate() {
            if *alias == Some(index) {
                let id = result.add_vertex(endpoint(&path.segments[index])?)?;
                vertices.insert(index, id);
            }
        }
        let mut implicit_origin = None;
        let mut commands = BTreeMap::new();
        for edge in &topology.edges {
            let start = match edge.start_anchor {
                Some(anchor) => *vertices.get(&anchor)?,
                None => match implicit_origin {
                    Some(id) => id,
                    None => {
                        let id = result.add_vertex(edge.curve.points[0])?;
                        implicit_origin = Some(id);
                        id
                    }
                },
            };
            let end = *vertices.get(&edge.end_anchor)?;
            if matches!(path.segments[edge.segment], PathSegment::Close) && start == end {
                continue;
            }
            let curve = if edge.curve.degree == 1 {
                NetworkCurve::Line
            } else {
                let (control1, control2, _) = edge.curve.as_cubic();
                NetworkCurve::Cubic { control1, control2 }
            };
            let id = result.insert_edge(start, end, curve, true)?;
            commands.insert(edge.segment, id);
        }
        let mut walk = Vec::new();
        for (index, command) in path.segments.iter().enumerate() {
            if matches!(command, PathSegment::MoveTo { .. }) {
                result.finish_import_contour(&mut walk)?;
            }
            if let Some(&edge) = commands.get(&index) {
                walk.push(NetworkEdgeRef::forward(edge));
            }
            if matches!(command, PathSegment::Close) {
                result.finish_import_contour(&mut walk)?;
            }
        }
        result.finish_import_contour(&mut walk)?;
        for (&anchor, &mode) in &path.anchor_modes {
            let vertex = *vertices.get(&topology.canonical(anchor)?)?;
            result.vertices.get_mut(&vertex)?.mode = mode;
        }
        result.validate().ok()?;
        Some(result)
    }

    fn insert_edge(
        &mut self,
        start: VectorVertexId,
        end: VectorVertexId,
        curve: NetworkCurve,
        stroke: bool,
    ) -> Option<VectorEdgeId> {
        let (id, next) = self.reserve_ids(1)?;
        self.edges.insert(id, NetworkEdge { start, end, curve, stroke });
        self.next_id = next;
        Some(id)
    }

    fn finish_import_contour(&mut self, walk: &mut Vec<NetworkEdgeRef>) -> Option<()> {
        let Some(first) = walk.first().copied() else {
            return Some(());
        };
        let origin = self.oriented_edge(first)?.0;
        let end = self.oriented_edge(*walk.last()?)?.1;
        if end != origin {
            let edge = self.insert_edge(end, origin, NetworkCurve::Line, false)?;
            walk.push(NetworkEdgeRef::forward(edge));
        }
        self.add_region(vec![std::mem::take(walk)])?;
        Some(())
    }

    /// One native path containing deterministic maximal stroke trails. Each
    /// stroked edge appears once; degree-two chains share joins, branch vertices
    /// terminate trails, and closed degree-two cycles retain their closing join.
    pub fn stroke_path(&self) -> VectorPath {
        let adjacency = self.adjacency(true);
        let mut result = VectorPath { fill_rule: self.fill_rule, ..VectorPath::default() };
        let mut visited = BTreeSet::new();
        for (&vertex, refs) in &adjacency {
            if refs.is_empty() {
                result.segments.push(PathSegment::MoveTo {
                    point: self.vertices[&vertex].position,
                });
            } else if refs.len() != 2 {
                for &reference in refs {
                    self.append_stroke_trail(&mut result, &adjacency, &mut visited, reference);
                }
            }
        }
        for (&edge, value) in &self.edges {
            if value.stroke && !visited.contains(&edge) {
                self.append_stroke_trail(
                    &mut result, &adjacency, &mut visited, NetworkEdgeRef::forward(edge),
                );
            }
        }
        result
    }

    fn append_stroke_trail(
        &self,
        path: &mut VectorPath,
        adjacency: &BTreeMap<VectorVertexId, Vec<NetworkEdgeRef>>,
        visited: &mut BTreeSet<VectorEdgeId>,
        mut reference: NetworkEdgeRef,
    ) {
        if visited.contains(&reference.edge) {
            return;
        }
        let Some((origin, _, _)) = self.oriented_edge(reference) else {
            return;
        };
        path.segments.push(PathSegment::MoveTo { point: self.vertices[&origin].position });
        loop {
            let Some((_, end, command)) = self.oriented_edge(reference) else {
                break;
            };
            if !visited.insert(reference.edge) {
                break;
            }
            path.segments.push(command);
            let Some(refs) = adjacency.get(&end) else {
                break;
            };
            if refs.len() != 2 {
                break;
            }
            if end == origin {
                path.segments.push(PathSegment::Close);
                break;
            }
            let Some(next) = refs.iter().find(|next| !visited.contains(&next.edge)) else {
                break;
            };
            reference = *next;
        }
    }

    /// Explicit fill regions only. Region boundaries share the document fill
    /// rule, so opposite winding or EvenOdd contours can describe holes. Open
    /// branches which do not belong to a region never acquire an implicit fill.
    pub fn fill_path(&self) -> VectorPath {
        self.regions_path(self.regions.values())
    }

    /// Legacy compound fill geometry before explicit region replacement. Keep
    /// the common fill rule across all inherited contours, including EvenOdd
    /// cancellation; drawing every inherited region independently changes it.
    pub fn inherited_fill_path(&self) -> VectorPath {
        self.regions_path(self.regions.values().filter(|region| region.fills.is_none()))
    }

    /// One region's exact authored commands, without the rest of the network.
    pub fn region_fill_path(&self, id: VectorRegionId) -> Option<VectorPath> {
        let region = self.regions.get(&id)?;
        self.region_is_valid(region).then(|| self.regions_path(std::iter::once(region)))
    }

    fn regions_path<'a>(&self, regions: impl IntoIterator<Item = &'a NetworkRegion>) -> VectorPath {
        let mut result = VectorPath { fill_rule: self.fill_rule, ..VectorPath::default() };
        for region in regions {
            if !self.region_is_valid(region) {
                continue;
            }
            for contour in &region.contours {
                let (start, _, _) = self.oriented_edge(contour[0]).unwrap();
                result.segments.push(PathSegment::MoveTo { point: self.vertices[&start].position });
                result.segments.extend(contour.iter().filter_map(|reference| {
                    self.oriented_edge(*reference).map(|(_, _, command)| command)
                }));
                result.segments.push(PathSegment::Close);
            }
        }
        result
    }

    /// Analytic Bézier bounds, including isolated editable vertices and hidden
    /// fill boundary edges. No flattened or derived path buffer is allocated.
    pub fn tight_bounds(&self) -> Option<Rect> {
        let mut bounds: Option<Rect> = None;
        let mut include = |point: Point| {
            let rect = Rect::new(point.x, point.y, 0.0, 0.0);
            bounds = Some(bounds.map_or(rect, |previous| previous.union(rect)));
        };
        for vertex in self.vertices.values() {
            if !finite(vertex.position) {
                return None;
            }
            include(vertex.position);
        }
        for &edge in self.edges.keys() {
            let curve = self.edge_curve(edge)?;
            if curve.points[..=curve.degree].iter().any(|point| !finite(*point)) {
                return None;
            }
            let power = curve.power_coefficients();
            for axis in [false, true] {
                let mut derivative = [0.0; 3];
                for i in 1..=curve.degree {
                    derivative[i - 1] = if axis { power[i].y } else { power[i].x } * i as f64;
                }
                for t in unit_polynomial_roots(&derivative[..curve.degree]) {
                    include(curve.evaluate(t));
                }
            }
        }
        bounds.filter(|bounds| bounds.is_finite())
    }

    pub fn contains_fill(&self, point: Point) -> bool {
        finite(point) && self.fill_path().contains(point)
    }

    /// Exact nearest point on all authored edges, including a region's hidden
    /// closing edges, for topology editing. Use nearest_stroke for painted hits.
    pub fn nearest_segment(&self, point: Point) -> Option<NetworkSegmentHit> {
        self.nearest_edge(point, false, None)
    }

    pub fn nearest_stroke(&self, point: Point) -> Option<NetworkSegmentHit> {
        self.nearest_edge(point, true, None)
    }

    /// Query in world or screen space without cloning/translating the graph.
    /// This preserves exact picking under nonuniform scale and shear; positions
    /// and distances use the supplied transform's space, while edge IDs/t remain
    /// suitable for local graph editing and subdivision.
    pub fn nearest_segment_transformed(&self, point: Point, transform: Transform) -> Option<NetworkSegmentHit> {
        self.nearest_edge(point, false, Some(transform))
    }

    fn nearest_edge(&self, point: Point, stroked_only: bool, transform: Option<Transform>) -> Option<NetworkSegmentHit> {
        if !finite(point) {
            return None;
        }
        let mut nearest: Option<NetworkSegmentHit> = None;
        for (&edge, value) in &self.edges {
            if stroked_only && !value.stroke {
                continue;
            }
            let Some(mut curve) = self.edge_curve(edge) else {
                continue;
            };
            if let Some(transform) = transform {
                curve.points = curve.points.map(|point| transform.apply(point));
            }
            if curve.points[..=curve.degree].iter().any(|point| !finite(*point)) {
                continue;
            }
            if nearest.is_some_and(|best| distance_to_rect(point, curve.hull_bounds()) > best.distance) {
                continue;
            }
            let (t, position, distance) = curve.nearest(point);
            if distance.is_finite() && nearest.is_none_or(|best| distance < best.distance) {
                nearest = Some(NetworkSegmentHit { edge, t, position, distance });
            }
        }
        nearest
    }

    pub fn hit_test(&self, point: Point, stroke_tolerance: f64, include_fill: bool) -> bool {
        (include_fill && self.contains_fill(point))
            || (stroke_tolerance.is_finite() && stroke_tolerance >= 0.0
                && self.nearest_stroke(point).is_some_and(|hit| hit.distance <= stroke_tolerance))
    }

    /// Pulls the point at `t` to the pointer with fixed endpoint vertices. The
    /// two cubic controls receive the minimum combined squared displacement;
    /// neighboring endpoint constraints propagate through their actual vertex
    /// ownership. Lines are exactly elevated before bending. A constrained
    /// self-loop is declined because its linked controls cannot generally meet
    /// an independent point constraint with its single anchor fixed.
    pub fn bend_edge(&mut self, edge: VectorEdgeId, t: f64, position: Point) -> bool {
        if !finite(position) || !t.is_finite() || !(0.0 < t && t < 1.0) {
            return false;
        }
        let Some(value) = self.edges.get(&edge) else {
            return false;
        };
        if value.start == value.end && value.stroke
            && self.vertices.get(&value.start).is_some_and(|vertex| vertex.mode != AnchorMode::Corner)
        {
            return false;
        }
        let Some(curve) = self.edge_curve(edge) else {
            return false;
        };
        let delta = position - curve.evaluate(t);
        if !finite(delta) || delta == Point::default() {
            return false;
        }
        let a = 3.0 * (1.0 - t) * (1.0 - t) * t;
        let b = 3.0 * (1.0 - t) * t * t;
        let denominator = a * a + b * b;
        if denominator <= f64::MIN_POSITIVE || !denominator.is_finite() {
            return false;
        }
        let (control1, control2, _) = curve.as_cubic();
        let first = control1 + delta * (a / denominator);
        let second = control2 + delta * (b / denominator);
        if !finite(first) || !finite(second) {
            return false;
        }
        let mut candidate = self.clone();
        candidate.edges.get_mut(&edge).unwrap().curve = NetworkCurve::Cubic { control1, control2 };
        if !candidate.move_edit_point(NetworkPointId::Control1(edge), first)
            || !candidate.move_edit_point(NetworkPointId::Control2(edge), second)
            || candidate.validate().is_err()
        {
            return false;
        }
        *self = candidate;
        true
    }

    /// Creates a region from an explicitly selected vertex cycle. Every pair
    /// must have exactly one connecting edge; ambiguous parallel edges require
    /// the edge-oriented add_region API. The final pair closes automatically.
    pub fn region_from_vertices(&mut self, vertices: &[VectorVertexId]) -> Option<VectorRegionId> {
        if vertices.len() < 2 {
            return None;
        }
        let mut contour = Vec::with_capacity(vertices.len());
        for (&start, &end) in vertices.iter().zip(vertices.iter().cycle().skip(1)) {
            let mut edges = self.edges.iter().filter_map(|(&id, edge)| {
                if edge.start == start && edge.end == end {
                    Some(NetworkEdgeRef::forward(id))
                } else if edge.start == end && edge.end == start {
                    Some(NetworkEdgeRef::reverse(id))
                } else {
                    None
                }
            });
            let edge = edges.next()?;
            if edges.next().is_some() {
                return None;
            }
            contour.push(edge);
        }
        self.add_region(vec![contour])
    }

    /// Exact subdivision keeps the first half's edge ID and inserts one vertex
    /// and one edge. Every forward/reverse region use is expanded consistently.
    /// A split curve vertex is Smooth; adjacent Symmetric constraints relax to
    /// Smooth only where keeping the exact shape changes relative handle length.
    pub fn split_edge(&mut self, edge: VectorEdgeId, t: f64) -> Option<VectorVertexId> {
        if !t.is_finite() || !(0.0 < t && t < 1.0) {
            return None;
        }
        let old = self.edges.get(&edge)?.clone();
        let curve = self.edge_curve(edge)?;
        let (vertex, next) = self.reserve_ids(2)?;
        let second = vertex + 1;
        let (position, first_curve, second_curve) = match old.curve {
            NetworkCurve::Line => (
                curve.evaluate(t), NetworkCurve::Line, NetworkCurve::Line,
            ),
            NetworkCurve::Cubic { control1, control2 } => {
                let a = interpolate(curve.points[0], control1, t);
                let b = interpolate(control1, control2, t);
                let c = interpolate(control2, curve.points[3], t);
                let d = interpolate(a, b, t);
                let e = interpolate(b, c, t);
                (
                    interpolate(d, e, t),
                    NetworkCurve::Cubic { control1: a, control2: d },
                    NetworkCurve::Cubic { control1: e, control2: c },
                )
            }
        };
        if !finite(position) || !curve_is_finite(&first_curve) || !curve_is_finite(&second_curve) {
            return None;
        }
        let mut candidate = self.clone();
        candidate.next_id = next;
        candidate.vertices.insert(vertex, NetworkVertex {
            position,
            mode: if curve.degree == 3 && old.stroke { AnchorMode::Smooth } else { AnchorMode::Corner },
        });
        candidate.edges.insert(edge, NetworkEdge {
            start: old.start, end: vertex, curve: first_curve, stroke: old.stroke,
        });
        candidate.edges.insert(second, NetworkEdge {
            start: vertex, end: old.end, curve: second_curve, stroke: old.stroke,
        });
        for region in candidate.regions.values_mut() {
            for contour in &mut region.contours {
                let mut expanded = Vec::with_capacity(contour.len() + 1);
                for &reference in contour.iter() {
                    if reference.edge != edge {
                        expanded.push(reference);
                    } else if reference.reversed {
                        expanded.push(NetworkEdgeRef::reverse(second));
                        expanded.push(NetworkEdgeRef::reverse(edge));
                    } else {
                        expanded.push(NetworkEdgeRef::forward(edge));
                        expanded.push(NetworkEdgeRef::forward(second));
                    }
                }
                *contour = expanded;
            }
        }
        for endpoint in [old.start, old.end] {
            if candidate.vertices[&endpoint].mode == AnchorMode::Symmetric
                && !candidate.vertex_mode_is_valid(endpoint, &candidate.stroke_refs(endpoint), AnchorMode::Symmetric)
            {
                candidate.vertices.get_mut(&endpoint)?.mode = AnchorMode::Smooth;
            }
        }
        candidate.validate().ok()?;
        *self = candidate;
        Some(vertex)
    }
}

/// Rebase graph geometry while keeping every vertex and control at its original
/// parent-space position, including under rotation, reflection and affine
/// residuals. Graph IDs and explicit region topology never change during rebasing.
pub fn normalize_network_bounds(node: &mut Node) -> bool {
    if node.kind != NodeKind::Vector || !node.path.segments.is_empty() {
        return false;
    }
    let Some(network) = &node.vector_network else {
        return false;
    };
    let Some(bounds) = network.tight_bounds() else {
        return network.vertices.is_empty() && network.validate().is_ok();
    };
    let width = bounds.width.max(1.0);
    let height = bounds.height.max(1.0);
    let center_delta = Point::new((width - node.width) * 0.5, (height - node.height) * 0.5);
    let rigid = Transform::rotation(node.rotation)
        * Transform::scale(
            if node.flip_x { -1.0 } else { 1.0 },
            if node.flip_y { -1.0 } else { 1.0 },
        );
    let origin = node.local_transform().apply_vector(Point::new(bounds.x, bounds.y));
    let new_position =
        Point::new(node.x, node.y) + origin + rigid.apply_vector(center_delta) - center_delta;
    if !finite(new_position) {
        return false;
    }
    let network = network.transformed(Transform::translation(-bounds.x, -bounds.y));
    if network.validate().is_err() {
        return false;
    }
    node.vector_network = Some(network);
    node.x = new_position.x;
    node.y = new_position.y;
    node.width = width;
    node.height = height;
    true
}

impl Editor {
    fn edit_vector_network<R>(
        &mut self,
        id: NodeId,
        label: &str,
        operation: impl FnOnce(&mut VectorNetwork) -> Option<R>,
    ) -> Option<R> {
        if self.document.effective_locked(id) {
            return None;
        }
        let mut node = self.document.nodes.get(&id)?.clone();
        if node.kind != NodeKind::Vector {
            return None;
        }
        let result = operation(node.vector_network.as_mut()?)?;
        if !normalize_network_bounds(&mut node) {
            return None;
        }
        self.edit_nodes(label, &[id], |editor| {
            editor.document.nodes.insert(id, node);
        });
        Some(result)
    }

    /// A reversible conversion to one authoritative graph. Existing paints and
    /// document transforms are retained; the legacy path is cleared. The node's
    /// layer identity stays unchanged, and undo restores its original path.
    pub fn convert_vector_to_network(&mut self, id: NodeId) -> bool {
        if self.document.effective_locked(id) {
            return false;
        }
        let Some(mut node) = self.document.nodes.get(&id).cloned() else {
            return false;
        };
        if node.kind != NodeKind::Vector || node.vector_network.is_some() {
            return false;
        }
        let Some(network) = VectorNetwork::from_path(&node.path) else {
            return false;
        };
        node.vector_network = Some(network);
        node.path = VectorPath::default();
        if !normalize_network_bounds(&mut node) {
            return false;
        }
        self.edit_nodes("Convert to vector network", &[id], |editor| {
            editor.document.nodes.insert(id, node);
        });
        true
    }

    /// Pointer-frequency edits use world positions so repeated bound rebasing
    /// cannot make subsequent updates jump. begin_node_transaction/commit wraps
    /// an entire gesture into one undo entry, as for ordinary path point edits.
    pub fn move_network_point(&mut self, id: NodeId, point: NetworkPointId, position: Point) -> bool {
        let Some(inverse) = self.document.world_transform(id).and_then(Transform::inverse) else {
            return false;
        };
        self.edit_vector_network(id, "Move network point", |network| {
            network.move_edit_point(point, inverse.apply(position)).then_some(())
        }).is_some()
    }

    pub fn add_network_vertex(&mut self, id: NodeId, position: Point) -> Option<VectorVertexId> {
        let inverse = self.document.world_transform(id)?.inverse()?;
        self.edit_vector_network(id, "Add network vertex", |network| {
            network.add_vertex(inverse.apply(position))
        })
    }

    /// Pen-style extension creates both the new endpoint and its connection in
    /// one undoable edit. Any failed connection leaves the document untouched.
    pub fn branch_network_vertex(
        &mut self,
        id: NodeId,
        start: VectorVertexId,
        position: Point,
    ) -> Option<(VectorVertexId, VectorEdgeId)> {
        let inverse = self.document.world_transform(id)?.inverse()?;
        self.edit_vector_network(id, "Extend vector network", |network| {
            network.vertices.get(&start)?;
            let vertex = network.add_vertex(inverse.apply(position))?;
            let edge = network.connect(start, vertex)?;
            Some((vertex, edge))
        })
    }

    pub fn connect_network_vertices(
        &mut self,
        id: NodeId,
        start: VectorVertexId,
        end: VectorVertexId,
    ) -> Option<VectorEdgeId> {
        self.edit_vector_network(id, "Connect network vertices", |network| network.connect(start, end))
    }

    /// Cubic control positions, like point-drag positions, are world-space.
    pub fn connect_network_curve(
        &mut self,
        id: NodeId,
        start: VectorVertexId,
        end: VectorVertexId,
        curve: NetworkCurve,
    ) -> Option<VectorEdgeId> {
        let inverse = self.document.world_transform(id)?.inverse()?;
        let curve = match curve {
            NetworkCurve::Line => NetworkCurve::Line,
            NetworkCurve::Cubic { control1, control2 } => NetworkCurve::Cubic {
                control1: inverse.apply(control1), control2: inverse.apply(control2),
            },
        };
        self.edit_vector_network(id, "Connect network curve", |network| {
            network.connect_curve(start, end, curve)
        })
    }

    pub fn split_network_edge(&mut self, id: NodeId, edge: VectorEdgeId, t: f64) -> Option<VectorVertexId> {
        self.edit_vector_network(id, "Split network edge", |network| network.split_edge(edge, t))
    }

    pub fn bend_network_edge(&mut self, id: NodeId, edge: VectorEdgeId, t: f64, position: Point) -> bool {
        let Some(inverse) = self.document.world_transform(id).and_then(Transform::inverse) else {
            return false;
        };
        self.edit_vector_network(id, "Bend network edge", |network| {
            network.bend_edge(edge, t, inverse.apply(position)).then_some(())
        }).is_some()
    }

    pub fn remove_network_edge(&mut self, id: NodeId, edge: VectorEdgeId) -> bool {
        self.edit_vector_network(id, "Erase network edge", |network| network.remove_edge(edge).then_some(())).is_some()
    }

    pub fn remove_network_vertex(&mut self, id: NodeId, vertex: VectorVertexId) -> bool {
        self.edit_vector_network(id, "Delete network vertex", |network| network.remove_vertex(vertex).then_some(())).is_some()
    }

    pub fn merge_network_vertices(&mut self, id: NodeId, source: VectorVertexId, target: VectorVertexId) -> bool {
        self.edit_vector_network(id, "Weld network vertices", |network| network.merge_vertices(source, target).then_some(())).is_some()
    }

    pub fn set_network_vertex_mode(&mut self, id: NodeId, vertex: VectorVertexId, mode: AnchorMode) -> bool {
        self.edit_vector_network(id, "Change network tangent", |network| network.set_vertex_mode(vertex, mode).then_some(())).is_some()
    }

    pub fn create_network_region(
        &mut self,
        id: NodeId,
        contours: Vec<Vec<NetworkEdgeRef>>,
    ) -> Option<VectorRegionId> {
        self.edit_vector_network(id, "Create network fill region", |network| network.add_region(contours))
    }

    pub fn create_network_region_from_vertices(
        &mut self,
        id: NodeId,
        vertices: &[VectorVertexId],
    ) -> Option<VectorRegionId> {
        self.edit_vector_network(id, "Create network fill region", |network| network.region_from_vertices(vertices))
    }

    pub fn remove_network_region(&mut self, id: NodeId, region: VectorRegionId) -> bool {
        self.edit_vector_network(id, "Remove network fill region", |network| network.remove_region(region).then_some(())).is_some()
    }
}

fn curve_is_finite(curve: &NetworkCurve) -> bool {
    match *curve {
        NetworkCurve::Line => true,
        NetworkCurve::Cubic { control1, control2 } => finite(control1) && finite(control2),
    }
}

/// Validate borrowed payloads: explicit face authoring and network import share
/// this boundary without copying potentially large embedded-image strings.
fn validate_region_fills(fills: &[crate::Fill]) -> Result<(), String> {
    let stops_valid = |stops: &[crate::GradientStop]| {
        !stops.is_empty()
            && stops.iter().all(|stop| stop.offset.is_finite() && (0.0..=1.0).contains(&stop.offset))
            && stops.windows(2).all(|pair| pair[0].offset <= pair[1].offset)
    };
    for fill in fills {
        if !fill.opacity.is_finite() || !(0.0..=1.0).contains(&fill.opacity) {
            return Err("Region paint opacity must be a finite value from 0 to 1.".into());
        }
        let valid = match &fill.paint {
            crate::Paint::Solid(_) => true,
            crate::Paint::LinearGradient { start, end, stops } => {
                finite(*start) && finite(*end) && stops_valid(stops)
            }
            crate::Paint::RadialGradient { center, radius, stops } => {
                finite(*center) && radius.is_finite() && *radius > 0.0 && stops_valid(stops)
            }
            crate::Paint::AngularGradient { center, angle, stops } => {
                finite(*center) && angle.is_finite() && stops_valid(stops)
            }
            crate::Paint::Image { href, crop, .. } => {
                !href.trim().is_empty() && crop.is_none_or(crate::valid_image_crop)
            }
        };
        if !valid { return Err("Region paint has invalid gradient coordinates, stops or image source.".into()); }
    }
    Ok(())
}

fn reverse_curve(curve: &NetworkCurve) -> NetworkCurve {
    match *curve {
        NetworkCurve::Line => NetworkCurve::Line,
        NetworkCurve::Cubic { control1, control2 } => NetworkCurve::Cubic {
            control1: control2,
            control2: control1,
        },
    }
}

fn region_key(region: &NetworkRegion) -> Vec<Vec<NetworkEdgeRef>> {
    let mut contours = Vec::with_capacity(region.contours.len());
    for contour in &region.contours {
        let Some((first, _)) = contour.iter().enumerate().min_by_key(|(_, edge)| **edge) else {
            continue;
        };
        let mut cycle = contour.clone();
        cycle.rotate_left(first);
        contours.push(cycle);
    }
    contours.sort();
    contours
}

fn network_tangents_match(a: Point, b: Point, anchor: Point, mode: AnchorMode) -> bool {
    let a = a - anchor;
    let b = b - anchor;
    let scale = a.x.abs().max(a.y.abs()).max(b.x.abs()).max(b.y.abs());
    if !scale.is_finite() {
        return false;
    }
    if scale <= GEOMETRY_EPSILON {
        return true;
    }
    let a = a * (1.0 / scale);
    let b = b * (1.0 / scale);
    if mode == AnchorMode::Symmetric {
        return (a + b).distance(Point::default()) <= 1e-7;
    }
    let (Some(a), Some(b)) = (unit(a), unit(b)) else {
        return true;
    };
    (a.x * b.y - a.y * b.x).abs() <= 1e-7 && a.x * b.x + a.y * b.y <= 0.0
}

#[cfg(test)]
#[path = "vector_network_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "vector_region_paints_tests.rs"]
mod region_paints_tests;
