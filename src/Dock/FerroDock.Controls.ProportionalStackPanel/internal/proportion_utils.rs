// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::ProportionalStackPanelSplitter;
use ferroui_base::layout::Orientation;
use ferroui_base::{Ref, Size};
use ferroui_controls::{Control, Controls};
pub(crate) struct ProportionUtils;
impl ProportionUtils {
    pub(crate) fn clamp(value: f64, min: f64, max: f64) -> f64 {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }
    pub(crate) fn is_valid_proportion(value: f64) -> bool {
        !value.is_nan() && value >= 0.0
    }
    pub(crate) fn dimension_to_proportion(value: f64, available: f64) -> f64 {
        if available > 0.0 {
            value / available
        } else {
            0.0
        }
    }
    pub(crate) fn get_relevant_dimension(size: Size, orientation: Orientation) -> f64 {
        if orientation == Orientation::Horizontal {
            size.width
        } else {
            size.height
        }
    }
    pub(crate) fn get_size_constraints(control: &Control, orientation: Orientation) -> (f64, f64) {
        if orientation == Orientation::Horizontal {
            (control.min_width(), control.max_width())
        } else {
            (control.min_height(), control.max_height())
        }
    }
    pub(crate) fn get_non_splitter_children(
        children: &Controls,
    ) -> impl Iterator<Item = Ref<Control>> + 'static {
        NonSplitterChildren {
            children: children.clone(),
            storage: children.snapshot(),
            index: 0,
        }
    }
}
// Native representations of System.Math comparisons. Rust f64::max/min discard
// NaNs, unlike the source; signed-zero selection is also retained explicitly.
pub(crate) fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a == 0.0 && b == 0.0 {
        f64::from_bits(a.to_bits() & b.to_bits())
    } else {
        a.max(b)
    }
}
pub(crate) fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a == 0.0 && b == 0.0 {
        f64::from_bits(a.to_bits() | b.to_bits())
    } else {
        a.min(b)
    }
}
pub(crate) fn math_clamp(value: f64, minimum: f64, maximum: f64) -> f64 {
    if minimum > maximum {
        panic!("minimum exceeds maximum");
    }
    ProportionUtils::clamp(value, minimum, maximum)
}

// The shared UI list uses copy-on-write storage rather than exposing a version.
// Retaining that storage and checking its identity preserves source enumeration
// invalidation without another notification subscriber or collection engine.
struct NonSplitterChildren {
    children: Controls,
    storage: std::rc::Rc<Vec<Ref<Control>>>,
    index: usize,
}
impl Iterator for NonSplitterChildren {
    type Item = Ref<Control>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            assert!(
                std::rc::Rc::ptr_eq(&self.storage, &self.children.snapshot()),
                "Collection was modified; enumeration operation may not execute."
            );
            let child = self.storage.get(self.index)?.clone();
            self.index += 1;
            if ProportionalStackPanelSplitter::is_splitter(Some(&child)).is_none() {
                return Some(child);
            }
        }
    }
}
