//! `Selection/TreeSelectedIndexes.cs`: the selected indexes of a tree selection model, as a list
//! that reads the selection.

use super::i_tree_selection_model::IReadOnlyList;
use super::index_range::IndexRange;
use super::index_ranges::IndexRanges;
use super::tree_selection_model_base::TreeSelectionModelBase;
use super::tree_selection_node::TreeSelectionNode;
use crate::accessor::TreeModel;
use crate::IndexPath;
use std::rc::{Rc, Weak};

/// The selected indexes of a tree selection model: a view of the selection, in order of the
/// nodes (the selected children of a node before the selection below them).
pub struct TreeSelectedIndexes<M: TreeModel> {
    owner: Weak<TreeSelectionModelBase<M>>,
}

impl<M: TreeModel> TreeSelectedIndexes<M> {
    pub fn new(owner: Weak<TreeSelectionModelBase<M>>) -> Self {
        Self { owner }
    }

    fn owner(&self) -> Rc<TreeSelectionModelBase<M>> {
        self.owner.upgrade().expect("the selection model is alive")
    }

    fn enumerate_node(node: &TreeSelectionNode<M>, result: &mut Vec<IndexPath>) {
        let path = node.path();

        for range in node.ranges().iter() {
            for i in range.begin()..=range.end() {
                result.push(path.append(i));
            }
        }

        if let Some(children) = node.children() {
            for child in children.iter().flatten() {
                Self::enumerate_node(child, result);
            }
        }
    }

    fn try_get_element_at(node: &TreeSelectionNode<M>, target: i32, next: &mut i32) -> Option<IndexPath> {
        let ranges = node.ranges();
        let node_count = IndexRange::get_count(&ranges);

        if target < *next + node_count {
            return Some(node.path().append(IndexRange::get_at(&ranges, target - *next)));
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

impl<M: TreeModel> IReadOnlyList<IndexPath> for TreeSelectedIndexes<M> {
    fn count(&self) -> usize {
        self.owner().count().max(0) as usize
    }

    /// # Panics
    /// When `index` is not below the count (`IndexOutOfRangeException`).
    fn get(&self, index: usize) -> IndexPath {
        let owner = self.owner();

        if index >= owner.count().max(0) as usize {
            panic!("The index was out of range.");
        }

        if owner.single_select() {
            owner.selected_index()
        } else {
            let mut next = 0;
            Self::try_get_element_at(&owner.root(), index as i32, &mut next).unwrap_or_default()
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = IndexPath> + '_> {
        let owner = self.owner();
        let mut result = Vec::new();

        if owner.single_select() {
            let selected_index = owner.selected_index();
            if selected_index.count() != 0 {
                result.push(selected_index);
            }
        } else {
            Self::enumerate_node(&owner.root(), &mut result);
        }

        Box::new(result.into_iter())
    }
}

/// A set of index paths read as the list of its paths: the indexes of a selection change.
impl IReadOnlyList<IndexPath> for IndexRanges {
    fn count(&self) -> usize {
        IndexRanges::count(self).max(0) as usize
    }

    fn get(&self, index: usize) -> IndexPath {
        IndexRanges::get(self, index as i32)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = IndexPath> + '_> {
        Box::new(IndexRanges::iter(self))
    }

    fn to_vec(&self) -> Vec<IndexPath> {
        IndexRanges::to_vec(self)
    }
}
