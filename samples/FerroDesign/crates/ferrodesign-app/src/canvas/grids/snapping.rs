//! Retained guide-only broadphase. Pointer queries have explicit limits and
//! never enumerate ordinary artwork or expand a guide's authored band count.
use super::*;
use std::collections::BTreeMap;

const CELL_SIZE: f64 = 512.0;
const MAX_OWNER_CELLS: usize = 64;
const MAX_QUERY_CELLS: usize = 64;
const MAX_QUERY_ENTRIES: usize = 4_096;
pub(super) const MAX_SNAP_OWNERS: usize = 256;

#[derive(Default)]
pub(super) struct GuideSnapIndex {
    bounds: BTreeMap<NodeId, Rect>,
    cells: BTreeMap<(i64, i64), BTreeSet<NodeId>>,
    large: BTreeSet<NodeId>,
}

fn cells(bounds: Rect, limit: usize) -> Option<Vec<(i64, i64)>> {
    if !bounds.is_finite() || bounds.width < 0.0 || bounds.height < 0.0 { return None; }
    let values = [bounds.x, bounds.y, bounds.right(), bounds.bottom()].map(|v| (v / CELL_SIZE).floor());
    if values.iter().any(|v| !v.is_finite() || *v <= i64::MIN as f64 || *v >= i64::MAX as f64) { return None; }
    let [x, y, right, bottom] = values.map(|v| v as i64);
    let width = right.checked_sub(x)?.checked_add(1)? as usize;
    let height = bottom.checked_sub(y)?.checked_add(1)? as usize;
    if width.checked_mul(height)? > limit { return None; }
    let mut result = Vec::with_capacity(width * height);
    for dx in 0..width { for dy in 0..height { result.push((x + dx as i64, y + dy as i64)); } }
    Some(result)
}

impl GuideSnapIndex {
    pub(super) fn clear(&mut self) { self.bounds.clear(); self.cells.clear(); self.large.clear(); }

    pub(super) fn update(&mut self, editor: &Editor, id: NodeId) {
        if let Some(old) = self.bounds.remove(&id) {
            if let Some(keys) = cells(old, MAX_OWNER_CELLS) {
                for key in keys {
                    let empty = if let Some(bucket) = self.cells.get_mut(&key) { bucket.remove(&id); bucket.is_empty() } else { false };
                    if empty { self.cells.remove(&key); }
                }
            }
            self.large.remove(&id);
        }
        let document = &editor.document;
        let Some(node) = document.nodes.get(&id) else { return; };
        if node.layout_guide_count() == 0 || !LayoutGrid::supports(node.kind)
            || document.page_of(id) != Some(editor.active_page) || !document.effective_visible(id) { return; }
        let Some(world) = document.world_transform(id) else { return; };
        let bounds = world.transform_rect(node.local_bounds());
        if !bounds.is_finite() { return; }
        if let Some(keys) = cells(bounds, MAX_OWNER_CELLS) {
            for key in keys { self.cells.entry(key).or_default().insert(id); }
        } else { self.large.insert(id); }
        self.bounds.insert(id, bounds);
    }

    fn near(&self, id: NodeId, points: &[Point], threshold: f64) -> bool {
        self.bounds.get(&id).is_some_and(|bounds| points.iter().take(core::MAX_LAYOUT_GUIDE_SNAP_POINTS)
            .any(|point| bounds.inflate(threshold).contains(*point)))
    }

    fn query(&self, points: &[Point], threshold: f64, preferred: &[NodeId]) -> Vec<NodeId> {
        let mut result = Vec::new(); let mut seen = BTreeSet::new();
        // A selected child's own frame takes precedence over crowded siblings,
        // and cannot disappear behind the defensive scene-query budget.
        for &id in preferred.iter().take(MAX_SNAP_OWNERS) {
            if seen.insert(id) && self.near(id, points, threshold) { result.push(id); }
            if result.len() == MAX_SNAP_OWNERS { return result; }
        }
        let mut query_cells = BTreeSet::new();
        for point in points.iter().take(core::MAX_LAYOUT_GUIDE_SNAP_POINTS) {
            if let Some(keys) = cells(Rect::new(point.x - threshold, point.y - threshold, threshold * 2.0, threshold * 2.0), MAX_QUERY_CELLS) {
                for key in keys { if query_cells.len() < MAX_QUERY_CELLS { query_cells.insert(key); } }
            }
        }
        let mut examined = 0;
        for &id in self.large.iter().chain(query_cells.iter().filter_map(|key| self.cells.get(key)).flat_map(|bucket| bucket.iter())) {
            examined += 1;
            if examined > MAX_QUERY_ENTRIES { break; }
            if seen.insert(id) && self.near(id, points, threshold) { result.push(id); }
            if result.len() == MAX_SNAP_OWNERS { break; }
        }
        result
    }
}

fn guide_clips(editor: &Editor, owner: NodeId) -> Option<Vec<(&Node, core::Transform)>> {
    let document = &editor.document;
    let mut clips = Vec::new();
    let mut next = Some(owner);
    for _ in 0..core::MAX_DOCUMENT_DEPTH {
        let Some(id) = next else { return Some(clips); };
        let Some(node) = document.nodes.get(&id) else { return document.page(id).is_some().then_some(clips); };
        if node.clip_content || id == owner {
            let world = editor.scene.transforms.get(&id).copied().or_else(|| document.world_transform(id))?;
            clips.push((node, world.inverse()?));
        }
        next = node.parent;
    }
    None
}

impl LayoutGridOverlay {
    pub(in crate::canvas) fn clear_snap(&mut self) { self.active_snap = None; }

    pub(in crate::canvas) fn snap_bounds(&mut self, editor: &Editor, bounds: Rect, threshold: f64,
        exclude: &[NodeId], direction: Option<Point>) -> Option<core::LayoutGuideSnap> {
        let center = bounds.center();
        let points = [Point::new(bounds.x, bounds.y), Point::new(bounds.right(), bounds.y),
            Point::new(bounds.right(), bounds.bottom()), Point::new(bounds.x, bounds.bottom()), center,
            Point::new(center.x, bounds.y), Point::new(bounds.right(), center.y),
            Point::new(center.x, bounds.bottom()), Point::new(bounds.x, center.y)];
        self.snap(editor, &points, threshold, exclude, direction)
    }

    /// Returns a bounded guide correction. Canvas callers retain their existing
    /// transaction and locked-node checks; passive references may be locked.
    pub(in crate::canvas) fn snap(&mut self, editor: &Editor, points: &[Point], threshold: f64,
        exclude: &[NodeId], direction: Option<Point>) -> Option<core::LayoutGuideSnap> {
        self.active_snap = None;
        if !threshold.is_finite() || threshold <= 0.0 { return None; }
        self.refresh(editor);
        let document = &editor.document;
        let mut preferred = Vec::new(); let mut ancestors = BTreeSet::new();
        for &id in exclude.iter().take(core::MAX_LAYOUT_GUIDE_SNAP_POINTS) {
            let mut next = document.nodes.get(&id).and_then(|node| node.parent);
            for _ in 0..core::MAX_DOCUMENT_DEPTH {
                let Some(id) = next else { break; };
                if !ancestors.insert(id) { break; }
                if self.grids.contains(&id) && preferred.len() < MAX_SNAP_OWNERS { preferred.push(id); }
                next = document.nodes.get(&id).and_then(|node| node.parent);
            }
        }
        let excluded: BTreeSet<_> = exclude.iter().copied().collect();
        for id in self.snap_index.query(points, threshold, &preferred) {
            // Avoid snapping a moving frame to itself or to one of its own
            // descendant guides. Ancestors remain useful passive references.
            let mut next = Some(id); let mut excluded_owner = false;
            for _ in 0..core::MAX_DOCUMENT_DEPTH {
                let Some(id) = next else { break; };
                if excluded.contains(&id) { excluded_owner = true; break; }
                next = document.nodes.get(&id).and_then(|node| node.parent);
            }
            if excluded_owner { continue; }
            let Some(node) = document.nodes.get(&id) else { continue; };
            let Some(world) = document.world_transform(id) else { continue; };
            let Some(clips) = guide_clips(editor, id) else { continue; };
            let Some(candidate) = core::snap_to_layout_guides(node, world, points, threshold, direction,
                |point| clips.iter().all(|(node, inverse)| node.hit_test_local(inverse.apply(point), 0.0))) else { continue; };
            if self.active_snap.is_none_or(|previous| candidate.delta.x.hypot(candidate.delta.y) < previous.delta.x.hypot(previous.delta.y)) {
                self.active_snap = Some(candidate);
            }
            // Own frame beats unrelated overlapping artwork. Stack order inside
            // it remains deterministic in the constant-work geometry helper.
            if preferred.contains(&id) { break; }
        }
        self.active_snap
    }

    /// Called with the world-camera transform already on the drawing stack.
    pub(super) fn render_snap(&self, ctx: &mut DrawingContext, editor: &Editor, viewport: Viewport) {
        let Some(snap) = self.active_snap else { return; };
        let document = &editor.document;
        for line in snap.lines.into_iter().flatten() {
            let Some(node) = document.nodes.get(&line.owner) else { continue; };
            if document.page_of(line.owner) != Some(editor.active_page) || !document.effective_visible(line.owner)
                || node.layout_guide(line.guide).is_none() { continue; }
            let Some(world) = document.world_transform(line.owner) else { continue; };
            let det = (world.a * world.d - world.b * world.c).abs();
            let scale = det / if line.vertical { world.c.hypot(world.d) } else { world.a.hypot(world.b) } * viewport.zoom;
            if !scale.is_finite() || scale <= 0.0 { continue; }
            let mut chain = Vec::new(); let mut next = Some(line.owner);
            for _ in 0..core::MAX_DOCUMENT_DEPTH {
                let Some(id) = next else { break; };
                let Some(node) = document.nodes.get(&id) else { break; };
                chain.push(node); next = node.parent;
            }
            let mut states = Vec::with_capacity(chain.len() * 2);
            for ancestor in chain.into_iter().rev() {
                let t = ancestor.local_transform();
                states.push(ctx.push_transform(Matrix::new(t.a, t.b, t.c, t.d, t.e, t.f)));
                if ancestor.clip_content || ancestor.id == line.owner {
                    states.push(if ancestor.corner_smoothing > 0.0 { ctx.push_geometry_clip(&crate::renderer::corner_geometry(ancestor)) }
                        else { ctx.push_rounded_clip(crate::renderer::rounded_rect(ancestor)) });
                }
            }
            let stroke = pen(0xFFDD498B, 1.0 / scale);
            let (start, end) = if line.vertical { (UiPoint::new(line.coordinate, 0.0), UiPoint::new(line.coordinate, node.height)) }
                else { (UiPoint::new(0.0, line.coordinate), UiPoint::new(node.width, line.coordinate)) };
            ctx.draw_line(&stroke, start, end);
            for state in states.into_iter().rev() { ctx.pop(state); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(document: &mut core::Document, parent: NodeId, x: f64, y: f64) -> NodeId {
        let mut node = Node::new(NodeKind::Frame, "Snap frame", x, y, 300.0, 200.0);
        node.set_layout_guides(&[LayoutGrid { size: 20.0, ..LayoutGrid::default() }]);
        document.insert_node(parent, node).unwrap()
    }

    #[test]
    fn retained_snap_index_ignores_plain_layers_and_tracks_parent_geometry_visibility_and_pages() {
        let mut document = core::Document::new("Guide snapping");
        let parent = document.insert_node(1, Node::new(NodeKind::Group, "Parent", 0.0, 0.0, 400.0, 300.0)).unwrap();
        let owner = frame(&mut document, parent, 0.0, 0.0);
        for index in 0..1000 { document.insert_node(1, Node::new(NodeKind::Rectangle, "Ordinary", index as f64, 20.0, 10.0, 10.0)).unwrap(); }
        let other_page = document.add_page("Other"); frame(&mut document, other_page, 0.0, 0.0);
        let mut editor = Editor::new(document); let mut overlay = LayoutGridOverlay::default();
        let points = [Point::new(38.0, 63.0)];
        let snap = overlay.snap(&editor, &points, 5.0, &[], None).unwrap();
        assert_eq!(snap.delta, Point::new(2.0, -3.0)); assert_eq!(overlay.snap_index.bounds.len(), 1);
        let examined = overlay.examined;
        for _ in 0..100 { overlay.snap(&editor, &points, 5.0, &[], None); }
        assert_eq!(overlay.examined, examined);
        editor.update_node(parent, "Move parent", |node| node.x = 500.0);
        assert!(overlay.snap(&editor, &points, 5.0, &[], None).is_none());
        assert!(overlay.snap(&editor, &[Point::new(538.0, 63.0)], 5.0, &[], None).is_some());
        editor.update_node(owner, "Hide guide owner", |node| node.visible = false);
        assert!(overlay.snap(&editor, &[Point::new(538.0, 63.0)], 5.0, &[], None).is_none());
    }

    #[test]
    fn self_descendant_exclusion_ancestor_priority_and_rounded_clips_are_honored() {
        let mut document = core::Document::new("Clip snapping");
        let owner = frame(&mut document, 1, 0.0, 0.0);
        document.nodes.get_mut(&owner).unwrap().corner_radius = 40.0;
        let child = document.insert_node(owner, Node::new(NodeKind::Rectangle, "Child", 40.0, 60.0, 20.0, 20.0)).unwrap();
        let mut editor = Editor::new(document); let mut overlay = LayoutGridOverlay::default();
        assert!(overlay.snap(&editor, &[Point::new(1.0, 1.0)], 5.0, &[child], None).is_none());
        let snap = overlay.snap(&editor, &[Point::new(38.0, 63.0)], 5.0, &[child], None).unwrap();
        assert_eq!(snap.lines[0].unwrap().owner, owner);
        assert!(overlay.snap(&editor, &[Point::new(38.0, 63.0)], 5.0, &[owner], None).is_none());
        editor.update_node(owner, "Lock passive guide", |node| node.locked = true);
        assert!(overlay.snap(&editor, &[Point::new(38.0, 63.0)], 5.0, &[], None).is_some());
    }

    #[test]
    fn snap_broadphase_has_explicit_query_budget_even_for_overlapping_guide_frames() {
        let mut document = core::Document::new("Bounded snapping");
        let mut last = 0;
        for _ in 0..600 { last = frame(&mut document, 1, 0.0, 0.0); }
        let editor = Editor::new(document); let mut overlay = LayoutGridOverlay::default(); overlay.refresh(&editor);
        let points = [Point::new(38.0, 63.0)];
        let candidates = overlay.snap_index.query(&points, 5.0, &[last]);
        assert_eq!(candidates[0], last); assert_eq!(candidates.len(), MAX_SNAP_OWNERS);
        assert_eq!(cells(Rect::new(-1e30, -1e30, 2e30, 2e30), MAX_OWNER_CELLS), None);
    }
}
