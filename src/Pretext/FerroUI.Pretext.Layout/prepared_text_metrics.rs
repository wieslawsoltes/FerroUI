use ferroui_pretext::{LayoutCursor, PreparedTextWithSegments, PretextLayout};

use crate::{LayoutError, WrapMetrics};

/// Sizing helpers that reuse preparation without constructing line strings.
pub struct PreparedTextMetrics;

impl PreparedTextMetrics {
    pub fn measure_max_line_width(prepared: &PreparedTextWithSegments) -> f64 {
        PretextLayout::measure_natural_width(prepared)
    }

    pub fn collect_wrap_metrics(
        prepared: &PreparedTextWithSegments,
        max_width: f64,
        line_height: f64,
    ) -> WrapMetrics {
        let stats = PretextLayout::measure_line_stats(prepared, max_width);
        WrapMetrics {
            line_count: stats.line_count,
            height: stats.line_count as f64 * line_height,
            max_line_width: stats.max_line_width,
        }
    }

    /// Find the smallest integral width with no more lines than `max_width`.
    /// Width is bounded to the exactly representable integer range of `f64` so
    /// binary search makes strict progress, including very large user inputs.
    pub fn find_tight_wrap_metrics(
        prepared: &PreparedTextWithSegments,
        max_width: f64,
        line_height: f64,
    ) -> Result<WrapMetrics, LayoutError> {
        if !max_width.is_finite() || max_width <= 0.0 || max_width > (1u64 << 53) as f64 {
            return Err(LayoutError::InvalidMaximumWidth);
        }
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(LayoutError::InvalidLineHeight);
        }
        let initial = Self::collect_wrap_metrics(prepared, max_width, line_height);
        let mut lo = 1.0;
        let mut hi = max_width.ceil().max(1.0);
        while lo < hi {
            let mid = lo + ((hi - lo) * 0.5).floor();
            let result = PretextLayout::layout(prepared, mid, line_height);
            if result.line_count <= initial.line_count {
                hi = mid;
            } else {
                lo = mid + 1.0;
            }
        }
        Ok(Self::collect_wrap_metrics(prepared, lo, line_height))
    }

    pub fn is_end(prepared: &PreparedTextWithSegments, cursor: LayoutCursor) -> bool {
        cursor.segment_index >= prepared.segments().len()
    }
}
