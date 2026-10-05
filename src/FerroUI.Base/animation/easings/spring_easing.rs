use crate::animation::easings::IEasing;
use crate::animation::Spring;

/// Eases a value using a user-defined spring formula.
pub struct SpringEasing {
    internal_spring: Spring,
}

impl SpringEasing {
    /// Creates a spring easing with all parameters zero.
    pub fn new() -> Self {
        Self { internal_spring: Spring::new() }
    }

    /// Creates a spring easing with the given parameters.
    pub fn with_values(mass: f64, stiffness: f64, damping: f64, initial_velocity: f64) -> Self {
        Self { internal_spring: Spring::with_values(mass, stiffness, damping, initial_velocity) }
    }

    /// The spring mass.
    pub fn mass(&self) -> f64 {
        self.internal_spring.mass()
    }

    pub fn set_mass(&self, value: f64) {
        self.internal_spring.set_mass(value)
    }

    /// The spring stiffness.
    pub fn stiffness(&self) -> f64 {
        self.internal_spring.stiffness()
    }

    pub fn set_stiffness(&self, value: f64) {
        self.internal_spring.set_stiffness(value)
    }

    /// The spring damping.
    pub fn damping(&self) -> f64 {
        self.internal_spring.damping()
    }

    pub fn set_damping(&self, value: f64) {
        self.internal_spring.set_damping(value)
    }

    /// The spring initial velocity.
    pub fn initial_velocity(&self) -> f64 {
        self.internal_spring.initial_velocity()
    }

    pub fn set_initial_velocity(&self, value: f64) {
        self.internal_spring.set_initial_velocity(value)
    }
}

impl Default for SpringEasing {
    fn default() -> Self {
        Self::new()
    }
}

impl IEasing for SpringEasing {
    fn ease(&self, progress: f64) -> f64 {
        self.internal_spring.get_spring_progress(progress)
    }
}
