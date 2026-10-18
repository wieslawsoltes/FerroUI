//! `Primitives/`: the virtualizing presenters of the rows, cells and column headers, the row, the
//! cells, the column header and the element factory that creates and recycles them.

mod i_final_measure_selector;
mod i_natural_width_measure_cache;
mod i_tree_data_grid_cell;
mod realized_stack_elements;
mod tree_data_grid_cell;
mod tree_data_grid_cells_presenter;
mod tree_data_grid_check_box_cell;
mod tree_data_grid_column_header;
mod tree_data_grid_column_headers_presenter;
mod tree_data_grid_columnar_presenter_base;
mod tree_data_grid_element_factory;
mod tree_data_grid_expander_cell;
mod tree_data_grid_presenter_base;
mod tree_data_grid_row;
mod tree_data_grid_row_unrealize_reason;
mod tree_data_grid_rows_presenter;
mod tree_data_grid_template_cell;
mod tree_data_grid_text_cell;

pub use i_final_measure_selector::IFinalMeasureSelector;
pub use i_natural_width_measure_cache::{as_natural_width_measure_cache, INaturalWidthMeasureCache};
pub use i_tree_data_grid_cell::ITreeDataGridCell;
pub use realized_stack_elements::RealizedStackElements;
pub use tree_data_grid_cell::{
    TreeDataGridCell, TreeDataGridCellImpl, TreeDataGridCellImplExt, TreeDataGridCellVTable,
};
pub use tree_data_grid_cells_presenter::{
    TreeDataGridCellsPresenter, TreeDataGridCellsPresenterImpl, TreeDataGridCellsPresenterImplExt,
    TreeDataGridCellsPresenterVTable,
};
pub use tree_data_grid_check_box_cell::TreeDataGridCheckBoxCell;
pub use tree_data_grid_column_header::TreeDataGridColumnHeader;
pub use tree_data_grid_column_headers_presenter::TreeDataGridColumnHeadersPresenter;
pub use tree_data_grid_columnar_presenter_base::{as_column_viewport_estimator, TreeDataGridColumnarPresenterBase};
pub use tree_data_grid_element_factory::{
    type_recycle_key, RecycleKey, TreeDataGridElementFactory, TreeDataGridElementFactoryImpl,
};
pub use tree_data_grid_expander_cell::TreeDataGridExpanderCell;
pub use tree_data_grid_presenter_base::{PresenterItemsChange, TreeDataGridPresenterBase, TreeDataGridPresenterHooks};
pub use tree_data_grid_row::{TreeDataGridRow, TreeDataGridRowImpl, TreeDataGridRowImplExt, TreeDataGridRowVTable};
pub use tree_data_grid_row_unrealize_reason::TreeDataGridRowUnrealizeReason;
pub use tree_data_grid_rows_presenter::TreeDataGridRowsPresenter;
pub use tree_data_grid_template_cell::{TemplateCellDataContext, TreeDataGridTemplateCell};
pub use tree_data_grid_text_cell::TreeDataGridTextCell;

#[cfg(test)]
pub(crate) mod tests;
