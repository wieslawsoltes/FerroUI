//! Where the `PERF` lines of a scene in a window go, and what scene a window shows.
//!
//! The desktop program prints them and takes the scene from its environment. An application of a
//! device may have neither a standard output that is read nor an environment of its own (an
//! Android application has none), so its host names both: the lines go to the log of the system
//! and the scene comes from the intent that started the activity.

use std::sync::OnceLock;

type Sink = Box<dyn Fn(&str) + Send + Sync>;

static SINK: OnceLock<Sink> = OnceLock::new();
static SCENE: OnceLock<fn() -> Option<String>> = OnceLock::new();

/// Sends the `PERF` lines to `sink` instead of the standard output, and turns the report of the
/// frame rate on as `PERF_REPORT=1` does. The first call counts.
pub fn report_to(sink: impl Fn(&str) + Send + Sync + 'static) {
    let _ = SINK.set(Box::new(sink));
}

/// Asks `source` for the name of the scene instead of the environment (`FRAMES_SCENE`). It is
/// asked when the window or the main view of the application is made and, while it has no answer
/// (the intent of an Android activity cannot be read before the activity is resumed), every few
/// milliseconds after that; the scene is built when it answers. The first call counts.
pub fn scene_from(source: fn() -> Option<String>) {
    let _ = SCENE.set(source);
}

/// A `PERF` line.
pub(crate) fn line(text: &str) {
    match SINK.get() {
        Some(sink) => sink(text),
        None => println!("{text}"),
    }
}

/// Whether `PERF fps N` is reported once a second.
pub(crate) fn reports_frame_rate() -> bool {
    SINK.get().is_some() || std::env::var_os("PERF_REPORT").is_some()
}

/// The name of the scene to show, or `None` while the source of the host has no answer: what
/// the source says, without one `FRAMES_SCENE`, else `text`.
pub(crate) fn scene_name() -> Option<String> {
    match SCENE.get() {
        Some(source) => source(),
        None => Some(std::env::var("FRAMES_SCENE").unwrap_or_else(|_| "text".to_string())),
    }
}
