//! `Primitives/TreeDataGridRowsPresenter.cs`: the presenter of the rows of the control, a vertical
//! virtualizing presenter of [`TreeDataGridRow`]s over the rows of a presentation.

use super::{
    PresenterItemsChange, TreeDataGridElementFactory, TreeDataGridPresenterBase, TreeDataGridPresenterHooks,
    TreeDataGridRow,
};
use crate::models::{IColumnLayoutBatch, IColumns, ITreeDataGridRows};
use crate::selection::ITreeDataGridSelectionInteraction;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{EffectiveViewportChangedEventArgs, LayoutableImpl, Orientation};
use ferroui_base::logical_tree::{ChildIndexChangedEventArgs, IChildIndexProvider, LogicalTreeAttachmentEventArgs};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{HandlerList, MathUtilities};
use ferroui_base::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElement, StyledElementImpl, StyledElementImplExt,
    StyledProperty, StyledPropertyOptions, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::{Border, Control, ControlImpl};
use ferroui_treedatagrid_core::models::{RowRef, RowsChangedArgs, RowsChangedHandler};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

type ChildIndexChangedHandlers = HandlerList<dyn Fn(&ChildIndexChangedEventArgs)>;

/// Presents the rows of a [`TreeDataGrid`](crate::TreeDataGrid), realizing only the rows in (and
/// around, see [`cache_length`](Self::cache_length)) the viewport.
#[repr(C)]
pub struct TreeDataGridRowsPresenter {
    base: Border,
    presenter: TreeDataGridPresenterBase,
    items: RefCell<Option<Rc<dyn ITreeDataGridRows>>>,
    columns: RefCell<Option<Rc<dyn IColumns>>>,
    is_attached_to_visual_tree: Cell<bool>,
    /// The columns whose `LayoutInvalidated` the presenter handles, and the token of the handler.
    layout_invalidated_columns: RefCell<Option<(Rc<dyn IColumns>, u64)>>,
    is_measuring_column_layout_batch: Cell<bool>,
    pending_column_layout_invalidation: Cell<bool>,
    child_index_changed: Rc<ChildIndexChangedHandlers>,
    child_index_provider: OnceCell<Rc<RowsPresenterChildIndexProvider>>,
}

ferro_class!(TreeDataGridRowsPresenter: Border);
ferroui_base::ferro_class_info!(TreeDataGridRowsPresenter { new: TreeDataGridRowsPresenter::new });
ferroui_base::ferro_impl_classes!(TreeDataGridRowsPresenter: InteractiveImpl, InputElementImpl, ControlImpl);

ferroui_base::ferro_properties! { impl TreeDataGridRowsPresenter {
    ferro_property!(
        /// Defines the `CacheLength` property.
        pub fn cache_length_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<TreeDataGridRowsPresenter, _>(
                "CacheLength",
                StyledPropertyOptions::new(0.0).validate(|v| *v >= 0.0 && *v <= 2.0),
            )
        }
    );

    ferro_property!(
        /// Defines the `Columns` property.
        pub fn columns_property() -> DirectProperty<TreeDataGridRowsPresenter, Option<Rc<dyn IColumns>>> {
            FerroProperty::register_direct::<TreeDataGridRowsPresenter, _>(
                "Columns",
                |o| o.columns(),
                Some(|o, v| o.set_columns(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `ElementFactory` property.
        pub fn element_factory_property() -> DirectProperty<TreeDataGridRowsPresenter, Option<Rc<TreeDataGridElementFactory>>> {
            FerroProperty::register_direct::<TreeDataGridRowsPresenter, _>(
                "ElementFactory",
                |o| o.element_factory(),
                Some(|o, v| o.set_element_factory(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Items` property.
        pub fn items_property() -> DirectProperty<TreeDataGridRowsPresenter, Option<Rc<dyn ITreeDataGridRows>>> {
            FerroProperty::register_direct::<TreeDataGridRowsPresenter, _>(
                "Items",
                |o| o.items(),
                Some(|o, v| o.set_items(v)),
                None,
            )
        }
    );
} }

impl FerroObjectImpl for TreeDataGridRowsPresenter {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::columns_property().as_property() {
            let old_value = change.get_old_value::<Option<Rc<dyn IColumns>>>().flatten();
            let new_value = change.get_new_value::<Option<Rc<dyn IColumns>>>();

            if let Some(old_value) = &old_value {
                this.unsubscribe_from_column_layout_invalidated(old_value);
            }
            if let Some(new_value) = &new_value {
                this.subscribe_to_column_layout_invalidated(new_value);
            }

            // When for an existing presenter the columns are recreated they won't get the viewport
            // set, so we need to track that and pass the viewport to the newly created object.
            if let (Some(_), Some(new_value)) = (&old_value, &new_value) {
                new_value.viewport_changed(this.viewport());
            }
        } else if change.property() == Self::cache_length_property().as_property() {
            this.invalidate_measure();
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl StyledElementImpl for TreeDataGridRowsPresenter {
    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_logical_tree(this, e);
        this.presenter.on_detached_from_logical_tree(this);
    }

    fn child_index_provider(this: &Self) -> Option<Rc<dyn IChildIndexProvider>> {
        let provider = this
            .child_index_provider
            .get_or_init(|| Rc::new(RowsPresenterChildIndexProvider { owner: this.to_ref().downgrade() }))
            .clone();
        Some(provider)
    }
}

impl VisualImpl for TreeDataGridRowsPresenter {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.presenter.on_attached_to_visual_tree(this);
        this.is_attached_to_visual_tree.set(true);

        if let Some(columns) = this.columns() {
            this.subscribe_to_column_layout_invalidated(&columns);
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        let columns = this.layout_invalidated_columns.borrow().as_ref().map(|(columns, _)| columns.clone());
        if let Some(columns) = columns {
            this.unsubscribe_from_column_layout_invalidated(&columns);
        }

        this.is_attached_to_visual_tree.set(false);
        Self::parent_on_detached_from_visual_tree(this, e);
        this.presenter.on_detached_from_visual_tree(this);
    }
}

impl LayoutableImpl for TreeDataGridRowsPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let columns = this.columns();
        let batch = columns.as_deref().and_then(|columns| columns.as_layout_batch());
        let needs_final_measure = Cell::new(false);
        let mut result;

        this.is_measuring_column_layout_batch.set(true);
        if let Some(batch) = batch {
            batch.begin_actual_width_batch();
        }

        {
            let _end_batch = EndColumnLayoutBatch { presenter: this, batch, needs_final_measure: &needs_final_measure };
            result = this.presenter.measure_override(this, available_size);
        }

        let needs_final_measure = needs_final_measure.get();

        // If we have no rows, then get the width from the columns.
        if let Some(columns) = &columns {
            if this.items().is_none_or(|items| items.count() == 0) {
                result = Size::new(columns.get_estimated_width(available_size.width), result.height);
            }
        }

        if needs_final_measure || this.pending_column_layout_invalidation.get() {
            this.pending_column_layout_invalidation.set(false);

            this.invalidate_cells_presenters();

            // Apply the widths gathered from all realized rows immediately. This avoids scheduling
            // a complete parent layout pass just to give the same rows their final column
            // constraints. Do not defer commits during this pass: a newly realized wider cell must
            // still request the normal fallback below.
            result = this.presenter.measure_override(this, available_size);

            if let Some(columns) = &columns {
                if this.items().is_none_or(|items| items.count() == 0) {
                    result = Size::new(columns.get_estimated_width(available_size.width), result.height);
                }
            }

            if this.pending_column_layout_invalidation.get() {
                this.pending_column_layout_invalidation.set(false);
                this.invalidate_measure();
                this.invalidate_cells_presenters();
            }
        }

        result
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(columns) = this.columns() {
            columns.commit_actual_widths();
        }
        this.presenter.arrange_override(this, final_size)
    }
}

impl TreeDataGridPresenterHooks for TreeDataGridRowsPresenter {
    type Item = RowRef;

    fn presenter_base(&self) -> &TreeDataGridPresenterBase {
        &self.presenter
    }

    fn orientation(&self) -> Orientation {
        Orientation::Vertical
    }

    fn item_count(&self) -> Option<usize> {
        self.items.borrow().as_ref().map(|items| items.count())
    }

    fn item_at(&self, index: usize) -> RowRef {
        let items = self.items().expect("the rows of the presenter");
        items.get(index)
    }

    fn subscribe_to_items(&self, handler: Rc<dyn Fn(&PresenterItemsChange)>) -> Option<u64> {
        let items = self.items()?;
        let handler: Rc<RowsChangedHandler> =
            Rc::new(move |e: &RowsChangedArgs<'_>| handler(&PresenterItemsChange::from_args(e)));
        Some(items.add_collection_changed(handler))
    }

    fn unsubscribe_from_items(&self, token: u64) {
        if let Some(items) = self.items() {
            items.remove_collection_changed(token);
        }
    }

    fn get_element_at(&self, position: f64) -> (i32, f64) {
        self.items().map_or((-1, -1.0), |items| items.get_row_at(position))
    }

    fn realize_element(&self, element: &Ref<Control>, _row_model: &RowRef, index: i32) {
        let row = element.cast::<TreeDataGridRow>().expect("an element of the rows presenter is a row");
        let selection = self.get_selection();
        row.realize(
            self.element_factory().as_ref(),
            selection.as_deref(),
            self.columns().as_ref(),
            self.items().as_ref(),
            index,
        );
        self.raise_child_index_changed(element, index);
    }

    fn update_element_index(&self, element: &Ref<Control>, _old_index: i32, new_index: i32) {
        let row = element.cast::<TreeDataGridRow>().expect("an element of the rows presenter is a row");
        row.update_index(new_index);
        self.raise_child_index_changed(element, new_index);
    }

    fn unrealize_element(&self, element: &Ref<Control>) {
        let row = element.cast::<TreeDataGridRow>().expect("an element of the rows presenter is a row");
        row.unrealize();
        self.raise_child_index_changed(element, row.row_index());
    }

    fn unrealize_element_on_item_removed(&self, element: &Ref<Control>) {
        let row = element.cast::<TreeDataGridRow>().expect("an element of the rows presenter is a row");
        row.unrealize_on_item_removed();
        self.raise_child_index_changed(element, row.row_index());
    }

    fn finalize_recycled_element(&self, element: &Ref<Control>) {
        if let Some(row) = element.cast::<TreeDataGridRow>() {
            row.finalize_unrealize();
        }
    }

    fn get_measure_viewport(&self, viewport: Rect) -> Rect {
        let cache_length = self.cache_length();

        if cache_length <= 0.0 || viewport.height <= 0.0 {
            return viewport;
        }

        let buffer = viewport.height * cache_length;
        let extent = if self.bounds().height > 0.0 { self.bounds().height } else { f64::INFINITY };
        let mut start = (viewport.top() - buffer).max(0.0);
        let mut end = extent.min(viewport.bottom() + buffer);
        let missing_before = buffer - (viewport.top() - start);
        let missing_after = buffer - (end - viewport.bottom());

        if missing_before > 0.0 {
            end = extent.min(end + missing_before);
        } else if missing_after > 0.0 {
            start = (start - missing_after).max(0.0);
        }

        Rect::new(viewport.x, start, viewport.width, end - start)
    }

    fn needs_measure_for_viewport_change(&self, measure_viewport: Rect, viewport: Rect) -> bool {
        if self.cache_length() <= 0.0 {
            return !MathUtilities::are_close(measure_viewport.width, viewport.width)
                || !self.presenter.is_viewport_covered_by_realized_elements(Orientation::Vertical, viewport);
        }

        viewport.top() < measure_viewport.top() || viewport.bottom() > measure_viewport.bottom()
    }

    fn on_effective_viewport_changed(&self, e: &EffectiveViewportChangedEventArgs) {
        self.presenter.default_on_effective_viewport_changed(self, e);
        if let Some(columns) = self.columns() {
            columns.viewport_changed(self.viewport());
        }
    }

    fn trace_enabled(&self) -> bool {
        true
    }
}

impl TreeDataGridRowsPresenter {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Border::construct(),
            presenter: TreeDataGridPresenterBase::new(),
            items: RefCell::new(None),
            columns: RefCell::new(None),
            is_attached_to_visual_tree: Cell::new(false),
            layout_invalidated_columns: RefCell::new(None),
            is_measuring_column_layout_batch: Cell::new(false),
            pending_column_layout_invalidation: Cell::new(false),
            child_index_changed: Rc::new(HandlerList::new()),
            child_index_provider: OnceCell::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The state of the presenter base.
    pub fn presenter_base(&self) -> &TreeDataGridPresenterBase {
        &self.presenter
    }

    /// The columns of the rows.
    pub fn columns(&self) -> Option<Rc<dyn IColumns>> {
        self.columns.borrow().clone()
    }

    pub fn set_columns(&self, value: Option<Rc<dyn IColumns>>) {
        self.set_and_raise(Self::columns_property(), &self.columns, value);
    }

    /// Gets or sets the additional row realization space before and after the viewport, expressed
    /// as a multiple of the viewport height.
    pub fn cache_length(&self) -> f64 {
        self.get_value(Self::cache_length_property())
    }

    pub fn set_cache_length(&self, value: f64) {
        self.set_value(Self::cache_length_property(), value)
    }

    /// The factory of the rows and cells.
    pub fn element_factory(&self) -> Option<Rc<TreeDataGridElementFactory>> {
        self.presenter.element_factory()
    }

    pub fn set_element_factory(&self, value: Option<Rc<TreeDataGridElementFactory>>) {
        self.set_and_raise(Self::element_factory_property(), self.presenter.element_factory_field(), value);
    }

    /// The rows the presenter shows.
    pub fn items(&self) -> Option<Rc<dyn ITreeDataGridRows>> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: Option<Rc<dyn ITreeDataGridRows>>) {
        let same = match (&*self.items.borrow(), &value) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };

        if !same {
            self.presenter.unsubscribe_from_item_changes(self);

            let old_value = self.items.replace(value.clone());

            self.presenter.subscribe_to_item_changes(self);

            self.raise_direct_property_changed(Self::items_property(), &old_value, &value);
            self.presenter.on_items_collection_changed(self, &PresenterItemsChange::reset());
        }
    }

    /// The realized rows, with an empty slot for each row of the realized range that has no
    /// element yet (`RealizedElements`).
    pub fn realized_elements(&self) -> Vec<Option<Ref<Control>>> {
        self.presenter.realized_elements()
    }

    /// The effective viewport of the presenter (`Viewport`).
    pub fn viewport(&self) -> Rect {
        self.presenter.viewport()
    }

    /// Whether the presenter is in its measure or arrange pass.
    pub fn is_layout_in_progress(&self) -> bool {
        self.presenter.is_layout_in_progress()
    }

    /// Brings the row at `index` into view (`BringIntoView`).
    pub fn bring_into_view(&self, index: i32, rect: Option<Rect>) -> Option<Ref<Control>> {
        self.presenter.bring_into_view(self, index, rect)
    }

    /// The realized rows (`GetRealizedElements`).
    pub fn get_realized_elements(&self) -> Vec<Ref<Control>> {
        self.presenter.get_realized_elements()
    }

    /// The realized row at `index`, if any (`TryGetElement`).
    pub fn try_get_element(&self, index: i32) -> Option<Ref<Control>> {
        self.presenter.try_get_element(index)
    }

    /// Recycles all the realized rows (`RecycleAllElements`).
    pub fn recycle_all_elements(&self) {
        self.presenter.recycle_all_elements(self, false, false);
    }

    /// Occurs when the index of a realized row changes (`ChildIndexChanged`). Disposing the
    /// returned handle unsubscribes.
    pub fn child_index_changed(&self, handler: impl Fn(&ChildIndexChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.child_index_changed.add(Rc::new(handler));
        let handlers = Rc::downgrade(&self.child_index_changed);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }

    /// Updates the selection state of the realized rows (`UpdateSelection`).
    pub fn update_selection(&self, selection: Option<&dyn ITreeDataGridSelectionInteraction>) {
        for element in self.realized_elements().into_iter().flatten() {
            if let Some(row) = element.cast::<TreeDataGridRow>().filter(|row| row.row_index() >= 0) {
                row.update_selection(selection);
            }
        }
    }

    /// The index of a row (`IChildIndexProvider.GetChildIndex`).
    pub fn get_child_index(&self, child: &StyledElement) -> i32 {
        match child.downcast_ref::<TreeDataGridRow>() {
            Some(row) => row.row_index(),
            None => -1,
        }
    }

    /// The number of rows (`IChildIndexProvider.TryGetTotalCount`).
    pub fn try_get_total_count(&self) -> Option<i32> {
        self.items().map(|items| items.count() as i32)
    }

    fn raise_child_index_changed(&self, element: &Ref<Control>, index: i32) {
        if !self.child_index_changed.is_empty() {
            let e = ChildIndexChangedEventArgs::new(element.clone().upcast(), index);
            for (_, handler) in self.child_index_changed.snapshot().iter() {
                handler(&e);
            }
        }
    }

    fn invalidate_cells_presenters(&self) {
        for element in self.realized_elements().into_iter().flatten() {
            if let Some(cells_presenter) = element.cast::<TreeDataGridRow>().and_then(|row| row.cells_presenter()) {
                cells_presenter.invalidate_measure();
            }
        }
    }

    fn subscribe_to_column_layout_invalidated(&self, columns: &Rc<dyn IColumns>) {
        let current = self.layout_invalidated_columns.borrow().as_ref().map(|(columns, _)| columns.clone());

        if !self.is_attached_to_visual_tree.get() || current.as_ref().is_some_and(|current| Rc::ptr_eq(current, columns)) {
            return;
        }

        if let Some(old_columns) = current {
            self.unsubscribe_from_column_layout_invalidated(&old_columns);
        }

        let weak: WeakRef<TreeDataGridRowsPresenter> = self.to_ref().downgrade();
        let token = columns.add_layout_invalidated(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_column_layout_invalidated();
            }
        }));
        *self.layout_invalidated_columns.borrow_mut() = Some((columns.clone(), token));
    }

    fn unsubscribe_from_column_layout_invalidated(&self, columns: &Rc<dyn IColumns>) {
        let subscription = {
            let mut current = self.layout_invalidated_columns.borrow_mut();
            match &*current {
                Some((current_columns, _)) if Rc::ptr_eq(current_columns, columns) => current.take(),
                _ => None,
            }
        };

        if let Some((columns, token)) = subscription {
            columns.remove_layout_invalidated(token);
        }
    }

    fn on_column_layout_invalidated(&self) {
        if self.presenter.is_in_layout() || self.is_measuring_column_layout_batch.get() {
            self.pending_column_layout_invalidation.set(true);
            return;
        }

        self.invalidate_measure();
        self.invalidate_cells_presenters();
    }

    fn get_selection(&self) -> Option<Rc<dyn ITreeDataGridSelectionInteraction>> {
        self.find_ancestor_of_type::<crate::TreeDataGrid>(false)
            .and_then(|tree_data_grid| tree_data_grid.selection_interaction())
    }
}

/// Ends the column width batch of a measure and clears the batch flag when the measure is left
/// (the reference's nested `finally` blocks).
struct EndColumnLayoutBatch<'a> {
    presenter: &'a TreeDataGridRowsPresenter,
    batch: Option<&'a dyn IColumnLayoutBatch>,
    needs_final_measure: &'a Cell<bool>,
}

impl Drop for EndColumnLayoutBatch<'_> {
    fn drop(&mut self) {
        if let Some(batch) = self.batch {
            self.needs_final_measure.set(batch.end_actual_width_batch());
        }
        self.presenter.is_measuring_column_layout_batch.set(false);
    }
}

/// The child index provider of a rows presenter: a row's index is its row index.
struct RowsPresenterChildIndexProvider {
    owner: WeakRef<TreeDataGridRowsPresenter>,
}

impl IChildIndexProvider for RowsPresenterChildIndexProvider {
    fn get_child_index(&self, child: &StyledElement) -> i32 {
        self.owner.upgrade().map_or(-1, |owner| owner.get_child_index(child))
    }

    fn try_get_total_count(&self) -> Option<i32> {
        self.owner.upgrade()?.try_get_total_count()
    }

    fn child_index_changed(&self, handler: Rc<dyn Fn(&ChildIndexChangedEventArgs)>) -> Rc<dyn IDisposable> {
        let Some(owner) = self.owner.upgrade() else { return Disposable::empty() };
        let token = owner.child_index_changed.add(handler);
        let handlers = Rc::downgrade(&owner.child_index_changed);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }
}
