//! Immutable, geometry-only selection transfer between layout and free frames.
//! Source and destination models remain untouched until one validated drop.
use super::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutTransferKind {
    /// Insert nonabsolute selected nodes into the destination's automatic flow.
    Flow,
    /// Retain the translated world geometry inside a free frame or page.
    Free,
    /// Return to the original footprint without changing order or history.
    Restore,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct LayoutTransferTarget {
    pub parent: NodeId,
    pub index: usize,
    pub marker: LayoutInsertionMarker,
    pub kind: LayoutTransferKind,
    /// Current world pointer position; free and absolute nodes keep its delta.
    pub position: Point,
}

#[derive(Clone, Debug)]
struct TransferNode {
    geometry: SiblingGeometry,
    world: Transform,
    corners: [Point; 4],
}

#[derive(Clone, Debug)]
struct TransferClip {
    id: NodeId,
    world: Transform,
    inverse: Transform,
    bounds: Rect,
    radius: f64,
    parent: Option<usize>,
}

#[derive(Clone, Debug)]
struct TransferFrame {
    id: NodeId,
    geometry: Option<SiblingGeometry>,
    children: Vec<NodeId>,
    siblings: Vec<SiblingGeometry>,
    flow: Vec<NodeId>,
    layout: Option<AutoLayout>,
    world: Transform,
    inverse: Transform,
    bounds: Rect,
    world_bounds: Rect,
    clip_content: bool,
    clip: Option<usize>,
    order: usize,
    eligible: bool,
    targets: Vec<LayoutDropTarget>,
    branches: Vec<SlotBranch>,
    preferred_index: usize,
}

impl TransferFrame {
    fn capture(
        document: &Document,
        id: NodeId,
        selected: &HashSet<NodeId>,
        clip: Option<usize>,
        order: usize,
        eligible: bool,
    ) -> Result<Self, DocumentError> {
        let node = document.nodes.get(&id);
        let children = document
            .children(id)
            .ok_or(DocumentError::InvalidParent(id))?;
        let world = if node.is_some() {
            document
                .world_transform(id)
                .ok_or_else(|| drag_error("The destination transform is unavailable"))?
        } else {
            Transform::IDENTITY
        };
        let inverse = world
            .inverse()
            .ok_or_else(|| drag_error("The destination transform is singular"))?;
        let bounds = node.map(Node::local_bounds).unwrap_or_default();
        let world_bounds = world.transform_rect(bounds);
        if !world_bounds.is_finite()
            || !world_bounds.right().is_finite()
            || !world_bounds.bottom().is_finite()
        {
            return Err(drag_error("The destination exceeds finite geometry bounds"));
        }
        let layout = node.and_then(|node| node.layout.clone());
        if layout.is_some() {
            document.validate_layout_node_settings(id)?;
        }
        let mut siblings = Vec::with_capacity(children.len());
        let mut flow = Vec::with_capacity(children.len());
        for child in children {
            let child = document
                .nodes
                .get(child)
                .ok_or(DocumentError::MissingNode(*child))?;
            let geometry = SiblingGeometry::capture(child);
            if child.parent != Some(id)
                || !geometry.bounds.is_finite()
                || !geometry.bounds.right().is_finite()
                || !geometry.bounds.bottom().is_finite()
            {
                return Err(drag_error(
                    "A frame contains invalid child geometry or parent links",
                ));
            }
            if layout
                .as_ref()
                .is_some_and(|layout| automatic_child(child, layout))
            {
                flow.push(child.id);
            }
            siblings.push(geometry);
        }
        if layout.as_ref().is_some_and(|layout| layout.reverse) {
            flow.reverse();
        }
        let preferred_index = flow
            .iter()
            .position(|id| selected.contains(id))
            .unwrap_or(0);
        let remaining: Vec<_> = flow
            .iter()
            .filter(|id| !selected.contains(id))
            .copied()
            .collect();
        let mut targets = Vec::with_capacity(2 * remaining.len().max(1));
        if let Some(layout) = &layout {
            let row = layout.axis != LayoutAxis::Column;
            for (index, child) in remaining.iter().enumerate() {
                for after in [false, true] {
                    targets.push(LayoutDropTarget {
                        index: index + usize::from(after),
                        marker: transfer_edge(document.nodes[child].bounds(), world, row, after),
                    });
                }
            }
            if targets.is_empty() {
                let p = layout.padding;
                let content = Rect::new(
                    p.left,
                    p.top,
                    (bounds.width - p.left - p.right).max(0.0),
                    (bounds.height - p.top - p.bottom).max(0.0),
                );
                targets.push(LayoutDropTarget {
                    index: 0,
                    marker: transfer_edge(content, world, row, false),
                });
            }
        }
        if targets
            .iter()
            .any(|target| !finite_point(target.marker.start) || !finite_point(target.marker.end))
        {
            return Err(drag_error(
                "A destination edge exceeds finite geometry bounds",
            ));
        }
        let mut branches = Vec::with_capacity(targets.len());
        if !targets.is_empty() {
            build_slots(&mut targets, 0, &mut branches);
        }
        Ok(Self {
            id,
            geometry: node.map(SiblingGeometry::capture),
            children: children.to_vec(),
            siblings,
            flow,
            layout,
            world,
            inverse,
            bounds,
            world_bounds,
            clip_content: node.is_some_and(|node| node.clip_content),
            clip,
            order,
            eligible,
            targets,
            branches,
            preferred_index,
        })
    }

    fn validate_current(&self, document: &Document) -> Result<(), DocumentError> {
        validate_layout_reorder_ancestry(document, self.id)?;
        let node = document.nodes.get(&self.id);
        let current_world = node
            .map(|_| document.world_transform(self.id))
            .unwrap_or(Some(Transform::IDENTITY));
        if document.children(self.id) != Some(self.children.as_slice())
            || node.map(SiblingGeometry::capture) != self.geometry
            || node.and_then(|node| node.layout.as_ref()) != self.layout.as_ref()
            || node.is_some_and(|node| node.clip_content) != self.clip_content
            || current_world != Some(self.world)
            || self.siblings.iter().any(|snapshot| {
                document
                    .nodes
                    .get(&snapshot.id)
                    .is_none_or(|node| SiblingGeometry::capture(node) != *snapshot)
            })
        {
            return Err(drag_error(
                "A source or destination changed during the drag; start a new drag",
            ));
        }
        Ok(())
    }

    fn slot(&self, point: Point) -> LayoutDropTarget {
        let mut best = None;
        nearest_transfer_slot(
            &self.targets,
            &self.branches,
            0,
            point,
            self.preferred_index,
            &mut best,
        );
        best.expect("a layout frame always has an insertion edge").1
    }
}

fn transfer_edge(bounds: Rect, world: Transform, row: bool, after: bool) -> LayoutInsertionMarker {
    let (start, end) = if row {
        let x = if after { bounds.right() } else { bounds.x };
        (Point::new(x, bounds.y), Point::new(x, bounds.bottom()))
    } else {
        let y = if after { bounds.bottom() } else { bounds.y };
        (Point::new(bounds.x, y), Point::new(bounds.right(), y))
    };
    LayoutInsertionMarker {
        start: world.apply(start),
        end: world.apply(end),
    }
}

/// A multi-selection drag snapshots geometry once, including candidate frames
/// on the source page. Both the frame broad phase and each frame's insertion
/// edges are indexed. Pointer queries allocate nothing and never mutate model
/// or history. Entire selected subtrees move by stable ID on a successful drop.
#[derive(Clone, Debug)]
pub struct LayoutTransferSession {
    revision: u64,
    page: NodeId,
    selection: Vec<NodeId>,
    nodes: Vec<TransferNode>,
    selected: HashSet<NodeId>,
    source_parents: BTreeSet<NodeId>,
    frames: Vec<TransferFrame>,
    frame_indices: BTreeMap<NodeId, usize>,
    spatial_frames: Vec<usize>,
    branches: Vec<SlotBranch>,
    clips: Vec<TransferClip>,
    anchor: NodeId,
    anchor_parent: NodeId,
    anchor_inverse: Transform,
    anchor_bounds: Rect,
    origin: Point,
    original_marker: LayoutInsertionMarker,
}

impl LayoutTransferSession {
    pub fn new(
        document: &Document,
        selection: &[NodeId],
        anchor: NodeId,
        origin: Point,
    ) -> Result<Self, DocumentError> {
        if selection.is_empty() || !selection.contains(&anchor) || !finite_point(origin) {
            return Err(drag_error(
                "Choose a visible selection and a finite drag origin",
            ));
        }
        let page = document
            .page_of(anchor)
            .ok_or(DocumentError::MissingNode(anchor))?;
        let all_selected: HashSet<_> = selection.iter().copied().collect();
        if all_selected.len() != selection.len() {
            return Err(drag_error("The drag selection contains duplicate layers"));
        }
        let selected: HashSet<_> = selection
            .iter()
            .copied()
            .filter(|id| {
                let mut current = document.nodes.get(id).and_then(|node| node.parent);
                for _ in 0..MAX_DOCUMENT_DEPTH {
                    let Some(parent) = current else {
                        break;
                    };
                    if all_selected.contains(&parent) {
                        return false;
                    }
                    current = document.nodes.get(&parent).and_then(|node| node.parent);
                }
                true
            })
            .collect();
        let mut excluded = HashSet::new();
        let mut source_parents = BTreeSet::new();
        for id in &selected {
            let node = document
                .nodes
                .get(id)
                .ok_or(DocumentError::MissingNode(*id))?;
            if document.page_of(*id) != Some(page) || node.locked || !node.visible {
                return Err(drag_error(
                    "Drag visible, unlocked layers from the same page",
                ));
            }
            let parent = node.parent.ok_or(DocumentError::InvalidParent(*id))?;
            validate_layout_reorder_ancestry(document, parent)?;
            source_parents.insert(parent);
            excluded.extend(document.descendants(*id));
        }
        // Visual traversal preserves the relative order of a selection inside
        // reversed layouts, independently of click order or source parents.
        let mut nodes = Vec::with_capacity(selected.len());
        let mut pending: Vec<_> = document
            .page(page)
            .unwrap()
            .children
            .iter()
            .rev()
            .copied()
            .collect();
        while let Some(id) = pending.pop() {
            let node = document
                .nodes
                .get(&id)
                .ok_or(DocumentError::MissingNode(id))?;
            if selected.contains(&id) {
                let world = document
                    .world_transform(id)
                    .ok_or_else(|| drag_error("A source transform is unavailable"))?;
                let corners = node
                    .local_bounds()
                    .corners()
                    .map(|point| world.apply(point));
                if world.inverse().is_none() || corners.iter().any(|point| !finite_point(*point)) {
                    return Err(drag_error(
                        "A source transform is singular or exceeds finite geometry bounds",
                    ));
                }
                nodes.push(TransferNode {
                    geometry: SiblingGeometry::capture(node),
                    world,
                    corners,
                });
            } else if node.layout.as_ref().is_some_and(|layout| layout.reverse) {
                pending.extend(node.children.iter().copied());
            } else {
                pending.extend(node.children.iter().rev().copied());
            }
        }
        if nodes.len() != selected.len() {
            return Err(drag_error("The selection is not reachable on its page"));
        }
        let anchor_node = &document.nodes[&anchor];
        let anchor_world = document
            .world_transform(anchor)
            .ok_or_else(|| drag_error("The drag origin transform is unavailable"))?;
        let anchor_inverse = anchor_world
            .inverse()
            .ok_or_else(|| drag_error("The drag origin transform is singular"))?;
        let anchor_parent = nodes
            .iter()
            .find(|node| {
                node.geometry.id == anchor || document.is_ancestor(node.geometry.id, anchor)
            })
            .unwrap()
            .geometry
            .parent
            .unwrap();
        let original_marker = transfer_edge(anchor_node.local_bounds(), anchor_world, true, false);
        let mut clips = Vec::new();
        let mut clip_indices = HashMap::new();
        let mut frames = vec![TransferFrame::capture(
            document, page, &selected, None, 0, true,
        )?];
        let draw_order = document.draw_order(page);
        for (order, id) in draw_order.into_iter().enumerate() {
            let node = &document.nodes[&id];
            let mut clip = node
                .parent
                .and_then(|parent| clip_indices.get(&parent).copied())
                .flatten();
            if node.clip_content {
                if let Some(world) = document.world_transform(id) {
                    if let Some(inverse) = world.inverse() {
                        let index = clips.len();
                        clips.push(TransferClip {
                            id,
                            world,
                            inverse,
                            bounds: node.local_bounds(),
                            radius: node.corner_radius,
                            parent: clip,
                        });
                        clip = Some(index);
                    }
                }
            }
            clip_indices.insert(id, clip);
            let source = source_parents.contains(&id);
            let frame_kind = matches!(
                node.kind,
                NodeKind::Frame | NodeKind::Component | NodeKind::Section
            ) || (node.layout.is_some() && node.kind == NodeKind::Group);
            let eligible = frame_kind
                && !excluded.contains(&id)
                && !document.is_component_set(id)
                && validate_layout_reorder_ancestry(document, id).is_ok();
            if eligible || source {
                match TransferFrame::capture(document, id, &selected, clip, order + 1, eligible) {
                    Ok(frame) => frames.push(frame),
                    Err(error) if source => return Err(error),
                    Err(_) => {}
                }
            }
        }
        let frame_indices: BTreeMap<_, _> = frames
            .iter()
            .enumerate()
            .map(|(index, frame)| (frame.id, index))
            .collect();
        if source_parents
            .iter()
            .any(|parent| !frame_indices.contains_key(parent))
        {
            return Err(drag_error("A source frame cannot be snapshotted"));
        }
        let mut spatial_frames: Vec<_> = frames
            .iter()
            .enumerate()
            .filter(|(_, frame)| frame.id != page && frame.eligible)
            .map(|(index, _)| index)
            .collect();
        let mut branches = Vec::with_capacity(spatial_frames.len());
        if !spatial_frames.is_empty() {
            build_transfer_frames(&mut spatial_frames, 0, &frames, &mut branches);
        }
        Ok(Self {
            revision: document.revision,
            page,
            selection: selection.to_vec(),
            nodes,
            selected,
            source_parents,
            frames,
            frame_indices,
            spatial_frames,
            branches,
            clips,
            anchor,
            anchor_parent,
            anchor_inverse,
            anchor_bounds: anchor_node.local_bounds(),
            origin,
            original_marker,
        })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn page(&self) -> NodeId {
        self.page
    }
    pub fn selection(&self) -> &[NodeId] {
        &self.selection
    }
    pub fn child(&self) -> NodeId {
        self.anchor
    }
    pub fn parent(&self) -> NodeId {
        self.anchor_parent
    }
    pub fn source_corners(&self) -> impl Iterator<Item = [Point; 4]> + '_ {
        self.nodes.iter().map(|node| node.corners)
    }
    pub fn source_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().map(|node| node.geometry.id)
    }
    pub fn has_layout_source(&self) -> bool {
        self.nodes.iter().any(|node| {
            !node.geometry.child.absolute
                && node
                    .geometry
                    .parent
                    .is_some_and(|parent| self.frames[self.frame_indices[&parent]].layout.is_some())
        })
    }
    pub fn reparents(&self, target: LayoutTransferTarget) -> bool {
        target.kind != LayoutTransferKind::Restore
            && self
                .nodes
                .iter()
                .any(|node| node.geometry.parent != Some(target.parent))
    }
    pub fn destination_corners(&self, target: LayoutTransferTarget) -> Option<[Point; 4]> {
        let frame = &self.frames[*self.frame_indices.get(&target.parent)?];
        (target.parent != self.page)
            .then(|| frame.bounds.corners().map(|point| frame.world.apply(point)))
    }

    pub fn target(&self, point: Point) -> Option<LayoutTransferTarget> {
        if !finite_point(point) {
            return None;
        }
        let local = self.anchor_inverse.apply(point);
        if finite_point(local) && self.anchor_bounds.contains(local) {
            return Some(LayoutTransferTarget {
                parent: self.anchor_parent,
                index: 0,
                marker: self.original_marker,
                kind: LayoutTransferKind::Restore,
                position: point,
            });
        }
        let mut best = None;
        if !self.branches.is_empty() {
            self.frame_at(0, point, &mut best);
        }
        self.target_in(self.frames[best.unwrap_or(0)].id, point)
    }

    /// Explicit destination targeting is useful for keyboard/accessible moves
    /// and tests. It uses the same insertion geometry and validation as a drop.
    pub fn target_in(&self, parent: NodeId, point: Point) -> Option<LayoutTransferTarget> {
        if !finite_point(point) {
            return None;
        }
        let frame = &self.frames[*self.frame_indices.get(&parent)?];
        if !frame.eligible {
            return None;
        }
        let (index, marker, kind) = if frame.layout.is_some() {
            let slot = frame.slot(point);
            (slot.index, slot.marker, LayoutTransferKind::Flow)
        } else {
            (
                frame.children.len(),
                transfer_edge(frame.bounds, frame.world, true, false),
                LayoutTransferKind::Free,
            )
        };
        Some(LayoutTransferTarget {
            parent,
            index,
            marker,
            kind,
            position: point,
        })
    }

    fn frame_at(&self, branch: usize, point: Point, best: &mut Option<usize>) {
        let branch = &self.branches[branch];
        if !branch.bounds.contains(point) {
            return;
        }
        if let Some((left, right)) = branch.children {
            self.frame_at(left, point, best);
            self.frame_at(right, point, best);
            return;
        }
        for index in &self.spatial_frames[branch.first..branch.first + branch.count] {
            let frame = &self.frames[*index];
            if best.is_some_and(|best| self.frames[best].order > frame.order)
                || !frame.bounds.contains(frame.inverse.apply(point))
                || !self.inside_clips(frame.clip, point)
            {
                continue;
            }
            *best = Some(*index);
        }
    }

    fn inside_clips(&self, mut index: Option<usize>, point: Point) -> bool {
        while let Some(current) = index {
            let clip = &self.clips[current];
            let local = clip.inverse.apply(point);
            if !clip.bounds.contains(local) {
                return false;
            }
            let radius = clip
                .radius
                .max(0.0)
                .min(clip.bounds.width * 0.5)
                .min(clip.bounds.height * 0.5);
            if radius > 0.0 {
                let center = Point::new(
                    local.x.clamp(radius, clip.bounds.width - radius),
                    local.y.clamp(radius, clip.bounds.height - radius),
                );
                if local.distance(center) > radius {
                    return false;
                }
            }
            index = clip.parent;
        }
        true
    }

    fn validate_current(
        &self,
        document: &Document,
        target: LayoutTransferTarget,
    ) -> Result<&TransferFrame, DocumentError> {
        if document.revision != self.revision || !finite_point(target.position) {
            return Err(drag_error(
                "The document changed during the drag; start a new drag",
            ));
        }
        let destination = self
            .frame_indices
            .get(&target.parent)
            .map(|index| &self.frames[*index])
            .filter(|frame| {
                frame.eligible
                    || (target.kind == LayoutTransferKind::Restore
                        && frame.id == self.anchor_parent)
            })
            .ok_or_else(|| drag_error("Choose an editable destination outside the selection"))?;
        destination.validate_current(document)?;
        for parent in &self.source_parents {
            self.frames[self.frame_indices[parent]].validate_current(document)?;
        }
        for node in &self.nodes {
            if document
                .nodes
                .get(&node.geometry.id)
                .is_none_or(|current| SiblingGeometry::capture(current) != node.geometry)
                || document.world_transform(node.geometry.id) != Some(node.world)
                || document.page_of(node.geometry.id) != Some(self.page)
            {
                return Err(drag_error(
                    "The selection changed during the drag; start a new drag",
                ));
            }
            if node.geometry.id == target.parent
                || document.is_ancestor(node.geometry.id, target.parent)
            {
                return Err(DocumentError::Cycle(node.geometry.id));
            }
        }
        let mut clip = destination.clip;
        while let Some(index) = clip {
            let snapshot = &self.clips[index];
            if document.nodes.get(&snapshot.id).is_none_or(|node| {
                !node.clip_content
                    || node.local_bounds() != snapshot.bounds
                    || node.corner_radius != snapshot.radius
            }) || document.world_transform(snapshot.id) != Some(snapshot.world)
            {
                return Err(drag_error(
                    "The destination clipping changed during the drag",
                ));
            }
            clip = snapshot.parent;
        }
        Ok(destination)
    }

    fn destination_order(
        &self,
        frame: &TransferFrame,
        target: LayoutTransferTarget,
    ) -> Result<Vec<NodeId>, DocumentError> {
        if target.kind == LayoutTransferKind::Free {
            let mut order = frame.children.clone();
            order.extend(
                self.nodes
                    .iter()
                    .filter(|node| node.geometry.parent != Some(frame.id))
                    .map(|node| node.geometry.id),
            );
            return Ok(order);
        }
        let Some(layout) = &frame.layout else {
            return Err(drag_error("The destination is not an auto-layout frame"));
        };
        let mut flow: Vec<_> = frame
            .flow
            .iter()
            .filter(|id| !self.selected.contains(id))
            .copied()
            .collect();
        if target.index > flow.len() {
            return Err(drag_error("The insertion target is out of range"));
        }
        flow.splice(
            target.index..target.index,
            self.nodes
                .iter()
                .filter(|node| !node.geometry.child.absolute)
                .map(|node| node.geometry.id),
        );
        if layout.reverse {
            flow.reverse();
        }
        let mut flow = flow.into_iter();
        let mut order = Vec::with_capacity(frame.children.len() + self.nodes.len());
        for sibling in &frame.siblings {
            let selected_flow = self.selected.contains(&sibling.id) && !sibling.child.absolute;
            let automatic = sibling.visible
                && !sibling.child.absolute
                && !(layout.axis == LayoutAxis::Grid
                    && sibling
                        .child
                        .grid
                        .is_some_and(|grid| grid.row.is_some() || grid.column.is_some()));
            if selected_flow || automatic {
                if let Some(id) = flow.next() {
                    order.push(id);
                }
            } else {
                order.push(sibling.id);
            }
        }
        order.extend(flow);
        order.extend(
            self.nodes
                .iter()
                .filter(|node| {
                    node.geometry.child.absolute && node.geometry.parent != Some(frame.id)
                })
                .map(|node| node.geometry.id),
        );
        Ok(order)
    }
}

impl Editor {
    /// Reparent and arrange an entire selection atomically. No-op and failed
    /// drops preserve revision, redo, selection and unrelated transactions.
    pub fn commit_layout_transfer(
        &mut self,
        session: &LayoutTransferSession,
        target: LayoutTransferTarget,
    ) -> Result<LayoutResult, DocumentError> {
        if self.has_active_transaction()
            || self.revision != session.revision
            || self.active_page != session.page
            || self.selection != session.selection
        {
            return Err(drag_error(
                "The drag no longer owns the current selection and editor state",
            ));
        }
        let destination = session.validate_current(&self.document, target)?;
        if target.kind == LayoutTransferKind::Restore {
            if target.parent != session.anchor_parent {
                return Err(drag_error("Invalid original drag target"));
            }
            return Ok(LayoutResult::default());
        }
        if (target.kind == LayoutTransferKind::Flow) != destination.layout.is_some() {
            return Err(drag_error(
                "The insertion mode does not match its destination",
            ));
        }
        let order = session.destination_order(destination, target)?;
        let delta = target.position - session.origin;
        let settings_for = |node: &TransferNode| {
            let mut settings = node.geometry.child;
            if target.kind == LayoutTransferKind::Flow
                && !settings.absolute
                && (node.geometry.parent != Some(target.parent)
                    || destination
                        .layout
                        .as_ref()
                        .is_some_and(|layout| layout.axis == LayoutAxis::Grid))
            {
                settings.grid = settings
                    .grid
                    .map(|grid| GridPlacement {
                        row: None,
                        column: None,
                        ..grid
                    })
                    .filter(|grid| *grid != GridPlacement::default());
            }
            settings
        };
        let moved = session.nodes.iter().any(|node| {
            node.geometry.parent != Some(target.parent)
                || settings_for(node) != node.geometry.child
                || ((target.kind == LayoutTransferKind::Free || node.geometry.child.absolute)
                    && delta.distance(Point::default()) > GEOMETRY_EPSILON)
        });
        if !moved && order == destination.children {
            return Ok(LayoutResult::default());
        }
        let mut candidate = self.document.clone();
        for parent in &session.source_parents {
            if *parent != target.parent {
                candidate
                    .children_mut(*parent)
                    .unwrap()
                    .retain(|id| !session.selected.contains(id));
            }
        }
        *candidate.children_mut(target.parent).unwrap() = order;
        for source in &session.nodes {
            let node = candidate.nodes.get_mut(&source.geometry.id).unwrap();
            if source.geometry.parent != Some(target.parent) {
                node.set_local_transform(
                    destination.inverse * Transform::translation(delta.x, delta.y) * source.world,
                );
            } else if target.kind == LayoutTransferKind::Free || source.geometry.child.absolute {
                let local = destination.inverse.apply_vector(delta);
                node.x += local.x;
                node.y += local.y;
            }
            node.parent = Some(target.parent);
            node.layout_child = settings_for(source);
        }
        candidate.repair_component_references();
        for parent in session
            .source_parents
            .iter()
            .copied()
            .chain(std::iter::once(target.parent))
        {
            if candidate
                .nodes
                .get(&parent)
                .is_some_and(|node| node.layout.is_some())
            {
                candidate.validate_layout_node_settings(parent)?;
            }
        }
        self.commit_layout_candidate(
            if session.nodes.len() == 1 {
                "Move layout child"
            } else {
                "Move layout selection"
            },
            candidate,
        )
    }
}

fn nearest_transfer_slot(
    targets: &[LayoutDropTarget],
    branches: &[SlotBranch],
    branch: usize,
    point: Point,
    preferred: usize,
    best: &mut Option<(f64, LayoutDropTarget)>,
) {
    let branch = &branches[branch];
    if best
        .as_ref()
        .is_some_and(|(distance, _)| rect_distance(branch.bounds, point) > *distance)
    {
        return;
    }
    if let Some((left, right)) = branch.children {
        let (first, second) = if rect_distance(branches[left].bounds, point)
            <= rect_distance(branches[right].bounds, point)
        {
            (left, right)
        } else {
            (right, left)
        };
        nearest_transfer_slot(targets, branches, first, point, preferred, best);
        nearest_transfer_slot(targets, branches, second, point, preferred, best);
    } else {
        for target in &targets[branch.first..branch.first + branch.count] {
            let distance = segment_distance(point, target.marker.start, target.marker.end);
            if distance.is_finite()
                && best.as_ref().is_none_or(|(old_distance, old)| {
                    distance < *old_distance
                        || (distance == *old_distance
                            && (target.index.abs_diff(preferred), target.index)
                                < (old.index.abs_diff(preferred), old.index))
                })
            {
                *best = Some((distance, *target));
            }
        }
    }
}

fn build_transfer_frames(
    indices: &mut [usize],
    first: usize,
    frames: &[TransferFrame],
    branches: &mut Vec<SlotBranch>,
) -> usize {
    let bounds = indices
        .iter()
        .map(|index| frames[*index].world_bounds)
        .reduce(Rect::union)
        .unwrap_or_default();
    let index = branches.len();
    branches.push(SlotBranch {
        bounds,
        first,
        count: indices.len(),
        children: None,
    });
    if indices.len() > 8 {
        let horizontal = bounds.width >= bounds.height;
        let middle = indices.len() / 2;
        indices.select_nth_unstable_by(middle, |a, b| {
            let a = frames[*a].world_bounds.center();
            let b = frames[*b].world_bounds.center();
            (if horizontal { a.x } else { a.y }).total_cmp(&if horizontal { b.x } else { b.y })
        });
        let (left, right) = indices.split_at_mut(middle);
        let left = build_transfer_frames(left, first, frames, branches);
        let right = build_transfer_frames(right, first + middle, frames, branches);
        branches[index].children = Some((left, right));
    }
    index
}

#[cfg(test)]
#[path = "layout_transfer_tests.rs"]
mod tests;
