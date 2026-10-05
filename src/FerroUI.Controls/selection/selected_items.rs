use super::index_range::IndexRange;
use super::read_only_selection_list_base::{IReadOnlySelectionList, ReadOnlySelectionListBase};
use super::selection_model::SelectionModel;
use crate::items_source::{box_item, ItemsChangedHandler};
use crate::items_source_view::ItemsSourceViewOf;
use ferroui_base::{BoxedValue, PropertyValue};
use std::rc::{Rc, Weak};

/// The selected items: a view of the selection of a model, or of a list of
/// ranges over a view of the source. A null item, and an item selected
/// while there is no source, is `None`.
///
/// The view of a model refers to it weakly (the model caches the view); it
/// is empty once the model is released.
pub struct SelectedItems<T: PropertyValue> {
    base: ReadOnlySelectionListBase,
    owner: Option<Weak<SelectionModel<T>>>,
    items: Option<ItemsSourceViewOf<T>>,
    ranges: Option<Rc<Vec<IndexRange>>>,
}

impl<T: PropertyValue> SelectedItems<T> {
    pub(crate) fn new(owner: Weak<SelectionModel<T>>) -> Self {
        Self { base: ReadOnlySelectionListBase::new(), owner: Some(owner), items: None, ranges: None }
    }

    pub(crate) fn from_ranges(ranges: Rc<Vec<IndexRange>>, items: Option<ItemsSourceViewOf<T>>) -> Self {
        Self { base: ReadOnlySelectionListBase::new(), owner: None, items, ranges: Some(ranges) }
    }

    pub(crate) fn create(
        ranges: Option<Rc<Vec<IndexRange>>>,
        items: Option<ItemsSourceViewOf<T>>,
    ) -> Option<Rc<SelectedItems<T>>> {
        ranges.map(|ranges| Rc::new(Self::from_ranges(ranges, items)))
    }

    /// Notifies the subscribers that the list changed.
    pub fn raise_collection_reset(&self) {
        self.base.raise_collection_reset();
    }

    fn owner(&self) -> Option<Rc<SelectionModel<T>>> {
        self.owner.as_ref().and_then(Weak::upgrade)
    }

    fn items(&self, owner: &Option<Rc<SelectionModel<T>>>) -> Option<ItemsSourceViewOf<T>> {
        match &self.items {
            Some(items) => Some(items.clone()),
            None => owner.as_ref().and_then(|owner| owner.node().items_view()),
        }
    }

    fn ranges(&self, owner: &Option<Rc<SelectionModel<T>>>) -> Option<Rc<Vec<IndexRange>>> {
        match &self.ranges {
            Some(ranges) => Some(ranges.clone()),
            None => owner.as_ref().map(|owner| owner.node().ranges()),
        }
    }
}

impl<T: PropertyValue> IReadOnlySelectionList<Option<T>> for SelectedItems<T> {
    fn get(&self, index: usize) -> Option<T> {
        if index >= self.count() {
            panic!("The index was out of range.");
        }

        let owner = self.owner();

        if let Some(owner) = owner.as_ref().filter(|owner| owner.single_select()) {
            return owner.selected_item();
        }

        match (self.items(&owner), self.ranges(&owner)) {
            (Some(items), Some(ranges)) => items.get_at(IndexRange::get_at(&ranges, index as i32) as usize),
            _ => None,
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
            _ => match self.ranges(&owner) {
                Some(ranges) => IndexRange::get_count(&ranges).max(0) as usize,
                None => 0,
            },
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Option<T>> + '_> {
        let owner = self.owner();

        if let Some(owner) = owner.as_ref().filter(|owner| owner.single_select()) {
            return Box::new((owner.selected_index() >= 0).then(|| owner.selected_item()).into_iter());
        }

        let items = self.items(&owner);
        let ranges = self.ranges(&owner).unwrap_or_default();

        Box::new(IndexRange::enumerate_indices(ranges).map(move |i| match &items {
            Some(items) => items.get_at(i as usize),
            None => None,
        }))
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.base.add_collection_changed(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.base.remove_collection_changed(token);
    }
}

/// The untyped view of a list of selected items.
pub struct SelectedItemsUntyped<T: PropertyValue> {
    base: ReadOnlySelectionListBase,
    source: Rc<dyn IReadOnlySelectionList<Option<T>>>,
}

impl<T: PropertyValue> SelectedItemsUntyped<T> {
    pub fn new(source: Rc<dyn IReadOnlySelectionList<Option<T>>>) -> Self {
        Self { base: ReadOnlySelectionListBase::new(), source }
    }

    /// Notifies the subscribers that the list changed.
    pub fn raise_collection_reset(&self) {
        self.base.raise_collection_reset();
    }
}

fn box_selected<T: PropertyValue>(item: Option<T>) -> Option<BoxedValue> {
    match item {
        Some(item) => box_item(&item),
        None => None,
    }
}

impl<T: PropertyValue> IReadOnlySelectionList<Option<BoxedValue>> for SelectedItemsUntyped<T> {
    fn get(&self, index: usize) -> Option<BoxedValue> {
        box_selected(self.source.get(index))
    }

    fn count(&self) -> usize {
        self.source.count()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Option<BoxedValue>> + '_> {
        Box::new(self.source.iter().map(box_selected))
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.base.add_collection_changed(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.base.remove_collection_changed(token);
    }
}
