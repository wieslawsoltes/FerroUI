use super::{Layoutable, MinMax};
use crate::utilities::MathUtilities;
use crate::{Point, Rect, Size, Thickness, Visual};

/// Provides helper methods needed for layout.
pub struct LayoutHelper;

impl LayoutHelper {
    /// Epsilon value used for certain layout calculations.
    /// Based on the value in WPF LayoutDoubleUtil.
    pub const LAYOUT_EPSILON: f64 = 0.00000153;

    /// Calculates a control's size based on its width, height, min-width,
    /// min-height, max-width and max-height.
    pub fn apply_layout_constraints(control: &Layoutable, constraints: Size) -> Size {
        Self::apply_layout_constraints_min_max(MinMax::new(control), constraints)
    }

    pub(crate) fn apply_layout_constraints_min_max(min_max: MinMax, constraints: Size) -> Size {
        Size::new(
            MathUtilities::clamp(constraints.width, min_max.min_width, min_max.max_width),
            MathUtilities::clamp(constraints.height, min_max.min_height, min_max.max_height),
        )
    }

    /// Measures a child with padding and a border, returning the desired
    /// size including them.
    pub fn measure_child_with_border(
        control: Option<&Layoutable>,
        available_size: Size,
        mut padding: Thickness,
        mut border_thickness: Thickness,
    ) -> Size {
        if let Some(scale) = Self::is_parent_layout_rounded(control) {
            padding = Self::round_layout_thickness(padding, scale);
            border_thickness = Self::round_layout_thickness(border_thickness, scale);
        }

        match control {
            Some(control) => {
                control.measure(available_size.deflate(padding + border_thickness));
                control.desired_size().inflate(padding + border_thickness)
            }
            None => Size::default().inflate(padding + border_thickness),
        }
    }

    /// Measures a child with padding, returning the desired size including
    /// the padding.
    pub fn measure_child(control: Option<&Layoutable>, available_size: Size, mut padding: Thickness) -> Size {
        if let Some(scale) = Self::is_parent_layout_rounded(control) {
            padding = Self::round_layout_thickness(padding, scale);
        }

        match control {
            Some(control) => {
                control.measure(available_size.deflate(padding));
                control.desired_size().inflate(padding)
            }
            None => Size::new(padding.left + padding.right, padding.bottom + padding.top),
        }
    }

    /// Arranges a child inside padding and a border.
    pub fn arrange_child_with_border(
        child: Option<&Layoutable>,
        available_size: Size,
        mut padding: Thickness,
        mut border_thickness: Thickness,
    ) -> Size {
        if let Some(scale) = Self::is_parent_layout_rounded(child) {
            padding = Self::round_layout_thickness(padding, scale);
            border_thickness = Self::round_layout_thickness(border_thickness, scale);
        }
        Self::arrange_child_internal(child, available_size, padding + border_thickness)
    }

    /// Arranges a child inside padding.
    pub fn arrange_child(child: Option<&Layoutable>, available_size: Size, mut padding: Thickness) -> Size {
        if let Some(scale) = Self::is_parent_layout_rounded(child) {
            padding = Self::round_layout_thickness(padding, scale);
        }
        Self::arrange_child_internal(child, available_size, padding)
    }

    fn arrange_child_internal(child: Option<&Layoutable>, available_size: Size, padding: Thickness) -> Size {
        if let Some(child) = child {
            child.arrange(Rect::from_size(available_size).deflate_thickness(padding));
        }
        available_size
    }

    /// Returns the layout scale of the child's parent if the parent uses
    /// layout rounding.
    fn is_parent_layout_rounded(child: Option<&Layoutable>) -> Option<f64> {
        let parent = child?.visual_parent()?;
        let parent = parent.downcast_ref::<Layoutable>()?;
        if !parent.use_layout_rounding() {
            return None;
        }
        Some(Self::get_layout_scale(parent))
    }

    /// Invalidates measure for the control and all of its visual descendants.
    pub fn invalidate_self_and_children_measure(control: &Layoutable) {
        fn inner(target: &Visual) {
            if let Some(layoutable) = target.downcast_ref::<Layoutable>() {
                layoutable.invalidate_measure();
            }
            if let Some(children) = target.visual_children_snapshot() {
                for child in children.iter() {
                    inner(child);
                }
            }
        }
        inner(control);
    }

    /// Obtains the layout scale of the given control, based on its layout
    /// root.
    pub fn get_layout_scale(control: &Layoutable) -> f64 {
        control.get_layout_root().map_or(1.0, |root| root.layout_scaling())
    }

    /// Rounds a size to an integer number of device pixels, rounding up.
    pub fn round_layout_size_up(size: Size, dpi_scale: f64) -> Size {
        // If DPI == 1, don't use DPI-aware rounding.
        if dpi_scale == 1.0 {
            Size::new(size.width.ceil(), size.height.ceil())
        } else {
            Size::new(
                (Self::round_to_8_digits(size.width) * dpi_scale).ceil() / dpi_scale,
                (Self::round_to_8_digits(size.height) * dpi_scale).ceil() / dpi_scale,
            )
        }
    }

    /// Rounds a thickness to integer numbers of device pixels.
    pub fn round_layout_thickness(thickness: Thickness, dpi_scale: f64) -> Thickness {
        // If DPI == 1, don't use DPI-aware rounding.
        if dpi_scale == 1.0 {
            Thickness::new(
                thickness.left.round_ties_even(),
                thickness.top.round_ties_even(),
                thickness.right.round_ties_even(),
                thickness.bottom.round_ties_even(),
            )
        } else {
            Thickness::new(
                (thickness.left * dpi_scale).round_ties_even() / dpi_scale,
                (thickness.top * dpi_scale).round_ties_even() / dpi_scale,
                (thickness.right * dpi_scale).round_ties_even() / dpi_scale,
                (thickness.bottom * dpi_scale).round_ties_even() / dpi_scale,
            )
        }
    }

    /// Rounds a point to integer device pixels.
    #[inline]
    pub fn round_layout_point(point: Point, dpi_scale: f64) -> Point {
        // If DPI == 1, don't use DPI-aware rounding.
        if dpi_scale == 1.0 {
            Point::new(point.x.round_ties_even(), point.y.round_ties_even())
        } else {
            Point::new(
                (point.x * dpi_scale).round_ties_even() / dpi_scale,
                (point.y * dpi_scale).round_ties_even() / dpi_scale,
            )
        }
    }

    /// Calculates the value to be used for layout rounding at high DPI by
    /// rounding the value up or down to the nearest pixel.
    pub fn round_layout_value(value: f64, dpi_scale: f64) -> f64 {
        // If DPI == 1, don't use DPI-aware rounding.
        if dpi_scale == 1.0 {
            value.round_ties_even()
        } else {
            (value * dpi_scale).round_ties_even() / dpi_scale
        }
    }

    /// Calculates the value to be used for layout rounding at high DPI by
    /// rounding the value up to the nearest pixel.
    pub fn round_layout_value_up(value: f64, dpi_scale: f64) -> f64 {
        // If DPI == 1, don't use DPI-aware rounding.
        if dpi_scale == 1.0 {
            value.ceil()
        } else {
            (Self::round_to_8_digits(value) * dpi_scale).ceil() / dpi_scale
        }
    }

    #[inline]
    fn round_to_8_digits(value: f64) -> f64 {
        // Round the value (towards zero) to avoid FP errors. This is needed
        // because if `value` has a floating point precision error (e.g.
        // 79.333333333333343) then when it's multiplied by `dpi_scale` and
        // rounded up, it will be rounded up to a value one greater than it
        // should be.
        if !value.is_finite() {
            return value;
        }
        (value * 1e8).trunc() / 1e8
    }

    /// Validates a render/layout scaling value, normalizing values close to
    /// one to exactly one. Panics on invalid values.
    pub fn validate_scaling(scaling: f64) -> f64 {
        if MathUtilities::is_negative_or_non_finite(scaling) || MathUtilities::is_zero(scaling) {
            panic!("Invalid render scaling value {scaling}");
        }
        if MathUtilities::is_one(scaling) {
            // Ensure we've got exactly 1.0 and not an approximation, so that
            // layout hot paths can compare exactly.
            return 1.0;
        }
        scaling
    }
}
