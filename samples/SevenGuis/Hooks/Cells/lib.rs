//! Cells: a spreadsheet of 26 columns and 100 rows; a cell holds text, a
//! number or a formula (`=B1 * C1`, `=sum(D1:D3)`), double-clicking it
//! edits it, and the values that depend on it follow.
//!
//! Written as components that run again, with their state in hooks; `cargo run -p seven-guis-cells-hooks`.

use ferroui_base::input::Key as KeyCode;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use seven_guis_model::cells::{CellRef, COLUMNS, ROWS};
use seven_guis_model::state::Sheet;

const CELL_WIDTH: f64 = 80.0;

/// The spreadsheet, over its sheet. It reads no signal, so it runs once;
/// each cell runs again on its own.
#[component(rerender)]
pub fn Cells(#[prop(default = Sheet::with_example())] sheet: Sheet) -> Element {
    let columns = format!("40,{}", vec![CELL_WIDTH.to_string(); COLUMNS].join(","));
    let rows = format!("Auto,{}", vec!["24"; ROWS].join(","));
    rsx! {
        ScrollViewer { horizontal_scroll_bar_visibility: ScrollBarVisibility::Auto,
            Grid { row_definitions: &rows, column_definitions: &columns,
                for column in 0..COLUMNS {
                    Header { text: CellRef::column_name(column), row: 0, column: column as i32 + 1 }
                }
                for row in 0..ROWS {
                    Header { text: row.to_string(), row: row as i32 + 1, column: 0 }
                    for column in 0..COLUMNS {
                        Cell { sheet: sheet.clone(), cell: CellRef { column, row } }
                    }
                }
            }
        }
    }
}

/// The header of a column or a row; memoized, so a parent that runs again
/// with the same text does not run it again.
#[component(rerender, memo)]
fn Header(text: String, row: i32, column: i32) -> Element {
    rsx! {
        Border { Grid.row: row, Grid.column: column, background: "#EEEEEE",
            border_brush: "#CCCCCC", border_thickness: (0.0, 0.0, 1.0, 1.0),
            TextBlock { text, horizontal_alignment: HorizontalAlignment::Center,
                vertical_alignment: VerticalAlignment::Center }
        }
    }
}

/// A cell: its value, or a field while it is edited. It runs again when its
/// value changes or it starts or stops being edited: whether it is edited is
/// a memo, so a change of the edited cell runs only the two cells concerned.
#[component(rerender)]
fn Cell(sheet: Sheet, cell: CellRef) -> Element {
    let editing = sheet.editing;
    let is_editing = use_memo(move || editing.get() == Some(cell));
    let content = sheet.content(cell);
    let inner = if is_editing.get() {
        rsx! {
            TextBox { text: content.two_way(), padding: 2.0, min_height: 0.0,
                on_key_down: move |e: &ferroui_base::input::KeyEventArgs| {
                    if e.key == KeyCode::Enter || e.key == KeyCode::Escape {
                        editing.set(None);
                    }
                },
                on_lost_focus: move |_| {
                    if editing.get_untracked() == Some(cell) {
                        editing.set(None);
                    }
                },
                with(|text_box: &ferroui_base::Ref<ferroui_controls::TextBox>| {
                    ferroui_base::input::InputElement::focus(text_box);
                }),
            }
        }
    } else {
        rsx! {
            TextBlock { margin: (4.0, 0.0), vertical_alignment: VerticalAlignment::Center,
                text: sheet.value_of(cell).display() }
        }
    };
    rsx! {
        Border { name: cell.to_string(), Grid.row: cell.row as i32 + 1, Grid.column: cell.column as i32 + 1,
            border_brush: "#DDDDDD", border_thickness: (0.0, 0.0, 1.0, 1.0), background: "White",
            on_double_tapped: move |_| editing.set(Some(cell)),
            {inner}
        }
    }
}
