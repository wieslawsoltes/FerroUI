//! Ordinary Select-mode corner-radius affordances. Artwork geometry comes from
//! the core plan; this module only positions native controls and owns a scoped
//! pointer transaction. Hover never writes to the document.
use super::*;
use ferroui_base::media::text_formatting::{TextLayout, TextLayoutOptions};

const HIT_RADIUS: f64 = 7.0;
const MIN_INSET: f64 = 16.0;
const RESIZE_HIT_RADIUS: f64 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Hover {
    node: NodeId,
    corner: usize,
}

#[derive(Clone, Copy, Debug)]
struct Handle {
    source: core::CornerRadiusHandle,
    local: Point,
    world: Point,
    screen: Point,
    vertex_world: Point,
    vertex_screen: Point,
    world_per_radius: Point,
}

struct CachedHandles {
    node: NodeId,
    token: core::EditorRenderToken,
    transform: core::Transform,
    viewport: Viewport,
    handles: Rc<Vec<Handle>>,
}

struct CachedSpacing {
    // Translation does not change inter-vertex spacing. Radius previews also
    // retain these vertices, so their bounded all-pairs work happens only when
    // the shape/linear transform/zoom changes, never on ordinary hover.
    vertices: Vec<Point>,
    maximum_offsets: Vec<f64>,
}

struct RadiusGesture {
    node: NodeId,
    page: NodeId,
    corner: usize,
    individual: bool,
    transform: core::Transform,
    inverse: core::Transform,
    viewport: Viewport,
    start: Point,
    local_per_radius: Point,
    original_radius: f64,
    radius: f64,
    transaction: core::EditorTransactionToken,
    token: core::EditorRenderToken,
    changed: bool,
}

impl RadiusGesture {
    fn owns_transaction(&self, editor: &Editor) -> bool {
        editor.active_transaction_token().as_ref() == Some(&self.transaction)
            && editor.render_change_token() == self.token
    }

    fn is_current(&self, canvas: &DesignCanvas, editor: &Editor) -> bool {
        self.owns_transaction(editor)
            && canvas.viewport.get() == self.viewport
            && editor.active_page == self.page
            && canvas.radius_target(editor) == Some((self.node, self.transform))
    }
}

#[derive(Default)]
pub(super) struct CornerRadiusState {
    hover: Option<Hover>,
    cache: Option<CachedHandles>,
    spacing: Option<CachedSpacing>,
    gesture: Option<RadiusGesture>,
    label: Option<(String, TextLayout)>,
    #[cfg(test)]
    preparations: usize,
    #[cfg(test)]
    spacing_preparations: usize,
}

fn finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn finite_transform(transform: core::Transform) -> bool {
    [
        transform.a,
        transform.b,
        transform.c,
        transform.d,
        transform.e,
        transform.f,
    ]
    .iter()
    .all(|value| value.is_finite())
}

fn protected_targets(node: &Node, transform: core::Transform, viewport: Viewport) -> [Point; 5] {
    let corners = node
        .local_bounds()
        .corners()
        .map(|point| viewport.world_to_screen(transform.apply(point)));
    let top = (corners[0] + corners[1]) * 0.5;
    let center = (corners[0] + corners[2]) * 0.5;
    let direction = top - center;
    let length = direction.distance(Point::default()).max(0.001);
    [
        corners[0],
        corners[1],
        corners[2],
        corners[3],
        top + direction * (24.0 / length),
    ]
}

impl DesignCanvas {
    fn radius_target(&self, editor: &Editor) -> Option<(NodeId, core::Transform)> {
        if self.tool.get() != Tool::Select
            || self.presentation.get().is_some()
            || self.space.get()
            || self.image_crop.get().is_some()
            || self.stroke_width_mode_active()
            || self.vector_node.get().is_some()
            || self.pen_node.get().is_some()
            || editor.selection.len() != 1
        {
            return None;
        }
        let id = editor.selection[0];
        let node = editor.document.nodes.get(&id)?;
        if !core::supports_corner_radius(node.kind)
            || !editor.document.effective_visible(id)
            || editor.document.effective_locked(id)
            || editor.document.page_of(id) != Some(editor.active_page)
        {
            return None;
        }
        let transform = editor.document.world_transform(id)?;
        if !finite_transform(transform) || !finite_transform(transform.inverse()?) {
            return None;
        }
        let viewport = self.viewport.get();
        (viewport.zoom.is_finite() && viewport.zoom > 0.0 && finite(viewport.pan))
            .then_some((id, transform))
    }

    fn prepared_radius_handles(
        &self,
        editor: &Editor,
        id: NodeId,
        transform: core::Transform,
    ) -> Rc<Vec<Handle>> {
        let token = editor.render_change_token();
        let viewport = self.viewport.get();
        let mut state = self.corner_radius.borrow_mut();
        if let Some(cache) = state.cache.as_ref().filter(|cache| {
            cache.node == id
                && cache.token == token
                && cache.transform == transform
                && cache.viewport == viewport
        }) {
            return cache.handles.clone();
        }
        let node = &editor.document.nodes[&id];
        let source = core::corner_radius_handles(node);
        let vertices: Vec<_> = source
            .iter()
            .map(|handle| transform.apply_vector(handle.vertex) * viewport.zoom)
            .collect();
        if !state
            .spacing
            .as_ref()
            .is_some_and(|spacing| spacing.vertices == vertices)
        {
            let maximum_offsets = vertices
                .iter()
                .enumerate()
                .map(|(index, &vertex)| {
                    vertices
                        .iter()
                        .enumerate()
                        .filter(|(other, _)| *other != index)
                        .map(|(_, &other)| vertex.distance(other))
                        .fold(f64::INFINITY, f64::min)
                        / 3.0
                })
                .collect();
            state.spacing = Some(CachedSpacing {
                vertices,
                maximum_offsets,
            });
            #[cfg(test)]
            {
                state.spacing_preparations += 1;
            }
        }
        let protected = protected_targets(node, transform, viewport);
        let spacing = state.spacing.as_ref().unwrap();
        let handles = source
            .into_iter()
            .zip(&spacing.maximum_offsets)
            .filter_map(|(source, &maximum)| {
                // Distinct corners can share one arc center at the geometric clamp.
                // Staying within one third of their nearest vertex separation keeps
                // every eligible displayed marker distinct and its 7px hit circle
                // accessible. Tiny shapes need zoom before both limits can fit.
                if !maximum.is_finite() || maximum < MIN_INSET {
                    return None;
                }
                let world_per_radius = transform.apply_vector(source.local_per_radius);
                let scale = world_per_radius.x.hypot(world_per_radius.y) * viewport.zoom;
                if !scale.is_finite() || scale <= 0.0 {
                    return None;
                }
                let offset = (source.effective_radius * scale).clamp(MIN_INSET, maximum);
                let local = source.vertex + source.local_per_radius * (offset / scale);
                if !finite(local) || !node.local_bounds().contains(local) {
                    return None;
                }
                let world = transform.apply(local);
                let screen = viewport.world_to_screen(world);
                let vertex_world = transform.apply(source.vertex);
                let vertex_screen = viewport.world_to_screen(vertex_world);
                if ![world, screen, vertex_world, vertex_screen, world_per_radius]
                    .into_iter()
                    .all(finite)
                    || protected
                        .iter()
                        .any(|target| screen.distance(*target) <= HIT_RADIUS + RESIZE_HIT_RADIUS)
                {
                    return None;
                }
                Some(Handle {
                    source,
                    local,
                    world,
                    screen,
                    vertex_world,
                    vertex_screen,
                    world_per_radius,
                })
            })
            .collect();
        let handles = Rc::new(handles);
        state.cache = Some(CachedHandles {
            node: id,
            token,
            transform,
            viewport,
            handles: handles.clone(),
        });
        #[cfg(test)]
        {
            state.preparations += 1;
        }
        handles
    }

    fn radius_hit(
        &self,
        id: NodeId,
        handles: &[Handle],
        world: Point,
        corridor: bool,
    ) -> Option<usize> {
        let pointer = self.viewport.get().world_to_screen(world);
        let previous = self
            .corner_radius
            .borrow()
            .hover
            .filter(|hover| hover.node == id);
        let distance = |handle: &Handle| {
            if corridor {
                core::distance_to_segment(pointer, handle.vertex_screen, handle.screen)
            } else {
                pointer.distance(handle.screen)
            }
        };
        // Preserve a revealed corner along its vertex-to-handle corridor. A
        // press still requires the real circle; it never claims the vertex's
        // resize target or an unrelated Alt layer-cycle click.
        if let Some(corner) = previous.map(|hover| hover.corner) {
            if handles
                .iter()
                .any(|handle| handle.source.corner == corner && distance(handle) <= HIT_RADIUS)
            {
                return Some(corner);
            }
        }
        handles
            .iter()
            .filter_map(|handle| {
                let distance = distance(handle);
                (distance <= HIT_RADIUS).then_some((distance, handle.source.corner))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
            .map(|(_, corner)| corner)
    }

    pub(super) fn corner_radius_hover(&self, world: Point, modifiers: KeyModifiers) -> bool {
        if !finite(world)
            || modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META)
            || !matches!(*self.gesture.borrow(), Gesture::None)
            || self.corner_radius_active()
        {
            self.clear_corner_radius_hover();
            return false;
        }
        let hover = {
            let editor = self.editor.borrow();
            if editor.has_active_transaction() {
                None
            } else {
                self.radius_target(&editor).and_then(|(id, transform)| {
                    let handles = self.prepared_radius_handles(&editor, id, transform);
                    self.radius_hit(id, &handles, world, true)
                        .map(|corner| Hover { node: id, corner })
                })
            }
        };
        let mut state = self.corner_radius.borrow_mut();
        let changed = state.hover != hover;
        state.hover = hover;
        drop(state);
        if changed {
            self.invalidate_visual();
        }
        hover.is_some()
    }

    pub(super) fn clear_corner_radius_hover(&self) {
        if self.corner_radius.borrow_mut().hover.take().is_some() {
            self.invalidate_visual();
        }
    }

    pub(super) fn corner_radius_press(&self, world: Point, modifiers: KeyModifiers) -> bool {
        if !finite(world)
            || modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META)
            || !matches!(*self.gesture.borrow(), Gesture::None)
            || self.handle_at(world).is_some()
        {
            return false;
        }
        let mut editor = self.editor.borrow_mut();
        if editor.has_active_transaction() {
            return false;
        }
        let Some((id, transform)) = self.radius_target(&editor) else {
            return false;
        };
        let handles = self.prepared_radius_handles(&editor, id, transform);
        let Some(corner) = self.radius_hit(id, &handles, world, false) else {
            return false;
        };
        let handle = handles
            .iter()
            .find(|handle| handle.source.corner == corner)
            .unwrap();
        let individual = modifiers.contains(KeyModifiers::ALT)
            && editor.document.nodes[&id].kind == NodeKind::Rectangle;
        editor.begin_node_transaction(
            &[id],
            if individual {
                "Drag individual corner radius"
            } else {
                "Drag corner radius"
            },
        );
        let gesture = RadiusGesture {
            node: id,
            page: editor.active_page,
            corner,
            individual,
            transform,
            inverse: transform.inverse().unwrap(),
            viewport: self.viewport.get(),
            start: world,
            local_per_radius: handle.source.local_per_radius,
            original_radius: handle.source.authored_radius,
            radius: handle.source.authored_radius,
            transaction: editor.active_transaction_token().unwrap(),
            token: editor.render_change_token(),
            changed: false,
        };
        drop(editor);
        {
            let mut state = self.corner_radius.borrow_mut();
            state.hover = Some(Hover { node: id, corner });
            state.gesture = Some(gesture);
        }
        self.emit(CanvasEvent::Draft);
        self.invalidate_visual();
        true
    }

    pub fn corner_radius_active(&self) -> bool {
        self.corner_radius.borrow().gesture.is_some()
    }

    pub(super) fn corner_radius_move(&self, world: Point) -> bool {
        let Some(mut gesture) = self.corner_radius.borrow_mut().gesture.take() else {
            return false;
        };
        if !gesture.is_current(self, &self.editor.borrow()) {
            self.corner_radius.borrow_mut().gesture = Some(gesture);
            self.cancel_corner_radius_gesture();
            return true;
        }
        let delta = gesture.inverse.apply(world) - gesture.inverse.apply(gesture.start);
        let axis = gesture.local_per_radius;
        let length = axis.x.hypot(axis.y);
        if !finite(world) || !finite(delta) || !length.is_finite() || length <= 0.0 {
            self.corner_radius.borrow_mut().gesture = Some(gesture);
            return true;
        }
        let projection = (delta.x * (axis.x / length) + delta.y * (axis.y / length)) / length;
        let screen_axis = gesture.transform.apply_vector(axis);
        let projection_pixels =
            projection.abs() * screen_axis.x.hypot(screen_axis.y) * gesture.viewport.zoom;
        let radius = if projection_pixels <= 1e-6 {
            gesture.original_radius
        } else {
            let value = gesture.original_radius + projection;
            if value.is_finite() {
                value.max(0.0)
            } else if value.is_sign_positive() {
                f64::MAX
            } else {
                0.0
            }
        };
        let mut error = None;
        if projection.is_finite() && radius != gesture.radius {
            let mut editor = self.editor.borrow_mut();
            if radius == gesture.original_radius {
                // Restore the exact bound/independent/override snapshot, not
                // merely its number. A later move starts another owned preview
                // against that same original document; redo remains untouched.
                editor.cancel_transaction();
                editor.begin_node_transaction(
                    &[gesture.node],
                    if gesture.individual {
                        "Drag individual corner radius"
                    } else {
                        "Drag corner radius"
                    },
                );
                gesture.transaction = editor.active_transaction_token().unwrap();
                gesture.token = editor.render_change_token();
                gesture.changed = false;
                gesture.radius = radius;
            } else {
                let result = if gesture.individual {
                    editor.set_individual_corner_radius(&[gesture.node], gesture.corner, radius)
                } else {
                    editor.set_corner_radius(&[gesture.node], radius)
                };
                match result {
                    Ok(changed) => {
                        gesture.changed |= changed > 0;
                        gesture.radius = radius;
                        gesture.token = editor.render_change_token();
                    }
                    Err(value) => error = Some(value.to_string()),
                }
            }
        }
        self.corner_radius.borrow_mut().gesture = Some(gesture);
        if let Some(error) = error {
            self.cancel_corner_radius_gesture();
            self.emit(CanvasEvent::Notice(error));
        } else {
            self.invalidate_visual();
        }
        true
    }

    pub(super) fn cancel_corner_radius_gesture(&self) -> bool {
        self.finish_radius_gesture(false)
    }

    /// Explicit save/pagehide boundaries commit this gesture once. Background
    /// autosave waits until release/cancellation and never serializes a draft.
    pub fn finish_corner_radius_edit(&self) -> bool {
        self.finish_radius_gesture(true)
    }

    fn finish_radius_gesture(&self, commit: bool) -> bool {
        let Some(gesture) = self.corner_radius.borrow_mut().gesture.take() else {
            return false;
        };
        let changed = {
            let mut editor = self.editor.borrow_mut();
            if !gesture.owns_transaction(&editor) {
                false
            } else if commit && gesture.changed && gesture.is_current(self, &editor) {
                editor.commit_transaction()
            } else {
                let selection = editor.selection.clone();
                let page = editor.active_page;
                editor.cancel_transaction();
                editor.selection = selection;
                editor.active_page = page;
                gesture.changed
            }
        };
        self.clear_corner_radius_hover();
        self.invalidate_visual();
        self.emit(CanvasEvent::Draft);
        self.emit(if changed {
            CanvasEvent::Document
        } else {
            CanvasEvent::Selection
        });
        true
    }

    pub(super) fn reconcile_corner_radius(&self) {
        let stale = self
            .corner_radius
            .borrow()
            .gesture
            .as_ref()
            .is_some_and(|gesture| !gesture.is_current(self, &self.editor.borrow()));
        if stale {
            self.cancel_corner_radius_gesture();
        }
        let target = self.radius_target(&self.editor.borrow()).map(|(id, _)| id);
        let stale_hover = self
            .corner_radius
            .borrow()
            .hover
            .is_some_and(|hover| target != Some(hover.node));
        if stale_hover || target.is_none() {
            self.clear_corner_radius_hover();
        }
    }

    /// Observational native input geometry only. The bridge does not expose a
    /// radius mutation, selection command, capture change or history operation.
    pub fn corner_radius_state(&self) -> serde_json::Value {
        let editor = self.editor.borrow();
        let target = self.radius_target(&editor);
        let handles = target
            .map(|(id, transform)| self.prepared_radius_handles(&editor, id, transform))
            .unwrap_or_default();
        let state = self.corner_radius.borrow();
        let node = target.map(|(id, _)| id);
        serde_json::json!({
            "node": node,
            "hover": state.hover.filter(|hover| Some(hover.node) == node).map(|hover| hover.corner),
            "active": state.gesture.is_some(),
            "corner": state.gesture.as_ref().map(|gesture| gesture.corner),
            "individual": state.gesture.as_ref().is_some_and(|gesture| gesture.individual),
            "handles": handles.iter().map(|handle| serde_json::json!({
                "corner": handle.source.corner,
                "vertexLocal": handle.source.vertex, "vertexWorld": handle.vertex_world,
                "local": handle.local, "world": handle.world, "screen": handle.screen,
                "localPerRadius": handle.source.local_per_radius, "worldPerRadius": handle.world_per_radius,
                "authoredRadius": handle.source.authored_radius, "effectiveRadius": handle.source.effective_radius,
            })).collect::<Vec<_>>(),
        })
    }

    pub(super) fn render_corner_radius(
        &self,
        ctx: &mut DrawingContext,
        editor: &Editor,
        id: NodeId,
    ) {
        let Some((target, transform)) = self.radius_target(editor) else {
            return;
        };
        if target != id {
            return;
        }
        let handles = self.prepared_radius_handles(editor, id, transform);
        let mut state = self.corner_radius.borrow_mut();
        let active = state
            .gesture
            .as_ref()
            .map(|gesture| (gesture.corner, gesture.individual, gesture.radius));
        let corner = active.map(|(corner, _, _)| corner).or_else(|| {
            state
                .hover
                .filter(|hover| hover.node == id)
                .map(|hover| hover.corner)
        });
        let Some(handle) = handles
            .iter()
            .find(|handle| Some(handle.source.corner) == corner)
        else {
            return;
        };
        let blue = pen(0xFF0D99FF, 1.0);
        let white = brush("#FFFFFF");
        let fill = brush("#0D99FF");
        ctx.draw_line(&blue, ui(handle.vertex_screen), ui(handle.screen));
        ctx.draw_ellipse_at(
            Some(if active.is_some() { &fill } else { &white }),
            Some(&blue),
            ui(handle.screen),
            4.5,
            4.5,
        );
        if let Some((corner, individual, radius)) = active {
            ctx.draw_ellipse_at(Some(&white), None, ui(handle.screen), 1.5, 1.5);
            let value = if individual {
                format!("R{} {:.2}", corner + 1, radius)
            } else {
                format!("R {:.2}", radius)
            };
            if !state
                .label
                .as_ref()
                .is_some_and(|(previous, _)| previous == &value)
            {
                let layout = TextLayout::new(
                    &value,
                    Typeface::from_name("fonts:Inter#Inter"),
                    TextLayoutOptions {
                        font_size: 11.0,
                        foreground: Some(fill),
                        ..TextLayoutOptions::default()
                    },
                );
                state.label = Some((value, layout));
            }
            let layout = &state.label.as_ref().unwrap().1;
            let viewport = self.viewport.get();
            let point = Point::new(
                (handle.screen.x + 10.0)
                    .min(viewport.width - layout.width())
                    .max(0.0),
                (handle.screen.y - 20.0)
                    .min(viewport.height - layout.height())
                    .max(0.0),
            );
            layout.draw(ctx, ui(point));
        }
    }
}

#[cfg(test)]
#[path = "corner_radius_tests.rs"]
mod tests;
