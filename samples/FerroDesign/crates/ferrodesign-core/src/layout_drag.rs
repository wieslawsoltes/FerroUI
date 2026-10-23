//! Geometry-only, deferred auto-layout reordering. Pointer-frequency queries
//! read a retained spatial tree; the document is changed exactly once on drop.
use crate::*;
use serde::Serialize;

#[path = "layout_transfer.rs"]
mod transfer;
pub use transfer::*;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct LayoutInsertionMarker {
    /// World coordinates, including every affine ancestor transform.
    pub start: Point,
    pub end: Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct LayoutDropTarget {
    /// Insertion index in visual automatic flow after removing the dragged
    /// child. The session translates it back into the stored sibling order.
    pub index: usize,
    pub marker: LayoutInsertionMarker,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SiblingGeometry {
    id: NodeId,
    parent: Option<NodeId>,
    kind: NodeKind,
    bounds: Rect,
    transform: Transform,
    child: LayoutChild,
    visible: bool,
    locked: bool,
}

impl SiblingGeometry {
    fn capture(node: &Node) -> Self {
        Self {
            id: node.id,
            parent: node.parent,
            kind: node.kind,
            bounds: node.bounds(),
            transform: node.local_transform(),
            child: node.layout_child,
            visible: node.visible,
            locked: node.locked,
        }
    }
}

#[derive(Clone, Debug)]
struct SlotBranch {
    bounds: Rect,
    first: usize,
    count: usize,
    children: Option<(usize, usize)>,
}

/// One selected direct child can be inserted before or after any automatic
/// sibling in a row, column, wrap, or grid. Hidden and absolute children, and
/// grid children with an explicit row or column, keep their exact list slots.
/// Explicit grid placement remains a separate authoring operation: dropping
/// into a hole does not silently clear or invent pinned coordinates.
///
/// Construction retains only sibling geometry, IDs, and layout settings; it
/// never clones paint, image, text, or vector payloads. Queries allocate nothing
/// and search a balanced tree of actual transformed insertion edges. Typical
/// queries visit logarithmic branches; overlapping geometry can visit more.
#[derive(Clone, Debug)]
pub struct LayoutDragSession {
    child: NodeId,
    parent: NodeId,
    revision: u64,
    children: Vec<NodeId>,
    siblings: Vec<SiblingGeometry>,
    flow: Vec<NodeId>,
    layout: AutoLayout,
    parent_bounds: Rect,
    world: Transform,
    inverse_world: Transform,
    source_bounds: Rect,
    source_corners: [Point; 4],
    original: LayoutDropTarget,
    targets: Vec<LayoutDropTarget>,
    branches: Vec<SlotBranch>,
}

fn drag_error(message: impl Into<String>) -> DocumentError {
    DocumentError::Invalid(message.into())
}

fn finite_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn automatic_child(node: &Node, layout: &AutoLayout) -> bool {
    node.visible
        && !node.layout_child.absolute
        && !(layout.axis == LayoutAxis::Grid
            && node
                .layout_child
                .grid
                .is_some_and(|grid| grid.row.is_some() || grid.column.is_some()))
}

/// Reordering materialized instance descendants would be overwritten by the
/// next component sync. An instance itself remains reorderable in its parent.
pub(crate) fn validate_layout_reorder_ancestry(
    document: &Document,
    parent: NodeId,
) -> Result<(), DocumentError> {
    let mut current = Some(parent);
    for _ in 0..MAX_DOCUMENT_DEPTH {
        let Some(id) = current else {
            return Ok(());
        };
        if document.page(id).is_some() {
            return Ok(());
        }
        let node = document
            .nodes
            .get(&id)
            .ok_or(DocumentError::MissingNode(id))?;
        if node.kind == NodeKind::Instance {
            return Err(drag_error(
                "Edit the main component or detach the instance before reordering its children",
            ));
        }
        if node.locked || !node.visible {
            return Err(drag_error("The auto-layout frame is hidden or locked"));
        }
        current = node.parent;
    }
    Err(drag_error(
        "The auto-layout hierarchy exceeds the supported depth",
    ))
}

impl LayoutDragSession {
    pub fn new(document: &Document, child: NodeId) -> Result<Self, DocumentError> {
        let node = document
            .nodes
            .get(&child)
            .ok_or(DocumentError::MissingNode(child))?;
        let parent = node
            .parent
            .and_then(|id| document.nodes.get(&id))
            .filter(|parent| parent.layout.is_some())
            .ok_or_else(|| drag_error("Select a direct child of an auto-layout frame"))?;
        let layout = parent.layout.as_ref().unwrap();
        validate_layout_reorder_ancestry(document, parent.id)?;
        if node.locked || !automatic_child(node, layout) {
            return Err(drag_error("Drag a visible, unlocked automatic flow child; edit pinned grid coordinates in Layout placement"));
        }
        document.validate_layout_node_settings(parent.id)?;
        let world = document
            .world_transform(parent.id)
            .ok_or_else(|| drag_error("The auto-layout transform is unavailable"))?;
        let inverse_world = world
            .inverse()
            .ok_or_else(|| drag_error("The auto-layout transform is singular"))?;
        let mut siblings = Vec::with_capacity(parent.children.len());
        let mut flow = Vec::with_capacity(parent.children.len());
        for id in &parent.children {
            let sibling = document
                .nodes
                .get(id)
                .ok_or(DocumentError::MissingNode(*id))?;
            let geometry = SiblingGeometry::capture(sibling);
            if sibling.parent != Some(parent.id)
                || !geometry.bounds.is_finite()
                || !geometry.bounds.right().is_finite()
                || !geometry.bounds.bottom().is_finite()
            {
                return Err(drag_error(
                    "The auto-layout children contain invalid geometry or parent links",
                ));
            }
            siblings.push(geometry);
            if automatic_child(sibling, layout) {
                flow.push(*id);
            }
        }
        if layout.reverse {
            flow.reverse();
        }
        let original_index = flow
            .iter()
            .position(|id| *id == child)
            .ok_or_else(|| drag_error("The child is missing from its auto-layout frame"))?;
        if flow.len() < 2 {
            return Err(drag_error(
                "At least two automatic flow children are needed to reorder",
            ));
        }
        let child_world = document
            .world_transform(child)
            .ok_or_else(|| drag_error("The child transform is unavailable"))?;
        let source_corners = node
            .local_bounds()
            .corners()
            .map(|point| child_world.apply(point));
        if source_corners.iter().any(|point| !finite_point(*point)) {
            return Err(drag_error(
                "The child transform exceeds finite geometry bounds",
            ));
        }
        let row = layout.axis != LayoutAxis::Column;
        let edge = |bounds: Rect, after: bool| {
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
        };
        let original = LayoutDropTarget {
            index: original_index,
            marker: edge(node.bounds(), false),
        };
        let mut targets = Vec::with_capacity(2 * flow.len().saturating_sub(1));
        for (index, id) in flow.iter().filter(|id| **id != child).enumerate() {
            let bounds = document.nodes[id].bounds();
            for after in [false, true] {
                let target = LayoutDropTarget {
                    index: index + usize::from(after),
                    marker: edge(bounds, after),
                };
                if !finite_point(target.marker.start) || !finite_point(target.marker.end) {
                    return Err(drag_error(
                        "An insertion edge exceeds finite geometry bounds",
                    ));
                }
                targets.push(target);
            }
        }
        let mut branches = Vec::with_capacity(targets.len());
        build_slots(&mut targets, 0, &mut branches);
        Ok(Self {
            child,
            parent: parent.id,
            revision: document.revision,
            children: parent.children.clone(),
            siblings,
            flow,
            layout: layout.clone(),
            parent_bounds: parent.bounds(),
            world,
            inverse_world,
            source_bounds: node.bounds(),
            source_corners,
            original,
            targets,
            branches,
        })
    }

    pub fn child(&self) -> NodeId {
        self.child
    }
    pub fn parent(&self) -> NodeId {
        self.parent
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn source_corners(&self) -> [Point; 4] {
        self.source_corners
    }
    pub fn changes_order(&self, target: LayoutDropTarget) -> bool {
        target.index != self.original.index
    }

    /// The original layer rectangle is a stable no-op region, including wide
    /// grid spans. Crossing and returning to it never creates a history entry.
    pub fn target(&self, world: Point) -> Option<LayoutDropTarget> {
        if !finite_point(world) {
            return None;
        }
        let local = self.inverse_world.apply(world);
        if !finite_point(local) {
            return None;
        }
        if self.source_bounds.contains(local) {
            return Some(self.original);
        }
        let mut best = None;
        self.nearest_slot(0, world, &mut best);
        best.map(|(_, target)| target)
    }

    fn nearest_slot(
        &self,
        branch: usize,
        point: Point,
        best: &mut Option<(f64, LayoutDropTarget)>,
    ) {
        let branch = &self.branches[branch];
        if best
            .as_ref()
            .is_some_and(|(distance, _)| rect_distance(branch.bounds, point) > *distance)
        {
            return;
        }
        if let Some((left, right)) = branch.children {
            let first_left = rect_distance(self.branches[left].bounds, point)
                <= rect_distance(self.branches[right].bounds, point);
            let (first, second) = if first_left {
                (left, right)
            } else {
                (right, left)
            };
            self.nearest_slot(first, point, best);
            self.nearest_slot(second, point, best);
        } else {
            for target in &self.targets[branch.first..branch.first + branch.count] {
                let distance = segment_distance(point, target.marker.start, target.marker.end);
                if !distance.is_finite() {
                    continue;
                }
                let preferred = best.as_ref().is_none_or(|(old_distance, old)| {
                    distance < *old_distance
                        || (distance == *old_distance
                            && (target.index.abs_diff(self.original.index), target.index)
                                < (old.index.abs_diff(self.original.index), old.index))
                });
                if preferred {
                    *best = Some((distance, *target));
                }
            }
        }
    }

    fn validate_current(&self, document: &Document) -> Result<(), DocumentError> {
        let parent = document
            .nodes
            .get(&self.parent)
            .ok_or(DocumentError::MissingNode(self.parent))?;
        validate_layout_reorder_ancestry(document, self.parent)?;
        if document.revision != self.revision
            || parent.children != self.children
            || parent.layout.as_ref() != Some(&self.layout)
            || parent.bounds() != self.parent_bounds
            || document.world_transform(self.parent) != Some(self.world)
            || self.siblings.iter().any(|snapshot| {
                document
                    .nodes
                    .get(&snapshot.id)
                    .is_none_or(|node| SiblingGeometry::capture(node) != *snapshot)
            })
        {
            return Err(drag_error(
                "The document changed during the drag; start a new drag",
            ));
        }
        Ok(())
    }

    fn reordered_children(&self, index: usize) -> Result<Vec<NodeId>, DocumentError> {
        if index >= self.flow.len() {
            return Err(drag_error("The layout insertion target is out of range"));
        }
        let mut order: Vec<_> = self
            .flow
            .iter()
            .copied()
            .filter(|id| *id != self.child)
            .collect();
        order.insert(index, self.child);
        if self.layout.reverse {
            order.reverse();
        }
        let mut order = order.into_iter();
        Ok(self
            .siblings
            .iter()
            .map(|snapshot| {
                if snapshot.visible
                    && !snapshot.child.absolute
                    && !(self.layout.axis == LayoutAxis::Grid
                        && snapshot
                            .child
                            .grid
                            .is_some_and(|grid| grid.row.is_some() || grid.column.is_some()))
                {
                    order
                        .next()
                        .expect("one retained slot for each automatic child")
                } else {
                    snapshot.id
                }
            })
            .collect())
    }
}

impl Editor {
    /// The immutable preview does not open history. Validate the snapshot and
    /// candidate completely before atomically committing one reorder/reflow.
    /// Cancellation consists of discarding the session; no restoration writes
    /// or redo-stack changes are needed. A no-op does not even change revision.
    pub fn commit_layout_drag(
        &mut self,
        session: &LayoutDragSession,
        target: LayoutDropTarget,
    ) -> Result<LayoutResult, DocumentError> {
        if self.has_active_transaction() {
            return Err(drag_error(
                "Finish the active edit before dropping a layout child",
            ));
        }
        session.validate_current(&self.document)?;
        let children = session.reordered_children(target.index)?;
        if children == self.document.nodes[&session.parent].children {
            return Ok(LayoutResult::default());
        }
        let mut candidate = self.document.clone();
        candidate.nodes.get_mut(&session.parent).unwrap().children = children;
        candidate.validate_layout_node_settings(session.parent)?;
        self.commit_layout_candidate("Reorder layout child", candidate)
    }
}

fn marker_bounds(target: &LayoutDropTarget) -> Rect {
    Rect::from_points(target.marker.start, target.marker.end)
}

fn build_slots(
    targets: &mut [LayoutDropTarget],
    first: usize,
    branches: &mut Vec<SlotBranch>,
) -> usize {
    let bounds = targets
        .iter()
        .map(marker_bounds)
        .reduce(Rect::union)
        .unwrap_or_default();
    let index = branches.len();
    branches.push(SlotBranch {
        bounds,
        first,
        count: targets.len(),
        children: None,
    });
    if targets.len() > 8 {
        let split_x = bounds.width >= bounds.height;
        let middle = targets.len() / 2;
        targets.select_nth_unstable_by(middle, |a, b| {
            let a = marker_bounds(a).center();
            let b = marker_bounds(b).center();
            (if split_x { a.x } else { a.y }).total_cmp(&if split_x { b.x } else { b.y })
        });
        let (left, right) = targets.split_at_mut(middle);
        let left = build_slots(left, first, branches);
        let right = build_slots(right, first + middle, branches);
        branches[index].children = Some((left, right));
    }
    index
}

fn rect_distance(rect: Rect, point: Point) -> f64 {
    let dx = (rect.x - point.x).max(point.x - rect.right()).max(0.0);
    let dy = (rect.y - point.y).max(point.y - rect.bottom()).max(0.0);
    dx.hypot(dy)
}

fn segment_distance(point: Point, start: Point, end: Point) -> f64 {
    let vector = end - start;
    let length = vector.x.hypot(vector.y);
    if length < GEOMETRY_EPSILON {
        return point.distance(start);
    }
    let direction = vector * (1.0 / length);
    let relative = point - start;
    let along = (relative.x * direction.x + relative.y * direction.y).clamp(0.0, length);
    point.distance(start + direction * along)
}

#[cfg(test)]
#[path = "layout_drag_tests.rs"]
mod tests;
