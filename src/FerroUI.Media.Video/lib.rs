//! Native-timed video/audio playback, independent of camera and QR packages.
#![forbid(unsafe_code)]
pub use ferroui_media as media;
pub use ferroui_media::{
    frames, MediaError, MediaErrorKind, MediaEvent, MediaFuture, MediaSubscription,
};
mod operation;
mod player;
pub use player::*;
