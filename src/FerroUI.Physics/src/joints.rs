//! Constraint & joint suite: Distance/Spring, Revolute (hinge + limits + motor),
//! Prismatic (slider + limits + motor), Weld, Wheel (suspension + drive), Rope, and Mouse/Target drag.

use crate::body::{BodyHandle, RigidBody};
use crate::math::{Mat2, Vec2};
use std::f32::consts::TAU;

/// Type-safe generational handle to a joint in a [`crate::PhysicsWorld`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JointHandle {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl JointHandle {
    pub const INVALID: Self = Self { index: u32::MAX, generation: 0 };
}

/// Specific mechanical constraint definition.
#[derive(Clone, Debug)]
pub enum JointKind {
    /// Maintains a target distance between two local anchor points, either rigidly or as a damped harmonic spring.
    Distance {
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        rest_length: f32,
        /// Spring stiffness frequency in Hz (`0.0` = rigid rod).
        frequency_hz: f32,
        /// Damping ratio (`1.0` = critically damped, `< 1.0` = bouncy).
        damping_ratio: f32,
    },
    /// Pin / hinge joint sharing a common anchor point with optional angular limits and motor.
    Revolute {
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        reference_angle: f32,
        enable_limit: bool,
        lower_angle: f32,
        upper_angle: f32,
        enable_motor: bool,
        motor_speed: f32,
        max_motor_torque: f32,
    },
    /// Slider / piston joint constraining motion to a local axis on body A, with limits and linear motor.
    Prismatic {
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        local_axis_a: Vec2,
        reference_angle: f32,
        enable_limit: bool,
        lower_translation: f32,
        upper_translation: f32,
        enable_motor: bool,
        motor_speed: f32,
        max_motor_force: f32,
    },
    /// Glues two bodies together rigidly or with compliant bending stiffness.
    Weld {
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        reference_angle: f32,
        frequency_hz: f32,
        damping_ratio: f32,
    },
    /// Vehicle wheel suspension spring along `local_axis_a` plus rotational drive motor.
    Wheel {
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        local_axis_a: Vec2,
        frequency_hz: f32,
        damping_ratio: f32,
        enable_motor: bool,
        motor_speed: f32,
        max_motor_torque: f32,
    },
    /// Enforces a maximum distance between two local anchors (goes slack when closer).
    Rope {
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        max_length: f32,
    },
    /// Pulls a local anchor on `body_a` toward a world-space `target` (interactive mouse/touch drag).
    Target {
        local_anchor_a: Vec2,
        target: Vec2,
        frequency_hz: f32,
        damping_ratio: f32,
        max_force: f32,
    },
}

/// A joint connecting `body_a` and `body_b` (for `Target`, `body_b` is ignored).
#[derive(Clone, Debug)]
pub struct Joint {
    pub handle: JointHandle,
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub kind: JointKind,
    pub collide_connected: bool,
    /// Optional impulse/force threshold above which the joint snaps (destructible structures!).
    pub break_force: Option<f32>,
    pub broken: bool,
    // Warm-start / diagnostic accumulated impulses
    pub(crate) impulse_linear: Vec2,
    pub(crate) impulse_angular: f32,
    pub(crate) impulse_motor: f32,
    pub(crate) impulse_limit: f32,
}

impl Joint {
    pub fn distance(
        body_a: BodyHandle,
        body_b: BodyHandle,
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        rest_length: f32,
    ) -> Self {
        Self::new(
            body_a,
            body_b,
            JointKind::Distance {
                local_anchor_a,
                local_anchor_b,
                rest_length: rest_length.max(0.0),
                frequency_hz: 0.0,
                damping_ratio: 0.7,
            },
        )
    }

    pub fn spring(
        body_a: BodyHandle,
        body_b: BodyHandle,
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        rest_length: f32,
        frequency_hz: f32,
        damping_ratio: f32,
    ) -> Self {
        Self::new(
            body_a,
            body_b,
            JointKind::Distance {
                local_anchor_a,
                local_anchor_b,
                rest_length: rest_length.max(0.0),
                frequency_hz: frequency_hz.max(0.1),
                damping_ratio: damping_ratio.max(0.0),
            },
        )
    }

    pub fn revolute(
        body_a: BodyHandle,
        body_b: BodyHandle,
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
    ) -> Self {
        Self::new(
            body_a,
            body_b,
            JointKind::Revolute {
                local_anchor_a,
                local_anchor_b,
                reference_angle: 0.0,
                enable_limit: false,
                lower_angle: -std::f32::consts::PI,
                upper_angle: std::f32::consts::PI,
                enable_motor: false,
                motor_speed: 0.0,
                max_motor_torque: 0.0,
            },
        )
    }

    pub fn prismatic(
        body_a: BodyHandle,
        body_b: BodyHandle,
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        local_axis_a: Vec2,
    ) -> Self {
        Self::new(
            body_a,
            body_b,
            JointKind::Prismatic {
                local_anchor_a,
                local_anchor_b,
                local_axis_a: local_axis_a.normalize_or_zero(),
                reference_angle: 0.0,
                enable_limit: false,
                lower_translation: 0.0,
                upper_translation: 0.0,
                enable_motor: false,
                motor_speed: 0.0,
                max_motor_force: 0.0,
            },
        )
    }

    pub fn weld(
        body_a: BodyHandle,
        body_b: BodyHandle,
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        reference_angle: f32,
    ) -> Self {
        Self::new(
            body_a,
            body_b,
            JointKind::Weld {
                local_anchor_a,
                local_anchor_b,
                reference_angle,
                frequency_hz: 0.0,
                damping_ratio: 1.0,
            },
        )
    }

    pub fn wheel(
        body_a: BodyHandle,
        body_b: BodyHandle,
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        local_axis_a: Vec2,
        frequency_hz: f32,
        damping_ratio: f32,
    ) -> Self {
        Self::new(
            body_a,
            body_b,
            JointKind::Wheel {
                local_anchor_a,
                local_anchor_b,
                local_axis_a: local_axis_a.normalize_or_zero(),
                frequency_hz: frequency_hz.max(0.5),
                damping_ratio: damping_ratio.max(0.05),
                enable_motor: false,
                motor_speed: 0.0,
                max_motor_torque: 0.0,
            },
        )
    }

    pub fn rope(
        body_a: BodyHandle,
        body_b: BodyHandle,
        local_anchor_a: Vec2,
        local_anchor_b: Vec2,
        max_length: f32,
    ) -> Self {
        Self::new(
            body_a,
            body_b,
            JointKind::Rope { local_anchor_a, local_anchor_b, max_length: max_length.max(0.0) },
        )
    }

    pub fn target(body_a: BodyHandle, local_anchor_a: Vec2, target: Vec2, max_force: f32) -> Self {
        Self::new(
            body_a,
            BodyHandle::INVALID,
            JointKind::Target {
                local_anchor_a,
                target,
                frequency_hz: 6.0,
                damping_ratio: 0.85,
                max_force: max_force.max(1.0),
            },
        )
    }

    pub fn new(body_a: BodyHandle, body_b: BodyHandle, kind: JointKind) -> Self {
        Self {
            handle: JointHandle::INVALID,
            body_a,
            body_b,
            kind,
            collide_connected: false,
            break_force: None,
            broken: false,
            impulse_linear: Vec2::ZERO,
            impulse_angular: 0.0,
            impulse_motor: 0.0,
            impulse_limit: 0.0,
        }
    }

    pub fn with_collide_connected(mut self, collide: bool) -> Self {
        self.collide_connected = collide;
        self
    }

    pub fn with_break_force(mut self, max_force: f32) -> Self {
        self.break_force = Some(max_force.max(1.0));
        self
    }

    pub fn with_motor(mut self, speed: f32, max_effort: f32) -> Self {
        match &mut self.kind {
            JointKind::Revolute { enable_motor, motor_speed, max_motor_torque, .. }
            | JointKind::Wheel { enable_motor, motor_speed, max_motor_torque, .. } => {
                *enable_motor = true;
                *motor_speed = speed;
                *max_motor_torque = max_effort;
            }
            JointKind::Prismatic { enable_motor, motor_speed, max_motor_force, .. } => {
                *enable_motor = true;
                *motor_speed = speed;
                *max_motor_force = max_effort;
            }
            _ => {}
        }
        self
    }

    pub fn with_limits(mut self, lower: f32, upper: f32) -> Self {
        match &mut self.kind {
            JointKind::Revolute { enable_limit, lower_angle, upper_angle, .. } => {
                *enable_limit = true;
                *lower_angle = lower;
                *upper_angle = upper;
            }
            JointKind::Prismatic { enable_limit, lower_translation, upper_translation, .. } => {
                *enable_limit = true;
                *lower_translation = lower;
                *upper_translation = upper;
            }
            _ => {}
        }
        self
    }

    /// World-space endpoints `(anchor_a, anchor_b)` for debug/UI rendering.
    pub fn world_anchors(&self, a: &RigidBody, b: Option<&RigidBody>) -> (Vec2, Vec2) {
        match &self.kind {
            JointKind::Distance { local_anchor_a, local_anchor_b, .. }
            | JointKind::Revolute { local_anchor_a, local_anchor_b, .. }
            | JointKind::Prismatic { local_anchor_a, local_anchor_b, .. }
            | JointKind::Weld { local_anchor_a, local_anchor_b, .. }
            | JointKind::Wheel { local_anchor_a, local_anchor_b, .. }
            | JointKind::Rope { local_anchor_a, local_anchor_b, .. } => {
                let wa = a.transform().transform_point(*local_anchor_a);
                let wb = b.map_or(wa, |rb| rb.transform().transform_point(*local_anchor_b));
                (wa, wb)
            }
            JointKind::Target { local_anchor_a, target, .. } => {
                (a.transform().transform_point(*local_anchor_a), *target)
            }
        }
    }
}

/// Computes Erin Catto's soft-constraint `(gamma, bias_factor)` parameters from `(frequency_hz, damping_ratio, eff_mass, dt)`.
#[inline]
fn soft_constraint_params(frequency_hz: f32, damping_ratio: f32, eff_mass: f32, dt: f32) -> (f32, f32) {
    if frequency_hz <= 0.0 || eff_mass <= 0.0 {
        return (0.0, 0.25 / dt.max(1e-5));
    }
    let omega = TAU * frequency_hz;
    let d = 2.0 * eff_mass * damping_ratio * omega;
    let k = eff_mass * omega * omega;
    let h = dt;
    let gamma_denom = h * (d + h * k);
    let gamma = if gamma_denom > 1e-8 { 1.0 / gamma_denom } else { 0.0 };
    let beta_over_h = h * k * gamma;
    (gamma, beta_over_h)
}

/// Solves one velocity iteration of a [`Joint`].
pub(crate) fn solve_joint_velocity(joint: &mut Joint, a: &mut RigidBody, b: Option<&mut RigidBody>, dt: f32) {
    if joint.broken {
        return;
    }

    match joint.kind {
        JointKind::Target { local_anchor_a, target, frequency_hz, damping_ratio, max_force } => {
            if !a.is_dynamic() {
                return;
            }
            let r = a.rotation.rotate(local_anchor_a);
            let p = a.position + r;
            let k11 = a.inv_mass + a.inv_inertia * r.y * r.y;
            let k12 = -a.inv_inertia * r.x * r.y;
            let k22 = a.inv_mass + a.inv_inertia * r.x * r.x;

            let eff_m = if k11 + k22 > 1e-8 { 2.0 / (k11 + k22) } else { 0.0 };
            let (gamma, bias_rate) = soft_constraint_params(frequency_hz, damping_ratio, eff_m, dt);

            let mat = Mat2::from_cols(Vec2::new(k11 + gamma, k12), Vec2::new(k12, k22 + gamma));
            let c = p - target;
            let vel = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, r);
            let rhs = -(vel + c * bias_rate);
            let mut impulse = mat.solve(rhs);

            let max_imp = max_force * dt;
            let imp_len = impulse.length();
            if imp_len > max_imp {
                impulse *= max_imp / imp_len;
            }

            a.linear_velocity += impulse * a.inv_mass;
            a.angular_velocity += r.cross(impulse) * a.inv_inertia;
            joint.impulse_linear = impulse;
        }
        JointKind::Distance { local_anchor_a, local_anchor_b, rest_length, frequency_hz, damping_ratio } => {
            let Some(b) = b else { return };
            let ra = a.rotation.rotate(local_anchor_a);
            let rb = b.rotation.rotate(local_anchor_b);
            let pa = a.position + ra;
            let pb = b.position + rb;
            let (axis, dist) = (pb - pa).normalize_with_length();
            if dist < 1e-6 {
                return;
            }
            let rxa = ra.cross(axis);
            let rxb = rb.cross(axis);
            let inv_eff = a.inv_mass + b.inv_mass + a.inv_inertia * rxa * rxa + b.inv_inertia * rxb * rxb;
            if inv_eff <= 1e-8 {
                return;
            }
            let eff_m = 1.0 / inv_eff;
            let (gamma, bias_rate) = soft_constraint_params(frequency_hz, damping_ratio, eff_m, dt);

            let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
            let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);
            let c_dot = (vb - va).dot(axis);
            let c = dist - rest_length;

            let lambda = -(c_dot + c * bias_rate) / (inv_eff + gamma);
            let p = axis * lambda;
            apply_pair_impulse(a, b, ra, rb, p);
            joint.impulse_linear = p;
            check_break(joint, lambda.abs() / dt.max(1e-5));
        }
        JointKind::Rope { local_anchor_a, local_anchor_b, max_length } => {
            let Some(b) = b else { return };
            let ra = a.rotation.rotate(local_anchor_a);
            let rb = b.rotation.rotate(local_anchor_b);
            let pa = a.position + ra;
            let pb = b.position + rb;
            let (axis, dist) = (pb - pa).normalize_with_length();
            let c = dist - max_length;
            if c <= 0.0 || dist < 1e-6 {
                return;
            }
            let rxa = ra.cross(axis);
            let rxb = rb.cross(axis);
            let inv_eff = a.inv_mass + b.inv_mass + a.inv_inertia * rxa * rxa + b.inv_inertia * rxb * rxb;
            if inv_eff <= 1e-8 {
                return;
            }
            let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
            let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);
            let c_dot = (vb - va).dot(axis);
            let bias = 0.3 * c / dt.max(1e-5);
            let lambda = (-(c_dot + bias) / inv_eff).min(0.0);
            let p = axis * lambda;
            apply_pair_impulse(a, b, ra, rb, p);
            joint.impulse_linear = p;
            check_break(joint, lambda.abs() / dt.max(1e-5));
        }
        JointKind::Revolute {
            local_anchor_a,
            local_anchor_b,
            reference_angle,
            enable_limit,
            lower_angle,
            upper_angle,
            enable_motor,
            motor_speed,
            max_motor_torque,
        } => {
            let Some(b) = b else { return };
            let ra = a.rotation.rotate(local_anchor_a);
            let rb = b.rotation.rotate(local_anchor_b);

            // 1. Point-to-point 2-DOF constraint
            let k11 = a.inv_mass + b.inv_mass + a.inv_inertia * ra.y * ra.y + b.inv_inertia * rb.y * rb.y;
            let k12 = -a.inv_inertia * ra.x * ra.y - b.inv_inertia * rb.x * rb.y;
            let k22 = a.inv_mass + b.inv_mass + a.inv_inertia * ra.x * ra.x + b.inv_inertia * rb.x * rb.x;
            let mat = Mat2::from_cols(Vec2::new(k11, k12), Vec2::new(k12, k22));

            let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
            let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);
            let c = (b.position + rb) - (a.position + ra);
            let bias = c * (0.25 / dt.max(1e-5));
            let impulse = mat.solve(-(vb - va + bias));
            apply_pair_impulse(a, b, ra, rb, impulse);
            joint.impulse_linear = impulse;

            let inv_i_sum = a.inv_inertia + b.inv_inertia;
            if inv_i_sum > 1e-8 {
                // 2. Optional motor
                if enable_motor {
                    let c_dot = b.angular_velocity - a.angular_velocity - motor_speed;
                    let mut motor_imp = -c_dot / inv_i_sum;
                    let max_imp = max_motor_torque * dt;
                    motor_imp = motor_imp.clamp(-max_imp, max_imp);
                    a.angular_velocity -= a.inv_inertia * motor_imp;
                    b.angular_velocity += b.inv_inertia * motor_imp;
                    joint.impulse_motor = motor_imp;
                }

                // 3. Optional angular limit
                if enable_limit {
                    let angle = b.angle - a.angle - reference_angle;
                    if angle < lower_angle {
                        let c_ang = angle - lower_angle;
                        let c_dot = b.angular_velocity - a.angular_velocity;
                        let bias = 0.25 * c_ang / dt.max(1e-5);
                        let imp = (-(c_dot + bias) / inv_i_sum).max(0.0);
                        a.angular_velocity -= a.inv_inertia * imp;
                        b.angular_velocity += b.inv_inertia * imp;
                        joint.impulse_limit = imp;
                    } else if angle > upper_angle {
                        let c_ang = angle - upper_angle;
                        let c_dot = b.angular_velocity - a.angular_velocity;
                        let bias = 0.25 * c_ang / dt.max(1e-5);
                        let imp = (-(c_dot + bias) / inv_i_sum).min(0.0);
                        a.angular_velocity -= a.inv_inertia * imp;
                        b.angular_velocity += b.inv_inertia * imp;
                        joint.impulse_limit = imp;
                    }
                }
            }
            check_break(joint, impulse.length() / dt.max(1e-5));
        }
        JointKind::Weld { local_anchor_a, local_anchor_b, reference_angle, frequency_hz, damping_ratio } => {
            let Some(b) = b else { return };
            let ra = a.rotation.rotate(local_anchor_a);
            let rb = b.rotation.rotate(local_anchor_b);

            let k11 = a.inv_mass + b.inv_mass + a.inv_inertia * ra.y * ra.y + b.inv_inertia * rb.y * rb.y;
            let k12 = -a.inv_inertia * ra.x * ra.y - b.inv_inertia * rb.x * rb.y;
            let k22 = a.inv_mass + b.inv_mass + a.inv_inertia * ra.x * ra.x + b.inv_inertia * rb.x * rb.x;
            let mat = Mat2::from_cols(Vec2::new(k11, k12), Vec2::new(k12, k22));

            let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
            let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);
            let c = (b.position + rb) - (a.position + ra);
            let bias = c * (0.25 / dt.max(1e-5));
            let impulse = mat.solve(-(vb - va + bias));
            apply_pair_impulse(a, b, ra, rb, impulse);
            joint.impulse_linear = impulse;

            // Angular lock (rigid or compliant bendable weld)
            let inv_i_sum = a.inv_inertia + b.inv_inertia;
            if inv_i_sum > 1e-8 {
                let (gamma, bias_rate) = soft_constraint_params(frequency_hz, damping_ratio, 1.0 / inv_i_sum, dt);
                let c_ang = b.angle - a.angle - reference_angle;
                let c_dot = b.angular_velocity - a.angular_velocity;
                let ang_imp = -(c_dot + c_ang * bias_rate) / (inv_i_sum + gamma);
                a.angular_velocity -= a.inv_inertia * ang_imp;
                b.angular_velocity += b.inv_inertia * ang_imp;
                joint.impulse_angular = ang_imp;
            }
            check_break(joint, impulse.length() / dt.max(1e-5));
        }
        JointKind::Prismatic {
            local_anchor_a,
            local_anchor_b,
            local_axis_a,
            reference_angle,
            enable_limit,
            lower_translation,
            upper_translation,
            enable_motor,
            motor_speed,
            max_motor_force,
        } => {
            let Some(b) = b else { return };
            let ra = a.rotation.rotate(local_anchor_a);
            let rb = b.rotation.rotate(local_anchor_b);
            let d = (b.position + rb) - (a.position + ra);
            let axis = a.rotation.rotate(local_axis_a).normalize_or_zero();
            let perp = axis.perp();

            let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
            let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);

            // 1. Constrain motion perpendicular to axis
            let rxa_p = (d + ra).cross(perp);
            let rxb_p = rb.cross(perp);
            let inv_eff_p = a.inv_mass + b.inv_mass + a.inv_inertia * rxa_p * rxa_p + b.inv_inertia * rxb_p * rxb_p;
            if inv_eff_p > 1e-8 {
                let c_p = d.dot(perp);
                let c_dot_p = (vb - va).dot(perp);
                let lambda = -(c_dot_p + 0.25 * c_p / dt.max(1e-5)) / inv_eff_p;
                apply_pair_impulse(a, b, ra, rb, perp * lambda);
                joint.impulse_linear = perp * lambda;
            }

            // 2. Lock relative rotation
            let inv_i_sum = a.inv_inertia + b.inv_inertia;
            if inv_i_sum > 1e-8 {
                let c_ang = b.angle - a.angle - reference_angle;
                let c_dot = b.angular_velocity - a.angular_velocity;
                let ang_imp = -(c_dot + 0.25 * c_ang / dt.max(1e-5)) / inv_i_sum;
                a.angular_velocity -= a.inv_inertia * ang_imp;
                b.angular_velocity += b.inv_inertia * ang_imp;
                joint.impulse_angular = ang_imp;
            }

            // 3. Linear motor & translation limits along axis
            let rxa_a = (d + ra).cross(axis);
            let rxb_a = rb.cross(axis);
            let inv_eff_a = a.inv_mass + b.inv_mass + a.inv_inertia * rxa_a * rxa_a + b.inv_inertia * rxb_a * rxb_a;
            if inv_eff_a > 1e-8 {
                if enable_motor {
                    let va2 = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
                    let vb2 = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);
                    let c_dot = (vb2 - va2).dot(axis) - motor_speed;
                    let max_imp = max_motor_force * dt;
                    let imp = (-c_dot / inv_eff_a).clamp(-max_imp, max_imp);
                    apply_pair_impulse(a, b, ra, rb, axis * imp);
                    joint.impulse_motor = imp;
                }
                if enable_limit {
                    let translation = d.dot(axis);
                    let va2 = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
                    let vb2 = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);
                    let c_dot = (vb2 - va2).dot(axis);
                    if translation < lower_translation {
                        let bias = 0.25 * (translation - lower_translation) / dt.max(1e-5);
                        let imp = (-(c_dot + bias) / inv_eff_a).max(0.0);
                        apply_pair_impulse(a, b, ra, rb, axis * imp);
                        joint.impulse_limit = imp;
                    } else if translation > upper_translation {
                        let bias = 0.25 * (translation - upper_translation) / dt.max(1e-5);
                        let imp = (-(c_dot + bias) / inv_eff_a).min(0.0);
                        apply_pair_impulse(a, b, ra, rb, axis * imp);
                        joint.impulse_limit = imp;
                    }
                }
            }
        }
        JointKind::Wheel {
            local_anchor_a,
            local_anchor_b,
            local_axis_a,
            frequency_hz,
            damping_ratio,
            enable_motor,
            motor_speed,
            max_motor_torque,
        } => {
            let Some(b) = b else { return };
            let ra = a.rotation.rotate(local_anchor_a);
            let rb = b.rotation.rotate(local_anchor_b);
            let d = (b.position + rb) - (a.position + ra);
            let axis = a.rotation.rotate(local_axis_a).normalize_or_zero();
            let perp = axis.perp();

            // 1. Rigid lateral guide (perpendicular to suspension travel)
            let rxa_p = (d + ra).cross(perp);
            let rxb_p = rb.cross(perp);
            let inv_eff_p = a.inv_mass + b.inv_mass + a.inv_inertia * rxa_p * rxa_p + b.inv_inertia * rxb_p * rxb_p;
            if inv_eff_p > 1e-8 {
                let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
                let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);
                let c_p = d.dot(perp);
                let c_dot_p = (vb - va).dot(perp);
                let lambda = -(c_dot_p + 0.3 * c_p / dt.max(1e-5)) / inv_eff_p;
                apply_pair_impulse(a, b, ra, rb, perp * lambda);
            }

            // 2. Suspension spring along `axis`
            let rxa_s = (d + ra).cross(axis);
            let rxb_s = rb.cross(axis);
            let inv_eff_s = a.inv_mass + b.inv_mass + a.inv_inertia * rxa_s * rxa_s + b.inv_inertia * rxb_s * rxb_s;
            if inv_eff_s > 1e-8 {
                let (gamma, bias_rate) = soft_constraint_params(frequency_hz, damping_ratio, 1.0 / inv_eff_s, dt);
                let va = a.linear_velocity + Vec2::cross_scalar_left(a.angular_velocity, ra);
                let vb = b.linear_velocity + Vec2::cross_scalar_left(b.angular_velocity, rb);
                let c_s = d.dot(axis);
                let c_dot_s = (vb - va).dot(axis);
                let lambda = -(c_dot_s + c_s * bias_rate) / (inv_eff_s + gamma);
                apply_pair_impulse(a, b, ra, rb, axis * lambda);
                joint.impulse_linear = axis * lambda;
            }

            // 3. Wheel drive motor
            let inv_i_sum = a.inv_inertia + b.inv_inertia;
            if enable_motor && inv_i_sum > 1e-8 {
                let c_dot = b.angular_velocity - a.angular_velocity - motor_speed;
                let max_imp = max_motor_torque * dt;
                let imp = (-c_dot / inv_i_sum).clamp(-max_imp, max_imp);
                a.angular_velocity -= a.inv_inertia * imp;
                b.angular_velocity += b.inv_inertia * imp;
                joint.impulse_motor = imp;
            }
        }
    }
}

#[inline]
fn apply_pair_impulse(a: &mut RigidBody, b: &mut RigidBody, ra: Vec2, rb: Vec2, impulse: Vec2) {
    a.linear_velocity -= impulse * a.inv_mass;
    a.angular_velocity -= ra.cross(impulse) * a.inv_inertia;
    b.linear_velocity += impulse * b.inv_mass;
    b.angular_velocity += rb.cross(impulse) * b.inv_inertia;
}

#[inline]
fn check_break(joint: &mut Joint, reaction_force: f32) {
    if let Some(limit) = joint.break_force {
        if reaction_force > limit {
            joint.broken = true;
        }
    }
}
