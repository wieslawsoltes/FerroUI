//! Direct editing of stable graph vertices, edges and explicit fill regions.
use super::*;
use ferrodesign_core::NetworkPointId;

impl DesignCanvas {
    pub fn active_network_region(&self) -> Option<(NodeId, core::VectorRegionId)> {
        let id = self.vector_node.get()?;
        let region = self.network_region.get()?;
        let editor = self.editor.borrow();
        (editor.selection.as_slice() == [id]
            && editor.document.nodes.get(&id).and_then(|node| node.vector_network.as_ref())
                .is_some_and(|network| network.regions.contains_key(&region)))
            .then_some((id, region))
    }

    pub fn convert_network(&self) -> bool {
        let id = self.editor.borrow().selection.first().copied();
        let Some(id) = id else { return false; };
        let changed = self.editor.borrow_mut().convert_vector_to_network(id);
        if changed {
            self.tool.set(Tool::Select);
            self.vector_node.set(Some(id));
            self.vector_point.set(None);
            self.network_point.set(None);
            self.network_region.set(None);
            self.network_vertices.borrow_mut().clear();
            self.emit(CanvasEvent::Document);
        }
        changed
    }

    pub fn network_connect_selection(&self) -> bool {
        let Some(id) = self.vector_node.get() else { return false; };
        let vertices = self.network_vertices.borrow().clone();
        if vertices.len() != 2 { return false; }
        let changed = self.editor.borrow_mut().connect_network_vertices(id, vertices[0], vertices[1]).is_some();
        if changed { self.emit(CanvasEvent::Document); }
        changed
    }

    pub fn network_fill_selection(&self) -> bool {
        let Some(id) = self.vector_node.get() else { return false; };
        let vertices = self.network_vertices.borrow().clone();
        let mut editor = self.editor.borrow_mut();
        editor.begin_node_transaction(&[id], "Fill network region");
        let changed = editor.create_network_region_from_vertices(id, &vertices).is_some();
        if changed && editor.document.nodes.get(&id).is_some_and(|node| node.fills.is_empty()) {
            editor.update_node(id, "Fill network region", |node| node.fills.push(core::Fill::solid(core::Color::from_hex("D9D9D9").unwrap())));
        }
        editor.commit_transaction();
        drop(editor);
        if changed { self.emit(CanvasEvent::Document); }
        changed
    }

    pub fn network_weld_selection(&self) -> bool {
        let Some(id) = self.vector_node.get() else { return false; };
        let vertices = self.network_vertices.borrow().clone();
        if vertices.len() != 2 { return false; }
        let changed = self.editor.borrow_mut().merge_network_vertices(id, vertices[0], vertices[1]);
        if changed {
            *self.network_vertices.borrow_mut() = vec![vertices[1]];
            self.network_point.set(Some(NetworkPointId::Vertex(vertices[1])));
            self.emit(CanvasEvent::Document);
        }
        changed
    }

    pub fn network_fill_faces(&self) -> Result<bool, core::NetworkFaceError> {
        let id = self.vector_node.get().ok_or(core::NetworkFaceError::NotEditable)?;
        // Preflight finishes before the core opens a scoped edit. Rejected and
        // repeated actions therefore do not advance even the editor revision.
        let result = self.editor.borrow_mut().fill_network_faces(id);
        if matches!(result, Ok(true)) { self.emit(CanvasEvent::Document); }
        result
    }

    pub(super) fn delete_network_selection(&self) -> bool {
        let Some(id) = self.vector_node.get() else { return false; };
        if self.editor.borrow().document.nodes.get(&id).and_then(|n| n.vector_network.as_ref()).is_none() {
            return false;
        }
        let vertices = self.network_vertices.borrow().clone();
        let mut editor = self.editor.borrow_mut();
        editor.begin_node_transaction(&[id], "Delete network vertices");
        for vertex in vertices { editor.remove_network_vertex(id, vertex); }
        editor.commit_transaction();
        drop(editor);
        self.network_point.set(None);
        self.network_vertices.borrow_mut().clear();
        self.emit(CanvasEvent::Document);
        true
    }

    pub(super) fn network_press(&self, world: Point, clicks: i32, additive: bool) -> bool {
        let Some(id) = self.vector_node.get() else { return false; };
        let viewport = self.viewport.get();
        let pointer = viewport.world_to_screen(world);
        let editor = self.editor.borrow();
        let Some(node) = editor.document.nodes.get(&id) else { return false; };
        let Some(network) = node.vector_network.as_ref() else { return false; };
        let Some(transform) = editor.document.world_transform(id) else { return false; };
        if self.vector_tool.get() == VectorTool::Bucket {
            drop(editor);
            return self.network_bucket_press(world, additive);
        }
        self.network_region.set(None);
        let active_vertex = self.network_point.get().and_then(|p| network.point_vertex(p));
        let points = network.edit_points();
        let hit = points.iter().filter(|p| matches!(p.id, NetworkPointId::Vertex(_)) || Some(p.vertex) == active_vertex)
            .filter_map(|point| {
                let distance = viewport.world_to_screen(transform.apply(point.position)).distance(pointer);
                (distance <= 8.0).then_some((*point, distance))
            })
            .min_by(|(a, da), (b, db)| da.total_cmp(db).then_with(|| {
                (!matches!(a.id, NetworkPointId::Vertex(_))).cmp(&(!matches!(b.id, NetworkPointId::Vertex(_))))
            })).map(|(point, _)| point);
        let edge = network.nearest_segment_transformed(world, transform)
            .filter(|hit| viewport.world_to_screen(hit.position).distance(pointer) <= 8.0);
        drop(editor);

        if self.vector_tool.get() == VectorTool::Connect {
            let vertex = hit.filter(|p| matches!(p.id, NetworkPointId::Vertex(_))).map(|p| p.vertex);
            let selected = self.network_vertices.borrow().last().copied();
            let mut editor = self.editor.borrow_mut();
            let mut changed = false;
            let next = match (selected, vertex) {
                (Some(start), Some(end)) => {
                    if start != end { changed = editor.connect_network_vertices(id, start, end).is_some(); }
                    Some(end)
                }
                (Some(start), None) => {
                    let result = editor.branch_network_vertex(id, start, self.snap_point(world));
                    changed = result.is_some(); result.map(|(vertex, _)| vertex)
                }
                (None, Some(vertex)) => Some(vertex),
                (None, None) => {
                    let result = editor.add_network_vertex(id, self.snap_point(world));
                    changed = result.is_some(); result
                }
            };
            drop(editor);
            if let Some(vertex) = next {
                self.network_point.set(Some(NetworkPointId::Vertex(vertex)));
                *self.network_vertices.borrow_mut() = vec![vertex];
            }
            self.emit(if changed { CanvasEvent::Document } else { CanvasEvent::Selection });
            self.invalidate_visual();
            return true;
        }
        if self.vector_tool.get() == VectorTool::Erase {
            let changed = if let Some(point) = hit.filter(|p| matches!(p.id, NetworkPointId::Vertex(_))) {
                self.editor.borrow_mut().remove_network_vertex(id, point.vertex)
            } else if let Some(edge) = edge {
                self.editor.borrow_mut().remove_network_edge(id, edge.edge)
            } else { false };
            if changed { self.emit(CanvasEvent::Document); }
            self.invalidate_visual();
            return true;
        }
        if self.vector_tool.get() == VectorTool::Bend {
            if let Some(edge) = edge {
                self.editor.borrow_mut().begin_node_transaction(&[id], "Bend network edge");
                *self.gesture.borrow_mut() = Gesture::NetworkBend { node: id, edge: edge.edge, t: edge.t };
            }
            return true;
        }
        if let Some(point) = hit {
            self.network_point.set(Some(point.id));
            if matches!(point.id, NetworkPointId::Vertex(_)) {
                let mut selected = self.network_vertices.borrow_mut();
                if additive {
                    if selected.contains(&point.vertex) { selected.retain(|v| *v != point.vertex); }
                    else { selected.push(point.vertex); }
                } else if !selected.contains(&point.vertex) {
                    *selected = vec![point.vertex];
                }
            }
            self.editor.borrow_mut().begin_node_transaction(&[id], "Move network point");
            *self.gesture.borrow_mut() = Gesture::NetworkPoint {
                node: id, point: point.id, start: world, original: transform.apply(point.position),
            };
            self.emit(CanvasEvent::Selection);
        } else if let Some(edge) = edge {
            if clicks >= 2 {
                if let Some(vertex) = self.editor.borrow_mut().split_network_edge(id, edge.edge, edge.t) {
                    self.network_point.set(Some(NetworkPointId::Vertex(vertex)));
                    *self.network_vertices.borrow_mut() = vec![vertex];
                }
                self.emit(CanvasEvent::Document);
            } else {
                self.network_point.set(None);
                self.network_vertices.borrow_mut().clear();
                self.emit(CanvasEvent::Selection);
            }
        } else {
            self.network_point.set(None);
            self.network_vertices.borrow_mut().clear();
            self.emit(CanvasEvent::Selection);
        }
        self.invalidate_visual();
        true
    }
}

pub(super) fn render_network_editor(
    ctx: &mut DrawingContext, network: &core::VectorNetwork, transform: core::Transform,
    viewport: Viewport, selected: Option<NetworkPointId>, selected_vertices: &[core::VectorVertexId],
) {
    let to_screen = |point| ui(viewport.world_to_screen(transform.apply(point)));
    let path = network.stroke_path().transformed(transform);
    let geometry = crate::renderer::path_geometry(&path);
    let state = ctx.push_transform(Matrix::create_scale(viewport.zoom, viewport.zoom)
        * Matrix::create_translation(viewport.pan.x, viewport.pan.y));
    let outline = pen(0xFF0D99FF, 1.0 / viewport.zoom);
    ctx.draw_geometry(None, Some(&outline), &geometry);
    ctx.pop(state);
    let blue = pen(0xFF0D99FF, 1.0);
    let blue_fill = brush("#0D99FF");
    let white = brush("#FFFFFF");
    let anchor = selected.and_then(|point| network.point_vertex(point));
    for point in network.edit_points() {
        let position = to_screen(point.position);
        let vertex = matches!(point.id, NetworkPointId::Vertex(_));
        if !vertex && Some(point.vertex) != anchor { continue; }
        let fill = if selected == Some(point.id) || (vertex && selected_vertices.contains(&point.vertex)) { &blue_fill } else { &white };
        if vertex {
            ctx.draw_rectangle(Some(fill), Some(&blue), UiRect::new(position.x-3.5,position.y-3.5,7.0,7.0),0.0,0.0,&BoxShadows::default());
        } else {
            ctx.draw_line(&blue, to_screen(point.anchor), position);
            ctx.draw_ellipse_at(Some(fill),Some(&blue),position,3.5,3.5);
        }
    }
}
