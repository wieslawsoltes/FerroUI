//! `Selection/SelectionNodeBase.cs`: the base of the nodes of a selection: the source collection,
//! its view, the selected ranges and the adjustment of the ranges to the changes of the source.
//!
//! This source file is adapted from the WinUI project (<https://github.com/microsoft/microsoft-ui-xaml>),
//! MIT License, Copyright (c) Microsoft Corporation.
//!
//! The reference's virtual members are modelled as the framework's selection does: the base keeps
//! the state and the base implementations (the inherent methods of [`SelectionNodeBase`]), and
//! holds a weak reference to the object deriving from it ([`SelectionNodeBaseImpl`]), through which
//! every virtual call is made. The base also holds the deriving object as the weak listener of the
//! collection changes of its source (`ICollectionChangedListener`), which the deriving object
//! forwards to [`SelectionNodeBase::pre_changed`], [`SelectionNodeBase::changed`] and
//! [`SelectionNodeBase::post_changed`].

use super::index_range::IndexRange;
use crate::accessor::TreeModel;
use crate::items_source_view::TreeDataGridItemsSourceView;
use crate::model_collection::{Models, ModelsChangedArgs};
use crate::models::collection_extensions::has_known_indexes;
use crate::utils::ICollectionChangedListener;
use ferroui_base::collections::NotifyCollectionChangedAction;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Whether two sources are the same collection (the reference's reference comparison of
/// `IEnumerable`s).
pub(crate) fn same_source<M: 'static>(a: Option<&Models<M>>, b: Option<&Models<M>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        _ => false,
    }
}

/// The overridable members of [`SelectionNodeBase`].
///
/// Every method but [`on_source_reset`](Self::on_source_reset) (abstract in the reference) defaults
/// to the base implementation.
pub trait SelectionNodeBaseImpl<M: TreeModel> {
    /// The base state of the node.
    fn node_base(&self) -> &SelectionNodeBase<M>;

    /// Called when the source collection starts changing.
    fn on_source_collection_change_started(&self) {}

    /// Called when the source collection changes.
    ///
    /// The base implementation ([`SelectionNodeBase::on_source_collection_changed`]) calls
    /// `on_items_added` and `on_items_removed` in order to calculate how the collection change
    /// affects the currently selected items. It then calls
    /// [`on_indexes_changed`](Self::on_indexes_changed) and
    /// [`on_selection_removed`](Self::on_selection_removed) if necessary, according to the
    /// [`CollectionChangeState`] returned by those methods.
    ///
    /// Override this method and
    /// [`on_source_collection_change_finished`](Self::on_source_collection_change_finished) to
    /// provide custom handling of source collection changes.
    fn on_source_collection_changed(&self, e: &ModelsChangedArgs<'_, M>) {
        SelectionNodeBase::on_source_collection_changed(self.node_base(), e);
    }

    /// Called when the source collection has finished changing, and all collection changed
    /// handlers have run.
    ///
    /// Override this method to respond to the end of a collection change instead of acting at
    /// the end of [`on_source_collection_changed`](Self::on_source_collection_changed) in order
    /// to ensure that all UI subscribers to the source collection change event have had chance
    /// to run.
    fn on_source_collection_change_finished(&self) {}

    /// Called by [`on_source_collection_changed`](Self::on_source_collection_changed), detailing
    /// the indexes changed by the collection changing.
    ///
    /// `shift_index` is the first index that was shifted; `shift_delta`, if positive, the number
    /// of items inserted, or if negative the number of items removed.
    fn on_indexes_changed(&self, _shift_index: i32, _shift_delta: i32) {}

    /// Called by [`on_source_collection_changed`](Self::on_source_collection_changed), on
    /// collection reset.
    fn on_source_reset(&self);

    /// Called by [`on_source_collection_changed`](Self::on_source_collection_changed), detailing
    /// the items removed by a collection change.
    fn on_selection_removed(&self, _index: i32, _count: i32, _deselected_items: Vec<Option<Rc<M>>>) {}

    /// Called by [`on_source_collection_changed`](Self::on_source_collection_changed), detailing
    /// selection that moved within the source collection.
    ///
    /// `old_start_index` and `new_start_index` are the starting indexes of the move before and
    /// after the change; `old_selected_ranges` and `new_selected_ranges` the ranges representing
    /// the selected indexes before and after the move; `moved_items` the selected items affected
    /// by the move.
    fn on_selection_moved(
        &self,
        _old_start_index: i32,
        _new_start_index: i32,
        _old_selected_ranges: &[IndexRange],
        _new_selected_ranges: &[IndexRange],
        _moved_items: &[Option<Rc<M>>],
    ) {
    }
}

/// Details the results of a collection change on the current selection.
pub struct CollectionChangeState<M> {
    /// The first index that was shifted as a result of the collection changing.
    pub shift_index: i32,
    /// How the indexes after `shift_index` were shifted.
    pub shift_delta: i32,
    /// The items removed by the collection change, if any.
    pub removed_items: Option<Vec<Option<Rc<M>>>>,
    /// The ranges deselected by the collection change, if any.
    pub deselected_ranges: Option<Vec<IndexRange>>,
}

/// Base class for selection models: the state of a node of a selection of models of type `M`.
pub struct SelectionNodeBase<M: TreeModel> {
    derived: Weak<dyn SelectionNodeBaseImpl<M>>,
    listener: Weak<dyn ICollectionChangedListener<M>>,
    source: RefCell<Option<Models<M>>>,
    items_view: RefCell<Option<Rc<TreeDataGridItemsSourceView<M>>>>,
    ranges_enabled: Cell<bool>,
    ranges: RefCell<Option<Rc<Vec<IndexRange>>>>,
}

impl<M: TreeModel> SelectionNodeBase<M> {
    /// The base state of the object deriving from it: `derived` receives the virtual calls, and
    /// `listener` (the same object) is registered as the listener of the changes of the source.
    pub fn new(derived: Weak<dyn SelectionNodeBaseImpl<M>>, listener: Weak<dyn ICollectionChangedListener<M>>) -> Self {
        Self {
            derived,
            listener,
            source: RefCell::new(None),
            items_view: RefCell::new(None),
            ranges_enabled: Cell::new(false),
            ranges: RefCell::new(None),
        }
    }

    fn derived(&self) -> Option<Rc<dyn SelectionNodeBaseImpl<M>>> {
        self.derived.upgrade()
    }

    /// Gets the source collection.
    pub fn source(&self) -> Option<Models<M>> {
        self.source.borrow().clone()
    }

    /// Sets the source collection: the node stops listening to the old one and listens to the new
    /// one through a view of it.
    pub fn set_source(&self, value: Option<Models<M>>) {
        let unchanged = same_source(self.source.borrow().as_ref(), value.as_ref());
        if unchanged {
            return;
        }

        let old_view = self.items_view();
        if let Some(old_view) = old_view {
            old_view.remove_listener(&self.listener);
        }

        let view = value.as_ref().map(|value| TreeDataGridItemsSourceView::get_or_create(Some(value.clone())));
        *self.source.borrow_mut() = value;
        *self.items_view.borrow_mut() = view.clone();

        if let Some(view) = view {
            view.add_listener(self.listener.clone());
        }
    }

    /// Gets a view of the [`source`](Self::source).
    pub fn items_view(&self) -> Option<Rc<TreeDataGridItemsSourceView<M>>> {
        self.items_view.borrow().clone()
    }

    /// Sets the view of the source.
    pub fn set_items_view(&self, value: Option<Rc<TreeDataGridItemsSourceView<M>>>) {
        *self.items_view.borrow_mut() = value;
    }

    /// Gets a value indicating whether range selection is currently enabled for the selection
    /// node.
    pub fn ranges_enabled(&self) -> bool {
        self.ranges_enabled.get()
    }

    /// Sets a value indicating whether range selection is currently enabled for the selection
    /// node; disabling it drops the ranges.
    pub fn set_ranges_enabled(&self, value: bool) {
        if self.ranges_enabled.get() != value {
            self.ranges_enabled.set(value);

            if !value {
                *self.ranges.borrow_mut() = None;
            }
        }
    }

    /// The selected ranges, as a snapshot that later changes to the selection do not affect.
    ///
    /// # Panics
    /// When ranges are not enabled (`InvalidOperationException`).
    pub fn ranges(&self) -> Rc<Vec<IndexRange>> {
        if !self.ranges_enabled() {
            panic!("Ranges not enabled.");
        }

        self.ranges.borrow_mut().get_or_insert_with(Default::default).clone()
    }

    /// `ICollectionChangedListener.PreChanged`: the source collection starts changing.
    pub fn pre_changed(&self, _collection: &Models<M>, _e: &ModelsChangedArgs<'_, M>) {
        if let Some(derived) = self.derived() {
            derived.on_source_collection_change_started();
        }
    }

    /// `ICollectionChangedListener.Changed`: the source collection changed.
    pub fn changed(&self, _collection: &Models<M>, e: &ModelsChangedArgs<'_, M>) {
        if let Some(derived) = self.derived() {
            derived.on_source_collection_changed(e);
        }
    }

    /// `ICollectionChangedListener.PostChanged`: the source collection finished changing.
    pub fn post_changed(&self, _collection: &Models<M>, _e: &ModelsChangedArgs<'_, M>) {
        if let Some(derived) = self.derived() {
            derived.on_source_collection_change_finished();
        }
    }

    /// The base implementation of
    /// [`SelectionNodeBaseImpl::on_source_collection_changed`]: adjusts the ranges to the change
    /// and makes the virtual calls that report it.
    pub fn on_source_collection_changed(&self, e: &ModelsChangedArgs<'_, M>) {
        let Some(derived) = self.derived() else {
            return;
        };

        if !has_known_indexes(e) {
            derived.on_source_reset();
            return;
        }

        let mut shift_delta = 0;
        let mut shift_index = -1;
        let mut removed: Option<Vec<Option<Rc<M>>>> = None;

        match e.action {
            NotifyCollectionChangedAction::Add => {
                let change = self.on_items_added(e.new_starting_index, e.new_items);
                shift_index = change.shift_index;
                shift_delta = change.shift_delta;
            }
            NotifyCollectionChangedAction::Remove => {
                let change = self.on_items_removed(e.old_starting_index, e.old_items);
                shift_index = change.shift_index;
                shift_delta = change.shift_delta;
                removed = change.removed_items;
            }
            NotifyCollectionChangedAction::Replace => {
                let remove_change = self.on_items_removed(e.old_starting_index, e.old_items);
                let add_change = self.on_items_added(e.new_starting_index, e.new_items);
                shift_index = remove_change.shift_index;
                shift_delta = remove_change.shift_delta + add_change.shift_delta;
                removed = remove_change.removed_items;
            }
            NotifyCollectionChangedAction::Move => {
                if e.old_starting_index < 0 {
                    // Handled as a reset.
                    derived.on_source_reset();
                } else {
                    let remove_change = self.on_items_removed(e.old_starting_index, e.old_items);
                    let insert_index = e.new_starting_index;

                    let add_change = self.on_items_added(insert_index, e.new_items);
                    shift_index = remove_change.shift_index;
                    shift_delta = remove_change.shift_delta + add_change.shift_delta;
                    removed = remove_change.removed_items;

                    if let Some(deselected_ranges) = remove_change.deselected_ranges.filter(|r| !r.is_empty()) {
                        let mut moved_ranges = Vec::with_capacity(deselected_ranges.len());

                        for range in &deselected_ranges {
                            let relative_begin = range.begin() - e.old_starting_index;
                            let begin = insert_index + relative_begin;
                            let end = begin + (range.end() - range.begin());
                            moved_ranges.push(IndexRange::new(begin, end));

                            if self.ranges_enabled() {
                                self.commit_select(begin, end);
                            }
                        }

                        let moved_items = match removed.take() {
                            Some(items) if !items.is_empty() => items,
                            _ => Vec::new(),
                        };

                        derived.on_selection_moved(
                            e.old_starting_index,
                            insert_index,
                            &deselected_ranges,
                            &moved_ranges,
                            &moved_items,
                        );

                        removed = None;
                    }
                }
            }
            NotifyCollectionChangedAction::Reset => derived.on_source_reset(),
        }

        if shift_delta != 0 {
            derived.on_indexes_changed(shift_index, shift_delta);
        }

        if let Some(removed) = removed {
            derived.on_selection_removed(shift_index, -shift_delta, removed);
        }
    }

    /// If ranges are enabled, adds the specified inclusive range to the selection. Returns the
    /// number of items selected.
    pub fn commit_select(&self, begin: i32, end: i32) -> i32 {
        if self.ranges_enabled() {
            let mut ranges = self.ranges.borrow_mut();
            let ranges = Rc::make_mut(ranges.get_or_insert_with(Default::default));
            return IndexRange::add(ranges, IndexRange::new(begin, end), None);
        }

        0
    }

    /// If ranges are enabled, removes the specified inclusive range from the selection. Returns
    /// the number of items deselected.
    pub fn commit_deselect(&self, begin: i32, end: i32) -> i32 {
        if self.ranges_enabled() {
            let mut ranges = self.ranges.borrow_mut();
            let ranges = Rc::make_mut(ranges.get_or_insert_with(Default::default));
            return IndexRange::remove(Some(ranges), IndexRange::new(begin, end), None);
        }

        0
    }

    /// Called by the handling of a collection change when items are added to the source
    /// collection: adjusts the selected ranges, assigning new indexes.
    pub fn on_items_added(&self, index: i32, items: &[Rc<M>]) -> CollectionChangeState<M> {
        let count = items.len() as i32;
        let mut shifted = false;

        if let Some(ranges) = self.ranges.borrow_mut().as_mut() {
            let ranges = Rc::make_mut(ranges);
            let mut to_add: Option<Vec<IndexRange>> = None;

            for i in 0..ranges.len() {
                let range = ranges[i];

                // The range is after the inserted items, need to shift the range right
                if range.end() >= index {
                    let mut begin = range.begin();

                    // If the index left of newIndex is inside the range,
                    // Split the range and remember the left piece to add later
                    if range.contains(index - 1) {
                        let (before, _) = range.split(index - 1);
                        to_add.get_or_insert_with(Vec::new).push(before);
                        begin = index;
                    }

                    // Shift the range to the right
                    ranges[i] = IndexRange::new(begin + count, range.end() + count);
                    shifted = true;
                }
            }

            if let Some(to_add) = to_add {
                for range in to_add {
                    IndexRange::add(ranges, range, None);
                }
            }
        }

        CollectionChangeState {
            shift_index: index,
            shift_delta: if shifted { count } else { 0 },
            removed_items: None,
            deselected_ranges: None,
        }
    }

    /// Called by the handling of a collection change when items are removed from the source
    /// collection: adjusts the selected ranges, assigning new indexes, and reports the selected
    /// items and ranges that were removed.
    pub fn on_items_removed(&self, index: i32, items: &[Rc<M>]) -> CollectionChangeState<M> {
        let count = items.len() as i32;
        let removed_range = IndexRange::new(index, index + count - 1);
        let mut shifted = false;
        let mut removed: Option<Vec<Option<Rc<M>>>> = None;
        let mut deselected: Option<Vec<IndexRange>> = None;

        if let Some(ranges) = self.ranges.borrow_mut().as_mut() {
            let ranges = Rc::make_mut(ranges);
            let mut deselected_ranges = Vec::new();

            if IndexRange::remove(Some(ranges), removed_range, Some(&mut deselected_ranges)) > 0 {
                let mut removed_items = Vec::new();

                for range in &deselected_ranges {
                    for i in range.begin()..=range.end() {
                        removed_items.push(items.get((i - index) as usize).cloned());
                    }
                }

                removed = Some(removed_items);
            }

            for existing in ranges.iter_mut() {
                if existing.end() > removed_range.begin() {
                    *existing = IndexRange::new(existing.begin() - count, existing.end() - count);
                    shifted = true;
                }
            }

            deselected = Some(deselected_ranges);
        }

        CollectionChangeState {
            shift_index: index,
            shift_delta: if shifted { -count } else { 0 },
            removed_items: removed,
            deselected_ranges: deselected.filter(|ranges| !ranges.is_empty()),
        }
    }

    /// Moves the selected ranges of a move of `count` items from `old_index` to `new_index`. As
    /// the reference, the list of changes it returns is always empty.
    pub fn on_items_moved(&self, old_index: i32, new_index: i32, count: i32) -> Vec<CollectionChangeState<M>> {
        let mut selected_items_moved = false;
        let mut unselected_items_moved = false;

        if let Some(ranges) = self.ranges.borrow_mut().as_mut() {
            let ranges = Rc::make_mut(ranges);
            let removed_range = IndexRange::new(old_index, old_index + count - 1);
            let mut moved_ranges = Vec::new();

            if IndexRange::remove(Some(ranges), removed_range, Some(&mut moved_ranges)) > 0 {
                for range in &moved_ranges {
                    let insert_range =
                        IndexRange::new(range.begin() + (new_index - old_index), range.end() + (new_index - old_index));
                    IndexRange::add(ranges, insert_range, None);
                    selected_items_moved = true;
                }
            }

            for existing in ranges.iter_mut() {
                if existing.begin() >= old_index && existing.end() < new_index {
                    *existing = IndexRange::new(existing.begin() - count, existing.end() - count);
                    unselected_items_moved = true;
                }
            }
        }

        // The reference returns an empty list either way.
        let _ = selected_items_moved || unselected_items_moved;
        Vec::new()
    }
}
