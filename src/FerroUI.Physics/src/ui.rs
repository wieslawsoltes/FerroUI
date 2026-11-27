//! Dedicated UI animation & interaction physics primitives.
//!
//! Designed for 60Hz–240Hz user interfaces:
//! - Exact closed-form analytical damped harmonic springs ([`Spring1D`], [`Spring2D`], [`Spring3D`])
//!   that are unconditionally stable at any `dt` (no numerical explosion on frame hitches!).
//! - Native-feel [`InertiaScroller`] with exponential deceleration, rubber-band overscroll
//!   resistance, spring bounce-back, and optional magnetic paging detents.
//! - [`GestureTracker`] velocity estimator for fluid drag-and-throw gestures.
//! - [`DraggableCard`] combining 2D spring positioning with velocity-driven physical tilt torque.

use crate::math::{Vec2, Vec3};
use std::collections::VecDeque;
use std::f32::consts::TAU;

/// Parameters for a damped harmonic spring (`m * x'' + c * x' + k * (x - target) = 0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringConfig {
    /// Spring stiffness `k`.
    pub stiffness: f32,
    /// Damping coefficient `c`.
    pub damping: f32,
    /// Mass `m` (default `1.0`).
    pub mass: f32,
    /// Position epsilon for [`Spring1D::is_settled`].
    pub position_tolerance: f32,
    /// Velocity epsilon for [`Spring1D::is_settled`].
    pub velocity_tolerance: f32,
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self::snappy()
    }
}

impl SpringConfig {
    pub const fn new(stiffness: f32, damping: f32, mass: f32) -> Self {
        Self {
            stiffness,
            damping,
            mass,
            position_tolerance: 0.001,
            velocity_tolerance: 0.001,
        }
    }

    /// Creates a spring from intuitive perceptual parameters (Apple SwiftUI / UIKit style):
    /// - `response`: approximate duration in seconds to settle (`0.15..=0.8`).
    /// - `damping_ratio`: `1.0` = critically damped (no overshoot), `< 1.0` = bouncy, `> 1.0` = overdamped.
    pub fn from_response_damping(response: f32, damping_ratio: f32) -> Self {
        let r = response.max(0.02);
        let zeta = damping_ratio.max(0.0);
        let omega = TAU / r;
        let stiffness = omega * omega;
        let damping = 2.0 * zeta * omega;
        Self::new(stiffness, damping, 1.0)
    }

    /// Fast, crisp UI transition with minimal overshoot.
    pub fn snappy() -> Self {
        Self::from_response_damping(0.28, 0.88)
    }

    /// Playful, tactile bounce for cards, chips, and interactive controls.
    pub fn bouncy() -> Self {
        Self::from_response_damping(0.38, 0.55)
    }

    /// Smooth, calm transition for drawers, sheets, and modals.
    pub fn gentle() -> Self {
        Self::from_response_damping(0.55, 0.92)
    }

    /// Critically damped (fastest approach with strictly zero overshoot).
    pub fn critically_damped(response: f32) -> Self {
        Self::from_response_damping(response, 1.0)
    }

    /// Expressive, elastic wobble for celebratory/game UI feedback.
    pub fn wobbly() -> Self {
        Self::from_response_damping(0.45, 0.32)
    }

    #[inline]
    pub fn damping_ratio(self) -> f32 {
        let denom = 2.0 * (self.stiffness * self.mass).max(1e-8).sqrt();
        self.damping / denom
    }
}

/// Exact analytical closed-form 1D harmonic spring.
#[derive(Clone, Copy, Debug)]
pub struct Spring1D {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
    pub config: SpringConfig,
}

impl Spring1D {
    pub fn new(initial: f32, config: SpringConfig) -> Self {
        Self { value: initial, velocity: 0.0, target: initial, config }
    }

    #[inline]
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    #[inline]
    pub fn impulse(&mut self, delta_velocity: f32) {
        self.velocity += delta_velocity;
    }

    #[inline]
    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }

    /// Returns `true` when both displacement and velocity are within settling tolerances.
    #[inline]
    pub fn is_settled(&self) -> bool {
        (self.value - self.target).abs() <= self.config.position_tolerance
            && self.velocity.abs() <= self.config.velocity_tolerance
    }

    /// Advances the spring by `dt` seconds using the exact closed-form ODE solution.
    /// Unconditionally stable for any `dt >= 0`.
    pub fn step(&mut self, dt: f32) -> f32 {
        if dt <= 0.0 || !dt.is_finite() {
            return self.value;
        }
        let m = self.config.mass.max(1e-5);
        let k = self.config.stiffness.max(0.0);
        let c = self.config.damping.max(0.0);

        let x0 = self.value - self.target;
        let v0 = self.velocity;

        if k <= 1e-8 {
            self.value += v0 * dt;
            return self.value;
        }

        let omega0 = (k / m).sqrt();
        let zeta = c / (2.0 * (k * m).sqrt());

        let (x_new, v_new) = if zeta < 1.0 - 1e-4 {
            // Underdamped
            let omega_d = omega0 * (1.0 - zeta * zeta).sqrt();
            let decay = (-zeta * omega0 * dt).exp();
            let (sin_wd, cos_wd) = (omega_d * dt).sin_cos();
            let c1 = x0;
            let c2 = (v0 + zeta * omega0 * x0) / omega_d;
            let x = decay * (c1 * cos_wd + c2 * sin_wd);
            let v = decay * ((c2 * omega_d - c1 * zeta * omega0) * cos_wd
                - (c1 * omega_d + c2 * zeta * omega0) * sin_wd);
            (x, v)
        } else if zeta > 1.0 + 1e-4 {
            // Overdamped
            let root = (zeta * zeta - 1.0).sqrt();
            let r1 = -omega0 * (zeta - root);
            let r2 = -omega0 * (zeta + root);
            let c2 = (v0 - r1 * x0) / (r2 - r1);
            let c1 = x0 - c2;
            let e1 = (r1 * dt).exp();
            let e2 = (r2 * dt).exp();
            let x = c1 * e1 + c2 * e2;
            let v = c1 * r1 * e1 + c2 * r2 * e2;
            (x, v)
        } else {
            // Critically damped
            let decay = (-omega0 * dt).exp();
            let c1 = x0;
            let c2 = v0 + omega0 * x0;
            let x = decay * (c1 + c2 * dt);
            let v = decay * (c2 - omega0 * (c1 + c2 * dt));
            (x, v)
        };

        self.value = self.target + x_new;
        self.velocity = v_new;

        if self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
        }
        self.value
    }
}

/// Analytical 2D vector spring for UI positions, offsets, scales, and camera tracking.
#[derive(Clone, Copy, Debug)]
pub struct Spring2D {
    pub x: Spring1D,
    pub y: Spring1D,
}

impl Spring2D {
    pub fn new(initial: Vec2, config: SpringConfig) -> Self {
        Self { x: Spring1D::new(initial.x, config), y: Spring1D::new(initial.y, config) }
    }

    pub fn set_config(&mut self, config: SpringConfig) {
        self.x.config = config;
        self.y.config = config;
    }

    #[inline]
    pub fn value(&self) -> Vec2 {
        Vec2::new(self.x.value, self.y.value)
    }

    #[inline]
    pub fn velocity(&self) -> Vec2 {
        Vec2::new(self.x.velocity, self.y.velocity)
    }

    #[inline]
    pub fn target(&self) -> Vec2 {
        Vec2::new(self.x.target, self.y.target)
    }

    #[inline]
    pub fn set_target(&mut self, target: Vec2) {
        self.x.set_target(target.x);
        self.y.set_target(target.y);
    }

    #[inline]
    pub fn set_velocity(&mut self, velocity: Vec2) {
        self.x.velocity = velocity.x;
        self.y.velocity = velocity.y;
    }

    #[inline]
    pub fn snap(&mut self, pos: Vec2) {
        self.x.snap(pos.x);
        self.y.snap(pos.y);
    }

    #[inline]
    pub fn is_settled(&self) -> bool {
        self.x.is_settled() && self.y.is_settled()
    }

    #[inline]
    pub fn step(&mut self, dt: f32) -> Vec2 {
        Vec2::new(self.x.step(dt), self.y.step(dt))
    }
}

/// Analytical 3D vector spring for 3D UI transforms, parallax, and color interpolation.
#[derive(Clone, Copy, Debug)]
pub struct Spring3D {
    pub x: Spring1D,
    pub y: Spring1D,
    pub z: Spring1D,
}

impl Spring3D {
    pub fn new(initial: Vec3, config: SpringConfig) -> Self {
        Self {
            x: Spring1D::new(initial.x, config),
            y: Spring1D::new(initial.y, config),
            z: Spring1D::new(initial.z, config),
        }
    }

    #[inline]
    pub fn value(&self) -> Vec3 {
        Vec3::new(self.x.value, self.y.value, self.z.value)
    }

    #[inline]
    pub fn set_target(&mut self, target: Vec3) {
        self.x.set_target(target.x);
        self.y.set_target(target.y);
        self.z.set_target(target.z);
    }

    #[inline]
    pub fn is_settled(&self) -> bool {
        self.x.is_settled() && self.y.is_settled() && self.z.is_settled()
    }

    #[inline]
    pub fn step(&mut self, dt: f32) -> Vec3 {
        Vec3::new(self.x.step(dt), self.y.step(dt), self.z.step(dt))
    }
}

/// Native iOS/macOS-style inertia scroller with rubber-band overscroll resistance,
/// bounce-back spring, velocity projection, and optional magnetic detents/paging.
#[derive(Clone, Debug)]
pub struct InertiaScroller {
    pub position: f32,
    pub velocity: f32,
    pub min_bound: f32,
    pub max_bound: f32,
    /// Per-millisecond deceleration rate (iOS normal = `0.998`, fast = `0.990`).
    pub deceleration_rate: f32,
    /// Rubber-band coefficient (`0.55` matches Apple UIScrollView).
    pub rubber_band_coeff: f32,
    /// Viewport extent used to scale rubber-band stretch.
    pub viewport_extent: f32,
    /// Spring used when snapping back from overscroll or toward a detent.
    pub snap_spring: SpringConfig,
    /// Optional snap detent positions (e.g., carousel pages, sheet stops).
    pub detents: Vec<f32>,
    is_dragging: bool,
    drag_raw_position: f32,
    snap_target: Option<f32>,
}

impl InertiaScroller {
    pub fn new(min_bound: f32, max_bound: f32, viewport_extent: f32) -> Self {
        Self {
            position: min_bound,
            velocity: 0.0,
            min_bound,
            max_bound: max_bound.max(min_bound),
            deceleration_rate: 0.997,
            rubber_band_coeff: 0.55,
            viewport_extent: viewport_extent.max(50.0),
            snap_spring: SpringConfig::from_response_damping(0.35, 0.92),
            detents: Vec::new(),
            is_dragging: false,
            drag_raw_position: min_bound,
            snap_target: None,
        }
    }

    /// Apple UIScrollView rubber-band resistance formula:
    /// `f(x, d, c) = (1 - 1 / (x * c / d + 1)) * d`.
    #[inline]
    pub fn rubber_band(offset: f32, dimension: f32, coeff: f32) -> f32 {
        let sign = offset.signum();
        let x = offset.abs();
        let d = dimension.max(1.0);
        sign * (1.0 - 1.0 / (x * coeff / d + 1.0)) * d
    }

    /// Predicts where the scroller will naturally come to rest given current position and velocity.
    #[inline]
    pub fn projected_rest_position(&self) -> f32 {
        let ln_rate = self.deceleration_rate.clamp(0.9, 0.9999).ln();
        self.position - self.velocity / (1000.0 * ln_rate)
    }

    pub fn begin_drag(&mut self) {
        self.is_dragging = true;
        self.velocity = 0.0;
        self.snap_target = None;
        self.drag_raw_position = self.position;
    }

    pub fn drag_by(&mut self, delta: f32) {
        if !self.is_dragging {
            self.begin_drag();
        }
        self.drag_raw_position += delta;
        self.position = if self.drag_raw_position < self.min_bound {
            self.min_bound + Self::rubber_band(self.drag_raw_position - self.min_bound, self.viewport_extent, self.rubber_band_coeff)
        } else if self.drag_raw_position > self.max_bound {
            self.max_bound + Self::rubber_band(self.drag_raw_position - self.max_bound, self.viewport_extent, self.rubber_band_coeff)
        } else {
            self.drag_raw_position
        };
    }

    pub fn end_drag(&mut self, release_velocity: f32) {
        self.is_dragging = false;
        self.velocity = release_velocity;

        if self.position < self.min_bound {
            self.snap_target = Some(self.min_bound);
        } else if self.position > self.max_bound {
            self.snap_target = Some(self.max_bound);
        } else if !self.detents.is_empty() {
            let projected = self.projected_rest_position().clamp(self.min_bound, self.max_bound);
            if let Some(&nearest) = self
                .detents
                .iter()
                .min_by(|a, b| (*a - projected).abs().total_cmp(&(*b - projected).abs()))
            {
                self.snap_target = Some(nearest);
            }
        }
    }

    pub fn step(&mut self, dt: f32) -> f32 {
        if self.is_dragging || dt <= 0.0 {
            return self.position;
        }

        if let Some(target) = self.snap_target {
            let mut s = Spring1D {
                value: self.position,
                velocity: self.velocity,
                target,
                config: self.snap_spring,
            };
            s.step(dt);
            self.position = s.value;
            self.velocity = s.velocity;
            if s.is_settled() {
                self.snap_target = None;
            }
            return self.position;
        }

        // Exponential decay: v(t) = v0 * rate^(1000 * t)
        let decay = self.deceleration_rate.powf(1000.0 * dt);
        let avg_v = self.velocity * (1.0 + decay) * 0.5;
        self.position += avg_v * dt;
        self.velocity *= decay;

        if self.position < self.min_bound {
            self.snap_target = Some(self.min_bound);
        } else if self.position > self.max_bound {
            self.snap_target = Some(self.max_bound);
        } else if self.velocity.abs() < 1.0 {
            self.velocity = 0.0;
        }
        self.position
    }
}

/// Velocity-aware magnetic anchor snapping (great for floating palettes, PiP windows, and Kanban cards).
#[derive(Clone, Debug)]
pub struct MagneticSnap {
    pub anchors: Vec<Vec2>,
    /// Multiplier projecting velocity into future target space (seconds, e.g. `0.14`).
    pub velocity_projection: f32,
}

impl MagneticSnap {
    pub fn new(anchors: Vec<Vec2>) -> Self {
        Self { anchors, velocity_projection: 0.14 }
    }

    /// Selects the best anchor for a released object at `position` with `velocity`.
    pub fn choose_anchor(&self, position: Vec2, velocity: Vec2) -> Option<(usize, Vec2)> {
        let projected = position + velocity * self.velocity_projection;
        self.anchors
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.distance_squared(projected).total_cmp(&b.distance_squared(projected))
            })
            .map(|(i, &p)| (i, p))
    }
}

/// Sliding-window pointer velocity tracker for silky-smooth throw/fling gestures.
#[derive(Clone, Debug)]
pub struct GestureTracker {
    samples: VecDeque<(f64, Vec2)>,
    window_secs: f64,
}

impl Default for GestureTracker {
    fn default() -> Self {
        Self::new(0.08)
    }
}

impl GestureTracker {
    pub fn new(window_secs: f64) -> Self {
        Self { samples: VecDeque::with_capacity(16), window_secs: window_secs.max(0.01) }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn push(&mut self, time_secs: f64, position: Vec2) {
        self.samples.push_back((time_secs, position));
        while let Some(&(t0, _)) = self.samples.front() {
            if time_secs - t0 > self.window_secs && self.samples.len() > 2 {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    /// Estimates instantaneous pointer velocity in units/second.
    pub fn velocity(&self) -> Vec2 {
        if self.samples.len() < 2 {
            return Vec2::ZERO;
        }
        let (t0, p0) = *self.samples.front().unwrap();
        let (t1, p1) = *self.samples.back().unwrap();
        let dt = (t1 - t0) as f32;
        if dt > 1e-4 {
            (p1 - p0) / dt
        } else {
            Vec2::ZERO
        }
    }
}

/// Interactive draggable UI card with 2D spring positioning, elevation scale spring, and
/// velocity-coupled physical tilt rotation.
#[derive(Clone, Debug)]
pub struct DraggableCard {
    pub position: Spring2D,
    pub tilt_radians: Spring1D,
    pub scale: Spring1D,
    pub is_dragging: bool,
    /// Tilt sensitivity in radians per unit/sec horizontal drag velocity.
    pub tilt_sensitivity: f32,
    pub max_tilt_radians: f32,
    last_drag_pos: Vec2,
}

impl DraggableCard {
    pub fn new(initial_pos: Vec2, config: SpringConfig) -> Self {
        Self {
            position: Spring2D::new(initial_pos, config),
            tilt_radians: Spring1D::new(0.0, SpringConfig::from_response_damping(0.25, 0.72)),
            scale: Spring1D::new(1.0, SpringConfig::snappy()),
            is_dragging: false,
            tilt_sensitivity: 0.00055,
            max_tilt_radians: 0.28,
            last_drag_pos: initial_pos,
        }
    }

    pub fn set_spring_config(&mut self, config: SpringConfig) {
        self.position.set_config(config);
    }

    pub fn begin_drag(&mut self, pos: Vec2) {
        self.is_dragging = true;
        self.last_drag_pos = pos;
        self.scale.set_target(1.06);
    }

    pub fn drag_to(&mut self, pos: Vec2, dt: f32) {
        let vx = if dt > 1e-4 { (pos.x - self.last_drag_pos.x) / dt } else { 0.0 };
        self.last_drag_pos = pos;
        self.position.set_target(pos);
        self.tilt_radians
            .set_target((vx * self.tilt_sensitivity).clamp(-self.max_tilt_radians, self.max_tilt_radians));
    }

    pub fn end_drag(&mut self, release_velocity: Vec2, snap_target: Vec2) {
        self.is_dragging = false;
        self.position.set_velocity(release_velocity);
        self.position.set_target(snap_target);
        self.tilt_radians.set_target(0.0);
        self.scale.set_target(1.0);
    }

    pub fn step(&mut self, dt: f32) {
        self.position.step(dt);
        if !self.is_dragging {
            let vx = self.position.velocity().x;
            self.tilt_radians.set_target(
                (vx * self.tilt_sensitivity * 0.5).clamp(-self.max_tilt_radians, self.max_tilt_radians),
            );
        }
        self.tilt_radians.step(dt);
        self.scale.step(dt);
    }
}
