//! Native linear, radial and angular gradient handles. Paint coordinates stay relative to their
//! exact native brush box; markers and hit tolerances use screen pixels.
use super::*;
#[cfg(test)]
use ferrodesign_core::LinearGradientEdit;
use ferrodesign_core::{CanvasGradientEdit, CanvasGradientGeometry};
use ferroui_base::media::{immutable::ImmutableSolidColorBrush, IBrush};

const HIT_RADIUS: f64 = 7.0;
const STOP_OFFSET: f64 = 16.0;
const MIN_ENDPOINT_SEPARATION: f64 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Start,
    End,
    Stop(usize),
    Axis,
}

impl Part {
    fn json(self) -> serde_json::Value {
        let (kind, stop) = match self {
            Self::Start => ("start", None),
            Self::End => ("end", None),
            Self::Stop(index) => ("stop", Some(index)),
            Self::Axis => ("axis", None),
        };
        serde_json::json!({ "kind": kind, "stop": stop })
    }
}

struct Handle {
    part: Part,
    local: Point,
    world: Point,
    screen: Point,
    marker: Point,
}

struct PreparedGradient {
    node: NodeId,
    fill: usize,
    token: core::EditorRenderToken,
    transform: core::Transform,
    viewport: Viewport,
    bounds: Rect,
    value: CanvasGradientEdit,
    axis: [Point; 2],
    screen_axis: [Point; 2],
    handles: Vec<Handle>,
    stop_brushes: Vec<Rc<dyn IBrush>>,
    ring: Vec<Point>,
}

struct GradientGesture {
    prepared: Rc<PreparedGradient>,
    page: NodeId,
    part: Part,
    previous_selection: Option<Part>,
    inverse: core::Transform,
    start: Point,
    original: CanvasGradientEdit,
    value: CanvasGradientEdit,
    inserted: bool,
    inserted_offset: Option<f64>,
    angular_base: Option<Point>,
    transaction: core::EditorTransactionToken,
    token: core::EditorRenderToken,
    changed: bool,
    angular_previous: Option<Point>,
    angular_change: f64,
}

impl GradientGesture {
    fn owns_transaction(&self, editor: &Editor) -> bool {
        editor.active_transaction_token().as_ref() == Some(&self.transaction)
            && editor.render_change_token() == self.token
    }

    fn is_current(&self, canvas: &DesignCanvas, editor: &Editor) -> bool {
        self.owns_transaction(editor)
            && canvas.linear_gradient.borrow().target
                == Some((self.prepared.node, self.prepared.fill))
            && canvas.tool.get() == Tool::Select
            && canvas.presentation.get().is_none()
            && canvas.viewport.get() == self.prepared.viewport
            && editor.active_page == self.page
            && validate_target(editor, self.prepared.node, self.prepared.fill).is_ok_and(
                |(transform, bounds)| {
                    transform == self.prepared.transform
                        && bounds == self.prepared.bounds
                        && matches!(
                            (
                                &self.original.geometry,
                                &editor.document.nodes[&self.prepared.node].fills
                                    [self.prepared.fill]
                                    .paint
                            ),
                            (
                                CanvasGradientGeometry::Linear { .. },
                                core::Paint::LinearGradient { .. }
                            ) | (
                                CanvasGradientGeometry::Radial { .. },
                                core::Paint::RadialGradient { .. }
                            ) | (
                                CanvasGradientGeometry::Angular { .. },
                                core::Paint::AngularGradient { .. }
                            )
                        )
                },
            )
    }
}

#[derive(Default)]
pub(super) struct GradientState {
    target: Option<(NodeId, usize)>,
    selected: Option<Part>,
    hover: Option<Part>,
    cache: Option<Rc<PreparedGradient>>,
    gesture: Option<GradientGesture>,
    #[cfg(test)]
    preparations: usize,
}

fn finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn validate_target(
    editor: &Editor,
    id: NodeId,
    fill: usize,
) -> Result<(core::Transform, Rect), String> {
    if editor.selection.as_slice() != [id]
        || editor.document.effective_locked(id)
        || !editor.document.effective_visible(id)
        || editor.document.page_of(id) != Some(editor.active_page)
    {
        return Err("Select one visible, unlocked layer to edit its gradient.".into());
    }
    let node = editor
        .document
        .nodes
        .get(&id)
        .ok_or("The gradient layer no longer exists.")?;
    let bounds = core::linear_gradient_canvas_bounds(node).ok_or(
        "Canvas gradients support rectangles, ellipses and box containers; use the paint fields for other geometry.")?;
    let fill = node
        .fills
        .get(fill)
        .ok_or("The selected fill no longer exists.")?;
    if !fill.visible
        || fill.opacity <= 0.0
        || !matches!(
            fill.paint,
            core::Paint::LinearGradient { .. }
                | core::Paint::RadialGradient { .. }
                | core::Paint::AngularGradient { .. }
        )
    {
        return Err(
            "Choose an enabled linear, radial or angular gradient fill to edit on canvas.".into(),
        );
    }
    let transform = editor
        .document
        .world_transform(id)
        .ok_or("The gradient layer has no usable transform.")?;
    let inverse = transform
        .inverse()
        .ok_or("Canvas gradient editing needs an invertible layer transform.")?;
    if [
        transform.a,
        transform.b,
        transform.c,
        transform.d,
        transform.e,
        transform.f,
        inverse.a,
        inverse.b,
        inverse.c,
        inverse.d,
        inverse.e,
        inverse.f,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err("Canvas gradient editing needs a finite layer transform and inverse.".into());
    }
    Ok((transform, bounds))
}

fn prepare_handles(
    value: &CanvasGradientEdit,
    bounds: Rect,
    transform: core::Transform,
    viewport: Viewport,
) -> Result<([Point; 2], [Point; 2], Vec<Handle>), String> {
    let local = value.local_endpoints(bounds);
    let axis = local.map(|point| transform.apply(point));
    let screen = axis.map(|point| viewport.world_to_screen(point));
    let delta = screen[1] - screen[0];
    let length = delta.x.hypot(delta.y);
    if !viewport.zoom.is_finite()
        || viewport.zoom <= 0.0
        || !length.is_finite()
        || !axis.iter().copied().all(finite)
        || !screen.iter().copied().all(finite)
    {
        return Err("The gradient is outside representable canvas coordinates.".into());
    }
    let direction = if length > 0.0 {
        Point::new(delta.x / length, delta.y / length)
    } else {
        Point::new(1.0, 0.0)
    };
    let normal = Point::new(-direction.y, direction.x);
    let midpoint = screen[0] + delta * 0.5;
    let marker = if length < MIN_ENDPOINT_SEPARATION {
        [
            midpoint - direction * (MIN_ENDPOINT_SEPARATION * 0.5),
            midpoint + direction * (MIN_ENDPOINT_SEPARATION * 0.5),
        ]
    } else {
        screen
    };
    let mut handles: Vec<Handle> = (0..2)
        .map(|index| Handle {
            part: if index == 0 { Part::Start } else { Part::End },
            local: local[index],
            world: axis[index],
            screen: screen[index],
            marker: marker[index],
        })
        .collect();
    // Stops keep their authored indices. A screen-space interval coloring gives
    // close/hard-transition stops distinct alternating lanes in O(n log n),
    // bounded by 256 stops. Nothing is sorted back into the document.
    let mut ordered: Vec<_> = value.stops.iter().enumerate().collect();
    ordered.sort_by(|a, b| a.1.offset.total_cmp(&b.1.offset).then(a.0.cmp(&b.0)));
    let mut active = std::collections::VecDeque::<(f64, usize)>::new();
    let mut available = std::collections::BinaryHeap::<std::cmp::Reverse<usize>>::new();
    let mut lane_count = 0;
    let mut offsets = vec![STOP_OFFSET; value.stops.len()];
    for (index, stop) in ordered {
        let along = stop.offset * length;
        while active.front().is_some_and(|(end, _)| *end <= along) {
            let (_, lane) = active.pop_front().unwrap();
            available.push(std::cmp::Reverse(lane));
        }
        let lane = available.pop().map(|lane| lane.0).unwrap_or_else(|| {
            let lane = lane_count;
            lane_count += 1;
            lane
        });
        active.push_back((along + HIT_RADIUS * 2.0 + 2.0, lane));
        let magnitude = STOP_OFFSET + (lane / 2) as f64 * (HIT_RADIUS * 2.0 + 2.0);
        offsets[index] = magnitude * if lane % 2 == 0 { 1.0 } else { -1.0 };
    }
    let mut angular_markers = MarkerBuckets::default();
    for handle in &handles {
        angular_markers.insert(handle.marker);
    }
    for (index, stop) in value.stops.iter().enumerate() {
        let point = value.local_stop(bounds, stop.offset);
        let world = transform.apply(point);
        let screen = viewport.world_to_screen(world);
        handles.push(Handle {
            part: Part::Stop(index),
            local: point,
            world,
            screen,
            marker: if value.is_angular() {
                // Stop 0 and stop 1 coincide on a sweep seam. Keep every source
                // entry independently selectable without modifying its offset.
                // Bounded spatial buckets avoid all-pairs scans on tied stops.
                angular_marker(
                    screen,
                    viewport.world_to_screen(axis[0]),
                    &mut angular_markers,
                )
            } else {
                screen + normal * offsets[index]
            },
        });
    }
    if handles
        .iter()
        .any(|handle| !finite(handle.local) || !finite(handle.world) || !finite(handle.marker))
    {
        return Err("The gradient handles are outside representable canvas coordinates.".into());
    }
    Ok((axis, screen, handles))
}

// This grid is only a bounded authoring aid. It stores at most 258 markers and
// never changes authored stops, their colors, or their stable source indices.
#[derive(Default)]
struct MarkerBuckets(std::collections::BTreeMap<(i64, i64), Vec<Point>>);
impl MarkerBuckets {
    fn cell(point: Point) -> (i64, i64) {
        let size = HIT_RADIUS * 2.0 + 2.0;
        (
            (point.x / size).floor() as i64,
            (point.y / size).floor() as i64,
        )
    }
    fn insert(&mut self, point: Point) {
        self.0.entry(Self::cell(point)).or_default().push(point);
    }
    fn clear_at(&self, point: Point) -> bool {
        let (x, y) = Self::cell(point);
        (-1_i64..=1).all(|dx| {
            (-1_i64..=1).all(|dy| {
                self.0
                    .get(&(x.saturating_add(dx), y.saturating_add(dy)))
                    .is_none_or(|items| {
                        items
                            .iter()
                            .all(|other| point.distance(*other) > HIT_RADIUS * 2.0 + 1.0)
                    })
            })
        })
    }
}
fn angular_marker(screen: Point, center: Point, occupied: &mut MarkerBuckets) -> Point {
    let delta = screen - center;
    let length = delta.x.hypot(delta.y);
    let direction = if length > 0.0 {
        delta * (1.0 / length)
    } else {
        Point::new(1.0, 0.0)
    };
    let mut marker = screen + direction * STOP_OFFSET;
    for lane in 0..=core::MAX_CANVAS_GRADIENT_STOPS * 2 + 4 {
        marker = screen + direction * (STOP_OFFSET + lane as f64 * (HIT_RADIUS * 2.0 + 2.0));
        if occupied.clear_at(marker) {
            break;
        }
    }
    occupied.insert(marker);
    marker
}

fn prepare_ring(
    value: &CanvasGradientEdit,
    bounds: Rect,
    transform: core::Transform,
    viewport: Viewport,
) -> Vec<Point> {
    if matches!(value.geometry, CanvasGradientGeometry::Linear { .. }) {
        return Vec::new();
    }
    let center = viewport.world_to_screen(transform.apply(value.local_endpoints(bounds)[0]));
    let extent = [0.0, 0.25]
        .into_iter()
        .filter_map(|fraction| value.local_ring(bounds, fraction))
        .map(|point| {
            viewport
                .world_to_screen(transform.apply(point))
                .distance(center)
        })
        .fold(0.0_f64, f64::max);
    // Chord error below roughly 0.25 pixels for ordinary views, with a hard
    // cap for extreme authored coordinates/zoom. Artwork stays native curves.
    let segments = ((std::f64::consts::TAU * (extent / 2.0).sqrt()).ceil() as usize).clamp(32, 512);
    (0..=segments)
        .filter_map(|index| value.local_ring(bounds, index as f64 / segments as f64))
        .map(|point| viewport.world_to_screen(transform.apply(point)))
        .collect()
}

fn angular_vector(value: &CanvasGradientEdit, bounds: Rect, part: Part) -> Option<Point> {
    if !value.is_angular() {
        return None;
    }
    let axis = value.local_endpoints(bounds);
    match part {
        Part::End => Some(axis[1] - axis[0]),
        Part::Stop(index) => {
            Some(value.local_stop(bounds, value.stops.get(index)?.offset) - axis[0])
        }
        _ => None,
    }
}

impl DesignCanvas {
    /// Use the identical bounded preparation path for the paint-card affordance
    /// and canvas entry. This may retain a read-only handle cache; it cannot
    /// change selection, focus, document values, transactions or history.
    pub fn can_edit_linear_gradient(&self, fill: usize) -> bool {
        let editor = self.editor.borrow();
        editor
            .selection
            .first()
            .copied()
            .is_some_and(|id| self.prepared_gradient(&editor, id, fill).is_ok())
    }

    fn prepared_gradient(
        &self,
        editor: &Editor,
        id: NodeId,
        fill: usize,
    ) -> Result<Rc<PreparedGradient>, String> {
        let (transform, bounds) = validate_target(editor, id, fill)?;
        let token = editor.render_change_token();
        let viewport = self.viewport.get();
        let mut state = self.linear_gradient.borrow_mut();
        if let Some(cache) = &state.cache {
            if cache.node == id
                && cache.fill == fill
                && cache.token == token
                && cache.transform == transform
                && cache.viewport == viewport
            {
                return Ok(Rc::clone(cache));
            }
        }
        let value = CanvasGradientEdit::from_node(&editor.document.nodes[&id], fill)
            .map_err(|error| error.to_string())?;
        let (axis, screen_axis, handles) = prepare_handles(&value, bounds, transform, viewport)?;
        // Core colors serialize translucent values as RGBA, whereas FerroUI's
        // eight-digit hex parser expects ARGB. Keep typed, cached brushes so
        // marker rendering preserves every channel without per-frame parsing.
        let stop_brushes: Vec<Rc<dyn IBrush>> = value
            .stops
            .iter()
            .map(|stop| -> Rc<dyn IBrush> {
                let color = stop.color;
                Rc::new(ImmutableSolidColorBrush::new(
                    ferroui_base::media::Color::from_argb(color.a, color.r, color.g, color.b),
                ))
            })
            .collect();
        let ring = prepare_ring(&value, bounds, transform, viewport);
        if ring.iter().copied().any(|point| !finite(point)) {
            return Err("The gradient ring is outside representable canvas coordinates.".into());
        }
        let result = Rc::new(PreparedGradient {
            node: id,
            fill,
            token,
            transform,
            viewport,
            bounds,
            value,
            axis,
            screen_axis,
            handles,
            stop_brushes,
            ring,
        });
        state.cache = Some(Rc::clone(&result));
        #[cfg(test)]
        {
            state.preparations += 1;
        }
        Ok(result)
    }

    pub fn edit_linear_gradient(&self, fill: usize) -> bool {
        self.cancel_active_edit();
        self.finish_image_crop();
        self.leave_stroke_width_edit();
        self.finish_pen();
        self.leave_vector_edit();
        self.clear_corner_radius_hover();
        let result = {
            let editor = self.editor.borrow();
            editor
                .selection
                .first()
                .copied()
                .ok_or_else(|| "Select a layer with a gradient fill.".to_string())
                .and_then(|id| self.prepared_gradient(&editor, id, fill).map(|_| id))
        };
        let id = match result {
            Ok(id) => id,
            Err(error) => {
                self.emit(CanvasEvent::Notice(error));
                return false;
            }
        };
        let mut state = self.linear_gradient.borrow_mut();
        state.target = Some((id, fill));
        state.selected = None;
        state.hover = None;
        drop(state);
        self.tool.set(Tool::Select);
        self.hover.set(None);
        self.focus();
        self.invalidate_visual();
        self.emit(CanvasEvent::Selection);
        true
    }

    pub fn leave_linear_gradient_edit(&self) {
        self.cancel_linear_gradient_gesture();
        let mut state = self.linear_gradient.borrow_mut();
        state.target = None;
        state.selected = None;
        state.hover = None;
        state.cache = None;
        drop(state);
        self.invalidate_visual();
    }

    pub fn linear_gradient_active(&self) -> bool {
        self.linear_gradient.borrow().gesture.is_some()
    }

    pub(super) fn clear_gradient_hover(&self) {
        if self.linear_gradient.borrow_mut().hover.take().is_some() {
            self.invalidate_visual();
        }
    }

    fn gradient_hit(&self, prepared: &PreparedGradient, world: Point) -> Option<Part> {
        let pointer = prepared.viewport.world_to_screen(world);
        let selected = self.linear_gradient.borrow().selected;
        prepared
            .handles
            .iter()
            .filter_map(|handle| {
                let distance = pointer.distance(handle.marker);
                (distance <= HIT_RADIUS).then_some((distance, handle.part))
            })
            .min_by(|a, b| {
                a.0.total_cmp(&b.0)
                    .then_with(|| (selected != Some(a.1)).cmp(&(selected != Some(b.1))))
            })
            .map(|(_, part)| part)
            .or_else(|| {
                (core::distance_to_segment(
                    pointer,
                    prepared.screen_axis[0],
                    prepared.screen_axis[1],
                ) <= HIT_RADIUS
                    || (prepared.value.is_angular()
                        && prepared.ring.windows(2).any(|edge| {
                            core::distance_to_segment(pointer, edge[0], edge[1]) <= HIT_RADIUS
                        })))
                .then_some(Part::Axis)
            })
    }

    pub(super) fn linear_gradient_press(&self, world: Point, clicks: i32) -> bool {
        let Some((id, fill)) = self.linear_gradient.borrow().target else {
            return false;
        };
        if !finite(world) {
            return true;
        }
        self.cancel_linear_gradient_gesture();
        let result = {
            let editor = self.editor.borrow();
            if editor.has_active_transaction() {
                return true;
            }
            self.prepared_gradient(&editor, id, fill)
        };
        let prepared = match result {
            Ok(value) => value,
            Err(error) => {
                self.emit(CanvasEvent::Notice(error));
                return true;
            }
        };
        let Some(mut part) = self.gradient_hit(&prepared, world) else {
            let mut state = self.linear_gradient.borrow_mut();
            state.selected = None;
            state.hover = None;
            drop(state);
            self.invalidate_visual();
            return true;
        };
        let mut value = prepared.value.clone();
        let mut inserted = false;
        if part == Part::Axis && clicks >= 2 {
            if prepared.value.is_angular() {
                let pointer = prepared.viewport.world_to_screen(world);
                if !prepared
                    .ring
                    .windows(2)
                    .any(|edge| core::distance_to_segment(pointer, edge[0], edge[1]) <= HIT_RADIUS)
                {
                    return true;
                }
            }
            let offset =
                if let CanvasGradientGeometry::Angular { angle, .. } = prepared.value.geometry {
                    let local = prepared.transform.inverse().unwrap().apply(world);
                    let center = prepared.value.local_endpoints(prepared.bounds)[0];
                    let direction = local - center;
                    (direction.x.hypot(direction.y) > 1e-12).then(|| {
                        (direction.x.atan2(-direction.y).to_degrees() - angle.rem_euclid(360.0))
                            .rem_euclid(360.0)
                            / 360.0
                    })
                } else {
                    core::gradient_offset_delta(
                        world - prepared.axis[0],
                        prepared.axis[1] - prepared.axis[0],
                    )
                };
            let Some(offset) = offset else {
                self.emit(CanvasEvent::Notice(
                    "Separate the gradient endpoints before adding a stop.".into(),
                ));
                return true;
            };
            if !(0.0..=1.0).contains(&offset) {
                return true;
            }
            match value.add_stop(offset) {
                Ok(index) => {
                    part = Part::Stop(index);
                    inserted = true;
                }
                Err(error) => {
                    self.emit(CanvasEvent::Notice(error.to_string()));
                    return true;
                }
            }
        }
        let previous_selection = self.linear_gradient.borrow().selected;
        let mut editor = self.editor.borrow_mut();
        editor.begin_node_transaction(&[id], "Edit gradient on canvas");
        let angular_previous = angular_vector(&value, prepared.bounds, part);
        let gesture = GradientGesture {
            angular_previous,
            angular_base: angular_previous,
            inserted_offset: if inserted {
                if let Part::Stop(index) = part {
                    Some(value.stops[index].offset)
                } else {
                    None
                }
            } else {
                None
            },
            angular_change: 0.0,
            original: prepared.value.clone(),
            value,
            page: editor.active_page,
            inverse: prepared.transform.inverse().unwrap(),
            prepared,
            part,
            previous_selection,
            start: world,
            inserted,
            transaction: editor.active_transaction_token().unwrap(),
            token: editor.render_change_token(),
            changed: false,
        };
        drop(editor);
        {
            let mut state = self.linear_gradient.borrow_mut();
            state.selected = Some(part);
            state.hover = Some(part);
            state.gesture = Some(gesture);
        }
        self.emit(CanvasEvent::Draft);
        if inserted {
            self.write_gradient_gesture();
        }
        self.invalidate_visual();
        true
    }

    fn write_gradient_gesture(&self) {
        let Some(mut gesture) = self.linear_gradient.borrow_mut().gesture.take() else {
            return;
        };
        let mut failure = None;
        {
            let mut editor = self.editor.borrow_mut();
            if !gesture.is_current(self, &editor) {
                failure = Some(None);
            } else if gesture.value == gesture.original {
                if gesture.changed {
                    // Restore the full original metadata and preserve redo even
                    // before release, then retain a fresh owned no-op draft.
                    editor.cancel_transaction();
                    editor.begin_node_transaction(
                        &[gesture.prepared.node],
                        "Edit gradient on canvas",
                    );
                    gesture.transaction = editor.active_transaction_token().unwrap();
                    gesture.token = editor.render_change_token();
                    gesture.changed = false;
                }
            } else {
                match editor.set_canvas_gradient(
                    gesture.prepared.node,
                    gesture.prepared.fill,
                    gesture.value.clone(),
                ) {
                    Ok(changed) => {
                        gesture.changed |= changed;
                        gesture.token = editor.render_change_token();
                    }
                    Err(error) => failure = Some(Some(error.to_string())),
                }
            }
        }
        self.linear_gradient.borrow_mut().gesture = Some(gesture);
        if let Some(error) = failure {
            self.cancel_linear_gradient_gesture();
            if let Some(error) = error {
                self.emit(CanvasEvent::Notice(error));
            }
        } else {
            self.invalidate_visual();
        }
    }

    pub(super) fn linear_gradient_move(&self, world: Point) -> bool {
        if !finite(world) {
            return self.linear_gradient.borrow().target.is_some();
        }
        let gesture = self.linear_gradient.borrow_mut().gesture.take();
        if let Some(mut gesture) = gesture {
            if !gesture.is_current(self, &self.editor.borrow()) {
                self.linear_gradient.borrow_mut().gesture = Some(gesture);
                self.cancel_linear_gradient_gesture();
                return true;
            }
            let world_delta = world - gesture.start;
            let local_delta = gesture.inverse.apply_vector(world_delta);
            let bounds = gesture.prepared.bounds;
            let delta = Point::new(local_delta.x / bounds.width, local_delta.y / bounds.height);
            if !finite(world_delta) || !finite(local_delta) || !finite(delta) {
                self.linear_gradient.borrow_mut().gesture = Some(gesture);
                return true;
            }
            let near_original =
                world_delta.x.hypot(world_delta.y) * gesture.prepared.viewport.zoom <= 1e-6;
            let mut value = if gesture.inserted {
                gesture.value.clone()
            } else {
                gesture.original.clone()
            };
            if let Some(base) = gesture.angular_base {
                // The logical direction, not the displaced screen marker,
                // follows pointer displacement. Accumulating shortest deltas
                // handles the 0/360 seam without jumping to the opposite end.
                let next = base + local_delta;
                if near_original {
                    gesture.angular_change = 0.0;
                    gesture.angular_previous = Some(base);
                } else if let Some(change) = gesture
                    .angular_previous
                    .and_then(|previous| core::gradient_angle_delta(previous, next))
                {
                    gesture.angular_change += change;
                    gesture.angular_previous = Some(next);
                }
                match gesture.part {
                    Part::End => {
                        if let CanvasGradientGeometry::Angular { angle, .. } = &mut value.geometry {
                            let CanvasGradientGeometry::Angular {
                                angle: original, ..
                            } = gesture.original.geometry
                            else {
                                unreachable!()
                            };
                            *angle = if near_original {
                                original
                            } else {
                                (original + gesture.angular_change.to_degrees())
                                    .clamp(-360_000.0, 360_000.0)
                            };
                        }
                    }
                    Part::Stop(index) => {
                        let original = if gesture.inserted {
                            // Insertion's base value is retained in the gesture
                            // separately from its changing preview.
                            gesture.inserted_offset.unwrap()
                        } else {
                            gesture.original.stops[index].offset
                        };
                        value.stops[index].offset = if near_original {
                            original
                        } else {
                            (original + gesture.angular_change / std::f64::consts::TAU)
                                .clamp(0.0, 1.0)
                        };
                    }
                    _ => {}
                }
            } else {
                match gesture.part {
                    Part::Start | Part::End => {
                        if !near_original {
                            if let Some(moved) = gesture.original.moved_endpoint(
                                gesture.part == Part::End,
                                local_delta,
                                bounds,
                            ) {
                                value = moved;
                            }
                        }
                    }
                    Part::Axis => {
                        if !near_original {
                            if let Some(moved) = gesture.original.translated(delta) {
                                value = moved;
                            }
                        }
                    }
                    Part::Stop(index) => {
                        let original = gesture
                            .inserted_offset
                            .unwrap_or_else(|| gesture.original.stops[index].offset);
                        if let Some(change) = core::gradient_offset_delta(
                            world_delta,
                            gesture.prepared.axis[1] - gesture.prepared.axis[0],
                        ) {
                            value.stops[index].offset = if near_original {
                                original
                            } else {
                                (original + change).clamp(0.0, 1.0)
                            };
                        }
                    }
                }
            }
            let changed = value != gesture.value;
            gesture.value = value;
            self.linear_gradient.borrow_mut().gesture = Some(gesture);
            if changed {
                self.write_gradient_gesture();
            }
            return true;
        }
        if !matches!(*self.gesture.borrow(), Gesture::None) || self.space.get() {
            self.clear_gradient_hover();
            return false;
        }
        let Some((id, fill)) = self.linear_gradient.borrow().target else {
            return false;
        };
        let hover = self
            .prepared_gradient(&self.editor.borrow(), id, fill)
            .ok()
            .and_then(|prepared| self.gradient_hit(&prepared, world));
        let mut state = self.linear_gradient.borrow_mut();
        let changed = state.hover != hover;
        state.hover = hover;
        drop(state);
        if changed {
            self.invalidate_visual();
        }
        true
    }

    pub(super) fn cancel_linear_gradient_gesture(&self) -> bool {
        self.finish_gradient_gesture(false)
    }
    pub fn finish_linear_gradient_edit(&self) -> bool {
        self.finish_gradient_gesture(true)
    }

    fn finish_gradient_gesture(&self, commit: bool) -> bool {
        let Some(gesture) = self.linear_gradient.borrow_mut().gesture.take() else {
            return false;
        };
        let changed = {
            let mut editor = self.editor.borrow_mut();
            if !gesture.owns_transaction(&editor) {
                false
            } else if commit
                && gesture.is_current(self, &editor)
                && gesture.changed
                && gesture.value != gesture.original
            {
                editor.commit_transaction()
            } else {
                let selection = editor.selection.clone();
                let page = editor.active_page;
                editor.cancel_transaction();
                editor.selection = selection;
                editor.active_page = page;
                if !commit {
                    self.linear_gradient.borrow_mut().selected = gesture.previous_selection;
                }
                gesture.changed
            }
        };
        self.clear_gradient_hover();
        self.invalidate_visual();
        self.emit(CanvasEvent::Draft);
        self.emit(if changed {
            CanvasEvent::Document
        } else {
            CanvasEvent::Selection
        });
        true
    }

    pub(super) fn reconcile_linear_gradient(&self) {
        let stale = self
            .linear_gradient
            .borrow()
            .gesture
            .as_ref()
            .is_some_and(|gesture| !gesture.is_current(self, &self.editor.borrow()));
        if stale {
            self.cancel_linear_gradient_gesture();
        }
        let target = self.linear_gradient.borrow().target;
        if let Some((id, fill)) = target {
            let result = self.prepared_gradient(&self.editor.borrow(), id, fill);
            if self.tool.get() != Tool::Select
                || self.presentation.get().is_some()
                || self.image_crop_state().is_some()
                || self.stroke_width_mode_active()
                || self.vector_node.get().is_some()
                || result.is_err()
            {
                self.leave_linear_gradient_edit();
                return;
            }
            let prepared = result.unwrap();
            let mut state = self.linear_gradient.borrow_mut();
            if state.selected.is_some_and(
                |part| matches!(part, Part::Stop(index) if index >= prepared.value.stops.len()),
            ) {
                state.selected = None;
            }
        }
    }

    pub(super) fn linear_gradient_key(&self, key: Key, modifiers: KeyModifiers) -> bool {
        if self.linear_gradient.borrow().target.is_none() {
            return false;
        }
        if key == Key::Escape {
            if !self.cancel_linear_gradient_gesture() {
                self.leave_linear_gradient_edit();
                self.emit(CanvasEvent::Selection);
            }
            return true;
        }
        if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META) {
            return false;
        }
        if key == Key::Enter {
            self.finish_linear_gradient_edit();
            self.leave_linear_gradient_edit();
            self.emit(CanvasEvent::Selection);
            return true;
        }
        if !matches!(
            key,
            Key::Tab | Key::Delete | Key::Back | Key::Left | Key::Right | Key::Up | Key::Down
        ) {
            return false;
        }
        self.finish_linear_gradient_edit();
        let (target, selected) = {
            let state = self.linear_gradient.borrow();
            (state.target, state.selected)
        };
        let Some((id, fill)) = target else {
            return true;
        };
        let prepared = match self.prepared_gradient(&self.editor.borrow(), id, fill) {
            Ok(value) => value,
            Err(_) => return true,
        };
        if key == Key::Tab {
            let total = prepared.value.stops.len() + 2;
            let selected = match selected {
                Some(Part::Start) => Some(0),
                Some(Part::End) => Some(1),
                Some(Part::Stop(index)) => Some(index + 2),
                _ => None,
            };
            let index = match selected {
                Some(index) if modifiers.contains(KeyModifiers::SHIFT) => {
                    (index + total - 1) % total
                }
                Some(index) => (index + 1) % total,
                None if modifiers.contains(KeyModifiers::SHIFT) => total - 1,
                None => 0,
            };
            self.linear_gradient.borrow_mut().selected = Some(match index {
                0 => Part::Start,
                1 => Part::End,
                _ => Part::Stop(index - 2),
            });
            self.invalidate_visual();
            return true;
        }
        let Some(part) = selected else {
            return true;
        };
        if self.editor.borrow().has_active_transaction() {
            return true;
        }
        let mut value = prepared.value.clone();
        let deleted = matches!(key, Key::Delete | Key::Back);
        if deleted {
            let Part::Stop(index) = part else {
                return true;
            };
            if let Err(error) = value.remove_stop(index) {
                self.emit(CanvasEvent::Notice(error.to_string()));
                return true;
            }
        } else {
            let sign = if matches!(key, Key::Left | Key::Down) {
                -1.0
            } else {
                1.0
            };
            if let Part::Stop(index) = part {
                let step = if modifiers.contains(KeyModifiers::SHIFT) {
                    0.1
                } else if modifiers.contains(KeyModifiers::ALT) {
                    0.001
                } else {
                    0.01
                };
                value.stops[index].offset =
                    (value.stops[index].offset + sign * step).clamp(0.0, 1.0);
            } else {
                let step = if modifiers.contains(KeyModifiers::SHIFT) {
                    10.0
                } else if modifiers.contains(KeyModifiers::ALT) {
                    0.1
                } else {
                    1.0
                };
                let world_delta = if matches!(key, Key::Left | Key::Right) {
                    Point::new(sign * step, 0.0)
                } else {
                    Point::new(0.0, if key == Key::Up { -step } else { step })
                };
                let local = prepared
                    .transform
                    .inverse()
                    .unwrap()
                    .apply_vector(world_delta);
                let delta = Point::new(
                    local.x / prepared.bounds.width,
                    local.y / prepared.bounds.height,
                );
                if part == Part::Axis {
                    if let Some(moved) = value.translated(delta) {
                        value = moved;
                    }
                } else if let Some(moved) =
                    value.moved_endpoint(part == Part::End, local, prepared.bounds)
                {
                    value = moved;
                }
            }
        }
        let result = self
            .editor
            .borrow_mut()
            .set_canvas_gradient(id, fill, value);
        match result {
            Ok(changed) => {
                if deleted {
                    self.linear_gradient.borrow_mut().selected = None;
                }
                if changed {
                    self.emit(CanvasEvent::Document);
                }
            }
            Err(error) => self.emit(CanvasEvent::Notice(error.to_string())),
        }
        self.invalidate_visual();
        true
    }

    /// Observational state only; every edit still enters through native input.
    pub fn linear_gradient_state(&self) -> serde_json::Value {
        let (target, selected, hover, active) = {
            let state = self.linear_gradient.borrow();
            (
                state.target,
                state.selected,
                state.hover,
                state.gesture.is_some(),
            )
        };
        let prepared = target
            .and_then(|(id, fill)| self.prepared_gradient(&self.editor.borrow(), id, fill).ok());
        let point = |point: Point| serde_json::json!({ "x": point.x, "y": point.y });
        let handles: Vec<_> = prepared.as_ref().map_or_else(Vec::new, |prepared| prepared.handles.iter().map(|handle| {
            let part = handle.part.json();
            let offset = if let Part::Stop(index) = handle.part { Some(prepared.value.stops[index].offset) } else { None };
            serde_json::json!({ "kind": part["kind"], "stop": part["stop"], "local": point(handle.local),
                "world": point(handle.world), "screen": point(handle.screen), "markerScreen": point(handle.marker),
                "markerWorld": point(prepared.viewport.screen_to_world(handle.marker)), "offset": offset })
        }).collect());
        serde_json::json!({ "node": target.map(|target| target.0), "fill": target.map(|target| target.1),
            "gradientKind": prepared.as_ref().map(|prepared| prepared.value.kind()),
            "ring": prepared.as_ref().map(|prepared| prepared.ring.iter().copied().map(point).collect::<Vec<_>>()),
            "active": active, "selected": selected.map(Part::json), "hover": hover.map(Part::json), "handles": handles,
            "axis": prepared.as_ref().map(|prepared| serde_json::json!({ "start": point(prepared.axis[0]), "end": point(prepared.axis[1]) })) })
    }

    pub(super) fn render_linear_gradient(
        &self,
        context: &mut DrawingContext,
        editor: &Editor,
        id: NodeId,
    ) -> bool {
        let Some((target, fill)) = self.linear_gradient.borrow().target else {
            return false;
        };
        if target != id {
            return false;
        }
        let Ok(prepared) = self.prepared_gradient(editor, id, fill) else {
            return true;
        };
        let state = self.linear_gradient.borrow();
        let ink = pen(0xFF0D99FF, 1.0);
        let outline = pen(0xFFFFFFFF, 3.0);
        let highlight = pen(0xFF0D99FF, 2.0);
        let white = brush("#FFFFFF");
        let blue = brush("#0D99FF");
        context.draw_line(
            &outline,
            ui(prepared.screen_axis[0]),
            ui(prepared.screen_axis[1]),
        );
        context.draw_line(
            &ink,
            ui(prepared.screen_axis[0]),
            ui(prepared.screen_axis[1]),
        );
        for edge in prepared.ring.windows(2) {
            context.draw_line(&outline, ui(edge[0]), ui(edge[1]));
        }
        for edge in prepared.ring.windows(2) {
            context.draw_line(&ink, ui(edge[0]), ui(edge[1]));
        }
        for handle in &prepared.handles {
            let selected = state.selected == Some(handle.part) || state.hover == Some(handle.part);
            context.draw_line(&ink, ui(handle.screen), ui(handle.marker));
            match handle.part {
                Part::Start | Part::End => context.draw_rectangle(
                    Some(if selected { &blue } else { &white }),
                    Some(&ink),
                    UiRect::new(handle.marker.x - 4.0, handle.marker.y - 4.0, 8.0, 8.0),
                    0.0,
                    0.0,
                    &BoxShadows::default(),
                ),
                Part::Stop(index) => {
                    context.draw_ellipse_at(
                        Some(&white),
                        Some(if selected { &highlight } else { &ink }),
                        ui(handle.marker),
                        5.0,
                        5.0,
                    );
                    context.draw_ellipse_at(
                        Some(&prepared.stop_brushes[index]),
                        None,
                        ui(handle.marker),
                        3.0,
                        3.0,
                    );
                }
                Part::Axis => {}
            }
        }
        true
    }
}

#[cfg(test)]
#[path = "gradient_edit_tests.rs"]
mod tests;
