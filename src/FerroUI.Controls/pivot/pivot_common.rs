//! The shared definitions of WinUI's Pivot (`dxaml/xcp/components/pivot/inc/PivotCommon.h` of
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

/// The direction of a change of the selected item (WinUI's `PivotAnimationDirection`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PivotAnimationDirection {
    #[default]
    Center,
    Left,
    Right,
    /// No animation: the visuals are put in place at once.
    Reset,
}

/// WinUI's `PositiveMod`: `x` modulo `n` in `0..n` (0 for `n == 0`).
pub(crate) fn positive_mod(x: i32, n: i32) -> i32 {
    if n == 0 {
        0
    } else {
        (x % n + n) % n
    }
}

/// WinUI's `GetPivotPanelMultiplier`: the panel of a wrapping pivot is this many sections wide.
pub(crate) const PIVOT_PANEL_MULTIPLIER: u32 = 1000;

/// WinUI's `PivotCurveGenerator::GetHeaderShiftOffset` (`PivotCurveGenerator.cpp`): how far the
/// static headers shift when the header with `a` of space on its left, a width of `b` and `c` of
/// space on its right is selected, in a viewport of width `v`.
pub(crate) fn header_shift_offset(a: f64, b: f64, c: f64, v: f64) -> f64 {
    if a + c == 0.0 {
        0.0
    } else {
        (a - a * (v - b.min(v)) / (a + c)).max(0.0)
    }
}

/// WinUI's `AreClose` with its default threshold.
pub(crate) fn are_close(a: f64, b: f64) -> bool {
    are_close_within(a, b, 0.1)
}

/// WinUI's `AreClose`.
pub(crate) fn are_close_within(a: f64, b: f64, threshold: f64) -> bool {
    (a - b).abs() < threshold
}
