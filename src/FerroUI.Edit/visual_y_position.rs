/// Vertical reference point within a formatted editor row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualYPosition {
    LineTop,
    TextTop,
    LineBottom,
    LineMiddle,
    TextBottom,
    TextMiddle,
    Baseline,
}
