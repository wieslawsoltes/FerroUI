//! `ColumnLayoutBenchmarks.cs`: committing the actual widths of a column list whose widths did not
//! change since the last commit, with 20, 200 and 1 000 columns (every tenth one auto, the others
//! 80 pixels).
//!
//! The iteration setup builds, measures and commits a new list before every iteration, as the
//! reference's `[IterationSetup]` does; the harness then invokes the body once an iteration.

use ferroui_base::{Rect, Size};
use ferroui_benchmarks::harness::Registry;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_controls_treedatagrid::models::{ColumnList, ColumnRef, TextColumn};
use ferroui_treedatagrid_core::Accessor;
use std::rc::Rc;

const COMMIT_COUNT: u32 = 100_000;

/// `RowModel(string Value)`: a record, compared by value. No model is created: the columns are
/// laid out without rows.
#[allow(dead_code)]
#[derive(PartialEq)]
struct RowModel {
    value: String,
}

pub struct ColumnLayoutBenchmarks {
    column_count: i32,
    /// Set by the iteration setup.
    columns: Option<Rc<ColumnList>>,
}

impl ColumnLayoutBenchmarks {
    pub fn new(column_count: i32) -> Self {
        Self { column_count, columns: None }
    }

    /// `[IterationSetup] Setup`.
    pub fn setup(&mut self) {
        let columns = ColumnList::new();

        for i in 0..self.column_count {
            let header: ferroui_base::BoxedValue = Rc::new(format!("Column {i}"));
            let width = if i % 10 == 0 { GridLength::AUTO } else { GridLength::new(80.0, GridUnitType::Pixel) };
            let column = TextColumn::<RowModel, String>::new(
                Some(header),
                Accessor::member("Value", |x: &Rc<RowModel>| x.value.clone()),
                Some(width),
                None,
            );
            columns.add(column as ColumnRef);
        }

        columns.viewport_changed(Rect::new(0.0, 0.0, 800.0, 500.0));

        for i in 0..self.column_count {
            columns.cell_measured(i, 0, Size::new(f64::from(80 + (i % 5)), 24.0));
        }

        columns.commit_actual_widths();
        self.columns = Some(columns);
    }

    /// `[Benchmark(OperationsPerInvoke = CommitCount)] CommitUnchangedWidths`.
    pub fn commit_unchanged_widths(&self) {
        let columns = self.columns.as_ref().expect("the iteration setup ran");

        for _ in 0..COMMIT_COUNT {
            columns.commit_actual_widths();
        }
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("treedatagrid", "ColumnLayoutBenchmarks");
    for column_count in [20, 200, 1_000] {
        class
            .benchmark_with_iteration_setup(
                "commit_unchanged_widths",
                format!("ColumnCount={column_count}"),
                move || ColumnLayoutBenchmarks::new(column_count),
                |b| b.setup(),
                |b| b.commit_unchanged_widths(),
            )
            .operations_per_invoke(COMMIT_COUNT);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn column_layout_benchmarks() {
        ferroui_benchmarks::harness::smoke_class(super::register, "ColumnLayoutBenchmarks");
    }
}
