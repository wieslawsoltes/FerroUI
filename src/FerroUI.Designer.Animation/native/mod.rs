//! The loaded FerroUI animation objects, clocks, and authored source identities shared by native preview hosts.

mod box_shadow_curve;
mod brush_curve;
mod clock;
mod curve;
mod curve_value;
mod programs;
mod property_transitions;
mod property_transition_catalog;
mod source;
mod state_route;
mod state_styles;
mod state_triggers;
mod target;
mod transform_curve;
mod typing;

pub use clock::{animation_timing, FrozenPreviewClock, NativePlayback};
pub use curve::{sample_curve_track, sample_keyframe_value, sample_scalar_track, sample_typed_keyframe};
pub use programs::NativeProgramIndex;
pub use property_transitions::native_property_transition_types;
pub use property_transition_catalog::{
    choose_transition, transition_choices, NativeTransitionChoice, TransitionPropertyChoice,
};
pub use source::NativeSourceIndex;
pub use state_route::NativeStateRoute;
pub use state_styles::NativeStateStyles;
pub use state_triggers::{NativeStateTriggers, StateTriggerCapture, StateTriggerSelection};
pub use target::validate_animation_target;
pub use typing::{
    native_keyframe_options, native_keyframe_options_for_target, native_list_base, native_list_kind,
};
