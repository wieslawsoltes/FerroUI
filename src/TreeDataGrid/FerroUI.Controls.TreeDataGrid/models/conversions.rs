//! The conversions between the view's lengths and options and the core's (the reference's
//! `Adapters/CoreConversions.cs` for what the view columns need): a view column is also a core
//! column, so it answers the core's width in the core's `GridLength`.

use super::ColumnOptions;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_treedatagrid_core as core;

/// The core's form of a view length.
pub fn to_core_grid_length(length: GridLength) -> core::GridLength {
    let unit = match length.grid_unit_type() {
        GridUnitType::Auto => core::GridUnitType::Auto,
        GridUnitType::Pixel => core::GridUnitType::Pixel,
        GridUnitType::Star => core::GridUnitType::Star,
    };
    core::GridLength::new(length.value(), unit)
}

/// The view's form of a core length.
pub fn from_core_grid_length(length: core::GridLength) -> GridLength {
    let unit = match length.grid_unit_type() {
        core::GridUnitType::Auto => GridUnitType::Auto,
        core::GridUnitType::Pixel => GridUnitType::Pixel,
        core::GridUnitType::Star => GridUnitType::Star,
    };
    GridLength::new(length.value(), unit)
}

/// The core's options of a column with the view options `options` (the gestures are the view's
/// own).
pub fn to_core_options<M: 'static>(options: &ColumnOptions<M>) -> core::models::ColumnOptions<M> {
    core::models::ColumnOptions {
        can_user_resize_column: options.can_user_resize_column(),
        can_user_sort_column: options.can_user_sort_column(),
        min_width: to_core_grid_length(options.min_width()),
        max_width: options.max_width().map(to_core_grid_length),
        compare_ascending: options.compare_ascending(),
        compare_descending: options.compare_descending(),
    }
}

/// The view options of a column with the core options `options` and the default gestures.
pub fn from_core_options<M: 'static>(options: &core::models::ColumnOptions<M>) -> ColumnOptions<M> {
    let result = ColumnOptions::new();
    result.set_can_user_resize_column(options.can_user_resize_column);
    result.set_can_user_sort_column(options.can_user_sort_column);
    result.set_min_width(from_core_grid_length(options.min_width));
    result.set_max_width(options.max_width.map(from_core_grid_length));
    result.set_compare_ascending(options.compare_ascending.clone());
    result.set_compare_descending(options.compare_descending.clone());
    result
}
