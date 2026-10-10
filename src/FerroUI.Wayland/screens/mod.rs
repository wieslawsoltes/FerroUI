//! The screens of the Wayland backend (the directory `Screens` of the
//! reference): snapshots of the outputs the worker tracks, and the screens
//! the UI thread makes of them.

pub mod i_wayland_outputs_sink;
pub mod snapshot_screens_impl;
pub mod wayland_output_snapshot;

pub use i_wayland_outputs_sink::{IWaylandOutputsSink, WaylandOutputsSinkProxy};
pub use snapshot_screens_impl::{SnapshotScreensImpl, WaylandScreenHandle, WaylandSnapshotScreen};
pub use wayland_output_snapshot::{
    WaylandOutputId, WaylandOutputSnapshot, WaylandOutputSubpixel, WaylandOutputTransform, WaylandOutputsSnapshot,
};
