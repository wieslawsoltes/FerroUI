//! The view contracts of `Models/TreeDataGrid/`: `ICell.cs`, `ICellOptions.cs` (with
//! `BeginEditGestures`), `IColumn.cs`, `IColumn\`1.cs`, `IColumns.cs`, `IUpdateColumnLayout.cs`,
//! `IColumnLayoutBatch.cs`, `IColumnMeasurementOptions.cs`, `ITreeDataGridRows.cs`, `ITextCell.cs`,
//! `ITextCellOptions.cs`, `ITemplateCellOptions.cs`, `IExpanderCell.cs`,
//! `IExpanderCellPresentation.cs`, `IRecyclableCell.cs` and `ISearchableTextColumn.cs`.
//!
//! The view columns are the columns the control lays out and presents; their rows are the rows of
//! the core (`ferroui_treedatagrid_core::models::IRow`), which the reference's view interfaces
//! extend only to convert `GridLength` and `IndexPath` between two copies of them.

use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::media::{TextAlignment, TextTrimming, TextWrapping};
use ferroui_base::{BoxedValue, Rect, Size};
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::models::{IExpander, ListSortDirection, RowRef, RowsChangedHandler};
use ferroui_treedatagrid_core::IndexPath;
use std::any::Any;
use std::rc::Rc;

/// The gestures that start editing a cell (`[Flags] BeginEditGestures`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BeginEditGestures(pub u32);

impl BeginEditGestures {
    pub const NONE: BeginEditGestures = BeginEditGestures(0x00);
    pub const F2: BeginEditGestures = BeginEditGestures(0x01);
    pub const TAP: BeginEditGestures = BeginEditGestures(0x02);
    pub const DOUBLE_TAP: BeginEditGestures = BeginEditGestures(0x04);
    pub const WHEN_SELECTED: BeginEditGestures = BeginEditGestures(0x1000);
    pub const DEFAULT: BeginEditGestures = BeginEditGestures(0x01 | 0x04);

    pub fn has_flag(self, flag: BeginEditGestures) -> bool {
        self.0 & flag.0 == flag.0
    }

    /// The gestures parsed from their markup form: flag names separated by commas (`"F2, Tap"`),
    /// or a number.
    pub fn parse(text: &str) -> Option<BeginEditGestures> {
        let mut value = 0u32;
        for part in text.split(',') {
            let part = part.trim();
            value |= match part {
                p if p.eq_ignore_ascii_case("None") => Self::NONE.0,
                p if p.eq_ignore_ascii_case("F2") => Self::F2.0,
                p if p.eq_ignore_ascii_case("Tap") => Self::TAP.0,
                p if p.eq_ignore_ascii_case("DoubleTap") => Self::DOUBLE_TAP.0,
                p if p.eq_ignore_ascii_case("WhenSelected") => Self::WHEN_SELECTED.0,
                p if p.eq_ignore_ascii_case("Default") => Self::DEFAULT.0,
                p => p.parse::<u32>().ok()?,
            };
        }
        Some(BeginEditGestures(value))
    }
}

impl Default for BeginEditGestures {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::ops::BitOr for BeginEditGestures {
    type Output = BeginEditGestures;

    fn bitor(self, rhs: BeginEditGestures) -> BeginEditGestures {
        BeginEditGestures(self.0 | rhs.0)
    }
}

/// The options of a cell (`ICellOptions`).
pub trait ICellOptions {
    fn begin_edit_gestures(&self) -> BeginEditGestures;
}

/// The options of a text cell (`ITextCellOptions`).
pub trait ITextCellOptions: ICellOptions {
    fn string_format(&self) -> String;
    fn text_trimming(&self) -> Rc<dyn TextTrimming>;
    fn text_wrapping(&self) -> TextWrapping;
    fn text_alignment(&self) -> TextAlignment;
}

/// The options of a template cell (`ITemplateCellOptions`).
pub trait ITemplateCellOptions: ICellOptions {}

/// A cell model (`ICell`).
pub trait ICell: Any {
    fn can_edit(&self) -> bool;
    fn edit_gestures(&self) -> BeginEditGestures;
    /// The value of the cell.
    fn value(&self) -> Option<BoxedValue>;
    fn as_any(&self) -> &dyn Any;

    /// The cell as a text cell (`is ITextCell`).
    fn as_text_cell(&self) -> Option<&dyn ITextCell> {
        None
    }
    /// The cell as an expander cell (`is IExpanderCellPresentation`).
    fn as_expander_cell(&self) -> Option<&dyn IExpanderCellPresentation> {
        None
    }
    /// The cell's property change notification, if it has one.
    fn as_notify_property_changed(&self) -> Option<&dyn INotifyPropertyChanged> {
        None
    }
    /// The cell as a cell that can be pooled (`is IRecyclableCell`).
    fn as_recyclable(&self) -> Option<&dyn IRecyclableCell> {
        None
    }
    /// The cell as an object whose edits are committed or cancelled (`is IEditableObject`).
    fn as_editable(&self) -> Option<&dyn super::IEditableObject> {
        None
    }
    /// Ends the cell's subscriptions (`IDisposable.Dispose`).
    fn dispose(&self) {}
}

/// A shared cell.
pub type CellRef = Rc<dyn ICell>;

/// A text cell (`ITextCell`).
pub trait ITextCell: ICell {
    fn text(&self) -> Option<String>;
    fn set_text(&self, value: Option<String>);
    fn text_trimming(&self) -> Rc<dyn TextTrimming>;
    fn text_wrapping(&self) -> TextWrapping;
    fn text_alignment(&self) -> TextAlignment;
}

/// The presentation of an expander cell (`IExpanderCellPresentation`, `IExpanderCell`).
pub trait IExpanderCellPresentation: ICell + IExpander {
    /// The cell the expander shows.
    fn content(&self) -> Option<CellRef>;
    /// The row of the cell.
    fn row(&self) -> RowRef;
}

/// A cell that can be suspended and pooled (`IRecyclableCell`): it releases its row and its
/// subscriptions without losing what a later retarget needs.
pub trait IRecyclableCell {
    fn try_suspend(&self) -> bool;
}

/// A view column (`IColumn` of the control): what the column headers and the layout read.
pub trait IColumn: INotifyPropertyChanged + Any {
    /// The width the column is laid out at.
    fn actual_width(&self) -> f64;
    fn can_user_resize(&self) -> Option<bool>;
    fn header(&self) -> Option<BoxedValue>;
    fn width(&self) -> GridLength;
    fn sort_direction(&self) -> Option<ListSortDirection>;
    fn set_sort_direction(&self, value: Option<ListSortDirection>);
    fn tag(&self) -> Option<BoxedValue>;
    fn set_tag(&self, value: Option<BoxedValue>);
    fn as_any(&self) -> &dyn Any;

    /// The column's layout (`is IUpdateColumnLayout`).
    fn as_update_column_layout(&self) -> Option<&dyn IUpdateColumnLayout> {
        None
    }
    /// The column as a column that creates cells (`IColumn<TModel>`, erased).
    fn as_cell_column(&self) -> Option<&dyn ICellColumn> {
        None
    }
    /// Whether the column must be measured with an unconstrained width (`IColumnMeasurementOptions`).
    fn requires_unconstrained_width_measurement(&self) -> bool {
        false
    }
    /// The core column the view column presents, if any.
    fn core_column(&self) -> Option<Rc<dyn ferroui_treedatagrid_core::models::IColumn>> {
        None
    }
}

/// A shared view column.
pub type ColumnRef = Rc<dyn IColumn>;

/// A view column that creates the cells of rows (`IColumn<TModel>.CreateCell`), with the row and
/// its model untyped.
pub trait ICellColumn {
    /// The cell of `row`.
    fn create_cell(&self, row: &RowRef) -> CellRef;
    /// The cell of `row`, reusing `cell` when the column can retarget it; `None` when it cannot
    /// (`ICellColumn.TryReuseCell`).
    fn try_reuse_cell(&self, _cell: &CellRef, _row: &RowRef) -> bool {
        false
    }
    /// For a column that searches its text (`ITextSearchableColumn`): the text of `model`.
    fn select_value(&self, _model: &BoxedValue) -> Option<String> {
        None
    }
    /// Whether the column takes part in type-ahead search.
    fn is_text_search_enabled(&self) -> bool {
        false
    }
}

/// The layout of a column (`IUpdateColumnLayout`).
pub trait IUpdateColumnLayout: IColumn {
    fn min_actual_width(&self) -> f64;
    fn max_actual_width(&self) -> f64;
    fn star_width_was_constrained(&self) -> bool;
    /// A cell of the column at `row_index` was measured at `width`; the width the cell gets.
    fn cell_measured(&self, width: f64, row_index: i32) -> f64;
    fn calculate_star_width(&self, available_width: f64, total_stars: f64);
    /// Commits the width laid out; true when it changed.
    fn commit_actual_width(&self) -> bool;
    fn set_width(&self, width: GridLength);
}

/// The columns of a presentation (`IColumns`).
pub trait IColumns {
    fn count(&self) -> usize;
    fn get(&self, index: usize) -> ColumnRef;
    /// The size a cell gets after it was measured at `size` (`CellMeasured`).
    fn cell_measured(&self, column_index: i32, row_index: i32, size: Size) -> Size;
    /// The index and left edge of the column at `x` (`GetColumnAt`).
    fn get_column_at(&self, x: f64) -> (i32, f64);
    fn get_estimated_width(&self, constraint: f64) -> f64;
    fn commit_actual_widths(&self);
    fn set_column_width(&self, column_index: i32, width: GridLength);
    fn viewport_changed(&self, viewport: Rect);
    /// `LayoutInvalidated`.
    fn add_layout_invalidated(&self, handler: Rc<dyn Fn()>) -> u64;
    fn remove_layout_invalidated(&self, token: u64);
    /// `CollectionChanged` (the columns as untyped view columns).
    fn add_collection_changed(&self, handler: Rc<ColumnsChangedHandler>) -> u64;
    fn remove_collection_changed(&self, token: u64);
    /// The column layout batch of the columns, if they batch width commits (`IColumnLayoutBatch`).
    fn as_layout_batch(&self) -> Option<&dyn IColumnLayoutBatch> {
        None
    }
    /// The viewport estimation of the columns, if they estimate the widths of the columns not
    /// realized (`IColumnViewportEstimator`).
    fn as_viewport_estimator(&self) -> Option<&dyn super::IColumnViewportEstimator> {
        None
    }
    fn as_any(&self) -> &dyn Any;
}

/// A change of the columns of a presentation.
pub type ColumnsChangedArgs<'a> = ferroui_base::collections::NotifyCollectionChangedEventArgs<'a, ColumnRef>;

/// A handler of [`IColumns::add_collection_changed`].
pub type ColumnsChangedHandler = dyn for<'a> Fn(&ColumnsChangedArgs<'a>);

/// The batching of the width commits of the columns during a layout (`IColumnLayoutBatch`).
pub trait IColumnLayoutBatch {
    fn is_actual_width_commit_deferred(&self) -> bool;
    fn begin_actual_width_batch(&self);
    fn end_actual_width_batch(&self) -> bool;
    fn request_final_measure(&self);
}

/// The rows of a presentation, which realize the cells (`ITreeDataGridRows`).
pub trait ITreeDataGridRows {
    fn count(&self) -> usize;
    fn get(&self, index: usize) -> RowRef;
    /// The index and top edge of the row at `y` (`GetRowAt`).
    fn get_row_at(&self, y: f64) -> (i32, f64);
    fn model_index_to_row_index(&self, model_index: &IndexPath) -> i32;
    fn row_index_to_model_index(&self, row_index: i32) -> IndexPath;
    /// The cell of `column` (at `column_index`) in the row at `row_index`.
    fn realize_cell(&self, column: &ColumnRef, column_index: i32, row_index: i32) -> CellRef;
    fn unrealize_cell(&self, cell: &CellRef, column_index: i32, row_index: i32);
    /// `CollectionChanged`.
    fn add_collection_changed(&self, handler: Rc<RowsChangedHandler>) -> u64;
    fn remove_collection_changed(&self, token: u64);
    /// Returns a cell to the pool of its column (`IRecyclingCellRows.TryRecycleCell`).
    fn try_recycle_cell(&self, _column: &ColumnRef, _cell: &CellRef) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any;
}
