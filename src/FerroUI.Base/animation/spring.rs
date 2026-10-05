use crate::utilities::{FormatError, SpanStringTokenizer, spring_solver::SpringSolver};
use std::cell::Cell;
use std::str::FromStr;

/// Determines how an animation is used based on spring formula.
pub struct Spring {
    spring_solver: Cell<SpringSolver>,
    mass: Cell<f64>,
    stiffness: Cell<f64>,
    damping: Cell<f64>,
    initial_velocity: Cell<f64>,
    is_dirty: Cell<bool>,
}


/// Compares by identity (reference equality), as the reference type this
/// mirrors.
impl PartialEq for Spring {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Default for Spring {
    fn default() -> Self {
        Self::new()
    }
}

impl Spring {
    /// Creates a spring with all parameters zero.
    pub fn new() -> Self {
        Self::with_values(0.0, 0.0, 0.0, 0.0)
    }

    /// Creates a spring with the given parameters.
    pub fn with_values(mass: f64, stiffness: f64, damping: f64, initial_velocity: f64) -> Self {
        Self {
            spring_solver: Cell::new(SpringSolver::default()),
            mass: Cell::new(mass),
            stiffness: Cell::new(stiffness),
            damping: Cell::new(damping),
            initial_velocity: Cell::new(initial_velocity),
            is_dirty: Cell::new(true),
        }
    }

    /// Parses a spring from four numbers: mass, stiffness, damping and
    /// initial velocity.
    pub fn parse(value: &str) -> Result<Spring, FormatError> {
        let error = |_| FormatError::from_string(format!("Invalid Spring string: \"{value}\"."));
        let mut tokenizer = SpanStringTokenizer::new(value);
        let result = Spring::with_values(
            tokenizer.read_double().map_err(error)?,
            tokenizer.read_double().map_err(error)?,
            tokenizer.read_double().map_err(error)?,
            tokenizer.read_double().map_err(error)?,
        );
        tokenizer.finish().map_err(error)?;
        Ok(result)
    }

    /// The spring mass.
    pub fn mass(&self) -> f64 {
        self.mass.get()
    }

    pub fn set_mass(&self, value: f64) {
        self.mass.set(value);
        self.is_dirty.set(true);
    }

    /// The spring stiffness.
    pub fn stiffness(&self) -> f64 {
        self.stiffness.get()
    }

    pub fn set_stiffness(&self, value: f64) {
        self.stiffness.set(value);
        self.is_dirty.set(true);
    }

    /// The spring damping.
    pub fn damping(&self) -> f64 {
        self.damping.get()
    }

    pub fn set_damping(&self, value: f64) {
        self.damping.set(value);
        self.is_dirty.set(true);
    }

    /// The spring initial velocity.
    pub fn initial_velocity(&self) -> f64 {
        self.initial_velocity.get()
    }

    pub fn set_initial_velocity(&self, value: f64) {
        self.initial_velocity.set(value);
        self.is_dirty.set(true);
    }

    /// Calculates the spring progress from a linear progress.
    pub fn get_spring_progress(&self, linear_progress: f64) -> f64 {
        if self.is_dirty.get() {
            self.build();
        }

        self.spring_solver.get().solve(linear_progress)
    }

    /// Creates the spring solver.
    fn build(&self) {
        self.spring_solver.set(SpringSolver::new(
            self.mass.get(),
            self.stiffness.get(),
            self.damping.get(),
            self.initial_velocity.get(),
        ));
        self.is_dirty.set(false);
    }
}

impl FromStr for Spring {
    type Err = FormatError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Spring::parse(s)
    }
}
