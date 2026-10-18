//! `Models/TreeDataGrid/`: the view models of the control: the view columns (text, check box,
//! template, expander and the columns of declarative markup), their layout (`ColumnListBase`,
//! `ColumnList`) and the cells they create (text, check box, template and expander cells).
//!
//! The rows, the sources and the neutral columns are the core's
//! (`ferroui_treedatagrid_core::models`); the view columns of this module are also core columns, so
//! that a core source holds them in its column list as the reference's classic sources hold them.

mod contracts;
pub use contracts::*;

mod cell_binding;
mod cell_value;
mod check_box_cell;
mod check_box_column;
mod check_box_column_options;
mod column_base;
mod column_list;
mod column_list_base;
mod column_options;
mod conversions;
pub(crate) mod double_utils;
mod drag_info;
mod expander_cell;
mod filtered_expander_column;
mod hierarchical_expander_column;
mod i_editable_object;
mod object_hierarchical_expander_column;
mod reflection_check_box_column;
mod reflection_text_column;
mod show_expander_observable;
mod template_cell;
mod template_column;
mod template_column_options;
mod text_cell;
mod text_column;
mod text_column_options;
mod tree_data_grid_row_header_column_internal;
mod value_column_base;
mod view_column;

pub use cell_binding::{model_notifier, AccessorBinding, ICellBinding, IRetargetableCellBinding};
pub use cell_value::{box_cell_value, cell_value_from_text, cell_value_text, unbox_cell_value};
pub use check_box_cell::CheckBoxCell;
pub use check_box_column::CheckBoxColumn;
pub use check_box_column_options::CheckBoxColumnOptions;
pub use column_base::ColumnBase;
pub use column_list::ColumnList;
pub use column_list_base::{ColumnListBase, IColumnViewportEstimator};
pub use column_options::{ColumnOptions, IColumnOptions};
pub use conversions::{from_core_grid_length, from_core_options, to_core_grid_length, to_core_options};
pub use drag_info::DragInfo;
pub use expander_cell::ExpanderCell;
pub use filtered_expander_column::{FilteredExpanderColumn, ModelFilter};
pub use hierarchical_expander_column::{ChildSelector, HierarchicalExpanderColumn};
pub use i_editable_object::{as_editable_object, register_editable_model, IEditableObject};
pub use object_hierarchical_expander_column::{ObjectHierarchicalExpanderColumn, ObjectLink};
pub use reflection_check_box_column::ReflectionCheckBoxColumn;
pub use reflection_text_column::{ObjectGetter, ObjectSetter, ReflectionTextColumn};
pub use show_expander_observable::ShowExpanderObservable;
pub use template_cell::{CellTemplateGetter, TemplateCell};
pub use template_column::TemplateColumn;
pub use template_column_options::{TemplateColumnOptions, TextSearchValueSelector};
pub use text_cell::TextCell;
pub use text_column::TextColumn;
pub use text_column_options::{ITextCellCultureOptions, TextColumnOptions};
pub use tree_data_grid_row_header_column_internal::TreeDataGridRowHeaderColumnInternal;
pub use value_column_base::ValueColumnBase;
pub use view_column::{view_column_of, IViewColumn, IViewExpanderColumn, ViewColumnHandle};

#[cfg(test)]
pub(crate) mod tests;
