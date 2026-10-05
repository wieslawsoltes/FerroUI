use super::i_selection_model::{BatchUpdateOperation, ISelectionModel};
use super::index_range::IndexRange;
use super::read_only_selection_list_base::{IReadOnlySelectionList, ReadOnlySelectionList};
use super::selected_indexes::SelectedIndexes;
use super::selected_items::{SelectedItems, SelectedItemsUntyped};
use super::selection_model_indexes_changed_event_args::SelectionModelIndexesChangedEventArgs;
use super::selection_model_selection_changed_event_args::{
    SelectionModelSelectionChangedEventArgs, SelectionModelSelectionChangedEventArgsOf,
};
use super::selection_node_base::{cast_item, CollectionChangeState, SelectionNodeBase, SelectionNodeBaseImpl};
use crate::items_source::{box_item, unbox_item, ItemsChangedEventArgs, ItemsSource, ItemsView};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, PropertyValue};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Runs a closure when dropped: the `finally` block of a `try`.
pub(crate) struct Finally<F: FnMut()>(F);

impl<F: FnMut()> Drop for Finally<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

pub(crate) fn finally<F: FnMut()>(f: F) -> Finally<F> {
    Finally(f)
}

/// Boxes a typed, nullable item as an untyped one.
fn box_nullable<T: PropertyValue>(item: &Option<T>) -> Option<BoxedValue> {
    match item {
        Some(item) => box_item(item),
        None => None,
    }
}

/// The members of [`SelectionModel`] that a type deriving from it can
/// override.
///
/// Every method receives the model and defaults to the model's own
/// implementation, which is the `base_*` method of the same name: an
/// override calls it where the managed code calls the base implementation.
/// A deriving type owns its model, implements this trait and registers
/// itself with [`SelectionModel::set_derived`]; every virtual call the
/// model makes (from its public members and from the handling of source
/// collection changes) then goes through the override. See the module
/// documentation of [`selection`](super).
pub trait SelectionModelImpl<T: PropertyValue> {
    fn set_source(&self, model: &SelectionModel<T>, value: Option<ItemsSource>) {
        model.base_set_source(value);
    }

    fn on_source_collection_change_started(&self, model: &SelectionModel<T>) {
        model.base_on_source_collection_change_started();
    }

    fn on_source_collection_changed(&self, model: &SelectionModel<T>, e: &ItemsChangedEventArgs<'_>) {
        model.base_on_source_collection_changed(e);
    }

    fn on_source_collection_change_finished(&self, model: &SelectionModel<T>) {
        model.base_on_source_collection_change_finished();
    }

    fn on_indexes_changed(&self, model: &SelectionModel<T>, shift_index: i32, shift_delta: i32) {
        model.base_on_indexes_changed(shift_index, shift_delta);
    }

    fn on_source_reset(&self, model: &SelectionModel<T>) {
        model.base_on_source_reset();
    }

    fn on_selection_removed(&self, model: &SelectionModel<T>, index: i32, count: i32, deselected_items: Vec<Option<T>>) {
        model.base_on_selection_removed(index, count, deselected_items);
    }

    fn on_items_added(
        &self,
        model: &SelectionModel<T>,
        index: i32,
        items: ItemsView<'_>,
    ) -> CollectionChangeState<T> {
        model.base_on_items_added(index, items)
    }

    fn on_items_removed(
        &self,
        model: &SelectionModel<T>,
        index: i32,
        items: ItemsView<'_>,
    ) -> CollectionChangeState<T> {
        model.base_on_items_removed(index, items)
    }

    fn is_valid_collection_change(&self, model: &SelectionModel<T>, e: &ItemsChangedEventArgs<'_>) -> bool {
        model.base_is_valid_collection_change(e)
    }
}

/// The pending changes of a batch update.
///
/// The operation is shared: a handler invoked while it is being committed
/// can start an update, which continues the same operation.
pub(crate) struct Operation<T> {
    pub update_count: Cell<i32>,
    pub is_source_update: Cell<bool>,
    pub skip_lost_selection: Cell<bool>,
    pub anchor_index: Cell<i32>,
    pub selected_index: Cell<i32>,
    pub selected_ranges: RefCell<Option<Rc<Vec<IndexRange>>>>,
    pub deselected_ranges: RefCell<Option<Rc<Vec<IndexRange>>>>,
    pub deselected_items: RefCell<Option<Rc<ReadOnlySelectionList<Option<T>>>>>,
}

impl<T: PropertyValue> Operation<T> {
    fn new(owner: &SelectionModel<T>) -> Self {
        Self {
            update_count: Cell::new(0),
            is_source_update: Cell::new(false),
            skip_lost_selection: Cell::new(false),
            anchor_index: Cell::new(owner.anchor_index()),
            selected_index: Cell::new(owner.selected_index()),
            selected_ranges: RefCell::new(None),
            deselected_ranges: RefCell::new(None),
            deselected_items: RefCell::new(None),
        }
    }
}

/// The items selected before a source is assigned, read as `T`.
struct InitSelectedItems<T: PropertyValue> {
    items: ItemsSource,
    marker: std::marker::PhantomData<fn() -> T>,
}

impl<T: PropertyValue> IReadOnlySelectionList<Option<T>> for InitSelectedItems<T> {
    fn count(&self) -> usize {
        self.items.count()
    }

    fn get(&self, index: usize) -> Option<T> {
        if index >= self.items.count() {
            panic!("The index was out of range.");
        }

        cast_item::<T>(&self.items.get_at(index))
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Option<T>> + '_> {
        Box::new(self.items.iter().map(|item| cast_item::<T>(&item)))
    }
}

/// A selection model: the selection of the items of a source collection of
/// items of type `T`. The untyped model is `SelectionModel<BoxedValue>`.
pub struct SelectionModel<T: PropertyValue> {
    base: SelectionNodeBase<T>,
    this: Weak<SelectionModel<T>>,
    derived: RefCell<Option<Weak<dyn SelectionModelImpl<T>>>>,
    single_select: Cell<bool>,
    anchor_index: Cell<i32>,
    selected_index: Cell<i32>,
    operation: RefCell<Option<Rc<Operation<T>>>>,
    selected_indexes: RefCell<Option<Rc<SelectedIndexes<T>>>>,
    selected_items: RefCell<Option<Rc<SelectedItems<T>>>>,
    selected_items_untyped: RefCell<Option<Rc<SelectedItemsUntyped<T>>>>,
    untyped_selection_changed: HandlerList<dyn Fn(&dyn SelectionModelSelectionChangedEventArgs)>,
    init_selected_items: RefCell<Option<ItemsSource>>,
    is_source_collection_changing: Cell<bool>,
    indexes_changed: HandlerList<dyn Fn(&SelectionModelIndexesChangedEventArgs)>,
    selection_changed: HandlerList<dyn Fn(&SelectionModelSelectionChangedEventArgsOf<T>)>,
    lost_selection: HandlerList<dyn Fn()>,
    source_reset: HandlerList<dyn Fn()>,
    property_changed: Event<str>,
}

impl<T: PropertyValue> SelectionModel<T> {
    /// Creates a selection model without a source.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| {
            let derived: Weak<dyn SelectionNodeBaseImpl<T>> = this.clone();
            Self {
                base: SelectionNodeBase::new(derived),
                this: this.clone(),
                derived: RefCell::new(None),
                single_select: Cell::new(true),
                anchor_index: Cell::new(-1),
                selected_index: Cell::new(-1),
                operation: RefCell::new(None),
                selected_indexes: RefCell::new(None),
                selected_items: RefCell::new(None),
                selected_items_untyped: RefCell::new(None),
                untyped_selection_changed: HandlerList::new(),
                init_selected_items: RefCell::new(None),
                is_source_collection_changing: Cell::new(false),
                indexes_changed: HandlerList::new(),
                selection_changed: HandlerList::new(),
                lost_selection: HandlerList::new(),
                source_reset: HandlerList::new(),
                property_changed: Event::new(),
            }
        })
    }

    /// Creates a selection model for a source.
    pub fn with_source(source: Option<ItemsSource>) -> Rc<Self> {
        let result = Self::new();
        result.set_source(source);
        result
    }

    /// Registers the object deriving from this model, which receives the
    /// virtual calls (see [`SelectionModelImpl`]).
    pub fn set_derived(&self, derived: Weak<dyn SelectionModelImpl<T>>) {
        *self.derived.borrow_mut() = Some(derived);
    }

    fn derived(&self) -> Option<Rc<dyn SelectionModelImpl<T>>> {
        self.derived.borrow().as_ref().and_then(Weak::upgrade)
    }

    /// The base state of the model.
    pub fn node(&self) -> &SelectionNodeBase<T> {
        &self.base
    }

    pub fn source(&self) -> Option<ItemsSource> {
        self.base.source()
    }

    pub fn set_source(&self, value: Option<ItemsSource>) {
        match self.derived() {
            Some(derived) => derived.set_source(self, value),
            None => self.base_set_source(value),
        }
    }

    pub fn single_select(&self) -> bool {
        self.single_select.get()
    }

    pub fn set_single_select(&self, value: bool) {
        if self.single_select.get() != value {
            if value {
                self.begin_batch_update();
                let selected_index = self.selected_index();
                self.clear();
                self.set_selected_index(selected_index);
                self.end_batch_update();
            }

            self.single_select.set(value);
            self.base.set_ranges_enabled(!value);

            if self.base.ranges_enabled() && self.selected_index.get() >= 0 {
                self.base.commit_select(self.selected_index.get(), self.selected_index.get());
            }

            self.raise_property_changed("SingleSelect");
        }
    }

    pub fn selected_index(&self) -> i32 {
        self.selected_index.get()
    }

    pub fn set_selected_index(&self, value: i32) {
        if let Some(operation) = self.operation() {
            // An operation is in the process of being committed. In this case, if the new
            // value for SelectedIndex is unchanged then we need to ignore it. It could be
            // the result of a two-way binding to SelectedIndex writing back to the
            // property.
            if operation.update_count.get() == 0 && value == self.selected_index.get() {
                return;
            }
        }

        self.begin_batch_update();
        self.clear();
        self.select(value);
        self.end_batch_update();
    }

    /// The selected indexes: a view of the selection, always the same list.
    pub fn selected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>> {
        self.selected_indexes
            .borrow_mut()
            .get_or_insert_with(|| Rc::new(SelectedIndexes::new(self.this.clone())))
            .clone()
    }

    pub fn selected_item(&self) -> Option<T> {
        if self.base.items_view().is_some() {
            return self.get_item_at(self.selected_index.get());
        }

        let init_selected_items = self.init_selected_items.borrow().clone();

        if let Some(items) = init_selected_items {
            if items.count() > 0 {
                return cast_item::<T>(&items.get_at(0));
            }
        }

        None
    }

    pub fn set_selected_item(&self, value: Option<T>) {
        if let Some(items_view) = self.base.items_view() {
            self.set_selected_index(items_view.index_of(&box_nullable(&value)));
        } else {
            self.clear();
            self.set_init_selected_items(ItemsSource::from_items([box_nullable(&value)]));
        }
    }

    /// The selected items: a view of the selection. Before a source is
    /// assigned, the items selected by item.
    pub fn selected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<T>>> {
        if self.base.items_view().is_none() {
            let init_selected_items = self.init_selected_items.borrow().clone();

            if let Some(items) = init_selected_items {
                return Rc::new(InitSelectedItems::<T> { items, marker: std::marker::PhantomData });
            }
        }

        self.selected_items.borrow_mut().get_or_insert_with(|| Rc::new(SelectedItems::new(self.this.clone()))).clone()
    }

    pub fn anchor_index(&self) -> i32 {
        self.anchor_index.get()
    }

    pub fn set_anchor_index(&self, value: i32) {
        self.begin_batch_update();
        let index = self.coerce_index(value);
        self.current_operation().anchor_index.set(index);
        self.end_batch_update();
    }

    /// The number of selected items.
    pub fn count(&self) -> usize {
        if self.single_select() {
            if self.selected_index.get() >= 0 {
                1
            } else {
                0
            }
        } else {
            IndexRange::get_count(&self.base.ranges()).max(0) as usize
        }
    }

    /// Subscribes to the event raised when a change to the source shifts
    /// the indexes of selected items.
    pub fn indexes_changed(
        &self,
        handler: impl Fn(&SelectionModelIndexesChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.indexes_changed.add(Rc::new(handler));
        self.subscription(move |this| {
            this.indexes_changed.remove(token);
        })
    }

    /// Subscribes to the event raised when the selection changes.
    pub fn selection_changed(
        &self,
        handler: impl Fn(&SelectionModelSelectionChangedEventArgsOf<T>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.selection_changed.add(Rc::new(handler));
        self.subscription(move |this| {
            this.selection_changed.remove(token);
        })
    }

    /// Subscribes to the event raised when the selection becomes empty.
    pub fn lost_selection(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.lost_selection.add(Rc::new(handler));
        self.subscription(move |this| {
            this.lost_selection.remove(token);
        })
    }

    /// Subscribes to the event raised when the source is reset.
    pub fn source_reset(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.source_reset.add(Rc::new(handler));
        self.subscription(move |this| {
            this.source_reset.remove(token);
        })
    }

    fn subscription(&self, remove: impl Fn(&SelectionModel<T>) + 'static) -> Rc<dyn IDisposable> {
        let this = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                remove(&this);
            }
        })
    }

    /// Begins a batch update that ends when the returned operation is
    /// disposed.
    pub fn batch_update(&self) -> BatchUpdateOperation {
        BatchUpdateOperation::new(self.this.upgrade().expect("the selection model is alive"))
    }

    pub fn begin_batch_update(&self) {
        let operation =
            self.operation.borrow_mut().get_or_insert_with(|| Rc::new(Operation::new(self))).clone();
        operation.skip_lost_selection.set(false);
        operation.update_count.set(operation.update_count.get() + 1);
    }

    pub fn end_batch_update(&self) {
        let operation = match self.operation() {
            Some(operation) if operation.update_count.get() != 0 => operation,
            _ => panic!("No batch update in progress."),
        };

        operation.update_count.set(operation.update_count.get() - 1);

        if operation.update_count.get() == 0 {
            // If the collection is currently changing, commit the update when the
            // collection change finishes.
            if !self.is_source_collection_changing.get() {
                self.commit_operation(&operation, true);
            }
        }
    }

    pub fn is_selected(&self, index: i32) -> bool {
        if index < 0 {
            false
        } else if self.single_select() {
            self.selected_index.get() == index
        } else {
            IndexRange::contains_in(Some(&self.base.ranges()), index)
        }
    }

    pub fn select(&self, index: i32) {
        self.select_range_core(index, index, false, true);
    }

    pub fn deselect(&self, index: i32) {
        self.deselect_range(index, index);
    }

    pub fn select_range(&self, start: i32, end: i32) {
        self.select_range_core(start, end, false, false);
    }

    pub fn deselect_range(&self, start: i32, end: i32) {
        self.begin_batch_update();
        let o = self.current_operation();
        let range = IndexRange::new(start.max(0), end);

        if self.base.ranges_enabled() {
            let mut selected = (*self.base.ranges()).clone();
            let mut deselected = Vec::new();
            let mut operation_deselected = Vec::new();

            {
                let mut deselected_ranges = o.deselected_ranges.borrow_mut();
                let deselected_ranges = Rc::make_mut(deselected_ranges.get_or_insert_with(Default::default));
                let mut selected_ranges = o.selected_ranges.borrow_mut();
                IndexRange::remove(selected_ranges.as_mut().map(Rc::make_mut), range, Some(&mut operation_deselected));
                IndexRange::remove(Some(&mut selected), range, Some(&mut deselected));
                IndexRange::add_ranges(deselected_ranges, &deselected, None);
            }

            if IndexRange::contains_in(Some(&deselected), o.selected_index.get())
                || IndexRange::contains_in(Some(&operation_deselected), o.selected_index.get())
            {
                o.selected_index.set(self.get_first_selected_index_from_ranges(Some(&deselected)));
            }
        } else if range.contains(self.selected_index.get()) {
            o.selected_index.set(-1);
        }

        *self.init_selected_items.borrow_mut() = None;
        self.end_batch_update();
    }

    pub fn select_all(&self) {
        self.select_range(0, i32::MAX);
    }

    pub fn clear(&self) {
        self.deselect_range(0, i32::MAX);
    }

    /// Raises the property changed event.
    pub fn raise_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }

    /// The implementation of [`set_source`](Self::set_source) of this type.
    pub fn base_set_source(&self, value: Option<ItemsSource>) {
        if self.base.source() != value {
            if self.operation().is_some_and(|operation| operation.update_count.get() > 0) {
                panic!("Cannot change source while update is in progress.");
            }

            if self.base.source().is_some() && value.is_some() {
                self.begin_batch_update();
                self.current_operation().skip_lost_selection.set(true);
                self.clear();
                self.end_batch_update();
            }

            self.base.set_source(value);

            self.begin_batch_update();
            let operation = self.current_operation();
            operation.is_source_update.set(true);

            let init_selected_items = self.init_selected_items.borrow().clone();

            match (init_selected_items, self.base.items_view()) {
                (Some(init_selected_items), Some(items_view)) => {
                    for i in init_selected_items.to_vec() {
                        cast_item::<T>(&i);
                        self.select(items_view.index_of(&i));
                    }

                    *self.init_selected_items.borrow_mut() = None;
                }
                _ => self.trim_invalid_selections(&operation),
            }

            self.raise_property_changed("Source");
            self.end_batch_update();
        }
    }

    /// The implementation of `on_indexes_changed` of this type.
    pub fn base_on_indexes_changed(&self, shift_index: i32, shift_delta: i32) {
        if !self.indexes_changed.is_empty() {
            let e = SelectionModelIndexesChangedEventArgs::new(shift_index, shift_delta);

            for (_, handler) in self.indexes_changed.snapshot().iter() {
                handler(&e);
            }
        }
    }

    /// The implementation of `on_source_collection_change_started` of this
    /// type.
    pub fn base_on_source_collection_change_started(&self) {
        self.base.on_source_collection_change_started();
        self.is_source_collection_changing.set(true);
    }

    /// The implementation of `on_source_reset` of this type.
    pub fn base_on_source_reset(&self) {
        self.selected_index.set(-1);
        self.anchor_index.set(-1);
        self.base.commit_deselect(0, i32::MAX);

        // When no handler is registered to handle the reset, the selection
        // may be out of sync.
        Self::raise(&self.source_reset);
    }

    /// The implementation of `on_selection_removed` of this type.
    pub fn base_on_selection_removed(&self, _index: i32, _count: i32, deselected_items: Vec<Option<T>>) {
        // Note: the update is *not* ended. A collection update is still in progress
        // so the operation won't get committed by normal means: we have to commit it manually.
        self.begin_batch_update();
        let operation = self.current_operation();

        *operation.deselected_items.borrow_mut() = Some(Rc::new(ReadOnlySelectionList::new(deselected_items)));

        if self.selected_index.get() == -1 {
            Self::raise(&self.lost_selection);
        }

        // Don't raise PropertyChanged events here as the OnSourceCollectionChanged event that
        // let to this method being called will raise them if necessary.
        self.commit_operation(&operation, false);
    }

    /// The implementation of `on_items_added` of this type.
    pub fn base_on_items_added(&self, index: i32, items: ItemsView<'_>) -> CollectionChangeState<T> {
        let count = items.len() as i32;
        let mut shifted = self.selected_index() >= index;
        let shift_count = if shifted { count } else { 0 };

        self.selected_index.set(self.selected_index.get() + shift_count);
        self.anchor_index.set(self.anchor_index.get() + shift_count);

        let base_result = self.base.on_items_added(index, items);
        shifted |= base_result.shift_delta != 0;

        CollectionChangeState { shift_index: index, shift_delta: if shifted { count } else { 0 }, removed_items: None }
    }

    /// The implementation of `on_items_removed` of this type.
    pub fn base_on_items_removed(&self, index: i32, items: ItemsView<'_>) -> CollectionChangeState<T> {
        let count = items.len() as i32;
        let removed_range = IndexRange::new(index, index + count - 1);
        let mut shifted = false;

        let base_result = self.base.on_items_removed(index, items);
        shifted |= base_result.shift_delta != 0;
        let mut removed = base_result.removed_items;

        if removed_range.contains(self.selected_index()) {
            if self.single_select() {
                removed = Some(vec![cast_item::<T>(&items.get((self.selected_index() - index) as usize))]);
            }

            self.selected_index.set(self.get_first_selected_index_from_ranges(None));
        } else if self.selected_index() >= index {
            self.selected_index.set(self.selected_index.get() - count);
            shifted = true;
        }

        if removed_range.contains(self.anchor_index()) {
            self.anchor_index.set(self.get_first_selected_index_from_ranges(None));
        } else if self.anchor_index() >= index {
            self.anchor_index.set(self.anchor_index.get() - count);
            shifted = true;
        }

        CollectionChangeState {
            shift_index: index,
            shift_delta: if shifted { -count } else { 0 },
            removed_items: removed,
        }
    }

    /// The implementation of `on_source_collection_changed` of this type.
    pub fn base_on_source_collection_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        if self.operation().is_some_and(|operation| operation.update_count.get() > 0) {
            panic!("Source collection was modified during selection update.");
        }

        let old_anchor_index = self.anchor_index.get();
        let old_selected_index = self.selected_index.get();

        self.base.on_source_collection_changed(e);

        if old_selected_index != self.selected_index.get() {
            self.raise_property_changed("SelectedIndex");
        }

        if (e.action == NotifyCollectionChangedAction::Remove && e.old_starting_index <= old_selected_index)
            || (e.action == NotifyCollectionChangedAction::Replace && e.old_starting_index == old_selected_index)
            || (e.action == NotifyCollectionChangedAction::Move && e.old_starting_index == old_selected_index)
            || e.action == NotifyCollectionChangedAction::Reset
        {
            self.raise_property_changed("SelectedItem");
        }

        if old_anchor_index != self.anchor_index.get() {
            self.raise_property_changed("AnchorIndex");
        }
    }

    /// Sets the items selected before a source is assigned. The list is
    /// read when the source is assigned.
    pub(crate) fn set_init_selected_items(&self, items: ItemsSource) {
        if self.source().is_some() {
            panic!("Cannot set init selected items when Source is set.");
        }

        *self.init_selected_items.borrow_mut() = Some(items);
    }

    /// The implementation of `is_valid_collection_change` of this type.
    pub fn base_is_valid_collection_change(&self, e: &ItemsChangedEventArgs<'_>) -> bool {
        if !self.base.is_valid_collection_change(e) {
            return false;
        }

        if let Some(items_view) = self.base.items_view() {
            if e.action == NotifyCollectionChangedAction::Add {
                let count = e.new_items.len() as i32;

                if e.new_starting_index <= self.selected_index.get() {
                    return self.selected_index.get() + count < items_view.count() as i32;
                }

                if e.new_starting_index <= self.anchor_index.get() {
                    return self.anchor_index.get() + count < items_view.count() as i32;
                }
            }
        }

        true
    }

    /// The implementation of `on_source_collection_change_finished` of this
    /// type.
    pub fn base_on_source_collection_change_finished(&self) {
        self.is_source_collection_changing.set(false);

        if let Some(operation) = self.operation() {
            self.commit_operation(&operation, true);
        }
    }

    fn operation(&self) -> Option<Rc<Operation<T>>> {
        self.operation.borrow().clone()
    }

    /// The operation of the batch update in progress.
    fn current_operation(&self) -> Rc<Operation<T>> {
        self.operation().expect("a batch update is in progress")
    }

    fn raise(handlers: &HandlerList<dyn Fn()>) {
        if !handlers.is_empty() {
            for (_, handler) in handlers.snapshot().iter() {
                handler();
            }
        }
    }

    fn get_first_selected_index_from_ranges(&self, except: Option<&[IndexRange]>) -> i32 {
        if self.base.ranges_enabled() {
            let ranges = self.base.ranges();
            let count = IndexRange::get_count(&ranges);
            let mut index = 0;

            while index < count {
                let result = IndexRange::get_at(&ranges, index);
                index += 1;

                if !IndexRange::contains_in(except, result) {
                    return result;
                }
            }
        }

        -1
    }

    fn select_range_core(&self, start: i32, end: i32, force_selected_index: bool, force_anchor_index: bool) {
        if self.single_select() && start != end {
            panic!("Cannot select range with single selection.");
        }

        let range = self.coerce_range(start, end);

        if range.begin() == -1 {
            return;
        }

        self.begin_batch_update();
        let o = self.current_operation();

        if self.base.ranges_enabled() {
            {
                let ranges = self.base.ranges();
                let mut selected_ranges = o.selected_ranges.borrow_mut();
                let selected_ranges = Rc::make_mut(selected_ranges.get_or_insert_with(Default::default));
                let mut deselected_ranges = o.deselected_ranges.borrow_mut();
                IndexRange::remove(deselected_ranges.as_mut().map(Rc::make_mut), range, None);
                IndexRange::add(selected_ranges, range, None);
                IndexRange::remove_ranges(selected_ranges, &ranges, None);
            }

            if o.selected_index.get() == -1 || force_selected_index {
                o.selected_index.set(range.begin());
            }

            if o.anchor_index.get() == -1 || force_anchor_index {
                o.anchor_index.set(range.begin());
            }
        } else {
            o.selected_index.set(start);
            o.anchor_index.set(start);
        }

        *self.init_selected_items.borrow_mut() = None;
        self.end_batch_update();
    }

    fn get_item_at(&self, index: i32) -> Option<T> {
        let items_view = self.base.items_view()?;

        if index < 0 || index >= items_view.count() as i32 {
            return None;
        }

        items_view.get_at(index as usize)
    }

    fn coerce_index(&self, index: i32) -> i32 {
        let mut index = index.max(-1);

        if let Some(items_view) = self.base.items_view() {
            if index >= items_view.count() as i32 {
                index = -1;
            }
        }

        index
    }

    fn coerce_range(&self, start: i32, end: i32) -> IndexRange {
        let max = match self.base.items_view() {
            Some(items_view) => items_view.count() as i32 - 1,
            None => i32::MAX,
        };

        if start > max || (start < 0 && end < 0) {
            return IndexRange::from_index(-1);
        }

        IndexRange::new(start.max(0), end.min(max))
    }

    fn trim_invalid_selections(&self, operation: &Operation<T>) {
        let Some(items_view) = self.base.items_view() else {
            return;
        };

        let max = items_view.count() as i32 - 1;

        if operation.selected_index.get() > max {
            operation.selected_index.set(self.get_first_selected_index_from_ranges(None));
        }

        if operation.anchor_index.get() > max {
            operation.anchor_index.set(self.get_first_selected_index_from_ranges(None));
        }

        if self.base.ranges_enabled() {
            let ranges = self.base.ranges();

            if !ranges.is_empty() {
                let mut selected = (*ranges).clone();

                if max < 0 {
                    *operation.deselected_ranges.borrow_mut() = Some(Rc::new(selected));
                } else {
                    let valid = IndexRange::new(0, max);
                    let mut removed = Vec::new();
                    IndexRange::intersect(&mut selected, valid, Some(&mut removed));
                    *operation.deselected_ranges.borrow_mut() = Some(Rc::new(removed));
                }
            }
        }
    }

    fn commit_operation(&self, operation: &Rc<Operation<T>>, raise_property_changed: bool) {
        let _reset = finally(|| {
            if let Ok(mut operation) = self.operation.try_borrow_mut() {
                *operation = None;
            }
        });

        let old_anchor_index = self.anchor_index.get();
        let old_selected_index = self.selected_index.get();
        let mut indexes_changed = false;

        if operation.selected_index.get() == -1
            && !self.lost_selection.is_empty()
            && !operation.skip_lost_selection.get()
        {
            // Bump the update count so that any selection change made by a LostSelection
            // handler is batched into the current operation. Decrement it again afterwards
            // so that the rest of the commit (in particular the SelectionChanged event) runs
            // with an update count of 0, allowing handlers to change the source.
            operation.update_count.set(operation.update_count.get() + 1);
            Self::raise(&self.lost_selection);
            operation.update_count.set(operation.update_count.get() - 1);
        }

        self.selected_index.set(operation.selected_index.get());
        self.anchor_index.set(operation.anchor_index.get());

        let selected_ranges = operation.selected_ranges.borrow().clone();

        if let Some(selected_ranges) = &selected_ranges {
            for range in selected_ranges.iter() {
                indexes_changed |= self.base.commit_select(range.begin(), range.end()) > 0;
            }
        }

        let deselected_ranges = operation.deselected_ranges.borrow().clone();

        if let Some(deselected_ranges) = &deselected_ranges {
            for range in deselected_ranges.iter() {
                indexes_changed |= self.base.commit_deselect(range.begin(), range.end()) > 0;
            }
        }

        if raise_property_changed {
            if old_selected_index != self.selected_index.get() {
                indexes_changed = true;
                self.raise_property_changed("SelectedIndex");
            }

            if old_selected_index != self.selected_index.get() || operation.is_source_update.get() {
                self.raise_property_changed("SelectedItem");
            }

            if old_anchor_index != self.anchor_index.get() {
                indexes_changed = true;
                self.raise_property_changed("AnchorIndex");
            }

            if indexes_changed {
                self.raise_property_changed("SelectedIndexes");
                let selected_indexes = self.selected_indexes.borrow().clone();

                if let Some(selected_indexes) = selected_indexes {
                    selected_indexes.raise_collection_reset();
                }
            }

            if indexes_changed || operation.is_source_update.get() {
                self.raise_property_changed("SelectedItems");
                let selected_items = self.selected_items.borrow().clone();

                if let Some(selected_items) = selected_items {
                    selected_items.raise_collection_reset();
                }
            }
        }

        if !self.selection_changed.is_empty() || !self.untyped_selection_changed.is_empty() {
            let mut deselected = operation.deselected_ranges.borrow().clone();
            let mut selected = operation.selected_ranges.borrow().clone();

            if self.single_select() && old_selected_index != self.selected_index.get() {
                if old_selected_index != -1 {
                    deselected = Some(Rc::new(vec![IndexRange::from_index(old_selected_index)]));
                }

                if self.selected_index.get() != -1 {
                    selected = Some(Rc::new(vec![IndexRange::from_index(self.selected_index.get())]));
                }
            }

            let operation_deselected_items = operation.deselected_items.borrow().clone();

            if deselected.as_ref().is_some_and(|ranges| !ranges.is_empty())
                || selected.as_ref().is_some_and(|ranges| !ranges.is_empty())
                || operation_deselected_items.is_some()
            {
                // If the operation was caused by Source being updated, then use a null source
                // so that the items will appear as nulls.
                let deselected_source =
                    if operation.is_source_update.get() { None } else { self.base.items_view() };

                // If the operation contains DeselectedItems then we're notifying a source
                // CollectionChanged event. LostFocus may have caused another item to have been
                // selected, but it can't have caused a deselection (as it was called due to
                // selection being lost) so we're ok to discard `deselected` here.
                let deselected_items: Option<Rc<dyn IReadOnlySelectionList<Option<T>>>> =
                    match operation_deselected_items {
                        Some(items) => Some(items),
                        None => SelectedItems::create(deselected.clone(), deselected_source)
                            .map(|items| items as Rc<dyn IReadOnlySelectionList<Option<T>>>),
                    };

                let selected_source = if self.source().is_some() { self.base.items_view() } else { None };

                let e = SelectionModelSelectionChangedEventArgsOf::new(
                    SelectedIndexes::<T>::create(deselected).map(|indexes| indexes as Rc<dyn IReadOnlySelectionList<i32>>),
                    SelectedIndexes::<T>::create(selected.clone())
                        .map(|indexes| indexes as Rc<dyn IReadOnlySelectionList<i32>>),
                    deselected_items,
                    SelectedItems::create(selected, selected_source)
                        .map(|items| items as Rc<dyn IReadOnlySelectionList<Option<T>>>),
                );

                if !self.selection_changed.is_empty() {
                    for (_, handler) in self.selection_changed.snapshot().iter() {
                        handler(&e);
                    }
                }

                if !self.untyped_selection_changed.is_empty() {
                    for (_, handler) in self.untyped_selection_changed.snapshot().iter() {
                        handler(&e);
                    }
                }
            }
        }
    }
}

impl<T: PropertyValue> SelectionNodeBaseImpl<T> for SelectionModel<T> {
    fn on_source_collection_change_started(&self) {
        match self.derived() {
            Some(derived) => derived.on_source_collection_change_started(self),
            None => self.base_on_source_collection_change_started(),
        }
    }

    fn on_source_collection_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        match self.derived() {
            Some(derived) => derived.on_source_collection_changed(self, e),
            None => self.base_on_source_collection_changed(e),
        }
    }

    fn on_source_collection_change_finished(&self) {
        match self.derived() {
            Some(derived) => derived.on_source_collection_change_finished(self),
            None => self.base_on_source_collection_change_finished(),
        }
    }

    fn on_indexes_changed(&self, shift_index: i32, shift_delta: i32) {
        match self.derived() {
            Some(derived) => derived.on_indexes_changed(self, shift_index, shift_delta),
            None => self.base_on_indexes_changed(shift_index, shift_delta),
        }
    }

    fn on_source_reset(&self) {
        match self.derived() {
            Some(derived) => derived.on_source_reset(self),
            None => self.base_on_source_reset(),
        }
    }

    fn on_selection_removed(&self, index: i32, count: i32, deselected_items: Vec<Option<T>>) {
        match self.derived() {
            Some(derived) => derived.on_selection_removed(self, index, count, deselected_items),
            None => self.base_on_selection_removed(index, count, deselected_items),
        }
    }

    fn on_items_added(&self, index: i32, items: ItemsView<'_>) -> CollectionChangeState<T> {
        match self.derived() {
            Some(derived) => derived.on_items_added(self, index, items),
            None => self.base_on_items_added(index, items),
        }
    }

    fn on_items_removed(&self, index: i32, items: ItemsView<'_>) -> CollectionChangeState<T> {
        match self.derived() {
            Some(derived) => derived.on_items_removed(self, index, items),
            None => self.base_on_items_removed(index, items),
        }
    }

    fn is_valid_collection_change(&self, e: &ItemsChangedEventArgs<'_>) -> bool {
        match self.derived() {
            Some(derived) => derived.is_valid_collection_change(self, e),
            None => self.base_is_valid_collection_change(e),
        }
    }
}

impl<T: PropertyValue> INotifyPropertyChanged for SelectionModel<T> {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl<T: PropertyValue> ISelectionModel for SelectionModel<T> {
    fn source(&self) -> Option<ItemsSource> {
        SelectionModel::source(self)
    }

    fn set_source(&self, value: Option<ItemsSource>) {
        SelectionModel::set_source(self, value);
    }

    fn single_select(&self) -> bool {
        SelectionModel::single_select(self)
    }

    fn set_single_select(&self, value: bool) {
        SelectionModel::set_single_select(self, value);
    }

    fn selected_index(&self) -> i32 {
        SelectionModel::selected_index(self)
    }

    fn set_selected_index(&self, value: i32) {
        SelectionModel::set_selected_index(self, value);
    }

    fn selected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>> {
        SelectionModel::selected_indexes(self)
    }

    fn selected_item(&self) -> Option<BoxedValue> {
        box_nullable(&SelectionModel::selected_item(self))
    }

    fn set_selected_item(&self, value: Option<BoxedValue>) {
        match unbox_item::<T>(&value) {
            Some(value) => SelectionModel::set_selected_item(self, Some(value)),
            None => SelectionModel::set_selected_index(self, -1),
        }
    }

    fn selected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<BoxedValue>>> {
        let untyped = self.selected_items_untyped.borrow().clone();

        match untyped {
            Some(untyped) => untyped,
            None => {
                let untyped = Rc::new(SelectedItemsUntyped::new(SelectionModel::selected_items(self)));
                *self.selected_items_untyped.borrow_mut() = Some(untyped.clone());
                untyped
            }
        }
    }

    fn anchor_index(&self) -> i32 {
        SelectionModel::anchor_index(self)
    }

    fn set_anchor_index(&self, value: i32) {
        SelectionModel::set_anchor_index(self, value);
    }

    fn count(&self) -> usize {
        SelectionModel::count(self)
    }

    fn indexes_changed(&self, handler: Rc<dyn Fn(&SelectionModelIndexesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.indexes_changed.add(handler);
        self.subscription(move |this| {
            this.indexes_changed.remove(token);
        })
    }

    fn selection_changed(
        &self,
        handler: Rc<dyn Fn(&dyn SelectionModelSelectionChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        let token = self.untyped_selection_changed.add(handler);
        self.subscription(move |this| {
            this.untyped_selection_changed.remove(token);
        })
    }

    fn lost_selection(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.lost_selection.add(handler);
        self.subscription(move |this| {
            this.lost_selection.remove(token);
        })
    }

    fn source_reset(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.source_reset.add(handler);
        self.subscription(move |this| {
            this.source_reset.remove(token);
        })
    }

    fn begin_batch_update(&self) {
        SelectionModel::begin_batch_update(self);
    }

    fn end_batch_update(&self) {
        SelectionModel::end_batch_update(self);
    }

    fn is_selected(&self, index: i32) -> bool {
        SelectionModel::is_selected(self, index)
    }

    fn select(&self, index: i32) {
        SelectionModel::select(self, index);
    }

    fn deselect(&self, index: i32) {
        SelectionModel::deselect(self, index);
    }

    fn select_range(&self, start: i32, end: i32) {
        SelectionModel::select_range(self, start, end);
    }

    fn deselect_range(&self, start: i32, end: i32) {
        SelectionModel::deselect_range(self, start, end);
    }

    fn select_all(&self) {
        SelectionModel::select_all(self);
    }

    fn clear(&self) {
        SelectionModel::clear(self);
    }
}
