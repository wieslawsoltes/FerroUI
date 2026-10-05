use super::read_only_selection_list_base::IReadOnlySelectionList;
use super::selection_model_indexes_changed_event_args::SelectionModelIndexesChangedEventArgs;
use super::selection_model_selection_changed_event_args::SelectionModelSelectionChangedEventArgs;
use crate::items_source::ItemsSource;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::reactive::IDisposable;
use ferroui_base::BoxedValue;
use std::cell::Cell;
use std::rc::Rc;

/// The untyped contract of a selection model: the selection of an items
/// control.
///
/// The property changed event is raised with the names `Source`,
/// `SingleSelect`, `SelectedIndex`, `SelectedIndexes`, `SelectedItem`,
/// `SelectedItems` and `AnchorIndex`.
///
/// Each event is subscribed to with the method of its name, which returns
/// the handle that unsubscribes the handler when disposed.
pub trait ISelectionModel: INotifyPropertyChanged {
    fn source(&self) -> Option<ItemsSource>;
    fn set_source(&self, value: Option<ItemsSource>);
    fn single_select(&self) -> bool;
    fn set_single_select(&self, value: bool);
    fn selected_index(&self) -> i32;
    fn set_selected_index(&self, value: i32);
    fn selected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>>;
    fn selected_item(&self) -> Option<BoxedValue>;
    fn set_selected_item(&self, value: Option<BoxedValue>);
    fn selected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<BoxedValue>>>;
    fn anchor_index(&self) -> i32;
    fn set_anchor_index(&self, value: i32);
    fn count(&self) -> usize;

    fn indexes_changed(&self, handler: Rc<dyn Fn(&SelectionModelIndexesChangedEventArgs)>) -> Rc<dyn IDisposable>;
    fn selection_changed(
        &self,
        handler: Rc<dyn Fn(&dyn SelectionModelSelectionChangedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    fn lost_selection(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
    fn source_reset(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;

    fn begin_batch_update(&self);
    fn end_batch_update(&self);
    fn is_selected(&self, index: i32) -> bool;
    fn select(&self, index: i32);
    fn deselect(&self, index: i32);
    fn select_range(&self, start: i32, end: i32);
    fn deselect_range(&self, start: i32, end: i32);
    fn select_all(&self);
    fn clear(&self);

    /// The model viewed as the internal selection model of the selecting
    /// items controls, if it is one.
    fn as_internal_selection_model(&self) -> Option<&super::InternalSelectionModel> {
        None
    }
}

impl PartialEq for dyn ISelectionModel {
    /// Selection models compare by identity.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// Whether two handles refer to the same selection model.
pub fn selection_model_ptr_eq(a: &Rc<dyn ISelectionModel>, b: &Rc<dyn ISelectionModel>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// Extension methods of [`ISelectionModel`].
pub struct SelectionModelExtensions;

impl SelectionModelExtensions {
    /// Begins a batch update that ends when the returned operation is
    /// disposed.
    pub fn batch_update(model: &Rc<dyn ISelectionModel>) -> BatchUpdateOperation {
        BatchUpdateOperation::new(model.clone())
    }
}

/// A batch update of a selection model in progress; disposing it ends the
/// update.
pub struct BatchUpdateOperation {
    owner: Rc<dyn ISelectionModel>,
    is_disposed: Cell<bool>,
}

impl BatchUpdateOperation {
    pub fn new(owner: Rc<dyn ISelectionModel>) -> Self {
        owner.begin_batch_update();
        Self { owner, is_disposed: Cell::new(false) }
    }
}

impl IDisposable for BatchUpdateOperation {
    fn dispose(&self) {
        if !self.is_disposed.get() {
            self.owner.end_batch_update();
            self.is_disposed.set(true);
        }
    }
}
