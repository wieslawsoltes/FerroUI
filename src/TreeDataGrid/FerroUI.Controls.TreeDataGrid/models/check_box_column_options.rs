//! `Models/TreeDataGrid/CheckBoxColumnOptions.cs`: the options of a check box column.

use super::{BeginEditGestures, ColumnOptions, ICellOptions, IColumnOptions};
use std::ops::Deref;

/// `CheckBoxColumnOptions<TModel>`.
pub struct CheckBoxColumnOptions<M: 'static> {
    base: ColumnOptions<M>,
}

impl<M: 'static> Default for CheckBoxColumnOptions<M> {
    fn default() -> Self {
        Self { base: ColumnOptions::default() }
    }
}

impl<M: 'static> CheckBoxColumnOptions<M> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<M: 'static> Deref for CheckBoxColumnOptions<M> {
    type Target = ColumnOptions<M>;

    fn deref(&self) -> &ColumnOptions<M> {
        &self.base
    }
}

impl<M: 'static> IColumnOptions<M> for CheckBoxColumnOptions<M> {
    fn column_options(&self) -> &ColumnOptions<M> {
        &self.base
    }
}

impl<M: 'static> ICellOptions for CheckBoxColumnOptions<M> {
    fn begin_edit_gestures(&self) -> BeginEditGestures {
        self.base.begin_edit_gestures()
    }
}
