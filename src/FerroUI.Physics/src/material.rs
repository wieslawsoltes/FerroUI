//! Physical surface materials and collision layer filtering.

/// How material coefficients (friction or restitution) from two colliding bodies are combined.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CombineMode {
    /// Geometric mean `sqrt(a * b)` (Box2D standard for friction).
    #[default]
    GeometricMean,
    /// Arithmetic average `(a + b) * 0.5`.
    Average,
    /// Minimum `a.min(b)`.
    Min,
    /// Maximum `a.max(b)` (standard for bouncy restitution).
    Max,
    /// Product `a * b`.
    Multiply,
}

impl CombineMode {
    #[inline]
    pub fn combine(self, a: f32, b: f32) -> f32 {
        match self {
            Self::GeometricMean => (a * b).max(0.0).sqrt(),
            Self::Average => 0.5 * (a + b),
            Self::Min => a.min(b),
            Self::Max => a.max(b),
            Self::Multiply => a * b,
        }
    }
}

/// Surface and inertial properties of a collider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsMaterial {
    /// Mass per unit area (kg/m^2).
    pub density: f32,
    /// Coefficient of restitution (`0.0` = inelastic, `1.0` = perfectly elastic).
    pub restitution: f32,
    /// Coulomb friction coefficient (`0.0` = frictionless ice, `1.0+` = grippy rubber).
    pub friction: f32,
    /// Rolling resistance torque factor for circles/capsules.
    pub rolling_resistance: f32,
    /// Rule for combining restitution with another material.
    pub restitution_combine: CombineMode,
    /// Rule for combining friction with another material.
    pub friction_combine: CombineMode,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            density: 1.0,
            restitution: 0.25,
            friction: 0.45,
            rolling_resistance: 0.01,
            restitution_combine: CombineMode::Max,
            friction_combine: CombineMode::GeometricMean,
        }
    }
}

impl PhysicsMaterial {
    pub const fn new(density: f32, restitution: f32, friction: f32) -> Self {
        Self {
            density,
            restitution,
            friction,
            rolling_resistance: 0.01,
            restitution_combine: CombineMode::Max,
            friction_combine: CombineMode::GeometricMean,
        }
    }

    pub const fn steel() -> Self {
        Self::new(7.8, 0.25, 0.35)
    }

    pub const fn wood() -> Self {
        Self::new(0.7, 0.2, 0.5)
    }

    pub const fn rubber() -> Self {
        Self::new(1.1, 0.85, 0.9)
    }

    pub const fn ice() -> Self {
        Self::new(0.92, 0.1, 0.03)
    }

    pub const fn stone() -> Self {
        Self::new(2.5, 0.12, 0.65)
    }

    /// Snappy, moderately bouncy preset tuned for interactive UI elements.
    pub const fn ui_card() -> Self {
        Self::new(1.0, 0.45, 0.35)
    }

    #[inline]
    pub fn combine_friction(a: Self, b: Self) -> f32 {
        a.friction_combine.combine(a.friction, b.friction)
    }

    #[inline]
    pub fn combine_restitution(a: Self, b: Self) -> f32 {
        a.restitution_combine.combine(a.restitution, b.restitution)
    }
}

/// 32-bit layer/mask collision filter (plus optional negative self-exclusion group index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionFilter {
    /// Bitmask of collision layers this body belongs to.
    pub membership: u32,
    /// Bitmask of collision layers this body collides with.
    pub mask: u32,
    /// Optional group override: positive equal groups always collide, negative equal groups never collide (e.g. ragdoll parts).
    pub group_index: i16,
}

impl Default for CollisionFilter {
    fn default() -> Self {
        Self::ALL
    }
}

impl CollisionFilter {
    pub const ALL: Self = Self { membership: u32::MAX, mask: u32::MAX, group_index: 0 };
    pub const NONE: Self = Self { membership: 0, mask: 0, group_index: 0 };

    #[inline]
    pub const fn new(membership: u32, mask: u32) -> Self {
        Self { membership, mask, group_index: 0 }
    }

    #[inline]
    pub const fn with_group(mut self, group_index: i16) -> Self {
        self.group_index = group_index;
        self
    }

    #[inline]
    pub const fn should_collide(self, other: Self) -> bool {
        if self.group_index != 0 && self.group_index == other.group_index {
            return self.group_index > 0;
        }
        (self.membership & other.mask) != 0 && (other.membership & self.mask) != 0
    }
}
