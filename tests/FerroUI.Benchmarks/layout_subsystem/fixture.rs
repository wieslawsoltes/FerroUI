//! The leaves, the passes and the queued jobs of the layout benchmarks.

use crate::trees::fixture::{Brushes, Fixture};
use ferroui_base::layout::testing::{set_is_arrange_valid, set_is_measure_valid};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::Ref;
use ferroui_controls::{Border, Control};

/// A border of a fixed size: its measure and its arrange depend on nothing but its own values.
pub fn leaf(width: f64, height: f64, brushes: &Brushes) -> Ref<Border> {
    let leaf = Border::new();
    leaf.set_width(width);
    leaf.set_height(height);
    leaf.set_background(Some(brushes.tile.clone()));
    leaf
}

/// `count` leaves of the same size.
pub fn leaves(count: usize, width: f64, height: f64, brushes: &Brushes) -> Vec<Ref<Border>> {
    (0..count).map(|_| leaf(width, height, brushes)).collect()
}

/// Shows `child` in the root and lays it out.
pub fn show(fixture: &Fixture, child: &Ref<Control>) {
    fixture.show(Some(child));
    fixture.lay_out();
}

/// Measures and arranges the root and every control of `controls` again.
///
/// The flags are set through their setters of the `testing` feature of the base crate and not
/// through `invalidate_measure`, as `layout::Measure.remeasure` does: the invalidations are not
/// what such a benchmark is about.
pub fn lay_out_all(fixture: &Fixture, controls: &[Ref<Control>]) {
    fixture.root.invalidate_measure();
    for control in controls {
        set_is_measure_valid(control, false);
        set_is_arrange_valid(control, false);
    }
    fixture.lay_out();
}

/// Runs the jobs the dispatcher holds, down to the priority of the render loop: the requests for
/// a pass that the invalidations between two passes run by hand leave behind. A request finds
/// its pass done and returns; one made after the last pass runs that pass.
pub fn run_queued_jobs() {
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
}
