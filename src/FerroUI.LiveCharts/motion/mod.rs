//! Clock-driven animation values. The host supplies elapsed milliseconds.

mod animation;
mod interpolate;
mod motion_property;

pub use animation::{Animation, Repeat};
pub use interpolate::{Discrete, Interpolate};
pub use motion_property::{ByReferenceMotionProperty, MotionProperty};

pub type DoubleMotionProperty = MotionProperty<f64>;
pub type FloatMotionProperty = MotionProperty<f32>;
pub type ColorMotionProperty = MotionProperty<ferroui_base::media::Color>;
pub type PointMotionProperty = MotionProperty<ferroui_base::Point>;
pub type SizeMotionProperty = MotionProperty<ferroui_base::Size>;
pub type PaddingMotionProperty = MotionProperty<Option<ferroui_base::Thickness>>;
pub type NullableDoubleMotionProperty = MotionProperty<Option<f64>>;
pub type FloatArrayMotionProperty = MotionProperty<Option<Vec<f32>>>;
