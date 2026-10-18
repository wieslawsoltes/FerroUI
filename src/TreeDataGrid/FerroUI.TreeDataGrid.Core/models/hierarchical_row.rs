//! `Models/HierarchicalRow.cs`: a row of a hierarchical source, which expands to show the rows of
//! its child models.

use super::sortable_rows_base::{SortableRow, SortableRowsBase, SortableRowsChangedArgs};
use super::{
    Comparison, IExpander, IExpanderColumn, IExpanderRow, IExpanderRowController, IIndentedRow, IModelIndexableRow, IRow,
    IRowOf, NotifyingBase, RowRef, RowsChangedArgs,
};
use crate::accessor::{as_notifier, PathSubscription, TreeModel};
use crate::model_collection::Models;
use crate::{GridLength, IndexPath, TreeDataGridItemsSourceView};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(crate) type ChildRows<M> = SortableRowsBase<M, Rc<HierarchicalRow<M>>>;

fn reset_rows_args<'a>() -> RowsChangedArgs<'a> {
    RowsChangedArgs {
        action: NotifyCollectionChangedAction::Reset,
        new_items: &[],
        old_items: &[],
        new_starting_index: -1,
        old_starting_index: -1,
    }
}

fn same_models<M: 'static>(a: &Option<Models<M>>, b: &Option<Models<M>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        _ => false,
    }
}

/// A row of a hierarchical source.
pub struct HierarchicalRow<M: TreeModel> {
    weak_self: Weak<Self>,
    notifying: NotifyingBase,
    controller: Weak<dyn IExpanderRowController<M>>,
    expander_column: Rc<dyn IExpanderColumn<M>>,
    comparison: RefCell<Option<Comparison<M>>>,
    child_models: RefCell<Option<Models<M>>>,
    child_rows: RefCell<Option<Rc<ChildRows<M>>>>,
    child_rows_token: Cell<Option<u64>>,
    is_expanded_subscription: RefCell<Option<PathSubscription<M>>>,
    observes_expansion_via_model: Cell<bool>,
    model_notifications: Cell<Option<u64>>,
    is_expanded: Cell<bool>,
    show_expander: Cell<Option<bool>>,
    model_index_path: RefCell<IndexPath>,
    // The length of the model index path, which a row keeps for its life (its updates change the
    // indexes, not the depth): the indent is read without the path.
    depth: usize,
    model: Rc<M>,
}

impl<M: TreeModel> HierarchicalRow<M> {
    /// A row of `model` at `model_index`.
    ///
    /// # Panics
    /// For an empty model index (`ArgumentException`).
    pub fn new(
        controller: Weak<dyn IExpanderRowController<M>>,
        expander_column: Rc<dyn IExpanderColumn<M>>,
        model_index: IndexPath,
        model: Rc<M>,
        comparison: Option<Comparison<M>>,
    ) -> Rc<Self> {
        if model_index.count() == 0 {
            panic!("Invalid model index");
        }
        let this = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            notifying: NotifyingBase::new(),
            controller,
            expander_column: expander_column.clone(),
            comparison: RefCell::new(comparison.clone()),
            child_models: RefCell::new(None),
            child_rows: RefCell::new(None),
            child_rows_token: Cell::new(None),
            is_expanded_subscription: RefCell::new(None),
            observes_expansion_via_model: Cell::new(false),
            model_notifications: Cell::new(None),
            is_expanded: Cell::new(false),
            show_expander: Cell::new(None),
            depth: model_index.count(),
            model_index_path: RefCell::new(model_index),
            model: model.clone(),
        });

        let expanded = expander_column.get_model_is_expanded(&model);
        if expanded == Some(true) && expander_column.has_children(&model) {
            let child_models = expander_column.get_child_models(&model);
            *this.child_models.borrow_mut() = child_models.clone();
            let rows = this.create_child_rows(child_models, comparison);
            this.is_expanded.set(rows.count() > 0);
            this.set_child_rows(Some(rows));
        }
        if expanded.is_some() {
            if let Some(observer) = expander_column.expansion_observer() {
                let weak = Rc::downgrade(&this);
                let subscription = observer.subscribe(
                    &model,
                    Rc::new(move || {
                        if let Some(this) = weak.upgrade() {
                            this.on_model_is_expanded_changed();
                        }
                    }),
                );
                *this.is_expanded_subscription.borrow_mut() = Some(subscription);
            }
        }
        let via_model = expanded.is_some() && this.is_expanded_subscription.borrow().is_none();
        this.observes_expansion_via_model.set(via_model);
        if via_model || this.is_expanded.get() {
            this.subscribe_to_model_changes();
        }
        this
    }

    pub(crate) fn this(&self) -> Rc<Self> {
        self.weak_self.upgrade().expect("a row that is alive")
    }

    /// The shared row.
    pub fn shared(&self) -> Rc<Self> {
        self.this()
    }

    fn controller(&self) -> Option<Rc<dyn IExpanderRowController<M>>> {
        self.controller.upgrade()
    }

    /// The rows of the children while the row is expanded, without copying them.
    pub(crate) fn expanded_child_rows(&self) -> Option<Rc<ChildRows<M>>> {
        if self.is_expanded.get() {
            self.child_rows.borrow().clone()
        } else {
            None
        }
    }

    /// The child rows while the row is expanded (`Children`).
    pub fn children(&self) -> Option<Vec<Rc<HierarchicalRow<M>>>> {
        if self.is_expanded.get() {
            self.child_rows.borrow().clone().map(|rows| rows.to_vec())
        } else {
            None
        }
    }

    /// The number of child rows while the row is expanded.
    pub fn children_count(&self) -> Option<usize> {
        if self.is_expanded.get() {
            self.child_rows.borrow().as_ref().map(|rows| rows.count())
        } else {
            None
        }
    }

    /// The child row at `index` while the row is expanded.
    pub fn child(&self, index: usize) -> Option<Rc<HierarchicalRow<M>>> {
        if self.is_expanded.get() {
            self.child_rows.borrow().clone().map(|rows| rows.get(index))
        } else {
            None
        }
    }

    /// The child rows whether the row is expanded or not (`MaterializedChildren`).
    pub fn materialized_children(&self) -> Option<Vec<Rc<HierarchicalRow<M>>>> {
        self.child_rows.borrow().clone().map(|rows| rows.to_vec())
    }

    /// The child rows object, whether the row is expanded or not.
    pub fn materialized_child_rows(&self) -> Option<Rc<SortableRowsBase<M, Rc<HierarchicalRow<M>>>>> {
        self.child_rows.borrow().clone()
    }

    pub fn model_index(&self) -> i32 {
        self.model_index_path.borrow().last()
    }

    pub fn model_index_path(&self) -> IndexPath {
        self.model_index_path.borrow().clone()
    }

    /// Whether the model index path of the row is `path` (without cloning the path).
    pub(crate) fn has_model_index_path(&self, path: &IndexPath) -> bool {
        *self.model_index_path.borrow() == *path
    }

    pub fn indent(&self) -> i32 {
        self.depth as i32 - 1
    }

    pub fn model(&self) -> &Rc<M> {
        &self.model
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        if self.is_expanded.get() != value {
            if value {
                self.expand();
            } else {
                self.collapse();
            }
        }
    }

    pub fn show_expander(&self) -> bool {
        self.show_expander.get().unwrap_or_else(|| self.expander_column.has_children(&self.model))
    }

    fn set_show_expander(&self, value: bool) {
        self.notifying.raise_and_set_cell_if_changed(&self.show_expander, Some(value), "ShowExpander");
    }

    /// The `PropertyChanged` of the row.
    pub fn notifying(&self) -> &NotifyingBase {
        &self.notifying
    }

    /// Releases the subscriptions and the child rows (`Dispose`).
    pub fn dispose(&self) {
        if let Some(subscription) = self.is_expanded_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.unsubscribe_from_model_changes();
        let rows = self.child_rows.borrow().clone();
        if let Some(rows) = rows {
            rows.dispose();
        }
    }

    fn on_model_property_changed(&self) {
        if self.is_expanded.get() || self.show_expander.get() == Some(false) {
            self.refresh_child_models();
        }
        if self.observes_expansion_via_model.get() {
            self.on_model_is_expanded_changed();
        }
    }

    fn on_model_is_expanded_changed(&self) {
        if let Some(expanded) = self.expander_column.get_model_is_expanded(&self.model) {
            self.set_is_expanded(expanded);
        }
    }

    /// Moves the row's model index by `delta` (`UpdateModelIndex`).
    pub fn update_model_index(&self, delta: i32) {
        let path = {
            let path = self.model_index_path.borrow();
            path.parent().append(path.last() + delta)
        };
        *self.model_index_path.borrow_mut() = path.clone();
        self.update_children_parent_index(&path);
    }

    /// Gives the row a new parent model index (`UpdateParentModelIndex`).
    pub fn update_parent_model_index(&self, parent_index: &IndexPath) {
        let path = parent_index.append(self.model_index());
        *self.model_index_path.borrow_mut() = path.clone();
        self.update_children_parent_index(&path);
    }

    fn update_children_parent_index(&self, path: &IndexPath) {
        let rows = self.child_rows.borrow().clone();
        if let Some(rows) = rows {
            for i in 0..rows.count() {
                rows.get(i).update_parent_model_index(path);
            }
        }
    }

    /// Sets whether the expander is shown (`UpdateShowExpander`).
    pub fn update_show_expander(&self, value: bool) {
        self.set_show_expander(value);
    }

    /// Forgets an explicit expander state, so that it follows whether the model has children.
    pub fn clear_show_expander(&self) {
        if self.show_expander.get().is_some() {
            self.show_expander.set(None);
            self.notifying.raise_property_changed("ShowExpander");
        }
    }

    /// Sorts the child rows and theirs (`SortChildren`).
    pub fn sort_children(&self, comparison: Option<Comparison<M>>) {
        *self.comparison.borrow_mut() = comparison.clone();
        let rows = self.child_rows.borrow().clone();
        let Some(rows) = rows else {
            return;
        };
        rows.sort(comparison.clone());
        for row in rows.to_vec() {
            row.sort_children(comparison.clone());
        }
    }

    fn create_child_rows(&self, child_models: Option<Models<M>>, comparison: Option<Comparison<M>>) -> Rc<ChildRows<M>> {
        let owner = self.weak_self.clone();
        ChildRows::new(TreeDataGridItemsSourceView::get_or_create(child_models), comparison, move |model_index, model| {
            let owner = owner.upgrade().expect("the owner of child rows");
            let comparison = owner.comparison.borrow().clone();
            HierarchicalRow::new(
                owner.controller.clone(),
                owner.expander_column.clone(),
                owner.model_index_path().append(model_index),
                model,
                comparison,
            )
        })
    }

    /// Makes `rows` the child rows, listening to their changes (`ChildRows.OnCollectionChanged`).
    fn set_child_rows(&self, rows: Option<Rc<ChildRows<M>>>) {
        if let Some(token) = self.child_rows_token.take() {
            if let Some(old) = self.child_rows.borrow().as_ref() {
                old.remove_collection_changed(token);
            }
        }
        if let Some(rows) = &rows {
            let owner = self.weak_self.clone();
            let token = rows.add_collection_changed(Rc::new(move |e: &SortableRowsChangedArgs<'_, Rc<HierarchicalRow<M>>>| {
                if let Some(owner) = owner.upgrade() {
                    owner.on_child_rows_changed(e);
                }
            }));
            self.child_rows_token.set(Some(token));
        }
        *self.child_rows.borrow_mut() = rows;
    }

    fn on_child_rows_changed(&self, e: &SortableRowsChangedArgs<'_, Rc<HierarchicalRow<M>>>) {
        self.clear_show_expander();
        self.update_model_change_subscription();
        if self.is_expanded.get() {
            if let Some(controller) = self.controller() {
                let new_items: Vec<RowRef> = e.new_items.iter().map(|r| r.clone() as RowRef).collect();
                let old_items: Vec<RowRef> = e.old_items.iter().map(|r| r.clone() as RowRef).collect();
                let args = RowsChangedArgs {
                    action: e.action,
                    new_items: &new_items,
                    old_items: &old_items,
                    new_starting_index: e.new_starting_index,
                    old_starting_index: e.old_starting_index,
                };
                controller.on_child_collection_changed(&*self.this(), &args);
            }
        }
    }

    fn expand(&self) {
        if !self.expander_column.has_children(&self.model) {
            self.expander_column.set_model_is_expanded(&*self.this());
            return;
        }
        let old_expanded = self.is_expanded.get();
        let child_models = self.expander_column.get_child_models(&self.model);
        if !same_models(&self.child_models.borrow(), &child_models) {
            *self.child_models.borrow_mut() = child_models.clone();
            let old = self.child_rows.borrow().clone();
            if let Some(old) = old {
                old.dispose();
            }
            let rows = self.create_child_rows(child_models, self.comparison.borrow().clone());
            self.set_child_rows(Some(rows));
        }
        let child_count = self.child_rows.borrow().as_ref().map(|rows| rows.count()).unwrap_or(0);
        let controller = self.controller();
        if child_count > 0 {
            if let Some(controller) = &controller {
                controller.on_begin_expand_collapse(&*self.this());
            }
            self.is_expanded.set(true);
        } else {
            self.set_show_expander(false);
            if let Some(rows) = self.child_rows.borrow().clone() {
                // Reading the rows creates them, so that the changes of the empty children are
                // observed (`ObserveChangesWhileEmpty`).
                rows.to_vec();
            }
        }
        if let Some(controller) = &controller {
            controller.on_child_collection_changed(&*self.this(), &reset_rows_args());
        }
        if self.is_expanded.get() != old_expanded {
            self.notifying.raise_property_changed("IsExpanded");
            if let Some(controller) = &controller {
                controller.on_end_expand_collapse(&*self.this());
            }
        }
        self.expander_column.set_model_is_expanded(&*self.this());
        self.update_model_change_subscription();
    }

    fn refresh_child_models(&self) {
        let child_models = self.expander_column.get_child_models(&self.model);
        if same_models(&self.child_models.borrow(), &child_models) {
            return;
        }
        let old_notifying = self.child_models.borrow().as_ref().is_some_and(|m| m.is_notifying());
        let new_notifying = child_models.as_ref().is_some_and(|m| m.is_notifying());
        if !old_notifying && !new_notifying {
            let rows = self.child_rows.borrow().clone();
            if rows.is_some_and(|rows| rows.has_same_items(child_models.as_ref())) {
                return;
            }
        }
        let old_expanded = self.is_expanded.get();
        let replacement = self.create_child_rows(child_models.clone(), self.comparison.borrow().clone());
        let new_expanded = replacement.count() > 0;
        let controller = self.controller();
        if new_expanded != old_expanded {
            if let Some(controller) = &controller {
                controller.on_begin_expand_collapse(&*self.this());
            }
        }
        *self.child_models.borrow_mut() = child_models.clone();
        let old = self.child_rows.borrow().clone();
        if let Some(old) = old {
            old.dispose();
        }
        self.set_child_rows(Some(replacement.clone()));
        self.is_expanded.set(new_expanded);
        if let Some(controller) = &controller {
            if controller.replaces_child_collections() {
                controller.on_child_collection_replaced(&*self.this(), child_models.as_ref());
            }
        }
        if new_expanded {
            self.clear_show_expander();
        } else {
            self.set_show_expander(false);
            replacement.to_vec();
        }
        if let Some(controller) = &controller {
            controller.on_child_collection_changed(&*self.this(), &reset_rows_args());
        }
        if self.is_expanded.get() != old_expanded {
            self.notifying.raise_property_changed("IsExpanded");
            if let Some(controller) = &controller {
                controller.on_end_expand_collapse(&*self.this());
            }
            self.expander_column.set_model_is_expanded(&*self.this());
        }
        self.update_model_change_subscription();
    }

    fn subscribe_to_model_changes(&self) {
        if self.model_notifications.get().is_some() {
            return;
        }
        if let Some(notifier) = as_notifier(&*self.model) {
            let weak = self.weak_self.clone();
            let token = notifier.property_changed().add(Rc::new(move |_name: &str| {
                if let Some(this) = weak.upgrade() {
                    this.on_model_property_changed();
                }
            }));
            self.model_notifications.set(Some(token));
        }
    }

    fn unsubscribe_from_model_changes(&self) {
        if let Some(token) = self.model_notifications.take() {
            if let Some(notifier) = as_notifier(&*self.model) {
                notifier.property_changed().remove(token);
            }
        }
    }

    fn update_model_change_subscription(&self) {
        if self.observes_expansion_via_model.get() || self.is_expanded.get() || self.show_expander.get() == Some(false) {
            self.subscribe_to_model_changes();
        } else {
            self.unsubscribe_from_model_changes();
        }
    }

    fn collapse(&self) {
        let controller = self.controller();
        if let Some(controller) = &controller {
            controller.on_begin_expand_collapse(&*self.this());
        }
        self.is_expanded.set(false);
        if let Some(controller) = &controller {
            controller.on_child_collection_changed(&*self.this(), &reset_rows_args());
        }
        self.notifying.raise_property_changed("IsExpanded");
        if let Some(controller) = &controller {
            controller.on_end_expand_collapse(&*self.this());
        }
        self.expander_column.set_model_is_expanded(&*self.this());
        self.update_model_change_subscription();
    }
}

impl<M: TreeModel> INotifyPropertyChanged for HierarchicalRow<M> {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl<M: TreeModel> IRow for HierarchicalRow<M> {
    fn header(&self) -> Option<BoxedValue> {
        Some(Rc::new(self.model_index_path()) as BoxedValue)
    }

    fn height(&self) -> GridLength {
        GridLength::AUTO
    }

    fn set_height(&self, _value: GridLength) {}

    fn model(&self) -> Option<BoxedValue> {
        Some(self.model.clone() as BoxedValue)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_indented(&self) -> Option<&dyn IIndentedRow> {
        Some(self)
    }

    fn as_model_indexable(&self) -> Option<&dyn IModelIndexableRow> {
        Some(self)
    }

    fn as_expander(&self) -> Option<&dyn IExpander> {
        Some(self)
    }

    fn as_notify_property_changed(&self) -> Option<&dyn INotifyPropertyChanged> {
        Some(self)
    }
}

impl<M: TreeModel> IRowOf<M> for HierarchicalRow<M> {
    fn model_of(&self) -> Rc<M> {
        self.model.clone()
    }

    fn update_model_index(&self, delta: i32) {
        HierarchicalRow::update_model_index(self, delta);
    }
}

impl<M: TreeModel> IIndentedRow for HierarchicalRow<M> {
    fn indent(&self) -> i32 {
        HierarchicalRow::indent(self)
    }
}

impl<M: TreeModel> IModelIndexableRow for HierarchicalRow<M> {
    fn model_index(&self) -> i32 {
        HierarchicalRow::model_index(self)
    }

    fn model_index_path(&self) -> IndexPath {
        HierarchicalRow::model_index_path(self)
    }
}

impl<M: TreeModel> IExpander for HierarchicalRow<M> {
    fn is_expanded(&self) -> bool {
        HierarchicalRow::is_expanded(self)
    }

    fn set_is_expanded(&self, value: bool) {
        HierarchicalRow::set_is_expanded(self, value);
    }

    fn show_expander(&self) -> bool {
        HierarchicalRow::show_expander(self)
    }
}

impl<M: TreeModel> IExpanderRow<M> for HierarchicalRow<M> {
    fn update_show_expander(&self, value: bool) {
        HierarchicalRow::update_show_expander(self, value);
    }
}

impl<M: TreeModel> SortableRow for Rc<HierarchicalRow<M>> {
    fn model_index(&self) -> i32 {
        HierarchicalRow::model_index(self)
    }

    fn update_model_index(&self, delta: i32) {
        HierarchicalRow::update_model_index(self, delta);
    }

    fn dispose(&self) {
        HierarchicalRow::dispose(self);
    }

    fn row_ref(&self) -> RowRef {
        self.clone() as RowRef
    }
}
