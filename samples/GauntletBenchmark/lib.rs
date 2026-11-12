//! The gauntlet and the still wall of the Cranpose benchmark, as a library: the program
//! `perf-compare-ferroui` (`program.rs`) opens them in a desktop window, and the frame gate
//! (`tests/frame_gate.rs`) shows them in a headless window and times their frames.
//!
//! Both screens drive themselves: each requests an animation frame of its top level when it is
//! attached, and every callback advances the frame index ([`Clock`]) once and requests the next
//! one, so that the frames follow the index and never wall time.

mod app;
mod gauntlet;
mod perf_data;
mod wall;
#[cfg(all(windows, feature = "renderer-measurement"))]
pub mod measurement;

pub use app::{App, DeviceRoboto, Launch};
pub use gauntlet::{Clock, GauntletView};
pub use wall::WallView;
