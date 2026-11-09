//! Synchronous wall-clock injection using native Rust values and ownership.
use std::{
    rc::Rc,
    time::{Duration, SystemTime},
};

pub type EditorClock = Rc<dyn Fn() -> SystemTime>;

pub(crate) fn default_clock() -> EditorClock {
    Rc::new(SystemTime::now)
}

/// An unset previous event is not a tap/edit. Backwards clocks keep the source's
/// signed elapsed-time comparison for a nonnegative window.
pub(crate) fn within_window(
    now: SystemTime,
    previous: Option<SystemTime>,
    window: Duration,
) -> bool {
    previous.is_some_and(|previous| {
        now.duration_since(previous)
            .map_or(true, |elapsed| elapsed < window)
    })
}
