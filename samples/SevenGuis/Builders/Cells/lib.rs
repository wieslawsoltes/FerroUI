//! Cells: a spreadsheet of 26 columns and 100 rows. A cell shows its value;
//! a double click edits what it holds (a number, text, or a formula after
//! `=`, see `seven_guis_model::cells`), and Enter or leaving the field keeps
//! it. Each cell's value is a memo of the values it names, so a change is
//! computed again in the cells that depend on it, and only in those.
//!
//! Written with the builders of `ferroui-view`; `cargo run -p seven-guis-cells-builders`.

use ferroui_base::input::Key as KeyCode;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_view::prelude::*;
use seven_guis_model::cells::{CellRef, COLUMNS, ROWS};
use seven_guis_model::state::Sheet;

/// The spreadsheet.
pub fn cells() -> Element {
    cells_of(Sheet::with_example())
}

const CELL_WIDTH: f64 = 80.0;

/// The spreadsheet of a sheet: what the tests drive.
pub fn cells_of(sheet: Sheet) -> Element {
    let columns = format!("40,{}", vec![CELL_WIDTH.to_string(); COLUMNS].join(","));
    let rows = format!("Auto,{}", vec!["24"; ROWS].join(","));
    let mut grid = grid().row_definitions(&rows).column_definitions(&columns);
    for column in 0..COLUMNS {
        grid = grid.child(header(CellRef::column_name(column), 0, column as i32 + 1));
    }
    for row in 0..ROWS {
        grid = grid.child(header(row.to_string(), row as i32 + 1, 0));
        for column in 0..COLUMNS {
            grid = grid.child(cell(sheet.clone(), CellRef { column, row }));
        }
    }
    scroll_viewer()
        .horizontal_scroll_bar_visibility(ferroui_controls::primitives::ScrollBarVisibility::Auto)
        .child(grid)
        .into()
}

fn header(text: String, row: i32, column: i32) -> El<ferroui_controls::Border> {
    border()
        .grid_row(row)
        .grid_column(column)
        .background("#EEEEEE")
        .border_brush("#CCCCCC")
        .border_thickness((0.0, 0.0, 1.0, 1.0))
        .child(text_block().text(text).horizontal_alignment(HorizontalAlignment::Center).vertical_alignment(VerticalAlignment::Center))
}

fn cell(sheet: Sheet, cell: CellRef) -> El<ferroui_controls::Border> {
    let editing = sheet.editing;
    let content = sheet.content(cell);
    let shown = sheet.clone();
    border()
        .name(cell.to_string())
        .grid_row(cell.row as i32 + 1)
        .grid_column(cell.column as i32 + 1)
        .border_brush("#DDDDDD")
        .border_thickness((0.0, 0.0, 1.0, 1.0))
        .background("White")
        .on_double_tapped(move |_| editing.set(Some(cell)))
        .child(show_else(
            move || editing.get() == Some(cell),
            move || {
                text_box()
                    .text(content.two_way())
                    .padding(2.0)
                    .min_height(0.0)
                    .on_key_down(move |e| {
                        if e.key == KeyCode::Enter || e.key == KeyCode::Escape {
                            editing.set(None);
                        }
                    })
                    .on_lost_focus(move |_| {
                        if editing.get_untracked() == Some(cell) {
                            editing.set(None);
                        }
                    })
                    .with(|text_box| {
                        ferroui_base::input::InputElement::focus(text_box);
                    })
            },
            move || {
                let shown = shown.clone();
                text_block()
                    .margin((4.0, 0.0))
                    .vertical_alignment(VerticalAlignment::Center)
                    .text(move || shown.value_of(cell).display())
            },
        ))
}
