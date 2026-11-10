//! The frame benchmarks of the local performance gate (`scripts/perf/gate.py`): scenes that each do
//! one kind of per-frame work of a heavy desktop screen (text that changes every frame, a scrolling
//! list of shadowed cards, panels moved by their render transforms, a layout under a changing width,
//! a still wall of tiles, tiles that move) and the start of each (building the scene, showing it);
//! and two scenes whose start alone is measured, phase by phase to the first frame: a wall of small
//! tiles at its densest, and a form of templated controls ([`STARTUP_SCENES`]).
//!
//! They are FerroUI's own regression benchmarks, kept apart from the ports of other frameworks'
//! benchmarks (`samples/GauntletBenchmark` follows the Cranpose benchmark and is synced with it):
//! a finding of a cross-framework comparison gets a scene or a micro-benchmark here or in
//! `tests/FerroUI.Benchmarks` (category `trees`), so that the gate guards it.
//!
//! - `tests/frames.rs`: every scene in a headless window over the compositor, rendered into a raster
//!   Skia framebuffer, timed frame by frame; `GATE {...}` lines for the gate.
//! - `program.rs` (`frame-benchmarks`): a scene in a desktop window (`FRAMES_SCENE`), printing
//!   `PERF first_frame` and, with `PERF_REPORT=1`, `PERF fps N` once a second, for the gate's
//!   window suite (the GPU backends).
//!
//! Every frame advances a frame index ([`Clock`]); the scenes follow it, never wall time.

mod clock;
mod fonts;
mod host;
pub mod scenes;

pub use clock::Clock;
pub use fonts::Roboto;
pub use host::FrameHost;
pub use scenes::{Scene, SCENES, STARTUP_SCENES};
