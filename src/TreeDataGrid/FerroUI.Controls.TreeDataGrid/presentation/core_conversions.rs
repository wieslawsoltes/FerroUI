//! `Adapters/CoreConversions.cs`: the conversions between the lengths of the core and of the
//! framework.
//!
//! The reference also converts between two `IndexPath` types and maps lists of them; the port has
//! one `IndexPath` (the core's), so only the lengths remain.

use ferroui_controls::{GridLength, GridUnitType};
use ferroui_treedatagrid_core as core;

/// The framework length of a core length (the reference's conversion to the framework's length).
pub fn to_view_length(value: core::GridLength) -> GridLength {
    let unit = match value.grid_unit_type() {
        core::GridUnitType::Auto => GridUnitType::Auto,
        core::GridUnitType::Pixel => GridUnitType::Pixel,
        core::GridUnitType::Star => GridUnitType::Star,
    };
    GridLength::new(value.value(), unit)
}

/// The core length of a framework length (`ToCore`).
pub fn to_core_length(value: GridLength) -> core::GridLength {
    let unit = match value.grid_unit_type() {
        GridUnitType::Auto => core::GridUnitType::Auto,
        GridUnitType::Pixel => core::GridUnitType::Pixel,
        GridUnitType::Star => core::GridUnitType::Star,
    };
    core::GridLength::new(value.value(), unit)
}
