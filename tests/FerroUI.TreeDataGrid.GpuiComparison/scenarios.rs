//! The scenarios: what one operation does, written once against [`GridTarget`] and
//! [`TreeTarget`], so that every target performs the same sequence of offsets and edits as the
//! TreeDataGrid benchmark it mirrors (`virtualization_benchmarks.rs`,
//! `first_layout_benchmarks.rs`, `neutral_source_benchmarks.rs`).
//!
//! A frame of a target is what its side does to bring the window up to date after a change:
//! a layout pass of the window for FerroUI (and, for the render target, a recording of the
//! window's drawing), a full window frame (layout, prepaint and paint into the scene) for GPUI.

use crate::model::{GridSpec, RowData, RowSet};

/// `VirtualizationBenchmarks.VerticalScrollOperations`.
pub const VERTICAL_SCROLL_OPERATIONS: u32 = 10_000;
/// `VirtualizationBenchmarks.StationaryLayoutOperations`.
pub const STATIONARY_LAYOUT_OPERATIONS: u32 = 25_000;
/// `VirtualizationBenchmarks.HorizontalScrollOperations`.
pub const HORIZONTAL_SCROLL_OPERATIONS: u32 = 6_000;
/// `VirtualizationBenchmarks.HorizontalColumnOperations`.
pub const HORIZONTAL_COLUMN_OPERATIONS: u32 = 1_000;
/// `VirtualizationBenchmarks.CollectionEditOperations`.
pub const COLLECTION_EDIT_OPERATIONS: u32 = 1_500;
/// `VirtualizationBenchmarks.CollectionMoveOperations`.
pub const COLLECTION_MOVE_OPERATIONS: u32 = 6_000;
/// `VirtualizationBenchmarks.DetachReattachOperations`.
pub const DETACH_REATTACH_OPERATIONS: u32 = 200;

/// A table of one side: a grid of rows and columns in a window of the viewport.
pub trait GridTarget {
    /// Creates the grid of `spec` in a window of the viewport and brings it to its first frame.
    fn create(&mut self, spec: &GridSpec);

    /// Closes the window of the grid, if there is one, and drops the grid.
    fn destroy(&mut self);

    /// Scrolls the rows to `x`, `y` (pixels from the top left of the content), without a frame.
    fn set_offset(&mut self, x: f64, y: f64);

    /// What `VerticalRowScrolls` and `ForcedStationaryVerticalLayouts` do before each layout:
    /// invalidate the measure of the rows presenter. A GPUI frame lays out every row anyway.
    fn invalidate_rows(&mut self) {}

    /// Brings the window up to date: one frame.
    fn frame(&mut self);

    fn insert_row(&mut self, index: usize, row: RowData);

    fn remove_row(&mut self, index: usize);

    fn move_row(&mut self, from: usize, to: usize);

    /// Takes the grid out of its window (the window stays open).
    fn detach(&mut self);

    /// Puts the grid back into its window.
    fn attach(&mut self);

    /// Sorts the rows by the id, in the direction opposite to that of the last sort (the first
    /// sort is descending, as `SortThousandRows` flips its direction before sorting).
    fn sort_next(&mut self);

    /// A value read from the grid after an operation, which keeps the work from the optimiser
    /// (the realized rows on the FerroUI side).
    fn checksum(&mut self) -> usize;
}

/// A tree of one side: a root with a thousand children in a window of the viewport.
pub trait TreeTarget {
    /// Expands the root, brings the window up to date when `frames` is set, collapses the root
    /// and brings the window up to date again.
    fn expand_collapse(&mut self, frames: bool) -> usize;
}

/// How a scenario uses its grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifetime {
    /// The body creates and destroys the grid itself (`CreateAndLayoutGrid`).
    PerInvocation,
    /// The iteration setup creates a new grid, and the body runs once an iteration (the
    /// benchmarks with an `[IterationSetup]`).
    PerIteration,
    /// One grid for the whole benchmark, created by its setup; the harness runs the body as many
    /// times as fills an iteration (`SortThousandRows`).
    Shared,
}

/// A scenario of the grid targets.
#[derive(Clone, Copy)]
pub struct GridScenario {
    /// The benchmark name, the snake case of the method of the TreeDataGrid benchmark it mirrors
    /// (or a name of its own for a scenario that the benchmarks do not have).
    pub name: &'static str,
    pub spec: GridSpec,
    pub operations: u32,
    pub lifetime: Lifetime,
    pub body: fn(&mut dyn GridTarget, &GridSpec, u32) -> usize,
}

impl GridScenario {
    /// `Rows=.., Columns=..[, Layout=..], Text=..`.
    pub fn parameters(&self, text: crate::model::Text) -> String {
        let mut parameters = format!("Rows={}, Columns={}", self.spec.row_count, self.spec.columns.count());
        if let Some(layout) = self.spec.columns.layout_name() {
            parameters.push_str(", Layout=");
            parameters.push_str(layout);
        }
        parameters.push_str(", Text=");
        parameters.push_str(text.name());
        parameters
    }
}

/// Creates the grid of `spec` and moves it to its initial offset, outside the measurement.
pub fn prepare(target: &mut dyn GridTarget, spec: &GridSpec) {
    target.create(spec);
    let (x, y) = spec.initial_offset;
    if x != 0.0 || y != 0.0 {
        target.set_offset(x, y);
        target.frame();
    }
}

/// `CreateAndLayoutGrid`, and the first layout of the columns of `FirstLayoutBenchmarks`:
/// create the grid in a window, bring it to its first frame, close the window.
pub fn create_and_layout(target: &mut dyn GridTarget, spec: &GridSpec, _operations: u32) -> usize {
    prepare(target, spec);
    let checksum = target.checksum();
    target.destroy();
    checksum
}

/// `VerticalSmallScrolls`: one pixel down at a time.
pub fn vertical_small_scrolls(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for i in 1..=operations {
        target.set_offset(0.0, f64::from(i));
        target.frame();
    }
    target.checksum()
}

/// `VerticalRowScrolls`: one row (24 pixels) down at a time, with the rows invalidated.
pub fn vertical_row_scrolls(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for i in 1..=operations {
        target.set_offset(0.0, f64::from(i * 24));
        target.invalidate_rows();
        target.frame();
    }
    target.checksum()
}

/// `VerticalJitterScrolls`: 100 pixels down and back.
pub fn vertical_jitter_scrolls(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for i in 0..operations {
        target.set_offset(0.0, if (i & 1) == 0 { 100.0 } else { 0.0 });
        target.frame();
    }
    target.checksum()
}

/// `ForcedStationaryVerticalLayouts`: the rows invalidated and the window brought up to date
/// without a change of the offset.
pub fn stationary_frames(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for _ in 0..operations {
        target.invalidate_rows();
        target.frame();
    }
    target.checksum()
}

/// `HorizontalSmallScrolls`: one pixel right at a time.
pub fn horizontal_small_scrolls(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for i in 1..=operations {
        target.set_offset(f64::from(i), 0.0);
        target.frame();
    }
    target.checksum()
}

/// `HorizontalColumnScrolls`: between 100 and 200 pixels (a column).
pub fn horizontal_column_scrolls(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for i in 0..operations {
        target.set_offset(if (i & 1) == 0 { 100.0 } else { 200.0 }, 0.0);
        target.frame();
    }
    target.checksum()
}

/// `HorizontalFarColumnScrolls`: between 90 100 and 90 000 pixels, near column 900 of 1 000.
pub fn horizontal_far_column_scrolls(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for i in 0..operations {
        target.set_offset(if (i & 1) == 0 { 90_100.0 } else { 90_000.0 }, 0.0);
        target.frame();
    }
    target.checksum()
}

/// `CollectionInsertRemoveBurst`: a row inserted at index 2 and removed again, a frame after
/// each.
pub fn collection_insert_remove_burst(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for i in 0..operations as i32 {
        target.insert_row(2, RowData::inserted(i));
        target.frame();
        target.remove_row(2);
        target.frame();
    }
    target.checksum()
}

/// `CollectionMoveBurst`: the row at index 2 moved to 12 and back, a frame after each.
pub fn collection_move_burst(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for _ in 0..operations {
        target.move_row(2, 12);
        target.frame();
        target.move_row(12, 2);
        target.frame();
    }
    target.checksum()
}

/// `DetachReattach`: the grid taken out of its window and put back, a frame after each.
pub fn detach_reattach(target: &mut dyn GridTarget, _: &GridSpec, operations: u32) -> usize {
    for _ in 0..operations {
        target.detach();
        target.frame();
        target.attach();
        target.frame();
    }
    target.checksum()
}

/// `SortThousandRows` with the grid on screen: the rows sorted by the id, in alternating
/// directions, and the next frame.
pub fn sort_with_frame(target: &mut dyn GridTarget, _: &GridSpec, _operations: u32) -> usize {
    target.sort_next();
    target.frame();
    target.checksum()
}

const fn spec(rows: RowSet, row_count: usize, columns: crate::model::ColumnSet) -> GridSpec {
    GridSpec { rows, row_count, columns, initial_offset: (0.0, 0.0) }
}

/// Every grid scenario, in the order of the table. The row counts that the TreeDataGrid
/// benchmarks fix are kept; the vertical scrolls and the collection edits also run with
/// 100 000 rows, which the benchmarks do not have.
pub fn grid_scenarios() -> Vec<GridScenario> {
    use crate::model::{ColumnLayout, ColumnSet};

    let virtualization = |columns| ColumnSet::Virtualization { columns };
    let mut scenarios = vec![GridScenario {
        name: "create_and_layout_grid",
        spec: spec(RowSet::Items, 1_000, virtualization(12)),
        operations: 1,
        lifetime: Lifetime::PerInvocation,
        body: create_and_layout,
    }];
    for layout in [ColumnLayout::Fixed, ColumnLayout::Auto, ColumnLayout::Star] {
        scenarios.push(GridScenario {
            name: "create_and_first_layout",
            spec: spec(RowSet::PaddedItems, 1_000, ColumnSet::FirstLayout(layout)),
            operations: 1,
            lifetime: Lifetime::PerInvocation,
            body: create_and_layout,
        });
    }

    for row_count in [10_000, 100_000] {
        scenarios.push(GridScenario {
            name: "vertical_small_scrolls",
            spec: spec(RowSet::Items, row_count, virtualization(12)),
            operations: VERTICAL_SCROLL_OPERATIONS,
            lifetime: Lifetime::PerIteration,
            body: vertical_small_scrolls,
        });
    }
    for row_count in [10_000, 100_000] {
        scenarios.push(GridScenario {
            name: "vertical_row_scrolls",
            spec: spec(RowSet::Items, row_count, virtualization(12)),
            operations: VERTICAL_SCROLL_OPERATIONS,
            lifetime: Lifetime::PerIteration,
            body: vertical_row_scrolls,
        });
    }
    for row_count in [10_000, 100_000] {
        scenarios.push(GridScenario {
            name: "vertical_jitter_scrolls",
            spec: spec(RowSet::Items, row_count, virtualization(12)),
            operations: VERTICAL_SCROLL_OPERATIONS,
            lifetime: Lifetime::PerIteration,
            body: vertical_jitter_scrolls,
        });
    }
    scenarios.push(GridScenario {
        name: "forced_stationary_vertical_layouts",
        spec: spec(RowSet::Items, 10_000, virtualization(12)),
        operations: STATIONARY_LAYOUT_OPERATIONS,
        lifetime: Lifetime::PerIteration,
        body: stationary_frames,
    });

    scenarios.push(GridScenario {
        name: "horizontal_small_scrolls",
        spec: spec(RowSet::Items, 1_000, virtualization(200)),
        operations: HORIZONTAL_SCROLL_OPERATIONS,
        lifetime: Lifetime::PerIteration,
        body: horizontal_small_scrolls,
    });
    scenarios.push(GridScenario {
        name: "horizontal_column_scrolls",
        spec: spec(RowSet::Items, 1_000, virtualization(200)),
        operations: HORIZONTAL_COLUMN_OPERATIONS,
        lifetime: Lifetime::PerIteration,
        body: horizontal_column_scrolls,
    });
    scenarios.push(GridScenario {
        name: "horizontal_far_column_scrolls",
        spec: GridSpec { initial_offset: (90_000.0, 0.0), ..spec(RowSet::Items, 1_000, virtualization(1_000)) },
        operations: HORIZONTAL_COLUMN_OPERATIONS,
        lifetime: Lifetime::PerIteration,
        body: horizontal_far_column_scrolls,
    });

    scenarios.push(GridScenario {
        name: "sort_thousand_rows_with_frame",
        spec: spec(RowSet::Descending, 1_000, ColumnSet::IdTitle),
        operations: 1,
        lifetime: Lifetime::Shared,
        body: sort_with_frame,
    });

    for row_count in [10_000, 100_000] {
        scenarios.push(GridScenario {
            name: "collection_insert_remove_burst",
            spec: spec(RowSet::Items, row_count, virtualization(12)),
            operations: COLLECTION_EDIT_OPERATIONS,
            lifetime: Lifetime::PerIteration,
            body: collection_insert_remove_burst,
        });
    }
    for row_count in [10_000, 100_000] {
        scenarios.push(GridScenario {
            name: "collection_move_burst",
            spec: spec(RowSet::Items, row_count, virtualization(12)),
            operations: COLLECTION_MOVE_OPERATIONS,
            lifetime: Lifetime::PerIteration,
            body: collection_move_burst,
        });
    }
    scenarios.push(GridScenario {
        name: "detach_reattach",
        spec: spec(RowSet::Items, 10_000, virtualization(50)),
        operations: DETACH_REATTACH_OPERATIONS,
        lifetime: Lifetime::PerIteration,
        body: detach_reattach,
    });

    scenarios
}
