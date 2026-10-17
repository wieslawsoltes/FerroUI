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
| `TreeDataGridDemo` and `TreeDataGridCoreDemo` | `treedatagrid-demo` (library: models, view models, pages) and `treedatagrid-demo-desktop` (the application) | `samples/TreeDataGridDemo`, `samples/TreeDataGridDemo.Desktop` |
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
| T1 | not started |
| T2 | not started |
| T3 | not started |
| T4 | not started |
| T5 | not started |
| T6 | not started |
| T7 | not started |
| T8 | not started |

## 6. Performance

To be filled by T8: the reference's suites (`./benchmarks/run-benchmarks.sh`, .NET on this machine) against the port's benches, workload by workload.
