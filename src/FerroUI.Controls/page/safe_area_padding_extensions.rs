use ferroui_base::Thickness;

/// Helpers combining a padding with the safe-area padding of a page.
pub(crate) trait SafeAreaPaddingExtensions {
    /// The padding enlarged, edge by edge, to at least the safe-area
    /// padding.
    fn apply_safe_area_padding(self, safe_area_padding: Thickness) -> Thickness;

    /// The part of the safe-area padding that the padding does not cover.
    fn get_remaining_safe_area_padding(self, safe_area_padding: Thickness) -> Thickness;
}

impl SafeAreaPaddingExtensions for Thickness {
    fn apply_safe_area_padding(self, safe_area_padding: Thickness) -> Thickness {
        Thickness::new(
            max(self.left, safe_area_padding.left),
            max(self.top, safe_area_padding.top),
            max(self.right, safe_area_padding.right),
            max(self.bottom, safe_area_padding.bottom),
        )
    }

    fn get_remaining_safe_area_padding(self, safe_area_padding: Thickness) -> Thickness {
        Thickness::new(
            max(0.0, safe_area_padding.left - self.left),
            max(0.0, safe_area_padding.top - self.top),
            max(0.0, safe_area_padding.right - self.right),
            max(0.0, safe_area_padding.bottom - self.bottom),
        )
    }
}

/// The larger of two values with the NaN handling of the reference: NaN if
/// either value is NaN.
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}
