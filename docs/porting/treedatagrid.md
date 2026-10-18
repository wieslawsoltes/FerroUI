# TreeDataGrid port

The port of TreeDataGrid (<https://github.com/wieslawsoltes/TreeDataGrid>, MIT, Copyright (c) 2026 Wiesław Šoltés; earlier parts MIT, Copyright (c) .NET Foundation and Contributors) to FerroUI: the framework-neutral core, the control library, its themes, the demo application, the unit tests and the benchmarks. The reference is the checkout at `/Users/wieslawsoltes/GitHub/Avalonia.Controls.TreeDataGrid`, branch `master`, commit `3ca47316` (2026-09-05), read only.

## 1. What the reference contains

| Project | Files | Lines | Role |
|---|---|---|---|
| `src/TreeDataGrid.Core` | 52 | 7,925 | Framework-neutral sources (`FlatTreeDataGridSource`, `HierarchicalTreeDataGridSource`), columns, rows, sorting, expansion, row and cell selection; may not reference the UI framework |
| `src/Avalonia.Controls.TreeDataGrid` | 150 + 2 themes | 21,300 + 373 | The control: virtualizing presenters, rows, cells, column headers, the view-side columns and cells, the presentation of a core source, selection interaction (keyboard, pointer, type-ahead), drag and drop, automation peers, a vendored typed-binding engine, and a "classic" copy of the core's sources, rows and selection in the framework's namespace |
| `src/TreeDataGrid.Avalonia` | 0 | 0 | A second package identity that compiles the files of the control project |
| `tests/TreeDataGrid.Core.Tests` | 7 | 5,140 | 192 tests without UI |
| `tests/Avalonia.Controls.TreeDataGrid.Tests` | 45 | 15,445 | 473 tests on the headless platform |
| `samples/TreeDataGridDemo` | 28 | 2,492 | Demo of the classic API: Countries, Files, Wikipedia, Drag/Drop, People (declarative columns), template column reuse, finding a displayed row, rows of different heights |
| `samples/TreeDataGridCoreDemo` | 6 | 689 | The same pages over the core API (`Model` and presentation options) |
| `benchmarks/...Benchmarks` | 11 | 1,542 | BenchmarkDotNet suites: virtualization (scrolling, collection bursts, detach and reattach), sources (create, sort, expand), first layout, column widths, column geometry, element factory, template cell recycling |

## 2. Crates and places

| Reference | Port | Path |
|---|---|---|
| `TreeDataGrid.Core` | `ferroui-treedatagrid-core` | `src/TreeDataGrid/FerroUI.TreeDataGrid.Core` |
| `Avalonia.Controls.TreeDataGrid` (and `TreeDataGrid.Avalonia`) | `ferroui-controls-treedatagrid` | `src/TreeDataGrid/FerroUI.Controls.TreeDataGrid` |
| `TreeDataGridDemo` and `TreeDataGridCoreDemo` | `treedatagrid-demo`: the library (models, view models, one view per page) and the binary of the application, as the other samples of the workspace | `samples/TreeDataGridDemo` |
| ControlCatalog pages | pages of the section "Collections & Data" that host the pages of `treedatagrid-demo` | `samples/ControlCatalog/Pages/TreeDataGrid*` |
| Unit tests | `tests/` modules of the two crates and of the demo | next to the code, as in every crate of the workspace |
| Benchmarks | `benches` of `ferroui-controls-treedatagrid` (a harness of the workspace's kind: headless application, many operations per sample, per-operation results) | `src/TreeDataGrid/FerroUI.Controls.TreeDataGrid/benches` |

The names follow the rules of the repository: no upstream framework name in code or crate names; the namespaces `Avalonia.Controls.*` of the control project become modules of `ferroui-controls-treedatagrid` (`primitives`, `models`, `selection`, `presentation`, `automation`), and `TreeDataGridCore.*` modules of `ferroui-treedatagrid-core`.

## 3. Decisions

1. **One data layer.** The reference keeps two copies of the sources, rows and selection: the core's (`TreeDataGridCore`) and a "classic" one in the framework's namespace, with interfaces that extend the core's and convert `GridLength` and `IndexPath` between the two. The port has one: the core's types, with the members the classic copy adds (`FlatTreeDataGridSource::filter`, `refresh_filter`, the selection interaction of the row selection model) where they belong. Both entry points of the control stay: `source` (sources with the view-side columns: `TextColumn`, `CheckBoxColumn`, `TemplateColumn`, `HierarchicalExpanderColumn` of the control crate) and `model` (sources with neutral columns, shown through presentation options). The adapters (`Adapters/CoreConversions`, `LegacySourcePresentation`) become conversions inside the control crate. Recorded in `DEVIATIONS.md`.
2. **Models are `Rc<M>`.** `TModel : class` is a reference type: the port's sources hold `Rc<M>` and compare models by identity (`Rc::ptr_eq`) where the reference uses `ReferenceEquals`. Model notifications use the framework's `INotifyPropertyChanged`; collections are the framework's notifying lists (`FerroList<Rc<M>>`, and any `ItemsSource` through an adapter of the control crate), whose change arguments carry the old and new items as the reference needs.
3. **Expressions become accessors.** The reference uses `Expression<Func<TModel, TValue>>` for getters, synthesized setters, property names and the chain of objects whose `PropertyChanged` it observes. The port has `Accessor<M, V>` (getter, optional setter, the property name, the links of the path), built by a macro (`accessor!(Person => name)`, `accessor!(Person => expansion.is_expanded)`) or from closures (`Accessor::from_fn`, observing the root object only, as the reference's delegate columns). The vendored typed-binding engine (`Experimental/`) is replaced by bindings of accessors on the framework's binding system.
4. **Generic visitors become erased dispatch.** Generic methods of interfaces (`Accept<TResult>`, `IColumnVisitor.Visit<TValue>`) are not object-safe in Rust; the dispatch they do moves into the generic implementations (`create_presentation`, `ColumnKind<M>`).
5. **Declarative columns of markup (the reference's "v12" API).** `TreeDataGridColumn` declarations with bindings are kept; their values are read through the framework's compiled and reflection bindings, without the reference's probe of private members.
6. **Themes.** `Generic.xaml` and `Fluent.xaml` are ported; the Simple and WinUI themes of the framework get the same resource keys mapped to their colours (a FerroUI addition), so that the ControlCatalog, whose default theme is WinUI, shows the control in its theme.
7. **Threading.** As the framework: single-threaded types (`Rc`, `RefCell`), changes from another thread are posted to the UI thread by the application.
8. **Re-entrancy.** Events are raised after the state they describe is committed and borrows are released, from snapshots of the handler lists, as the framework's collections do.

### 3.1 The control layer

- **Untyped presentation.** The reference's control talks to a `TreeDataGridPresentation` (columns as `IColumns`, rows as `ITreeDataGridRows` that realize cells, selection, sorting, moving rows), created for a typed source through a visitor (`TreeDataGridPresentation<TModel>`). A source given to the control in markup or by a binding arrives as an untyped value, and Rust has no generic dispatch over a trait object, so the port's presentation is untyped: models are boxed values, the values of neutral columns are read through the erased access of the core (`IValueColumn` and the untyped column contract), and the source is reached through `ITreeDataGridSource`. The typed sources stay the API of applications.
- **View columns.** The columns of the control crate (`TextColumn<M, V>`, `CheckBoxColumn<M>`, `TemplateColumn<M>`, `HierarchicalExpanderColumn<M>`, the reflection, filtered and object expander columns) are the reference's classic columns: they create their cells and take part in the column layout (`IUpdateColumnLayout`), and they are also core columns (`IColumnOf<M>`), so a core source holds them as it holds neutral columns. A view column answers the core's `IColumn::view` with its handle; the presentation uses the handle when there is one and presents a neutral column through the presentation options otherwise (`CellColumnFactory`).
- **Bindings of cells.** The vendored typed-binding engine of the reference (`Experimental/`) is replaced by accessors (`Accessor<M, V>` of the core): a cell reads its value with the getter, writes it with the setter, and follows the model's `PropertyChanged` for the member name and the path observer for a nested path.
- **Framework idioms.** Controls are `ferro_class!` classes with `ferro_properties!`, as in `ferroui-controls` and `ferroui-controls-color-picker`; the themes are compiled markup of the crate (`build.rs`, as the colour picker's).

## 4. Stages

| Stage | Content | Gate |
|---|---|---|
| T1 | Core primitives and notification infrastructure: `GridLength`, `IndexPath`, `CellIndex`, sorting helpers, `ListSpan`, notifying bases, index ranges, `CollectionChangedEventManager`, items source view, property path observer, accessors | unit tests of the module |
| T2 | Core columns and rows: `Columns`, expander column, sortable rows, anonymous rows, hierarchical rows | |
| T3 | Core selection and sources: tree selection model, row and cell selection, flat and hierarchical sources | the 192 tests of `TreeDataGrid.Core.Tests` |
| T4 | View-side models: column list with star and auto layout and geometry, cells (text, check box, template, expander), view-side columns, presentation of a core source, cell model pool | the model tests (`Models/`, `CoreSource*`, `TypedBinding` equivalents) |
| T5 | Virtualization primitives: presenter base, rows, cells and column header presenters, element factory, row, cells, column header | the `Primitives/` tests |
| T6 | The control: `TreeDataGrid`, selection interaction, drag and drop, themes, automation peers, declarative columns | the control tests (`TreeDataGridTests_*`, `*SourceTests`, selection, automation, `V12`) |
| T7 | Demo library and application, ControlCatalog pages sharing the demo's pages | the demo tests; screenshots |
| T8 | Benchmarks, compared with the reference's suite run on the same machine | results in section 6 |

## 5. Progress

| Stage | State |
|---|---|
| T1 | done |
| T2 | done |
| T3 | done: the 192 tests of `TreeDataGrid.Core.Tests` pass (`cargo test -p ferroui-treedatagrid-core`, with 6 tests of the port's own) |
| T4 | done |
| T5 | done |
| T6 | done: the 497 tests of the control crate pass (`cargo test -p ferroui-controls-treedatagrid`): the reference's control tests and the port's own |
| T7 | done: `samples/TreeDataGridDemo` (one crate: the library of models, view models and one view per page, and the binary `treedatagrid-demo`), every page of the reference demo and the core demo as the page "Core"; its 53 tests pass (the reference's demo tests, the generated document tests and a test that shows and draws every page); the ControlCatalog page TreeDataGrid shows the view of the tabs of the demo. The deviations are in `DEVIATIONS.md`, section TreeDataGrid |
| T8 | done: `tests/FerroUI.TreeDataGrid.Benchmarks` ports every benchmark class of the reference; the comparison is in section 6 |

## 6. Performance

The reference's suite (`benchmarks/run-benchmarks.sh`: BenchmarkDotNet, 5 warm-up and 10 measured iterations a benchmark, the operations of an invocation normalised) and the port's (`tests/FerroUI.TreeDataGrid.Benchmarks`: the harness of the workspace, 3 warm-up and 15 measured iterations, the same normalisation) were run one after the other on one machine with nothing else running: an Apple M3 Pro, macOS 26.6; the reference at its commit 3ca47316 on .NET 10.0.201 (the suite targets .NET 8 and runs on the installed .NET 10 with `DOTNET_ROLL_FORWARD=Major`); the port in a release build with rustc 1.90.0. The times are the mean time of one operation.

```sh
# The reference, in a clone outside this repository.
dotnet benchmarks/Avalonia.Controls.TreeDataGrid.Benchmarks/bin/Release/net8.0/Avalonia.Controls.TreeDataGrid.Benchmarks.dll --filter '*' --exporters json
# The port.
cargo test --release -p ferroui-treedatagrid-benchmarks --lib run_benchmarks -- --ignored --nocapture --test-threads=1
```

The first run of the port was faster in 37 of the 71 benchmarks, with a geometric mean of the ratio of 0.91: scrolling was 1.5 to 1.9 times slower, reusing an element 2.5 times, finding a column 1.8 times. The profiles showed the costs in the framework and in the port's own code rather than in the algorithms, which follow the reference: the scroll presenter translating every anchor candidate to itself walked both visuals up to the root (`transform_to_visual`, now one walk for an ancestor) and read the render transform of every visual it passed through the value store (now only for the visuals that have one); maps keyed by identities hashed with SipHash (the layout queues, the scroll anchors, the pools of the element factory, the collection change listeners: now the Fx hasher); the recycle keys were type names built on every lookup (now interned); the text of a cell went through the composite formatter for the default format `{0}`; a column lookup called an outlined geometry check. The deviations this needed are in `DEVIATIONS.md` (sections Visual tree and TreeDataGrid).

After these changes the port is as fast or faster in 58 of the 71 benchmarks, with a geometric mean of the ratio port / reference of 0.68. Of the 13 others, 12 are within 14 % of the reference (scrolls that realize rows and cells, bursts of changes of the items, reusing an element from a pool of 100): what remains there is spread over allocation and the property system of the framework. Expanding and collapsing a thousand rows takes 1.8 to 1.9 times as long (14.5 against 7.7 microseconds): the rows of an expansion are counted references, copied into the flattened rows and into the arguments of the change the handlers receive, where the reference copies plain references; a contract of the change that does not copy the rows would remove it.

| Benchmark | Parameters | Reference (.NET) | Port | Port / reference |
|---|---|---|---|---|
| ColumnGeometryBenchmarks.locate_viewport | ColumnCount=1000 | 13.7 ns | 10.0 ns | 0.73 |
| ColumnGeometryBenchmarks.locate_viewport | ColumnCount=20 | 6.7 ns | 6.0 ns | 0.88 |
| ColumnGeometryBenchmarks.locate_viewport | ColumnCount=200 | 10.7 ns | 7.8 ns | 0.73 |
| ColumnLayoutBenchmarks.commit_unchanged_widths | ColumnCount=1000 | 11.6 ns | 3.3 ns | 0.29 |
| ColumnLayoutBenchmarks.commit_unchanged_widths | ColumnCount=20 | 11.2 ns | 3.3 ns | 0.30 |
| ColumnLayoutBenchmarks.commit_unchanged_widths | ColumnCount=200 | 14.1 ns | 3.3 ns | 0.23 |
| ColumnWidthBatchBenchmarks.batched_width_commit | Layout=Auto, RowCount=20 | 16.14 us | 2.73 us | 0.17 |
| ColumnWidthBatchBenchmarks.batched_width_commit | Layout=Auto, RowCount=40 | 16.99 us | 3.41 us | 0.20 |
| ColumnWidthBatchBenchmarks.batched_width_commit | Layout=Mixed, RowCount=20 | 16.87 us | 3.03 us | 0.18 |
| ColumnWidthBatchBenchmarks.batched_width_commit | Layout=Mixed, RowCount=40 | 17.11 us | 4.05 us | 0.24 |
| ColumnWidthBatchBenchmarks.incremental_width_commits | Layout=Auto, RowCount=20 | 18.05 us | 4.73 us | 0.26 |
| ColumnWidthBatchBenchmarks.incremental_width_commits | Layout=Auto, RowCount=40 | 20.09 us | 7.31 us | 0.36 |
| ColumnWidthBatchBenchmarks.incremental_width_commits | Layout=Mixed, RowCount=20 | 18.59 us | 5.56 us | 0.30 |
| ColumnWidthBatchBenchmarks.incremental_width_commits | Layout=Mixed, RowCount=40 | 21.42 us | 9.13 us | 0.43 |
| ElementFactoryBenchmarks.same_parent_reuse | PoolSize=10 | 59.1 ns | 60.6 ns | 1.03 |
| ElementFactoryBenchmarks.same_parent_reuse | PoolSize=100 | 70.9 ns | 79.2 ns | 1.12 |
| ElementFactoryBenchmarks.same_parent_reuse | PoolSize=1000 | 90.7 ns | 90.6 ns | 1.00 |
| FirstLayoutBenchmarks.legacy_natural_width_probe | Layout=Auto, CacheLength=0 | 1.74 ms | 755.51 us | 0.44 |
| FirstLayoutBenchmarks.legacy_natural_width_probe | Layout=Auto, CacheLength=0.5 | 1.67 ms | 1.36 ms | 0.81 |
| FirstLayoutBenchmarks.legacy_natural_width_probe | Layout=Fixed, CacheLength=0 | 1.80 ms | 1.04 ms | 0.58 |
| FirstLayoutBenchmarks.legacy_natural_width_probe | Layout=Fixed, CacheLength=0.5 | 1.66 ms | 1.34 ms | 0.81 |
| FirstLayoutBenchmarks.legacy_natural_width_probe | Layout=Mixed, CacheLength=0 | 919.56 us | 802.30 us | 0.87 |
| FirstLayoutBenchmarks.legacy_natural_width_probe | Layout=Mixed, CacheLength=0.5 | 1.68 ms | 1.41 ms | 0.84 |
| FirstLayoutBenchmarks.legacy_natural_width_probe | Layout=Star, CacheLength=0 | 1.81 ms | 773.44 us | 0.43 |
| FirstLayoutBenchmarks.legacy_natural_width_probe | Layout=Star, CacheLength=0.5 | 1.66 ms | 1.36 ms | 0.82 |
| FirstLayoutBenchmarks.resolved_width_constraint | Layout=Auto, CacheLength=0 | 1.02 ms | 779.72 us | 0.77 |
| FirstLayoutBenchmarks.resolved_width_constraint | Layout=Auto, CacheLength=0.5 | 1.68 ms | 1.35 ms | 0.81 |
| FirstLayoutBenchmarks.resolved_width_constraint | Layout=Fixed, CacheLength=0 | 1.09 ms | 729.30 us | 0.67 |
| FirstLayoutBenchmarks.resolved_width_constraint | Layout=Fixed, CacheLength=0.5 | 1.62 ms | 1.27 ms | 0.79 |
| FirstLayoutBenchmarks.resolved_width_constraint | Layout=Mixed, CacheLength=0 | 901.76 us | 775.56 us | 0.86 |
| FirstLayoutBenchmarks.resolved_width_constraint | Layout=Mixed, CacheLength=0.5 | 1.71 ms | 1.41 ms | 0.83 |
| FirstLayoutBenchmarks.resolved_width_constraint | Layout=Star, CacheLength=0 | 1.05 ms | 763.82 us | 0.73 |
| FirstLayoutBenchmarks.resolved_width_constraint | Layout=Star, CacheLength=0.5 | 1.64 ms | 1.31 ms | 0.80 |
| NeutralSourceBenchmarks.create_source_and_presentation | NeutralSource=false | 5.74 us | 2.62 us | 0.46 |
| NeutralSourceBenchmarks.create_source_and_presentation | NeutralSource=true | 5.71 us | 2.75 us | 0.48 |
| NeutralSourceBenchmarks.expand_collapse_thousand_rows | NeutralSource=false | 7.65 us | 14.59 us | 1.91 |
| NeutralSourceBenchmarks.expand_collapse_thousand_rows | NeutralSource=true | 8.01 us | 14.42 us | 1.80 |
| NeutralSourceBenchmarks.sort_thousand_rows | NeutralSource=false | 710.78 us | 75.42 us | 0.11 |
| NeutralSourceBenchmarks.sort_thousand_rows | NeutralSource=true | 73.00 us | 73.44 us | 1.01 |
| TemplateCellRecyclingBenchmarks.alternating_template_providers |  | 2.35 us | 1.45 us | 0.62 |
| TemplateCellRecyclingBenchmarks.same_template_provider |  | 2.33 us | 1.40 us | 0.60 |
| VirtualizationBenchmarks.collection_insert_remove_burst | NeutralSource=false | 51.13 us | 57.21 us | 1.12 |
| VirtualizationBenchmarks.collection_insert_remove_burst | NeutralSource=true | 54.31 us | 53.93 us | 0.99 |
| VirtualizationBenchmarks.collection_move_burst | NeutralSource=false | 20.99 us | 23.83 us | 1.14 |
| VirtualizationBenchmarks.collection_move_burst | NeutralSource=true | 23.96 us | 23.87 us | 1.00 |
| VirtualizationBenchmarks.create_and_layout_grid | NeutralSource=false | 1.97 ms | 1.06 ms | 0.54 |
| VirtualizationBenchmarks.create_and_layout_grid | NeutralSource=true | 2.08 ms | 1.09 ms | 0.52 |
| VirtualizationBenchmarks.detach_reattach | NeutralSource=false | 888.92 us | 514.66 us | 0.58 |
| VirtualizationBenchmarks.detach_reattach | NeutralSource=true | 885.75 us | 519.40 us | 0.59 |
| VirtualizationBenchmarks.forced_stationary_vertical_layouts | NeutralSource=false | 4.79 us | 4.00 us | 0.83 |
| VirtualizationBenchmarks.forced_stationary_vertical_layouts | NeutralSource=true | 4.68 us | 3.88 us | 0.83 |
| VirtualizationBenchmarks.horizontal_column_scrolls | NeutralSource=false | 176.28 us | 172.49 us | 0.98 |
| VirtualizationBenchmarks.horizontal_column_scrolls | NeutralSource=true | 157.28 us | 176.29 us | 1.12 |
| VirtualizationBenchmarks.horizontal_far_column_scrolls | NeutralSource=false | 162.19 us | 161.84 us | 1.00 |
| VirtualizationBenchmarks.horizontal_far_column_scrolls | NeutralSource=true | 142.61 us | 160.37 us | 1.12 |
| VirtualizationBenchmarks.horizontal_small_scrolls | NeutralSource=false | 20.20 us | 18.10 us | 0.90 |
| VirtualizationBenchmarks.horizontal_small_scrolls | NeutralSource=true | 20.06 us | 17.77 us | 0.89 |
| VirtualizationBenchmarks.horizontal_unbatched_small_scrolls | NeutralSource=false | 40.43 us | 43.44 us | 1.07 |
| VirtualizationBenchmarks.horizontal_unbatched_small_scrolls | NeutralSource=true | 40.44 us | 44.42 us | 1.10 |
| VirtualizationBenchmarks.vertical_buffered_jitter_scrolls | NeutralSource=false | 24.98 us | 24.57 us | 0.98 |
| VirtualizationBenchmarks.vertical_buffered_jitter_scrolls | NeutralSource=true | 24.78 us | 23.61 us | 0.95 |
| VirtualizationBenchmarks.vertical_buffered_small_scrolls | NeutralSource=false | 23.25 us | 21.01 us | 0.90 |
| VirtualizationBenchmarks.vertical_buffered_small_scrolls | NeutralSource=true | 23.29 us | 21.26 us | 0.91 |
| VirtualizationBenchmarks.vertical_jitter_scrolls | NeutralSource=false | 101.20 us | 113.27 us | 1.12 |
| VirtualizationBenchmarks.vertical_jitter_scrolls | NeutralSource=true | 103.42 us | 117.95 us | 1.14 |
| VirtualizationBenchmarks.vertical_row_scrolls | NeutralSource=false | 34.68 us | 32.89 us | 0.95 |
| VirtualizationBenchmarks.vertical_row_scrolls | NeutralSource=true | 34.57 us | 33.56 us | 0.97 |
| VirtualizationBenchmarks.vertical_small_scrolls | NeutralSource=false | 19.75 us | 17.82 us | 0.90 |
| VirtualizationBenchmarks.vertical_small_scrolls | NeutralSource=true | 19.89 us | 18.47 us | 0.93 |
| VirtualizationBenchmarks.vertical_unbatched_small_scrolls | NeutralSource=false | 24.97 us | 22.68 us | 0.91 |
| VirtualizationBenchmarks.vertical_unbatched_small_scrolls | NeutralSource=true | 24.58 us | 21.91 us | 0.89 |

71 benchmarks compared; the port is as fast or faster in 58; geometric mean of port / reference: 0.68.

