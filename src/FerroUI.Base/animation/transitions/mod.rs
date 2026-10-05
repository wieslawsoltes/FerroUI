//! The transition classes for the built-in value types.

mod bool_transition;
mod box_shadows_transition;
mod brush_transition;
mod color_transition;
mod corner_radius_transition;
mod double_transition;
mod effect_transition;
mod float_transition;
mod integer_transition;
mod point_transition;
mod relative_point_transition;
mod rotate_3d_transition;
mod size_transition;
mod thickness_transition;
mod transform_operations_transition;
mod vector_transition;

pub use bool_transition::BoolTransition;
pub use box_shadows_transition::BoxShadowsTransition;
pub use brush_transition::BrushTransition;
pub use color_transition::ColorTransition;
pub use corner_radius_transition::CornerRadiusTransition;
pub use double_transition::DoubleTransition;
pub use effect_transition::EffectTransition;
pub use float_transition::FloatTransition;
pub use integer_transition::IntegerTransition;
pub use point_transition::PointTransition;
pub use relative_point_transition::RelativePointTransition;
pub use rotate_3d_transition::Rotate3DTransition;
pub use size_transition::SizeTransition;
pub use thickness_transition::ThicknessTransition;
pub use transform_operations_transition::TransformOperationsTransition;
pub use vector_transition::VectorTransition;
