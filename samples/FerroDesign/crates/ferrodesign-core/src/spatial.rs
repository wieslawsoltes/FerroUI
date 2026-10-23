use crate::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
/// Uniform-grid broadphase; oversized shapes use a bounded-cost separate set.
#[derive(Clone, Debug)]
pub struct SpatialIndex {
    cell_size: f64,
    cells: HashMap<(i32, i32), HashSet<NodeId>>,
    bounds: BTreeMap<NodeId, Rect>,
    large: HashSet<NodeId>,
}
impl Default for SpatialIndex {
    fn default() -> Self {
        Self::new(256.0)
    }
}
impl SpatialIndex {
    pub fn new(cell_size: f64) -> Self {
        Self {
            cell_size: cell_size.max(1.0),
            cells: HashMap::new(),
            bounds: BTreeMap::new(),
            large: HashSet::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.bounds.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }
    pub fn clear(&mut self) {
        self.cells.clear();
        self.bounds.clear();
        self.large.clear()
    }
    fn range(&self, r: Rect) -> (i32, i32, i32, i32) {
        (
            (r.x / self.cell_size).floor() as i32,
            (r.y / self.cell_size).floor() as i32,
            (r.right() / self.cell_size).floor() as i32,
            (r.bottom() / self.cell_size).floor() as i32,
        )
    }
    fn cell_count((a, b, c, d): (i32, i32, i32, i32)) -> u64 {
        ((c as i64 - a as i64 + 1).max(0) as u64)
            .saturating_mul((d as i64 - b as i64 + 1).max(0) as u64)
    }
    pub fn insert(&mut self, id: NodeId, bounds: Rect) {
        if !bounds.is_finite() {
            self.remove(id);
            return;
        }
        let range = self.range(bounds);
        if let Some(previous) = self.bounds.get(&id).copied() {
            // Most pointer/nudge updates remain in the same cells. Preserve
            // their bucket memberships and replace only the exact bounds.
            if self.range(previous) == range
                || (self.large.contains(&id) && Self::cell_count(range) > 1024)
            {
                self.bounds.insert(id, bounds);
                return;
            }
        }
        self.remove(id);
        self.bounds.insert(id, bounds);
        if Self::cell_count(range) > 1024 {
            self.large.insert(id);
            return;
        }
        let (x0, y0, x1, y1) = range;
        for x in x0..=x1 {
            for y in y0..=y1 {
                self.cells.entry((x, y)).or_default().insert(id);
            }
        }
    }
    pub fn remove(&mut self, id: NodeId) {
        let Some(bounds) = self.bounds.remove(&id) else {
            return;
        };
        if self.large.remove(&id) {
            return;
        }
        let (x0, y0, x1, y1) = self.range(bounds);
        for x in x0..=x1 {
            for y in y0..=y1 {
                if let Some(cell) = self.cells.get_mut(&(x, y)) {
                    cell.remove(&id);
                    if cell.is_empty() {
                        self.cells.remove(&(x, y));
                    }
                }
            }
        }
    }
    pub fn bounds(&self, id: NodeId) -> Option<Rect> {
        self.bounds.get(&id).copied()
    }
    pub fn query(&self, rect: Rect) -> Vec<NodeId> {
        if !rect.is_finite() {
            return Vec::new();
        }
        let range = self.range(rect);
        if Self::cell_count(range) > 16_384 {
            return self
                .bounds
                .iter()
                .filter_map(|(id, b)| b.intersects(rect).then_some(*id))
                .collect();
        }
        let mut ids: Vec<_> = self.large.iter().copied().collect();
        let (x0, y0, x1, y1) = range;
        for x in x0..=x1 {
            for y in y0..=y1 {
                if let Some(cell) = self.cells.get(&(x, y)) {
                    ids.extend(cell.iter().copied())
                }
            }
        }
        // A flat buffer avoids one tree allocation per candidate while keeping
        // query order deterministic and removing shapes spanning several cells.
        ids.sort_unstable();
        ids.dedup();
        ids.retain(|id| self.bounds.get(id).is_some_and(|b| b.intersects(rect)));
        ids
    }
}
#[derive(Clone, Copy, Debug)]
struct WalkState {
    transform: Transform,
    visible: bool,
    clip: Option<Rect>,
    clipped: bool,
    operand: bool,
}
impl Default for WalkState {
    fn default() -> Self {
        Self {
            transform: Transform::IDENTITY,
            visible: true,
            clip: None,
            clipped: false,
            operand: false,
        }
    }
}
impl WalkState {
    fn intersect_clip(&mut self, bounds: Rect) {
        if let Some(clip) = self.clip {
            if let Some(intersection) = clip.intersection(bounds) {
                self.clip = Some(intersection);
            } else {
                self.clipped = true;
            }
        } else {
            self.clip = Some(bounds);
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct SceneIndex {
    pub spatial: SpatialIndex,
    pub transforms: BTreeMap<NodeId, Transform>,
    pub draw_order: Vec<NodeId>,
    pub z_order: BTreeMap<NodeId, usize>,
    pub revision: u64,
    variable_strokes: BTreeMap<NodeId, crate::stroke_hit::VariableStrokeHits>,
}
impl SceneIndex {
    pub fn rebuild(&mut self, document: &Document, page: NodeId) {
        self.spatial.clear();
        self.transforms.clear();
        self.draw_order.clear();
        self.z_order.clear();
        self.variable_strokes.clear();
        let roots = document
            .page(page)
            .map(|page| {
                page.children
                    .iter()
                    .rev()
                    .map(|id| (*id, WalkState::default()))
                    .collect()
            })
            .unwrap_or_default();
        self.walk(document, roots, true);
        self.revision = document.revision;
    }

    /// Re-index only changed subtrees while retaining stable paint order. Changes
    /// to hierarchy, visibility or boolean ownership use a full rebuild instead.
    pub fn update_nodes(&mut self, document: &Document, page: NodeId, changed: &[NodeId]) {
        let dirty: BTreeSet<_> = changed.iter().copied().collect();
        let mut roots = Vec::new();
        for id in changed {
            if document.page_of(*id) != Some(page) {
                continue;
            }
            let mut ancestor = document.nodes.get(id).and_then(|n| n.parent);
            let mut nested = false;
            let mut state = WalkState::default();
            if let Some(parent) = ancestor {
                state.transform = self
                    .transforms
                    .get(&parent)
                    .copied()
                    .or_else(|| document.world_transform(parent))
                    .unwrap_or_default();
            }
            for _ in 0..MAX_DOCUMENT_DEPTH {
                let Some(parent) = ancestor else { break };
                if dirty.contains(&parent) {
                    nested = true;
                    break;
                }
                let Some(node) = document.nodes.get(&parent) else {
                    break;
                };
                state.visible &= node.visible;
                state.operand |= node.kind == NodeKind::Boolean;
                if node.clip_content {
                    let transform = self
                        .transforms
                        .get(&parent)
                        .copied()
                        .or_else(|| document.world_transform(parent))
                        .unwrap_or_default();
                    state.intersect_clip(transform.transform_rect(node.local_bounds()));
                }
                ancestor = node.parent;
            }
            if !nested {
                roots.push((*id, state));
            }
        }
        self.walk(document, roots, false);
        self.revision = document.revision;
    }

    fn walk(
        &mut self,
        document: &Document,
        mut stack: Vec<(NodeId, WalkState)>,
        rebuild_order: bool,
    ) {
        while let Some((id, parent)) = stack.pop() {
            let Some(node) = document.nodes.get(&id) else {
                continue;
            };
            let transform = parent.transform * node.local_transform();
            self.transforms.insert(id, transform);
            if !self.variable_strokes.get(&id).is_some_and(|widths| widths.matches(node)) {
                if let Some(widths) = crate::stroke_hit::VariableStrokeHits::prepare(node) {
                    self.variable_strokes.insert(id, widths);
                } else {
                    self.variable_strokes.remove(&id);
                }
            }
            let mut indexed = false;
            let visible = parent.visible && node.visible;
            let bounds = transform.transform_rect(node.local_bounds());
            if visible && !parent.operand {
                // Keep a stable order even for layers currently outside a clip:
                // moving one into view must not change its sibling stacking.
                if rebuild_order {
                    self.z_order.insert(id, self.draw_order.len());
                    self.draw_order.push(id);
                }
                if !parent.clipped {
                    let mut paint = bounds;
                    for stroke in node.strokes.iter().filter(|s| s.visible) {
                        if stroke.width_profile.is_uniform() { paint = paint.inflate(stroke.width.max(0.0)); }
                    }
                    if let Some(widths) = self.variable_strokes.get(&id) {
                        paint = paint.union(transform.transform_rect(widths.bounds()));
                    }
                    for effect in node.effects.iter().filter(|e| e.visible) {
                        paint = paint.union(
                            Rect::new(
                                bounds.x + effect.offset.x,
                                bounds.y + effect.offset.y,
                                bounds.width,
                                bounds.height,
                            )
                            .inflate(effect.radius * 2.0 + effect.spread.max(0.0)),
                        );
                    }
                    if let Some(clip) = parent.clip {
                        if let Some(intersection) = paint.intersection(clip) {
                            self.spatial.insert(id, intersection);
                            indexed = true;
                        }
                    } else {
                        self.spatial.insert(id, paint);
                        indexed = true;
                    }
                }
            }
            if !indexed {
                self.spatial.remove(id);
            }
            let mut next = WalkState {
                transform,
                visible,
                operand: parent.operand || node.kind == NodeKind::Boolean,
                ..parent
            };
            if node.clip_content {
                next.intersect_clip(bounds);
            }
            stack.extend(node.children.iter().rev().map(|child| (*child, next)));
        }
    }
    pub fn visible_nodes(&self, r: Rect) -> Vec<NodeId> {
        let mut ids = self.spatial.query(r);
        ids.sort_unstable_by_key(|id| self.z_order.get(id).copied().unwrap_or(0));
        ids
    }
    pub fn hit_test(
        &self,
        d: &Document,
        p: Point,
        tolerance: f64,
        include_locked: bool,
    ) -> Option<NodeId> {
        self.hit_test_filtered(d, p, tolerance, include_locked, |_| true)
    }
    /// Exact geometry hits in front-to-back painter order. The spatial query
    /// visits only intersecting buckets; overlap menus and cycling never need
    /// to enumerate the page. Hidden, locked and ancestor-clipped shapes obey
    /// the same rules as ordinary picking.
    pub fn hit_test_candidates(
        &self,
        d: &Document,
        p: Point,
        tolerance: f64,
        include_locked: bool,
    ) -> Vec<NodeId> {
        self.point_candidates(p, tolerance)
            .into_iter()
            .filter(|&id| self.is_hit(d, id, p, tolerance, include_locked, None))
            .collect()
    }
    /// Apply a renderer-specific visibility predicate after exact geometry and
    /// clipping. Rejected front layers cannot swallow a visible layer behind
    /// them; ordinary picking stops as soon as the first accepted hit is found.
    pub fn hit_test_filtered(
        &self,
        d: &Document,
        p: Point,
        tolerance: f64,
        include_locked: bool,
        mut accept: impl FnMut(NodeId) -> bool,
    ) -> Option<NodeId> {
        self.point_candidates(p, tolerance)
            .into_iter()
            .find(|&id| self.is_hit(d, id, p, tolerance, include_locked, None) && accept(id))
    }
    fn point_candidates(&self, p: Point, tolerance: f64) -> Vec<NodeId> {
        if !valid_pick(p, tolerance) {
            return Vec::new();
        }
        let mut ids = self.spatial.query(Rect::new(
            p.x - tolerance,
            p.y - tolerance,
            tolerance * 2.0,
            tolerance * 2.0,
        ));
        ids.sort_unstable_by_key(|id| {
            std::cmp::Reverse(self.z_order.get(id).copied().unwrap_or(0))
        });
        ids
    }
    /// Limit the broadphase before z-order selection so overlapping unrelated
    /// page frames cannot swallow presentation input. Presentation roots are
    /// rendered in isolation: retain their inherited transform, but disregard
    /// clipping and Boolean ownership above them. Cross-page roots and roots
    /// excluded by an authored ancestor clip need an independent subtree query.
    pub fn hit_test_in_subtree(
        &self,
        d: &Document,
        root: NodeId,
        p: Point,
        tolerance: f64,
        include_locked: bool,
    ) -> Option<NodeId> {
        self.hit_test_in_subtree_filtered(d, root, p, tolerance, include_locked, |_| true)
    }
    /// Ordered exact hits in an isolated rendered subtree. External clips and
    /// Boolean ownership are intentionally ignored, matching presentation.
    pub fn hit_test_candidates_in_subtree(
        &self,
        d: &Document,
        root: NodeId,
        p: Point,
        tolerance: f64,
        include_locked: bool,
    ) -> Vec<NodeId> {
        self.subtree_point_candidates(d, root, p, tolerance)
            .into_iter()
            .filter(|&id| self.is_hit(d, id, p, tolerance, include_locked, Some(root)))
            .collect()
    }
    pub fn hit_test_in_subtree_filtered(
        &self,
        d: &Document,
        root: NodeId,
        p: Point,
        tolerance: f64,
        include_locked: bool,
        mut accept: impl FnMut(NodeId) -> bool,
    ) -> Option<NodeId> {
        self.subtree_point_candidates(d, root, p, tolerance)
            .into_iter()
            .find(|&id| self.is_hit(d, id, p, tolerance, include_locked, Some(root)) && accept(id))
    }
    fn subtree_point_candidates(
        &self,
        d: &Document,
        root: NodeId,
        p: Point,
        tolerance: f64,
    ) -> Vec<NodeId> {
        if !valid_pick(p, tolerance) || !d.nodes.contains_key(&root) {
            return Vec::new();
        }
        let mut parent = d.nodes.get(&root).and_then(|node| node.parent);
        let mut externally_clipped = false;
        for _ in 0..MAX_DOCUMENT_DEPTH {
            let Some(id) = parent else { break };
            let Some(node) = d.nodes.get(&id) else { break };
            if node.clip_content || node.kind == NodeKind::Boolean {
                externally_clipped = true;
                break;
            }
            parent = node.parent;
        }
        if self.transforms.contains_key(&root) && !externally_clipped {
            let mut ids = self.point_candidates(p, tolerance);
            ids.retain(|id| *id == root || d.is_ancestor(root, *id));
            ids
        } else {
            let mut ids = d.descendants(root);
            ids.reverse();
            ids
        }
    }
    fn is_hit(
        &self,
        d: &Document,
        id: NodeId,
        p: Point,
        tolerance: f64,
        include_locked: bool,
        root: Option<NodeId>,
    ) -> bool {
        let Some(n) = d.nodes.get(&id) else {
            return false;
        };
        if !d.effective_visible(id) || (!include_locked && d.effective_locked(id)) {
            return false;
        }
        let mut parent = if root == Some(id) { None } else { n.parent };
        for _ in 0..MAX_DOCUMENT_DEPTH {
            let Some(pid) = parent else { break };
            let Some(n) = d.nodes.get(&pid) else { break };
            if n.kind == NodeKind::Boolean {
                return false;
            }
            if n.clip_content {
                if let Some(inv) = self
                    .transforms
                    .get(&pid)
                    .copied()
                    .or_else(|| d.world_transform(pid))
                    .and_then(Transform::inverse)
                {
                    if !n.contains_clip_point(inv.apply(p)) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            if root == Some(pid) {
                break;
            }
            parent = n.parent
        }
        if n.kind != NodeKind::Boolean {
            if let Some(inverse) = self.transforms.get(&id).copied().or_else(|| d.world_transform(id)).and_then(Transform::inverse) {
                return n.hit_test_local_prepared(inverse.apply(p), tolerance, self.variable_strokes.get(&id));
            }
        }
        d.hit_test_node(id, p, tolerance)
    }
}

fn valid_pick(p: Point, tolerance: f64) -> bool {
    p.x.is_finite() && p.y.is_finite() && tolerance.is_finite() && tolerance >= 0.0
}

/// A retained index for one independently rendered subtree, including a root
/// on another page or outside an authored ancestor's clip. Build once for the
/// document revision and reuse it across presentation pointer events. It keeps
/// inherited affine transforms and internal clips without cloning artwork.
#[derive(Clone, Debug)]
pub struct IsolatedSelectionIndex {
    root: NodeId,
    scene: SceneIndex,
}

impl IsolatedSelectionIndex {
    pub fn new(document: &Document, root: NodeId) -> Option<Self> {
        let node = document.nodes.get(&root)?;
        let transform = match node.parent {
            Some(parent) => document.world_transform(parent)?,
            None => Transform::IDENTITY,
        };
        let mut scene = SceneIndex::default();
        scene.walk(
            document,
            vec![(
                root,
                WalkState {
                    transform,
                    visible: document.effective_visible(root),
                    ..WalkState::default()
                },
            )],
            true,
        );
        scene.revision = document.revision;
        Some(Self { root, scene })
    }

    pub fn root(&self) -> NodeId {
        self.root
    }
    pub fn len(&self) -> usize {
        self.scene.spatial.len()
    }
    pub fn is_empty(&self) -> bool {
        self.scene.spatial.is_empty()
    }

    pub fn hit_test_candidates(
        &self,
        document: &Document,
        point: Point,
        tolerance: f64,
        include_locked: bool,
    ) -> Vec<NodeId> {
        self.scene
            .point_candidates(point, tolerance)
            .into_iter()
            .filter(|&id| {
                self.scene.is_hit(
                    document,
                    id,
                    point,
                    tolerance,
                    include_locked,
                    Some(self.root),
                )
            })
            .collect()
    }

    pub fn hit_test_filtered(
        &self,
        document: &Document,
        point: Point,
        tolerance: f64,
        include_locked: bool,
        mut accept: impl FnMut(NodeId) -> bool,
    ) -> Option<NodeId> {
        self.scene
            .point_candidates(point, tolerance)
            .into_iter()
            .find(|&id| {
                self.scene.is_hit(
                    document,
                    id,
                    point,
                    tolerance,
                    include_locked,
                    Some(self.root),
                ) && accept(id)
            })
    }
}
