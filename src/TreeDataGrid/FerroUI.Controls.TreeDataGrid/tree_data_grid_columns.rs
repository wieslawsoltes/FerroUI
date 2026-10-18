//! `TreeDataGridColumns.cs`: the declarative columns of a grid (`ColumnDefinitions`), a notifying
//! list.

use crate::TreeDataGridColumn;
use ferroui_base::collections::FerroList;
use std::ops::Deref;
use std::rc::Rc;

/// The declarative columns of a grid (`TreeDataGridColumns`, a list of [`TreeDataGridColumn`]s).
#[derive(Clone, PartialEq)]
pub struct TreeDataGridColumns(FerroList<Rc<dyn TreeDataGridColumn>>);

impl TreeDataGridColumns {
    pub fn new() -> Self {
        Self(FerroList::new())
    }

    /// The list.
    pub fn list(&self) -> &FerroList<Rc<dyn TreeDataGridColumn>> {
        &self.0
    }

    /// Adds a column of any declarative column type.
    pub fn add_column(&self, column: Rc<impl TreeDataGridColumn>) {
        self.0.add(column as Rc<dyn TreeDataGridColumn>);
    }
}

impl Default for TreeDataGridColumns {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for TreeDataGridColumns {
    type Target = FerroList<Rc<dyn TreeDataGridColumn>>;

    fn deref(&self) -> &FerroList<Rc<dyn TreeDataGridColumn>> {
        &self.0
    }
}
