//! The count that experiment #821 asks for before anything is built: of the drawings the
//! render thread replays in a frame, how many have not changed for many frames, and what share
//! of the replay they are (docs/porting/desktop-performance.md, #821). A diagnostic, not from
//! the reference.
//!
//! A frame replays every drawing that meets its dirty area, also the ones that did not change:
//! the drawing items of a composition visual (section 9.42) and the own drawing of a draw list
//! visual. A surface kept on the GPU for content that rests would skip their replay; it pays
//! only when such drawings are a large share of what a frame replays. This module counts it.
//!
//! It is on in a build with the feature `perf-counters` when the environment sets
//! `FERROUI_UNCHANGED_DRAWINGS`: `1` counts a drawing as settled when nothing of it changed for
//! [`DEFAULT_SETTLED_FRAMES`] frames, a larger number sets those frames. Every
//! [`REPORT_FRAMES`] frames drawn, the render thread prints to the standard error, a frame:
//! the drawings replayed and those of them that are settled, the bytes of their recorded
//! operations (a measure of their commands), and the time the render thread spent replaying
//! them into the drawing context of the frame. With a pipelined Skia layer that is the
//! recording of the picture the bands play; the time of the bands themselves is in the report
//! of the bands (`FERROUI_BAND_TIMES=1`), which the share of the commands carries over to.
//!
//! What counts as a change of a drawing: another drawing, the same drawing drawn again, a
//! resource of it that changed (a brush, a geometry), and for a drawing item also another place
//! in its holder or another clip. A change of what is above the drawing (a transform, an
//! opacity, a clip of a composition visual) does not reset it: a kept surface would be drawn
//! under those as the drawing is.

use crate::diagnostics::perf_counters;
use std::cell::RefCell;
use std::time::{Duration, Instant};

/// The frames without a change after which a drawing is settled, by default: a second at 60
/// frames a second.
pub(crate) const DEFAULT_SETTLED_FRAMES: u64 = 60;

/// The frames of a report.
pub(crate) const REPORT_FRAMES: u64 = 300;

/// What was replayed over the frames of a report.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub(crate) struct Replayed {
    pub drawings: u64,
    pub bytes: u64,
    pub time: Duration,
}

impl Replayed {
    fn add(&mut self, bytes: usize, time: Duration) {
        self.drawings += 1;
        self.bytes += bytes as u64;
        self.time += time;
    }
}

/// The counts of the frames of a report.
#[derive(Default)]
pub(crate) struct Counts {
    /// The number of the frame being drawn: the frames drawn since the counting began, from 1.
    frame: u64,
    /// The frames of the report so far.
    frames: u64,
    /// Every drawing replayed, and the settled ones among them.
    pub all: Replayed,
    pub settled: Replayed,
}

struct State {
    settled_frames: u64,
    counts: Counts,
}

impl State {
    fn from_environment() -> Option<Self> {
        if !perf_counters::ENABLED {
            return None;
        }
        let value = std::env::var("FERROUI_UNCHANGED_DRAWINGS").ok()?;
        let settled_frames = parse_settled_frames(&value)?;
        Some(State { settled_frames, counts: Counts { frame: 1, ..Counts::default() } })
    }
}

/// The frames after which a drawing is settled, from the value of the variable: `1` is the
/// default, a larger number is the frames; anything else turns the count off.
pub(crate) fn parse_settled_frames(value: &str) -> Option<u64> {
    match value.trim().parse::<u64>().ok()? {
        0 => None,
        1 => Some(DEFAULT_SETTLED_FRAMES),
        frames => Some(frames),
    }
}

crate::ferro_thread_local! {
    /// The count of the render thread, when it is on.
    static STATE: RefCell<Option<State>> = RefCell::new(State::from_environment());
}

/// The number of the frame being drawn, which a drawing notes when it changes; 0 when the
/// count is off (and always in a build without `perf-counters`).
pub(crate) fn frame() -> u64 {
    if !perf_counters::ENABLED {
        return 0;
    }
    STATE.with(|state| state.borrow().as_ref().map_or(0, |state| state.counts.frame))
}

/// Replays a drawing with `replay`, counting it when the count is on: `changed` is the frame of
/// its last change ([`frame`]), `bytes` the size of its recorded operations.
pub(crate) fn replay<R>(changed: u64, bytes: impl FnOnce() -> usize, replay: impl FnOnce() -> R) -> R {
    if !perf_counters::ENABLED || !STATE.with(|state| state.borrow().is_some()) {
        return replay();
    }
    let started = Instant::now();
    let result = replay();
    let time = started.elapsed();
    let bytes = bytes();
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            let counts = &mut state.counts;
            counts.all.add(bytes, time);
            if counts.frame.saturating_sub(changed) >= state.settled_frames {
                counts.settled.add(bytes, time);
            }
        }
    });
    result
}

/// A frame was drawn: the next drawings belong to the next frame, and every [`REPORT_FRAMES`]
/// frames the counts are printed and begin again.
pub(crate) fn frame_drawn() {
    if !perf_counters::ENABLED {
        return;
    }
    let report = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        let counts = &mut state.counts;
        counts.frame += 1;
        counts.frames += 1;
        if counts.frames < REPORT_FRAMES {
            return None;
        }
        let report = format_report(counts, state.settled_frames);
        let frame = counts.frame;
        *counts = Counts { frame, ..Counts::default() };
        Some(report)
    });
    if let Some(report) = report {
        eprintln!("{report}");
    }
}

/// The line of a report, a frame.
pub(crate) fn format_report(counts: &Counts, settled_frames: u64) -> String {
    let frames = counts.frames.max(1) as f64;
    let share = |part: f64, whole: f64| if whole > 0.0 { 100.0 * part / whole } else { 0.0 };
    let (all, settled) = (counts.all, counts.settled);
    format!(
        "unchanged drawings: {} frames, a frame: {:.1} drawings replayed, {:.1} settled \
         ({settled_frames} frames without a change; {:.1} %); \
         bytes of operations {:.0}, settled {:.0} ({:.1} %); \
         replay {:.3} ms, settled {:.3} ms ({:.1} %)",
        counts.frames,
        all.drawings as f64 / frames,
        settled.drawings as f64 / frames,
        share(settled.drawings as f64, all.drawings as f64),
        all.bytes as f64 / frames,
        settled.bytes as f64 / frames,
        share(settled.bytes as f64, all.bytes as f64),
        all.time.as_secs_f64() * 1000.0 / frames,
        settled.time.as_secs_f64() * 1000.0 / frames,
        share(settled.time.as_secs_f64(), all.time.as_secs_f64()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_variable_turns_the_count_on_with_the_frames_of_a_settled_drawing() {
        assert_eq!(parse_settled_frames("1"), Some(DEFAULT_SETTLED_FRAMES));
        assert_eq!(parse_settled_frames(" 120 "), Some(120));
        assert_eq!(parse_settled_frames("0"), None);
        assert_eq!(parse_settled_frames("on"), None);
    }

    #[test]
    fn a_report_gives_the_share_of_the_settled_drawings_a_frame() {
        let counts = Counts {
            frame: 301,
            frames: 2,
            all: Replayed { drawings: 10, bytes: 1000, time: Duration::from_millis(4) },
            settled: Replayed { drawings: 4, bytes: 250, time: Duration::from_millis(1) },
        };
        let line = format_report(&counts, 60);
        assert!(line.contains("5.0 drawings replayed, 2.0 settled"), "{line}");
        assert!(line.contains("(60 frames without a change; 40.0 %)"), "{line}");
        assert!(line.contains("bytes of operations 500, settled 125 (25.0 %)"), "{line}");
        assert!(line.contains("replay 2.000 ms, settled 0.500 ms (25.0 %)"), "{line}");
    }

    #[test]
    fn without_the_count_a_drawing_is_replayed_and_nothing_is_noted() {
        // The test process does not set the variable.
        if std::env::var_os("FERROUI_UNCHANGED_DRAWINGS").is_some() {
            return;
        }
        assert_eq!(frame(), 0);
        assert_eq!(replay(0, || unreachable!("no size is asked"), || 7), 7);
        frame_drawn();
    }
}
