//! ferroui-treedatagrid-gpui-comparison
//!
//! Compares FerroUI's TreeDataGrid with the tables of GPUI in the scenarios of the TreeDataGrid
//! benchmarks (`tests/FerroUI.TreeDataGrid.Benchmarks`): the same rows, columns, viewport,
//! offsets and operation counts, measured with the same harness (`ferroui_benchmarks::harness`)
//! in the same process. The targets:
//!
//! - `FerroUiLayout`: the TreeDataGrid, a frame being a layout pass of its window (what the
//!   TreeDataGrid benchmarks measure);
//! - `FerroUiRender`: the TreeDataGrid, a frame being the layout pass and the recording of the
//!   drawing of the whole window (see [`ferroui_targets`]);
//! - `GpuiComponent`: the `DataTable` of gpui-component (and its `Tree` for the tree scenario);
//! - `GpuiUniformList`: a table of one's own on GPUI's `uniform_list`, the least a GPUI table is.
//!
//! A GPUI frame is a full window frame of GPUI's test platform: render, layout, prepaint and
//! paint into the scene, without presenting it (see [`gpui_targets`]).
//!
//! Each benchmark is registered as `comparison::<Target>.<scenario>[<parameters>]`, so the
//! filter of the harness selects a scenario on every target (`vertical_small_scrolls`) or a
//! target (`GpuiComponent.`). [`run`] prints one table: a row per scenario and parameters, a
//! column per target, and the ratio of each GPUI target to FerroUI.
//!
//! See `README.md` for the command, the methodology and the caveats.

pub mod ferroui_targets;
pub mod gpui_targets;
pub mod model;
pub mod scenarios;

mod program;

pub use program::run;

use ferroui_benchmarks::harness::Registry;
use ferroui_treedatagrid_benchmarks::neutral_source_benchmarks::NeutralSourceBenchmarks;
use model::{ColumnSet, GridSpec, RowSet, Text, GPUI_HAS_PLATFORM_TEXT};
use scenarios::{GridScenario, GridTarget, Lifetime, TreeTarget};

/// The category of every benchmark of the crate.
pub const CATEGORY: &str = "comparison";

/// A target of the comparison: a column of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    FerroUiLayout,
    FerroUiRender,
    GpuiComponent,
    GpuiUniformList,
}

impl TargetKind {
    pub const ALL: [TargetKind; 4] =
        [TargetKind::FerroUiLayout, TargetKind::FerroUiRender, TargetKind::GpuiComponent, TargetKind::GpuiUniformList];

    /// The class of the target's benchmarks.
    pub fn class(self) -> &'static str {
        match self {
            TargetKind::FerroUiLayout => "FerroUiLayout",
            TargetKind::FerroUiRender => "FerroUiRender",
            TargetKind::GpuiComponent => "GpuiComponent",
            TargetKind::GpuiUniformList => "GpuiUniformList",
        }
    }

    /// The heading of the target's column in the table.
    pub fn heading(self) -> &'static str {
        match self {
            TargetKind::FerroUiLayout => "FerroUI TDG layout",
            TargetKind::FerroUiRender => "FerroUI TDG layout+render",
            TargetKind::GpuiComponent => "gpui-component",
            TargetKind::GpuiUniformList => "gpui uniform_list",
        }
    }

    fn is_gpui(self) -> bool {
        matches!(self, TargetKind::GpuiComponent | TargetKind::GpuiUniformList)
    }

    /// Whether the target runs with `text`: the GPUI targets have the text of the platform on
    /// macOS only.
    pub fn supports_text(self, text: Text) -> bool {
        !(self.is_gpui() && text == Text::Platform && !GPUI_HAS_PLATFORM_TEXT)
    }

    /// Whether the target can show the grid of `spec` with `text`.
    pub fn supports_grid(self, spec: &GridSpec, text: Text) -> bool {
        self.supports_text(text)
            && match self {
                TargetKind::FerroUiLayout | TargetKind::FerroUiRender => true,
                TargetKind::GpuiComponent => gpui_targets::GpuiComponentGrid::supports(spec),
                TargetKind::GpuiUniformList => gpui_targets::GpuiListGrid::supports(spec),
            }
    }

    /// A grid target of this kind, before it creates a grid.
    pub fn grid(self, text: Text) -> Box<dyn GridTarget> {
        match self {
            TargetKind::FerroUiLayout => Box::new(ferroui_targets::FerroGrid::new(text, false)),
            TargetKind::FerroUiRender => Box::new(ferroui_targets::FerroGrid::new(text, true)),
            TargetKind::GpuiComponent => Box::new(gpui_targets::GpuiComponentGrid::new(text)),
            TargetKind::GpuiUniformList => Box::new(gpui_targets::GpuiListGrid::new(text)),
        }
    }

    /// The tree target of this kind, with its tree on screen.
    pub fn tree(self, text: Text) -> Box<dyn TreeTarget> {
        match self {
            TargetKind::FerroUiLayout => Box::new(ferroui_targets::FerroTree::new(text, false)),
            TargetKind::FerroUiRender => Box::new(ferroui_targets::FerroTree::new(text, true)),
            TargetKind::GpuiComponent => Box::new(gpui_targets::GpuiComponentTree::new(text)),
            TargetKind::GpuiUniformList => Box::new(gpui_targets::GpuiListTree::new(text)),
        }
    }
}

/// How many operations an invocation performs: the counts of the TreeDataGrid benchmarks, or at
/// most two for the quick tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Full,
    Quick,
}

impl Scale {
    pub fn operations(self, full: u32) -> u32 {
        match self {
            Scale::Full => full,
            Scale::Quick => full.min(2),
        }
    }
}

const TEXTS: [Text; 2] = [Text::Synthetic, Text::Platform];

/// The grid of the sorting scenarios: the thousand rows and the two columns of
/// `NeutralSourceBenchmarks`.
const SORT_SPEC: GridSpec =
    GridSpec { rows: RowSet::Descending, row_count: 1_000, columns: ColumnSet::IdTitle, initial_offset: (0.0, 0.0) };

/// Registers every benchmark of the crate with the operation counts of the TreeDataGrid
/// benchmarks.
pub fn register(registry: &mut Registry) {
    register_scaled(registry, Scale::Full);
}

/// Registers every benchmark of the crate with at most two operations an invocation: what the
/// quick tests run.
pub fn register_quick(registry: &mut Registry) {
    register_scaled(registry, Scale::Quick);
}

/// Registers the benchmarks scenario by scenario, so that the targets of a scenario run one
/// after another.
pub fn register_scaled(registry: &mut Registry, scale: Scale) {
    for scenario in scenarios::grid_scenarios() {
        for text in TEXTS {
            for target in TargetKind::ALL {
                if target.supports_grid(&scenario.spec, text) {
                    register_grid(registry, &scenario, text, target, scale);
                }
            }
        }
        if scenario.name == "sort_thousand_rows_with_frame" {
            register_source_only_sort(registry);
        }
    }
    register_source_only_expansion(registry);
    register_tree_with_frames(registry);
}

fn register_grid(registry: &mut Registry, scenario: &GridScenario, text: Text, target: TargetKind, scale: Scale) {
    let operations = scale.operations(scenario.operations);
    let spec = scenario.spec;
    let body = scenario.body;
    let parameters = scenario.parameters(text);
    let mut class = registry.class(CATEGORY, target.class());
    match scenario.lifetime {
        Lifetime::PerInvocation => {
            class
                .benchmark(
                    scenario.name,
                    parameters,
                    move || target.grid(text),
                    move |grid: &mut Box<dyn GridTarget>| body(grid.as_mut(), &spec, operations),
                )
                .operations_per_invoke(operations);
        }
        Lifetime::PerIteration => {
            // Each iteration setup cleans up the grid of the previous iteration first.
            class
                .benchmark_with_iteration_setup(
                    scenario.name,
                    parameters,
                    move || target.grid(text),
                    move |grid: &mut Box<dyn GridTarget>| {
                        grid.destroy();
                        scenarios::prepare(grid.as_mut(), &spec);
                    },
                    move |grid: &mut Box<dyn GridTarget>| body(grid.as_mut(), &spec, operations),
                )
                .operations_per_invoke(operations);
        }
        Lifetime::Shared => {
            class
                .benchmark(
                    scenario.name,
                    parameters,
                    move || {
                        let mut grid = target.grid(text);
                        scenarios::prepare(grid.as_mut(), &spec);
                        grid
                    },
                    move |grid: &mut Box<dyn GridTarget>| body(grid.as_mut(), &spec, operations),
                )
                .operations_per_invoke(operations);
        }
    }
}

/// `SortThousandRows` without a frame: on the FerroUI side the benchmark of
/// `NeutralSourceBenchmarks` itself (the classic columns, a source and its presentation without
/// a control); on the GPUI side the sort of the delegate of a `DataTable` on screen, without the
/// next frame. The table of one's own sorts the same vector the same way, so it is left out.
fn register_source_only_sort(registry: &mut Registry) {
    let parameters = "Rows=1000, Columns=2, Text=Synthetic";
    registry.class(CATEGORY, TargetKind::FerroUiLayout.class()).benchmark(
        "sort_thousand_rows",
        parameters,
        || NeutralSourceBenchmarks::new(false),
        |benchmarks: &mut NeutralSourceBenchmarks| benchmarks.sort_thousand_rows(),
    );
    registry.class(CATEGORY, TargetKind::GpuiComponent.class()).benchmark(
        "sort_thousand_rows",
        parameters,
        || {
            let mut grid = gpui_targets::GpuiComponentGrid::new(Text::Synthetic);
            scenarios::prepare(&mut grid, &SORT_SPEC);
            grid
        },
        |grid: &mut gpui_targets::GpuiComponentGrid| {
            grid.sort_next();
            0_usize
        },
    );
}

/// `ExpandCollapseThousandRows` without frames: on the FerroUI side the benchmark of
/// `NeutralSourceBenchmarks` itself; on the GPUI side the expansion and the collapse of the
/// root of the `Tree` of gpui-component through its actions, without frames.
fn register_source_only_expansion(registry: &mut Registry) {
    let parameters = "Rows=1000, Columns=2, Text=Synthetic";
    registry.class(CATEGORY, TargetKind::FerroUiLayout.class()).benchmark(
        "expand_collapse_thousand_rows",
        parameters,
        || NeutralSourceBenchmarks::new(false),
        |benchmarks: &mut NeutralSourceBenchmarks| benchmarks.expand_collapse_thousand_rows(),
    );
    registry.class(CATEGORY, TargetKind::GpuiComponent.class()).benchmark(
        "expand_collapse_thousand_rows",
        parameters,
        || gpui_targets::GpuiComponentTree::new(Text::Synthetic),
        |tree: &mut gpui_targets::GpuiComponentTree| tree.expand_collapse(false),
    );
}

/// `ExpandCollapseThousandRows` with the tree on screen and a frame after the expansion and
/// after the collapse, on every target.
fn register_tree_with_frames(registry: &mut Registry) {
    for text in TEXTS {
        for target in TargetKind::ALL {
            if !target.supports_text(text) {
                continue;
            }
            registry.class(CATEGORY, target.class()).benchmark(
                "expand_collapse_thousand_rows_with_frames",
                format!("Rows=1000, Columns=2, Text={}", text.name()),
                move || target.tree(text),
                |tree: &mut Box<dyn TreeTarget>| tree.expand_collapse(true),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_benchmarks::harness;
    use std::collections::HashSet;

    #[test]
    fn every_benchmark_has_a_name_of_its_own() {
        let mut registry = Registry::new();
        register(&mut registry);
        assert!(!registry.is_empty(), "the crate registers benchmarks");

        let mut names = HashSet::new();
        for descriptor in registry.descriptors() {
            assert!(descriptor.operations_per_invoke > 0);
            assert!(names.insert(descriptor.full_name()), "{} is registered twice", descriptor.full_name());
        }
    }

    #[test]
    fn every_scenario_has_every_target_that_can_show_it() {
        let mut registry = Registry::new();
        register(&mut registry);
        for scenario in scenarios::grid_scenarios() {
            for target in TargetKind::ALL {
                let registered = registry
                    .descriptors()
                    .any(|descriptor| descriptor.name == scenario.name && descriptor.class == target.class());
                let supported = target.supports_grid(&scenario.spec, Text::Synthetic);
                assert!(registered || !supported, "{} has no benchmark on {}", scenario.name, target.class());
            }
        }
    }

    /// One invocation of every benchmark, with at most two operations, on a thread of its own:
    /// the setup, the iteration setup and the body of every scenario on every target.
    #[test]
    fn every_scenario_runs_once_on_every_target() {
        harness::smoke(register_quick);
    }

    #[test]
    fn the_data_table_scrolls_its_rows_and_columns() {
        let mut grid = gpui_targets::GpuiComponentGrid::new(Text::Synthetic);
        let spec = GridSpec {
            rows: RowSet::Items,
            row_count: 10_000,
            columns: ColumnSet::Virtualization { columns: 50 },
            initial_offset: (0.0, 0.0),
        };
        scenarios::prepare(&mut grid, &spec);
        grid.set_offset(1_000.0, 240.0);
        grid.frame();
        let (rows, columns) = grid.visible_range();
        assert_eq!(rows.start, 10, "240 pixels are ten rows of 24");
        assert!(rows.len() < 30, "the rows are virtualized: {rows:?}");
        assert!(columns.start > 0, "1 000 pixels are past the first columns: {columns:?}");
        assert!(columns.len() < 20, "the columns are virtualized: {columns:?}");
    }

    #[test]
    fn the_uniform_list_table_scrolls_its_rows_and_columns() {
        let mut grid = gpui_targets::GpuiListGrid::new(Text::Synthetic);
        let spec = GridSpec {
            rows: RowSet::Items,
            row_count: 10_000,
            columns: ColumnSet::Virtualization { columns: 50 },
            initial_offset: (0.0, 0.0),
        };
        scenarios::prepare(&mut grid, &spec);
        grid.set_offset(1_000.0, 240.0);
        grid.frame();
        let (row, column) = grid.first_visible();
        assert_eq!(row, 10, "240 pixels are ten rows of 24");
        // The id column is 80 pixels wide and the others 100: 1 000 pixels are in column 10.
        assert_eq!(column, 10);
    }
}
