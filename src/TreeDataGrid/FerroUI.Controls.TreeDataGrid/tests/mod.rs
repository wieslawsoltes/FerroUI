//! The control tests of the reference (the test project of the control): the grid
//! over flat and hierarchical sources, the declarative columns, the sources seen through the grid,
//! the presentation of core sources, cell recycling and cell selection, the diagnostics, the tree
//! selection model, the bindings of cells and the automation peers. One module per test file of
//! the reference.

pub(crate) mod support;

mod core_cell_recycling_tests;
mod core_cell_selection_tests;
mod core_presentation_regression_tests;
mod core_source_presentation_tests;
mod flat_tree_data_grid_source_tests;
mod hierarchical_tree_data_grid_source_tests;
mod tree_data_grid_diagnostics_tests;
mod tree_data_grid_tests_flat;
mod tree_data_grid_tests_hierarchical;
mod tree_data_grid_v12_tests;
mod tree_selection_model_base_tests_multiple;
mod tree_selection_model_base_tests_single;
mod typed_binding_tests;

mod automation_peer_test_helper;
mod tree_data_grid_automation_peer_tests;
mod tree_data_grid_cell_automation_peer_tests;
mod tree_data_grid_check_box_cell_automation_peer_tests;
mod tree_data_grid_column_header_automation_peer_tests;
mod tree_data_grid_column_headers_presenter_automation_peer_tests;
mod tree_data_grid_row_automation_peer_tests;
