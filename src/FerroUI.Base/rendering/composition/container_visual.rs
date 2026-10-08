//! `CompositionContainerVisual`: every visual has children, so the members
//! of the container class are members of [`CompositionVisual`].

use super::hit_testing::{CompositionHitTestAabbTree, ICompositionHitTester};
use super::{CompositionTarget, CompositionVisual};
use crate::media::IntersectionResult;
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

    /// Collects the children that may be hit by `input`, topmost first,
    /// using the hit test index. Returns `false` when the children are not
    /// indexed (there are too few of them).
    pub(crate) fn try_query_hit_test_children<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        results: &mut Vec<Rc<CompositionVisual>>,
    ) -> bool {
        if self.children().count() < HIT_TEST_AABB_TREE_THRESHOLD {
            *self.hit_test_children.borrow_mut() = None;
            return false;
        }

        let read_revision = self.compositor().readback().read_revision();
        let mut tree = self.hit_test_children.borrow_mut();
        let tree = tree.get_or_insert_with(|| CompositionHitTestAabbTree::new(self.children().clone()));
        tree.query::<H>(input, results, read_revision);
        true
    }

    /// Finds the topmost hit among the children using the hit test index.
    /// Returns `None` when the children are not indexed, otherwise the hit
    /// (if any) and its intersection result.
    pub(crate) fn try_query_first_hit_test_child<H: ICompositionHitTester>(
        &self,
        target: &CompositionTarget,
        input: &H::Input,
        filter: Option<&dyn Fn(&Rc<CompositionVisual>) -> bool>,
        result_filter: Option<&dyn Fn(&Rc<CompositionVisual>) -> bool>,
    ) -> Option<(Option<Rc<CompositionVisual>>, IntersectionResult)> {
        if self.children().count() < HIT_TEST_AABB_TREE_THRESHOLD {
            *self.hit_test_children.borrow_mut() = None;
            return None;
        }

        let read_revision = self.compositor().readback().read_revision();
        let mut tree = self.hit_test_children.borrow_mut();
        let tree = tree.get_or_insert_with(|| CompositionHitTestAabbTree::new(self.children().clone()));
        Some(tree.query_first::<H>(target, input, filter, result_filter, read_revision))
    }

    fn update_hit_test_child_order(&self, tree: &mut CompositionHitTestAabbTree) {
        for (i, child) in self.children().items().iter().enumerate() {
            tree.update_order(child, i as i32);
        }
    }
}
