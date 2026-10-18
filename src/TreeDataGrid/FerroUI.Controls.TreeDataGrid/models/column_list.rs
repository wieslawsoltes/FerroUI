//! `Models/TreeDataGrid/ColumnList.cs`: the list of view columns a presentation lays out.
//!
//! The reference's `ColumnList<TModel>` is `ColumnListBase<IColumn<TModel>>`, typed by the model
//! only to accept the columns of one model type. The presentation of the port is untyped and holds
//! its columns as view columns, so the column list is the column list base, and a list is built
//! from view columns of any model type or from the columns of a core source whose views are view
//! columns of this crate.

use super::{view_column_of, ColumnListBase, ColumnRef};
use ferroui_treedatagrid_core::models as core_models;
use std::rc::Rc;

/// `ColumnList<TModel>`.
pub type ColumnList = ColumnListBase;

impl ColumnListBase {
    /// A list of the view columns of `columns` (the columns of a core source), in order; a column
    /// that is not a view column (a neutral column of the core, which the presentation presents
    /// through its options) is given to `present`, which returns its view column or `None` to
    /// leave it out.
    pub fn from_core_columns(
        columns: impl IntoIterator<Item = Rc<dyn core_models::IColumn>>,
        mut present: impl FnMut(&Rc<dyn core_models::IColumn>) -> Option<ColumnRef>,
    ) -> Rc<Self> {
        let this = Self::new();
        for column in columns {
            let view = view_column_of(&*column).or_else(|| present(&column));
            if let Some(view) = view {
                this.add(view);
            }
        }
        this
    }

    /// Adds the view of a core column; false when the core column is not a view column of this
    /// crate.
    pub fn add_core_column(&self, column: &dyn core_models::IColumn) -> bool {
        match view_column_of(column) {
            Some(view) => {
                self.add(view);
                true
            }
            None => false,
        }
    }
}
