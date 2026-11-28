use crate::Vector;

/// Original backend wheel data, before normalization to framework wheel units.
///
/// Backends and synthesized events may omit this data. Normal scrolling
/// controls should continue to use the normalized event `delta()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlatformWheelDelta {
    /// Original AppKit axes: pixels for precise input, lines otherwise.
    MacOS { delta: Vector, is_precise: bool },
    /// Original signed Win32 wheel-message axes, in WHEEL_DELTA units.
    /// Horizontal signs retain the message value for both mouse and pointer
    /// protocols, independently of their normalized framework direction.
    Win32 { delta: Vector },
}
