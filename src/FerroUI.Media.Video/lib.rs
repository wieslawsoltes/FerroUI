//! Native-timed video/audio playback, independent of camera and QR packages.
#![forbid(unsafe_code)]
pub use ferroui_media as media;
pub use ferroui_media::{
    frames, MediaError, MediaErrorKind, MediaEvent, MediaFuture, MediaSubscription,
};
#[cfg(not(target_family = "wasm"))]
mod native_source;
#[cfg(not(target_family = "wasm"))]
pub use native_source::*;
mod operation;
mod player;
pub use player::*;
