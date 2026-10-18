//! `ColumnGeometryBenchmarks.cs`: finding the column at a position of the viewport in a column
//! list of 20, 200 and 1 000 pixel columns, from the cached geometry of the list.
//!
//! The reference runs five warm-up and ten measured iterations; the harness takes its counts from
//! the environment (`FERROUI_BENCH_WARMUP`, `FERROUI_BENCH_ITERATIONS`).

use ferroui_benchmarks::harness::Registry;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_controls_treedatagrid::models::{ColumnList, ColumnRef, TextColumn};
use ferroui_treedatagrid_core::Accessor;
use std::rc::Rc;

const LOOKUPS: u32 = 1024;

/// `RowModel(string Value)`: a record, compared by value. No model is created: the columns are
/// laid out without rows.
#[allow(dead_code)]
#[derive(PartialEq)]
struct RowModel {
    value: String,
}

pub struct ColumnGeometryBenchmarks {
    columns: Rc<ColumnList>,
    column_count: i32,
}

impl ColumnGeometryBenchmarks {
    /// `[GlobalSetup] Setup`: `column_count` text columns 100 pixels wide.
    pub fn new(column_count: i32) -> Self {
        let columns = ColumnList::new();
        for _ in 0..column_count {
            let column = TextColumn::<RowModel, String>::new(
                None,
                Accessor::member("Value", |x: &Rc<RowModel>| x.value.clone()),
                Some(GridLength::new(100.0, GridUnitType::Pixel)),
                None,
            );
            columns.add(column as ColumnRef);
        }
        Self { columns, column_count }
    }

    /// `[Benchmark(OperationsPerInvoke = Lookups)] LocateViewport`.
    pub fn locate_viewport(&self) -> i32 {
        let mut result = 0;
        for i in 0..LOOKUPS as i32 {
            result += self.columns.get_column_at(f64::from((i % self.column_count) * 100 + 50)).0;
        }
        result
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("treedatagrid", "ColumnGeometryBenchmarks");
    for column_count in [20, 200, 1000] {
        class
            .benchmark(
                "locate_viewport",
                format!("ColumnCount={column_count}"),
                move || ColumnGeometryBenchmarks::new(column_count),
                |b| b.locate_viewport(),
            )
            .operations_per_invoke(LOOKUPS);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn column_geometry_benchmarks() {
        ferroui_benchmarks::harness::smoke_class(super::register, "ColumnGeometryBenchmarks");
    }
}
