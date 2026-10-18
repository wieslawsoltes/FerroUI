//! `Models/TreeDataGrid/TextColumn.cs`: a column that shows its values as text.

use super::column_base::impl_view_column_base;
use super::view_column::row_model;
use super::{
    box_cell_value, cell_value_text, from_core_grid_length, unbox_cell_value, CellRef, ICellColumn, ITextCellCultureOptions,
    TextCell, TextColumnOptions, ValueColumnBase,
};
use ferroui_base::BoxedValue;
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{
    default_comparer, ColumnKind, Comparison, IColumnOf, IValueColumn, ListSortDirection, RowRef, ValueColumn,
    ValueComparer,
};
use ferroui_treedatagrid_core::Accessor;
use std::any::Any;
use std::rc::{Rc, Weak};

/// `TextColumn<TModel, TValue>`.
pub struct TextColumn<M: TreeModel, V: Clone + PartialEq + 'static> {
    weak_self: Weak<Self>,
    base: ValueColumnBase<M, V>,
    options: Rc<TextColumnOptions<M>>,
}

impl<M: TreeModel, V: Clone + PartialEq + PartialOrd + 'static> TextColumn<M, V> {
    /// A text column of the value `accessor` reads (`TextColumn(header, getter, [setter,] width,
    /// options)`); the accessor's setter makes the column's cells editable. `width` `None` is
    /// `Auto`.
    pub fn new(
        header: Option<BoxedValue>,
        accessor: Accessor<M, V>,
        width: Option<GridLength>,
        options: Option<Rc<TextColumnOptions<M>>>,
    ) -> Rc<Self> {
        Self::with_comparer(header, accessor, Some(default_comparer::<V>()), width, options)
    }

    /// The view of a neutral text column of the core (`TextColumn(ValueColumn<TModel, TValue>,
    /// options)`): its header, value and width, with the view options `options`.
    pub fn from_core(column: &ValueColumn<M, V>, options: Rc<TextColumnOptions<M>>) -> Rc<Self> {
        let width = from_core_grid_length(ferroui_treedatagrid_core::models::IColumn::width(column));
        Self::with_comparer(
            ferroui_treedatagrid_core::models::IColumn::header(column),
            column.accessor().clone(),
            Some(default_comparer::<V>()),
            Some(width),
            Some(options),
        )
    }
}

impl<M: TreeModel, V: Clone + PartialEq + 'static> TextColumn<M, V> {
    /// A text column whose values compare with `comparer` (`None`: the column sorts only with the
    /// comparisons of its options).
    pub fn with_comparer(
        header: Option<BoxedValue>,
        accessor: Accessor<M, V>,
        comparer: Option<ValueComparer<V>>,
        width: Option<GridLength>,
        options: Option<Rc<TextColumnOptions<M>>>,
    ) -> Rc<Self> {
        let options = options.unwrap_or_else(|| Rc::new(TextColumnOptions::default()));
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            base: ValueColumnBase::new(header, accessor, comparer, width, options.clone()),
            options,
        })
    }

    /// The options of the column.
    pub fn options(&self) -> &Rc<TextColumnOptions<M>> {
        &self.options
    }

    /// The value part of the column (`ColumnBase<TModel, TValue>`).
    pub fn value_base(&self) -> &ValueColumnBase<M, V> {
        &self.base
    }

    /// Sets the column header.
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.base.set_header(value);
    }

    /// The value of the column for `model`.
    pub fn get_value(&self, model: &Rc<M>) -> V {
        (self.base.value_selector())(model)
    }

    /// The cell of `model`.
    pub fn create_cell_of(&self, model: &Rc<M>) -> CellRef {
        let options: Rc<dyn ITextCellCultureOptions> = self.options.clone();
        TextCell::<V>::new(self.base.create_binding_expression(model), !self.base.can_write(), Some(options))
    }
}

impl_view_column_base!(impl[M: TreeModel, V: Clone + PartialEq + 'static] TextColumn<M, V>, model M);

impl<M: TreeModel, V: Clone + PartialEq + 'static> ICellColumn for TextColumn<M, V> {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        self.create_cell_of(&row_model::<M>(row))
    }

    fn try_reuse_cell(&self, cell: &CellRef, row: &RowRef) -> bool {
        cell.as_any()
            .downcast_ref::<TextCell<V>>()
            .is_some_and(|text_cell| text_cell.try_set_source(row.model().as_ref()))
    }

    fn select_value(&self, model: &BoxedValue) -> Option<String> {
        let any: Rc<dyn Any> = model.clone();
        let model = any.downcast::<M>().ok()?;
        let value = self.get_value(&model);
        cell_value_text(box_cell_value(&value).as_ref())
    }

    fn is_text_search_enabled(&self) -> bool {
        self.options.is_text_search_enabled()
    }
}

impl<M: TreeModel, V: Clone + PartialEq + 'static> IColumnOf<M> for TextColumn<M, V> {
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>> {
        self.base.get_comparison(direction)
    }

    fn kind(&self) -> ColumnKind<'_, M> {
        ColumnKind::Value(self)
    }
}

impl<M: TreeModel, V: Clone + PartialEq + 'static> IValueColumn<M> for TextColumn<M, V> {
    fn get_boxed_value(&self, model: &Rc<M>) -> Option<BoxedValue> {
        box_cell_value(&self.get_value(model))
    }

    fn set_boxed_value(&self, model: &Rc<M>, value: Option<&BoxedValue>) -> bool {
        if !self.base.can_write() {
            return false;
        }
        match unbox_cell_value::<V>(value) {
            Some(value) => self.base.accessor().set(model, value),
            None => false,
        }
    }

    fn can_set(&self) -> bool {
        self.base.can_write()
    }

    fn property_name(&self) -> Option<&'static str> {
        self.base.accessor().property_name()
    }

    fn value_type_name(&self) -> &'static str {
        std::any::type_name::<V>()
    }

    fn as_value_any(&self) -> &dyn Any {
        self
    }
}
