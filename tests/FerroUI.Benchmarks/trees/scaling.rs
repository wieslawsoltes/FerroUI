//! Scaling pairs: the operations of a wall of tiles at 1,000 and at 4,000 tiles. A cost that is
//! linear in the tiles takes four times the instructions at four times the tiles; one that grows
//! faster takes more, and the gate holds the ratio of each pair against that
//! (`scripts/perf/gate.py`, "scaling pairs"; experiment #755). The regressions that cost the most
//! were of this kind: linear at the size a benchmark had and quadratic above it.
//!
//! Each benchmark is one whole operation on the wall (not the time of one tile), so the
//! instructions of a pair compare as they are:
//!
//! - `start`: a wall built, shown in the root and laid out;
//! - `attach`: a built wall shown in the root;
//! - `detach`: an attached, laid out wall taken out of the root;
//! - `clear`: the tiles of a canvas removed at once;
//! - `remove_half`: half the tiles of a canvas removed one at a time, each the one before the last;
//! - `toggle_class`: a class set on every tile of an attached wall, which a style selects;
//! - `resource_changed`: a resource of the root replaced.
//!
//! The scroll offset has its pair in `layout_subsystem::ScrollContent.offset`.

use super::lifetime::Lifetime;
use super::styling::Styling;
use crate::harness::Registry;

/// The sizes of a pair: the second is `SCALING_FACTOR` of the gate times the first.
const SIZES: [usize; 2] = [1_000, 4_000];

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("trees", "Scaling").min_iteration_time(200);
    for tiles in SIZES {
        let parameters = format!("Tiles={tiles}");
        let wall = move || Lifetime::new(tiles);
        class.benchmark_with_iteration_setup("start", parameters.clone(), wall, Lifetime::prepare_start, Lifetime::start);
        class.benchmark_with_iteration_setup("attach", parameters.clone(), wall, Lifetime::prepare_detached, Lifetime::attach);
        class.benchmark_with_iteration_setup("detach", parameters.clone(), wall, Lifetime::prepare_laid_out, Lifetime::detach);
        class.benchmark_with_iteration_setup("clear", parameters.clone(), wall, Lifetime::prepare_canvas, Lifetime::clear);
        class.benchmark_with_iteration_setup(
            "remove_half",
            parameters.clone(),
            wall,
            Lifetime::prepare_canvas,
            Lifetime::remove_half,
        );
        let styled = move || Styling::new_attached(tiles);
        class.benchmark_with_iteration_setup("toggle_class", parameters.clone(), styled, Styling::settle, Styling::toggle_class);
        class.benchmark_with_iteration_setup("resource_changed", parameters, styled, Styling::settle, Styling::resource_changed);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "Scaling");
    }
}
