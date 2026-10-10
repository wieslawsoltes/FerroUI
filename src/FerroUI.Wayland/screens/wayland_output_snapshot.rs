//! An output as the UI thread sees it (the port of
//! `WaylandOutputSnapshot.cs`).

use ferroui_base::{PixelPoint, PixelSize};
use std::sync::atomic::{AtomicU64, Ordering};

/// The identity of an output across snapshots. The reference uses a
/// reference to an object nobody else has (`new object()`); a number that is
/// never given twice is the same thing as a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WaylandOutputId(u64);

impl WaylandOutputId {
    /// An identity no other output has.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for WaylandOutputId {
    fn default() -> Self {
        Self::new()
    }
}

/// How the subpixels of an output are laid out (`wl_output.subpixel`). The
/// reference keeps the enumeration of its protocol bindings; the snapshot
/// crosses to the UI thread and to systems without the bindings, so it has
/// one of its own with the values of the protocol.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaylandOutputSubpixel {
    #[default]
    Unknown = 0,
    None = 1,
    HorizontalRgb = 2,
    HorizontalBgr = 3,
    VerticalRgb = 4,
    VerticalBgr = 5,
}

/// The transform of an output (`wl_output.transform`), with the values of the protocol.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaylandOutputTransform {
    #[default]
    Normal = 0,
    Rotated90 = 1,
    Rotated180 = 2,
    Rotated270 = 3,
    Flipped = 4,
    Flipped90 = 5,
    Flipped180 = 6,
    Flipped270 = 7,
}

/// Immutable per-output state published to the UI thread.
#[derive(Clone, Debug, PartialEq)]
pub struct WaylandOutputSnapshot {
    pub id: WaylandOutputId,
    pub name: Option<String>,
    pub description: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub logical_position: PixelPoint,
    pub logical_size: PixelSize,
    pub integer_scale: i32,
    pub refresh_rate_hz: f64,
    pub subpixel: WaylandOutputSubpixel,
    pub transform: WaylandOutputTransform,
    pub physical_size_mm: PixelSize,
}

/// All outputs at one moment.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WaylandOutputsSnapshot {
    pub outputs: Vec<WaylandOutputSnapshot>,
}

impl WaylandOutputsSnapshot {
    pub fn new(outputs: Vec<WaylandOutputSnapshot>) -> Self {
        Self { outputs }
    }
}
