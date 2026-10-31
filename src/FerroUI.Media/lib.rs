//! Shared media frames, events, errors and renderer-host contracts.
//! Video playback, camera capture and QR recognition are separate crates.
#![forbid(unsafe_code)]
mod event;
mod renderer;
pub use event::{MediaEvent, MediaSubscription};
pub use ferroui_base::rendering::composition::frames;
pub use renderer::*;
use std::{future::Future, pin::Pin};
pub type MediaFuture<T> = Pin<Box<dyn Future<Output = Result<T, MediaError>>>>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaError {
    pub kind: MediaErrorKind,
    pub message: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaErrorKind {
    Unsupported,
    InvalidSource,
    Permission,
    NativeFailure,
    Cancelled,
    AutoplayDenied,
}
impl MediaError {
    pub fn new(kind: MediaErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub fn cancelled() -> Self {
        Self::new(
            MediaErrorKind::Cancelled,
            "Source or operation was replaced",
        )
    }
}
impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for MediaError {}

/// Allocates one process-unique framework source identity. Generation and frame
/// sequence remain local to that source. Native clients may use this for hosts.
pub fn allocate_source_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1))
        .expect("media source identity exhausted")
}
