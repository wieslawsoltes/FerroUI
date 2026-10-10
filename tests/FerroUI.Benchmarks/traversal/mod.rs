//! The benchmarks of walking the visual tree.

use crate::harness::Registry;

pub mod visual_tree_traversal;

pub fn register(registry: &mut Registry) {
    visual_tree_traversal::register(registry);
}
