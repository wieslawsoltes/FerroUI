//! The tree mechanics of the framework on trees shaped like heavy desktop screens: building,
//! attaching, detaching and laying out walls of small tiles, passing values down them (inherited
//! properties, enabled and visible states, flow direction, data contexts, theme variants, styles),
//! styling them, and the text of their labels. FerroUI's own regression benchmarks, run by the
//! local performance gate (`scripts/perf/gate.py`, suite `framework`): each guards a finding of the
//! performance notes (`docs/porting/desktop-performance.md`, sections 8.1 and 9) and the walks of
//! the tree that were made to stop where nothing changes.
//!
//! The services are a desktop's: the Fluent theme in its light variant, Skia's font manager and
//! HarfBuzz; the roots render nothing.

use crate::harness::Registry;

pub mod fixture;
pub mod lifetime;
pub mod propagation;
pub mod styling;
pub mod text;

pub fn register(registry: &mut Registry) {
    lifetime::register(registry);
    propagation::register(registry);
    styling::register(registry);
    text::register(registry);
}
