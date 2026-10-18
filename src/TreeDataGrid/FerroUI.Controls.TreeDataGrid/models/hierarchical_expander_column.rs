//! `Models/TreeDataGrid/HierarchicalExpanderColumn.cs`: a column of a hierarchical source whose
//! cells show an expander that reveals the nested data, around the cells of an inner column.
//!
//! It is the expander column of the core's hierarchical source too (`IExpanderColumn<TModel>`):
//! the source builds the rows of the child models with it.

use super::double_utils::double_equals;
use super::{
    to_core_grid_length, AccessorBinding, CellRef, ColumnRef, ExpanderCell, ICellBinding, ICellColumn, IColumn,
    IUpdateColumnLayout, IViewColumn, ShowExpanderObservable, ViewColumnHandle,
};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::BoxedValue;
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{
    self as core_models, as_hierarchical_row, ColumnKind, Comparison, IColumnOf, IExpander, IExpanderColumn,
    IExpanderRow, IRowOf, ListSortDirection, NotifyingBase, RowRef,
};
use ferroui_treedatagrid_core::{Accessor, Models, PropertyPathObserver};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The child models of a model (`Func<TModel, IEnumerable<TModel>?>`).
pub type ChildSelector<M> = Rc<dyn Fn(&Rc<M>) -> Option<Models<M>>>;

/// `HierarchicalExpanderColumn<TModel>`.
pub struct HierarchicalExpanderColumn<M: TreeModel> {
    weak_self: Weak<Self>,
    notifying: NotifyingBase,
    inner: Rc<dyn IViewColumn<M>>,
    inner_token: Cell<Option<u64>>,
    child_selector: ChildSelector<M>,
    has_children_selector: Option<Accessor<M, bool>>,
    is_expanded_selector: Option<Accessor<M, bool>>,
    expanded_observer: Option<Rc<PropertyPathObserver<M>>>,
    actual_width: Cell<f64>,
}

impl<M: TreeModel> HierarchicalExpanderColumn<M> {
    /// A column that shows `inner` with an expander. `child_selector` gives the children of a
    /// model; `has_children_selector` whether a model has children without selecting them (only
    /// needed when selecting the children is expensive); `is_expanded_selector` a read/write
    /// expansion state of the model (its setter writes the state back).
    pub fn new(
        inner: Rc<dyn IViewColumn<M>>,
        child_selector: impl Fn(&Rc<M>) -> Option<Models<M>> + 'static,
        has_children_selector: Option<Accessor<M, bool>>,
        is_expanded_selector: Option<Accessor<M, bool>>,
    ) -> Rc<Self> {
        let actual_width = IColumn::actual_width(&*inner);
        let expanded_observer = is_expanded_selector.as_ref().and_then(PropertyPathObserver::create);
        let this = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            notifying: NotifyingBase::new(),
            inner: inner.clone(),
            inner_token: Cell::new(None),
            child_selector: Rc::new(child_selector),
            has_children_selector,
            is_expanded_selector,
            expanded_observer,
            actual_width: Cell::new(actual_width),
        });
        let weak = Rc::downgrade(&this);
        let token = inner.property_changed().add(Rc::new(move |name: &str| {
            if let Some(this) = weak.upgrade() {
                this.on_inner_property_changed(name);
            }
        }));
        this.inner_token.set(Some(token));
        this
    }

    /// The column that defines how the cells are shown.
    pub fn inner(&self) -> &Rc<dyn IViewColumn<M>> {
        &self.inner
    }

    /// The selector of the child models.
    pub fn child_selector(&self) -> &ChildSelector<M> {
        &self.child_selector
    }

    pub fn has_children_selector(&self) -> Option<&Accessor<M, bool>> {
        self.has_children_selector.as_ref()
    }

    pub fn is_expanded_selector(&self) -> Option<&Accessor<M, bool>> {
        self.is_expanded_selector.as_ref()
    }

    /// The width of the column after measurement.
    pub fn actual_width(&self) -> f64 {
        self.actual_width.get()
    }

    fn set_actual_width(&self, value: f64) {
        if !double_equals(self.actual_width.get(), value) {
            self.actual_width.set(value);
            self.notifying.raise_property_changed("ActualWidth");
        }
    }

    /// Whether `model` has children.
    pub fn has_children(&self, model: &Rc<M>) -> bool {
        match &self.has_children_selector {
            Some(selector) => selector.get(model),
            None => (self.child_selector)(model).is_some_and(|children| children.count() > 0),
        }
    }

    /// The child models of `model`.
    pub fn get_child_models(&self, model: &Rc<M>) -> Option<Models<M>> {
        (self.child_selector)(model)
    }

    /// The cell of the expander row `row`.
    pub fn create_expander_cell(&self, row: Rc<dyn IExpanderRow<M>>, row_ref: &RowRef) -> CellRef {
        let model = row.model_of();
        let show_expander =
            ShowExpanderObservable::new(self.child_selector.clone(), self.has_children_selector.clone(), model.clone());
        let is_expanded = self.is_expanded_selector.as_ref().map(|selector| {
            AccessorBinding::with_observer(selector.clone(), self.expanded_observer.clone(), Some(model.clone()))
                as Rc<dyn ICellBinding<bool>>
        });
        let inner = ICellColumn::create_cell(&*self.inner, row_ref);
        ExpanderCell::new(inner, row, show_expander, is_expanded)
    }

    fn on_inner_property_changed(&self, name: &str) {
        if name == "CanUserResize" || name == "Header" || name == "SortDirection" || name == "Width" {
            self.notifying.raise_property_changed(name);
        }
    }

    fn set_width_core(&self, width: GridLength) {
        IUpdateColumnLayout::set_width(&*self.inner, width);

        if width.is_absolute() {
            self.set_actual_width(width.value());
        }
    }
}

impl<M: TreeModel> Drop for HierarchicalExpanderColumn<M> {
    fn drop(&mut self) {
        if let Some(token) = self.inner_token.take() {
            self.inner.property_changed().remove(token);
        }
    }
}

impl<M: TreeModel> INotifyPropertyChanged for HierarchicalExpanderColumn<M> {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl<M: TreeModel> IColumn for HierarchicalExpanderColumn<M> {
    fn actual_width(&self) -> f64 {
        self.actual_width.get()
    }

    fn can_user_resize(&self) -> Option<bool> {
        IColumn::can_user_resize(&*self.inner)
    }

    fn header(&self) -> Option<BoxedValue> {
        IColumn::header(&*self.inner)
    }

    fn width(&self) -> GridLength {
        IColumn::width(&*self.inner)
    }

    fn sort_direction(&self) -> Option<ListSortDirection> {
        IColumn::sort_direction(&*self.inner)
    }

    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        IColumn::set_sort_direction(&*self.inner, value)
    }

    fn tag(&self) -> Option<BoxedValue> {
        IColumn::tag(&*self.inner)
    }

    fn set_tag(&self, value: Option<BoxedValue>) {
        IColumn::set_tag(&*self.inner, value)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_update_column_layout(&self) -> Option<&dyn IUpdateColumnLayout> {
        Some(self)
    }

    fn as_cell_column(&self) -> Option<&dyn ICellColumn> {
        Some(self)
    }

    fn requires_unconstrained_width_measurement(&self) -> bool {
        IColumn::requires_unconstrained_width_measurement(&*self.inner)
    }

    fn core_column(&self) -> Option<Rc<dyn core_models::IColumn>> {
        self.weak_self.upgrade().map(|column| column as Rc<dyn core_models::IColumn>)
    }
}

impl<M: TreeModel> IUpdateColumnLayout for HierarchicalExpanderColumn<M> {
    fn min_actual_width(&self) -> f64 {
        self.inner.min_actual_width()
    }

    fn max_actual_width(&self) -> f64 {
        self.inner.max_actual_width()
    }

    fn star_width_was_constrained(&self) -> bool {
        self.inner.star_width_was_constrained()
    }

    fn cell_measured(&self, width: f64, row_index: i32) -> f64 {
        self.inner.cell_measured(width, row_index)
    }

    fn calculate_star_width(&self, available_width: f64, total_stars: f64) {
        self.inner.calculate_star_width(available_width, total_stars);
        self.set_actual_width(IColumn::actual_width(&*self.inner));
    }

    fn commit_actual_width(&self) -> bool {
        let result = self.inner.commit_actual_width();
        self.set_actual_width(IColumn::actual_width(&*self.inner));
        result
    }

    fn set_width(&self, width: GridLength) {
        self.set_width_core(width);
    }
}

impl<M: TreeModel> ICellColumn for HierarchicalExpanderColumn<M> {
    /// # Panics
    /// When the row is not an expander row of models of `M` (`NotSupportedException`).
    fn create_cell(&self, row: &RowRef) -> CellRef {
        match as_hierarchical_row::<M>(row) {
            Some(expander_row) => self.create_expander_cell(expander_row, row),
            None => panic!("Specified method is not supported."),
        }
    }
}

impl<M: TreeModel> core_models::IColumn for HierarchicalExpanderColumn<M> {
    fn id(&self) -> String {
        core_models::IColumn::id(&*self.inner)
    }

    fn header(&self) -> Option<BoxedValue> {
        IColumn::header(&*self.inner)
    }

    fn width(&self) -> ferroui_treedatagrid_core::GridLength {
        to_core_grid_length(IColumn::width(&*self.inner))
    }

    fn set_width(&self, value: ferroui_treedatagrid_core::GridLength) {
        self.set_width_core(super::from_core_grid_length(value));
    }

    fn is_visible(&self) -> bool {
        core_models::IColumn::is_visible(&*self.inner)
    }

    fn set_is_visible(&self, value: bool) {
        core_models::IColumn::set_is_visible(&*self.inner, value)
    }

    fn presentation_key(&self) -> Option<String> {
        core_models::IColumn::presentation_key(&*self.inner)
    }

    fn set_presentation_key(&self, value: Option<String>) {
        core_models::IColumn::set_presentation_key(&*self.inner, value)
    }

    fn sort_direction(&self) -> Option<ListSortDirection> {
        IColumn::sort_direction(&*self.inner)
    }

    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        IColumn::set_sort_direction(&*self.inner, value)
    }

    fn tag(&self) -> Option<BoxedValue> {
        IColumn::tag(&*self.inner)
    }

    fn set_tag(&self, value: Option<BoxedValue>) {
        IColumn::set_tag(&*self.inner, value)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn model_type(&self) -> TypeId {
        TypeId::of::<M>()
    }

    fn view(&self) -> Option<Rc<dyn Any>> {
        let column = self.weak_self.upgrade()?;
        let view: ColumnRef = column;
        Some(Rc::new(ViewColumnHandle(view)) as Rc<dyn Any>)
    }
}

impl<M: TreeModel> IColumnOf<M> for HierarchicalExpanderColumn<M> {
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>> {
        self.inner.get_comparison(direction)
    }

    fn kind(&self) -> ColumnKind<'_, M> {
        self.inner.kind()
    }

    fn as_expander_column(&self) -> Option<&dyn IExpanderColumn<M>> {
        Some(self)
    }

    fn as_expander_column_any(&self) -> bool {
        true
    }
}

impl<M: TreeModel> IExpanderColumn<M> for HierarchicalExpanderColumn<M> {
    fn has_children(&self, model: &Rc<M>) -> bool {
        HierarchicalExpanderColumn::has_children(self, model)
    }

    fn get_model_is_expanded(&self, model: &Rc<M>) -> Option<bool> {
        self.is_expanded_selector.as_ref().map(|selector| selector.get(model))
    }

    fn get_child_models(&self, model: &Rc<M>) -> Option<Models<M>> {
        HierarchicalExpanderColumn::get_child_models(self, model)
    }

    fn set_model_is_expanded(&self, row: &dyn IExpanderRow<M>) {
        if let Some(selector) = &self.is_expanded_selector {
            selector.set(&row.model_of(), row.is_expanded());
        }
    }

    fn expansion_observer(&self) -> Option<Rc<PropertyPathObserver<M>>> {
        self.expanded_observer.clone()
    }
}
