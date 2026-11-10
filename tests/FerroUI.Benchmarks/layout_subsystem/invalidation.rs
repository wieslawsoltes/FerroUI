//! Invalidations in a laid out tree and the passes that answer them: how much of the tree a pass
//! touches when a part of it is invalid (section 9.21 of the performance notes: the walks stop
//! where nothing changes; section 9.15: a child whose constraint is unchanged is not measured).
//!
//! The tree is a screen of ten sections: a stack of borders, each around a stack of ten rows of
//! forty leaves, so a leaf is five elements below the root.
//!
//! - `measure_one_leaf`: the measure of one leaf is invalidated and the layout manager runs a
//!   pass. The size the leaf desires does not change, so the pass measures and arranges the leaf
//!   and nothing else;
//! - `resize_one_leaf`: the width of one leaf changes and the layout manager runs a pass. The size
//!   every ancestor desires changes, so the pass measures and arranges the path from the leaf to
//!   the root, and each panel on it visits its children, which have nothing to do;
//! - `arrange_one_leaf`: the arrange of one leaf is invalidated and the layout manager runs a pass;
//! - `measure_tenth_of_leaves`: one leaf in ten is invalidated, then one pass;
//! - `measure_every_leaf`: every leaf is invalidated, then one pass;
//! - `invalidate_every_leaf`: the invalidations of `measure_every_leaf` without the pass: what the
//!   call itself costs on a valid element (the flags, the queue of the layout manager, the request
//!   for a redraw).
//!
//! The first three repeat their step over a thousand leaves in one invocation and report the time
//! of one pass; the requests for a pass the invalidations leave with the render loop are dropped
//! between iterations.

use super::fixture::{leaves, run_queued_jobs, show};
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::layout::Orientation;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{Border, Control, StackPanel};

const SECTIONS: usize = 10;
const SECTION_ROWS: usize = 10;
const ROW_LEAVES: usize = 40;
const LEAVES: usize = SECTIONS * SECTION_ROWS * ROW_LEAVES;

/// The leaves a benchmark of one leaf goes through in one invocation: one in four, so every row.
const ROUNDS: u32 = 1_000;
const ROUND_STEP: usize = LEAVES / ROUNDS as usize;

const LEAF_WIDTH: f64 = 5.0;

pub struct Invalidation {
    fixture: Fixture,
    leaves: Vec<Ref<Border>>,
}

impl Invalidation {
    fn new() -> Self {
        let fixture = Fixture::new();
        let leaves = leaves(LEAVES, LEAF_WIDTH, 4.0, &fixture.brushes);
        let screen = StackPanel::new();
        for section in leaves.chunks(SECTION_ROWS * ROW_LEAVES) {
            let rows = StackPanel::new();
            for row_leaves in section.chunks(ROW_LEAVES) {
                let row = StackPanel::new();
                row.set_orientation(Orientation::Horizontal);
                for leaf in row_leaves {
                    row.children().add(leaf);
                }
                rows.children().add(&row);
            }
            let frame = Border::new();
            frame.set_padding(Thickness::uniform(2.0));
            frame.set_child(&rows);
            screen.children().add(&frame);
        }
        show(&fixture, &screen.upcast::<Control>());
        Self { fixture, leaves }
    }

    /// Leaves the tree valid, with no request for a pass left behind.
    fn settle(&mut self) {
        run_queued_jobs();
        self.fixture.lay_out();
    }

    fn round_leaves(&self) -> impl Iterator<Item = &Ref<Border>> {
        self.leaves.iter().step_by(ROUND_STEP)
    }

    fn measure_one_leaf(&mut self) {
        for leaf in self.round_leaves() {
            leaf.invalidate_measure();
            self.fixture.lay_out();
        }
    }

    fn resize_one_leaf(&mut self) {
        for leaf in self.round_leaves() {
            leaf.set_width(LEAF_WIDTH + 1.0);
            self.fixture.lay_out();
            leaf.set_width(LEAF_WIDTH);
            self.fixture.lay_out();
        }
    }

    fn arrange_one_leaf(&mut self) {
        for leaf in self.round_leaves() {
            leaf.invalidate_arrange();
            self.fixture.lay_out();
        }
    }

    fn measure_tenth_of_leaves(&mut self) {
        for leaf in self.leaves.iter().step_by(10) {
            leaf.invalidate_measure();
        }
        self.fixture.lay_out();
    }

    fn measure_every_leaf(&mut self) {
        self.invalidate_every_leaf();
        self.fixture.lay_out();
    }

    fn invalidate_every_leaf(&mut self) {
        for leaf in &self.leaves {
            leaf.invalidate_measure();
        }
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "Invalidation").min_iteration_time(200);
    let parameters = format!("Leaves={LEAVES}");
    class
        .benchmark_with_iteration_setup(
            "measure_one_leaf",
            parameters.clone(),
            Invalidation::new,
            Invalidation::settle,
            Invalidation::measure_one_leaf,
        )
        .operations_per_invoke(ROUNDS);
    class
        .benchmark_with_iteration_setup(
            "resize_one_leaf",
            parameters.clone(),
            Invalidation::new,
            Invalidation::settle,
            Invalidation::resize_one_leaf,
        )
        .operations_per_invoke(2 * ROUNDS);
    class
        .benchmark_with_iteration_setup(
            "arrange_one_leaf",
            parameters.clone(),
            Invalidation::new,
            Invalidation::settle,
            Invalidation::arrange_one_leaf,
        )
        .operations_per_invoke(ROUNDS);
    class.benchmark("measure_tenth_of_leaves", parameters.clone(), Invalidation::new, Invalidation::measure_tenth_of_leaves);
    class.benchmark("measure_every_leaf", parameters.clone(), Invalidation::new, Invalidation::measure_every_leaf);
    class.benchmark_with_iteration_setup(
        "invalidate_every_leaf",
        parameters,
        Invalidation::new,
        Invalidation::settle,
        Invalidation::invalidate_every_leaf,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "Invalidation");
    }
}
