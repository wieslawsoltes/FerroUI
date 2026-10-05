use super::index_range::IndexRange;
use crate::items_source::{unbox_item, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource, ItemsView};
use crate::items_source_view::{ItemsSourceView, ItemsSourceViewOf};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::{BoxedValue, PropertyValue};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The overridable members of [`SelectionNodeBase`].
///
/// The selection classes are plain shared structs, not part of the class
/// model, so their virtual members are modelled explicitly: the base keeps
/// the state and the base implementations (the inherent methods of the same
/// names on [`SelectionNodeBase`]), and holds a weak reference to the
/// object deriving from it, through which every virtual call is made. See
/// the module documentation of [`selection`](super).
pub trait SelectionNodeBaseImpl<T: PropertyValue> {
    /// Called when the source collection starts changing.
    fn on_source_collection_change_started(&self);

    /// Called when the source collection changes.
    fn on_source_collection_changed(&self, e: &ItemsChangedEventArgs<'_>);

    /// Called when the source collection has finished changing, and all
    /// collection changed handlers have run.
    fn on_source_collection_change_finished(&self);

    /// Called by the handling of a collection change, detailing the indexes
    /// changed by the collection changing.
    fn on_indexes_changed(&self, shift_index: i32, shift_delta: i32);

    /// Called by the handling of a collection change, on collection reset.
    fn on_source_reset(&self);

    /// Called by the handling of a collection change, detailing the items
    /// removed by a collection change.
    fn on_selection_removed(&self, index: i32, count: i32, deselected_items: Vec<Option<T>>);

    /// Called by the handling of a collection change when items are added
    /// to the source collection.
    fn on_items_added(&self, index: i32, items: ItemsView<'_>) -> CollectionChangeState<T>;

    /// Called by the handling of a collection change when items are removed
    /// from the source collection.
    fn on_items_removed(&self, index: i32, items: ItemsView<'_>) -> CollectionChangeState<T>;

    /// Whether a collection change can be applied to the selection.
    fn is_valid_collection_change(&self, e: &ItemsChangedEventArgs<'_>) -> bool;
}

/// Details the results of a collection change on the current selection.
pub struct CollectionChangeState<T> {
    /// The first index that was shifted as a result of the collection
    /// changing.
    pub shift_index: i32,
    /// How the indexes after `shift_index` were shifted.
    pub shift_delta: i32,
    /// The items removed by the collection change, if any.
    pub removed_items: Option<Vec<Option<T>>>,
}

/// Reads an untyped item as a `T`: `None` for a null item. Panics if the
/// item is of another type.
pub(crate) fn cast_item<T: PropertyValue>(item: &Option<BoxedValue>) -> Option<T> {
    item.as_ref()?;

    match unbox_item::<T>(item) {
        Some(value) => Some(value),
        None => panic!("Unable to cast the item to type '{}'.", std::any::type_name::<T>()),
    }
}

/// Base class for selection models.
///
/// `T` is the type of the element being selected.
pub struct SelectionNodeBase<T: PropertyValue> {
    derived: Weak<dyn SelectionNodeBaseImpl<T>>,
    source: RefCell<Option<ItemsSource>>,
    items_view: RefCell<Option<ItemsSourceViewOf<T>>>,
    subscriptions: Cell<(u64, u64, u64)>,
    ranges_enabled: Cell<bool>,
    ranges: RefCell<Option<Rc<Vec<IndexRange>>>>,
}

impl<T: PropertyValue> SelectionNodeBase<T> {
    /// Creates the base state of the object deriving from it; `derived`
    /// receives the virtual calls.
    pub fn new(derived: Weak<dyn SelectionNodeBaseImpl<T>>) -> Self {
        Self {
            derived,
            source: RefCell::new(None),
            items_view: RefCell::new(None),
            subscriptions: Cell::new((0, 0, 0)),
            ranges_enabled: Cell::new(false),
            ranges: RefCell::new(None),
        }
    }

    fn derived(&self) -> Rc<dyn SelectionNodeBaseImpl<T>> {
        self.derived.upgrade().expect("the selection node is alive")
    }

    /// Gets the source collection.
    pub fn source(&self) -> Option<ItemsSource> {
        self.source.borrow().clone()
    }

    /// Sets the source collection.
    pub fn set_source(&self, value: Option<ItemsSource>) {
        if *self.source.borrow() != value {
            if let Some(view) = self.items_view() {
                let (pre, changed, post) = self.subscriptions.get();
                view.remove_pre_collection_changed(pre);
                view.remove_collection_changed(changed);
                view.remove_post_collection_changed(post);
            }

            let view = value.as_ref().map(|value| ItemsSourceView::get_or_create_of::<T>(Some(value)));
            *self.source.borrow_mut() = value;
            *self.items_view.borrow_mut() = view.clone();

            if let Some(view) = view {
                let derived = self.derived.clone();
                let on_pre_changed: Rc<ItemsChangedHandler> = Rc::new(move |_: &ItemsChangedEventArgs<'_>| {
                    if let Some(derived) = derived.upgrade() {
                        derived.on_source_collection_change_started();
                    }
                });
                let derived = self.derived.clone();
                let on_changed: Rc<ItemsChangedHandler> = Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
                    if let Some(derived) = derived.upgrade() {
                        derived.on_source_collection_changed(e);
                    }
                });
                let derived = self.derived.clone();
                let on_post_changed: Rc<ItemsChangedHandler> = Rc::new(move |_: &ItemsChangedEventArgs<'_>| {
                    if let Some(derived) = derived.upgrade() {
                        derived.on_source_collection_change_finished();
                    }
                });

                self.subscriptions.set((
                    view.add_pre_collection_changed(on_pre_changed),
                    view.add_collection_changed(on_changed),
                    view.add_post_collection_changed(on_post_changed),
                ));
            }
        }
    }

    /// Gets a view of the [`source`](Self::source).
    pub fn items_view(&self) -> Option<ItemsSourceViewOf<T>> {
        self.items_view.borrow().clone()
    }

    /// Sets the view of the source.
    pub fn set_items_view(&self, value: Option<ItemsSourceViewOf<T>>) {
        *self.items_view.borrow_mut() = value;
    }

    /// Gets a value indicating whether range selection is currently enabled
    /// for the selection node.
    pub fn ranges_enabled(&self) -> bool {
        self.ranges_enabled.get()
    }

    /// Sets a value indicating whether range selection is currently enabled
    /// for the selection node.
    pub fn set_ranges_enabled(&self, value: bool) {
        if self.ranges_enabled.get() != value {
            self.ranges_enabled.set(value);

            if !value {
                *self.ranges.borrow_mut() = None;
            }
        }
    }

    /// The selected ranges, as a snapshot that later changes to the
    /// selection do not affect.
    pub(crate) fn ranges(&self) -> Rc<Vec<IndexRange>> {
        if !self.ranges_enabled() {
            panic!("Ranges not enabled.");
        }

        self.ranges.borrow_mut().get_or_insert_with(Default::default).clone()
    }

    /// Called when the source collection starts changing.
    pub fn on_source_collection_change_started(&self) {}

    /// Called when the [`source`](Self::source) collection changes.
    ///
    /// The implementation here calls `on_items_added` and
    /// `on_items_removed` in order to calculate how the collection change
    /// affects the currently selected items. It then calls
    /// `on_indexes_changed` and `on_selection_removed` if necessary,
    /// according to the [`CollectionChangeState`] returned by those methods.
    pub fn on_source_collection_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        let derived = self.derived();
        let mut shift_delta = 0;
        let mut shift_index = -1;
        let mut removed: Option<Vec<Option<T>>> = None;

        if !derived.is_valid_collection_change(e) {
            return;
        }

        let action = match e.action {
            NotifyCollectionChangedAction::Replace | NotifyCollectionChangedAction::Move
                if e.old_starting_index < 0 =>
            {
                NotifyCollectionChangedAction::Reset
            }
            action => action,
        };

        match action {
            NotifyCollectionChangedAction::Add => {
                let change = derived.on_items_added(e.new_starting_index, e.new_items);
                shift_index = change.shift_index;
                shift_delta = change.shift_delta;
            }
            NotifyCollectionChangedAction::Remove => {
                let change = derived.on_items_removed(e.old_starting_index, e.old_items);
                shift_index = change.shift_index;
                shift_delta = change.shift_delta;
                removed = change.removed_items;
            }
            NotifyCollectionChangedAction::Replace => {
                let remove_change = derived.on_items_removed(e.old_starting_index, e.old_items);
                let add_change = derived.on_items_added(e.new_starting_index, e.new_items);
                shift_index = remove_change.shift_index;
                shift_delta = remove_change.shift_delta + add_change.shift_delta;
                removed = remove_change.removed_items;
            }
            NotifyCollectionChangedAction::Move => {
                let remove_change = derived.on_items_removed(e.old_starting_index, e.old_items);
                let mut insert_index = e.new_starting_index;

                if e.new_starting_index > e.old_starting_index {
                    insert_index -= e.old_items.len() as i32 - 1;
                }

                let add_change = derived.on_items_added(insert_index, e.new_items);
                shift_index = remove_change.shift_index;
                shift_delta = remove_change.shift_delta + add_change.shift_delta;
                removed = remove_change.removed_items;
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

    /// Called when the source collection has finished changing, and all
    /// collection changed handlers have run.
    pub fn on_source_collection_change_finished(&self) {}

    /// Called by the handling of a collection change, detailing the indexes
    /// changed by the collection changing.
    pub fn on_indexes_changed(&self, _shift_index: i32, _shift_delta: i32) {}

    /// Called by the handling of a collection change, detailing the items
    /// removed by a collection change.
    pub fn on_selection_removed(&self, _index: i32, _count: i32, _deselected_items: Vec<Option<T>>) {}

    /// If ranges are enabled, adds the specified inclusive range to the
    /// selection. Returns the number of items selected.
    pub fn commit_select(&self, begin: i32, end: i32) -> i32 {
        if self.ranges_enabled() {
            let mut ranges = self.ranges.borrow_mut();
            let ranges = Rc::make_mut(ranges.get_or_insert_with(Default::default));
            return IndexRange::add(ranges, IndexRange::new(begin, end), None);
        }

        0
    }

    /// If ranges are enabled, removes the specified inclusive range from
    /// the selection. Returns the number of items deselected.
    pub fn commit_deselect(&self, begin: i32, end: i32) -> i32 {
        if self.ranges_enabled() {
            let mut ranges = self.ranges.borrow_mut();
            let ranges = Rc::make_mut(ranges.get_or_insert_with(Default::default));
            return IndexRange::remove(Some(ranges), IndexRange::new(begin, end), None);
        }

        0
    }

    /// Called by the handling of a collection change when items are added
    /// to the source collection.
    ///
    /// The implementation here adjusts the selected ranges, assigning new
    /// indexes.
    pub fn on_items_added(&self, index: i32, items: ItemsView<'_>) -> CollectionChangeState<T> {
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

        CollectionChangeState { shift_index: index, shift_delta: if shifted { count } else { 0 }, removed_items: None }
    }

    /// Called by the handling of a collection change when items are removed
    /// from the source collection.
    ///
    /// The implementation here adjusts the selected ranges, assigning new
    /// indexes.
    pub fn on_items_removed(&self, index: i32, items: ItemsView<'_>) -> CollectionChangeState<T> {
        let count = items.len() as i32;
        let removed_range = IndexRange::new(index, index + count - 1);
        let mut shifted = false;
        let mut removed: Option<Vec<Option<T>>> = None;

        if let Some(ranges) = self.ranges.borrow_mut().as_mut() {
            let ranges = Rc::make_mut(ranges);
            let mut deselected = Vec::new();

            if IndexRange::remove(Some(ranges), removed_range, Some(&mut deselected)) > 0 {
                let mut removed_items = Vec::new();

                for range in &deselected {
                    for i in range.begin()..=range.end() {
                        removed_items.push(cast_item::<T>(&items.get((i - index) as usize)));
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
        }

        CollectionChangeState {
            shift_index: index,
            shift_delta: if shifted { -count } else { 0 },
            removed_items: removed,
        }
    }

    /// Whether a collection change can be applied to the selection.
    pub fn is_valid_collection_change(&self, e: &ItemsChangedEventArgs<'_>) -> bool {
        // If the selection is modified in a CollectionChanged handler before the selection
        // model's CollectionChanged handler has had chance to run then we can end up with
        // a selected index that refers to the *new* state of the Source intermixed with
        // indexes that reference an old state of the source.
        //
        // There's not much we can do in this situation, so detect whether shifting the
        // current selected indexes would result in an invalid index in the source, and if
        // so bail.
        //
        // See unit test Handles_Selection_Made_In_CollectionChanged for more details.
        if let Some(items_view) = self.items_view() {
            if self.ranges_enabled() && e.action == NotifyCollectionChangedAction::Add {
                let ranges = self.ranges();

                if let Some(last) = ranges.last() {
                    let last_index = last.end();

                    if e.new_starting_index <= last_index {
                        return last_index + (e.new_items.len() as i32) < items_view.count() as i32;
                    }
                }
            }
        }

        true
    }
}
