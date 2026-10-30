// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use crate::ProportionalStackPanel as Panel;
use ferroui_base::layout::Orientation;
use ferroui_base::{Ref, Size};
use ferroui_controls::{Control, Controls};
struct ChildInfo {
    control: Ref<Control>,
    is_collapsed: bool,
    current_proportion: f64,
    target_proportion: f64,
}
impl ChildInfo {
    fn new(control: Ref<Control>) -> Self {
        Self {
            is_collapsed: Panel::get_is_collapsed(&control),
            current_proportion: Panel::get_proportion(&control),
            control,
            target_proportion: f64::NAN,
        }
    }
}
pub(crate) struct ProportionManager {
    child_infos: Vec<ChildInfo>,
    constraint_handler: ProportionConstraintHandler,
}
impl ProportionManager {
    pub(crate) fn new(
        children: &Controls,
        size: Size,
        splitter: f64,
        orientation: Orientation,
    ) -> Self {
        let available = max(
            1.0,
            ProportionUtils::get_relevant_dimension(size, orientation) - splitter,
        );
        Self {
            child_infos: ProportionUtils::get_non_splitter_children(children)
                .map(ChildInfo::new)
                .collect(),
            constraint_handler: ProportionConstraintHandler::new(orientation, available),
        }
    }
    pub(crate) fn assign_proportions(&mut self) {
        self.handle_collapsed_children();
        self.assign_unassigned_proportions();
        self.normalize_proportions();
        self.apply_proportions();
    }
    fn handle_collapsed_children(&mut self) {
        for info in &mut self.child_infos {
            if info.is_collapsed {
                if ProportionUtils::is_valid_proportion(info.current_proportion)
                    && info.current_proportion > 0.0
                {
                    Panel::set_collapsed_proportion(&info.control, info.current_proportion);
                }
                info.target_proportion = 0.0;
            } else {
                let stored = Panel::get_collapsed_proportion(&info.control);
                info.target_proportion = if ProportionUtils::is_valid_proportion(stored) {
                    stored
                } else {
                    info.current_proportion
                };
            }
        }
    }
    fn assign_unassigned_proportions(&mut self) {
        let count = self
            .child_infos
            .iter()
            .filter(|info| {
                !info.is_collapsed && !ProportionUtils::is_valid_proportion(info.target_proportion)
            })
            .count();
        if count == 0 {
            return;
        }
        let mut assigned_total = 0.0;
        for info in &self.child_infos {
            if ProportionUtils::is_valid_proportion(info.target_proportion) {
                assigned_total += info.target_proportion;
            }
        }
        let per_child = max(0.0, 1.0 - assigned_total) / count as f64;
        for info in &mut self.child_infos {
            if !info.is_collapsed && !ProportionUtils::is_valid_proportion(info.target_proportion) {
                info.target_proportion = per_child;
            }
        }
    }
    fn normalize_proportions(&mut self) {
        let mut count = 0;
        let mut total = 0.0;
        for info in &self.child_infos {
            if !info.is_collapsed {
                count += 1;
                total += info.target_proportion;
            }
        }
        if count == 0 || (total - 1.0).abs() < 1e-10 || total <= 0.0 {
            return;
        }
        let scale = 1.0 / total;
        for info in &mut self.child_infos {
            if !info.is_collapsed {
                info.target_proportion *= scale;
            }
        }
    }
    fn apply_proportions(&self) {
        let has_collapsed = self.child_infos.iter().any(|info| info.is_collapsed);
        for info in &self.child_infos {
            let clamped = self
                .constraint_handler
                .clamp_proportion(&info.control, info.target_proportion);
            Panel::set_proportion(&info.control, clamped);
            if !info.is_collapsed && !has_collapsed {
                Panel::set_collapsed_proportion(&info.control, clamped);
            }
        }
    }
}
