//! A chain of nested decorators, one child each, two hundred levels deep: what a level of the tree
//! costs in the measure and the arrange of a layoutable. The levels are in turn a border with a
//! border thickness and a decorator with a padding; each has a margin, the alignments change from
//! level to level, and every third level has a minimum and a maximum size, so the pass goes
//! through each branch of the size and alignment arithmetic of a layoutable (section 9.9 of the
//! performance notes: the copies of the size, the margin and the alignments).
//!
//! - `remeasure_rearrange`: every level is marked as not measured and not arranged, and the
//!   layout manager runs a pass;
//! - `remeasure_rearrange_unrounded`: the same without layout rounding (the chain inherits it
//!   from its outermost level), which leaves out the rounding of each size and each margin;
//! - `leaf_resized`: the width of the innermost element changes, and the layout manager runs a
//!   pass. The desired size of each level changes in turn, so each level is invalidated by its
//!   child, taken from the queue of the layout manager and measured after a walk of its ancestors:
//!   the cost of a change of size that reaches the root from the bottom of a deep tree.

use super::fixture::{lay_out_all, leaf, show};
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{Border, Control, Decorator};
use std::cell::Cell;

/// The levels above the innermost element. A level adds four to each axis (its margin and its
/// border or padding), so the chain fits the 1280 x 820 of the root.
const DEPTH: usize = 200;

const HORIZONTAL: [HorizontalAlignment; 4] =
    [HorizontalAlignment::Stretch, HorizontalAlignment::Left, HorizontalAlignment::Center, HorizontalAlignment::Right];

const VERTICAL: [VerticalAlignment; 4] =
    [VerticalAlignment::Stretch, VerticalAlignment::Top, VerticalAlignment::Center, VerticalAlignment::Bottom];

pub struct NestedDepth {
    fixture: Fixture,
    /// The elements of the chain, the innermost first.
    controls: Vec<Ref<Control>>,
    innermost: Ref<Border>,
    flip: Cell<bool>,
}

impl NestedDepth {
    fn new(depth: usize, layout_rounding: bool) -> Self {
        let fixture = Fixture::new();
        let innermost = leaf(8.0, 8.0, &fixture.brushes);
        let mut child = innermost.clone().upcast::<Control>();
        let mut controls = vec![child.clone()];
        for level in 0..depth {
            let parent = if level % 2 == 0 {
                let border = Border::new();
                border.set_border_thickness(Thickness::uniform(1.0));
                border.set_child(&child);
                border.upcast::<Control>()
            } else {
                let decorator = Decorator::new();
                decorator.set_padding(Thickness::uniform(1.0));
                decorator.set_child(&child);
                decorator.upcast::<Control>()
            };
            parent.set_margin(Thickness::uniform(1.0));
            parent.set_horizontal_alignment(HORIZONTAL[level % HORIZONTAL.len()]);
            parent.set_vertical_alignment(VERTICAL[(level / 2) % VERTICAL.len()]);
            if level % 3 == 0 {
                // Bounds that never bind: they are read and applied, and change no size.
                parent.set_min_width(4.0);
                parent.set_min_height(4.0);
                parent.set_max_width(4_000.0);
                parent.set_max_height(4_000.0);
            }
            controls.push(parent.clone());
            child = parent;
        }
        child.set_use_layout_rounding(layout_rounding);
        show(&fixture, &child);
        Self { fixture, controls, innermost, flip: Cell::new(false) }
    }

    fn flip(&self) -> bool {
        let value = !self.flip.get();
        self.flip.set(value);
        value
    }

    fn remeasure_rearrange(&mut self) {
        lay_out_all(&self.fixture, &self.controls);
    }

    fn leaf_resized(&mut self) {
        let width = if self.flip() { 6.0 } else { 8.0 };
        self.innermost.set_width(width);
        self.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "NestedDepth").min_iteration_time(200);
    let parameters = format!("Depth={DEPTH}");
    class.benchmark("remeasure_rearrange", parameters.clone(), || NestedDepth::new(DEPTH, true), NestedDepth::remeasure_rearrange);
    class.benchmark(
        "remeasure_rearrange_unrounded",
        parameters.clone(),
        || NestedDepth::new(DEPTH, false),
        NestedDepth::remeasure_rearrange,
    );
    class.benchmark("leaf_resized", parameters, || NestedDepth::new(DEPTH, true), NestedDepth::leaf_resized);
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "NestedDepth");
    }
}
