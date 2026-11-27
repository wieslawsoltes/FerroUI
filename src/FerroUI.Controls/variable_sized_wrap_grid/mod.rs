//! WinUI's `VariableSizedWrapGrid` (#1271; docs/porting/winui-controls.md, section 10.19).

mod occupancy_map;
#[allow(clippy::module_inception)]
mod variable_sized_wrap_grid;

#[cfg(test)]
mod variable_sized_wrap_grid_tests;

pub use variable_sized_wrap_grid::VariableSizedWrapGrid;
