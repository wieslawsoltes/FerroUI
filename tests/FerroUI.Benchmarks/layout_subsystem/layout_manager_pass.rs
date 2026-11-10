//! The pass of the layout manager itself: its two queues, the walk from each queued control up to
//! the root that precedes its measure and its arrange, and what the pass does when it has nothing
//! to do (sections 9.21 and 9.28 of the performance notes).
//!
//! The tree is a stack of a thousand branches; a branch is a leaf inside zero to seven nested
//! decorators, so the leaves are one to eight levels below the stack and none is the ancestor of
//! another.
//!
//! - `idle`: a pass with nothing invalid: the fixed cost of a pass, paid by every frame that
//!   changes no layout;
//! - `measure_unrelated`: a pass after the measure of every leaf was invalidated. No desired size
//!   changes, so each leaf is taken from the queue, measured and arranged alone, after a walk of
//!   its valid ancestors;
//! - `arrange_unrelated`: the same for the arrange of every leaf;
//! - `measure_related`: a pass after the measure of every leaf and of every decorator was
//!   invalidated, the innermost first: an element measured as the child of an earlier one is
//!   still in the queue and is taken from it with nothing left to do.
//!
//! The invalidations are made by the iteration setup, so the three measure the pass alone.

use super::fixture::{leaf, run_queued_jobs, show};
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::Ref;
use ferroui_controls::{Control, Decorator, StackPanel};

const LEAVES: usize = 1_000;

/// A branch has its index modulo this many decorators around its leaf.
const BRANCH_DEPTHS: usize = 8;

pub struct LayoutManagerPass {
    fixture: Fixture,
    leaves: Vec<Ref<Control>>,
    /// The leaves and the decorators around them, each branch from its leaf outwards.
    branches: Vec<Ref<Control>>,
}

impl LayoutManagerPass {
    fn new() -> Self {
        let fixture = Fixture::new();
        let panel = StackPanel::new();
        let mut leaves = Vec::with_capacity(LEAVES);
        let mut branches = Vec::new();
        for index in 0..LEAVES {
            let inner = leaf(10.0, 4.0, &fixture.brushes).upcast::<Control>();
            leaves.push(inner.clone());
            branches.push(inner.clone());
            let mut branch = inner;
            for _ in 0..index % BRANCH_DEPTHS {
                let decorator = Decorator::new();
                decorator.set_child(&branch);
                branch = decorator.upcast::<Control>();
                branches.push(branch.clone());
            }
            panel.children().add(&branch);
        }
        show(&fixture, &panel.upcast::<Control>());
        Self { fixture, leaves, branches }
    }

    fn lay_out(&mut self) {
        self.fixture.lay_out();
    }

    fn invalidate_measure_of_leaves(&mut self) {
        run_queued_jobs();
        for leaf in &self.leaves {
            leaf.invalidate_measure();
        }
    }

    fn invalidate_arrange_of_leaves(&mut self) {
        run_queued_jobs();
        for leaf in &self.leaves {
            leaf.invalidate_arrange();
        }
    }

    fn invalidate_measure_of_branches(&mut self) {
        run_queued_jobs();
        for control in &self.branches {
            control.invalidate_measure();
        }
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "LayoutManagerPass").min_iteration_time(200);
    let parameters = format!("Leaves={LEAVES}");
    class.benchmark("idle", parameters.clone(), LayoutManagerPass::new, LayoutManagerPass::lay_out).baseline();
    class.benchmark_with_iteration_setup(
        "measure_unrelated",
        parameters.clone(),
        LayoutManagerPass::new,
        LayoutManagerPass::invalidate_measure_of_leaves,
        LayoutManagerPass::lay_out,
    );
    class.benchmark_with_iteration_setup(
        "arrange_unrelated",
        parameters.clone(),
        LayoutManagerPass::new,
        LayoutManagerPass::invalidate_arrange_of_leaves,
        LayoutManagerPass::lay_out,
    );
    class.benchmark_with_iteration_setup(
        "measure_related",
        parameters,
        LayoutManagerPass::new,
        LayoutManagerPass::invalidate_measure_of_branches,
        LayoutManagerPass::lay_out,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "LayoutManagerPass");
    }
}
