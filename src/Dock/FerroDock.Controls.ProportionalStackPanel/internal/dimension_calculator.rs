// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferroui_base::layout::Orientation;
use ferroui_controls::Control;
pub(crate) struct DimensionCalculator;
impl DimensionCalculator {
    pub(crate) fn calculate_dimension(dimension: f64, proportion: f64, sum: &mut f64) -> f64 {
        let child = dimension * proportion;
        let floor = child.floor();
        *sum += child - floor;
        let round = if sum.abs() < 1e16 {
            (*sum * 10.0).round_ties_even() / 10.0
        } else {
            *sum
        };
        let clamp = math_clamp(sum.floor(), 1.0, f64::MAX);
        if round - clamp >= 0.0 {
            *sum -= sum.round_ties_even();
            return max(0.0, floor + 1.0);
        }
        max(0.0, floor)
    }
    pub(crate) fn calculate_dimension_with_constraints(
        control: &Control,
        orientation: Orientation,
        dimension: f64,
        proportion: f64,
        sum: &mut f64,
    ) -> f64 {
        let mut calculated = Self::calculate_dimension(dimension, proportion, sum);
        let (minimum, maximum) = ProportionUtils::get_size_constraints(control, orientation);
        if !minimum.is_nan() && calculated < minimum {
            calculated = minimum;
        }
        if !maximum.is_nan() && maximum != f64::INFINITY && calculated > maximum {
            calculated = maximum;
        }
        calculated
    }
}
