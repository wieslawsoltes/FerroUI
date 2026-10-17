//! `Selection/ITreeSelectionModel.cs`: the untyped contract of the selection of items in a
//! hierarchical data source, with the read-only lists it exposes.
//!
//! The reference's `IReadOnlyList<T>` becomes [`IReadOnlyList`], and its untyped `IEnumerable`
//! source becomes [`UntypedModels`]: a [`Models<M>`] whose model type is erased, so that the
//! contract stays object-safe and free of the model type, as the reference's interface is.

use super::tree_selection_model_indexes_changed_event_args::TreeSelectionModelIndexesChangedEventArgs;
use super::tree_selection_model_selection_changed_event_args::TreeSelectionModelSelectionChangedEventArgs;
use super::tree_selection_model_source_reset_event_args::TreeSelectionModelSourceResetEventArgs;
use crate::accessor::TreeModel;
use crate::model_collection::Models;
use crate::IndexPath;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::rc::Rc;

/// A read-only list (`IReadOnlyList<T>`): a count, indexed access and enumeration.
///
/// Enumeration works on a snapshot where the list is a view of a selection, so that the selection
/// may change while the items are enumerated.
pub trait IReadOnlyList<T> {
    /// The number of items.
    fn count(&self) -> usize;

    /// The item at `index`.
    ///
    /// # Panics
    /// When `index` is not below [`count`](Self::count).
    fn get(&self, index: usize) -> T;

    /// The items, in order.
    fn iter(&self) -> Box<dyn Iterator<Item = T> + '_>;

    /// The items as a vector.
    fn to_vec(&self) -> Vec<T> {
        self.iter().collect()
    }
}

/// A read-only list of owned items (an array or a `List<T>` of the reference given as an
/// `IReadOnlyList<T>`).
///
/// A wrapper rather than an implementation of [`IReadOnlyList`] for `Vec<T>`, so that the methods
/// of the trait do not shadow the slice methods of vectors (`iter`, `get`) where the trait is in
/// scope.
pub struct ReadOnlyList<T>(Vec<T>);

impl<T> ReadOnlyList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self(items)
    }

    /// An empty list.
    pub fn empty() -> Self {
        Self(Vec::new())
    }
}

impl<T: Clone> IReadOnlyList<T> for ReadOnlyList<T> {
    fn count(&self) -> usize {
        self.0.len()
    }

    fn get(&self, index: usize) -> T {
        if index >= self.0.len() {
            panic!("Index was outside the bounds of the array.");
        }
        self.0[index].clone()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = T> + '_> {
        Box::new(self.0.as_slice().iter().cloned())
    }

    fn to_vec(&self) -> Vec<T> {
        self.0.clone()
    }
}

/// An untyped collection of models (the reference's `IEnumerable`): a [`Models<M>`] with its model
/// type erased. Made with [`erase_models`], read with [`downcast_models`].
pub type UntypedModels = Rc<dyn Any>;

/// Erases the model type of a collection of models.
pub fn erase_models<M: TreeModel>(models: Models<M>) -> UntypedModels {
    Rc::new(models)
}

/// The collection of models of an untyped collection, or `None` when it holds models of another
/// type.
pub fn downcast_models<M: TreeModel>(models: &UntypedModels) -> Option<Models<M>> {
    let any: &dyn Any = &**models;
    any.downcast_ref::<Models<M>>().cloned()
}

/// Boxes a typed, nullable model as an untyped item (`object?`). The box is the model's own
/// allocation, so the untyped item keeps the identity of the model.
pub fn box_model<M: TreeModel>(item: &Option<Rc<M>>) -> Option<BoxedValue> {
    item.as_ref().map(|model| model.clone() as BoxedValue)
}

/// Stores and manipulates the selection state of items in a hierarchical data source.
///
/// The property changed event is raised with the names `Count`, `SingleSelect`, `SelectedIndex`,
/// `SelectedIndexes`, `SelectedItem`, `SelectedItems`, `AnchorIndex` and `RangeAnchorIndex`.
pub trait ITreeSelectionModel: INotifyPropertyChanged {
    /// Gets the root of the hierarchical data.
    fn source(&self) -> Option<UntypedModels>;

    /// Sets the root of the hierarchical data.
    fn set_source(&self, value: Option<UntypedModels>);

    /// Gets a value indicating whether the model supports single or multiple selection.
    fn single_select(&self) -> bool;

    /// Sets a value indicating whether the model supports single or multiple selection.
    fn set_single_select(&self, value: bool);

    /// Gets the selected index within the data.
    ///
    /// In the case of multiple selected items, returns the index of the item that was first
    /// selected.
    fn selected_index(&self) -> IndexPath;

    /// Sets the selected index within the data.
    fn set_selected_index(&self, value: IndexPath);

    /// Gets the indexes of the selected items.
    fn selected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>>;

    /// Gets the selected item.
    ///
    /// In the case of multiple selected items, returns the item that was first selected.
    fn selected_item(&self) -> Option<BoxedValue>;

    /// Gets the selected items.
    fn selected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>>;

    /// Gets the anchor index.
    ///
    /// The anchor index holds the index of the item from which non-ranged keyboard selection
    /// will start, i.e. in a vertical list it will hold the item from which selection should
    /// be moved by pressing the up or down arrow key. This is usually the last selected item.
    ///
    /// The anchor index is automatically set when selecting an item via the selected index or
    /// [`select`](Self::select).
    fn anchor_index(&self) -> IndexPath;

    /// Sets the anchor index.
    fn set_anchor_index(&self, value: IndexPath);

    /// Gets the range anchor index.
    ///
    /// The range anchor index holds the index of the item from which ranged selection will
    /// start, i.e. when shift-clicking an item it represents the start of the range to be
    /// selected whereas the shift-clicked item will be the end of the range.
    ///
    /// The range anchor index is set when selecting an item via the selected index but not via
    /// [`select`](Self::select).
    fn range_anchor_index(&self) -> IndexPath;

    /// Sets the range anchor index.
    fn set_range_anchor_index(&self, value: IndexPath);

    /// Gets the number of selected items.
    fn count(&self) -> i32;

    /// Subscribes to the event raised when the selection changes due to items being
    /// selected/deselected, or selected items being removed from the source.
    ///
    /// Note that, as in the notification model of the collections, this event will not be raised
    /// when a reset signal is received due to e.g. clearing a notifying list. For this reason it
    /// is advised that all subscriptions to the selection changed event are paired with a
    /// subscription to the source reset event.
    fn add_selection_changed(&self, handler: Rc<dyn Fn(&dyn TreeSelectionModelSelectionChangedEventArgs)>) -> u64;

    /// Removes a handler of [`add_selection_changed`](Self::add_selection_changed).
    fn remove_selection_changed(&self, token: u64);

    /// Subscribes to the event raised when the indexes of the selected items are changed due to
    /// items being added or removed from the source data.
    fn add_indexes_changed(&self, handler: Rc<dyn Fn(&TreeSelectionModelIndexesChangedEventArgs)>) -> u64;

    /// Removes a handler of [`add_indexes_changed`](Self::add_indexes_changed).
    fn remove_indexes_changed(&self, token: u64);

    /// Subscribes to the event raised when a reset signal is received on the source data.
    fn add_source_reset(&self, handler: Rc<dyn Fn(&TreeSelectionModelSourceResetEventArgs)>) -> u64;

    /// Removes a handler of [`add_source_reset`](Self::add_source_reset).
    fn remove_source_reset(&self, token: u64);

    /// Clears the selection.
    fn clear(&self);

    /// Selects an item by its index in the source data.
    fn select(&self, index: &IndexPath);

    /// Deselects an item by its index in the source data.
    fn deselect(&self, index: &IndexPath);

    /// Queries whether an item is currently selected.
    fn is_selected(&self, index: &IndexPath) -> bool;

    /// Begins a batch update of the selection.
    ///
    /// During a batch update no changes to the selection will be reflected in the model's
    /// properties and no events will be raised until [`end_batch_update`](Self::end_batch_update)
    /// is called.
    ///
    /// `begin_batch_update` may be called multiple times, even when a batch update is already in
    /// progress; each call must have a corresponding call to `end_batch_update` in order to finish
    /// the operation.
    fn begin_batch_update(&self);

    /// Ends a batch update started by [`begin_batch_update`](Self::begin_batch_update).
    fn end_batch_update(&self);
}
