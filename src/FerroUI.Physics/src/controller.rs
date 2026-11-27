//! Kinematic 2D character controller (`CharacterController2D`) with swept move-and-slide,
//! slope classification, coyote time, jump buffering, and dynamic rigid-body pushing.

use crate::body::BodyHandle;
use crate::material::CollisionFilter;
use crate::math::{Transform2D, Vec2};
use crate::narrowphase::collide_shapes;
use crate::shapes::Shape2D;
use crate::world::PhysicsWorld;

/// Output of a single [`CharacterController2D::move_and_slide`] call.
#[derive(Clone, Copy, Debug, Default)]
pub struct CharacterMoveResult {
    pub position: Vec2,
    pub velocity: Vec2,
    pub is_grounded: bool,
    pub is_on_wall: bool,
    pub is_on_ceiling: bool,
    pub ground_normal: Vec2,
    pub slope_angle_radians: f32,
    pub hit_count: usize,
}

/// Production-ready 2D game character controller with swept slide-and-collide, slope limits,
/// coyote-time / jump-buffer ergonomics, and impulse transfer onto dynamic boxes.
#[derive(Clone, Debug)]
pub struct CharacterController2D {
    pub shape: Shape2D,
    pub position: Vec2,
    pub velocity: Vec2,
    /// Maximum walkable slope angle in radians (default ~48 degrees).
    pub max_slope_radians: f32,
    /// Small collision skin margin to prevent sticking on acute corners.
    pub skin_width: f32,
    /// Impulse multiplier applied when pushing dynamic rigid bodies.
    pub push_strength: f32,
    pub filter: CollisionFilter,
    /// Coyote time remaining (seconds after walking off a ledge where jump is still allowed).
    pub coyote_timer: f32,
    pub coyote_duration: f32,
    /// Jump buffer remaining (seconds a pressed jump stays queued before landing).
    pub jump_buffer_timer: f32,
    pub jump_buffer_duration: f32,
    pub last_result: CharacterMoveResult,
}

impl CharacterController2D {
    pub fn new(shape: Shape2D, position: Vec2) -> Self {
        Self {
            shape,
            position,
            velocity: Vec2::ZERO,
            max_slope_radians: 48.0_f32.to_radians(),
            skin_width: 0.015,
            push_strength: 1.6,
            filter: CollisionFilter::ALL,
            coyote_timer: 0.0,
            coyote_duration: 0.12,
            jump_buffer_timer: 0.0,
            jump_buffer_duration: 0.12,
            last_result: CharacterMoveResult::default(),
        }
    }

    /// Queues a jump request (honored immediately if grounded/in coyote time, or upon landing within jump buffer).
    pub fn request_jump(&mut self) {
        self.jump_buffer_timer = self.jump_buffer_duration;
    }

    /// Consumes a buffered jump if currently allowed and returns `true` when jump fired.
    pub fn try_consume_jump(&mut self, jump_speed: f32) -> bool {
        if self.jump_buffer_timer > 0.0 && self.coyote_timer > 0.0 {
            self.velocity.y = -jump_speed.abs();
            self.coyote_timer = 0.0;
            self.jump_buffer_timer = 0.0;
            true
        } else {
            false
        }
    }

    /// Sweeps and slides the character through `world` over `dt` seconds, resolving slopes,
    /// walls, ceilings, and pushing dynamic rigid bodies it walks into.
    pub fn move_and_slide(
        &mut self,
        world: &mut PhysicsWorld,
        dt: f32,
        ignore_body: Option<BodyHandle>,
    ) -> CharacterMoveResult {
        if dt <= 0.0 {
            return self.last_result;
        }
        self.jump_buffer_timer = (self.jump_buffer_timer - dt).max(0.0);

        let mut result = CharacterMoveResult {
            position: self.position,
            velocity: self.velocity,
            ground_normal: Vec2::new(0.0, -1.0),
            ..Default::default()
        };

        let mut remaining_disp = self.velocity * dt;
        let max_cos_slope = self.max_slope_radians.cos();

        for _slide in 0..4 {
            let step_len = remaining_disp.length();
            if step_len < 1e-5 {
                break;
            }
            let dir = remaining_disp / step_len;

            if let Some(hit) = world.shape_cast(
                &self.shape,
                Transform2D::from_position(self.position),
                dir,
                step_len + self.skin_width,
                self.filter,
                ignore_body,
            ) {
                let advance = (hit.toi - self.skin_width).max(0.0);
                self.position += dir * advance;
                result.hit_count += 1;

                // Classify surface normal (world is Y-down: ground normal points roughly (0, -1))
                let up_dot = hit.normal.dot(Vec2::new(0.0, -1.0));
                if up_dot >= max_cos_slope {
                    result.is_grounded = true;
                    result.ground_normal = hit.normal;
                    result.slope_angle_radians = up_dot.clamp(-1.0, 1.0).acos();
                } else if up_dot <= -0.7 {
                    result.is_on_ceiling = true;
                } else {
                    result.is_on_wall = true;
                }

                // Transfer push impulse to dynamic bodies
                let vn = self.velocity.dot(hit.normal);
                if vn < 0.0 {
                    if let Some(body) = world.body_mut(hit.body) {
                        if body.is_dynamic() {
                            body.apply_impulse_at_point(-hit.normal * (-vn * self.push_strength * body.mass.min(3.0)), hit.point);
                        }
                    }
                    // Slide velocity along surface plane
                    self.velocity -= hit.normal * vn;
                }

                let leftover = (step_len - advance).max(0.0);
                let rem_vec = dir * leftover;
                remaining_disp = rem_vec - hit.normal * rem_vec.dot(hit.normal);
            } else {
                self.position += remaining_disp;
                break;
            }
        }

        // Depenetration pass against moving/overlapping bodies
        let query_aabb = self.shape.compute_aabb(Transform2D::from_position(self.position)).expanded(0.05);
        let candidates = world.aabb_query(query_aabb, self.filter);
        for h in candidates {
            if Some(h) == ignore_body {
                continue;
            }
            let Some(body) = world.body(h) else { continue };
            if body.is_sensor {
                continue;
            }
            if let Some(m) = collide_shapes(
                BodyHandle::INVALID,
                &self.shape,
                Transform2D::from_position(self.position),
                h,
                &body.shape,
                body.transform(),
            ) {
                let max_d = m.points.iter().map(|p| p.depth).fold(0.0_f32, f32::max);
                if max_d > 0.0 {
                    // Normal points from character -> body, so push character along -normal
                    let push_n = -m.normal;
                    self.position += push_n * max_d;
                    if push_n.dot(Vec2::new(0.0, -1.0)) >= max_cos_slope {
                        result.is_grounded = true;
                        result.ground_normal = push_n;
                        if self.velocity.y > 0.0 {
                            self.velocity.y = 0.0;
                        }
                    }
                }
            }
        }

        if result.is_grounded {
            self.coyote_timer = self.coyote_duration;
        } else {
            self.coyote_timer = (self.coyote_timer - dt).max(0.0);
        }

        result.position = self.position;
        result.velocity = self.velocity;
        self.last_result = result;
        result
    }
}
