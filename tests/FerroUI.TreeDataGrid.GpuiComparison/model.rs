//! What every target shows: the viewport, the rows, the columns and the text of the cells, taken
//! from the TreeDataGrid benchmarks (`tests/FerroUI.TreeDataGrid.Benchmarks`), and the grid
//! specification a scenario creates.

/// The window of `VirtualizationBenchmarks.cs` and `FirstLayoutBenchmarks.cs`: 800 x 500.
pub const VIEWPORT_WIDTH: f64 = 800.0;
pub const VIEWPORT_HEIGHT: f64 = 500.0;
/// The height of a row (the style of the rows of the benchmarks).
pub const ROW_HEIGHT: f64 = 24.0;
/// The font size of the text: the default of FerroUI's text elements, given to GPUI explicitly.
pub const FONT_SIZE: f64 = 12.0;
/// The family of the platform text variants, on both sides. Present on every macOS.
pub const PLATFORM_FONT_FAMILY: &str = "Helvetica";

/// A row: the `RowModel(int Id, string Title)` of the benchmarks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowData {
    pub id: i32,
    pub title: String,
}

impl RowData {
    pub fn new(id: i32, title: String) -> Self {
        Self { id, title }
    }

    /// The row the insert/remove burst inserts at its `i`-th operation (`RowModel(-i, $"Inserted {i}")`).
    pub fn inserted(i: i32) -> Self {
        Self::new(-i, format!("Inserted {i}"))
    }
}

/// The rows of a scenario.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowSet {
    /// `VirtualizationBenchmarks`: `new RowModel(x, $"Item {x}")`.
    Items,
    /// `FirstLayoutBenchmarks`: `new RowModel(x, $"Item {x}{new string('x', x % 20)}")`.
    PaddedItems,
    /// `NeutralSourceBenchmarks`: `new Row(1000 - x)` titled `$"Row {id}"`, so that a sort by
    /// the id reverses them.
    Descending,
}

impl RowSet {
    pub fn rows(self, count: usize) -> Vec<RowData> {
        (0..count as i32)
            .map(|x| match self {
                RowSet::Items => RowData::new(x, format!("Item {x}")),
                RowSet::PaddedItems => RowData::new(x, format!("Item {x}{}", "x".repeat((x % 20) as usize))),
                RowSet::Descending => {
                    let id = count as i32 - x;
                    RowData::new(id, format!("Row {id}"))
                }
            })
            .collect()
    }
}

/// The widths of the six columns of `FirstLayoutBenchmarks` (its `ColumnLayout` without `Mixed`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnLayout {
    /// 124 pixels each.
    Fixed,
    /// The width of the widest realized cell.
    Auto,
    /// One star each: the width of the viewport shared out.
    Star,
}

/// The width of a column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnWidth {
    Pixels(f64),
    Auto,
    Star(f64),
}

/// What a cell of a column shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    /// `x => x.Id`.
    Id,
    /// `x => x.Title`.
    Title,
    /// `x => $"{x.Title}-{column}"`.
    Suffixed(usize),
}

impl CellKind {
    pub fn text(self, row: &RowData) -> String {
        match self {
            CellKind::Id => row.id.to_string(),
            CellKind::Title => row.title.clone(),
            CellKind::Suffixed(column) => format!("{}-{column}", row.title),
        }
    }
}

/// A column: its header, its cells and its width.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnDef {
    pub header: String,
    pub cell: CellKind,
    pub width: ColumnWidth,
}

/// The columns of a scenario.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnSet {
    /// `VirtualizationBenchmarks.CreateGrid`: an id column 80 pixels wide and `columns - 1` text
    /// columns 100 pixels wide.
    Virtualization { columns: usize },
    /// `FirstLayoutBenchmarks.CreateGrid`: six text columns of the layout.
    FirstLayout(ColumnLayout),
    /// The two columns of `NeutralSourceBenchmarks.CreateFlat` (an id and a title), with the widths
    /// of the virtualization grid (80 and 100 pixels), since GPUI's tables have no automatic width.
    IdTitle,
}

impl ColumnSet {
    pub fn columns(self) -> Vec<ColumnDef> {
        match self {
            ColumnSet::Virtualization { columns } => {
                let mut list = vec![ColumnDef { header: "ID".into(), cell: CellKind::Id, width: ColumnWidth::Pixels(80.0) }];
                for column in 1..columns {
                    list.push(ColumnDef {
                        header: format!("Column {column}"),
                        cell: CellKind::Suffixed(column),
                        width: ColumnWidth::Pixels(100.0),
                    });
                }
                list
            }
            ColumnSet::FirstLayout(layout) => (0..6)
                .map(|column| ColumnDef {
                    header: format!("Column {column}"),
                    cell: CellKind::Suffixed(column),
                    width: match layout {
                        ColumnLayout::Fixed => ColumnWidth::Pixels(124.0),
                        ColumnLayout::Auto => ColumnWidth::Auto,
                        ColumnLayout::Star => ColumnWidth::Star(1.0),
                    },
                })
                .collect(),
            ColumnSet::IdTitle => vec![
                ColumnDef { header: "ID".into(), cell: CellKind::Id, width: ColumnWidth::Pixels(80.0) },
                ColumnDef { header: "Title".into(), cell: CellKind::Title, width: ColumnWidth::Pixels(100.0) },
            ],
        }
    }

    pub fn count(self) -> usize {
        match self {
            ColumnSet::Virtualization { columns } => columns,
            ColumnSet::FirstLayout(_) => 6,
            ColumnSet::IdTitle => 2,
        }
    }

    pub fn has_auto(self) -> bool {
        matches!(self, ColumnSet::FirstLayout(ColumnLayout::Auto))
    }

    pub fn has_star(self) -> bool {
        matches!(self, ColumnSet::FirstLayout(ColumnLayout::Star))
    }

    /// The `Layout=` part of the parameters, when the scenario states one.
    pub fn layout_name(self) -> Option<&'static str> {
        match self {
            ColumnSet::FirstLayout(ColumnLayout::Fixed) => Some("Fixed"),
            ColumnSet::FirstLayout(ColumnLayout::Auto) => Some("Auto"),
            ColumnSet::FirstLayout(ColumnLayout::Star) => Some("Star"),
            _ => None,
        }
    }
}

/// The grid a scenario creates: its rows, its columns and the offset it starts at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSpec {
    pub rows: RowSet,
    pub row_count: usize,
    pub columns: ColumnSet,
    /// The offset the grid is scrolled to after its first layout, outside the measurement
    /// (`HorizontalFarColumnScrolls` starts at 90 000 pixels).
    pub initial_offset: (f64, f64),
}

/// Sorts the rows by their id, as the id column's comparison does (a stable sort).
pub fn sort_rows(rows: &mut [RowData], descending: bool) {
    if descending {
        rows.sort_by(|a, b| b.id.cmp(&a.id));
    } else {
        rows.sort_by(|a, b| a.id.cmp(&b.id));
    }
}

/// The text the targets are given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text {
    /// The text systems of the tests of each side: one glyph per character with a fixed advance.
    /// FerroUI's is the test shaper and the test fonts of `TestServices::styled_window` (what the
    /// TreeDataGrid benchmarks use), GPUI's the no-op text system of its test platform.
    Synthetic,
    /// The text of the system fonts: HarfBuzz over the fonts of the Skia font manager on the
    /// FerroUI side, CoreText (GPUI's macOS text system) on the GPUI side, both in
    /// [`PLATFORM_FONT_FAMILY`].
    Platform,
}

impl Text {
    pub fn name(self) -> &'static str {
        match self {
            Text::Synthetic => "Synthetic",
            Text::Platform => "Platform",
        }
    }

    /// The family the targets set, if any: the platform variants name one, the synthetic ones
    /// keep the default of each side.
    pub fn font_family(self) -> Option<&'static str> {
        match self {
            Text::Synthetic => None,
            Text::Platform => Some(PLATFORM_FONT_FAMILY),
        }
    }
}

/// Whether the GPUI targets can be given the text system of the platform: on macOS, where
/// GPUI's text system (`gpui_apple::AppleTextSystem`) runs on any thread.
pub const GPUI_HAS_PLATFORM_TEXT: bool = cfg!(target_os = "macos");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_are_those_of_the_benchmarks() {
        assert_eq!(RowSet::Items.rows(3)[2], RowData::new(2, "Item 2".into()));
        assert_eq!(RowSet::PaddedItems.rows(3)[2], RowData::new(2, "Item 2xx".into()));
        let descending = RowSet::Descending.rows(1000);
        assert_eq!(descending[0], RowData::new(1000, "Row 1000".into()));
        assert_eq!(descending[999], RowData::new(1, "Row 1".into()));
    }

    #[test]
    fn the_columns_are_those_of_the_benchmarks() {
        let columns = ColumnSet::Virtualization { columns: 12 }.columns();
        assert_eq!(columns.len(), 12);
        assert_eq!(columns[0].width, ColumnWidth::Pixels(80.0));
        assert_eq!(columns[11].header, "Column 11");
        assert_eq!(columns[11].cell.text(&RowData::new(7, "Item 7".into())), "Item 7-11");
        assert_eq!(ColumnSet::FirstLayout(ColumnLayout::Fixed).columns()[5].width, ColumnWidth::Pixels(124.0));
    }

    #[test]
    fn a_sort_by_the_id_alternates_the_order() {
        let mut rows = RowSet::Descending.rows(10);
        sort_rows(&mut rows, false);
        assert_eq!(rows[0].id, 1);
        sort_rows(&mut rows, true);
        assert_eq!(rows[0].id, 10);
    }
}
