use super::i_selection_model::ISelectionModel;
use super::read_only_selection_list_base::IReadOnlySelectionList;
use super::selection_model::{finally, SelectionModel, SelectionModelImpl};
use super::selection_model_indexes_changed_event_args::SelectionModelIndexesChangedEventArgs;
use super::selection_model_selection_changed_event_args::{
    SelectionModelSelectionChangedEventArgs, SelectionModelSelectionChangedEventArgsOf,
};
use crate::items_source::{items_equal, ItemsChangedEventArgs, ItemsSource, ItemsView};
use ferroui_base::collections::{
    CollectionChangedHandler, FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs,
};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::reactive::IDisposable;
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// The list of selected items that can be written to: a notifying list of
/// untyped, nullable items.
pub type WritableSelectedItems = Rc<FerroList<Option<BoxedValue>>>;

/// The selection model of the selecting items controls: an untyped
/// selection model that keeps a writable list of the selected items in sync
/// with the selection, both ways.
///
/// It derives from the untyped [`SelectionModel`]: it owns the model,
/// overrides its virtual members through [`SelectionModelImpl`] and
/// dereferences to it, so every member of the model is available on it. It
/// implements [`ISelectionModel`] itself, so an `Rc<InternalSelectionModel>`
/// is used wherever an `Rc<dyn ISelectionModel>` is expected.
pub struct InternalSelectionModel {
    model: Rc<SelectionModel<BoxedValue>>,
    writable_selected_items: RefCell<Option<WritableSelectedItems>>,
    selected_items_handler: Rc<CollectionChangedHandler<Option<BoxedValue>>>,
    selected_items_subscription: Cell<Option<u64>>,
    ignore_model_changes: Cell<i32>,
    ignore_selected_items_changes: Cell<bool>,
    skip_sync_from_selected_items: Cell<bool>,
    is_resetting: Cell<bool>,
}

impl Deref for InternalSelectionModel {
    type Target = SelectionModel<BoxedValue>;

    fn deref(&self) -> &SelectionModel<BoxedValue> {
        &self.model
    }
}

impl InternalSelectionModel {
    pub fn new() -> Rc<Self> {
        let result = Rc::new_cyclic(|this: &Weak<Self>| {
            let weak = this.clone();
            Self {
                model: SelectionModel::new(),
                writable_selected_items: RefCell::new(None),
                selected_items_handler: Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Option<BoxedValue>>| {
                    if let Some(this) = weak.upgrade() {
                        this.on_selected_items_collection_changed(&e.into());
                    }
                }),
                selected_items_subscription: Cell::new(None),
                ignore_model_changes: Cell::new(0),
                ignore_selected_items_changes: Cell::new(false),
                skip_sync_from_selected_items: Cell::new(false),
                is_resetting: Cell::new(false),
            }
        });

        let weak: Weak<Self> = Rc::downgrade(&result);
        let derived: Weak<dyn SelectionModelImpl<BoxedValue>> = weak;
        result.model.set_derived(derived);

        let this = Rc::downgrade(&result);
        result.model.selection_changed(move |e| {
            if let Some(this) = this.upgrade() {
                this.on_selection_changed(e);
            }
        });

        let this = Rc::downgrade(&result);
        result.model.source_reset(move || {
            if let Some(this) = this.upgrade() {
                this.sync_from_selected_items();
            }
        });

        result
    }

    /// The untyped selection model this model derives from.
    pub fn model(&self) -> &Rc<SelectionModel<BoxedValue>> {
        &self.model
    }

    /// Gets the writable list of the selected items.
    pub fn writable_selected_items(&self) -> WritableSelectedItems {
        let items = self.writable_selected_items.borrow().clone();

        match items {
            Some(items) => items,
            None => {
                let items: WritableSelectedItems = Rc::new(FerroList::new());
                *self.writable_selected_items.borrow_mut() = Some(items.clone());
                self.subscribe_to_selected_items();
                items
            }
        }
    }

    /// Sets the writable list of the selected items; `None` assigns a new,
    /// empty list.
    pub fn set_writable_selected_items(&self, value: Option<WritableSelectedItems>) {
        let value = value.unwrap_or_else(|| Rc::new(FerroList::new()));
        let current = self.writable_selected_items.borrow().clone();

        if !current.is_some_and(|current| Rc::ptr_eq(&current, &value)) {
            self.unsubscribe_from_selected_items();
            *self.writable_selected_items.borrow_mut() = Some(value.clone());
            self.sync_from_selected_items();
            self.subscribe_to_selected_items();

            if self.model.node().items_view().is_none() {
                self.model.set_init_selected_items(ItemsSource::from(value));
            }

            self.model.raise_property_changed("WritableSelectedItems");
        }
    }

    /// Assigns the source and, if given, the writable list of the selected
    /// items in one step.
    pub fn update(&self, source: Option<ItemsSource>, selected_items: Option<Option<WritableSelectedItems>>) {
        let previous_source = self.model.source();
        let previous_writable_selected_items = self.writable_selected_items.borrow().clone();

        self.model.base_on_source_collection_change_started();

        {
            let _finally = finally(|| self.skip_sync_from_selected_items.set(false));
            self.skip_sync_from_selected_items.set(true);
            self.model.set_source(source);

            if let Some(selected_items) = selected_items {
                self.set_writable_selected_items(selected_items);
            }
        }

        let writable_selected_items = self.writable_selected_items.borrow().clone();
        let writable_selected_items_changed = match (&previous_writable_selected_items, &writable_selected_items) {
            (Some(previous), Some(current)) => !Rc::ptr_eq(previous, current),
            (None, None) => false,
            _ => true,
        };

        // We skipped the sync from WritableSelectedItems before; do it now that both
        // the source and WritableSelectedItems are updated.
        if writable_selected_items_changed {
            self.model.base_on_source_collection_change_finished();
            self.sync_from_selected_items();
        } else if previous_source != self.model.source() {
            self.sync_from_selected_items();
            self.model.base_on_source_collection_change_finished();
        } else {
            self.model.base_on_source_collection_change_finished();
        }
    }

    fn sync_to_selected_items(&self) {
        let writable_selected_items = self.writable_selected_items.borrow().clone();

        if let Some(writable_selected_items) = writable_selected_items {
            let selected_items = self.model.selected_items();

            if !Self::sequence_equal(&writable_selected_items, &*selected_items) {
                let _finally = finally(|| self.ignore_selected_items_changes.set(false));
                self.ignore_selected_items_changes.set(true);
                writable_selected_items.clear();

                for i in selected_items.iter() {
                    writable_selected_items.add(i);
                }
            }
        }
    }

    fn sync_from_selected_items(&self) {
        let writable_selected_items = self.writable_selected_items.borrow().clone();
        let source = self.model.source();

        let (Some(source), Some(writable_selected_items)) = (source, writable_selected_items) else {
            return;
        };

        if self.skip_sync_from_selected_items.get() {
            return;
        }

        let _finally = finally(|| self.ignore_model_changes.set(self.ignore_model_changes.get() - 1));
        self.ignore_model_changes.set(self.ignore_model_changes.get() + 1);

        self.model.begin_batch_update();
        self.model.clear();

        let mut i = 0;

        while i < writable_selected_items.count() {
            let index = source.index_of(&writable_selected_items.get(i));

            if index != -1 {
                self.model.select(index);
                i += 1;
            } else {
                writable_selected_items.remove_at(i);
            }
        }

        self.model.end_batch_update();
    }

    fn subscribe_to_selected_items(&self) {
        let writable_selected_items = self.writable_selected_items.borrow().clone();

        if let Some(writable_selected_items) = writable_selected_items {
            let token = writable_selected_items.add_collection_changed(self.selected_items_handler.clone());
            self.selected_items_subscription.set(Some(token));
        }
    }

    fn unsubscribe_from_selected_items(&self) {
        let writable_selected_items = self.writable_selected_items.borrow().clone();

        if let (Some(writable_selected_items), Some(token)) =
            (writable_selected_items, self.selected_items_subscription.take())
        {
            writable_selected_items.remove_collection_changed(token);
        }
    }

    fn on_selection_changed(&self, e: &SelectionModelSelectionChangedEventArgsOf<BoxedValue>) {
        if self.ignore_model_changes.get() > 0 {
            return;
        }

        let _finally = finally(|| self.ignore_selected_items_changes.set(false));
        let items = self.writable_selected_items();
        let deselected = e.deselected_items().to_vec();
        let selected = e.selected_items().to_vec();

        self.ignore_selected_items_changes.set(true);

        for i in deselected {
            items.remove(&i);
        }

        for i in selected {
            items.add(i);
        }
    }

    fn on_selected_items_collection_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        if self.ignore_selected_items_changes.get() {
            return;
        }

        let Some(writable_selected_items) = self.writable_selected_items.borrow().clone() else {
            panic!("CollectionChanged raised but we don't have items.");
        };

        let remove = || {
            for i in e.old_items {
                let index = Self::index_of(&self.model.source(), &i);

                if index != -1 {
                    self.model.deselect(index);
                }
            }
        };

        let _finally = finally(|| self.ignore_model_changes.set(self.ignore_model_changes.get() - 1));
        self.model.begin_batch_update();
        self.ignore_model_changes.set(self.ignore_model_changes.get() + 1);

        match e.action {
            NotifyCollectionChangedAction::Add => self.add(e.new_items),
            NotifyCollectionChangedAction::Remove => remove(),
            NotifyCollectionChangedAction::Replace => {
                remove();
                self.add(e.new_items);
            }
            NotifyCollectionChangedAction::Reset => {
                self.model.clear();
                self.add(ItemsView::from_slice(&writable_selected_items.snapshot()));
            }
            NotifyCollectionChangedAction::Move => {}
        }

        // The update is committed while changes of the model are still ignored.
        self.model.end_batch_update();
    }

    fn add(&self, new_items: ItemsView<'_>) {
        for i in new_items {
            let index = Self::index_of(&self.model.source(), &i);

            if index != -1 {
                self.model.select(index);
            }
        }
    }

    fn index_of(source: &Option<ItemsSource>, item: &Option<BoxedValue>) -> i32 {
        match source {
            Some(source) => source.index_of(item),
            None => -1,
        }
    }

    fn sequence_equal(
        first: &FerroList<Option<BoxedValue>>,
        second: &dyn IReadOnlySelectionList<Option<BoxedValue>>,
    ) -> bool {
        let first = first.snapshot();
        let mut e2 = second.iter();

        for item in first.iter() {
            match e2.next() {
                Some(other) if items_equal(item, &other) => {}
                _ => return false,
            }
        }

        e2.next().is_none()
    }
}

impl SelectionModelImpl<BoxedValue> for InternalSelectionModel {
    fn set_source(&self, model: &SelectionModel<BoxedValue>, value: Option<ItemsSource>) {
        if model.source() == value {
            return;
        }

        let mut old_selection: Option<Vec<Option<BoxedValue>>> = None;

        if model.source().is_some() && value.is_some() {
            old_selection = Some(self.writable_selected_items().to_vec());
        }

        {
            let _finally = finally(|| {
                self.ignore_model_changes.set(self.ignore_model_changes.get() - 1);
                self.ignore_selected_items_changes.set(false);
            });
            self.ignore_selected_items_changes.set(true);
            self.ignore_model_changes.set(self.ignore_model_changes.get() + 1);
            model.base_set_source(value);
        }

        if old_selection.is_none() {
            self.sync_to_selected_items();
        } else {
            self.sync_from_selected_items();
        }
    }

    fn on_source_collection_changed(&self, model: &SelectionModel<BoxedValue>, e: &ItemsChangedEventArgs<'_>) {
        if e.action == NotifyCollectionChangedAction::Reset {
            self.ignore_model_changes.set(self.ignore_model_changes.get() + 1);
            self.is_resetting.set(true);
        }

        model.base_on_source_collection_changed(e);
    }

    fn on_source_collection_change_finished(&self, model: &SelectionModel<BoxedValue>) {
        model.base_on_source_collection_change_finished();

        if self.is_resetting.get() {
            self.ignore_model_changes.set(self.ignore_model_changes.get() - 1);
            self.is_resetting.set(false);
        }
    }
}

impl INotifyPropertyChanged for InternalSelectionModel {
    fn property_changed(&self) -> &Event<str> {
        self.model.property_changed()
    }
}

impl ISelectionModel for InternalSelectionModel {
    fn as_internal_selection_model(&self) -> Option<&InternalSelectionModel> {
        Some(self)
    }

    fn source(&self) -> Option<ItemsSource> {
        self.model.source()
    }

    fn set_source(&self, value: Option<ItemsSource>) {
        self.model.set_source(value);
    }

    fn single_select(&self) -> bool {
        self.model.single_select()
    }

    fn set_single_select(&self, value: bool) {
        self.model.set_single_select(value);
    }

    fn selected_index(&self) -> i32 {
        self.model.selected_index()
    }

    fn set_selected_index(&self, value: i32) {
        self.model.set_selected_index(value);
    }

    fn selected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>> {
        self.model.selected_indexes()
    }

    fn selected_item(&self) -> Option<BoxedValue> {
        ISelectionModel::selected_item(&*self.model)
    }

    fn set_selected_item(&self, value: Option<BoxedValue>) {
        ISelectionModel::set_selected_item(&*self.model, value);
    }

    fn selected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<BoxedValue>>> {
        ISelectionModel::selected_items(&*self.model)
    }

    fn anchor_index(&self) -> i32 {
        self.model.anchor_index()
    }

    fn set_anchor_index(&self, value: i32) {
        self.model.set_anchor_index(value);
    }

    fn count(&self) -> usize {
        self.model.count()
    }

    fn indexes_changed(&self, handler: Rc<dyn Fn(&SelectionModelIndexesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        ISelectionModel::indexes_changed(&*self.model, handler)
    }

    fn selection_changed(
        &self,
        handler: Rc<dyn Fn(&dyn SelectionModelSelectionChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        ISelectionModel::selection_changed(&*self.model, handler)
    }

    fn lost_selection(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        ISelectionModel::lost_selection(&*self.model, handler)
    }

    fn source_reset(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        ISelectionModel::source_reset(&*self.model, handler)
    }

    fn begin_batch_update(&self) {
        self.model.begin_batch_update();
    }

    fn end_batch_update(&self) {
        self.model.end_batch_update();
    }

    fn is_selected(&self, index: i32) -> bool {
        self.model.is_selected(index)
    }

    fn select(&self, index: i32) {
        self.model.select(index);
    }

    fn deselect(&self, index: i32) {
        self.model.deselect(index);
    }

    fn select_range(&self, start: i32, end: i32) {
        self.model.select_range(start, end);
    }

    fn deselect_range(&self, start: i32, end: i32) {
        self.model.deselect_range(start, end);
    }

    fn select_all(&self) {
        self.model.select_all();
    }

    fn clear(&self) {
        self.model.clear();
    }
}
