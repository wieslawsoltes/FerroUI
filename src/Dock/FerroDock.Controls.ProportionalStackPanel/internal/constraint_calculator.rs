// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferroui_base::layout::Orientation;
use ferroui_controls::Control;
pub(crate) struct ConstraintCalculator {
    orientation: Orientation,
    available_size: f64,
    minimum_proportion_size: f64,
}
impl ConstraintCalculator {
    pub(crate) fn new(
        orientation: Orientation,
        available_size: f64,
        minimum_proportion_size: f64,
    ) -> Self {
        Self {
            orientation,
            available_size,
            minimum_proportion_size,
        }
    }
    pub(crate) fn get_constraints(&self, control: &Control) -> (f64, f64) {
        let (minimum, maximum) = ProportionUtils::get_size_constraints(control, self.orientation);
        let mut min_proportion = ProportionUtils::dimension_to_proportion(
            self.minimum_proportion_size,
            self.available_size,
        );
        let mut max_proportion = f64::INFINITY;
        if !minimum.is_nan() && minimum > 0.0 {
            min_proportion = max(
                min_proportion,
                ProportionUtils::dimension_to_proportion(minimum, self.available_size),
            );
        }
        if !maximum.is_nan() && maximum != f64::INFINITY {
            max_proportion = ProportionUtils::dimension_to_proportion(maximum, self.available_size);
        }
        (min_proportion, max_proportion)
    }
}
