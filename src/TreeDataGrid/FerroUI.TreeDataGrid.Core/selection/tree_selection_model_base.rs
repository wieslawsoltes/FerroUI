//! `Selection/TreeSelectionModelBase.cs`: the selection of the items of a hierarchical source,
//! kept as a tree of [`TreeSelectionNode`]s of selected ranges, with batched updates and the
//! adjustment of the selected, anchor and range anchor indexes to the changes of the source.
//!
//! The reference's abstract and virtual members (`GetChildren`, `TryGetItemAt`,
//! `OnSourceCollectionChangeFinished`, `OnNodeCollectionReset`) are the methods of
//! [`TreeSelectionModelHooks`], implemented by the type deriving from the model (the row
//! selection model takes the children from the expander column of its source). See the trait for
//! the ownership.

use super::i_tree_selection_model::{
    box_model, downcast_models, erase_models, IReadOnlyList, ITreeSelectionModel, ReadOnlyList, UntypedModels,
};
use super::index_range::IndexRange;
use super::index_ranges::IndexRanges;
use super::tree_selected_indexes::TreeSelectedIndexes;
use super::tree_selected_items::TreeSelectedItems;
use super::tree_selection_changed_items::TreeSelectionChangedItems;
use super::tree_selection_model_indexes_changed_event_args::TreeSelectionModelIndexesChangedEventArgs;
use super::tree_selection_model_selection_changed_event_args::{
    TreeSelectionModelSelectionChangedEventArgs, TreeSelectionModelSelectionChangedEventArgsOf, UntypedItems,
};
use super::tree_selection_model_source_reset_event_args::TreeSelectionModelSourceResetEventArgs;
use super::tree_selection_node::TreeSelectionNode;
use super::selection_node_base::same_source;
use crate::accessor::TreeModel;
use crate::model_collection::{ModelCollection, Models};
use crate::IndexPath;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The overridable members of [`TreeSelectionModelBase`].
///
/// The type deriving from the model owns it (`Rc<TreeSelectionModelBase<M>>`), implements this
/// trait and registers itself, weakly, with [`TreeSelectionModelBase::set_hooks`]; every virtual
/// call the model makes then goes through the implementation. A model without hooks has no
/// children (a flat list) and uses the base implementations.
///
/// Each method receives the model; the default methods are the base implementations (the
/// `base_*` methods of the model), which an override calls where the reference calls the base
/// implementation.
///
/// ```ignore
/// struct RowSelection<M: TreeModel> { base: Rc<TreeSelectionModelBase<M>>, /* ... */ }
///
/// impl<M: TreeModel> RowSelection<M> {
///     fn new(items: Option<Models<M>>) -> Rc<Self> {
///         Rc::new_cyclic(|this: &Weak<Self>| {
///             let base = TreeSelectionModelBase::with_source(items);
///             base.set_hooks(this.clone());
///             Self { base }
///         })
///     }
/// }
///
/// impl<M: TreeModel> TreeSelectionModelHooks<M> for RowSelection<M> {
///     fn get_children(&self, _model: &TreeSelectionModelBase<M>, node: &Rc<M>) -> Option<Models<M>> {
///         /* the children of `node` read by the expander column of the source */
///     }
/// }
/// ```
pub trait TreeSelectionModelHooks<M: TreeModel> {
    /// The children of `node` (abstract `GetChildren`).
    fn get_children(&self, model: &TreeSelectionModelBase<M>, node: &Rc<M>) -> Option<Models<M>>;

    /// The model at `index`, or `None` when there is none (`TryGetItemAt`).
    fn try_get_item_at(&self, model: &TreeSelectionModelBase<M>, index: &IndexPath) -> Option<Rc<M>> {
        model.base_try_get_item_at(index)
    }

    /// Called when a change of a collection of the source has finished, and all the handlers of
    /// its change have run.
    fn on_source_collection_change_finished(&self, model: &TreeSelectionModelBase<M>) {
        model.base_on_source_collection_change_finished();
    }

    /// Called when the collection of the children of `parent_index` was reset, `remove_count`
    /// selected items being removed with it.
    fn on_node_collection_reset(&self, model: &TreeSelectionModelBase<M>, parent_index: &IndexPath, remove_count: i32) {
        model.base_on_node_collection_reset(parent_index, remove_count);
    }
}

/// The pending changes of a batch update.
///
/// The operation is shared: a handler invoked while it is being committed can start an update,
/// which continues the same operation, as in the reference.
pub struct Operation<M: TreeModel> {
    pub update_count: Cell<i32>,
    pub is_source_update: Cell<bool>,
    pub anchor_index: RefCell<IndexPath>,
    pub range_anchor_index: RefCell<IndexPath>,
    pub selected_index: RefCell<IndexPath>,
    pub selected_ranges: RefCell<Option<IndexRanges>>,
    pub deselected_ranges: RefCell<Option<IndexRanges>>,
    pub deselected_items: RefCell<Option<Rc<dyn IReadOnlyList<Option<Rc<M>>>>>>,
}

impl<M: TreeModel> Operation<M> {
    /// An operation that starts from the indexes of `owner`.
    pub fn new(owner: &TreeSelectionModelBase<M>) -> Self {
        Self {
            update_count: Cell::new(0),
            is_source_update: Cell::new(false),
            anchor_index: RefCell::new(owner.anchor_index()),
            range_anchor_index: RefCell::new(owner.range_anchor_index()),
            selected_index: RefCell::new(owner.selected_index()),
            selected_ranges: RefCell::new(None),
            deselected_ranges: RefCell::new(None),
            deselected_items: RefCell::new(None),
        }
    }
}

/// A batch update in progress (`BatchUpdateOperation`): ends the update when disposed or dropped.
pub struct BatchUpdateOperation<M: TreeModel> {
    owner: Rc<TreeSelectionModelBase<M>>,
    is_disposed: Cell<bool>,
}

impl<M: TreeModel> BatchUpdateOperation<M> {
    /// Begins a batch update of `owner`.
    pub fn new(owner: Rc<TreeSelectionModelBase<M>>) -> Self {
        owner.begin_batch_update();
        Self { owner, is_disposed: Cell::new(false) }
    }

    /// The operation of the update.
    pub fn operation(&self) -> Rc<Operation<M>> {
        self.owner.current_operation()
    }

    /// Ends the update.
    pub fn dispose(&self) {
        if !self.is_disposed.get() {
            self.is_disposed.set(true);
            self.owner.end_batch_update();
        }
    }
}

impl<M: TreeModel> Drop for BatchUpdateOperation<M> {
    fn drop(&mut self) {
        // While unwinding, ending the update could panic again.
        if !std::thread::panicking() {
            self.dispose();
        }
    }
}

/// The `using var update = BatchUpdate()` of the model's own members.
struct BatchGuard<'a, M: TreeModel>(&'a TreeSelectionModelBase<M>);

impl<M: TreeModel> Drop for BatchGuard<'_, M> {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.0.end_batch_update();
        }
    }
}

/// The parents and their ranges of a set of index paths, copied so that the set is not borrowed
/// while the nodes are created (which reads the children of the models).
fn snapshot_ranges(ranges: &IndexRanges) -> Vec<(IndexPath, Vec<IndexRange>)> {
    ranges.ranges().iter().map(|(parent, ranges)| (parent.clone(), ranges.clone())).collect()
}

/// A copy of a set of index paths.
fn copy_index_ranges(ranges: &IndexRanges) -> IndexRanges {
    let mut result = IndexRanges::new();
    for (parent, ranges) in ranges.ranges() {
        result.add_ranges(parent, ranges.clone());
    }
    result
}

/// The base of the selection models of a hierarchical source of models of type `M`.
pub struct TreeSelectionModelBase<M: TreeModel> {
    this: Weak<Self>,
    hooks: RefCell<Option<Weak<dyn TreeSelectionModelHooks<M>>>>,
    root: Rc<TreeSelectionNode<M>>,
    count: Cell<i32>,
    single_select: Cell<bool>,
    anchor_index: RefCell<IndexPath>,
    range_anchor_index: RefCell<IndexPath>,
    selected_index: RefCell<IndexPath>,
    operation: RefCell<Option<Rc<Operation<M>>>>,
    selected_indexes: RefCell<Option<Rc<TreeSelectedIndexes<M>>>>,
    selected_items: RefCell<Option<Rc<TreeSelectedItems<M>>>>,
    selected_items_untyped: RefCell<Option<Rc<dyn IReadOnlyList<Option<BoxedValue>>>>>,
    collection_changing: Cell<i32>,
    old_selected_index_during_source_change: RefCell<IndexPath>,
    old_anchor_index_during_source_change: RefCell<IndexPath>,
    old_range_anchor_index_during_source_change: RefCell<IndexPath>,
    selected_index_moved_during_source_change: Cell<bool>,
    anchor_index_moved_during_source_change: Cell<bool>,
    range_anchor_index_moved_during_source_change: Cell<bool>,
    selection_changed: Event<TreeSelectionModelSelectionChangedEventArgsOf<M>>,
    untyped_selection_changed: HandlerList<dyn Fn(&dyn TreeSelectionModelSelectionChangedEventArgs)>,
    property_changed: Event<str>,
    indexes_changed: Event<TreeSelectionModelIndexesChangedEventArgs>,
    source_reset: Event<TreeSelectionModelSourceResetEventArgs>,
}

impl<M: TreeModel> TreeSelectionModelBase<M> {
    /// A model without a source.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| Self {
            this: this.clone(),
            hooks: RefCell::new(None),
            root: TreeSelectionNode::new(this.clone()),
            count: Cell::new(0),
            single_select: Cell::new(true),
            anchor_index: RefCell::new(IndexPath::UNSELECTED),
            range_anchor_index: RefCell::new(IndexPath::UNSELECTED),
            selected_index: RefCell::new(IndexPath::UNSELECTED),
            operation: RefCell::new(None),
            selected_indexes: RefCell::new(None),
            selected_items: RefCell::new(None),
            selected_items_untyped: RefCell::new(None),
            collection_changing: Cell::new(0),
            old_selected_index_during_source_change: RefCell::new(IndexPath::UNSELECTED),
            old_anchor_index_during_source_change: RefCell::new(IndexPath::UNSELECTED),
            old_range_anchor_index_during_source_change: RefCell::new(IndexPath::UNSELECTED),
            selected_index_moved_during_source_change: Cell::new(false),
            anchor_index_moved_during_source_change: Cell::new(false),
            range_anchor_index_moved_during_source_change: Cell::new(false),
            selection_changed: Event::new(),
            untyped_selection_changed: HandlerList::new(),
            property_changed: Event::new(),
            indexes_changed: Event::new(),
            source_reset: Event::new(),
        })
    }

    /// A model of `source`.
    pub fn with_source(source: Option<Models<M>>) -> Rc<Self> {
        let result = Self::new();
        result.set_source(source);
        result
    }

    /// Registers the object deriving from the model, which receives the virtual calls (see
    /// [`TreeSelectionModelHooks`]).
    pub fn set_hooks(&self, hooks: Weak<dyn TreeSelectionModelHooks<M>>) {
        *self.hooks.borrow_mut() = Some(hooks);
    }

    fn hooks(&self) -> Option<Rc<dyn TreeSelectionModelHooks<M>>> {
        self.hooks.borrow().as_ref().and_then(Weak::upgrade)
    }

    /// Gets the number of selected items.
    pub fn count(&self) -> i32 {
        self.count.get()
    }

    fn set_count(&self, value: i32) {
        if self.count.get() != value {
            self.count.set(value);
            self.raise_property_changed("Count");
        }
    }

    /// Gets a value indicating whether the model supports single or multiple selection.
    pub fn single_select(&self) -> bool {
        self.single_select.get()
    }

    /// Sets a value indicating whether the model supports single or multiple selection; switching
    /// to single selection keeps only the selected index.
    pub fn set_single_select(&self, value: bool) {
        if self.single_select.get() != value {
            if value {
                self.set_selected_index(self.selected_index());
            }

            self.single_select.set(value);

            self.raise_property_changed("SingleSelect");
        }
    }

    /// Gets the selected index: in the case of multiple selected items, the index of the item
    /// that was first selected.
    pub fn selected_index(&self) -> IndexPath {
        self.selected_index.borrow().clone()
    }

    /// Sets the selected index: clears the selection and selects the index, which becomes the
    /// anchor and the range anchor.
    pub fn set_selected_index(&self, value: IndexPath) {
        let _update = self.batch();
        self.clear();
        self.select_core(&value, true);
    }

    /// Gets the indexes of the selected items: a view of the selection, always the same list.
    pub fn selected_indexes(&self) -> Rc<TreeSelectedIndexes<M>> {
        self.selected_indexes.borrow_mut().get_or_insert_with(|| Rc::new(TreeSelectedIndexes::new(self.this.clone()))).clone()
    }

    /// Gets the selected item: in the case of multiple selected items, the item that was first
    /// selected.
    pub fn selected_item(&self) -> Option<Rc<M>> {
        let selected_index = self.selected_index();
        if self.source().is_none() || selected_index.is_empty() {
            None
        } else {
            Some(self.get_selected_item_at(&selected_index))
        }
    }

    /// Gets the selected items: a view of the selection, always the same list.
    pub fn selected_items(&self) -> Rc<TreeSelectedItems<M>> {
        self.selected_items.borrow_mut().get_or_insert_with(|| Rc::new(TreeSelectedItems::new(self.this.clone()))).clone()
    }

    /// Gets the anchor index.
    pub fn anchor_index(&self) -> IndexPath {
        self.anchor_index.borrow().clone()
    }

    /// Sets the anchor index; an index of no item is ignored.
    pub fn set_anchor_index(&self, value: IndexPath) {
        if !value.is_empty() && self.try_get_item_at(&value).is_none() {
            return;
        }
        let _update = self.batch();
        *self.current_operation().anchor_index.borrow_mut() = value;
    }

    /// Gets the range anchor index.
    pub fn range_anchor_index(&self) -> IndexPath {
        self.range_anchor_index.borrow().clone()
    }

    /// Sets the range anchor index; an index of no item is ignored.
    pub fn set_range_anchor_index(&self, value: IndexPath) {
        if !value.is_empty() && self.try_get_item_at(&value).is_none() {
            return;
        }
        let _update = self.batch();
        *self.current_operation().range_anchor_index.borrow_mut() = value;
    }

    /// The root node of the selection.
    pub fn root(&self) -> Rc<TreeSelectionNode<M>> {
        self.root.clone()
    }

    /// Replaces the children collection of the node of `parent_index`, if the node exists, and
    /// handles the replacement as a reset of the collection.
    pub fn reset_children_source(&self, parent_index: &IndexPath, source: Option<Models<M>>) {
        if let Some(node) = self.get_node(parent_index) {
            node.reset_source(source);
        }
    }

    /// Whether a collection of the source is changing.
    pub fn is_source_collection_changing(&self) -> bool {
        self.collection_changing.get() > 0
    }

    /// Gets the source collection.
    pub fn source(&self) -> Option<Models<M>> {
        self.root.source()
    }

    /// Sets the source collection; replacing a source by another clears the selection.
    pub fn set_source(&self, value: Option<Models<M>>) {
        let current = self.root.source();
        if !same_source(current.as_ref(), value.as_ref()) {
            if current.is_some() && value.is_some() {
                let _update = self.batch();
                self.clear();
            }

            self.root.set_source(value);
        }
    }

    /// Subscribes to the selection changed event (typed).
    pub fn add_selection_changed(&self, handler: Rc<dyn Fn(&TreeSelectionModelSelectionChangedEventArgsOf<M>)>) -> u64 {
        self.selection_changed.add(handler)
    }

    /// Removes a handler of [`add_selection_changed`](Self::add_selection_changed).
    pub fn remove_selection_changed(&self, token: u64) {
        self.selection_changed.remove(token);
    }

    /// Subscribes to the selection changed event of the untyped contract.
    pub fn add_untyped_selection_changed(&self, handler: Rc<dyn Fn(&dyn TreeSelectionModelSelectionChangedEventArgs)>) -> u64 {
        self.untyped_selection_changed.add(handler)
    }

    /// Removes a handler of [`add_untyped_selection_changed`](Self::add_untyped_selection_changed).
    pub fn remove_untyped_selection_changed(&self, token: u64) {
        self.untyped_selection_changed.remove(token);
    }

    /// Subscribes to the event raised when the indexes of the selected items are changed due to
    /// items being added or removed from the source data.
    pub fn add_indexes_changed(&self, handler: Rc<dyn Fn(&TreeSelectionModelIndexesChangedEventArgs)>) -> u64 {
        self.indexes_changed.add(handler)
    }

    /// Removes a handler of [`add_indexes_changed`](Self::add_indexes_changed).
    pub fn remove_indexes_changed(&self, token: u64) {
        self.indexes_changed.remove(token);
    }

    /// Subscribes to the event raised when a collection of the source is reset.
    pub fn add_source_reset(&self, handler: Rc<dyn Fn(&TreeSelectionModelSourceResetEventArgs)>) -> u64 {
        self.source_reset.add(handler)
    }

    /// Removes a handler of [`add_source_reset`](Self::add_source_reset).
    pub fn remove_source_reset(&self, token: u64) {
        self.source_reset.remove(token);
    }

    /// Begins a batch update that ends when the returned operation is disposed or dropped.
    pub fn batch_update(&self) -> BatchUpdateOperation<M> {
        BatchUpdateOperation::new(self.this.upgrade().expect("the selection model is alive"))
    }

    fn batch(&self) -> BatchGuard<'_, M> {
        self.begin_batch_update();
        BatchGuard(self)
    }

    /// Begins a batch update of the selection.
    ///
    /// During a batch update no changes to the selection will be reflected in the model's
    /// properties and no events will be raised until [`end_batch_update`](Self::end_batch_update)
    /// is called. Each call must have a corresponding call to `end_batch_update`.
    pub fn begin_batch_update(&self) {
        let operation = {
            let mut slot = self.operation.borrow_mut();
            match slot.as_ref() {
                Some(operation) => operation.clone(),
                None => {
                    let operation = Rc::new(Operation::new(self));
                    *slot = Some(operation.clone());
                    operation
                }
            }
        };
        operation.update_count.set(operation.update_count.get() + 1);
    }

    /// Ends a batch update started by [`begin_batch_update`](Self::begin_batch_update); the end of
    /// the outermost update commits it.
    ///
    /// # Panics
    /// When no batch update is in progress (`InvalidOperationException`).
    pub fn end_batch_update(&self) {
        let current = self.operation.borrow().clone();
        let operation = match current {
            Some(operation) if operation.update_count.get() != 0 => operation,
            _ => panic!("No batch update in progress."),
        };

        operation.update_count.set(operation.update_count.get() - 1);

        if operation.update_count.get() == 0 {
            self.commit_operation(&operation);
        }
    }

    /// Clears the selection.
    pub fn clear(&self) {
        let _update = self.batch();
        let o = self.current_operation();
        self.root.clear(&o);
        *o.selected_index.borrow_mut() = IndexPath::UNSELECTED;
    }

    /// Deselects an item by its index in the source data.
    pub fn deselect(&self, index: &IndexPath) {
        if !self.is_selected(index) {
            return;
        }

        let _update = self.batch();
        let o = self.current_operation();

        if let Some(selected_ranges) = o.selected_ranges.borrow_mut().as_mut() {
            selected_ranges.remove(index);
        }
        o.deselected_ranges.borrow_mut().get_or_insert_with(IndexRanges::new).add(index);

        let selected_index = self.selected_index();
        let deselected_selected_index =
            o.deselected_ranges.borrow().as_ref().is_some_and(|deselected| deselected.contains(&selected_index));

        if deselected_selected_index {
            let first = {
                let deselected = o.deselected_ranges.borrow();
                self.get_first_selected_index(&self.root, deselected.as_ref())
            };
            *o.selected_index.borrow_mut() = first;
        }
    }

    /// Queries whether an item is currently selected.
    pub fn is_selected(&self, index: &IndexPath) -> bool {
        if index.is_empty() {
            return false;
        }
        let node = self.get_node(&index.parent());
        let ranges = node.map(|node| node.ranges());
        IndexRange::contains_in(ranges.as_ref().map(|ranges| ranges.as_slice()), index.last())
    }

    /// Selects an item by its index in the source data; the index becomes the anchor, not the
    /// range anchor.
    pub fn select(&self, index: &IndexPath) {
        self.select_core(index, false);
    }

    /// The children of `node`, read by the hooks; a model without hooks has none.
    pub fn get_children(&self, node: &Rc<M>) -> Option<Models<M>> {
        match self.hooks() {
            Some(hooks) => hooks.get_children(self, node),
            None => None,
        }
    }

    /// The model at `index`, or `None` when there is none (`TryGetItemAt`).
    pub fn try_get_item_at(&self, index: &IndexPath) -> Option<Rc<M>> {
        match self.hooks() {
            Some(hooks) => hooks.try_get_item_at(self, index),
            None => self.base_try_get_item_at(index),
        }
    }

    /// The base implementation of [`try_get_item_at`](Self::try_get_item_at): walks the source and
    /// the children of the models along the path.
    pub fn base_try_get_item_at(&self, index: &IndexPath) -> Option<Rc<M>> {
        let mut items: Option<Models<M>> = self.root.items_view().map(|items_view| items_view.inner());
        let count = index.count();

        for i in 0..count {
            let current = items.take()?;

            match Self::try_get_element_at(&current, index.get(i)) {
                Some(item) => {
                    if i == count - 1 {
                        return Some(item);
                    }
                    items = self.get_children(&item);
                }
                None => break,
            }
        }

        None
    }

    /// Called when a change of a collection of the source has finished, and all the handlers of
    /// its change have run.
    pub fn on_source_collection_change_finished(&self) {
        match self.hooks() {
            Some(hooks) => hooks.on_source_collection_change_finished(self),
            None => self.base_on_source_collection_change_finished(),
        }
    }

    /// The base implementation of
    /// [`on_source_collection_change_finished`](Self::on_source_collection_change_finished):
    /// nothing.
    pub fn base_on_source_collection_change_finished(&self) {}

    /// Raises the property changed event.
    pub fn raise_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }

    /// The model at `path`, which must be the path of a model of the source.
    ///
    /// # Panics
    /// For the empty path or a path of no node (`ArgumentOutOfRangeException`), and without a
    /// source (`InvalidOperationException`).
    pub fn get_selected_item_at(&self, path: &IndexPath) -> Rc<M> {
        if path.is_empty() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'path')");
        }
        if self.source().is_none() {
            panic!("Cannot get item from null Source.");
        }

        if let Some(node) = self.get_node(&path.parent()) {
            let items_view = node.items_view().expect("Object reference not set to an instance of an object.");
            return items_view.get_at(path.last() as usize);
        }

        panic!("Specified argument was out of the range of valid values. (Parameter 'path')");
    }

    /// Called by a node when its collection starts changing.
    pub fn on_node_collection_change_started(&self) {
        if self.collection_changing.get() == 0 {
            *self.old_selected_index_during_source_change.borrow_mut() = self.selected_index();
            *self.old_anchor_index_during_source_change.borrow_mut() = self.anchor_index();
            *self.old_range_anchor_index_during_source_change.borrow_mut() = self.range_anchor_index();
            self.selected_index_moved_during_source_change.set(false);
            self.anchor_index_moved_during_source_change.set(false);
            self.range_anchor_index_moved_during_source_change.set(false);
        }

        self.collection_changing.set(self.collection_changing.get() + 1);
    }

    /// Called by a node when its collection changed: shifts, moves or clears the selected and
    /// anchor indexes, and raises the events of the change.
    ///
    /// # Panics
    /// During a batch update (`InvalidOperationException`).
    #[allow(clippy::too_many_arguments)]
    pub fn on_node_collection_changed(
        &self,
        parent_index: &IndexPath,
        shift_start_index: i32,
        shift_end_index: i32,
        shift_delta: i32,
        raise_indexes_changed: bool,
        removed: Option<&[Option<Rc<M>>]>,
        move_old_index: i32,
        move_new_index: i32,
        move_count: i32,
        replace_old_index: i32,
        replace_old_count: i32,
        replace_new_count: i32,
    ) {
        let updating = self.operation.borrow().as_ref().is_some_and(|operation| operation.update_count.get() > 0);
        if updating {
            panic!("Source collection was modified during selection update.");
        }

        let removed = removed.filter(|removed| !removed.is_empty());

        if shift_delta == 0 && removed.is_none() && !raise_indexes_changed && replace_old_index < 0 {
            return;
        }

        if raise_indexes_changed {
            self.indexes_changed.raise(&TreeSelectionModelIndexesChangedEventArgs::new(
                parent_index.clone(),
                shift_start_index,
                shift_end_index,
                shift_delta,
            ));
        }

        let map_collection_index = |path: &mut IndexPath| -> bool {
            if move_count > 0 {
                Self::move_index(parent_index, move_old_index, move_new_index, move_count, path)
            } else if replace_old_index >= 0 {
                Self::replace_index(parent_index, replace_old_index, replace_old_count, replace_new_count, path)
            } else {
                Self::shift_index(parent_index, shift_start_index, shift_delta, path)
            }
        };

        // Shift or clear the selected and anchor indexes according to the shift index/delta.
        let had_selection = !self.selected_index().is_empty();
        let mut selected_index_changed = false;
        if !self.selected_index_moved_during_source_change.get() {
            let mut path = self.selected_index();
            selected_index_changed = map_collection_index(&mut path);
            *self.selected_index.borrow_mut() = path;
        }

        let mut anchor_index_changed = false;
        if !self.anchor_index_moved_during_source_change.get() {
            let mut path = self.anchor_index();
            anchor_index_changed = map_collection_index(&mut path);
            *self.anchor_index.borrow_mut() = path;
        }

        let mut range_anchor_index_changed = false;
        if !self.range_anchor_index_moved_during_source_change.get() {
            let mut path = self.range_anchor_index();
            range_anchor_index_changed = map_collection_index(&mut path);
            *self.range_anchor_index.borrow_mut() = path;
        }

        let mut selected_item_changed = false;

        // Check that the selected index is still selected in the node. It can get
        // unselected as the result of a replace operation.
        if had_selection && !self.is_selected(&self.selected_index()) {
            let first = self.get_first_selected_index(&self.root, None);
            *self.selected_index.borrow_mut() = first;
            selected_index_changed = true;
            selected_item_changed = true;
        }

        if let Some(removed) = removed {
            if self.selection_changed.has_handlers() || !self.untyped_selection_changed.is_empty() {
                let e = TreeSelectionModelSelectionChangedEventArgsOf::with_deselected_items(Rc::new(ReadOnlyList::new(
                    removed.to_vec(),
                )));
                self.selection_changed.raise(&e);
                self.raise_untyped_selection_changed(&e);
            }

            self.set_count(self.count() - removed.len() as i32);
        }

        if self.selected_index_moved_during_source_change.get()
            && *self.old_selected_index_during_source_change.borrow() != self.selected_index()
        {
            selected_index_changed = true;
        }

        if self.anchor_index_moved_during_source_change.get()
            && *self.old_anchor_index_during_source_change.borrow() != self.anchor_index()
        {
            anchor_index_changed = true;
        }

        if self.range_anchor_index_moved_during_source_change.get()
            && *self.old_range_anchor_index_during_source_change.borrow() != self.range_anchor_index()
        {
            range_anchor_index_changed = true;
        }

        if selected_index_changed {
            self.raise_property_changed("SelectedIndex");
        }
        if selected_item_changed {
            self.raise_property_changed("SelectedItem");
        }
        if anchor_index_changed {
            self.raise_property_changed("AnchorIndex");
        }
        if range_anchor_index_changed {
            self.raise_property_changed("RangeAnchorIndex");
        }
    }

    /// Called by a node when selected items moved within its collection: moves the selected and
    /// anchor indexes with them.
    pub fn on_node_selection_moved(
        &self,
        parent_index: &IndexPath,
        old_selected_ranges: &[IndexRange],
        new_selected_ranges: &[IndexRange],
        _moved_items: &[Option<Rc<M>>],
    ) {
        let mut path = self.selected_index();
        let moved = Self::try_move_index(parent_index, old_selected_ranges, new_selected_ranges, &mut path);
        *self.selected_index.borrow_mut() = path;
        self.selected_index_moved_during_source_change.set(self.selected_index_moved_during_source_change.get() | moved);

        let mut path = self.anchor_index();
        let moved = Self::try_move_index(parent_index, old_selected_ranges, new_selected_ranges, &mut path);
        *self.anchor_index.borrow_mut() = path;
        self.anchor_index_moved_during_source_change.set(self.anchor_index_moved_during_source_change.get() | moved);

        let mut path = self.range_anchor_index();
        let moved = Self::try_move_index(parent_index, old_selected_ranges, new_selected_ranges, &mut path);
        *self.range_anchor_index.borrow_mut() = path;
        self.range_anchor_index_moved_during_source_change
            .set(self.range_anchor_index_moved_during_source_change.get() | moved);
    }

    /// Called by a node when its collection finished changing; the end of the outermost change
    /// calls [`on_source_collection_change_finished`](Self::on_source_collection_change_finished).
    pub fn on_node_collection_change_finished(&self) {
        self.collection_changing.set(self.collection_changing.get() - 1);

        if self.collection_changing.get() == 0 {
            *self.old_selected_index_during_source_change.borrow_mut() = IndexPath::UNSELECTED;
            *self.old_anchor_index_during_source_change.borrow_mut() = IndexPath::UNSELECTED;
            *self.old_range_anchor_index_during_source_change.borrow_mut() = IndexPath::UNSELECTED;
            self.selected_index_moved_during_source_change.set(false);
            self.anchor_index_moved_during_source_change.set(false);
            self.range_anchor_index_moved_during_source_change.set(false);
            self.on_source_collection_change_finished();
        }
    }

    /// Called by a node when its collection was reset, `remove_count` selected items being
    /// removed with it.
    pub fn on_node_collection_reset(&self, parent_index: &IndexPath, remove_count: i32) {
        match self.hooks() {
            Some(hooks) => hooks.on_node_collection_reset(self, parent_index, remove_count),
            None => self.base_on_node_collection_reset(parent_index, remove_count),
        }
    }

    /// The base implementation of [`on_node_collection_reset`](Self::on_node_collection_reset):
    /// revalidates the selected and anchor indexes and raises the source reset event.
    pub fn base_on_node_collection_reset(&self, parent_index: &IndexPath, remove_count: i32) {
        let mut selected_index_changed = false;
        let mut anchor_index_changed = false;
        let mut range_anchor_index_changed = false;
        let mut selected_item_changed = false;

        // Check that the selected index is still selected in the node. It can get
        // unselected as the result of a replace operation.
        let selected_index = self.selected_index();
        if !selected_index.is_empty() && !self.is_selected(&selected_index) {
            let first = self.get_first_selected_index(&self.root, None);
            *self.selected_index.borrow_mut() = first;
            selected_index_changed = true;
            selected_item_changed = true;
        }

        // If the anchor index is invalid, clear it.
        let anchor_index = self.anchor_index();
        if !anchor_index.is_empty() && self.try_get_item_at(&anchor_index).is_none() {
            *self.anchor_index.borrow_mut() = IndexPath::UNSELECTED;
            anchor_index_changed = true;
        }

        let range_anchor_index = self.range_anchor_index();
        if !range_anchor_index.is_empty() && self.try_get_item_at(&range_anchor_index).is_none() {
            *self.range_anchor_index.borrow_mut() = IndexPath::UNSELECTED;
            range_anchor_index_changed = true;
        }

        self.set_count(self.count() - remove_count);
        self.source_reset.raise(&TreeSelectionModelSourceResetEventArgs::new(parent_index.clone()));

        if selected_index_changed {
            self.raise_property_changed("SelectedIndex");
        }
        if selected_item_changed {
            self.raise_property_changed("SelectedItem");
        }
        if anchor_index_changed {
            self.raise_property_changed("AnchorIndex");
        }
        if range_anchor_index_changed {
            self.raise_property_changed("RangeAnchorIndex");
        }
    }

    fn raise_untyped_selection_changed(&self, e: &TreeSelectionModelSelectionChangedEventArgsOf<M>) {
        if !self.untyped_selection_changed.is_empty() {
            for (_, handler) in self.untyped_selection_changed.snapshot().iter() {
                handler(e);
            }
        }
    }

    fn current_operation(&self) -> Rc<Operation<M>> {
        self.operation.borrow().clone().expect("a batch update is in progress")
    }

    fn get_first_selected_index(&self, node: &TreeSelectionNode<M>, except: Option<&IndexRanges>) -> IndexPath {
        let ranges = node.ranges();

        if !ranges.is_empty() {
            let count = IndexRange::get_count(&ranges);
            let path = node.path();
            let mut index = 0;

            while index < count {
                let result = path.append(IndexRange::get_at(&ranges, index));
                index += 1;
                if !except.is_some_and(|except| except.contains(&result)) {
                    return result;
                }
            }
        }

        if let Some(children) = node.children() {
            for child in children.iter().flatten() {
                let i = self.get_first_selected_index(child, except);

                if !i.is_empty() {
                    return i;
                }
            }
        }

        IndexPath::UNSELECTED
    }

    fn get_node(&self, path: &IndexPath) -> Option<Rc<TreeSelectionNode<M>>> {
        let mut node = self.root.clone();

        for i in 0..path.count() {
            node = node.get_child(path.get(i))?;
        }

        Some(node)
    }

    fn get_or_create_node(&self, path: &IndexPath) -> Option<Rc<TreeSelectionNode<M>>> {
        let mut node = self.root.clone();

        for i in 0..path.count() {
            node = node.get_or_create_child(path.get(i))?;
        }

        Some(node)
    }

    fn select_core(&self, index: &IndexPath, update_range_anchor_index: bool) {
        if index.is_empty() || self.try_get_item_at(index).is_none() {
            return;
        }

        let _update = self.batch();
        let o = self.current_operation();

        if self.single_select() {
            self.clear();
        }

        if let Some(deselected_ranges) = o.deselected_ranges.borrow_mut().as_mut() {
            deselected_ranges.remove(index);
        }

        if !self.is_selected(index) {
            o.selected_ranges.borrow_mut().get_or_insert_with(IndexRanges::new).add(index);
        }

        let no_selected_index = o.selected_index.borrow().is_empty();
        if no_selected_index {
            *o.selected_index.borrow_mut() = index.clone();
        }

        *o.anchor_index.borrow_mut() = index.clone();

        if update_range_anchor_index {
            *o.range_anchor_index.borrow_mut() = index.clone();
        }
    }

    fn commit_operation(&self, operation: &Rc<Operation<M>>) {
        let old_anchor_index = self.anchor_index();
        let old_range_anchor_index = self.range_anchor_index();
        let old_selected_index = self.selected_index();
        let mut indexes_changed = false;

        *self.selected_index.borrow_mut() = operation.selected_index.borrow().clone();
        *self.anchor_index.borrow_mut() = operation.anchor_index.borrow().clone();
        *self.range_anchor_index.borrow_mut() = operation.range_anchor_index.borrow().clone();

        let selected = operation.selected_ranges.borrow().as_ref().map(snapshot_ranges);
        if let Some(selected) = &selected {
            indexes_changed |= self.commit_select_ranges(selected) > 0;
        }

        let deselected = operation.deselected_ranges.borrow().as_ref().map(snapshot_ranges);
        if let Some(deselected) = &deselected {
            indexes_changed |= self.commit_deselect_ranges(deselected) > 0;
        }

        let selected_count = operation.selected_ranges.borrow().as_ref().map_or(0, |ranges| ranges.count());
        let deselected_count = operation.deselected_ranges.borrow().as_ref().map_or(0, |ranges| ranges.count());
        self.set_count(self.count() + selected_count - deselected_count);

        let operation_deselected_items = operation.deselected_items.borrow().clone();

        if (self.selection_changed.has_handlers() || !self.untyped_selection_changed.is_empty())
            && (deselected_count > 0 || selected_count > 0 || operation_deselected_items.is_some())
        {
            let deselected_indexes: Option<Rc<IndexRanges>> =
                operation.deselected_ranges.borrow().as_ref().map(|ranges| Rc::new(copy_index_ranges(ranges)));
            let selected_indexes: Option<Rc<IndexRanges>> =
                operation.selected_ranges.borrow().as_ref().map(|ranges| Rc::new(copy_index_ranges(ranges)));

            let deselected_items: Option<Rc<dyn IReadOnlyList<Option<Rc<M>>>>> = match operation_deselected_items {
                Some(items) => Some(items),
                None => TreeSelectionChangedItems::create(self.this.clone(), deselected_indexes.clone())
                    .map(|items| items as Rc<dyn IReadOnlyList<Option<Rc<M>>>>),
            };
            let selected_items: Option<Rc<dyn IReadOnlyList<Option<Rc<M>>>>> =
                TreeSelectionChangedItems::create(self.this.clone(), selected_indexes.clone())
                    .map(|items| items as Rc<dyn IReadOnlyList<Option<Rc<M>>>>);

            let e = TreeSelectionModelSelectionChangedEventArgsOf::new(
                deselected_indexes.map(|indexes| indexes as Rc<dyn IReadOnlyList<IndexPath>>),
                selected_indexes.map(|indexes| indexes as Rc<dyn IReadOnlyList<IndexPath>>),
                deselected_items,
                selected_items,
            );
            self.selection_changed.raise(&e);
            self.raise_untyped_selection_changed(&e);
        }

        self.root.prune_empty_children();

        if old_selected_index != self.selected_index() {
            indexes_changed = true;
            self.raise_property_changed("SelectedIndex");
            self.raise_property_changed("SelectedItem");
        }

        if old_anchor_index != self.anchor_index() {
            self.raise_property_changed("AnchorIndex");
        }

        if old_range_anchor_index != self.range_anchor_index() {
            self.raise_property_changed("RangeAnchorIndex");
        }

        if indexes_changed {
            self.raise_property_changed("SelectedIndexes");
            self.raise_property_changed("SelectedItems");
        }

        *self.operation.borrow_mut() = None;
    }

    fn commit_select_ranges(&self, selected_ranges: &[(IndexPath, Vec<IndexRange>)]) -> i32 {
        let mut result = 0;

        for (parent, ranges) in selected_ranges {
            if let Some(node) = self.get_or_create_node(parent) {
                for range in ranges {
                    result += node.commit_select_range(*range);
                }
            }
        }

        result
    }

    fn commit_deselect_ranges(&self, deselected_ranges: &[(IndexPath, Vec<IndexRange>)]) -> i32 {
        let mut result = 0;

        for (parent, ranges) in deselected_ranges {
            if let Some(node) = self.get_or_create_node(parent) {
                for range in ranges {
                    result += node.commit_deselect_range(*range);
                }
            }
        }

        result
    }

    fn try_move_index(
        parent_index: &IndexPath,
        old_selected_ranges: &[IndexRange],
        new_selected_ranges: &[IndexRange],
        path: &mut IndexPath,
    ) -> bool {
        if path.is_empty() {
            return false;
        }

        let depth = parent_index.count();

        if path.count() <= depth {
            return false;
        }

        if parent_index.count() > 0 && !parent_index.is_ancestor_of(path) {
            return false;
        }

        let index_at_depth = path.get(depth);
        let range_count = old_selected_ranges.len().min(new_selected_ranges.len());

        for i in 0..range_count {
            let old_range = old_selected_ranges[i];
            let new_range = new_selected_ranges[i];

            if index_at_depth >= old_range.begin() && index_at_depth <= old_range.end() {
                let offset = index_at_depth - old_range.begin();
                let new_index = new_range.begin() + offset;

                if new_index == index_at_depth {
                    return false;
                }

                let mut indexes = path.to_vec();
                indexes[depth] = new_index;
                *path = IndexPath::from_indexes(&indexes);
                return true;
            }
        }

        false
    }

    /// Shifts or clears `path` for a change of the children of `parent_index`: an index removed
    /// (`shift_delta` negative, within the removed range) clears the path, an index at or after
    /// `shift_index` is shifted by `shift_delta`. Returns whether the path changed.
    pub fn shift_index(parent_index: &IndexPath, shift_index: i32, shift_delta: i32, path: &mut IndexPath) -> bool {
        if parent_index.is_ancestor_of(path) && path.get(parent_index.count()) >= shift_index {
            let change_depth = parent_index.count();
            let path_index = path.get(change_depth);

            if shift_delta < 0 && path_index >= shift_index && path_index < shift_index - shift_delta {
                // Item was removed, clear the path.
                *path = IndexPath::UNSELECTED;
                return true;
            }

            if path_index >= shift_index {
                // Item remains, but index was shifted.
                let mut indexes = path.to_vec();
                indexes[change_depth] += shift_delta;
                *path = IndexPath::from_indexes(&indexes);
                return true;
            }
        }

        false
    }

    fn replace_index(parent_index: &IndexPath, old_index: i32, old_count: i32, new_count: i32, path: &mut IndexPath) -> bool {
        if path.is_empty() || !parent_index.is_ancestor_of(path) {
            return false;
        }

        let depth = parent_index.count();
        let current = path.get(depth);
        if current < old_index {
            return false;
        }
        if current < old_index + old_count {
            *path = IndexPath::UNSELECTED;
            return true;
        }

        let delta = new_count - old_count;
        if delta == 0 {
            return false;
        }
        let mut indexes = path.to_vec();
        indexes[depth] += delta;
        *path = IndexPath::from_indexes(&indexes);
        true
    }

    fn move_index(parent_index: &IndexPath, old_index: i32, new_index: i32, count: i32, path: &mut IndexPath) -> bool {
        if path.is_empty() || !parent_index.is_ancestor_of(path) || count <= 0 || old_index == new_index {
            return false;
        }

        let depth = parent_index.count();
        let current = path.get(depth);
        let mut mapped = current;

        if current >= old_index && current < old_index + count {
            mapped = new_index + current - old_index;
        } else if old_index < new_index && current >= old_index + count && current < new_index + count {
            mapped = current - count;
        } else if old_index > new_index && current >= new_index && current < old_index {
            mapped = current + count;
        }

        if mapped == current {
            return false;
        }

        let mut indexes = path.to_vec();
        indexes[depth] = mapped;
        *path = IndexPath::from_indexes(&indexes);
        true
    }

    fn try_get_element_at(items: &Models<M>, index: i32) -> Option<Rc<M>> {
        let items: &dyn ModelCollection<M> = &**items;
        if index >= 0 && (index as usize) < ModelCollection::count(items) {
            Some(ModelCollection::get(items, index as usize))
        } else {
            None
        }
    }
}

impl<M: TreeModel> INotifyPropertyChanged for TreeSelectionModelBase<M> {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl<M: TreeModel> ITreeSelectionModel for TreeSelectionModelBase<M> {
    fn source(&self) -> Option<UntypedModels> {
        TreeSelectionModelBase::source(self).map(erase_models)
    }

    /// # Panics
    /// Always (`NotSupportedException`): the source of the base is set by the deriving type.
    fn set_source(&self, _value: Option<UntypedModels>) {
        panic!("Specified method is not supported.");
    }

    fn single_select(&self) -> bool {
        TreeSelectionModelBase::single_select(self)
    }

    fn set_single_select(&self, value: bool) {
        TreeSelectionModelBase::set_single_select(self, value);
    }

    fn selected_index(&self) -> IndexPath {
        TreeSelectionModelBase::selected_index(self)
    }

    fn set_selected_index(&self, value: IndexPath) {
        TreeSelectionModelBase::set_selected_index(self, value);
    }

    fn selected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>> {
        TreeSelectionModelBase::selected_indexes(self)
    }

    fn selected_item(&self) -> Option<BoxedValue> {
        box_model(&TreeSelectionModelBase::selected_item(self))
    }

    fn selected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>> {
        let cached = self.selected_items_untyped.borrow().clone();
        match cached {
            Some(untyped) => untyped,
            None => {
                let typed: Rc<dyn IReadOnlyList<Option<Rc<M>>>> = TreeSelectionModelBase::selected_items(self);
                let untyped: Rc<dyn IReadOnlyList<Option<BoxedValue>>> = Rc::new(UntypedItems::new(typed));
                *self.selected_items_untyped.borrow_mut() = Some(untyped.clone());
                untyped
            }
        }
    }

    fn anchor_index(&self) -> IndexPath {
        TreeSelectionModelBase::anchor_index(self)
    }

    fn set_anchor_index(&self, value: IndexPath) {
        TreeSelectionModelBase::set_anchor_index(self, value);
    }

    fn range_anchor_index(&self) -> IndexPath {
        TreeSelectionModelBase::range_anchor_index(self)
    }

    fn set_range_anchor_index(&self, value: IndexPath) {
        TreeSelectionModelBase::set_range_anchor_index(self, value);
    }

    fn count(&self) -> i32 {
        TreeSelectionModelBase::count(self)
    }

    fn add_selection_changed(&self, handler: Rc<dyn Fn(&dyn TreeSelectionModelSelectionChangedEventArgs)>) -> u64 {
        self.add_untyped_selection_changed(handler)
    }

    fn remove_selection_changed(&self, token: u64) {
        self.remove_untyped_selection_changed(token);
    }

    fn add_indexes_changed(&self, handler: Rc<dyn Fn(&TreeSelectionModelIndexesChangedEventArgs)>) -> u64 {
        TreeSelectionModelBase::add_indexes_changed(self, handler)
    }

    fn remove_indexes_changed(&self, token: u64) {
        TreeSelectionModelBase::remove_indexes_changed(self, token);
    }

    fn add_source_reset(&self, handler: Rc<dyn Fn(&TreeSelectionModelSourceResetEventArgs)>) -> u64 {
        TreeSelectionModelBase::add_source_reset(self, handler)
    }

    fn remove_source_reset(&self, token: u64) {
        TreeSelectionModelBase::remove_source_reset(self, token);
    }

    fn clear(&self) {
        TreeSelectionModelBase::clear(self);
    }

    fn select(&self, index: &IndexPath) {
        TreeSelectionModelBase::select(self, index);
    }

    fn deselect(&self, index: &IndexPath) {
        TreeSelectionModelBase::deselect(self, index);
    }

    fn is_selected(&self, index: &IndexPath) -> bool {
        TreeSelectionModelBase::is_selected(self, index)
    }

    fn begin_batch_update(&self) {
        TreeSelectionModelBase::begin_batch_update(self);
    }

    fn end_batch_update(&self) {
        TreeSelectionModelBase::end_batch_update(self);
    }
}

/// Reads the typed source of an untyped one for a deriving type that implements the setter of the
/// untyped source (`ITreeDataGridSelection.Source`).
///
/// # Panics
/// When the collection holds models of another type (`InvalidCastException`).
pub fn typed_source<M: TreeModel>(value: Option<UntypedModels>) -> Option<Models<M>> {
    value.map(|value| {
        downcast_models::<M>(&value).unwrap_or_else(|| {
            panic!("Unable to cast the source to a collection of '{}'.", std::any::type_name::<M>())
        })
    })
}
