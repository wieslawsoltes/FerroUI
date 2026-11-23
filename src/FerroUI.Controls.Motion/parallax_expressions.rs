//! The expressions of `ParallaxView.cpp` of WinUI, as text. `source` is the manipulation property
//! set of the scroll viewer: `source.Translation` is the negated offset and `source.Scale` the
//! zoom. The position X of the formulas of the source is `-source.Translation.X`.

use ferroui_base::layout::Orientation;

/// The names an axis reads and animates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Axis {
    pub(crate) orientation: Orientation,
    pub(crate) index: usize,
    /// The channel of the visual that moves.
    pub(crate) target: &'static str,
    /// The offset in the property set of the source.
    pub(crate) source: &'static str,
}

pub(crate) const HORIZONTAL: Axis = Axis {
    orientation: Orientation::Horizontal,
    index: 0,
    target: "Translation.X",
    source: "source.Translation.X",
};

pub(crate) const VERTICAL: Axis = Axis {
    orientation: Orientation::Vertical,
    index: 1,
    target: "Translation.Y",
    source: "source.Translation.Y",
};

/// The zoom in the property set of the source, uniform on both axes.
const SCALE: &str = "source.Scale.X";

/// The expression of the source start offset.
pub(crate) fn start_offset(is_relative: bool, is_target_in_source: bool, start_offset: f32) -> String {
    match (is_relative, is_target_in_source) {
        (true, true) => format!("(parallaxViewOffset + startOffset) * {SCALE} - viewportSize - maxUnderpanOffset"),
        (true, false) => format!("startOffset * {SCALE} - maxUnderpanOffset"),
        (false, _) if start_offset > 0.0 => format!("startOffset * {SCALE}"),
        (false, _) => "startOffset".to_owned(),
    }
}

/// The expression of the source end offset.
pub(crate) fn end_offset(is_relative: bool, is_target_in_source: bool, end_offset: f32, content: f32, viewport: f32) -> String {
    if is_relative {
        return if is_target_in_source {
            format!("(parallaxViewOffset + parallaxViewSize + endOffset) * {SCALE} + maxOverpanOffset")
        } else {
            format!("Max(0.0, (contentSize + endOffset) * {SCALE} - viewportSize) + maxOverpanOffset")
        };
    }
    if content > viewport {
        if end_offset <= content - viewport {
            format!("Max(0.0, endOffset * {SCALE})")
        } else {
            format!("Max(0.0, (contentSize - viewportSize) * {SCALE}) + endOffset - contentSize + viewportSize")
        }
    } else if end_offset <= 0.0 {
        format!("Max(0.0, (contentSize + endOffset) * {SCALE} - viewportSize)")
    } else {
        format!("Max(0.0, contentSize * {SCALE} - viewportSize) + endOffset")
    }
}

/// The expression of the translation of the child, over the expressions of the two offsets. The
/// source keeps those two in animated variables; here they are part of the one expression, so a
/// change of the scrolling evaluates one animation.
pub(crate) fn parallax(axis: &Axis, is_clamped: bool, shift: f32, start: &str, end: &str) -> String {
    let (x, start, end) = (format!("-{}", axis.source), format!("({start})"), format!("({end})"));
    match (is_clamped, shift > 0.0) {
        (true, true) => format!(
            "({x} <= {start}) ? 0.0 : (({x} < {end}) ? (-Min(maxRatio, (shift / ({end} - {start}))) * ({x} - {start})) : -Min(maxRatio * Max(0.0, {end} - {start}), shift))"
        ),
        (true, false) => format!(
            "({x} <= {start}) ? -Min(maxRatio * Max(0.0, {end} - {start}), -shift) : (({x} < {end}) ? (Min(maxRatio, (shift / ({start} - {end}))) * ({x} - {end})) : 0.0)"
        ),
        (false, true) => format!("({start} == {end}) ? 0.0 : -Min(maxRatio, shift / ({end} - {start})) * ({x} - {start})"),
        (false, false) => format!("({start} == {end}) ? 0.0 : Min(maxRatio, shift / ({start} - {end})) * ({x} - {end})"),
    }
}
