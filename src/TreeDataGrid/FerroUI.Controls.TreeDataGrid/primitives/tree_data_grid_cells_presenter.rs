//! `Primitives/TreeDataGridCellsPresenter.cs`: the presenter of the cells of a row, a horizontal
//! virtualizing presenter of [`TreeDataGridCell`]s over the columns, which realizes the cell models
//! through the rows of the presentation.

use super::{
    PresenterItemsChange, TreeDataGridCell, TreeDataGridColumnarPresenterBase, TreeDataGridElementFactory,
    TreeDataGridPresenterBase, TreeDataGridPresenterHooks, TreeDataGridRowsPresenter, TreeDataGridTemplateCell,
    IFinalMeasureSelector,
};
use crate::models::{CellRef, ColumnRef, ColumnsChangedArgs, ColumnsChangedHandler, IColumns, ITreeDataGridRows};
use crate::presentation::as_reusable_cell_rows;
use crate::selection::ITreeDataGridSelectionInteraction;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::logical_tree::{ChildIndexChangedEventArgs, IChildIndexProvider, LogicalTreeAttachmentEventArgs};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElement, StyledElementImpl, StyledElementImplExt,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::{Border, Control, ControlImpl};
use std::any::Any;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

type ChildIndexChangedHandlers = HandlerList<dyn Fn(&ChildIndexChangedEventArgs)>;

/// Presents the cells of a [`TreeDataGridRow`](super::TreeDataGridRow), realizing only the cells
/// of the columns in the viewport.
#[repr(C)]
pub struct TreeDataGridCellsPresenter {
    base: Border,
    presenter: TreeDataGridPresenterBase,
    columnar: TreeDataGridColumnarPresenterBase,
    items: RefCell<Option<Rc<dyn IColumns>>>,
    rows: RefCell<Option<Rc<dyn ITreeDataGridRows>>>,
    rebind_realized_cells: Cell<bool>,
    row_index: Cell<i32>,
    child_index_changed: Rc<ChildIndexChangedHandlers>,
    child_index_provider: OnceCell<Rc<CellsPresenterChildIndexProvider>>,
}

ferro_class! {
    TreeDataGridCellsPresenter: Border, virtuals TreeDataGridCellsPresenterImpl: ControlImpl {
        /// Measures the cell of the column at `index` (`MeasureElement`): the measure of the
        /// columnar presenters, which reports the size to the columns.
        fn measure_element(this, index: i32, element: &Ref<Control>, available_size: Size) -> Size;
    }
}
ferroui_base::ferro_class_info!(TreeDataGridCellsPresenter { new: TreeDataGridCellsPresenter::new });
ferroui_base::ferro_impl_classes!(TreeDataGridCellsPresenter: InteractiveImpl, InputElementImpl, ControlImpl);

ferroui_base::ferro_properties! { impl TreeDataGridCellsPresenter {
    ferro_property!(
        /// Defines the `ElementFactory` property.
        pub fn element_factory_property() -> DirectProperty<TreeDataGridCellsPresenter, Option<Rc<TreeDataGridElementFactory>>> {
            FerroProperty::register_direct::<TreeDataGridCellsPresenter, _>(
                "ElementFactory",
                |o| o.element_factory(),
                Some(|o, v| o.set_element_factory(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Items` property: the columns.
        pub fn items_property() -> DirectProperty<TreeDataGridCellsPresenter, Option<Rc<dyn IColumns>>> {
            FerroProperty::register_direct::<TreeDataGridCellsPresenter, _>(
                "Items",
                |o| o.items(),
                Some(|o, v| o.set_items(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Rows` property.
        pub fn rows_property() -> DirectProperty<TreeDataGridCellsPresenter, Option<Rc<dyn ITreeDataGridRows>>> {
            FerroProperty::register_direct::<TreeDataGridCellsPresenter, _>(
                "Rows",
                |o| o.rows(),
                Some(|o, v| o.set_rows(v)),
                None,
            )
        }
    );
} }

impl FerroObjectImpl for TreeDataGridCellsPresenter {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Border::background_property().as_property() {
            this.invalidate_visual();
        }
    }
}

impl StyledElementImpl for TreeDataGridCellsPresenter {
    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_logical_tree(this, e);
        this.presenter.on_detached_from_logical_tree(this);
    }

    fn child_index_provider(this: &Self) -> Option<Rc<dyn IChildIndexProvider>> {
        let provider = this
            .child_index_provider
            .get_or_init(|| Rc::new(CellsPresenterChildIndexProvider { owner: this.to_ref().downgrade() }))
            .clone();
        Some(provider)
    }
}

impl VisualImpl for TreeDataGridCellsPresenter {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.presenter.on_attached_to_visual_tree(this);
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        // If the owning row was not reused during the layout pass, release the deferred cell
        // models while their controls are still parented to this presenter. The entire row is
        // already leaving the visual tree, so preserving the internal subtree avoids removing and
        // re-adding the same controls (and reapplying their styles) on reattachment.
        this.recycle_deferred_cells();
        Self::parent_on_detached_from_visual_tree(this, e);
        this.presenter.on_detached_from_visual_tree(this);
    }
}

impl LayoutableImpl for TreeDataGridCellsPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if this.row_index.get() == -1 {
            Size::default()
        } else {
            this.presenter.measure_override(this, available_size)
        }
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.presenter.arrange_override(this, final_size)
    }
}

impl TreeDataGridCellsPresenterImpl for TreeDataGridCellsPresenter {
    fn measure_element(this: &Self, index: i32, element: &Ref<Control>, available_size: Size) -> Size {
        let columns = this.items().expect("the columns of the cells presenter");
        TreeDataGridColumnarPresenterBase::measure_column_element(
            &*columns,
            index,
            this.row_index.get(),
            element,
            available_size,
        )
    }
}

impl IFinalMeasureSelector for TreeDataGridCellsPresenter {
    fn needs_final_measure(&self, element: &Ref<Control>, index: i32) -> bool {
        let columns = self.items().expect("the columns of the cells presenter");
        TreeDataGridColumnarPresenterBase::needs_final_measure(&*columns, element, index)
    }
}

impl TreeDataGridPresenterHooks for TreeDataGridCellsPresenter {
    type Item = ColumnRef;

    fn presenter_base(&self) -> &TreeDataGridPresenterBase {
        &self.presenter
    }

    fn orientation(&self) -> Orientation {
        Orientation::Horizontal
    }

    fn item_count(&self) -> Option<usize> {
        self.items.borrow().as_ref().map(|items| items.count())
    }

    fn item_at(&self, index: usize) -> ColumnRef {
        self.items().expect("the columns of the cells presenter").get(index)
    }

    fn subscribe_to_items(&self, handler: Rc<dyn Fn(&PresenterItemsChange)>) -> Option<u64> {
        let items = self.items()?;
        let handler: Rc<ColumnsChangedHandler> =
            Rc::new(move |e: &ColumnsChangedArgs<'_>| handler(&PresenterItemsChange::from_args(e)));
        Some(items.add_collection_changed(handler))
    }

    fn unsubscribe_from_items(&self, token: u64) {
        if let Some(items) = self.items() {
            items.remove_collection_changed(token);
        }
    }

    fn realize_element(&self, element: &Ref<Control>, column: &ColumnRef, index: i32) {
        let cell = element.cast::<TreeDataGridCell>().expect("an element of the cells presenter is a cell");
        cell.set_recycling_column(Some(column.clone()));

        if cell.column_index() == index && cell.row_index() == self.row_index.get() {
            self.raise_child_index_changed(element, index);
        } else if cell.column_index() == -1 && cell.row_index() == -1 {
            let rows = self.rows().expect("the rows of the cells presenter");
            let factory = self.element_factory().expect("the element factory of the cells presenter");
            let model = rows.realize_cell(column, index, self.row_index.get());
            let selection = self.get_selection();
            cell.realize(&factory, selection.as_deref(), &model, index, self.row_index.get());
            self.raise_child_index_changed(element, index);
        } else {
            panic!("Cell already realized");
        }
    }

    fn update_element_index(&self, element: &Ref<Control>, _old_index: i32, new_index: i32) {
        self.raise_child_index_changed(element, new_index);
    }

    fn unrealize_element(&self, element: &Ref<Control>) {
        let cell = element.cast::<TreeDataGridCell>().expect("an element of the cells presenter is a cell");
        let model = cell.model().expect("a realized cell has a model");
        let column_index = cell.column_index();
        let row_index = cell.row_index();
        let column = cell.recycling_column();
        cell.unrealize();

        let rows = self.rows();
        let recycled = match (&rows, &column) {
            (Some(rows), Some(column)) if self.row_index.get() >= 0 => rows.try_recycle_cell(column, &model),
            _ => false,
        };

        if !recycled {
            if let Some(rows) = &rows {
                rows.unrealize_cell(&model, column_index, row_index);
            }
        }

        self.raise_child_index_changed(element, cell.row_index());
    }

    fn unrealize_element_on_item_removed(&self, element: &Ref<Control>) {
        // Collection changes have already replaced/removed the column at this index. Dispose
        // directly rather than associating its binding with a different column.
        let cell = element.cast::<TreeDataGridCell>().expect("an element of the cells presenter is a cell");
        if let (Some(rows), Some(model)) = (self.rows(), cell.model()) {
            rows.unrealize_cell(&model, cell.column_index(), cell.row_index());
        }
        cell.unrealize();
        self.raise_child_index_changed(element, cell.row_index());
    }

    fn finalize_recycled_element(&self, element: &Ref<Control>) {
        if let Some(cell) = element.cast::<TreeDataGridTemplateCell>() {
            cell.finalize_unrealize();
        }
    }

    fn measure_element(&self, index: i32, element: &Ref<Control>, available_size: Size) -> Size {
        TreeDataGridCellsPresenter::measure_element(self, index, element, available_size)
    }

    fn get_element_from_factory(&self, column: &ColumnRef, index: i32) -> Ref<Control> {
        let rows = self.rows().expect("the rows of the cells presenter");
        let factory = self.element_factory().expect("the element factory of the cells presenter");
        let model = rows.realize_cell(column, index, self.row_index.get());
        let element = self.presenter.get_element_from_factory_for(&model as &dyn Any, self);
        let cell = element.cast::<TreeDataGridCell>().expect("the element of a cell model is a cell");
        cell.set_recycling_column(Some(column.clone()));
        let selection = self.get_selection();
        cell.realize(&factory, selection.as_deref(), &model, index, self.row_index.get());
        element
    }

    fn get_initial_constraint(&self, _element: &Ref<Control>, index: i32, available_size: Size) -> Size {
        let columns = self.items().expect("the columns of the cells presenter");
        TreeDataGridColumnarPresenterBase::get_initial_constraint(&*columns, index, available_size)
    }

    fn needs_final_measure_pass(&self, first_index: i32, elements: &[Option<Ref<Control>>]) -> bool {
        let columns = self.items().expect("the columns of the cells presenter");
        TreeDataGridColumnarPresenterBase::needs_final_measure_pass(&*columns, first_index, elements)
    }

    fn get_final_constraint(&self, _element: &Ref<Control>, index: i32, _available_size: Size) -> Size {
        let columns = self.items().expect("the columns of the cells presenter");
        TreeDataGridColumnarPresenterBase::get_final_constraint(&*columns, index)
    }

    fn get_element_at(&self, position: f64) -> (i32, f64) {
        let columns = self.items().expect("the columns of the cells presenter");
        TreeDataGridColumnarPresenterBase::get_element_at(&*columns, position)
    }

    fn calculate_size_u(&self, available_size: Size) -> f64 {
        TreeDataGridColumnarPresenterBase::calculate_size_u(self.items().as_deref(), available_size)
    }

    fn get_or_estimate_anchor_element_for_viewport(
        &self,
        viewport_start: f64,
        viewport_end: f64,
        item_count: i32,
    ) -> (i32, f64) {
        self.columnar.get_or_estimate_anchor_element_for_viewport(
            &self.presenter,
            self.items().as_deref(),
            viewport_start,
            viewport_end,
            item_count,
        )
    }

    fn estimate_element_size_u(&self) -> f64 {
        self.columnar.estimate_element_size_u(self.items().as_deref())
    }

    fn get_parent_presenter_viewport(&self) -> Option<Rect> {
        self.get_visual_ancestors()
            .find_map(|visual| visual.cast::<TreeDataGridRowsPresenter>())
            .map(|rows_presenter| rows_presenter.viewport())
    }

    fn needs_measure_for_viewport_change(&self, measure_viewport: Rect, viewport: Rect) -> bool {
        TreeDataGridColumnarPresenterBase::needs_measure_for_viewport_change(
            &self.presenter,
            Orientation::Horizontal,
            measure_viewport,
            viewport,
        )
    }

    fn as_final_measure_selector(&self) -> Option<&dyn IFinalMeasureSelector> {
        Some(self)
    }
}

impl TreeDataGridCellsPresenter {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Border::construct(),
            presenter: TreeDataGridPresenterBase::new(),
            columnar: TreeDataGridColumnarPresenterBase::new(),
            items: RefCell::new(None),
            rows: RefCell::new(None),
            rebind_realized_cells: Cell::new(false),
            row_index: Cell::new(-1),
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

    /// The factory of the cells.
    pub fn element_factory(&self) -> Option<Rc<TreeDataGridElementFactory>> {
        self.presenter.element_factory()
    }

    pub fn set_element_factory(&self, value: Option<Rc<TreeDataGridElementFactory>>) {
        self.set_and_raise(Self::element_factory_property(), self.presenter.element_factory_field(), value);
    }

    /// The columns whose cells the presenter shows.
    pub fn items(&self) -> Option<Rc<dyn IColumns>> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: Option<Rc<dyn IColumns>>) {
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

    /// The rows that realize the cell models.
    pub fn rows(&self) -> Option<Rc<dyn ITreeDataGridRows>> {
        self.rows.borrow().clone()
    }

    pub fn set_rows(&self, value: Option<Rc<dyn ITreeDataGridRows>>) {
        let same = match (&*self.rows.borrow(), &value) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };

        if !same {
            self.recycle_deferred_cells();
            self.set_and_raise(Self::rows_property(), &self.rows, value);
        }
    }

    /// The index of the row the presenter shows the cells of, or -1 when it is not realized.
    pub fn row_index(&self) -> i32 {
        self.row_index.get()
    }

    /// The realized cells, with an empty slot for each column of the realized range that has no
    /// cell yet (`RealizedElements`).
    pub fn realized_elements(&self) -> Vec<Option<Ref<Control>>> {
        self.presenter.realized_elements()
    }

    /// Whether the presenter is in its measure or arrange pass.
    pub fn is_layout_in_progress(&self) -> bool {
        self.presenter.is_layout_in_progress()
    }

    /// Brings the cell of the column at `index` into view (`BringIntoView`).
    pub fn bring_into_view(&self, index: i32, rect: Option<Rect>) -> Option<Ref<Control>> {
        self.presenter.bring_into_view(self, index, rect)
    }

    /// The realized cells (`GetRealizedElements`).
    pub fn get_realized_elements(&self) -> Vec<Ref<Control>> {
        self.presenter.get_realized_elements()
    }

    /// The realized cell of the column at `index`, if any (`TryGetElement`).
    pub fn try_get_element(&self, index: i32) -> Option<Ref<Control>> {
        self.presenter.try_get_element(index)
    }

    /// Recycles all the realized cells (`RecycleAllElements`).
    pub fn recycle_all_elements(&self) {
        self.presenter.recycle_all_elements(self, false, false);
    }

    /// Occurs when the index of a realized cell changes (`ChildIndexChanged`). Disposing the
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

    /// Realizes the presenter for the row at `index`.
    pub fn realize(&self, index: i32) {
        if self.row_index.get() != -1 {
            panic!("Row is already realized.");
        }

        self.row_index.set(index);

        if self.rebind_realized_cells.get() {
            self.rebind_realized_cells.set(false);
            self.rebind_realized_cells_core();
        }

        self.invalidate_measure();
    }

    /// Unrealizes the presenter.
    pub fn unrealize(&self) {
        if self.row_index.get() == -1 {
            panic!("Row is not realized.");
        }
        self.row_index.set(-1);

        // A row leaving one edge of the viewport is commonly reused at the other edge in the same
        // measure pass. Keep its cells realized until we know whether the row is rebound or
        // actually detached, avoiding a recycle/factory/re-realize round-trip for every column.
        self.rebind_realized_cells.set(true);
    }

    /// The index of the row changed (`UpdateRowIndex`).
    pub fn update_row_index(&self, index: i32) {
        if index < 0 || self.rows().is_none_or(|rows| index as usize >= rows.count()) {
            return;
        }

        if self.row_index.get() == -1 {
            return;
        }

        self.row_index.set(index);

        for element in self.realized_elements().into_iter().flatten() {
            if let Some(cell) =
                element.cast::<TreeDataGridCell>().filter(|cell| cell.row_index() >= 0 && cell.column_index() >= 0)
            {
                cell.update_row_index(index);
            }
        }
    }

    /// Updates the selection state of the realized cells (`UpdateSelection`).
    pub fn update_selection(&self, selection: Option<&dyn ITreeDataGridSelectionInteraction>) {
        for element in self.realized_elements().into_iter().flatten() {
            if let Some(cell) =
                element.cast::<TreeDataGridCell>().filter(|cell| cell.row_index() >= 0 && cell.column_index() >= 0)
            {
                cell.update_selection(selection);
            }
        }
    }

    /// Unrealizes the presenter because its row was removed (`UnrealizeOnRowRemoved`).
    pub fn unrealize_on_row_removed(&self) {
        if self.row_index.get() == -1 {
            panic!("Row is not realized.");
        }
        self.row_index.set(-1);
        self.rebind_realized_cells.set(false);
        self.presenter.recycle_all_elements_on_item_removed(self, true, true);
    }

    /// Recycles the cells kept for a rebind of the row when the row was not rebound
    /// (`FinalizeUnrealize`).
    pub fn finalize_unrealize(&self) {
        self.recycle_deferred_cells();
    }

    /// The index of a cell (`IChildIndexProvider.GetChildIndex`).
    pub fn get_child_index(&self, child: &StyledElement) -> i32 {
        match child.downcast_ref::<TreeDataGridCell>() {
            Some(cell) => cell.column_index(),
            None => -1,
        }
    }

    /// The number of columns (`IChildIndexProvider.TryGetTotalCount`).
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

    /// Rebinds the cells kept by [`unrealize`](Self::unrealize) to the row the presenter is
    /// realized for (`RebindRealizedCells`).
    fn rebind_realized_cells_core(&self) {
        let items = self.items();
        let rows = self.rows();
        let factory = self.element_factory();
        let elements = self.realized_elements();
        let first_index = self.presenter.first_index();

        let (Some(items), Some(rows), Some(factory)) = (items, rows, factory) else {
            self.presenter.recycle_all_elements(self, true, true);
            return;
        };

        if first_index < 0 || first_index as usize + elements.len() > items.count() {
            self.presenter.recycle_all_elements(self, true, true);
            return;
        }

        let row_index = self.row_index.get();

        for (i, element) in elements.iter().enumerate() {
            let Some(cell) = element.as_ref().and_then(|element| element.cast::<TreeDataGridCell>()) else {
                continue;
            };

            let column_index = first_index + i as i32;
            let old_column_index = cell.column_index();
            let old_row_index = cell.row_index();
            let mut model = cell.model().expect("a realized cell has a model");
            let column = items.get(column_index as usize);

            let rebind = RebindScope { cell: &cell, realized: Cell::new(false) };
            cell.begin_rebind();
            cell.unrealize();
            cell.set_recycling_column(Some(column.clone()));
            self.raise_child_index_changed(element.as_ref().expect("a cell"), cell.row_index());

            let selection = self.get_selection();
            if Self::try_reuse_cell(&*rows, &column, &model, row_index) {
                cell.realize(&factory, selection.as_deref(), &model, column_index, row_index);
            } else {
                rows.unrealize_cell(&model, old_column_index, old_row_index);
                model = rows.realize_cell(&column, column_index, row_index);
                cell.realize(&factory, selection.as_deref(), &model, column_index, row_index);
            }

            rebind.realized.set(true);
            self.raise_child_index_changed(element.as_ref().expect("a cell"), column_index);
            drop(rebind);
        }
    }

    /// Whether the rows retarget `cell` of `column` to the row at `row_index`
    /// (`_rows is IReusableCellRows reusableRows && reusableRows.TryReuseCell(...)`).
    fn try_reuse_cell(rows: &dyn ITreeDataGridRows, column: &ColumnRef, cell: &CellRef, row_index: i32) -> bool {
        as_reusable_cell_rows(rows).is_some_and(|rows| rows.try_reuse_cell(column, cell, row_index))
    }

    fn recycle_deferred_cells(&self) {
        if !self.rebind_realized_cells.get() {
            return;
        }

        self.rebind_realized_cells.set(false);
        self.presenter.recycle_all_elements(self, true, true);
    }

    fn get_selection(&self) -> Option<Rc<dyn ITreeDataGridSelectionInteraction>> {
        self.find_ancestor_of_type::<crate::TreeDataGrid>(false)
            .and_then(|tree_data_grid| tree_data_grid.selection_interaction())
    }
}

/// Ends the rebind of a cell, also when a lifecycle callback panics (the reference's `finally`).
struct RebindScope<'a> {
    cell: &'a Ref<TreeDataGridCell>,
    realized: Cell<bool>,
}

impl Drop for RebindScope<'_> {
    fn drop(&mut self) {
        self.cell.end_rebind(self.realized.get());
    }
}

/// The child index provider of a cells presenter: a cell's index is its column index.
struct CellsPresenterChildIndexProvider {
    owner: WeakRef<TreeDataGridCellsPresenter>,
}

impl IChildIndexProvider for CellsPresenterChildIndexProvider {
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
