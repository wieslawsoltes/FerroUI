//! `Primitives/TreeDataGridColumnarPresenterBase.cs`: the layout shared by the presenters that show
//! their items in virtualized columns, the cells presenter and the column headers presenter.
//!
//! The reference's abstract class overrides members of the presenter base; here it is the state
//! the two presenters embed, and the overrides are its methods, which the presenters' implementations
//! of [`TreeDataGridPresenterHooks`](super::TreeDataGridPresenterHooks) call.
//!
//! The reference asks the columns for the viewport estimate when they implement
//! `IColumnViewportEstimator`: the column list of the crate does, and is found through
//! [`as_column_viewport_estimator`].

use super::{as_natural_width_measure_cache, TreeDataGridPresenterBase};
use crate::models::{ColumnListBase, IColumnViewportEstimator, IColumns};
use ferroui_base::layout::Orientation;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{Rect, Ref, Size};
use ferroui_controls::{Control, GridUnitType};
use std::cell::Cell;

/// The state of the reference's `TreeDataGridColumnarPresenterBase<TItem>`.
pub struct TreeDataGridColumnarPresenterBase {
    last_estimated_element_size_u: Cell<f64>,
}

impl Default for TreeDataGridColumnarPresenterBase {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeDataGridColumnarPresenterBase {
    pub fn new() -> Self {
        Self { last_estimated_element_size_u: Cell::new(25.0) }
    }

    /// `GetInitialConstraint`: the actual width of the column when it is known and the column does
    /// not need an unconstrained measure, otherwise its maximum width.
    pub fn get_initial_constraint(columns: &dyn IColumns, index: i32, available_size: Size) -> Size {
        let column = columns.get(index as usize);
        let layout = column
            .as_update_column_layout()
            .expect("A column of a columnar presenter must update its layout.");
        let width = if !column.requires_unconstrained_width_measurement() && column.actual_width().is_finite() {
            column.actual_width()
        } else {
            layout.max_actual_width()
        };
        Size::new(available_size.width.min(width), available_size.height)
    }

    /// `GetOrEstimateAnchorElementForViewport`.
    pub fn get_or_estimate_anchor_element_for_viewport(
        &self,
        base: &TreeDataGridPresenterBase,
        columns: Option<&dyn IColumns>,
        viewport_start: f64,
        viewport_end: f64,
        item_count: i32,
    ) -> (i32, f64) {
        if let Some(columns) = columns {
            let (index, position) = columns.get_column_at(viewport_start);
            if index >= 0 {
                return (index, position);
            }

            if let Some(estimator) = as_column_viewport_estimator(columns) {
                let mut estimated_element_size_u = self.last_estimated_element_size_u.get();
                let (index, position) = estimator.get_or_estimate_column_at(
                    viewport_start,
                    viewport_end,
                    item_count,
                    base.start_u(),
                    base.first_index(),
                    &mut estimated_element_size_u,
                );
                self.last_estimated_element_size_u.set(estimated_element_size_u);
                if index >= 0 {
                    return (index, position);
                }
            }
        }

        base.default_get_or_estimate_anchor_element_for_viewport(viewport_start, viewport_end, item_count)
    }

    /// `EstimateElementSizeU`: the average width of the measured columns.
    pub fn estimate_element_size_u(&self, columns: Option<&dyn IColumns>) -> f64 {
        let Some(estimator) = columns.and_then(as_column_viewport_estimator) else {
            return self.last_estimated_element_size_u.get();
        };

        let result = estimator.estimate_element_size();
        if result >= 0.0 {
            self.last_estimated_element_size_u.set(result);
        }

        self.last_estimated_element_size_u.get()
    }

    /// `NeedsMeasureForViewportChange`.
    pub fn needs_measure_for_viewport_change(
        base: &TreeDataGridPresenterBase,
        orientation: Orientation,
        measure_viewport: Rect,
        viewport: Rect,
    ) -> bool {
        !MathUtilities::are_close(measure_viewport.height, viewport.height)
            || !base.is_viewport_covered_by_realized_elements(orientation, viewport)
    }

    /// `NeedsFinalMeasurePass`.
    pub fn needs_final_measure_pass(columns: &dyn IColumns, first_index: i32, elements: &[Option<Ref<Control>>]) -> bool {
        columns.commit_actual_widths();

        // A rows presenter can defer the shared column-width commit until all realized rows have
        // contributed their natural sizes. Measuring against an earlier row's width here would only
        // make the row invalid again when the batch commits its final maximum.
        if let Some(batch) = columns.as_layout_batch().filter(|batch| batch.is_actual_width_commit_deferred()) {
            for (i, element) in elements.iter().enumerate() {
                if let Some(element) = element {
                    if Self::needs_final_measure(columns, element, i as i32 + first_index) {
                        batch.request_final_measure();
                        break;
                    }
                }
            }

            return false;
        }

        // We need to do a second measure pass if any of the controls were measured with a width
        // that is greater than the final column width.
        elements
            .iter()
            .enumerate()
            .any(|(i, e)| e.as_ref().is_some_and(|e| Self::needs_final_measure(columns, e, i as i32 + first_index)))
    }

    /// `IFinalMeasureSelector.NeedsFinalMeasure`: whether the element was measured wider than its
    /// column, or at another width than its auto-sized column.
    pub fn needs_final_measure(columns: &dyn IColumns, element: &Control, index: i32) -> bool {
        let column = columns.get(index as usize);
        let previous = element.previous_measure().expect("the element was measured");

        previous.width > column.actual_width()
            || (column.width().grid_unit_type() == GridUnitType::Auto
                && !MathUtilities::are_close(previous.width, column.actual_width()))
    }

    /// `GetElementAt`.
    pub fn get_element_at(columns: &dyn IColumns, position: f64) -> (i32, f64) {
        columns.get_column_at(position)
    }

    /// `GetFinalConstraint`: the actual width of the column, with an unconstrained height.
    pub fn get_final_constraint(columns: &dyn IColumns, index: i32) -> Size {
        let column = columns.get(index as usize);
        Size::new(column.actual_width(), f64::INFINITY)
    }

    /// `MeasureColumnElement`: measures the element of a column and reports the measure to the
    /// columns, which give the size the element gets.
    pub fn measure_column_element(
        columns: &dyn IColumns,
        index: i32,
        row_index: i32,
        element: &Control,
        available_size: Size,
    ) -> Size {
        let previous_constraint = element.previous_measure();

        if element.is_measure_valid() {
            if available_size.width == f64::INFINITY {
                if let Some(natural_size) =
                    as_natural_width_measure_cache(element).and_then(|cache| cache.natural_desired_size())
                {
                    return columns.cell_measured(index, row_index, natural_size);
                }
            }

            if previous_constraint == Some(available_size) {
                return columns.cell_measured(index, row_index, element.desired_size());
            }
        }

        element.measure(available_size);

        if available_size.width == f64::INFINITY {
            if let Some(cache) = as_natural_width_measure_cache(element) {
                cache.set_natural_desired_size(Some(element.desired_size()));
            }
        }

        columns.cell_measured(index, row_index, element.desired_size())
    }

    /// `CalculateSizeU`: the estimated width of the columns.
    pub fn calculate_size_u(columns: Option<&dyn IColumns>, available_size: Size) -> f64 {
        columns.map_or(0.0, |columns| columns.get_estimated_width(available_size.width))
    }
}

/// The columns as a viewport estimator (`Columns is IColumnViewportEstimator`): the column list of
/// the crate.
pub fn as_column_viewport_estimator(columns: &dyn IColumns) -> Option<&dyn IColumnViewportEstimator> {
    columns.as_any().downcast_ref::<ColumnListBase>().map(|columns| columns as &dyn IColumnViewportEstimator)
}
