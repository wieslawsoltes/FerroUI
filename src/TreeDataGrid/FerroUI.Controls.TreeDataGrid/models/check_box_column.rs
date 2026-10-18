//! `Models/TreeDataGrid/CheckBoxColumn.cs`: a column that shows a check box.

use super::column_base::impl_view_column_base;
use super::view_column::row_model;
use super::{from_core_grid_length, CellRef, CheckBoxCell, CheckBoxColumnOptions, ICellColumn, ValueColumnBase};
use ferroui_base::BoxedValue;
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{
    default_comparer, ColumnKind, Comparison, IColumnOf, IValueColumn, ListSortDirection, RowRef, ValueColumn,
};
use ferroui_treedatagrid_core::Accessor;
use std::any::Any;
use std::rc::{Rc, Weak};

/// `CheckBoxColumn<TModel>`: a column of a `bool?` value.
pub struct CheckBoxColumn<M: TreeModel> {
    weak_self: Weak<Self>,
    base: ValueColumnBase<M, Option<bool>>,
    options: Rc<CheckBoxColumnOptions<M>>,
    is_three_state: bool,
}

impl<M: TreeModel> CheckBoxColumn<M> {
    /// A two-state check box column of a `bool` (`CheckBoxColumn(header, Expression<Func<TModel,
    /// bool>>, setter, width, options)`): the accessor's setter makes the column editable, and a
    /// cleared value writes `false`.
    pub fn new(
        header: Option<BoxedValue>,
        accessor: Accessor<M, bool>,
        width: Option<GridLength>,
        options: Option<Rc<CheckBoxColumnOptions<M>>>,
    ) -> Rc<Self> {
        Self::with_three_state(header, accessor.map(Some, |value: Option<bool>| value.unwrap_or(false)), width, options, false)
    }

    /// A three-state check box column of a `bool?` (`CheckBoxColumn(header, Expression<Func<TModel,
    /// bool?>>, setter, width, options)`).
    pub fn new_three_state(
        header: Option<BoxedValue>,
        accessor: Accessor<M, Option<bool>>,
        width: Option<GridLength>,
        options: Option<Rc<CheckBoxColumnOptions<M>>>,
    ) -> Rc<Self> {
        Self::with_three_state(header, accessor, width, options, true)
    }

    /// A check box column of a `bool?` with the given number of states.
    pub fn with_three_state(
        header: Option<BoxedValue>,
        accessor: Accessor<M, Option<bool>>,
        width: Option<GridLength>,
        options: Option<Rc<CheckBoxColumnOptions<M>>>,
        is_three_state: bool,
    ) -> Rc<Self> {
        let options = options.unwrap_or_else(|| Rc::new(CheckBoxColumnOptions::default()));
        let column = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            base: ValueColumnBase::new(header, accessor, Some(default_comparer::<Option<bool>>()), width, options.clone()),
            options,
            is_three_state,
        });
        column.base.set_presentation_key(Some("CheckBox".to_string()));
        column
    }

    /// The view of a neutral check box column of the core (`CheckBoxColumn(CheckBoxColumn<TModel>,
    /// options)`).
    pub fn from_core(column: &ValueColumn<M, Option<bool>>, options: Rc<CheckBoxColumnOptions<M>>) -> Rc<Self> {
        let width = from_core_grid_length(ferroui_treedatagrid_core::models::IColumn::width(column));
        Self::with_three_state(
            ferroui_treedatagrid_core::models::IColumn::header(column),
            column.accessor().clone(),
            Some(width),
            Some(options),
            column.is_three_state(),
        )
    }

    /// The options of the column.
    pub fn options(&self) -> &Rc<CheckBoxColumnOptions<M>> {
        &self.options
    }

    /// The value part of the column (`ColumnBase<TModel, bool?>`).
    pub fn value_base(&self) -> &ValueColumnBase<M, Option<bool>> {
        &self.base
    }

    /// Whether the check boxes have an indeterminate state.
    pub fn is_three_state(&self) -> bool {
        self.is_three_state
    }

    /// Sets the column header.
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.base.set_header(value);
    }

    /// The value of the column for `model`.
    pub fn get_value(&self, model: &Rc<M>) -> Option<bool> {
        (self.base.value_selector())(model)
    }

    /// The cell of `model`.
    pub fn create_cell_of(&self, model: &Rc<M>) -> CellRef {
        CheckBoxCell::new(self.base.create_binding_expression(model), !self.base.can_write(), self.is_three_state)
    }
}

impl_view_column_base!(impl[M: TreeModel] CheckBoxColumn<M>, model M);

impl<M: TreeModel> ICellColumn for CheckBoxColumn<M> {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        self.create_cell_of(&row_model::<M>(row))
    }

    fn try_reuse_cell(&self, cell: &CellRef, row: &RowRef) -> bool {
        cell.as_any()
            .downcast_ref::<CheckBoxCell>()
            .is_some_and(|check_box_cell| check_box_cell.try_set_source(row.model().as_ref()))
    }
}

impl<M: TreeModel> IColumnOf<M> for CheckBoxColumn<M> {
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>> {
        self.base.get_comparison(direction)
    }

    fn kind(&self) -> ColumnKind<'_, M> {
        ColumnKind::Value(self)
    }
}

impl<M: TreeModel> IValueColumn<M> for CheckBoxColumn<M> {
    fn get_boxed_value(&self, model: &Rc<M>) -> Option<BoxedValue> {
        self.get_value(model).map(|value| Rc::new(value) as BoxedValue)
    }

    fn set_boxed_value(&self, model: &Rc<M>, value: Option<&BoxedValue>) -> bool {
        if !self.base.can_write() {
            return false;
        }
        let value = match value {
            None => None,
            Some(value) => match value.downcast_ref::<bool>() {
                Some(value) => Some(*value),
                None => match value.downcast_ref::<Option<bool>>() {
                    Some(value) => *value,
                    None => return false,
                },
            },
        };
        self.base.accessor().set(model, value)
    }

    fn can_set(&self) -> bool {
        self.base.can_write()
    }

    fn property_name(&self) -> Option<&'static str> {
        self.base.accessor().property_name()
    }

    fn value_type_name(&self) -> &'static str {
        std::any::type_name::<Option<bool>>()
    }

    fn as_value_any(&self) -> &dyn Any {
        self
    }
}
