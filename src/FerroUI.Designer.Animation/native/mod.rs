//! The loaded FerroUI animation objects, clocks, and authored source identities shared by native preview hosts.

mod clock;
mod curve;
mod programs;
mod source;
mod state_route;
mod typing;

pub use clock::{animation_timing, FrozenPreviewClock, NativePlayback};
pub use curve::{sample_keyframe_value, sample_scalar_track};
pub use programs::{validate_animation_target, NativeProgramIndex};
pub use source::NativeSourceIndex;
pub use state_route::NativeStateRoute;
pub use typing::native_keyframe_options;
