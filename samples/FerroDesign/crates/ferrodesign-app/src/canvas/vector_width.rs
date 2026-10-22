//! Native contour width-point handles. Hover is immutable; pointer edits own
//! an explicit scoped transaction and cannot cancel another editor's draft.
use super::*;
use ferrodesign_core::{StrokeWidthContour, StrokeWidthPoint, StrokeWidthProfile};

const STROKE_INDEX: usize = 0;
const MAX_EDIT_CONTOURS: usize = 64;
const MAX_EDIT_POINTS: usize = 2_048;
const POSITION_GAP: f64 = 1e-6;
const HIT_RADIUS: f64 = 7.0;
const MIN_HANDLE_OFFSET: f64 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Selection {
    contour: usize,
    point: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Position,
    Width(i8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Hover {
    Handle(Selection, Part),
    Insert {
        contour: usize,
        position: f64,
        point: Point,
    },
}

struct CachedContours {
    node: NodeId,
    token: core::EditorRenderToken,
    contours: Result<Rc<Vec<StrokeWidthContour>>, String>,
}

struct WidthGesture {
    node: NodeId,
    page: NodeId,
    transform: core::Transform,
    inverse: core::Transform,
    contours: Rc<Vec<StrokeWidthContour>>,
    selection: Selection,
    previous_selection: Option<Selection>,
    original_points: Vec<StrokeWidthPoint>,
    points: Vec<StrokeWidthPoint>,
    original: StrokeWidthPoint,
    normal: Point,
    start: Point,
    part: Part,
    radius_per_multiplier: f64,
    transaction: core::EditorTransactionToken,
    token: core::EditorRenderToken,
    changed: bool,
}

impl WidthGesture {
    fn owns_transaction(&self, editor: &Editor) -> bool {
        editor.active_transaction_token().as_ref() == Some(&self.transaction)
            && editor.render_change_token() == self.token
    }

    fn is_current(&self, canvas: &DesignCanvas, editor: &Editor) -> bool {
        self.owns_transaction(editor)
            && canvas.stroke_width.borrow().node == Some(self.node)
            && canvas.tool.get() == Tool::Select
            && canvas.presentation.get().is_none()
            && editor.selection.as_slice() == [self.node]
            && editor.active_page == self.page
            && editor.document.world_transform(self.node) == Some(self.transform)
            && editor.document.effective_visible(self.node)
            && !editor.document.effective_locked(self.node)
    }
}

#[derive(Default)]
pub(super) struct WidthState {
    node: Option<NodeId>,
    selected: Option<Selection>,
    hover: Option<Hover>,
    cache: Option<CachedContours>,
    gesture: Option<WidthGesture>,
    #[cfg(test)]
    preparations: usize,
}

fn radius_per_multiplier(node: &Node) -> f64 {
    let stroke = &node.strokes[STROKE_INDEX];
    stroke.width
        * if stroke.align == core::StrokeAlign::Center || !node.has_closed_stroke_contours() {
            0.5
        } else {
            1.0
        }
}

fn validate_editor_target(editor: &Editor, id: NodeId) -> Result<core::Transform, String> {
    if editor.selection.as_slice() != [id]
        || editor.document.effective_locked(id)
        || !editor.document.effective_visible(id)
        || editor.document.page_of(id) != Some(editor.active_page)
    {
        return Err("Select one visible, unlocked layer to edit its stroke width points.".into());
    }
    let node = editor
        .document
        .nodes
        .get(&id)
        .ok_or("The selected layer is no longer available.")?;
    let stroke = node
        .strokes
        .get(STROKE_INDEX)
        .ok_or("Add a first stroke before editing width points.")?;
    if !stroke.visible || !stroke.width.is_finite() || stroke.width <= 0.0 {
        return Err("Enable the first stroke and give it a positive base width before editing width points.".into());
    }
    if radius_per_multiplier(node) <= 0.0 {
        return Err(
            "The base stroke width is too small for representable canvas width handles.".into(),
        );
    }
    let transform = editor
        .document
        .world_transform(id)
        .ok_or("The selected layer has no usable transform.")?;
    transform
        .inverse()
        .ok_or("Stroke width handles need an invertible layer transform.")?;
    Ok(transform)
}

fn prepare(node: &Node) -> Result<Rc<Vec<StrokeWidthContour>>, String> {
    let mut stroke = node.strokes[STROKE_INDEX].clone();
    // Uniform's preflight intentionally permits dashed/branched paths. A handle
    // edit will become Custom, so validate that real destination before entry.
    stroke.width_profile = StrokeWidthProfile::Custom {
        points: stroke.width_profile.control_points().to_vec(),
    };
    if stroke.cap == core::StrokeCap::Arrow {
        stroke.cap = core::StrokeCap::Butt;
    }
    let contours = core::stroke_width_contours(node, &stroke).map_err(|error| error.to_string())?;
    if contours.is_empty() {
        return Err("The selected stroke has no drawable contour.".into());
    }
    if contours.len() > MAX_EDIT_CONTOURS
        || contours.len() * stroke.width_profile.control_points().len() > MAX_EDIT_POINTS
    {
        return Err("Canvas width editing supports at most 64 contours and 2,048 repeated width points. Simplify this path or use the numeric profile editor.".into());
    }
    Ok(Rc::new(contours))
}

fn handle_points(
    location: core::StrokeWidthLocation,
    width: f64,
    radius: f64,
    transform: core::Transform,
    viewport: Viewport,
) -> Option<[Point; 3]> {
    let center = viewport.world_to_screen(transform.apply(location.point));
    let direction = transform.apply_vector(location.normal) * viewport.zoom;
    let scale = direction.x.hypot(direction.y);
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    // Keep zero/tiny widths accessible without changing the authored width.
    // Dragging uses pointer displacement, so this visual minimum causes no jump.
    let offset = direction * ((radius * width).max(MIN_HANDLE_OFFSET / scale));
    let result = [center, center + offset, center - offset];
    result
        .iter()
        .all(|point| point.x.is_finite() && point.y.is_finite())
        .then_some(result)
}

fn nearest(
    contours: &[StrokeWidthContour],
    world: Point,
    transform: core::Transform,
) -> Option<(usize, core::StrokeWidthLocation, f64)> {
    contours
        .iter()
        .enumerate()
        .filter_map(|(index, contour)| {
            contour
                .nearest_transformed(world, transform)
                .map(|(location, distance)| (index, location, distance))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
}

fn point_position(points: &[StrokeWidthPoint], index: usize, desired: f64) -> f64 {
    let min = if index == 0 {
        0.0
    } else {
        points[index - 1].position + POSITION_GAP
    };
    let max = if index + 1 == points.len() {
        1.0
    } else {
        points[index + 1].position - POSITION_GAP
    };
    // Imported custom points can be closer than the canvas spacing affordance.
    // Preserve their position instead of reordering or silently merging them.
    if min > max {
        points[index].position
    } else {
        desired.clamp(min, max)
    }
}

impl DesignCanvas {
    fn prepared_stroke_width(
        &self,
        editor: &Editor,
        id: NodeId,
    ) -> Result<Rc<Vec<StrokeWidthContour>>, String> {
        let token = editor.render_change_token();
        let mut state = self.stroke_width.borrow_mut();
        if !state
            .cache
            .as_ref()
            .is_some_and(|cache| cache.node == id && cache.token == token)
        {
            let contours = prepare(
                editor
                    .document
                    .nodes
                    .get(&id)
                    .ok_or("The selected layer is no longer available.")?,
            );
            state.cache = Some(CachedContours {
                node: id,
                token,
                contours,
            });
            #[cfg(test)]
            {
                state.preparations += 1;
            }
        }
        state.cache.as_ref().unwrap().contours.clone()
    }

    /// Enters an editor-only mode without converting the layer or its path.
    /// The first stroke is consistent with the existing cap/join controls.
    pub fn edit_stroke_width(&self) -> bool {
        self.cancel_active_edit();
        self.finish_image_crop();
        self.finish_pen();
        let prepared = {
            let editor = self.editor.borrow();
            let id = editor.selection.first().copied();
            id.ok_or_else(|| {
                "Select a layer with a stroke before editing width points.".to_string()
            })
            .and_then(|id| {
                validate_editor_target(&editor, id)?;
                self.prepared_stroke_width(&editor, id)?;
                Ok(id)
            })
        };
        let id = match prepared {
            Ok(id) => id,
            Err(error) => {
                self.emit(CanvasEvent::Notice(error));
                return false;
            }
        };
        self.leave_vector_edit();
        self.tool.set(Tool::Select);
        let mut state = self.stroke_width.borrow_mut();
        state.node = Some(id);
        state.selected = None;
        state.hover = None;
        drop(state);
        self.focus();
        self.invalidate_visual();
        self.emit(CanvasEvent::Selection);
        true
    }

    pub fn leave_stroke_width_edit(&self) {
        self.cancel_stroke_width_gesture();
        let mut state = self.stroke_width.borrow_mut();
        state.node = None;
        state.selected = None;
        state.hover = None;
        state.cache = None;
        drop(state);
        self.invalidate_visual();
    }

    pub fn stroke_width_state(&self) -> serde_json::Value {
        let state = self.stroke_width.borrow();
        serde_json::json!({
            "node": state.node,
            "point": state.selected.map(|selection| selection.point),
            "contour": state.selected.map(|selection| selection.contour),
            "active": state.gesture.is_some(),
            "hover": state.hover.is_some(),
            "contours": state.cache.as_ref().and_then(|cache| cache.contours.as_ref().ok()).map_or(0, |contours| contours.len()),
        })
    }

    pub fn stroke_width_active(&self) -> bool {
        self.stroke_width.borrow().gesture.is_some()
    }

    pub(super) fn clear_stroke_width_hover(&self) {
        if self.stroke_width.borrow_mut().hover.take().is_some() {
            self.invalidate_visual();
        }
    }

    fn width_hit(
        &self,
        editor: &Editor,
        id: NodeId,
        contours: &[StrokeWidthContour],
        world: Point,
        transform: core::Transform,
    ) -> Option<Hover> {
        let node = &editor.document.nodes[&id];
        let points = node.strokes[STROKE_INDEX].width_profile.control_points();
        let radius = radius_per_multiplier(node);
        let viewport = self.viewport.get();
        let pointer = viewport.world_to_screen(world);
        let selected = self.stroke_width.borrow().selected;
        let mut hit: Option<(f64, Hover)> = None;
        for (contour_index, contour) in contours.iter().enumerate() {
            for (index, point) in points.iter().enumerate() {
                let Some(location) = contour.sample(point.position) else {
                    continue;
                };
                let Some(positions) =
                    handle_points(location, point.width, radius, transform, viewport)
                else {
                    continue;
                };
                for (position, part) in
                    positions
                        .into_iter()
                        .zip([Part::Position, Part::Width(1), Part::Width(-1)])
                {
                    let distance = pointer.distance(position);
                    let selection = Selection {
                        contour: contour_index,
                        point: index,
                    };
                    if distance <= HIT_RADIUS && hit.as_ref().is_none_or(|(previous, previous_hit)| distance < *previous
                        || ((distance - *previous).abs() < 1e-6 && selected == Some(selection)
                            && !matches!(previous_hit, Hover::Handle(previous, _) if Some(*previous) == selected))) {
                        hit = Some((distance, Hover::Handle(selection, part)));
                    }
                }
            }
        }
        hit.map(|(_, hit)| hit).or_else(|| {
            nearest(contours, world, transform)
                .filter(|(_, _, distance)| distance * viewport.zoom <= HIT_RADIUS)
                .map(|(contour, location, _)| Hover::Insert {
                    contour,
                    position: location.position,
                    point: location.point,
                })
        })
    }

    pub(super) fn stroke_width_press(&self, world: Point) -> bool {
        let Some(id) = self.stroke_width.borrow().node else {
            return false;
        };
        let result = {
            let editor = self.editor.borrow();
            validate_editor_target(&editor, id).and_then(|transform| {
                let contours = self.prepared_stroke_width(&editor, id)?;
                let hit = self.width_hit(&editor, id, &contours, world, transform);
                Ok((transform, contours, hit))
            })
        };
        let (transform, contours, hit) = match result {
            Ok(result) => result,
            Err(error) => {
                self.emit(CanvasEvent::Notice(error));
                return true;
            }
        };
        let Some(hit) = hit else {
            self.stroke_width.borrow_mut().selected = None;
            self.clear_stroke_width_hover();
            self.invalidate_visual();
            return true;
        };
        let previous_selection = self.stroke_width.borrow().selected;
        let mut editor = self.editor.borrow_mut();
        if editor.has_active_transaction() {
            return true;
        }
        let node = &editor.document.nodes[&id];
        let profile = &node.strokes[STROKE_INDEX].width_profile;
        let mut points = profile.control_points().to_vec();
        let original_points = points.clone();
        let mut insert = false;
        let (selection, part) = match hit {
            Hover::Handle(selection, part) => (selection, part),
            Hover::Insert {
                contour, position, ..
            } => {
                if let Some(index) = points
                    .iter()
                    .position(|point| (point.position - position).abs() < POSITION_GAP)
                {
                    (
                        Selection {
                            contour,
                            point: index,
                        },
                        Part::Position,
                    )
                } else {
                    if points.len() >= core::MAX_STROKE_WIDTH_POINTS
                        || (points.len() + 1) * contours.len() > MAX_EDIT_POINTS
                    {
                        drop(editor);
                        self.emit(CanvasEvent::Notice(
                            "This profile has reached the canvas width-point limit.".into(),
                        ));
                        return true;
                    }
                    let index = points.partition_point(|point| point.position < position);
                    points.insert(
                        index,
                        StrokeWidthPoint::new(position, profile.sample(position)),
                    );
                    insert = true;
                    (
                        Selection {
                            contour,
                            point: index,
                        },
                        Part::Width(1),
                    )
                }
            }
        };
        let original = points[selection.point];
        let Some(location) = contours[selection.contour].sample(original.position) else {
            return true;
        };
        let radius_per_multiplier = radius_per_multiplier(node);
        editor.begin_node_transaction(&[id], "Edit stroke width points");
        let gesture = WidthGesture {
            node: id,
            page: editor.active_page,
            transform,
            inverse: transform.inverse().unwrap(),
            contours,
            selection,
            previous_selection,
            original_points,
            points,
            original,
            normal: location.normal,
            start: world,
            part,
            radius_per_multiplier,
            transaction: editor.active_transaction_token().unwrap(),
            token: editor.render_change_token(),
            changed: false,
        };
        drop(editor);
        {
            let mut state = self.stroke_width.borrow_mut();
            state.selected = Some(selection);
            state.gesture = Some(gesture);
        }
        self.emit(CanvasEvent::Draft);
        if insert {
            self.write_width_gesture(true);
        }
        self.invalidate_visual();
        true
    }

    fn write_width_gesture(&self, force: bool) {
        let gesture = self.stroke_width.borrow_mut().gesture.take();
        let Some(mut gesture) = gesture else {
            return;
        };
        let result = {
            let mut editor = self.editor.borrow_mut();
            if !gesture.is_current(self, &editor) {
                None
            } else {
                let profile = StrokeWidthProfile::Custom {
                    points: gesture.points.clone(),
                };
                if force
                    || editor.document.nodes[&gesture.node].strokes[STROKE_INDEX].width_profile
                        != profile
                {
                    let result =
                        editor.set_stroke_width_profile(&[gesture.node], STROKE_INDEX, profile);
                    if let Ok(changed) = result {
                        gesture.changed |= changed > 0;
                        gesture.token = editor.render_change_token();
                    }
                    Some(result)
                } else {
                    Some(Ok(0))
                }
            }
        };
        self.stroke_width.borrow_mut().gesture = Some(gesture);
        match result {
            None => {
                self.cancel_stroke_width_gesture();
            }
            Some(Err(error)) => {
                self.cancel_stroke_width_gesture();
                self.emit(CanvasEvent::Notice(error.to_string()));
            }
            Some(Ok(_)) => {
                self.invalidate_visual();
            }
        }
    }

    pub(super) fn stroke_width_move(&self, world: Point) -> bool {
        if !world.x.is_finite() || !world.y.is_finite() {
            return self.stroke_width.borrow().node.is_some();
        }
        let gesture = self.stroke_width.borrow_mut().gesture.take();
        if let Some(mut gesture) = gesture {
            if !gesture.is_current(self, &self.editor.borrow()) {
                self.stroke_width.borrow_mut().gesture = Some(gesture);
                self.cancel_stroke_width_gesture();
                return true;
            }
            let previous = gesture.points[gesture.selection.point];
            match gesture.part {
                Part::Position => {
                    if let Some((location, _)) = gesture.contours[gesture.selection.contour]
                        .nearest_transformed(world, gesture.transform)
                    {
                        gesture.points[gesture.selection.point].position = point_position(
                            &gesture.points,
                            gesture.selection.point,
                            location.position,
                        );
                    }
                }
                Part::Width(side) => {
                    let delta = gesture.inverse.apply(world) - gesture.inverse.apply(gesture.start);
                    let distance =
                        (delta.x * gesture.normal.x + delta.y * gesture.normal.y) * f64::from(side);
                    let width = (gesture.original.width + distance / gesture.radius_per_multiplier)
                        .max(0.0);
                    // A nonfinite pointer projection must never enter document
                    // data. The core additionally checks width * base width.
                    if width.is_finite() {
                        gesture.points[gesture.selection.point].width = width;
                    }
                }
            }
            let changed = gesture.points[gesture.selection.point] != previous;
            self.stroke_width.borrow_mut().gesture = Some(gesture);
            if changed {
                self.write_width_gesture(false);
            }
            return true;
        }
        if !matches!(*self.gesture.borrow(), Gesture::None) || self.space.get() {
            self.clear_stroke_width_hover();
            return false;
        }
        let Some(id) = self.stroke_width.borrow().node else {
            return false;
        };
        let hover = {
            let editor = self.editor.borrow();
            validate_editor_target(&editor, id)
                .ok()
                .and_then(|transform| {
                    self.prepared_stroke_width(&editor, id)
                        .ok()
                        .and_then(|contours| {
                            self.width_hit(&editor, id, &contours, world, transform)
                        })
                })
        };
        let mut state = self.stroke_width.borrow_mut();
        let changed = state.hover != hover;
        state.hover = hover;
        drop(state);
        if changed {
            self.invalidate_visual();
        }
        true
    }

    pub(super) fn cancel_stroke_width_gesture(&self) -> bool {
        self.finish_width_gesture(false)
    }
    /// Save/lifecycle boundary: commits only this pointer-owned transaction.
    pub fn finish_stroke_width_edit(&self) -> bool {
        self.finish_width_gesture(true)
    }

    fn finish_width_gesture(&self, commit: bool) -> bool {
        let Some(gesture) = self.stroke_width.borrow_mut().gesture.take() else {
            return false;
        };
        let changed = {
            let mut editor = self.editor.borrow_mut();
            if !gesture.owns_transaction(&editor) {
                false
            } else if commit
                && gesture.is_current(self, &editor)
                && gesture.points != gesture.original_points
            {
                editor.commit_transaction()
            } else {
                let selection = editor.selection.clone();
                let page = editor.active_page;
                editor.cancel_transaction();
                editor.selection = selection;
                editor.active_page = page;
                if !commit {
                    self.stroke_width.borrow_mut().selected = gesture.previous_selection;
                }
                gesture.changed
            }
        };
        self.clear_stroke_width_hover();
        self.invalidate_visual();
        self.emit(CanvasEvent::Draft);
        self.emit(if changed {
            CanvasEvent::Document
        } else {
            CanvasEvent::Selection
        });
        true
    }

    pub(super) fn reconcile_stroke_width(&self) {
        let invalid_gesture = self
            .stroke_width
            .borrow()
            .gesture
            .as_ref()
            .is_some_and(|gesture| !gesture.is_current(self, &self.editor.borrow()));
        if invalid_gesture {
            self.cancel_stroke_width_gesture();
        }
        let id = self.stroke_width.borrow().node;
        if let Some(id) = id {
            let editor = self.editor.borrow();
            if self.tool.get() != Tool::Select
                || self.presentation.get().is_some()
                || validate_editor_target(&editor, id).is_err()
            {
                drop(editor);
                self.leave_stroke_width_edit();
                return;
            }
            let count = editor.document.nodes[&id].strokes[STROKE_INDEX]
                .width_profile
                .control_points()
                .len();
            let mut state = self.stroke_width.borrow_mut();
            if state
                .selected
                .is_some_and(|selection| selection.point >= count)
            {
                state.selected = None;
            }
        }
    }

    pub(super) fn stroke_width_key(&self, key: Key, modifiers: KeyModifiers) -> bool {
        if self.stroke_width.borrow().node.is_none() {
            return false;
        }
        if key == Key::Escape {
            if !self.cancel_stroke_width_gesture() {
                self.leave_stroke_width_edit();
                self.emit(CanvasEvent::Selection);
            }
            return true;
        }
        if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META) {
            return false;
        }
        if !matches!(
            key,
            Key::Delete | Key::Back | Key::Left | Key::Right | Key::Up | Key::Down | Key::Tab
        ) {
            return false;
        }
        self.finish_stroke_width_edit();
        let (id, selected) = {
            let state = self.stroke_width.borrow();
            (state.node, state.selected)
        };
        let Some(id) = id else {
            return true;
        };
        if key == Key::Tab {
            let editor = self.editor.borrow();
            if validate_editor_target(&editor, id).is_err() {
                return true;
            }
            let Ok(contours) = self.prepared_stroke_width(&editor, id) else {
                return true;
            };
            let count = editor.document.nodes[&id].strokes[STROKE_INDEX]
                .width_profile
                .control_points()
                .len();
            let total = count * contours.len();
            let reverse = modifiers.contains(KeyModifiers::SHIFT);
            let index = selected.map(|selected| selected.contour * count + selected.point);
            let index = match index {
                Some(index) if reverse => (index + total - 1) % total,
                Some(index) => (index + 1) % total,
                None if reverse => total - 1,
                None => 0,
            };
            self.stroke_width.borrow_mut().selected = Some(Selection {
                contour: index / count,
                point: index % count,
            });
            self.invalidate_visual();
            return true;
        }
        let Some(selected) = selected else {
            return true;
        };
        let mut editor = self.editor.borrow_mut();
        if editor.has_active_transaction() || validate_editor_target(&editor, id).is_err() {
            return true;
        }
        let mut points = editor.document.nodes[&id].strokes[STROKE_INDEX]
            .width_profile
            .control_points()
            .to_vec();
        if selected.point >= points.len() {
            return true;
        }
        let delete = matches!(key, Key::Delete | Key::Back);
        if delete {
            points.remove(selected.point);
        } else {
            let step = if modifiers.contains(KeyModifiers::SHIFT) {
                0.1
            } else {
                0.01
            };
            match key {
                Key::Left | Key::Right => {
                    let direction = if key == Key::Left { -1.0 } else { 1.0 };
                    points[selected.point].position = point_position(
                        &points,
                        selected.point,
                        points[selected.point].position + direction * step,
                    );
                }
                Key::Up | Key::Down => {
                    let direction = if key == Key::Down { -1.0 } else { 1.0 };
                    points[selected.point].width =
                        (points[selected.point].width + direction * step * 10.0).max(0.0);
                }
                _ => {}
            }
        }
        let profile = if points.is_empty() {
            StrokeWidthProfile::Uniform
        } else {
            StrokeWidthProfile::Custom { points }
        };
        let result = editor.set_stroke_width_profile(&[id], STROKE_INDEX, profile);
        drop(editor);
        match result {
            Ok(changed) => {
                if delete {
                    self.stroke_width.borrow_mut().selected = None;
                }
                if changed > 0 {
                    self.emit(CanvasEvent::Document);
                }
            }
            Err(error) => self.emit(CanvasEvent::Notice(error.to_string())),
        }
        self.invalidate_visual();
        true
    }

    pub(super) fn render_stroke_width(
        &self,
        ctx: &mut DrawingContext,
        editor: &Editor,
        id: NodeId,
        transform: core::Transform,
        viewport: Viewport,
    ) -> bool {
        if self.stroke_width.borrow().node != Some(id) {
            return false;
        }
        if validate_editor_target(editor, id).is_err() {
            return true;
        }
        let Ok(contours) = self.prepared_stroke_width(editor, id) else {
            return true;
        };
        let node = &editor.document.nodes[&id];
        let profile = &node.strokes[STROKE_INDEX].width_profile;
        let radius = radius_per_multiplier(node);
        let state = self.stroke_width.borrow();
        let ink = pen(0xFF0D99FF, 1.0);
        let fill = brush("#0D99FF");
        let white = brush("#FFFFFF");
        for (contour_index, contour) in contours.iter().enumerate() {
            for (index, point) in profile.control_points().iter().enumerate() {
                let Some(location) = contour.sample(point.position) else {
                    continue;
                };
                let Some([center, plus, minus]) =
                    handle_points(location, point.width, radius, transform, viewport)
                else {
                    continue;
                };
                let selection = Selection {
                    contour: contour_index,
                    point: index,
                };
                let selected = state.selected == Some(selection);
                ctx.draw_line(&ink, ui(minus), ui(plus));
                for (position, part) in [(plus, Part::Width(1)), (minus, Part::Width(-1))] {
                    let highlighted =
                        selected || state.hover == Some(Hover::Handle(selection, part));
                    ctx.draw_ellipse_at(
                        Some(if highlighted { &fill } else { &white }),
                        Some(&ink),
                        ui(position),
                        3.5,
                        3.5,
                    );
                }
                ctx.draw_rectangle(
                    Some(if selected { &fill } else { &white }),
                    Some(&ink),
                    UiRect::new(center.x - 3.0, center.y - 3.0, 6.0, 6.0),
                    0.0,
                    0.0,
                    &BoxShadows::default(),
                );
                if selected {
                    label(
                        ctx,
                        &format!("{:.1}% · {:.2}×", point.position * 100.0, point.width),
                        Point::new(plus.x + 8.0, plus.y - 14.0),
                        10.0,
                        "#0D99FF",
                    );
                }
            }
        }
        if let Some(Hover::Insert { point, .. }) = state.hover {
            let point = viewport.world_to_screen(transform.apply(point));
            ctx.draw_ellipse_at(Some(&white), Some(&ink), ui(point), 3.5, 3.5);
            ctx.draw_line(
                &ink,
                ui(Point::new(point.x - 2.0, point.y)),
                ui(Point::new(point.x + 2.0, point.y)),
            );
            ctx.draw_line(
                &ink,
                ui(Point::new(point.x, point.y - 2.0)),
                ui(Point::new(point.x, point.y + 2.0)),
            );
        }
        true
    }
}

#[cfg(test)]
#[path = "vector_width_tests.rs"]
mod tests;
