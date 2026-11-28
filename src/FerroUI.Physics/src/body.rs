//! Rigid body representation, generational handles, and ergonomic fluent builder API.

use crate::material::{CollisionFilter, PhysicsMaterial};
use crate::math::{Aabb2, Rot2, Transform2D, Vec2};
use crate::shapes::Shape2D;

/// Type-safe generational handle to a rigid body inside a [`crate::PhysicsWorld`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BodyHandle {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl BodyHandle {
    pub const INVALID: Self = Self { index: u32::MAX, generation: 0 };

    #[inline]
    pub const fn index(self) -> usize {
        self.index as usize
    }

    #[inline]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

/// Dynamics mode of a rigid body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BodyType {
    /// Simulated by forces, gravity, impulses, and constraints.
    #[default]
    Dynamic,
    /// Infinite mass, driven by user velocity/position (moving platforms, dragged UI panels).
    Kinematic,
    /// Infinite mass, immovable world geometry (floors, walls, static terrain).
    Static,
}

/// Rigid body in 2D space with full inertial, material, damping, CCD, and sleep state.
#[derive(Clone, Debug)]
pub struct RigidBody {
    pub handle: BodyHandle,
    pub body_type: BodyType,
    pub shape: Shape2D,
    pub material: PhysicsMaterial,
    pub filter: CollisionFilter,

    // Transform state (current & previous for smooth render interpolation)
    pub position: Vec2,
    pub prev_position: Vec2,
    pub rotation: Rot2,
    pub prev_rotation: Rot2,
    pub angle: f32,

    // Velocity state
    pub linear_velocity: Vec2,
    pub angular_velocity: f32,

    // Force & torque accumulators (cleared after each step)
    pub force: Vec2,
    pub torque: f32,

    // Mass properties
    pub mass: f32,
    pub inv_mass: f32,
    pub inertia: f32,
    pub inv_inertia: f32,

    // Behavioral flags & tuning
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
    pub fixed_rotation: bool,
    pub is_sensor: bool,
    pub is_bullet: bool,
    pub can_sleep: bool,
    pub is_sleeping: bool,
    pub sleep_time: f32,

    // Cached world AABB & broadphase proxy ID
    pub aabb: Aabb2,
    pub(crate) bvh_proxy: Option<usize>,

    /// Caller-owned opaque identifier (e.g., ECS entity ID, UI widget ID, gameplay tag).
    pub user_data: u64,
}

impl RigidBody {
    #[inline]
    pub fn transform(&self) -> Transform2D {
        Transform2D { position: self.position, rotation: self.rotation }
    }

    /// Interpolated transform between the previous and current physics tick (`alpha` in `0..=1`).
    #[inline]
    pub fn interpolated_transform(&self, alpha: f32) -> Transform2D {
        Transform2D {
            position: self.prev_position.lerp(self.position, alpha),
            rotation: self.prev_rotation.nlerp(self.rotation, alpha),
        }
    }

    #[inline]
    pub fn is_dynamic(&self) -> bool {
        self.body_type == BodyType::Dynamic
    }

    #[inline]
    pub fn is_static(&self) -> bool {
        self.body_type == BodyType::Static
    }

    #[inline]
    pub fn is_kinematic(&self) -> bool {
        self.body_type == BodyType::Kinematic
    }

    /// Wakes a sleeping dynamic body.
    #[inline]
    pub fn wake_up(&mut self) {
        self.is_sleeping = false;
        self.sleep_time = 0.0;
    }

    /// Puts a dynamic body to sleep immediately, zeroing velocity.
    #[inline]
    pub fn put_to_sleep(&mut self) {
        if self.can_sleep && self.is_dynamic() {
            self.is_sleeping = true;
            self.linear_velocity = Vec2::ZERO;
            self.angular_velocity = 0.0;
            self.force = Vec2::ZERO;
            self.torque = 0.0;
        }
    }

    /// Applies a continuous force at the center of mass.
    #[inline]
    pub fn apply_force(&mut self, force: Vec2) {
        if self.is_dynamic() {
            self.wake_up();
            self.force += force;
        }
    }

    /// Applies a continuous force at a world-space point, producing both linear force and torque.
    #[inline]
    pub fn apply_force_at_point(&mut self, force: Vec2, world_point: Vec2) {
        if self.is_dynamic() {
            self.wake_up();
            self.force += force;
            self.torque += (world_point - self.position).cross(force);
        }
    }

    /// Applies a continuous rotational torque.
    #[inline]
    pub fn apply_torque(&mut self, torque: f32) {
        if self.is_dynamic() && !self.fixed_rotation {
            self.wake_up();
            self.torque += torque;
        }
    }

    /// Applies an instantaneous linear impulse at the center of mass.
    #[inline]
    pub fn apply_impulse(&mut self, impulse: Vec2) {
        if self.is_dynamic() {
            self.wake_up();
            self.linear_velocity += impulse * self.inv_mass;
        }
    }

    /// Applies an instantaneous impulse at a world-space point.
    #[inline]
    pub fn apply_impulse_at_point(&mut self, impulse: Vec2, world_point: Vec2) {
        if self.is_dynamic() {
            self.wake_up();
            self.linear_velocity += impulse * self.inv_mass;
            self.angular_velocity += (world_point - self.position).cross(impulse) * self.inv_inertia;
        }
    }

    /// Applies an instantaneous angular impulse.
    #[inline]
    pub fn apply_angular_impulse(&mut self, impulse: f32) {
        if self.is_dynamic() && !self.fixed_rotation {
            self.wake_up();
            self.angular_velocity += impulse * self.inv_inertia;
        }
    }

    /// Velocity of a world-space point attached to this rigid body.
    #[inline]
    pub fn velocity_at_world_point(&self, world_point: Vec2) -> Vec2 {
        let r = world_point - self.position;
        self.linear_velocity + Vec2::cross_scalar_left(self.angular_velocity, r)
    }

    /// Kinetic energy `0.5 * m * v^2 + 0.5 * I * w^2`.
    #[inline]
    pub fn kinetic_energy(&self) -> f32 {
        if !self.is_dynamic() {
            return 0.0;
        }
        0.5 * self.mass * self.linear_velocity.length_squared()
            + 0.5 * self.inertia * self.angular_velocity * self.angular_velocity
    }

    #[inline]
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
        self.prev_position = pos;
        self.aabb = self.shape.compute_aabb(self.transform());
        self.wake_up();
    }

    #[inline]
    pub fn set_angle(&mut self, radians: f32) {
        self.angle = radians;
        self.rotation = Rot2::from_radians(radians);
        self.prev_rotation = self.rotation;
        self.aabb = self.shape.compute_aabb(self.transform());
        self.wake_up();
    }

    /// Recomputes mass & inertia from the current shape, density, and body type.
    pub fn recompute_mass_properties(&mut self) {
        if self.body_type != BodyType::Dynamic {
            self.mass = 0.0;
            self.inv_mass = 0.0;
            self.inertia = 0.0;
            self.inv_inertia = 0.0;
            return;
        }
        let props = self.shape.compute_mass_properties(self.material.density);
        if props.mass > 1e-6 {
            self.mass = props.mass;
            self.inv_mass = 1.0 / props.mass;
        } else {
            self.mass = 1.0;
            self.inv_mass = 1.0;
        }
        if self.fixed_rotation || props.inertia <= 1e-6 {
            self.inertia = 0.0;
            self.inv_inertia = 0.0;
        } else {
            self.inertia = props.inertia;
            self.inv_inertia = 1.0 / props.inertia;
        }
    }
}

/// Fluent builder for creating [`RigidBody`] instances cleanly.
#[derive(Clone, Debug)]
pub struct BodyBuilder {
    body_type: BodyType,
    shape: Shape2D,
    material: PhysicsMaterial,
    filter: CollisionFilter,
    position: Vec2,
    angle: f32,
    linear_velocity: Vec2,
    angular_velocity: f32,
    linear_damping: f32,
    angular_damping: f32,
    gravity_scale: f32,
    fixed_rotation: bool,
    is_sensor: bool,
    is_bullet: bool,
    can_sleep: bool,
    mass_override: Option<f32>,
    user_data: u64,
}

impl BodyBuilder {
    pub fn dynamic(shape: Shape2D) -> Self {
        Self::new(BodyType::Dynamic, shape)
    }

    pub fn static_body(shape: Shape2D) -> Self {
        Self::new(BodyType::Static, shape)
    }

    pub fn kinematic(shape: Shape2D) -> Self {
        Self::new(BodyType::Kinematic, shape)
    }

    pub fn new(body_type: BodyType, shape: Shape2D) -> Self {
        Self {
            body_type,
            shape,
            material: PhysicsMaterial::default(),
            filter: CollisionFilter::ALL,
            position: Vec2::ZERO,
            angle: 0.0,
            linear_velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            linear_damping: 0.08,
            angular_damping: 0.12,
            gravity_scale: 1.0,
            fixed_rotation: false,
            is_sensor: false,
            is_bullet: false,
            can_sleep: true,
            mass_override: None,
            user_data: 0,
        }
    }

    #[inline]
    pub fn position(mut self, pos: impl Into<Vec2>) -> Self {
        self.position = pos.into();
        self
    }

    #[inline]
    pub fn angle(mut self, radians: f32) -> Self {
        self.angle = radians;
        self
    }

    #[inline]
    pub fn linear_velocity(mut self, vel: impl Into<Vec2>) -> Self {
        self.linear_velocity = vel.into();
        self
    }

    #[inline]
    pub fn angular_velocity(mut self, omega: f32) -> Self {
        self.angular_velocity = omega;
        self
    }

    #[inline]
    pub fn material(mut self, material: PhysicsMaterial) -> Self {
        self.material = material;
        self
    }

    #[inline]
    pub fn density(mut self, density: f32) -> Self {
        self.material.density = density;
        self
    }

    #[inline]
    pub fn restitution(mut self, restitution: f32) -> Self {
        self.material.restitution = restitution;
        self
    }

    #[inline]
    pub fn friction(mut self, friction: f32) -> Self {
        self.material.friction = friction;
        self
    }

    #[inline]
    pub fn filter(mut self, filter: CollisionFilter) -> Self {
        self.filter = filter;
        self
    }

    #[inline]
    pub fn linear_damping(mut self, damping: f32) -> Self {
        self.linear_damping = damping.max(0.0);
        self
    }

    #[inline]
    pub fn angular_damping(mut self, damping: f32) -> Self {
        self.angular_damping = damping.max(0.0);
        self
    }

    #[inline]
    pub fn gravity_scale(mut self, scale: f32) -> Self {
        self.gravity_scale = scale;
        self
    }

    #[inline]
    pub fn fixed_rotation(mut self, fixed: bool) -> Self {
        self.fixed_rotation = fixed;
        self
    }

    #[inline]
    pub fn sensor(mut self, is_sensor: bool) -> Self {
        self.is_sensor = is_sensor;
        self
    }

    #[inline]
    pub fn bullet(mut self, is_bullet: bool) -> Self {
        self.is_bullet = is_bullet;
        self
    }

    #[inline]
    pub fn can_sleep(mut self, can_sleep: bool) -> Self {
        self.can_sleep = can_sleep;
        self
    }

    #[inline]
    pub fn mass(mut self, mass: f32) -> Self {
        self.mass_override = Some(mass.max(1e-5));
        self
    }

    #[inline]
    pub fn user_data(mut self, user_data: u64) -> Self {
        self.user_data = user_data;
        self
    }

    pub(crate) fn build(self, handle: BodyHandle) -> RigidBody {
        let rot = Rot2::from_radians(self.angle);
        let transform = Transform2D { position: self.position, rotation: rot };
        let aabb = self.shape.compute_aabb(transform);
        let mut body = RigidBody {
            handle,
            body_type: self.body_type,
            shape: self.shape,
            material: self.material,
            filter: self.filter,
            position: self.position,
            prev_position: self.position,
            rotation: rot,
            prev_rotation: rot,
            angle: self.angle,
            linear_velocity: self.linear_velocity,
            angular_velocity: self.angular_velocity,
            force: Vec2::ZERO,
            torque: 0.0,
            mass: 0.0,
            inv_mass: 0.0,
            inertia: 0.0,
            inv_inertia: 0.0,
            linear_damping: self.linear_damping,
            angular_damping: self.angular_damping,
            gravity_scale: self.gravity_scale,
            fixed_rotation: self.fixed_rotation,
            is_sensor: self.is_sensor,
            is_bullet: self.is_bullet,
            can_sleep: self.can_sleep,
            is_sleeping: false,
            sleep_time: 0.0,
            aabb,
            bvh_proxy: None,
            user_data: self.user_data,
        };
        body.recompute_mass_properties();
        if let Some(m) = self.mass_override {
            if body.is_dynamic() && body.mass > 1e-6 {
                let ratio = m / body.mass;
                body.mass = m;
                body.inv_mass = 1.0 / m;
                if !body.fixed_rotation {
                    body.inertia *= ratio;
                    body.inv_inertia = if body.inertia > 1e-6 { 1.0 / body.inertia } else { 0.0 };
                }
            }
        }
        body
    }
}
