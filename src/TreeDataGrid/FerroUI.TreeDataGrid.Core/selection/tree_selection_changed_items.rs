//! `Selection/TreeSelectionChangedItems.cs`: the items of the indexes of a selection change, read
//! from the model when they are read.

use super::i_tree_selection_model::IReadOnlyList;
use super::index_ranges::IndexRanges;
use super::tree_selection_model_base::TreeSelectionModelBase;
use crate::accessor::TreeModel;
use std::rc::{Rc, Weak};

/// The items at the index paths of a selection change. The items are read from the model when
/// they are read, so they are only valid while the event of the change is raised.
///
/// The items are never null; they are `Option`s as the item lists of the change are
/// (`IReadOnlyList<T?>`).
pub struct TreeSelectionChangedItems<M: TreeModel> {
    owner: Weak<TreeSelectionModelBase<M>>,
    ranges: Rc<IndexRanges>,
}

impl<M: TreeModel> TreeSelectionChangedItems<M> {
    pub fn new(owner: Weak<TreeSelectionModelBase<M>>, ranges: Rc<IndexRanges>) -> Self {
        Self { owner, ranges }
    }

    /// The items of `ranges`, or `None` without ranges.
    pub fn create(owner: Weak<TreeSelectionModelBase<M>>, ranges: Option<Rc<IndexRanges>>) -> Option<Rc<Self>> {
        ranges.map(|ranges| Rc::new(Self::new(owner, ranges)))
    }

    fn owner(&self) -> Rc<TreeSelectionModelBase<M>> {
        self.owner.upgrade().expect("the selection model is alive")
    }
}

impl<M: TreeModel> IReadOnlyList<Option<Rc<M>>> for TreeSelectionChangedItems<M> {
    fn count(&self) -> usize {
        self.ranges.count().max(0) as usize
    }

    fn get(&self, index: usize) -> Option<Rc<M>> {
        Some(self.owner().get_selected_item_at(&self.ranges.get(index as i32)))
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Option<Rc<M>>> + '_> {
        let owner = self.owner();
        Box::new(self.ranges.iter().map(move |index| Some(owner.get_selected_item_at(&index))))
    }
}
