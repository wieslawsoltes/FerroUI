use crate::{Controls, TableViewColumn};
use ferroui_base::collections::FerroList;
use ferroui_base::layout::LayoutHelper;
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::{Rect, Ref, Size};

/// The layout shared by the presenter of the column headers and the
/// presenters of the cells of a table view.
pub(crate) struct TableViewLayoutHelper;

impl TableViewLayoutHelper {
    /// Distributes `available_width` among the columns. Pixel columns take
    /// their fixed size; remaining space is split proportionally among star
    /// columns. Auto is treated as 1*.
    pub(crate) fn update_actual_widths(
        columns: &FerroList<Ref<TableViewColumn>>,
        available_width: f64,
        use_layout_rounding: bool,
        layout_scale: f64,
    ) -> bool {
        let columns = columns.snapshot();

        if columns.is_empty() {
            return false;
        }

        let mut fixed_total = 0.0;
        let mut star_total = 0.0;
        let mut modified = false;

        for column in columns.iter() {
            if !column.is_visible() {
                if column.actual_width() != 0.0 {
                    column.set_actual_width(0.0);
                    modified = true;
                }
                continue;
            }

            let width = column.width();
            if width.is_absolute() {
                let mut actual_width = width.value();

                if use_layout_rounding {
                    actual_width = LayoutHelper::round_layout_value(actual_width, layout_scale);
                }

                if column.actual_width() != actual_width {
                    column.set_actual_width(actual_width);
                    modified = true;
                }

                fixed_total += actual_width;
            } else {
                // Star or Auto: treat both as star
                star_total += if width.is_star() { width.value() } else { 1.0 };
            }
        }

        let star_budget = if available_width == f64::INFINITY {
            // The headers aren't supposed to be measured with infinity, as
            // they're normally outside the main scroll viewer. If they've
            // been relocated, or are missing, use 1000 pixels arbitrarily so
            // star columns are still displayed.
            1000.0
        } else {
            max(0.0, available_width - fixed_total)
        };

        // Distribute the star budget by rounding the cumulative right edge of
        // each star column rather than each width in isolation. This spreads
        // the sub-pixel remainders across the columns instead of letting them
        // accumulate, so the sum of the rounded star widths equals the
        // rounded star budget and the final layout matches the available
        // width.
        let mut unrounded_edge = 0.0;
        let mut rounded_edge = 0.0;

        for column in columns.iter() {
            if !column.is_visible() {
                continue;
            }

            let width = column.width();
            if !width.is_absolute() {
                let share = if width.is_star() { width.value() } else { 1.0 };
                unrounded_edge += if star_total > 0.0 { share / star_total * star_budget } else { 0.0 };

                let mut actual_width = unrounded_edge - rounded_edge;

                if use_layout_rounding {
                    let next_rounded_edge = LayoutHelper::round_layout_value(unrounded_edge, layout_scale);
                    actual_width = next_rounded_edge - rounded_edge;
                    rounded_edge = next_rounded_edge;
                } else {
                    rounded_edge = unrounded_edge;
                }

                if column.actual_width() != actual_width {
                    column.set_actual_width(actual_width);
                    modified = true;
                }
            }
        }

        modified
    }

    /// Whether the actual widths of the columns have to be computed.
    ///
    /// Contract between [`update_actual_widths`](Self::update_actual_widths),
    /// this function and [`reset_actual_widths`](Self::reset_actual_widths):
    /// if the actual width of a column is NaN, a recalculation of the actual
    /// widths is needed. All column widths are reset and recalculated
    /// together, so checking the first one is sufficient.
    pub(crate) fn needs_actual_widths(columns: &FerroList<Ref<TableViewColumn>>) -> bool {
        columns.try_get(0).is_some_and(|column| column.actual_width().is_nan())
    }

    pub(crate) fn reset_actual_widths(columns: &FerroList<Ref<TableViewColumn>>) {
        for column in columns.snapshot().iter() {
            column.set_actual_width(f64::NAN);
        }
    }

    pub(crate) fn measure_row(
        columns: &FerroList<Ref<TableViewColumn>>,
        cells: &Controls,
        available_size: Size,
    ) -> Size {
        let columns = columns.snapshot();
        let cells = cells.snapshot();

        if cells.len() != columns.len() {
            return Size::default();
        }

        let mut total_width = 0.0;
        let mut total_height = 0.0;

        for (column, child) in columns.iter().zip(cells.iter()) {
            if !column.is_visible() {
                continue;
            }

            let column_width = column.actual_width();
            child.measure(Size::new(column_width, available_size.height));
            total_width += column_width;
            total_height = max(total_height, child.desired_size().height);
        }

        Size::new(total_width, total_height)
    }

    pub(crate) fn arrange_row(
        columns: &FerroList<Ref<TableViewColumn>>,
        cells: &Controls,
        final_size: Size,
        offset: f64,
    ) -> Size {
        let columns = columns.snapshot();
        let cells = cells.snapshot();

        if cells.len() != columns.len() {
            return final_size;
        }

        let mut x = offset;
        for (column, cell) in columns.iter().zip(cells.iter()) {
            if !column.is_visible() {
                continue;
            }

            let width = column.actual_width();
            cell.arrange(Rect::new(x, 0.0, width, final_size.height));
            x += width;
        }

        final_size
    }
}
