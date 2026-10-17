//! `Selection/TreeSelectedItems.cs`: the selected items of a tree selection model, as a list that
//! reads the selection.
//!
//! The reference's typed base (`TreeSelectedItemsBase<T>`) and its untyped subclass are one type
//! here; the untyped list of the model's contract wraps it in
//! [`UntypedItems`](super::UntypedItems).

use super::i_tree_selection_model::IReadOnlyList;
use super::index_range::IndexRange;
use super::tree_selection_model_base::TreeSelectionModelBase;
use super::tree_selection_node::TreeSelectionNode;
use crate::accessor::TreeModel;
use std::rc::{Rc, Weak};

/// The selected items of a tree selection model: a view of the selection, in the order of the
/// selected indexes.
pub struct TreeSelectedItems<M: TreeModel> {
    owner: Weak<TreeSelectionModelBase<M>>,
}

impl<M: TreeModel> TreeSelectedItems<M> {
    pub fn new(owner: Weak<TreeSelectionModelBase<M>>) -> Self {
        Self { owner }
    }

    fn owner(&self) -> Rc<TreeSelectionModelBase<M>> {
        self.owner.upgrade().expect("the selection model is alive")
    }

    fn enumerate_node(node: &TreeSelectionNode<M>, result: &mut Vec<Option<Rc<M>>>) {
        let items_view = node.items_view();

        for range in node.ranges().iter() {
            for i in range.begin()..=range.end() {
                if let Some(items_view) = &items_view {
                    result.push(Some(items_view.get_at(i as usize)));
                }
            }
        }

        if let Some(children) = node.children() {
            for child in children.iter().flatten() {
                Self::enumerate_node(child, result);
            }
        }
    }

    fn try_get_element_at(node: &TreeSelectionNode<M>, target: i32, next: &mut i32) -> Option<Option<Rc<M>>> {
        let ranges = node.ranges();
        let node_count = IndexRange::get_count(&ranges);

        if target < *next + node_count {
            let items_view = node.items_view().expect("Object reference not set to an instance of an object.");
            return Some(Some(items_view.get_at(IndexRange::get_at(&ranges, target - *next) as usize)));
        }

        *next += node_count;

        if let Some(children) = node.children() {
            for child in children.iter().flatten() {
                if let Some(result) = Self::try_get_element_at(child, target, next) {
                    return Some(result);
                }
            }
        }

        None
    }
}

impl<M: TreeModel> IReadOnlyList<Option<Rc<M>>> for TreeSelectedItems<M> {
    fn count(&self) -> usize {
        self.owner().count().max(0) as usize
    }

    /// # Panics
    /// When `index` is not below the count (`IndexOutOfRangeException`).
    fn get(&self, index: usize) -> Option<Rc<M>> {
        let owner = self.owner();

        if index >= owner.count().max(0) as usize {
            panic!("The index was out of range.");
        }

        if owner.single_select() {
            owner.selected_item()
        } else {
            let mut next = 0;
            Self::try_get_element_at(&owner.root(), index as i32, &mut next).flatten()
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Option<Rc<M>>> + '_> {
        let owner = self.owner();
        let mut result = Vec::new();

        if owner.single_select() {
            if owner.selected_index().count() > 0 {
                result.push(owner.selected_item());
            }
        } else {
            Self::enumerate_node(&owner.root(), &mut result);
        }

        Box::new(result.into_iter())
    }
}
