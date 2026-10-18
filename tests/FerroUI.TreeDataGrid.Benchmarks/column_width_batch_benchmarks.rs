//! `ColumnWidthBatchBenchmarks.cs`: measuring 20 and 40 rows of six columns and committing the
//! widths after every row (the baseline), against the same commits inside an actual width batch
//! (`IColumnLayoutBatch`) that defers them to the end of the batch.
//!
//! The reference fixes 8 192 invocations an iteration, ten warm-up and twenty measured
//! iterations; the harness finds the invocations an iteration with its pilot and takes the
//! iteration counts from the environment.

use ferroui_base::{Rect, Size};
use ferroui_benchmarks::harness::Registry;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_controls_treedatagrid::models::{ColumnList, ColumnRef, IColumnLayoutBatch, TextColumn};
use ferroui_treedatagrid_core::Accessor;
use std::cell::RefCell;
use std::rc::Rc;

/// `ColumnLayout`: the widths of the six columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnLayout {
    Auto,
    Mixed,
}

/// `Model`: a class with a settable `Value`, compared by reference. No model is created: the
/// columns are measured without rows.
#[allow(dead_code)]
struct Model {
    value: RefCell<String>,
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub struct ColumnWidthBatchBenchmarks {
    layout: ColumnLayout,
    row_count: i32,
}

impl ColumnWidthBatchBenchmarks {
    pub fn new(layout: ColumnLayout, row_count: i32) -> Self {
        Self { layout, row_count }
    }

    /// `[Benchmark(Baseline = true)] IncrementalWidthCommits`.
    pub fn incremental_width_commits(&self) -> i32 {
        let columns = self.create_columns();

        for row in 0..self.row_count {
            Self::measure_row(&columns, row);
            columns.commit_actual_widths();
        }

        Self::checksum(&columns)
    }

    /// `[Benchmark] BatchedWidthCommit`.
    pub fn batched_width_commit(&self) -> i32 {
        let columns = self.create_columns();
        let batch: &dyn IColumnLayoutBatch = &*columns;
        batch.begin_actual_width_batch();

        for row in 0..self.row_count {
            Self::measure_row(&columns, row);
            columns.commit_actual_widths();
            batch.request_final_measure();
        }

        let _ = batch.end_actual_width_batch();
        Self::checksum(&columns)
    }

    fn create_columns(&self) -> Rc<ColumnList> {
        let result = ColumnList::new();

        for column in 0..6 {
            let width = match self.layout {
                ColumnLayout::Auto => GridLength::AUTO,
                ColumnLayout::Mixed => match column % 3 {
                    0 => GridLength::AUTO,
                    1 => GridLength::new(1.0, GridUnitType::Star),
                    _ => GridLength::new(124.0, GridUnitType::Pixel),
                },
            };
            let column = TextColumn::<Model, String>::new(
                None,
                Accessor::member("Value", |x: &Rc<Model>| x.value.borrow().clone()),
                Some(width),
                None,
            );
            result.add(column as ColumnRef);
        }

        result.viewport_changed(Rect::new(0.0, 0.0, 800.0, 500.0));
        result
    }

    fn measure_row(columns: &ColumnList, row: i32) {
        for column in 0..columns.count() as i32 {
            columns.cell_measured(column, row, Size::new(f64::from(40 + row + column), 24.0));
        }
    }

    fn checksum(columns: &ColumnList) -> i32 {
        let mut result = 0;

        for column in 0..columns.count() {
            result += columns.get(column).actual_width() as i32;
        }

        result
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("treedatagrid", "ColumnWidthBatchBenchmarks");
    for layout in [ColumnLayout::Auto, ColumnLayout::Mixed] {
        for row_count in [20, 40] {
            let parameters = format!("Layout={layout:?}, RowCount={row_count}");
            class
                .benchmark(
                    "incremental_width_commits",
                    parameters.clone(),
                    move || ColumnWidthBatchBenchmarks::new(layout, row_count),
                    |b| b.incremental_width_commits(),
                )
                .baseline();
            class.benchmark(
                "batched_width_commit",
                parameters,
                move || ColumnWidthBatchBenchmarks::new(layout, row_count),
                |b| b.batched_width_commit(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn column_width_batch_benchmarks() {
        ferroui_benchmarks::harness::smoke_class(super::register, "ColumnWidthBatchBenchmarks");
    }
}
