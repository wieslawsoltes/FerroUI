//! `Presentation/CellColumnOptions.cs`: the view-only sizing options of a presented column.

use ferroui_controls::GridLength;

/// View-only sizing options (`CellColumnOptions`). Sorting and value access belong to the core's
/// columns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellColumnOptions {
    pub can_user_resize_column: Option<bool>,
    pub min_width: GridLength,
    pub max_width: Option<GridLength>,
}

impl Default for CellColumnOptions {
    fn default() -> Self {
        Self { can_user_resize_column: None, min_width: GridLength::from_pixels(30.0), max_width: None }
    }
}
