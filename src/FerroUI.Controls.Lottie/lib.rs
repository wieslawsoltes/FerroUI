//! Lottie animation playback and custom Skia composition controls.
//! Enable the `skia` feature on desktop to use these controls.
#![cfg(feature = "skia")]

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
compile_error!("Lottie currently supports the Skia renderer on desktop targets.");

mod composition_animated_control;
mod composition_animated_custom_visual_handler;
mod feedback;
mod lottie;
mod normalize_result;
mod playback;
mod register_types;
mod visual_payload;

pub use composition_animated_control::{
    CompositionAnimatedControl, CompositionAnimation, CompositionAnimationFactory,
};
pub use lottie::Lottie;
pub use normalize_result::NormalizeResult;
pub use register_types::{ASSEMBLY, register_types};
