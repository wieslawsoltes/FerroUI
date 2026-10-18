//! `TreeDataGrid.V12.cs`: the grid of an `ItemsSource` and declarative columns
//! (`ColumnDefinitions`), from which it generates a source; the selection mode; and the control's
//! `SelectionChanged`.

use super::TreeDataGrid;
use crate::presentation::{apply_selection_mode_to, TreeDataGridSourcePresentation};
use crate::{TreeDataGridSelectionChangedEventArgs, TreeDataGridSelectionMode, TreeDataGridSourceRef};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use ferroui_controls::{ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource};
use ferroui_treedatagrid_core::model_collection::{ModelCollection, ModelsChangedHandler};
use ferroui_treedatagrid_core::{FlatTreeDataGridSource, HierarchicalTreeDataGridSource, Models};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The items of an `ItemsSource` as the models of a generated source (`GetSourceItems`): the
/// non-null items, each boxed once so that a model keeps its identity, following the changes of a
/// notifying list.
pub struct ItemsSourceModels {
    weak_self: Weak<Self>,
    source: ItemsSource,
    // The items of the source, with `None` for a null item, so that an index of the source maps
    // to an index of the models.
    items: RefCell<Vec<Option<Rc<BoxedValue>>>>,
    handlers: HandlerList<ModelsChangedHandler<BoxedValue>>,
    token: Cell<Option<u64>>,
}

impl ItemsSourceModels {
    /// The models of `source`.
    pub fn new(source: ItemsSource) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            source: source.clone(),
            items: RefCell::new(Vec::new()),
            handlers: HandlerList::new(),
            token: Cell::new(None),
        });
        this.rebuild();
        let weak = Rc::downgrade(&this);
        let handler: Rc<ItemsChangedHandler> = Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
            if let Some(this) = weak.upgrade() {
                this.on_items_changed(e);
            }
        });
        this.token.set(source.list().add_collection_changed(handler));
        this
    }

    /// The items source.
    pub fn source(&self) -> &ItemsSource {
        &self.source
    }

    fn box_item(item: Option<BoxedValue>) -> Option<Rc<BoxedValue>> {
        item.map(Rc::new)
    }

    fn rebuild(&self) {
        let items: Vec<Option<Rc<BoxedValue>>> = self.source.iter().map(Self::box_item).collect();
        *self.items.borrow_mut() = items;
    }

    // The index of the models of the item at `index` of the source: the non-null items before it.
    fn model_index(&self, index: usize) -> usize {
        self.items.borrow().iter().take(index).filter(|item| item.is_some()).count()
    }

    fn raise(&self, args: &NotifyCollectionChangedEventArgs<'_, Rc<BoxedValue>>) {
        for (_, handler) in self.handlers.snapshot().iter() {
            handler(args);
        }
    }

    fn raise_reset(&self) {
        let args = NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Reset,
            new_items: &[],
            old_items: &[],
            new_starting_index: -1,
            old_starting_index: -1,
        };
        self.raise(&args);
    }

    fn on_items_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        match e.action {
            NotifyCollectionChangedAction::Add if e.new_starting_index >= 0 => {
                let index = e.new_starting_index as usize;
                let added: Vec<Option<Rc<BoxedValue>>> = e.new_items.iter().map(Self::box_item).collect();
                let model_index = self.model_index(index);
                {
                    let mut items = self.items.borrow_mut();
                    let at = index.min(items.len());
                    for (offset, item) in added.iter().enumerate() {
                        items.insert(at + offset, item.clone());
                    }
                }
                let models: Vec<Rc<BoxedValue>> = added.into_iter().flatten().collect();
                if !models.is_empty() {
                    let args = NotifyCollectionChangedEventArgs {
                        action: NotifyCollectionChangedAction::Add,
                        new_items: &models,
                        old_items: &[],
                        new_starting_index: model_index as i32,
                        old_starting_index: -1,
                    };
                    self.raise(&args);
                }
            }
            NotifyCollectionChangedAction::Remove if e.old_starting_index >= 0 => {
                let index = e.old_starting_index as usize;
                let count = e.old_items.len();
                let model_index = self.model_index(index);
                let removed: Vec<Rc<BoxedValue>> = {
                    let mut items = self.items.borrow_mut();
                    let end = (index + count).min(items.len());
                    items.drain(index.min(end)..end).flatten().collect()
                };
                if !removed.is_empty() {
                    let args = NotifyCollectionChangedEventArgs {
                        action: NotifyCollectionChangedAction::Remove,
                        new_items: &[],
                        old_items: &removed,
                        new_starting_index: -1,
                        old_starting_index: model_index as i32,
                    };
                    self.raise(&args);
                }
            }
            _ => {
                // Replacements, moves and resets are shown as a reset of the models.
                self.rebuild();
                self.raise_reset();
            }
        }
    }

    /// The shared models.
    pub fn this(&self) -> Option<Rc<Self>> {
        self.weak_self.upgrade()
    }
}

impl ModelCollection<BoxedValue> for ItemsSourceModels {
    fn count(&self) -> usize {
        self.items.borrow().iter().filter(|item| item.is_some()).count()
    }

    fn get(&self, index: usize) -> Rc<BoxedValue> {
        self.items.borrow().iter().flatten().nth(index).cloned().expect("an index of the models")
    }

    fn add_collection_changed(&self, handler: Rc<ModelsChangedHandler<BoxedValue>>) -> Option<u64> {
        if !self.source.is_notifying() {
            return None;
        }
        Some(self.handlers.add(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.handlers.remove(token);
    }

    fn is_notifying(&self) -> bool {
        self.source.is_notifying()
    }

    fn to_vec(&self) -> Vec<Rc<BoxedValue>> {
        self.items.borrow().iter().flatten().cloned().collect()
    }
}

impl Drop for ItemsSourceModels {
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            self.source.list().remove_collection_changed(token);
        }
    }
}

impl TreeDataGrid {
    /// The items whose rows the declarative columns show (`ItemsSource`).
    pub fn items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_source_property())
    }

    /// Sets [`items_source`](Self::items_source).
    pub fn set_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::items_source_property(), value)
    }

    /// How rows or cells are selected (`SelectionMode`).
    pub fn selection_mode(&self) -> TreeDataGridSelectionMode {
        self.get_value(Self::selection_mode_property())
    }

    /// Sets [`selection_mode`](Self::selection_mode).
    pub fn set_selection_mode(&self, value: TreeDataGridSelectionMode) {
        self.set_value(Self::selection_mode_property(), value)
    }

    /// The declarative columns (`[Content] ColumnDefinitions`).
    pub fn column_definitions(&self) -> &crate::TreeDataGridColumns {
        &self.column_definitions
    }

    /// `SelectionChanged`: the rows or cells selected changed. Disposing the returned handle
    /// unsubscribes.
    pub fn selection_changed(&self, handler: impl Fn(&TreeDataGridSelectionChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.selection_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.selection_changed.remove(token);
            }
        })
    }

    /// The source generated from the items and the declarative columns.
    pub fn generated_source(&self) -> Option<TreeDataGridSourceRef> {
        self.generated_source.borrow().clone()
    }

    /// `InitializeV12Support`.
    pub(super) fn initialize_v12_support(&self) {
        let weak = self.to_ref().downgrade();
        let token = self.column_definitions.list().add_collection_changed(Rc::new(
            move |_: &NotifyCollectionChangedEventArgs<'_, Rc<dyn crate::TreeDataGridColumn>>| {
                if let Some(this) = weak.upgrade() {
                    this.rebuild_generated_source();
                }
            },
        ));
        self.column_definitions_token.set(Some(token));
    }

    /// `RebuildGeneratedSource`.
    pub(super) fn rebuild_generated_source(&self) {
        let items_source = self.items_source();
        let columns = self.column_definitions.list().to_vec();
        let Some(items_source) = items_source.filter(|_| !columns.is_empty()) else {
            *self.generated_source.borrow_mut() = None;
            self.update_active_source();
            return;
        };

        let sample = items_source.iter().flatten().next();

        for column in &columns {
            column.initialize_from_sample(sample.as_ref());
        }

        let items: Models<BoxedValue> = ItemsSourceModels::new(items_source);

        let source = if columns.iter().any(|column| column.is_hierarchical_expander()) {
            let source = HierarchicalTreeDataGridSource::<BoxedValue>::new(items);
            for column in &columns {
                source.columns().add(column.create_untyped_column());
            }
            TreeDataGridSourceRef::from(source)
        } else {
            let source = FlatTreeDataGridSource::<BoxedValue>::new(items);
            for column in &columns {
                source.columns().add(column.create_untyped_column());
            }
            TreeDataGridSourceRef::from(source)
        };

        *self.generated_source.borrow_mut() = Some(source);
        self.update_active_source();
    }

    /// `UpdateActiveSource`: the explicit source, or else the generated one, is the source shown
    /// when there is no model.
    pub(super) fn update_active_source(&self) {
        if self.model().is_some() {
            return;
        }
        let next = self.explicit_source.borrow().clone().or_else(|| self.generated_source.borrow().clone());
        let current = self.source.borrow().clone();
        let has_presentation = self.presentation.borrow().is_some();
        if current == next && (if next.is_none() { !has_presentation } else { has_presentation }) {
            self.apply_selection_mode();
            self.set_selection_interaction(self.presentation().and_then(|p| p.selection_interaction()));
            return;
        }
        let old_source = current;
        *self.source.borrow_mut() = next.clone();
        self.apply_selection_mode_to_source(next.as_ref());
        self.set_presentation(
            next.as_ref().map(|next| TreeDataGridSourcePresentation::new(next.source().clone(), None) as crate::presentation::PresentationRef),
        );
        self.raise_direct_property_changed(Self::source_property(), &old_source, &next);
        if let Some(rows_presenter) = self.rows_presenter() {
            rows_presenter.recycle_all_elements();
            rows_presenter.invalidate_measure();
        }
    }

    /// `ApplySelectionMode()`.
    pub(super) fn apply_selection_mode(&self) {
        if self.model().is_none() {
            let source = self.source.borrow().clone();
            self.apply_selection_mode_to_source(source.as_ref());
        } else if !self.updating_selection_mode.get() && self.is_set(Self::selection_mode_property().as_property()) {
            self.updating_selection_mode.set(true);
            struct Reset<'a>(&'a Cell<bool>);
            impl Drop for Reset<'_> {
                fn drop(&mut self) {
                    self.0.set(false);
                }
            }
            let _reset = Reset(&self.updating_selection_mode);
            if let Some(presentation) = self.presentation() {
                presentation.apply_selection_mode(self.selection_mode());
            }
        }
    }

    /// `OnNativeSelectionChanged`: the control raises the presentation's selection changes while
    /// it is attached.
    pub(super) fn on_native_selection_changed(&self, e: &TreeDataGridSelectionChangedEventArgs) {
        if self.is_attached_to_visual_tree.get() {
            for (_, handler) in self.selection_changed.snapshot().iter() {
                handler(e);
            }
        }
    }

    /// `ApplySelectionMode(ITreeDataGridSource?)`: the selection mode applied to the source shown
    /// when it is the generated source or the mode is set.
    fn apply_selection_mode_to_source(&self, source: Option<&TreeDataGridSourceRef>) {
        let Some(source) = source else {
            return;
        };
        if self.updating_selection_mode.get() || !self.should_apply_selection_mode(source) {
            return;
        }

        self.updating_selection_mode.set(true);
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.updating_selection_mode);
        apply_selection_mode_to(source.source(), self.selection_mode());
    }

    fn should_apply_selection_mode(&self, source: &TreeDataGridSourceRef) -> bool {
        self.generated_source.borrow().as_ref().is_some_and(|generated| generated == source)
            || self.is_set(Self::selection_mode_property().as_property())
    }
}
