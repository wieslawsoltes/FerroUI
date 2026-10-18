//! `Models/TreeDataGrid/FilteredExpanderColumn.cs`: an expander column whose child models are
//! those of another expander column that pass the filter of the source.

use super::{
    from_core_grid_length, CellRef, ColumnRef, ICellColumn, IColumn, IUpdateColumnLayout, IViewExpanderColumn,
    ViewColumnHandle,
};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::BoxedValue;
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{
    self as core_models, ColumnKind, Comparison, IColumnOf, IExpanderColumn, IExpanderRow, ListSortDirection,
    NotifyingBase, RowRef,
};
use ferroui_treedatagrid_core::{Models, PropertyPathObserver};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// A filter of models (`Func<TModel, bool>`).
pub type ModelFilter<M> = Rc<dyn Fn(&Rc<M>) -> bool>;

/// `FilteredExpanderColumn<TModel>`.
pub struct FilteredExpanderColumn<M: TreeModel> {
    weak_self: Weak<Self>,
    notifying: NotifyingBase,
    inner: Rc<dyn IViewExpanderColumn<M>>,
    inner_token: Cell<Option<u64>>,
    filter_accessor: Rc<dyn Fn() -> Option<ModelFilter<M>>>,
}

impl<M: TreeModel> FilteredExpanderColumn<M> {
    /// The expander column `inner` with its child models filtered by the filter
    /// `filter_accessor` returns when the children are read.
    pub fn new(inner: Rc<dyn IViewExpanderColumn<M>>, filter_accessor: impl Fn() -> Option<ModelFilter<M>> + 'static) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            notifying: NotifyingBase::new(),
            inner: inner.clone(),
            inner_token: Cell::new(None),
            filter_accessor: Rc::new(filter_accessor),
        });
        let weak = Rc::downgrade(&this);
        let token = inner.property_changed().add(Rc::new(move |name: &str| {
            if let Some(this) = weak.upgrade() {
                this.notifying.raise_property_changed(name);
            }
        }));
        this.inner_token.set(Some(token));
        this
    }

    /// The expander column whose children are filtered.
    pub fn inner(&self) -> &Rc<dyn IViewExpanderColumn<M>> {
        &self.inner
    }

    /// Whether `model` has children that pass the filter.
    pub fn has_children(&self, model: &Rc<M>) -> bool {
        self.get_child_models(model).is_some_and(|children| children.count() > 0)
    }

    /// The child models of `model` that pass the filter.
    pub fn get_child_models(&self, model: &Rc<M>) -> Option<Models<M>> {
        let children = self.inner.get_child_models(model);
        let filter = (self.filter_accessor)();

        match (filter, children) {
            (Some(filter), Some(children)) => {
                let filtered: Vec<Rc<M>> = children.to_vec().into_iter().filter(|child| filter(child)).collect();
                Some(Rc::new(filtered) as Models<M>)
            }
            (_, children) => children,
        }
    }
}

impl<M: TreeModel> Drop for FilteredExpanderColumn<M> {
    fn drop(&mut self) {
        if let Some(token) = self.inner_token.take() {
            self.inner.property_changed().remove(token);
        }
    }
}

impl<M: TreeModel> INotifyPropertyChanged for FilteredExpanderColumn<M> {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl<M: TreeModel> IColumn for FilteredExpanderColumn<M> {
    fn actual_width(&self) -> f64 {
        IColumn::actual_width(&*self.inner)
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

impl<M: TreeModel> IUpdateColumnLayout for FilteredExpanderColumn<M> {
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
        self.inner.calculate_star_width(available_width, total_stars)
    }

    fn commit_actual_width(&self) -> bool {
        self.inner.commit_actual_width()
    }

    fn set_width(&self, width: GridLength) {
        IUpdateColumnLayout::set_width(&*self.inner, width)
    }
}

impl<M: TreeModel> ICellColumn for FilteredExpanderColumn<M> {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        ICellColumn::create_cell(&*self.inner, row)
    }

    fn try_reuse_cell(&self, cell: &CellRef, row: &RowRef) -> bool {
        ICellColumn::try_reuse_cell(&*self.inner, cell, row)
    }
}

impl<M: TreeModel> core_models::IColumn for FilteredExpanderColumn<M> {
    fn id(&self) -> String {
        core_models::IColumn::id(&*self.inner)
    }

    fn header(&self) -> Option<BoxedValue> {
        IColumn::header(&*self.inner)
    }

    fn width(&self) -> ferroui_treedatagrid_core::GridLength {
        core_models::IColumn::width(&*self.inner)
    }

    fn set_width(&self, value: ferroui_treedatagrid_core::GridLength) {
        IUpdateColumnLayout::set_width(&*self.inner, from_core_grid_length(value))
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

impl<M: TreeModel> IColumnOf<M> for FilteredExpanderColumn<M> {
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

impl<M: TreeModel> IExpanderColumn<M> for FilteredExpanderColumn<M> {
    fn has_children(&self, model: &Rc<M>) -> bool {
        FilteredExpanderColumn::has_children(self, model)
    }

    fn get_model_is_expanded(&self, model: &Rc<M>) -> Option<bool> {
        self.inner.get_model_is_expanded(model)
    }

    fn get_child_models(&self, model: &Rc<M>) -> Option<Models<M>> {
        FilteredExpanderColumn::get_child_models(self, model)
    }

    fn set_model_is_expanded(&self, row: &dyn IExpanderRow<M>) {
        self.inner.set_model_is_expanded(row);
    }

    fn expansion_observer(&self) -> Option<Rc<PropertyPathObserver<M>>> {
        self.inner.expansion_observer()
    }
}
