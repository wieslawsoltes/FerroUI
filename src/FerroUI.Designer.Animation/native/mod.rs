//! The loaded FerroUI animation objects, clocks, and authored source identities shared by native preview hosts.

mod clock;
mod programs;
mod source;

pub use clock::{animation_timing, FrozenPreviewClock, NativePlayback};
pub use programs::{validate_animation_target, NativeProgramIndex};
pub use source::NativeSourceIndex;
