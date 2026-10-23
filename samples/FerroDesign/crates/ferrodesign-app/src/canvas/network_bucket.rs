//! Captured vector-region paint strokes and immutable native hover adorners.
use super::*;
use ferroui_base::media::Geometry;
use std::collections::BTreeSet;

struct CachedFaces {
    node: NodeId,
    token: core::EditorRenderToken,
    map: Result<Rc<core::NetworkFaceMap>, core::NetworkFaceError>,
}

struct HoverFace {
    node: NodeId,
    face: usize,
    geometry: Ref<Geometry>,
    clear: bool,
}

struct PaintStroke {
    node: NodeId,
    page: NodeId,
    inverse: core::Transform,
    transform: core::Transform,
    last: Point,
    fills: Vec<core::Fill>,
    map: Rc<core::NetworkFaceMap>,
    visited: BTreeSet<usize>,
    transaction: core::EditorTransactionToken,
    token: core::EditorRenderToken,
    changed: bool,
    previous_region: Option<core::VectorRegionId>,
}

impl PaintStroke {
    fn owns_transaction(&self, editor: &Editor) -> bool {
        editor.active_transaction_token().as_ref() == Some(&self.transaction)
            && editor.render_change_token() == self.token
    }

    fn is_current(&self, canvas: &DesignCanvas, editor: &Editor) -> bool {
        self.owns_transaction(editor)
            && canvas.tool.get() == Tool::Select && canvas.vector_tool.get() == VectorTool::Bucket
            && canvas.vector_node.get() == Some(self.node) && canvas.presentation.get().is_none()
            && editor.active_page == self.page && editor.selection.as_slice() == [self.node]
            && editor.document.world_transform(self.node) == Some(self.transform)
            && !editor.document.effective_locked(self.node) && editor.document.effective_visible(self.node)
    }
}

#[derive(Default)]
pub(super) struct BucketState {
    cache: Option<CachedFaces>,
    hover: Option<HoverFace>,
    stroke: Option<PaintStroke>,
    #[cfg(test)]
    preparations: usize,
}

impl DesignCanvas {
    fn bucket_map(&self) -> Result<(NodeId, core::Transform, Rc<core::NetworkFaceMap>), core::NetworkFaceError> {
        let editor = self.editor.borrow();
        let id = self.vector_node.get().ok_or(core::NetworkFaceError::NotEditable)?;
        if self.tool.get() != Tool::Select || self.vector_tool.get() != VectorTool::Bucket
            || self.presentation.get().is_some() || editor.selection.as_slice() != [id]
            || editor.document.effective_locked(id) || !editor.document.effective_visible(id)
        { return Err(core::NetworkFaceError::NotEditable); }
        let node = editor.document.nodes.get(&id).ok_or(core::NetworkFaceError::NotEditable)?;
        let network = node.vector_network.as_ref().ok_or(core::NetworkFaceError::NotEditable)?;
        let transform = editor.document.world_transform(id).ok_or(core::NetworkFaceError::NotEditable)?;
        if transform.inverse().is_none() { return Err(core::NetworkFaceError::NotEditable); }
        let token = editor.render_change_token();
        let mut state = self.network_bucket.borrow_mut();
        if let Some(cache) = &mut state.cache {
            if cache.node == id && cache.token != token && cache.map.as_ref().is_ok_and(|map| map.matches_source(network)) {
                cache.token = token.clone();
            }
        }
        if !state.cache.as_ref().is_some_and(|cache| cache.node == id && cache.token == token) {
            let map = core::NetworkFaceMap::prepare(network).map(Rc::new);
            state.cache = Some(CachedFaces { node: id, token, map });
            state.hover = None;
            #[cfg(test)] { state.preparations += 1; }
        }
        Ok((id, transform, state.cache.as_ref().unwrap().map.clone()?))
    }

    fn update_bucket_hover(&self, id: NodeId, map: &core::NetworkFaceMap, local: Point, clear: bool) {
        let face = map.face_at(local);
        let mut state = self.network_bucket.borrow_mut();
        if state.hover.as_ref().map(|hover| (hover.node, hover.face, hover.clear)) == face.map(|face| (id, face, clear)) { return; }
        state.hover = face.map(|face| HoverFace { node: id, face, clear,
            geometry: crate::renderer::path_geometry(map.path(face).unwrap()) });
        drop(state);
        self.invalidate_visual();
    }

    pub(super) fn clear_bucket_hover(&self) {
        if self.network_bucket.borrow_mut().hover.take().is_some() { self.invalidate_visual(); }
    }

    /// Exposes native state for diagnostics without providing a parallel editor.
    pub fn network_bucket_state(&self) -> (Option<usize>, bool, usize) {
        let state = self.network_bucket.borrow();
        (state.hover.as_ref().map(|hover| hover.face), state.stroke.is_some(),
            state.stroke.as_ref().map_or(0, |stroke| stroke.visited.len()))
    }

    pub(super) fn network_bucket_press(&self, world: Point, clear: bool) -> bool {
        self.cancel_network_bucket_gesture();
        let (id, transform, map) = match self.bucket_map() {
            Ok(prepared) => prepared,
            Err(error) => {
                self.clear_bucket_hover();
                self.emit(CanvasEvent::Notice(error.to_string()));
                return true;
            }
        };
        let inverse = transform.inverse().unwrap();
        let local = inverse.apply(world);
        let stroke = {
            let mut editor = self.editor.borrow_mut();
            // Do not join another inspector or gesture's pending transaction.
            if editor.has_active_transaction() { return true; }
            let node = &editor.document.nodes[&id];
            let fills = if clear { Vec::new() } else if node.fills.is_empty() {
                vec![core::Fill::solid(core::Color::rgb(217, 217, 217))]
            } else { node.fills.clone() };
            editor.begin_node_transaction(&[id], if clear { "Clear vector regions" } else { "Paint vector regions" });
            PaintStroke { node: id, page: editor.active_page, inverse, transform, last: local, fills, map,
                visited: BTreeSet::new(), transaction: editor.active_transaction_token().unwrap(),
                token: editor.render_change_token(), changed: false, previous_region: self.network_region.get() }
        };
        self.network_bucket.borrow_mut().stroke = Some(stroke);
        self.network_point.set(None);
        self.network_vertices.borrow_mut().clear();
        // A lifecycle boundary must see the pending native gesture even when
        // the last committed document is already durably saved.
        self.emit(CanvasEvent::Draft);
        self.network_bucket_move(world, clear);
        true
    }

    pub(super) fn network_bucket_move(&self, world: Point, clear: bool) -> bool {
        let stroke = self.network_bucket.borrow_mut().stroke.take();
        let Some(mut stroke) = stroke else {
            if !matches!(*self.gesture.borrow(), Gesture::None) || self.space.get() {
                self.clear_bucket_hover();
                return false;
            }
            if self.vector_tool.get() != VectorTool::Bucket || self.vector_node.get().is_none() { return false; }
            match self.bucket_map() {
                Ok((id, transform, map)) => self.update_bucket_hover(id, &map, transform.inverse().unwrap().apply(world), clear),
                Err(_) => self.clear_bucket_hover(),
            }
            return true;
        };
        if !stroke.is_current(self, &self.editor.borrow()) {
            self.network_bucket.borrow_mut().stroke = Some(stroke);
            self.cancel_network_bucket_gesture();
            return true;
        }
        let local = stroke.inverse.apply(world);
        let result = stroke.map.faces_along_segment(stroke.last, local).and_then(|faces| {
            let mut editor = self.editor.borrow_mut();
            let points: Vec<_> = faces.into_iter().filter_map(|(face, point)| stroke.visited.insert(face).then_some(point)).collect();
            if !points.is_empty() {
                stroke.changed |= editor.paint_mapped_network_faces(stroke.node, &stroke.map, &points, &stroke.fills)?;
                let point = *points.last().unwrap();
                self.network_region.set(editor.document.nodes.get(&stroke.node).and_then(|node| node.vector_network.as_ref())
                    .and_then(|network| network.region_at(point)));
                stroke.token = editor.render_change_token();
            }
            Ok(())
        });
        stroke.last = local;
        if let Some(cache) = self.network_bucket.borrow_mut().cache.as_mut() { cache.token = stroke.token.clone(); }
        self.update_bucket_hover(stroke.node, &stroke.map, local, stroke.fills.is_empty());
        self.network_bucket.borrow_mut().stroke = Some(stroke);
        if let Err(error) = result {
            self.cancel_network_bucket_gesture();
            self.emit(CanvasEvent::Notice(error.to_string()));
        } else {
            self.invalidate_visual();
        }
        true
    }

    pub fn cancel_network_bucket_gesture(&self) -> bool { self.finish_network_bucket_gesture(false) }

    /// Lifecycle/save boundary: commit the captured stroke once, before any
    /// external edit can join its history transaction.
    pub fn finish_network_bucket(&self) -> bool { self.finish_network_bucket_gesture(true) }

    pub(super) fn finish_network_bucket_gesture(&self, commit: bool) -> bool {
        let stroke = self.network_bucket.borrow_mut().stroke.take();
        let Some(stroke) = stroke else { return false; };
        let changed = {
            let mut editor = self.editor.borrow_mut();
            if !stroke.owns_transaction(&editor) { false }
            else if commit && stroke.is_current(self, &editor) { editor.commit_transaction() }
            else {
                // Selection/page changes may have caused this cancellation.
                // Preserve them while restoring this gesture's authored graph.
                let selection = editor.selection.clone();
                let page = editor.active_page;
                editor.cancel_transaction();
                editor.selection = selection;
                editor.active_page = page;
                self.network_region.set(stroke.previous_region);
                stroke.changed
            }
        };
        self.clear_bucket_hover();
        self.invalidate_visual();
        self.emit(CanvasEvent::Draft);
        self.emit(if changed { CanvasEvent::Document } else { CanvasEvent::Selection });
        true
    }

    pub(super) fn reconcile_network_bucket(&self) {
        let invalid = self.network_bucket.borrow().stroke.as_ref()
            .is_some_and(|stroke| !stroke.is_current(self, &self.editor.borrow()));
        if invalid { self.cancel_network_bucket_gesture(); }
        if self.vector_tool.get() != VectorTool::Bucket || self.vector_node.get().is_none() {
            self.clear_bucket_hover();
        }
    }

    pub(super) fn render_bucket_hover(&self, ctx: &mut DrawingContext, editor: &Editor, id: NodeId, transform: core::Transform, viewport: Viewport) {
        if self.vector_tool.get() != VectorTool::Bucket { return; }
        let state = self.network_bucket.borrow();
        if !state.cache.as_ref().is_some_and(|cache| cache.node == id && cache.token == editor.render_change_token()) { return; }
        let Some(hover) = state.hover.as_ref().filter(|hover| hover.node == id) else { return; };
        let camera = ctx.push_transform(Matrix::create_scale(viewport.zoom, viewport.zoom)
            * Matrix::create_translation(viewport.pan.x, viewport.pan.y));
        let local = ctx.push_transform(Matrix::new(transform.a, transform.b, transform.c, transform.d, transform.e, transform.f));
        let fill = brush(if hover.clear { "#38EF4444" } else { "#300D99FF" });
        // A screen-scaled outline remains visible while the exact native face
        // geometry carries transparent hover coverage and its original holes.
        let scale = transform.a.hypot(transform.b).max(transform.c.hypot(transform.d)) * viewport.zoom;
        let outline = pen(if hover.clear { 0xFFEF4444 } else { 0xFF0D99FF }, 1.5 / scale.max(0.01));
        ctx.draw_geometry(Some(&fill), Some(&outline), &hover.geometry);
        ctx.pop(local);
        ctx.pop(camera);
    }
}

#[cfg(test)]
#[path = "network_bucket_tests.rs"]
mod tests;
