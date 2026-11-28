//! Efficient, zero-layout-thrash FerroUI Visual & Logical Tree Physics Bindings.
//!
//! Applies rigid-body dynamics, elastic home-springs, explosions, interactive dragging,
//! magnetic hover, velocity-coupled tilt, and analytical spring micro-interactions directly to
//! **any** FerroUI [`Visual`] or [`StyledElement`] subtree by updating persistent
//! [`MatrixTransform`] render transforms (`O(active)`, zero layout invalidation, automatic
//! cleanup when visuals detach).

use crate::body::{BodyBuilder, BodyHandle, BodyType};
use crate::joints::{Joint, JointHandle};
use crate::material::PhysicsMaterial;
use crate::math::Vec2;
use crate::shapes::Shape2D;
use crate::solver::SolverConfig;
use crate::ui::{DraggableCard, GestureTracker, MagneticSnap, Spring1D, Spring2D, SpringConfig};
use crate::world::{PhysicsWorld, StepStats};
use ferroui_base::media::{ITransform, MatrixTransform};
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{Matrix, Rect, Ref, RelativePoint, StyledElement, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Configuration when binding a FerroUI [`Visual`] to a rigid body in [`VisualPhysicsHost`].
#[derive(Clone, Copy, Debug)]
pub struct VisualBodyOptions {
    pub body_type: BodyType,
    pub material: PhysicsMaterial,
    pub fixed_rotation: bool,
    /// When `true`, uses an inscribed circle collider instead of a box matching `visual.bounds()`.
    pub circular: bool,
    /// Optional spring tether pulling the visual back to its original layout center `(freq_hz, damping_ratio)`.
    pub home_spring: Option<(f32, f32)>,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub user_data: u64,
}

impl Default for VisualBodyOptions {
    fn default() -> Self {
        Self {
            body_type: BodyType::Dynamic,
            material: PhysicsMaterial::ui_card(),
            fixed_rotation: false,
            circular: false,
            home_spring: None,
            linear_damping: 0.45,
            angular_damping: 0.65,
            user_data: 0,
        }
    }
}

impl VisualBodyOptions {
    pub fn dynamic_card() -> Self {
        Self::default()
    }

    pub fn elastic_card(frequency_hz: f32, damping_ratio: f32) -> Self {
        Self {
            home_spring: Some((frequency_hz, damping_ratio)),
            ..Self::default()
        }
    }

    pub fn static_collider() -> Self {
        Self {
            body_type: BodyType::Static,
            ..Self::default()
        }
    }
}

/// Owns a visual transform override and restores the previous value on detach.
/// A later user-owned transform is never overwritten during cleanup.
struct RenderTransformBinding {
    visual: WeakRef<Visual>,
    transform: Ref<MatrixTransform>,
    installed: Rc<dyn ITransform>,
    previous: Option<Rc<dyn ITransform>>,
    previous_origin: RelativePoint,
}

impl RenderTransformBinding {
    fn new(visual: &Ref<Visual>) -> Self {
        let previous = visual.render_transform();
        let previous_origin = visual.render_transform_origin();
        let transform = MatrixTransform::with_matrix(Matrix::IDENTITY);
        let installed: Rc<dyn ITransform> = (&transform).into();
        visual.set_render_transform_origin(RelativePoint::CENTER);
        visual.set_render_transform(Some(installed.clone()));
        Self {
            visual: visual.downgrade(),
            transform,
            installed,
            previous,
            previous_origin,
        }
    }
}

impl Drop for RenderTransformBinding {
    fn drop(&mut self) {
        if let Some(visual) = self.visual.upgrade() {
            if visual
                .render_transform()
                .is_some_and(|current| Rc::ptr_eq(&current, &self.installed))
            {
                visual.set_render_transform(self.previous.clone());
                visual.set_render_transform_origin(self.previous_origin);
            }
        }
    }
}

struct BoundVisual {
    binding: RenderTransformBinding,
    body: BodyHandle,
    home_anchor: Option<BodyHandle>,
    home_joint: Option<JointHandle>,
    /// Layout center in container pixel coordinates when bound.
    rest_center_px: Vec2,
    /// Last matrix values applied (to skip redundant writes for sleeping visuals).
    last_tx: f32,
    last_ty: f32,
    last_angle: f32,
}

/// Telemetry from syncing a [`VisualPhysicsHost`] with the FerroUI visual tree.
#[derive(Clone, Copy, Debug, Default)]
pub struct VisualSyncStats {
    pub step: StepStats,
    pub bound_visuals: usize,
    pub transforms_written: usize,
    pub sync_us: u64,
}

/// Hosts a [`PhysicsWorld`] directly coupled to live FerroUI [`Visual`] and [`StyledElement`] trees.
///
/// All motion is applied through persistent [`MatrixTransform`] instances on `Visual::render_transform`,
/// avoiding expensive layout passes while keeping hit-testing, focus, and pointer events alive on
/// the real controls.
pub struct VisualPhysicsHost {
    pub world: PhysicsWorld,
    /// Conversion factor from FerroUI DIPs/pixels to physics meters (default `64.0`).
    pub pixels_per_meter: f32,
    bound: Vec<BoundVisual>,
    wall_handles: Vec<BodyHandle>,
    drag_joint: Option<JointHandle>,
    last_sync: VisualSyncStats,
}

impl Default for VisualPhysicsHost {
    fn default() -> Self {
        Self::new(64.0)
    }
}

impl VisualPhysicsHost {
    pub fn new(pixels_per_meter: f32) -> Self {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        world.config = SolverConfig::ui_realtime();
        Self {
            world,
            pixels_per_meter: pixels_per_meter.max(8.0),
            bound: Vec::new(),
            wall_handles: Vec::new(),
            drag_joint: None,
            last_sync: VisualSyncStats::default(),
        }
    }

    #[inline]
    pub fn px_to_world(&self, px: Vec2) -> Vec2 {
        px / self.pixels_per_meter
    }

    #[inline]
    pub fn world_to_px(&self, m: Vec2) -> Vec2 {
        m * self.pixels_per_meter
    }

    pub fn bound_count(&self) -> usize {
        self.bound.len()
    }

    pub fn last_stats(&self) -> VisualSyncStats {
        self.last_sync
    }

    /// Detaches one bound visual, restores its prior transform, and removes its
    /// rigid body and home tether. Other bindings keep their handles.
    pub fn detach_visual(&mut self, body: BodyHandle) -> bool {
        let Some(index) = self.bound.iter().position(|entry| entry.body == body) else {
            return false;
        };
        let removed = self.bound.swap_remove(index);
        if let Some(anchor) = removed.home_anchor {
            self.world.remove_body(anchor);
        }
        self.world.remove_body(removed.body);
        if self
            .drag_joint
            .is_some_and(|joint| self.world.joint(joint).is_none())
        {
            self.drag_joint = None;
        }
        true
    }

    /// Creates/replaces 4 invisible static boundary walls matching `container_bounds` (in pixels)
    /// so physics-driven UI controls bounce cleanly inside their panel or window.
    pub fn set_container_bounds(&mut self, bounds: Rect) {
        for h in self.wall_handles.drain(..) {
            self.world.remove_body(h);
        }
        if bounds.width <= 1.0 || bounds.height <= 1.0 {
            return;
        }
        let w = bounds.width as f32 / self.pixels_per_meter;
        let h = bounds.height as f32 / self.pixels_per_meter;
        let ox = bounds.x as f32 / self.pixels_per_meter;
        let oy = bounds.y as f32 / self.pixels_per_meter;
        let t = 0.5;
        let mat = PhysicsMaterial::new(0.0, 0.35, 0.5);

        let walls = [
            // Floor
            (
                Vec2::new(ox + w * 0.5, oy + h + t * 0.5),
                Vec2::new(w + 2.0 * t, t),
            ),
            // Ceiling
            (
                Vec2::new(ox + w * 0.5, oy - t * 0.5),
                Vec2::new(w + 2.0 * t, t),
            ),
            // Left wall
            (
                Vec2::new(ox - t * 0.5, oy + h * 0.5),
                Vec2::new(t, h + 2.0 * t),
            ),
            // Right wall
            (
                Vec2::new(ox + w + t * 0.5, oy + h * 0.5),
                Vec2::new(t, h + 2.0 * t),
            ),
        ];
        for (pos, size) in walls {
            let bh = self.world.add_body(
                BodyBuilder::static_body(Shape2D::rect(size.x, size.y))
                    .position(pos)
                    .material(mat),
            );
            self.wall_handles.push(bh);
        }
    }

    /// Binds a single FerroUI [`Visual`] to a rigid body sized automatically from `visual.bounds()`
    /// (or `fallback_bounds` if layout has not yet run).
    pub fn attach_visual(
        &mut self,
        visual: &Ref<Visual>,
        fallback_bounds: Option<Rect>,
        options: VisualBodyOptions,
    ) -> BodyHandle {
        if let Some(existing) = self.bound.iter().find(|item| {
            item.binding
                .visual
                .upgrade()
                .is_some_and(|bound| bound.ptr_eq(visual))
        }) {
            return existing.body;
        }
        let mut rect = visual.bounds();
        if rect.width <= 1.0 || rect.height <= 1.0 {
            if let Some(fb) = fallback_bounds {
                rect = fb;
            } else {
                rect = Rect::new(0.0, 0.0, 120.0, 48.0);
            }
        }
        let center_px = Vec2::new(
            (rect.x + rect.width * 0.5) as f32,
            (rect.y + rect.height * 0.5) as f32,
        );
        let size_m = Vec2::new(
            (rect.width as f32 / self.pixels_per_meter).max(0.15),
            (rect.height as f32 / self.pixels_per_meter).max(0.15),
        );
        let shape = if options.circular {
            Shape2D::circle(size_m.x.min(size_m.y) * 0.5)
        } else {
            Shape2D::rect(size_m.x, size_m.y)
        };

        let center_m = self.px_to_world(center_px);
        let body = self.world.add_body(
            BodyBuilder::new(options.body_type, shape)
                .position(center_m)
                .material(options.material)
                .fixed_rotation(options.fixed_rotation)
                .linear_damping(options.linear_damping)
                .angular_damping(options.angular_damping)
                .user_data(options.user_data),
        );

        let (home_anchor, home_joint) = if let Some((freq, damp)) = options.home_spring {
            let anchor = self.world.add_body(
                BodyBuilder::static_body(Shape2D::circle(0.05))
                    .position(center_m)
                    .sensor(true),
            );
            let joint = self.world.add_joint(Joint::spring(
                anchor,
                body,
                Vec2::ZERO,
                Vec2::ZERO,
                0.0,
                freq,
                damp,
            ));
            (Some(anchor), Some(joint))
        } else {
            (None, None)
        };

        let binding = RenderTransformBinding::new(visual);

        self.bound.push(BoundVisual {
            binding,
            body,
            home_anchor,
            home_joint,
            rest_center_px: center_px,
            last_tx: 0.0,
            last_ty: 0.0,
            last_angle: 0.0,
        });
        body
    }

    /// Binds every direct visual child of `container` into the physics simulation in one call.
    pub fn attach_visual_children(
        &mut self,
        container: &Visual,
        options: VisualBodyOptions,
    ) -> Vec<BodyHandle> {
        let Some(children) = container.visual_children_snapshot() else {
            return Vec::new();
        };
        children
            .iter()
            .map(|child| self.attach_visual(child, None, options))
            .collect()
    }

    /// Binds every direct logical child of `container` that is a [`Visual`] into the physics simulation.
    pub fn attach_logical_children(
        &mut self,
        container: &StyledElement,
        options: VisualBodyOptions,
    ) -> Vec<BodyHandle> {
        let Some(children) = container.logical_children_snapshot() else {
            return Vec::new();
        };
        let mut handles = Vec::new();
        for child in children.iter() {
            if let Some(vis) = child.cast::<Visual>() {
                handles.push(self.attach_visual(&vis, None, options));
            }
        }
        handles
    }

    /// Recursively walks the logical tree rooted at `root` and binds every visual matching `predicate`.
    pub fn attach_logical_subtree(
        &mut self,
        root: &StyledElement,
        predicate: &impl Fn(&Ref<StyledElement>) -> bool,
        options: VisualBodyOptions,
    ) -> Vec<BodyHandle> {
        let mut handles = Vec::new();
        let mut stack: Vec<Ref<StyledElement>> = Vec::new();
        if let Some(children) = root.logical_children_snapshot() {
            stack.extend(children.iter().cloned());
        }
        while let Some(elem) = stack.pop() {
            if predicate(&elem) {
                if let Some(vis) = elem.cast::<Visual>() {
                    handles.push(self.attach_visual(&vis, None, options));
                }
            } else if let Some(children) = elem.logical_children_snapshot() {
                stack.extend(children.iter().cloned());
            }
        }
        handles
    }

    /// Enables elastic spring tethers pulling every bound visual smoothly back to its layout rest slot.
    pub fn enable_home_springs(&mut self, frequency_hz: f32, damping_ratio: f32) {
        for item in &mut self.bound {
            if item.home_joint.is_some() {
                continue;
            }
            let rest_m = item.rest_center_px / self.pixels_per_meter;
            let anchor = self.world.add_body(
                BodyBuilder::static_body(Shape2D::circle(0.05))
                    .position(rest_m)
                    .sensor(true),
            );
            let jh = self.world.add_joint(Joint::spring(
                anchor,
                item.body,
                Vec2::ZERO,
                Vec2::ZERO,
                0.0,
                frequency_hz,
                damping_ratio,
            ));
            item.home_anchor = Some(anchor);
            item.home_joint = Some(jh);
            if let Some(b) = self.world.body_mut(item.body) {
                b.wake_up();
            }
        }
    }

    /// Removes home-spring tethers so bound visuals fall/tumble freely under gravity and collisions.
    pub fn disable_home_springs(&mut self) {
        for item in &mut self.bound {
            if let Some(jh) = item.home_joint.take() {
                self.world.remove_joint(jh);
            }
            if let Some(ah) = item.home_anchor.take() {
                self.world.remove_body(ah);
            }
            if let Some(b) = self.world.body_mut(item.body) {
                b.wake_up();
            }
        }
    }

    /// Applies an explosion shockwave at `center_px` with `radius_px` and `strength` (m/s impulse).
    pub fn apply_explosion_px(&mut self, center_px: Vec2, radius_px: f32, strength: f32) {
        let c = self.px_to_world(center_px);
        let r = radius_px / self.pixels_per_meter;
        self.world.apply_explosion(c, r, strength);
    }

    /// Picks a bound visual at `pointer_px` and starts dragging it with a damped [`Joint::target`].
    pub fn begin_pointer_drag_px(&mut self, pointer_px: Vec2) -> Option<BodyHandle> {
        self.end_pointer_drag();
        let pt = self.px_to_world(pointer_px);
        let handle = self.world.pick_body(pt, true)?;
        let body = self.world.body(handle)?;
        let local = body.transform().inv_transform_point(pt);
        let jh = self
            .world
            .add_joint(Joint::target(handle, local, pt, 420.0 * body.mass));
        self.drag_joint = Some(jh);
        Some(handle)
    }

    pub fn update_pointer_drag_px(&mut self, pointer_px: Vec2) {
        if let Some(jh) = self.drag_joint {
            self.world
                .set_target_joint_position(jh, self.px_to_world(pointer_px));
        }
    }

    pub fn end_pointer_drag(&mut self) {
        if let Some(jh) = self.drag_joint.take() {
            self.world.remove_joint(jh);
        }
    }

    /// Resets every bound visual cleanly to its original layout position & zero rotation.
    pub fn reset_all_to_rest(&mut self) {
        self.end_pointer_drag();
        for item in &mut self.bound {
            let rest_m = item.rest_center_px / self.pixels_per_meter;
            if let Some(b) = self.world.body_mut(item.body) {
                b.set_position(rest_m);
                b.set_angle(0.0);
                b.linear_velocity = Vec2::ZERO;
                b.angular_velocity = 0.0;
                b.force = Vec2::ZERO;
                b.torque = 0.0;
            }
            item.binding.transform.set_matrix(Matrix::IDENTITY);
            item.last_tx = 0.0;
            item.last_ty = 0.0;
            item.last_angle = 0.0;
        }
    }

    /// Re-captures layout `bounds()` for all bound visuals (call after container resize if needed).
    pub fn refresh_rest_layout(&mut self) {
        for item in &mut self.bound {
            if let Some(vis) = item.binding.visual.upgrade() {
                let rect = vis.bounds();
                if rect.width > 1.0 && rect.height > 1.0 {
                    let center_px = Vec2::new(
                        (rect.x + rect.width * 0.5) as f32,
                        (rect.y + rect.height * 0.5) as f32,
                    );
                    item.rest_center_px = center_px;
                    let center_m = center_px / self.pixels_per_meter;
                    if let Some(ah) = item.home_anchor {
                        if let Some(ab) = self.world.body_mut(ah) {
                            ab.set_position(center_m);
                        }
                    }
                    if let Some(b) = self.world.body_mut(item.body) {
                        let width = (rect.width as f32 / self.pixels_per_meter).max(0.15);
                        let height = (rect.height as f32 / self.pixels_per_meter).max(0.15);
                        b.shape = match b.shape {
                            Shape2D::Circle { .. } => Shape2D::circle(width.min(height) * 0.5),
                            _ => Shape2D::rect(width, height),
                        };
                        b.recompute_mass_properties();
                        if b.linear_velocity.length_squared() < 1e-4
                            && item.last_tx.abs() < 0.5
                            && item.last_ty.abs() < 0.5
                        {
                            b.set_position(center_m);
                        }
                    }
                }
            }
        }
    }

    /// Steps physics by `dt` and writes updated [`MatrixTransform`]s to moved visuals (`O(active)`).
    pub fn step_and_sync(&mut self, dt: f32) -> VisualSyncStats {
        if !dt.is_finite() || dt <= 0.0 {
            return self.last_sync;
        }
        let step = self.world.step(dt);
        let t_sync = Instant::now();
        let mut written = 0usize;

        // Prune detached visuals cleanly
        let mut i = 0;
        while i < self.bound.len() {
            let alive = self.bound[i].binding.visual.upgrade().is_some();
            if !alive {
                let removed = self.bound.swap_remove(i);
                if let Some(jh) = removed.home_joint {
                    self.world.remove_joint(jh);
                }
                if let Some(ah) = removed.home_anchor {
                    self.world.remove_body(ah);
                }
                self.world.remove_body(removed.body);
                continue;
            }

            let item = &mut self.bound[i];
            if let Some(body) = self.world.body(item.body) {
                let cur_px = body.position * self.pixels_per_meter;
                let tx = cur_px.x - item.rest_center_px.x;
                let ty = cur_px.y - item.rest_center_px.y;
                let angle = body.angle;

                // Softly damp angular twist when tethered home
                if item.home_joint.is_some() && angle.abs() > 1e-3 {
                    if let Some(bm) = self.world.body_mut(item.body) {
                        bm.angular_velocity += (-angle * 18.0 - bm.angular_velocity * 4.0) * dt;
                    }
                }

                if (tx - item.last_tx).abs() > 0.05
                    || (ty - item.last_ty).abs() > 0.05
                    || (angle - item.last_angle).abs() > 0.0005
                {
                    let (sin, cos) = (angle as f64).sin_cos();
                    let m = Matrix::new(cos, sin, -sin, cos, tx as f64, ty as f64);
                    item.binding.transform.set_matrix(m);
                    item.last_tx = tx;
                    item.last_ty = ty;
                    item.last_angle = angle;
                    written += 1;
                }
            }
            i += 1;
        }

        self.last_sync = VisualSyncStats {
            step,
            bound_visuals: self.bound.len(),
            transforms_written: written,
            sync_us: t_sync.elapsed().as_micros() as u64,
        };
        self.last_sync
    }
}

/// Handle to a visual driven by [`VisualSpringDriver`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpringVisualId(pub usize);

struct DrivenSpringVisual {
    binding: RenderTransformBinding,
    offset: Spring2D,
    scale: Spring1D,
    rotation: Spring1D,
    magnetic_radius_px: f32,
    magnetic_pull_px: f32,
    magnetic_tilt_rad: f32,
    settled: bool,
}

/// Analytical spring & micro-interaction driver for **any** FerroUI [`Visual`] (magnetic hover,
/// button press pop, error shake, smooth reflow, entry springs) with zero layout invalidation.
pub struct VisualSpringDriver {
    entries: Vec<Option<DrivenSpringVisual>>,
}

impl Default for VisualSpringDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualSpringDriver {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Detaches a spring binding and restores the visual's previous transform.
    /// IDs are never recycled, so a stale ID cannot affect a newly attached visual.
    pub fn detach(&mut self, id: SpringVisualId) -> bool {
        self.entries.get_mut(id.0).and_then(Option::take).is_some()
    }

    /// Attaches analytical `(offset, scale, rotation)` springs to `visual`.
    pub fn attach(&mut self, visual: &Ref<Visual>, config: SpringConfig) -> SpringVisualId {
        if let Some(index) = self.entries.iter().position(|entry| {
            entry.as_ref().is_some_and(|entry| {
                entry
                    .binding
                    .visual
                    .upgrade()
                    .is_some_and(|bound| bound.ptr_eq(visual))
            })
        }) {
            return SpringVisualId(index);
        }
        let binding = RenderTransformBinding::new(visual);

        let id = SpringVisualId(self.entries.len());
        self.entries.push(Some(DrivenSpringVisual {
            binding,
            offset: Spring2D::new(Vec2::ZERO, config),
            scale: Spring1D::new(1.0, config),
            rotation: Spring1D::new(0.0, config),
            magnetic_radius_px: 0.0,
            magnetic_pull_px: 0.0,
            magnetic_tilt_rad: 0.0,
            settled: true,
        }));
        id
    }

    /// Enables macOS/visionOS-style magnetic hover pull & subtle rotation when the pointer nears the visual.
    pub fn configure_magnetic_hover(
        &mut self,
        id: SpringVisualId,
        radius_px: f32,
        max_pull_px: f32,
        max_tilt_rad: f32,
    ) {
        if let Some(e) = self.entries.get_mut(id.0).and_then(Option::as_mut) {
            e.magnetic_radius_px = radius_px;
            e.magnetic_pull_px = max_pull_px;
            e.magnetic_tilt_rad = max_tilt_rad;
        }
    }

    /// Updates all magnetic-hover visuals given a pointer position in their shared parent coordinate space.
    pub fn update_pointer_in_parent(&mut self, pointer_px: Vec2) {
        for e in self.entries.iter_mut().filter_map(Option::as_mut) {
            if e.magnetic_radius_px <= 0.0 {
                continue;
            }
            let Some(vis) = e.binding.visual.upgrade() else {
                continue;
            };
            let b = vis.bounds();
            let center = Vec2::new((b.x + b.width * 0.5) as f32, (b.y + b.height * 0.5) as f32);
            let delta = pointer_px - center;
            let dist = delta.length();
            if dist < e.magnetic_radius_px && dist > 1e-3 {
                let factor = (1.0 - dist / e.magnetic_radius_px).powf(1.3);
                let pull = (delta / dist) * (e.magnetic_pull_px * factor);
                e.offset.set_target(pull);
                e.scale.set_target(1.0 + 0.065 * factor);
                e.rotation
                    .set_target((delta.x / e.magnetic_radius_px) * e.magnetic_tilt_rad * factor);
                e.settled = false;
            } else {
                e.offset.set_target(Vec2::ZERO);
                e.scale.set_target(1.0);
                e.rotation.set_target(0.0);
                e.settled = false;
            }
        }
    }

    /// Fires a tactile spring pop/click impulse (instant scale kick + quick bounce back).
    pub fn impulse_pop(&mut self, id: SpringVisualId, scale_velocity: f32) {
        if let Some(e) = self.entries.get_mut(id.0).and_then(Option::as_mut) {
            e.scale.impulse(scale_velocity);
            e.settled = false;
        }
    }

    /// Fires a classic macOS/iOS damped horizontal error-shake impulse on a visual (e.g., invalid form).
    pub fn impulse_shake(&mut self, id: SpringVisualId, horizontal_velocity_px: f32) {
        if let Some(e) = self.entries.get_mut(id.0).and_then(Option::as_mut) {
            e.offset.x.impulse(horizontal_velocity_px);
            e.rotation.impulse(horizontal_velocity_px * 0.0008);
            e.settled = false;
        }
    }

    /// Sets an initial entrance offset so the visual springs cleanly into its resting layout position.
    pub fn trigger_entrance(
        &mut self,
        id: SpringVisualId,
        start_offset_px: Vec2,
        start_scale: f32,
    ) {
        if let Some(e) = self.entries.get_mut(id.0).and_then(Option::as_mut) {
            e.offset.x.value = start_offset_px.x;
            e.offset.y.value = start_offset_px.y;
            e.offset.set_velocity(Vec2::ZERO);
            e.offset.set_target(Vec2::ZERO);
            e.scale.value = start_scale;
            e.scale.velocity = 0.0;
            e.scale.set_target(1.0);
            e.settled = false;
        }
    }

    /// Updates spring config across all driven visuals.
    pub fn set_all_configs(&mut self, config: SpringConfig) {
        for e in self.entries.iter_mut().filter_map(Option::as_mut) {
            e.offset.set_config(config);
            e.scale.config = config;
            e.rotation.config = config;
        }
    }

    /// Advances all unsettled springs and updates their persistent [`MatrixTransform`]s.
    /// Returns the number of active visuals updated this frame.
    pub fn step(&mut self, dt: f32) -> usize {
        if !dt.is_finite() || dt <= 0.0 {
            return 0;
        }
        for slot in &mut self.entries {
            if slot
                .as_ref()
                .is_some_and(|entry| entry.binding.visual.upgrade().is_none())
            {
                *slot = None;
            }
        }
        let mut active = 0usize;
        for e in self.entries.iter_mut().filter_map(Option::as_mut) {
            if e.settled || e.binding.visual.upgrade().is_none() {
                continue;
            }
            let off = e.offset.step(dt);
            let sc = e.scale.step(dt).max(0.05) as f64;
            let rot = e.rotation.step(dt) as f64;
            let (sin, cos) = rot.sin_cos();
            let m = Matrix::new(
                cos * sc,
                sin * sc,
                -sin * sc,
                cos * sc,
                off.x as f64,
                off.y as f64,
            );
            e.binding.transform.set_matrix(m);
            active += 1;

            if e.offset.is_settled() && e.scale.is_settled() && e.rotation.is_settled() {
                e.settled = true;
            }
        }
        active
    }
}

/// Interactive drag-and-throw controller for a live FerroUI [`Visual`] with velocity-coupled tilt
/// and magnetic slot snapping.
pub struct DraggableVisual {
    binding: RenderTransformBinding,
    pub card: DraggableCard,
    pub snap: MagneticSnap,
    tracker: GestureTracker,
    drag_start_pointer: Vec2,
    drag_start_offset: Vec2,
    time_secs: f64,
}

impl DraggableVisual {
    pub fn attach(visual: &Ref<Visual>, config: SpringConfig, snap_offsets_px: Vec<Vec2>) -> Self {
        let binding = RenderTransformBinding::new(visual);
        let mut card = DraggableCard::new(Vec2::ZERO, config);
        card.tilt_sensitivity = 0.00035;
        Self {
            binding,
            card,
            snap: MagneticSnap::new(snap_offsets_px),
            tracker: GestureTracker::default(),
            drag_start_pointer: Vec2::ZERO,
            drag_start_offset: Vec2::ZERO,
            time_secs: 0.0,
        }
    }

    pub fn begin_drag(&mut self, pointer_px: Vec2) {
        self.drag_start_pointer = pointer_px;
        self.drag_start_offset = self.card.position.value();
        self.tracker.clear();
        self.tracker.push(self.time_secs, pointer_px);
        self.card.begin_drag(self.drag_start_offset);
    }

    pub fn drag_to(&mut self, pointer_px: Vec2, dt: f32) {
        if !self.card.is_dragging || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        // Timestamp pointer samples by event elapsed time, independently of the
        // render timer; multiple input events can arrive within one frame.
        self.time_secs += dt as f64;
        self.tracker.push(self.time_secs, pointer_px);
        let target = self.drag_start_offset + (pointer_px - self.drag_start_pointer);
        self.card.drag_to(target, dt);
    }

    pub fn end_drag(&mut self) {
        if !self.card.is_dragging {
            return;
        }
        let vel = self.tracker.velocity();
        let pos = self.card.position.value();
        let target = self
            .snap
            .choose_anchor(pos, vel)
            .map_or(Vec2::ZERO, |(_, a)| a);
        self.card.end_drag(vel, target);
    }

    pub fn step(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if self.binding.visual.upgrade().is_none() {
            return;
        }
        self.card.step(dt);
        let p = self.card.position.value();
        let sc = self.card.scale.value as f64;
        let rot = self.card.tilt_radians.value as f64;
        let (sin, cos) = rot.sin_cos();
        let m = Matrix::new(
            cos * sc,
            sin * sc,
            -sin * sc,
            cos * sc,
            p.x as f64,
            p.y as f64,
        );
        self.binding.transform.set_matrix(m);
    }
}

/// Starts a 60Hz FerroUI [`DispatcherTimer`] driving both a [`VisualPhysicsHost`] and [`VisualSpringDriver`].
pub fn start_ui_physics_timer(
    host: Rc<RefCell<VisualPhysicsHost>>,
    springs: Rc<RefCell<VisualSpringDriver>>,
    on_tick: impl Fn(VisualSyncStats, usize) + 'static,
) -> Rc<DispatcherTimer> {
    let timer = DispatcherTimer::new();
    timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
    let last = std::cell::Cell::new(Instant::now());
    timer.tick(move |_| {
        let now = Instant::now();
        let dt = now
            .duration_since(last.replace(now))
            .as_secs_f32()
            .clamp(0.001, 0.033);
        let sync_stats = host.borrow_mut().step_and_sync(dt);
        let active_springs = springs.borrow_mut().step(dt);
        on_tick(sync_stats, active_springs);
    });
    timer.start();
    timer
}
