//! `Presentation/`: the presentation of a core source to the control: the view columns of the
//! source's columns, the rows that realize the cells, the selection interaction and sorting.
//!
//! The reference's presentation is typed by the model (`TreeDataGridPresentation<TModel>`) and
//! created through a visitor of the source; the port's presentation is untyped
//! (docs/porting/treedatagrid.md, 3.1): models are untyped values, the values of neutral columns
//! are read through the core's erased access to its columns, and the typed operations the
//! presentation needs of a model type (creating selections, reading a cell selection, following
//! the source's columns) come from the model type's registration
//! ([`register_tree_data_grid_model`](crate::selection::register_tree_data_grid_model)).
//!
//! The adapters of the reference (`Adapters/CoreConversions.cs`, `Adapters/LegacySourcePresentation.cs`)
//! fold into this module: a source whose columns are the control crate's view columns is
//! presented by the same presentation, which uses their views directly.

mod contract;
pub use contract::*;

mod cell_binding;
mod cell_column;
mod cell_column_base;
mod cell_column_factory;
mod cell_column_options;
mod cell_model_pool;
mod core_conversions;
mod expander_column_presentation;
mod tree_data_grid_presentation;
mod tree_data_grid_presentation_options;
mod tree_data_grid_rows;

pub use cell_binding::{check_box_conversion, untyped_conversion, CellValueConversion, CoreCellBinding};
pub use cell_column::{CoreCheckBoxColumn, CoreTextColumn};
pub use cell_column_base::CellColumnBase;
pub(crate) use cell_column_base::{are_close, is_zero};
pub use cell_column_factory::CellColumnFactory;
pub use cell_column_options::CellColumnOptions;
pub use cell_model_pool::CellModelPool;
pub use core_conversions::{to_core_length, to_view_length};
pub use expander_column_presentation::{CoreExpanderCell, ExpanderColumnPresentation};
pub use tree_data_grid_presentation::{apply_selection_mode_to, create_presentation, PresentationColumns, TreeDataGridSourcePresentation};
pub use tree_data_grid_presentation_options::{
    ColumnPresentationFactory, ITreeDataGridPresentationOptions, TreeDataGridPresentationOptions,
};
pub use tree_data_grid_rows::{as_reusable_cell_rows, IReusableCellRows, TreeDataGridRows};

use crate::models::ColumnRef;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

impl PartialEq for dyn TreeDataGridPresentation {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// A view column that releases what it holds when its presentation drops it
/// (`(column as IDisposable)?.Dispose()`).
pub trait IDisposableColumn {
    fn dispose(&self);
}

type ColumnDisposer = Rc<dyn Fn(&dyn Any)>;

ferroui_base::ferro_thread_local! {
    static COLUMN_DISPOSERS: RefCell<HashMap<TypeId, ColumnDisposer>> = RefCell::new(HashMap::new());
}

/// Registers the view column type `T` as one that a presentation disposes when it drops a column of
/// the type (the reference's `IDisposable` columns): a view column is a trait object, which cannot
/// be asked for another trait it implements.
pub fn register_disposable_column<T: IDisposableColumn + 'static>() {
    let disposer: ColumnDisposer = Rc::new(|column: &dyn Any| {
        if let Some(column) = column.downcast_ref::<T>() {
            column.dispose();
        }
    });
    COLUMN_DISPOSERS.with(|disposers| {
        disposers.borrow_mut().insert(TypeId::of::<T>(), disposer);
    });
}

/// Disposes a view column a presentation drops (`(cell as IDisposable)?.Dispose()`).
pub fn dispose_column(column: &ColumnRef) {
    let any = column.as_any();
    if let Some(expander) = any.downcast_ref::<ExpanderColumnPresentation>() {
        expander.dispose();
        return;
    }
    let disposer = COLUMN_DISPOSERS.with(|disposers| disposers.borrow().get(&any.type_id()).cloned());
    if let Some(disposer) = disposer {
        disposer(any);
    }
}
