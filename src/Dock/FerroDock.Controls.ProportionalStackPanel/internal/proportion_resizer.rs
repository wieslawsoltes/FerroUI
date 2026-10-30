// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use crate::ProportionalStackPanel;
use ferroui_base::layout::Orientation;
use ferroui_controls::Control;
pub(crate) struct ProportionResizer<'a> {
    panel: &'a ProportionalStackPanel,
    target: &'a Control,
    neighbor: &'a Control,
    drag_delta: f64,
    minimum_size: f64,
    available_size: f64,
}
impl<'a> ProportionResizer<'a> {
    pub(crate) fn new(
        panel: &'a ProportionalStackPanel,
        target: &'a Control,
        neighbor: &'a Control,
        drag_delta: f64,
        minimum_size: f64,
    ) -> Self {
        Self {
            panel,
            target,
            neighbor,
            drag_delta,
            minimum_size,
            available_size: if panel.orientation() == Orientation::Vertical {
                panel.bounds().height
            } else {
                panel.bounds().width
            },
        }
    }
    pub(crate) fn apply_resize(&self) {
        let target = ProportionalStackPanel::get_proportion(self.target);
        let neighbor = ProportionalStackPanel::get_proportion(self.neighbor);
        let mut delta =
            ProportionUtils::dimension_to_proportion(self.drag_delta, self.available_size);
        if target + delta < 0.0 {
            delta = -target;
        }
        if neighbor - delta < 0.0 {
            delta = neighbor;
        }
        let mut target = target + delta;
        let mut neighbor = neighbor - delta;
        let calculator = ConstraintCalculator::new(
            self.panel.orientation(),
            self.available_size,
            self.minimum_size,
        );
        let target_constraints = calculator.get_constraints(self.target);
        let neighbor_constraints = calculator.get_constraints(self.neighbor);
        Self::apply_constraint_pair(
            &mut target,
            &mut neighbor,
            target_constraints,
            neighbor_constraints,
        );
        Self::apply_constraint_pair(
            &mut neighbor,
            &mut target,
            neighbor_constraints,
            target_constraints,
        );
        ProportionalStackPanel::set_proportion(self.target, max(0.0, target));
        ProportionalStackPanel::set_proportion(self.neighbor, max(0.0, neighbor));
    }
    fn apply_constraint_pair(primary: &mut f64, secondary: &mut f64, p: (f64, f64), s: (f64, f64)) {
        if *primary < p.0 {
            let deficit = p.0 - *primary;
            *primary = p.0;
            *secondary = max(s.0, *secondary - deficit);
        } else if *primary > p.1 {
            let excess = *primary - p.1;
            *primary = p.1;
            *secondary = min(s.1, *secondary + excess);
        }
    }
}
