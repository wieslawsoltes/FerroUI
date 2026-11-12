//! `CompositionContainerVisual`: every visual has children, so the members
//! of the container class are members of [`CompositionVisual`].

use super::hit_testing::{CompositionHitTestAabbTree, ICompositionHitTester};
use super::CompositionVisual;
use std::rc::Rc;

/// A visual that can have children: every visual.
pub type CompositionContainerVisual = CompositionVisual;

/// The number of children from which the children of a visual are indexed
/// for hit testing.
pub(crate) const HIT_TEST_AABB_TREE_THRESHOLD: usize = 32;

impl CompositionVisual {
    pub(crate) fn add_hit_test_child(&self, child: &Rc<CompositionVisual>) {
        let mut tree = self.hit_test_children.borrow_mut();
        let Some(tree) = tree.as_mut() else { return };

        if let Some(order) = self.children().index_of(child) {
            tree.update(child, order as i32);
        }
        Self::update_hit_test_child_order(self, tree);
    }

    pub(crate) fn remove_hit_test_child(&self, child: &Rc<CompositionVisual>) {
        let mut tree = self.hit_test_children.borrow_mut();
        let Some(tree) = tree.as_mut() else { return };

        tree.remove(child);
        Self::update_hit_test_child_order(self, tree);
    }

    pub(crate) fn clear_hit_test_children(&self) {
        if let Some(tree) = self.hit_test_children.borrow_mut().as_mut() {
            tree.clear();
        }
    }

    /// Collects the children that may be hit by `input`, topmost first, each with where it is
    /// among the children, using the hit test index. Returns `false` when the children are
    /// not indexed (there are too few of them).
    pub(crate) fn try_query_hit_test_children<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        results: &mut Vec<(Rc<CompositionVisual>, usize)>,
    ) -> bool {
        self.try_query_hit_test_children_until::<H, ()>(input, &mut |child, index| {
            results.push((child.clone(), index));
            None
        })
        .is_some()
    }

    /// Calls `each` with the children that may be hit by `input`, topmost first, and with
    /// where each is among the children, until it answers, using the hit test index. Returns
    /// `None` when the children are not indexed, otherwise the answer (if any).
    pub(crate) fn try_query_hit_test_children_until<H: ICompositionHitTester, R>(
        &self,
        input: &H::Input,
        each: &mut dyn FnMut(&Rc<CompositionVisual>, usize) -> Option<R>,
    ) -> Option<Option<R>> {
        if self.children().count() < HIT_TEST_AABB_TREE_THRESHOLD {
            *self.hit_test_children.borrow_mut() = None;
            return None;
        }

        let read_revision = self.compositor().readback().read_revision();
        let mut tree = self.hit_test_children.borrow_mut();
        let tree = tree.get_or_insert_with(|| Box::new(CompositionHitTestAabbTree::new(self.children().clone())));
        Some(tree.query_until::<H, R>(input, read_revision, each))
    }

    fn update_hit_test_child_order(&self, tree: &mut CompositionHitTestAabbTree) {
        for (i, child) in self.children().items().iter().enumerate() {
            tree.update_order(child, i as i32);
        }
    }
}
