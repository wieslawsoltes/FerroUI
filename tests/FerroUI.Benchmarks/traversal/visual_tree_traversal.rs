//! Walking up the visual tree: the ancestor of a class and the common
//! ancestor of two visuals.

use crate::control_hierarchy_creator::ControlHierarchyCreator;
use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::Ref;
use ferroui_controls::testing::NullRenderer;
use ferroui_controls::{Control, StackPanel};

pub struct VisualTreeTraversal {
    controls: Vec<Ref<Control>>,
    shuffled_controls: Vec<Ref<Control>>,
    root: Ref<TestRoot>,
    /// The dispatcher of the thread of the benchmark: the layout pass of the root reads it, and
    /// a dispatcher belongs to the thread that first asks for it. Declared last: released after
    /// the tree.
    _dispatcher: UnitTestDispatcherScope,
}

/// The keys the controls are ordered by to shuffle them. Upstream orders by
/// the values of the random number generator of its platform with the seed
/// 1; that sequence is not reproduced here, so the order differs from the
/// upstream one, and is the same on every run as it is upstream.
struct ShuffleKeys(u64);

impl ShuffleKeys {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next key: a step of the SplitMix64 generator, reduced to the
    /// non-negative 31 bit range of the upstream keys.
    fn next(&mut self) -> i32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 33) as i32
    }
}

impl VisualTreeTraversal {
    pub fn new() -> Self {
        let dispatcher = Dispatcher::unit_test_scope();
        let panel = StackPanel::new();
        let root = TestRoot::new();
        root.set_child(&panel);
        root.set_renderer(NullRenderer::new());
        let mut controls: Vec<Ref<Control>> = Vec::new();
        controls.push(panel.clone().upcast());
        let controls = ControlHierarchyCreator::create_children(controls, &panel, 3, 5, 4);

        let mut random = ShuffleKeys::new(1);

        // A stable sort by one key per control, as the upstream ordering.
        let mut keyed: Vec<(i32, Ref<Control>)> =
            controls.iter().map(|control| (random.next(), control.clone())).collect();
        keyed.sort_by_key(|(key, _)| *key);
        let shuffled_controls = keyed.into_iter().map(|(_, control)| control).collect();

        root.layout_manager().execute_initial_layout_pass();

        Self { controls, shuffled_controls, root, _dispatcher: dispatcher }
    }

    /// The upstream benchmark filters the sequence of the visual and its
    /// ancestors by class and takes the first.
    pub fn find_ancestor_of_type_linq(&self) {
        for control in &self.controls {
            let found = control.get_self_and_visual_ancestors().find_map(|visual| visual.cast::<TestRoot>());
            std::hint::black_box(found);
        }
    }

    pub fn find_ancestor_of_type_optimized(&self) {
        for control in &self.controls {
            std::hint::black_box(control.find_ancestor_of_type::<TestRoot>(false));
        }
    }

    pub fn find_common_visual_ancestor(&self) {
        for first in &self.controls {
            for second in &self.shuffled_controls {
                std::hint::black_box(first.find_common_visual_ancestor(second));
            }
        }
    }

    /// The root of the tree the benchmarks walk.
    pub fn root(&self) -> &Ref<TestRoot> {
        &self.root
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("traversal", "VisualTreeTraversal");
    class.benchmark("find_ancestor_of_type_linq", "", VisualTreeTraversal::new, |b| b.find_ancestor_of_type_linq());
    class.benchmark("find_ancestor_of_type_optimized", "", VisualTreeTraversal::new, |b| {
        b.find_ancestor_of_type_optimized()
    });
    class.benchmark("find_common_visual_ancestor", "", VisualTreeTraversal::new, |b| b.find_common_visual_ancestor());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_tree_traversal() {
        crate::harness::smoke_class(super::register, "VisualTreeTraversal");
    }

    #[test]
    fn every_control_of_the_tree_has_the_root_as_an_ancestor() {
        let benchmark = VisualTreeTraversal::new();
        assert_eq!(benchmark.controls.len(), benchmark.shuffled_controls.len());
        for control in &benchmark.controls {
            let found = control.find_ancestor_of_type::<TestRoot>(false);
            assert!(found.as_ref() == Some(benchmark.root()), "the root is the ancestor of every control");
        }
    }
}
