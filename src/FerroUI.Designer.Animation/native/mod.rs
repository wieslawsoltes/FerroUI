//! The loaded FerroUI animation objects, clocks, and authored source identities shared by native preview hosts.

mod box_shadow_curve;
mod brush_curve;
mod clock;
mod curve;
mod curve_value;
mod design_tokens;
mod programs;
mod property_transitions;
mod property_transition_catalog;
mod property_transition_edit;
mod source;
mod state_route;
mod state_styles;
mod state_triggers;
mod target;
mod transform_curve;
mod typing;
mod xaml_types;

pub use clock::{animation_timing, FrozenPreviewClock, NativePlayback};
pub use curve::{sample_curve_track, sample_keyframe_value, sample_scalar_track, sample_typed_keyframe};
pub use design_tokens::plan_design_tokens;
pub use programs::NativeProgramIndex;
pub use property_transitions::native_property_transition_types;
pub use property_transition_catalog::{
    choose_transition, transition_choices, NativeTransitionChoice, TransitionPropertyChoice,
};
pub use property_transition_edit::{plan_xaml_property_transition, transition_target_type};
pub use source::NativeSourceIndex;
pub use state_route::NativeStateRoute;
pub use state_styles::NativeStateStyles;
pub use state_triggers::{NativeStateTriggers, StateTriggerCapture, StateTriggerSelection};
pub use target::validate_animation_target;
pub use typing::{
    native_keyframe_options, native_keyframe_options_for_target, native_list_base, native_list_kind,
};
pub use xaml_types::{element_type, parent_style, registered_type, registered_xml_type, selector_parser, setter_target_type};
