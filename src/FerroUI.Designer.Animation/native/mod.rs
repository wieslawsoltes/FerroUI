//! The loaded FerroUI animation objects, clocks, and authored source identities shared by native preview hosts.

mod clock;
mod curve;
mod curve_value;
mod programs;
mod source;
mod state_route;
mod state_styles;
mod state_triggers;
mod typing;

pub use clock::{animation_timing, FrozenPreviewClock, NativePlayback};
pub use curve::{sample_curve_track, sample_keyframe_value, sample_scalar_track, sample_typed_keyframe};
pub use programs::{validate_animation_target, NativeProgramIndex};
pub use source::NativeSourceIndex;
pub use state_route::NativeStateRoute;
pub use state_styles::NativeStateStyles;
pub use state_triggers::{NativeStateTriggers, StateTriggerCapture, StateTriggerSelection};
pub use typing::native_keyframe_options;
