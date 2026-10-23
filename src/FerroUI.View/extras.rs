//! Builder methods for what the generator cannot see: values of classes that
//! are not registered properties.

use crate::builder::El;
use ferroui_base::{ObjectType, Upcast};
use ferroui_controls::{ColumnDefinitions, Grid, RowDefinitions};

/// The rows and columns of a grid, in the text form XAML takes
/// (`"Auto,*,2*,40"`).
pub trait GridDefinitions {
    /// `Grid.RowDefinitions`.
    fn row_definitions(self, rows: &str) -> Self;
    /// `Grid.ColumnDefinitions`.
    fn column_definitions(self, columns: &str) -> Self;
}

impl<T: ObjectType + Upcast<Grid>> GridDefinitions for El<T> {
    fn row_definitions(self, rows: &str) -> Self {
        let rows = RowDefinitions::parse(rows).unwrap_or_else(|_| panic!("`{rows}` are not row definitions"));
        self.with(move |grid| {
            let grid: &Ref<Grid> = grid.upcast_ref();
            grid.set_row_definitions(rows.clone());
        })
    }

    fn column_definitions(self, columns: &str) -> Self {
        let columns =
            ColumnDefinitions::parse(columns).unwrap_or_else(|_| panic!("`{columns}` are not column definitions"));
        self.with(move |grid| {
            let grid: &Ref<Grid> = grid.upcast_ref();
            grid.set_column_definitions(columns.clone());
        })
    }
}

use ferroui_base::Ref;
