//! The property system, on real controls: what one read and one write of a property costs a
//! control of the tree, by the kind of the property and by where the value comes from.
//! FerroUI's own regression benchmarks, run by the local performance gate
//! (`scripts/perf/gate.py`, suite `framework`); the notes they belong to are section 9.35 of
//! `docs/porting/desktop-performance.md`.
//!
//! The benchmarks of `base` measure the property store on objects that have nothing but
//! properties, in a loop over one object. Here the objects are controls with the class handlers of
//! their whole hierarchy, and a benchmark walks many of them, as a layout pass, a styling pass or
//! a render pass does: a walk over sixteen controls measures the instructions of a read, a walk
//! over ten thousand also measures the memory a read has to reach.
//!
//! Every benchmark states its operations (`operations_per_invoke`), so the time reported is that
//! of one read or one write.

use crate::harness::Registry;

pub mod control_read;
pub mod control_write;

pub fn register(registry: &mut Registry) {
    control_read::register(registry);
    control_write::register(registry);
}
