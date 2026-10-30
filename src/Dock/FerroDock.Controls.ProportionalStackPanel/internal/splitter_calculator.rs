// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::ProportionalStackPanelSplitter;
use ferroui_base::FerroObject;
use ferroui_controls::Controls;
pub(crate) struct SplitterCalculator;
impl SplitterCalculator {
    pub(crate) fn get_total_splitter_thickness(
        children: &Controls,
        get_collapsed: impl Fn(&FerroObject) -> bool,
    ) -> f64 {
        let mut total = 0.0;
        let mut previous_was_collapsed = false;
        let mut index = 0;
        while index < children.count() {
            let child = children.get(index);
            if let Some(splitter) = ProportionalStackPanelSplitter::is_splitter(Some(&child)) {
                if Self::should_skip_splitter(
                    index,
                    children,
                    previous_was_collapsed,
                    &get_collapsed,
                ) {
                    index += 1;
                    continue;
                }
                total += splitter.thickness();
            } else {
                previous_was_collapsed = get_collapsed(&child);
            }
            index += 1;
        }
        if total.is_nan() {
            0.0
        } else {
            total
        }
    }
    pub(crate) fn should_skip_splitter(
        index: usize,
        children: &Controls,
        previous_collapsed: bool,
        get_collapsed: impl Fn(&FerroObject) -> bool,
    ) -> bool {
        if previous_collapsed {
            return true;
        }
        index + 1 < children.count() && get_collapsed(&children.get(index + 1))
    }
}
