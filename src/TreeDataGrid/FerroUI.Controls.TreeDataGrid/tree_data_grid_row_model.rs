//! `TreeDataGridRowModel.cs` and `TreeDataGridRowModelEventArgs.cs`: a row as its model and the
//! index of the model.

use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::IndexPath;

/// A row of a grid as its model and the index of the model in the data (`TreeDataGridRowModel`).
#[derive(Clone)]
pub struct TreeDataGridRowModel {
    model: Option<BoxedValue>,
    model_index_path: IndexPath,
}

impl TreeDataGridRowModel {
    pub fn new(model: Option<BoxedValue>, model_index_path: IndexPath) -> Self {
        Self { model, model_index_path }
    }

    /// The model of the row.
    pub fn model(&self) -> Option<BoxedValue> {
        self.model.clone()
    }

    /// The index of the model from the root of the data.
    pub fn model_index_path(&self) -> &IndexPath {
        &self.model_index_path
    }
}

impl PartialEq for TreeDataGridRowModel {
    fn eq(&self, other: &Self) -> bool {
        let same_model = match (&self.model, &other.model) {
            (Some(a), Some(b)) => std::ptr::addr_eq(std::rc::Rc::as_ptr(a), std::rc::Rc::as_ptr(b)),
            (None, None) => true,
            _ => false,
        };
        same_model && self.model_index_path == other.model_index_path
    }
}

/// The arguments of an event about a row model (`TreeDataGridRowModelEventArgs`).
#[derive(Clone)]
pub struct TreeDataGridRowModelEventArgs {
    row: TreeDataGridRowModel,
}

impl TreeDataGridRowModelEventArgs {
    pub fn new(row: TreeDataGridRowModel) -> Self {
        Self { row }
    }

    /// The row.
    pub fn row(&self) -> &TreeDataGridRowModel {
        &self.row
    }
}
