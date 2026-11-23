//! The records of pipelines an application ships (docs/porting/gpu-rendering.md, "Pipelines made
//! ahead of their first draw").

use std::borrow::Cow;
use std::sync::{Mutex, PoisonError};

static REGISTERED: Mutex<Vec<Cow<'static, [u8]>>> = Mutex::new(Vec::new());

/// Registers a record of pipelines, to be made ahead when the GPU context of the process is
/// made: before the platform starts, in `main`.
///
/// A record is what a run of the application wrote with `FERROUI_PIPELINE_RECORD=<file>` in its
/// environment: the keys of the pipelines its screens made, a few kilobytes, which the
/// application ships (`include_bytes!`, or a file it reads). With it the pipelines of its first
/// screen are compiled on threads while the screen is built, where a first start after an
/// install waits for them inside its first frame. A record belongs to the version of the
/// framework and to the kind of device it was recorded on; a backend that has no use for it (any
/// but Skia's Graphite today) keeps nothing of it.
pub fn register_pipeline_record(record: impl Into<Cow<'static, [u8]>>) {
    REGISTERED.lock().unwrap_or_else(PoisonError::into_inner).push(record.into());
}

/// What was registered.
#[cfg_attr(not(all(target_vendor = "apple", not(ferro_skia_apple_ganesh))), allow(dead_code))]
pub(crate) fn registered() -> Vec<Cow<'static, [u8]>> {
    REGISTERED.lock().unwrap_or_else(PoisonError::into_inner).clone()
}
