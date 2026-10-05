//! Helpers shared by the grid tests.

use crate::{Border, ColumnDefinitions, Grid, RowDefinitions};
use ferroui_base::{Rect, Ref, Size};

pub(crate) struct GridMock;

impl GridMock {
    /// Creates a mock grid to test its layout. This contains the arrange
    /// (`Grid::new()`) and action (`measure()`/`arrange()`) steps.
    ///
    /// * `measure`: the measure size of the grid. Infinity by default.
    /// * `arrange`: the arrange size of the grid. Its desired size by
    ///   default.
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new(measure: Option<Size>, arrange: Option<Size>) -> Ref<Grid> {
        let grid = Grid::new();
        grid.children().add(Border::new());
        grid.measure(measure.unwrap_or(Size::INFINITY));
        grid.arrange(Rect::from_size(arrange.unwrap_or(grid.desired_size())));
        grid
    }

    /// Creates a mock grid to test its row layout.
    ///
    /// * `measure`: the measure height of the grid. Infinity when zero.
    /// * `arrange`: the arrange height of the grid, when not zero.
    pub(crate) fn new_rows(rows: RowDefinitions, measure: f64, mut arrange: f64) -> Ref<Grid> {
        let grid = Grid::new();
        grid.set_row_definitions(rows);
        for i in 0..grid.row_definitions().count() {
            let border = Border::new();
            Grid::set_row(&border, i as i32);
            grid.children().add(border);
        }

        grid.measure(Size::new(
            f64::INFINITY,
            if measure == 0.0 { f64::INFINITY } else { measure },
        ));
        if arrange == 0.0 {
            arrange = if measure == 0.0 {
                grid.desired_size().width
            } else {
                measure
            };
        }

        grid.arrange(Rect::new(0.0, 0.0, 0.0, arrange));

        grid
    }

    /// Creates a mock grid to test its column layout.
    ///
    /// * `measure`: the measure width of the grid. Infinity when zero.
    /// * `arrange`: the arrange width of the grid, when not zero.
    pub(crate) fn new_columns(columns: ColumnDefinitions, measure: f64, mut arrange: f64) -> Ref<Grid> {
        let grid = Grid::new();
        grid.set_column_definitions(columns);
        for i in 0..grid.column_definitions().count() {
            let border = Border::new();
            Grid::set_column(&border, i as i32);
            grid.children().add(border);
        }

        grid.measure(Size::new(
            if measure == 0.0 { f64::INFINITY } else { measure },
            f64::INFINITY,
        ));
        if arrange == 0.0 {
            arrange = if measure == 0.0 {
                grid.desired_size().width
            } else {
                measure
            };
        }

        grid.arrange(Rect::new(0.0, 0.0, arrange, 0.0));

        grid
    }
}

pub(crate) struct GridAssert;

impl GridAssert {
    /// Asserts all the children heights. This assumes that the grid children
    /// count equals the row count.
    pub(crate) fn children_height(grid: &Grid, rows: &[f64]) {
        assert_eq!(grid.children().count(), rows.len());

        for (i, row) in rows.iter().enumerate() {
            assert_eq!(*row, grid.children().get(i).bounds().height);
        }
    }

    /// Asserts all the children widths. This assumes that the grid children
    /// count equals the column count.
    pub(crate) fn children_width(grid: &Grid, columns: &[f64]) {
        assert_eq!(grid.children().count(), columns.len());

        for (i, column) in columns.iter().enumerate() {
            assert_eq!(*column, grid.children().get(i).bounds().width);
        }
    }
}
