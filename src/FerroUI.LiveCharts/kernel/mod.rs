//! Chart coordinates and shared numerical algorithms.

mod coordinate;
mod error;
pub mod line_spline_math;
mod stacker;

pub use coordinate::Coordinate;
pub use error::Error;
pub use stacker::{StackedValue, Stacker};
