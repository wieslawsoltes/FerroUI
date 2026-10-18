//! `Models/TreeDataGrid/TreeDataGridRowHeaderColumnInternal.cs`: the column of row headers that
//! show the row's number.

use super::column_base::impl_view_column_base;
use super::{CellRef, ColumnBase, ColumnOptions, ICellColumn, IColumnOptions, TextCell};
use ferroui_base::BoxedValue;
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{ColumnKind, Comparison, IColumnOf, IValueColumn, ListSortDirection, RowRef};
use std::any::Any;
use std::rc::{Rc, Weak};

/// `TreeDataGridRowHeaderColumnInternal<TModel>`.
pub struct TreeDataGridRowHeaderColumnInternal<M: TreeModel> {
    weak_self: Weak<Self>,
    base: ColumnBase<M>,
}

impl<M: TreeModel> TreeDataGridRowHeaderColumnInternal<M> {
    pub fn new(header: Option<BoxedValue>, width: Option<GridLength>, options: Option<Rc<ColumnOptions<M>>>) -> Rc<Self> {
        let options: Rc<dyn IColumnOptions<M>> = options.unwrap_or_else(|| Rc::new(ColumnOptions::default()));
        Rc::new_cyclic(|weak_self| Self { weak_self: weak_self.clone(), base: ColumnBase::new(header, width, options) })
    }

    /// Sets the column header.
    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.base.set_header(value);
    }
}

impl_view_column_base!(impl[M: TreeModel] TreeDataGridRowHeaderColumnInternal<M>, model M);

impl<M: TreeModel> ICellColumn for TreeDataGridRowHeaderColumnInternal<M> {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        match row.as_model_indexable() {
            Some(indexable) => TextCell::from_value(Some((indexable.model_index_path().last() + 1).to_string())),
            None => TextCell::from_value(Some(String::new())),
        }
    }
}

impl<M: TreeModel> IColumnOf<M> for TreeDataGridRowHeaderColumnInternal<M> {
    fn get_comparison(&self, _direction: ListSortDirection) -> Option<Comparison<M>> {
        None
    }

    fn kind(&self) -> ColumnKind<'_, M> {
        ColumnKind::Value(self)
    }
}

// The value of a row header is the row's number, which the model does not give: the column has
// no value of a model.
impl<M: TreeModel> IValueColumn<M> for TreeDataGridRowHeaderColumnInternal<M> {
    fn get_boxed_value(&self, _model: &Rc<M>) -> Option<BoxedValue> {
        None
    }

    fn set_boxed_value(&self, _model: &Rc<M>, _value: Option<&BoxedValue>) -> bool {
        false
    }

    fn can_set(&self) -> bool {
        false
    }

    fn property_name(&self) -> Option<&'static str> {
        None
    }

    fn value_type_name(&self) -> &'static str {
        std::any::type_name::<String>()
    }

    fn as_value_any(&self) -> &dyn Any {
        self
    }
}
