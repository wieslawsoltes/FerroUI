use super::index_range::IndexRange;
use super::read_only_selection_list_base::{IReadOnlySelectionList, ReadOnlySelectionListBase};
use super::selection_model::SelectionModel;
use crate::items_source::ItemsChangedHandler;
use ferroui_base::PropertyValue;
use std::rc::{Rc, Weak};

/// The selected indexes: a view of the selection of a model, or of a list
/// of ranges.
///
/// The view of a model refers to it weakly (the model caches the view); it
/// is empty once the model is released.
pub struct SelectedIndexes<T: PropertyValue> {
    base: ReadOnlySelectionListBase,
    owner: Option<Weak<SelectionModel<T>>>,
    ranges: Option<Rc<Vec<IndexRange>>>,
}

impl<T: PropertyValue> SelectedIndexes<T> {
    pub(crate) fn new(owner: Weak<SelectionModel<T>>) -> Self {
        Self { base: ReadOnlySelectionListBase::new(), owner: Some(owner), ranges: None }
    }

    pub(crate) fn from_ranges(ranges: Rc<Vec<IndexRange>>) -> Self {
        Self { base: ReadOnlySelectionListBase::new(), owner: None, ranges: Some(ranges) }
    }

    pub(crate) fn create(ranges: Option<Rc<Vec<IndexRange>>>) -> Option<Rc<SelectedIndexes<T>>> {
        ranges.map(|ranges| Rc::new(Self::from_ranges(ranges)))
    }

    /// Notifies the subscribers that the list changed.
    pub fn raise_collection_reset(&self) {
        self.base.raise_collection_reset();
    }

    fn owner(&self) -> Option<Rc<SelectionModel<T>>> {
        self.owner.as_ref().and_then(Weak::upgrade)
    }

    fn ranges(&self, owner: &Option<Rc<SelectionModel<T>>>) -> Rc<Vec<IndexRange>> {
        match (&self.ranges, owner) {
            (Some(ranges), _) => ranges.clone(),
            (None, Some(owner)) => owner.node().ranges(),
            (None, None) => Rc::default(),
        }
    }
}

impl<T: PropertyValue> IReadOnlySelectionList<i32> for SelectedIndexes<T> {
    fn get(&self, index: usize) -> i32 {
        if index >= self.count() {
            panic!("The index was out of range.");
        }

        let owner = self.owner();

        match &owner {
            Some(owner) if owner.single_select() => owner.selected_index(),
            _ => IndexRange::get_at(&self.ranges(&owner), index as i32),
        }
    }

    fn count(&self) -> usize {
        let owner = self.owner();

        match &owner {
            Some(owner) if owner.single_select() => {
                if owner.selected_index() == -1 {
                    0
                } else {
                    1
                }
            }
            _ => IndexRange::get_count(&self.ranges(&owner)).max(0) as usize,
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = i32> + '_> {
        let owner = self.owner();

        match &owner {
            Some(owner) if owner.single_select() => {
                let selected_index = owner.selected_index();
                Box::new((selected_index >= 0).then_some(selected_index).into_iter())
            }
            _ => Box::new(IndexRange::enumerate_indices(self.ranges(&owner))),
        }
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.base.add_collection_changed(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.base.remove_collection_changed(token);
    }
}
