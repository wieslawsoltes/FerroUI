# Changelog

All notable changes to FerroUI are recorded in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html). All crates of the workspace are released together at one version; a version with a pre-release identifier (`-preview.N`) is a preview, which may change any part of the public API. The release process is described in [docs/release.md](docs/release.md).

## [Unreleased]

### Added

- Opt-in application inspection and automation through MCP (#235): `ferroui-inspect` provides weak node IDs, trees, properties and styles, resource lookup, selectors, raw input, automation patterns, PNG screenshots, condition probes, logs and render/layout statistics. `ferroui-mcp` exposes the core over stdio and authenticated loopback Streamable HTTP, with UI dispatcher marshalling and separate input/property-write permissions.
- ControlCatalog's `mcp` feature, native and headless launch modes, a protocol smoke test for both transports, and [MCP/Claude Code setup documentation](docs/mcp.md).

## [0.4.0] - 2026-10-22

The fourth release. It completes the declarative Rust API of `0.4.0-preview.1` and adds the developer tools:
- the declarative API in all its phases ([docs/declarative-ui.md](docs/declarative-ui.md)): element builders and `rsx!`, signals, components that run again with hooks and a reconciler, interoperation with XAML and data templates, styles, resources, transitions and popups as elements, hot reload of the literals of `rsx!`, diagnostics and an inspector, the fewest moves of keyed children, asynchronous data, error boundaries and a router;
- the samples: the 7GUIs in three versions, a gallery, TodoMVC and ReactiveDemo as components;
- the developer tools, `ferroui-diagnostics`, a port of upstream's last open-source version on the WinUI 3 theme, with the TreeDataGrid for the trees and the property grid.

One crate joins, `ferroui-diagnostics`. The additions since the preview are listed below; the known limitations are those of `0.4.0-preview.1`. Within 0.4 the public API is kept; 0.5.0 may change it.

### Added

- **Components that run again** in `ferroui-view` (phase 1 of the declarative API, `docs/declarative-ui.md`, sections 7 and 8):
  - `rerender(..)` and `#[component(rerender)]`: a component whose function runs again when a signal it read changes or its parent runs again; `#[component(rerender, memo)]` skips a run for equal parameters;
  - hooks: `use_hook`, `use_signal`, `use_memo`, `use_effect`, `use_effect_with`; contexts: `provide_context`, `use_context`;
  - a reconciler that patches the mounted controls into the elements a component returns: siblings matched by key and by place, equal values left alone, values that are gone cleared, handlers attached again, controls kept;
  - the 7GUIs tasks as components that run again (`samples/SevenGuis/Hooks`), in the scenario tests with the other ports;
  - `docs/declarative-ui-react.md`, a comparison of the API with React, and the plan of the phases that remain.
- **XAML and templates** in `ferroui-view` (phase 2, `docs/declarative-ui.md`, section 9):
  - `ComponentHost`, a control that shows a component in XAML or any tree of controls; `register_component` names components for its `Component` property; `register_types` registers the types of the crate for markup (`using:FerroUI.View`);
  - `template(|item: T| ..)`, a data template whose items are components, which recycles its hosts through a pool so that a virtualized list patches its items as it scrolls;
  - `use_data_context`, `use_data_context_value` and `use_parent_control`;
  - with the feature `xaml`: `load_xaml`, `load_xaml_uri`, `xaml` and `use_xaml`, XAML in a component;
  - the sample `samples/ViewInterop`: a virtualized directory of 10,000 people with XAML details and a component inside the XAML.
- **The rest of the API** in `ferroui-view` (phase 3, `docs/declarative-ui.md`, section 10):
  - the builders of the TreeDataGrid, the colour picker, the dialogs and the ReactiveUI controls, each behind a feature (`treedatagrid`, `color-picker`, `dialogs`, `reactiveui`), from the same generator;
  - styles: `style("Button.accent").set(..).nested(style("^:pointerover")..)`, `style_of(selector)`, and `.styles([..])` of an element;
  - hooks: `use_parent_property`, `use_theme_variant`, `use_resource`, `use_resource_as`;
  - transitions and animations: `transition`, `transition_eased`, `transitions`, `ms`, `animation(..).key_frame(..)`, `use_animation`;
  - a `Popup` holds its child element, and a property that holds a control of a class (`context_menu`) takes an element;
  - `ferroui-view` exports its type model, so compiled XAML names `ComponentHost`; `samples/ViewInterop` compiles its XAML;
  - the sample `samples/ViewGallery`: styles, the theme variant and resources, a transition, a context menu, a `ColorView` and a `TreeDataGrid`, light and dark.
- **Tools** in `ferroui-view` (phase 4, `docs/declarative-ui.md`, section 11):
  - hot reload of the literals of `rsx!` (feature `hot-reload`): `hot::lit`, `hot::watch`, `hot::poll`, `hot::apply_source`; `launch` watches the sources;
  - diagnostics in debug builds: `Diagnostic` (`MissingKeys`, `DuplicateKey`, `RunawayComponent`, `NeedsRebuild`, `HotReloaded`), `set_diagnostics_handler`; `Element::List` for iterations;
  - the inspector: `mounted_trees()`, `inspector()`, `open_inspector()`, `attach_inspector()` (Shift+F12 in debug builds).
- **Performance** of `ferroui-view` (phase 5, `docs/declarative-ui.md`, section 12):
  - keyed children of panels and items controls reordered with the fewest moves (the longest increasing subsequence), a panel moving controls without taking them out of the visual tree;
  - handlers kept attached with their closures swapped on each run (`Handler`), and values that follow the same signal or memo kept (`PropValue::Follows`);
  - `view_stats()`, the counts of the mounting and the reconciler;
  - benchmarks of the declarative API in `ferroui-benchmarks` (`declarative`);
  - the README gains a section on the declarative API with its features and a comparison with React, and `examples/counter.rs` keeps its example building.
- **Patterns of larger applications** in `ferroui-view` (phase 6, `docs/declarative-ui.md`, section 13):
  - asynchronous data: `use_future(deps, make)`, a hook that runs a future on the executor of the thread and starts it again when its dependencies change, abandoning the one before; its state `AsyncState` (`Pending`, `Ready`, `Failed`); `suspense(state, pending, ready, failed)`; `sleep(duration)` on a timer of the dispatcher;
  - error boundaries: `error_boundary(child, fallback)` shows a fallback when a run of a component below it panics, with `ErrorInfo` (`message`, `reset`), and reports `Diagnostic::ErrorCaught`;
  - navigation modelled on the routing of ReactiveUI: `router(initial, routes)`, `route(pattern, view)` with `:name` parameters and `*`, `Router` (`navigate`, `navigate_back`, `navigate_and_reset`, `current`, `can_go_back`, `stack`), `use_router`, `try_use_router`, `RouteMatch`;
  - the samples `samples/TodoMvc` (TodoMVC, its filters as routes) and `samples/ReactiveDemoComponents` (the crate browser of ReactiveDemo as components), in the WinUI 3 theme.
- **Developer tools** (`ferroui-diagnostics`, a new published crate): the port of upstream's developer tools at their last open-source version, opened with F12 on a window or an application:
  - the logical and visual trees with a filter, the details of the selected control (its properties with their priorities and an editor for each, the value frames with their setters and resources, the pseudo-classes, the layout explorer), the routed events with their routes, the hot keys, and screenshots of a control;
  - the property grid is the TreeDataGrid of the port, grouped and sortable, in place of upstream's DataGrid;
  - the window has the WinUI 3 theme in its compact density, in place of upstream's Simple theme, with compact rows; the logical and visual trees and the tree of the routed events are TreeDataGrids too, in place of tree views: their rows are virtualized, and selecting a node (Ctrl+Shift over a control, a link of an event chain) expands its ancestors and scrolls to its row even when it was not shown;
  - a combined tree, the first tab and the default view: the logical tree with the parts of the template of each element, greyed out;
  - a header in the title bar with a button that collapses the tree pane and a command bar in place of the menus, and the pages in the tab view of WinUI 3;
  - one design for every view (`DevToolsStyles.xaml`): consistent spacing, section headers, thin dividers, a status bar, and a property grid whose editors share one compact form, with read-only values shown as text and brushes as outlined swatches;
  - Ctrl+Shift selects the control under the pointer; selecting a node shows its properties without a delay and the property grid scrolls smoothly (its editors are kept for the recycled rows and the view of the details is reused); the colours of the TreeDataGrids, the colour picker and the layout box model follow the light and dark variants of the WinUI 3 theme; the tree pane is set apart from the details, a toggle hides the side pane of the details, and a page with nothing to show says what to do;
  - `MainViewModel::register_page` adds pages of other crates to the tools;
  - the ControlCatalog attaches them in debug builds, and `FERROUI_SMOKE_DEVTOOLS=<directory>` runs them without a person and writes pictures of their window;
  - the facade has them as the feature `diagnostics`.
- **Text**: `TextBlock.TextLineBounds` (`Full`, `TrimToCapHeight`, `TrimToBaseline`, `Tight`), after WinUI's: the box of each line can be trimmed to the cap height of its font and to its baseline, so that centred text is centred by its capitals whatever the font; `FontMetrics::cap_height` reads the cap height of a font. The WinUI 3 theme bounds the labels of command bar buttons beside their icons that way.

### Changed

- **WinUI 3 theme**: the controls whose WinUI style leaves the content alignments unset centre their content, as WinUI's defaults do (the placeholder of a combo box, the content of stretched buttons, the fields of the date and time pickers); a single-line text box centres its line, and a command bar button with its label on the right centres the icon and the label, which with the fonts of macOS and Linux sat above the middle (`docs/porting/winui-theme/DEVIATIONS.md`, G-27 to G-29).
- **macOS**: a window that extends into a title bar taller than the system's (`ExtendClientAreaTitleBarHeightHint`) centres the close, minimise and zoom buttons in it vertically.
- **`#[component(rerender)]` memoizes by default** a component whose parameters are `PartialEq + Clone`; `#[component(rerender, always)]` runs it with its parent every time, and `memo` makes comparable parameters required.
- **Markup compiler**: a setter of a style class in a style (`<Setter Property="Classes.name" .../>`) compiles to Rust; it was refused (`FRN3000`).
- **Markup**: `Binding` returns itself from `BindingBase::as_any`, as the other bindings do; `Window.Close()` can be called from markup.
- **`ferroui-view`**: mounting is built on instances that the reconciler patches (`mount.rs`). A routed handler attached by an element is detached when the owner of the element ends. A dynamic list may give other dynamic lists.

## [0.4.0-preview.1] - 2026-10-20

The first preview of 0.4. It adds:
- a declarative Rust API for user interfaces without XAML: element builders for every control, the `rsx!` macro and `#[component]`, and signals that the properties of controls follow ([docs/declarative-ui.md](docs/declarative-ui.md), phase 0 of issue #237);
- the 7GUIs as samples, each task in two versions, one with the builders and one with the macros;
- the performance work on the Vello backend: in software it is now as fast as Skia or faster on macOS, Linux and Windows, as it is with ANGLE on the Windows virtual machine, and on Metal it is faster for partial redraws and effects ([docs/porting/vello-performance.md](docs/porting/vello-performance.md)).

Two crates join: `ferroui-view` and `ferroui-view-macros`. As a preview, it may change any part of the public API.

### Added

- **`ferroui-view`**: the declarative API, built beside the controls and adding nothing to them:
  - an element tree that is data (the class, a key, a name, classes, property values, handlers and children), mounted into controls between `begin_init` and `end_init`;
  - element builders with chained setters for every control and property of `ferroui-controls`, generated from their sources by `scripts/generate_view_builders.py` (a check in the continuous integration keeps them current); a setter takes a value, a signal, a memo or a closure, with conversions of the common types;
  - a reactive runtime of signals, memos, effects and owners with `Copy` handles, which the properties of controls follow;
  - children that follow state: keyed lists (`for_each`), conditions (`show`, `show_else`) and `dynamic`;
  - `launch`, `launch_with`, `launch_themed` and `open_window` (feature `desktop`).
- **`ferroui-view-macros`**: `rsx!`, with attributes, shorthand, attached properties (`Grid.row`), method-call attributes, `if` and `for`; and `#[component]`, which makes a function a component with its builder.
- **The 7GUIs samples** (`samples/SevenGuis`): counter, temperature converter, flight booker, timer, CRUD, circle drawer and cells, each a crate of its own in a builders version and a macros version, on the WinUI 3 theme, with a shared crate for the logic and the state and 14 scenario tests. The screenshots and the code of each are in [docs/declarative-ui.md](docs/declarative-ui.md).
- **Vello performance**, rounds 1 to 5 and an experimental round (#263 to #268), on the published Vello crates without patches:
  - rectangles and rectangle clips drawn by the renderers' own paths; text of the hybrid mode from its glyph atlas;
  - the CPU mode keeps its render contexts, renders kept targets and windows only where they changed, renders large scenes on the thread pool of `vello_cpu`, and keeps the pictures of box shadows;
  - the GPU mode draws its filters on the device and renders kept targets only where they are cleared;
  - on Linux and Windows a device of the backend's own (Vulkan or OpenGL through EGL; Direct3D 12), and the hybrid mode on the OpenGL context of the platform (ANGLE on Windows);
  - a depth buffer for the hybrid renderer, so opaque content is drawn front to back; the Metal queues ordered by a shared event instead of a wait on the processor;
  - `FERROUI_FRAME_TIMES`, and benchmark and profiling scripts (`scripts/vello-perf`).
- **Draw-list fragments**: `IDrawingContextImplWithFragments` in `ferroui-base`, an optional extension of the drawing context (`None` by default), lets a backend keep what it made of the commands of a draw-list visual and draw it again in later frames until the commands or their resources change. The GPU mode of Vello uses it. The deviation is recorded in `docs/porting/DEVIATIONS.md`.

### Known limitations

- **No reconciler yet.** A component runs once; what changes later is changed through signals and the children that follow state. Running a component again and comparing its element trees is the next phase ([docs/declarative-ui.md](docs/declarative-ui.md), section 8).
- **Builders for the controls of `ferroui-controls` only.** The TreeDataGrid, the colour picker and other crates of controls are used through `Element::Existing` until the generator covers them.
- **Vello stays optional.** Skia is the default backend. On a real GPU, a full redraw (scrolling, a page drawn again) is still slower with Vello than with Skia. Vulkan and Direct3D 12 were tested in software only, not on hardware.
- The limitations of 0.3.0 below still apply.

## [0.3.0] - 2026-10-19

The third release. It is `0.3.0-preview.1` as a stable release: the reactive foundation for applications written in the MVVM pattern, with nothing changed since the preview. It contains:
- ports of Rx.NET, DynamicData, Splat and ReactiveUI, with ReactiveUI.Binding and ReactiveUI.Testing;
- their integration with the controls;
- the samples ReactiveDemo (a crate browser) and ReactiveUIExample (with a DynamicData page);
- the guide [docs/mvvm.md](docs/mvvm.md).

The additions and the known limitations are those of `0.3.0-preview.1` below. Within 0.3 the public API is kept; 0.4.0 may change it.

## [0.3.0-preview.1] - 2026-10-19

The first preview of 0.3. It adds the reactive foundation for applications written in the MVVM pattern:
- ports of Rx.NET (System.Reactive and Microsoft.Reactive.Testing), DynamicData, Splat and ReactiveUI (with the run time of ReactiveUI.Binding and ReactiveUI.Testing);
- their integration with the controls;
- two samples and a guide ([docs/mvvm.md](docs/mvvm.md)).

None of the new libraries depends on the user interface except the integration. Eight crates join: `ferroui-system`, `ferroui-reactive`, `ferroui-reactive-testing`, `ferroui-dynamic-data`, `ferroui-splat`, `ferroui-reactiveui`, `ferroui-reactiveui-testing` and `ferroui-reactiveui-controls`. As a preview, it may change any part of the public API.

### Added

- **`ferroui-splat`**: a port of Splat (phase 6 of `docs/porting/reactive.md`): the service locator (`AppLocator`) with the modern and generic-first dependency resolvers, logging (`IEnableLogger`, `IFullLogger`, the log managers and wrapping loggers, `LogHost`), mode detection, feature usage tracking and the application builder (`AppBuilder`, `IModule`), with 864 ported tests.
- **`ferroui-system`**: the parts of the .NET base class library that FerroUI and its reusable libraries share, in a crate with no dependency on the framework: `ferro_thread_local!` with its table for Android, the multicast events (`Event`, `HandlerList`), `INotifyPropertyChanged`, `INotifyCollectionChanged` with the arguments of a change (`NotifyCollectionChangedAction`, `NotifyCollectionChangedEventArgs`, `CollectionChange`), and `CancellationToken` with its source and registration. It is the first step of the reactive foundation (`docs/porting/reactive.md`). `ferroui-base` depends on it and re-exports every item at the path it had there, so no code changes.
- **`ferroui-reactive`**: the core of a port of Rx.NET's System.Reactive (`docs/porting/reactive.md`, phase 2): the disposables, the schedulers (immediate, current thread, virtual time, historical), the subjects (`Subject`, `BehaviorSubject`, `AsyncSubject`, `ReplaySubject` with its bounded, unbounded and windowed forms, `ConnectableObservable`), `Notification`, the observers, `Observable::create` and the first operators (`select`, `where_`), single-threaded like the object model of FerroUI. The interfaces it implements (`IObservable`, `IObserver`, `IDisposable`) moved to `ferroui-system`, where the framework finds them too.
- **`ferroui-reactive-testing`**: the port of Microsoft.Reactive.Testing (`TestScheduler`, hot and cold test observables, recorded notifications and subscriptions, `assert_messages!`). The unit tests of Rx.NET for the core are ported in `tests/FerroUI.Reactive.Tests`.
- **`ferroui-reactive` operators** (phase 3): the query operators of System.Reactive and the experimental ones of `ObservableEx`, the pull adapters (`collect`, `chunkify`, `most_recent`), `Subscribe` with a `CancellationToken`, `DisableOptimizations`, `ObserverBase` and `ObservableBase`; 2,453 ported unit tests. The blocking operators (`Wait`, `First`, `ToEnumerable`, ...) are left out on purpose: they would deadlock the only thread. The async forms come with phase 4.
- **Tasks and asynchronous Rx** (phase 4): `ferroui-system` gains `Task<T>` and `TaskCompletionSource<T>`, the running of futures on their thread (`spawn_local`, a thread poster for wakes from other threads, `block_on`, `run` for background work), `CancellationToken::register_local`, `SynchronizationContext` and `IProgress<T>`. `ferroui-reactive` gains the asynchronous forms of Rx over them: `from_async`, `start_async`, `to_task`, `run_async` and `get_awaiter`, `for_each_async`, the task forms of `select_many`, `create`, `defer` and `using`, `schedule_async` with `sleep` and `yield_`, `to_async` for the arities 0 to 16, and the `SynchronizationContext` forms of `observe_on` and `subscribe_on`.
- **`ferroui-dynamic-data`**: a port of DynamicData (phase 5 of `docs/porting/reactive.md`): change sets of keyed caches and of lists, `SourceCache` and `SourceList` with their nested edits, the cache and list operators (transforms with their asynchronous forms, filters, sorting, paging and virtualisation, grouping, joins, `And`/`Or`/`Xor`/`Except` and `Switch`, the lifecycle of items, buffering and expiry), the binding to observable collections (`ObservableCollectionExtended`, `Bind`, `SortAndBind`), aggregation (`Count`, `Sum`, `Avg`, `Min`, `Max`, `StdDev`) and `ObservableChangeSet`, with 2,193 ported tests. `ferroui-system` gains `Dictionary` with the order of enumeration of .NET, `ObservableCollection` and typed property paths (`PropertyPath`, `prop!`), the counterpart of upstream's expressions.
- **`ferroui-reactiveui`** and **`ferroui-reactiveui-testing`**: a port of ReactiveUI in its System.Reactive flavour (phase 7 of `docs/porting/reactive.md`). It covers `ReactiveObject` and `ReactiveRecord` with the macros `reactive_object!` and `reactive_properties!` (the counterpart of the `[Reactive]` source generator), `ReactiveCommand`, interactions, the property observation of ReactiveUI.Binding (`when_any_value` and its family over typed property paths, `ObservableAsPropertyHelper` and `to_property`), the bindings (`bind`, `one_way_bind`, `bind_to`, `bind_command`, `bind_interaction`) with their type converters, views and view location, activation (`when_activated`), routing, the message bus, suspension and AutoPersist, `ReactiveProperty`, and the ReactiveUI builder. ReactiveUI.Testing is ported on the virtual-time `TestScheduler`. The crates have 2,207 ported tests. `ferroui-system` gains `INotifyPropertyChanging`, `INotifyDataErrorInfo`, `BackingField` and the type-erased steps of property paths.
- **`ferroui-reactiveui-controls`**: the integration of ReactiveUI with FerroUI (phase 8 of `docs/porting/reactive.md`):
  - `FerroScheduler` on the dispatcher, whose synchronization context becomes the UI thread's;
  - the property observation of FerroUI objects (`ferro_prop!`), the view-thread invoker, and command binding to controls through `ReactiveCommandAdapter`;
  - the activation of views on loading and unloading;
  - `ReactiveUserControl` and `ReactiveWindow` (their view model follows `DataContext`), `RoutedViewHost` and `ViewModelViewHost`;
  - the automatic item template of `ItemsControl`s, `AutoSuspendHelper`, view registration, and `use_reactive_ui` on the application builder;
  - 166 ported tests.
- **The ReactiveUI samples**, both on the WinUI 3 theme (`ferroui-themes-winui`):
  - `samples/ReactiveDemo` (`cargo run -p reactive-demo`): the port of `ReactiveDemo`, the getting-started sample of ReactiveUI (ReactiveUI.Samples), a crates.io browser (the reference browses NuGet) with a throttled search, a reactive command and views located for their view models; with `--no-default-features` it searches an in-memory catalogue of crates instead of crates.io; 28 tests;
  - `samples/ReactiveUIExample` (`cargo run -p reactiveui-example`): the port of the example application of the ReactiveUI integration (routing, view model view hosts, live metrics, a command lab with interactions, `AutoSuspendHelper`), with a DynamicData page that the reference does not have; 75 tests.
- **Items sources from the reactive libraries**: the `ObservableCollection` of `ferroui-system` is an items source of the controls, as in .NET, so a collection that DynamicData's `sort_and_bind` or `bind` fills (`ObservableCollectionExtended::collection()`) is shown by a `ListBox` or an `ItemsControl` directly.
- **Port tracking for several references**: the port-status tooling tracks Rx.NET beside the framework (`docs/porting/tracking/reactive/`).

### Known limitations

- **One thread.** The observables, view models and subscriptions are single-threaded, as FerroUI's object model is. Work on another thread goes through `ferroui_system::threading::tasks::run` and comes back to the thread that started it; there is no thread-pool scheduler.
- **No reflection or expression trees.** Property paths (`prop!`) stand for the expressions of .NET. The `*Unsafe` overloads, `WhenAnyDynamic`, assembly scanning and the type-based APIs of Splat are not ported. The adapters of Splat and ReactiveUI to other dependency-injection containers are not ported either; a port of Microsoft.Extensions.DependencyInjection is tracked separately.
- **Types that wait for a later preview.** The binding converters of `DateTime`, `DateTimeOffset`, `DateOnly`, `TimeOnly`, `Uri` and `Decimal`, and the Rx and DynamicData tests that use `Decimal`, wait until those types move from `ferroui-base` to `ferroui-system`.
- Every difference from the reference libraries is recorded in `docs/porting/DEVIATIONS.md`, sections Reactive, DynamicData, Splat, ReactiveUI, ReactiveUI controls and Samples.
- The limitations of 0.2.0 below still apply.

## [0.2.0] - 2026-10-18

The second release. It adds the TreeDataGrid: the port of TreeDataGrid, its demo and its benchmarks, with the pages of the demo in the ControlCatalog. Scrolling a TreeDataGrid led to changes of the framework that make translating a visual into the coordinates of an ancestor, and the layout queues, faster for every control. The port follows upstream at 66b7273b9a, as 0.1.0 did: upstream has no later commits.

### Added

- **TreeDataGrid**: the port of TreeDataGrid (<https://github.com/wieslawsoltes/TreeDataGrid>) as two new crates. `ferroui-treedatagrid-core` is the framework-neutral data layer: flat and hierarchical sources, value, text, check box, template and hierarchical expander columns, sorting, expansion, filtering, row and cell selection. `ferroui-controls-treedatagrid` is the control: a virtualizing view of rows and columns with star and auto column layout, resizing and sorting from the headers, row and cell selection, editing, row drag and drop, automation peers, the Generic and Fluent themes, and declarative columns in markup (`<TreeDataGrid ItemsSource=".."><TreeDataGridTextColumn Binding=".." /></TreeDataGrid>`). The tests of the reference are ported (the 198 tests of the core and the 497 of the control pass), and `docs/porting/treedatagrid.md` describes the port.
- **The TreeDataGrid demo** (`samples/TreeDataGridDemo`, `cargo run -p treedatagrid-demo`): every page of the reference demo (countries with filtering and editing, a file tree, people declared in markup, drag and drop, the articles of the day from Wikipedia, finding a displayed row, rows of different heights, template column reuse) and the core demo. The ControlCatalog shows the same pages in its new page TreeDataGrid of the section Collections & Data.
- **The TreeDataGrid benchmarks** (`tests/FerroUI.TreeDataGrid.Benchmarks`): the benchmark suite of the reference on the harness of the workspace. Run on one machine against the reference's suite, the port is as fast or faster in all 71 benchmarks, with a geometric mean of the ratio port / reference of 0.56 (`docs/porting/treedatagrid.md`, section 6).
- **The feature `treedatagrid` of the facade crate `ferroui`**: the modules `controls_treedatagrid` and `treedatagrid_core`.

### Changed

- **The effective viewports of a layout pass** (`EffectiveViewportChanged`) share the viewports of the ancestors of their listeners while the geometry of the visuals is unchanged, where each listener walked from the root down to itself; the viewports are the same (`docs/porting/DEVIATIONS.md`, Layout). Data context changes compare the inheritance parent of a child without taking a handle to it.
- **Fewer allocations when creating and rebinding controls**: a list of event handlers (`HandlerList`) allocates on its first handler and keeps up to two handlers in that one allocation, the change events of collections take their tokens from a counter of the process (`HandlerList::with_ordered_ids`, `Event::with_ordered_ids`), template bindings subscribe to property changes by token (`FerroObject::add_property_changed_handler` and `remove_property_changed_handler`), and a reevaluation of a value store and the effective viewports of a layout pass reuse their lists. A first layout of a TreeDataGrid allocates a third less. `HandlerList::snapshot` and `Event::snapshot` return a `HandlerSnapshot`, which dereferences to the slice of the handlers.
- **TreeDataGrid**: an observer of a cell binding (`ICellBinding::subscribe`) takes the value by value, and `ITextCellOptions` adds `has_default_string_format`; a text cell no longer copies its text and its value again.
- **Translating a visual into the coordinates of one of its ancestors** (`Visual::transform_to_visual`, `translate_point`) walks up from the visual once, where it walked both visuals up to the root; the transforms of the visual tree and the effective viewport read the render transform only of visuals that have one; the layout queues and the scroll anchors of a scroll presenter keep their entries in maps with the Fx hasher. The results are the same (`docs/porting/DEVIATIONS.md`, Visual tree); scrolling a list of many realized rows spent most of its time there.
- **The day items of the calendar of the WinUI 3 theme** have the padding 0,0,0,2, which places the text of the days where WinUI 3 draws it at a scaling of 2 (CDB-10).

### Fixed

- **Path icons of the WinUI 3 theme are scaled into their boxes again**: a style meant for the icons of menu items (`MenuItem > ^` in the control theme of `PathIcon`) matched every path icon, because the nesting selector stands for the whole selector of the theme and what precedes it is dropped (as upstream). Every path icon got the unscaled template, so the icons of the ControlCatalog (its navigation, the cards of its home page, the logo) were drawn at the size of their geometry and cut. The style is now `MenuItem > PathIcon`, a style of the theme.

### Known limitations

- **The Wikipedia page of the TreeDataGrid demo** loads only with the demo's feature `online` (the default of the demo, not used by the ControlCatalog), and the names of its drag and drop page are not the reference's (`docs/porting/DEVIATIONS.md`, TreeDataGrid).
- The limitations of 0.1.0 below still apply: verification on continuous integration, virtual machines, the iOS simulator and the Android emulator rather than real devices; the experimental Vello backend; the open parts of XAML; the browser scripts outside the packages.

## [0.1.0] - 2026-10-17

The first release that is not a preview. It is the fourth preview with the work that followed it: the port at upstream 66b7273b9a, the WinUI 3 theme placing text on the baseline of native WinUI 3, and touch scrolling that coasts after a flick, in the browser on a phone too. The crates of 0.1.0 depend on one another at exactly that version, as the previews did; within 0.1 a later release keeps the public API, and the work towards 0.2.0 is published as previews of it (`0.2.0-preview.N`).

### Added

- **`TextBlock.LayoutRoundingPlacement`** (`TextLayoutRoundingPlacement`: `Below`, the default, or `Above`), an inherited attached property of `TextBlock` and a FerroUI addition: where a text block puts the height that layout rounding adds to it. With `Above` the text, the selection and hit testing of `SelectableTextBlock`, the access key underline of `AccessText` and the embedded controls move down by that height, as WinUI's TextBlock places its text (`docs/porting/text-rendering.md`, section 6).

### Changed

- **Upstream 66b7273b9a.** The port follows upstream's 49 commits since 17350180 (`docs/porting/CONTINUATION.md`, hand-over of 2026-10-17): among them the immutable stream geometry builder (`IStreamGeometryBuilder`; the mutable stream geometry implementation is removed, its interfaces deprecated), text fixes (bidi after a reset and the run index of the reorderer, the ellipsis of wrapped text cut by a height or a line limit, the caret of centred text, empty text ranges, the text layout released on detach), styling and focus fixes, Wayland popup positions and application id, the 120 Hz display link on iOS, the headless window positioning options, AT-SPI headings, lists and children changes, and the rewritten pointers page of the ControlCatalog.
- **The WinUI 3 theme places text as WinUI 3 does**: its top-levels set `TextBlock.LayoutRoundingPlacement` to `Above`, which moves the text of the theme by up to one device pixel down to the baseline of native WinUI 3 (measured at 200 %: from 0.99 px above to 0.01 px). The Fluent and Simple themes keep the reference's placement and their pictures are unchanged.
- **The tabs of the WinUI 3 theme are as wide as each other**, as the tabs of WinUI's TabView in its default `TabWidthMode="Equal"`: the strip of a TabControl at the top or the bottom gives every tab the width of the strip divided by their number, between `TabViewItemMinWidth` (100) and `TabViewItemMaxWidth` (240), shrinks them as the strip narrows and scrolls them, with its scroll buttons, when they do not fit at the minimum width; a tab was as wide as its header. The width comes from `TabViewEqualWidthPanel`, a new primitive of the theme crate (`FerroUI.Themes.WinUI.Primitives`) and the items panel of the strip; the tabs at the left and the right stay stacked, and TabbedPage's top pane keeps items as wide as their content.
- **The development variable `FERROUI_TEXT_BASELINE_ROUNDING` is removed**; the property above replaces it, and a calibration run of the WinUI theme gallery sets the property with its key `baseline` (`below`, `above`, `theme`).

### Fixed

- **A touch flick scrolls on after the finger is lifted** (every touch platform, and the browser on a phone, where a swipe moved the content only as far as the finger): the inertia of the scroll gesture started its clock with its first frame, which therefore scrolled by nothing, and the scroll viewer, which chains scrolling by default, took that for the end of the content and ended the gesture. The inertia is timed from the release, as upstream; measured in Chrome with touch input at a phone's viewport, a flick of 200 px now scrolls the home page of the ControlCatalog by more than 630 px, where it scrolled by 195 px.
- **A touch flick scrolls on in a browser whose page draws slowly** (the page without threads on a slow phone): a pointer event of the browser is stamped with the time the browser raised it rather than the time the view handled it. The view handles input between the frames it draws, and a frame longer than 40 ms made a finger lifted while it moved look as if it had rested, so the scroll gesture got no speed to coast with.
- **Touching a text box on a phone no longer zooms the page in the browser**, which then could not be zoomed out after the keyboard hid: the hidden input element of the text input method has text of 16 px, the size mobile browsers do not zoom into (it had the default 13.33 px); the ControlCatalog page and the pages of the browser examples ask Chrome on Android to lay the view out in what the keyboard leaves (`interactive-widget=resizes-content`), and an offset a mobile browser leaves a page that cannot scroll at after the keyboard hides is undone.
- **A window follows a change of its text rendering from the next frame** (Skia backend): the layer the composition target keeps for a window whose frames are not kept (every GPU swap chain) kept the text contrast, gamma and subpixel order of the frame it was created in until the window was resized; it is now created again when they change, so a change of the system's ClearType settings, or a calibration override, reaches the window at once.
- **The WinUI 3 theme after the last comparison with native WinUI 3** (`docs/porting/winui-theme/DEVIATIONS.md`): the calendar is as tall as WinUI's, with its days where WinUI draws them (CDB-10); the text of CalendarDatePicker starts where WinUI's does and its calendar glyph has the size and the shape of the native one (CDP-2, CDP-3); the shadows of menus, context menus, flyouts, tool tips and the date and time picker flyouts are drawn, which the presenters clipped away as templated controls (G-24); the path icons of menu items are as large as WinUI's (12 px of a 24-unit geometry where they were 16, PI-3); the elements of the templates that draw a background and a border draw the background inside the border, WinUI's default, so that translucent strokes such as the borders of the notification cards are drawn over the page (G-25).
- **The text calibration page of the WinUI theme gallery** has the layout of its native twin: its grid is aligned to the top, as tall as its rows, where the stretched grid gave each row one more device pixel under layout rounding and moved the lower rows by up to 11 px at 200 %.

### Known limitations

- **Verification.** Every platform was verified in continuous integration, in virtual machines, in the iOS simulator and on the Android emulator; not on a range of real devices.
- **The Vello backend** is experimental (`docs/porting/vello-backend.md`, section 13).
- **XAML**: hot reload and parts of the build integration are open.
- **The browser platform**: the scripts that assemble a site are in the repository and not in a package (`docs/release.md`, section 6.1).

## [0.1.0-preview.4] - 2026-10-17

The fourth preview. It adds a second complete theme, the WinUI 3 theme of microsoft-ui-xaml, validated against native WinUI 3 by a gallery, makes it the ControlCatalog's first theme, and draws text as sharp and as dark as the native text of the platforms.

### Added

- **The WinUI 3 theme.** A second theme next to the Fluent theme, which is unchanged: the crate `ferroui-themes-winui` (`WinUITheme`), ported from the WinUI 3 theme of microsoft-ui-xaml (MIT). Every control has a WinUI 3 template, in light, dark and high contrast, with the compact density, the system accent colours and WinUI's popup shadows and scroll indicators. Every difference from WinUI is recorded with its reason in `docs/porting/winui-theme/DEVIATIONS.md`; `docs/porting/winui-theme/PROGRESS.md` tracks the port. The facade crate has the feature `winui-theme`, and the ControlCatalog offers the theme as a third choice (Settings, or `FERROUI_CATALOG_THEME=winui`).
- **The WinUI theme gallery** (`samples/WinUIThemeGallery`): a page per control family, a capture mode, and on Windows a host that compares each page with the same page of native WinUI 3 controls; it runs in the Windows job of CI (`docs/porting/winui-theme-gallery.md`).
- **Accent shades of the system.** The platform colour values can carry the six accent shades of the system (`PlatformColorValues::accent_shades`); the Windows backend reports them from `UISettings`, as WinUI reads them.

### Changed

- **Text rendering defaults.** Text without an explicit rendering mode is drawn with greyscale anti-aliasing, as WinUI 3, macOS and the browsers draw it, and with subpixel (ClearType) anti-aliasing only where the platform reports it on for the display, the subpixel order is known, the surface is opaque and not composited, and the render scaling is below 1.5. The Windows backend reports ClearType, its subpixel order and the contrast and gamma of DirectWrite; the X11 backend the subpixel order of `Xft.rgba`. Before, every surface asked for subpixel text with an assumed RGB order and Skia's default contrast, which drew colour fringes and lighter text than native (`docs/porting/text-rendering.md`; `RenderOptions.TextRenderingMode` still selects subpixel text explicitly).
- **The ControlCatalog starts in the WinUI 3 theme**; the Fluent and Simple themes stay on its settings page (`FERROUI_CATALOG_THEME=fluent` or `simple`).

## [0.1.0-preview.3] - 2026-10-16

The third preview. It completes the port of upstream's samples and tests: every applicable sample, every applicable unit test and the integration tests are ported, and the defects they found are fixed.

### Added

- **The GpuInterop sample** (`samples/GpuInterop`, not published): drawing with a graphics API of the platform into a drawing surface of the compositor through its GPU interop, as the upstream sample does. The Vulkan demo (the teapot, over `ash`) shares its images as opaque handles with semaphores (Linux, Windows), Direct3D 11 textures (Windows) or IOSurfaces with a Metal shared event (macOS, MoltenVK); the Direct3D 11 demo (`--d3d`, Windows) shares textures by their DXGI handles with keyed mutexes; the dma-buf demo (`--dmabuf`, Linux) hands Linux dma-bufs to the EGL compositor. `--smoke` checks that the frames of the GPU API are in the snapshots of the compositor. Verified on macOS against the Metal and the OpenGL compositor; the Windows and Linux paths are compiled by CI and not run yet.
- **Samples**: `SafeAreaDemo` (the safe area, the system bars and the input pane of a top-level) with its desktop, Android and iOS hosts; `XEmbedSample` (a page of the catalog in a socket of a GTK 3 window through XEmbed, Linux); `PlatformSanityChecks` (the threading of the X11 platform, Linux); `SingleProjectSandbox` (one package with the application and its desktop, macOS, Android and iOS entry points). `scripts/android/apk.sh` builds the Android hosts (`safearea`, `sandbox`). The small samples take `FERROUI_SMOKE_SCREENSHOT=<file.png>`, a picture of the main window drawn by the framework.
- **Upstream's integration tests** (`tests/FerroUI.IntegrationTests.Appium`, not published): the 117 tests of upstream's Appium project, run against the IntegrationTestApp sample by the port's own automation layer over UI Automation and input injection on Windows and the accessibility API and posted input events on macOS (no Appium or WebDriver server). They move the pointer and type, so they run only with `FERROUI_AUTOMATION_TESTS=1`. In the Windows 11 virtual machine 183 of 185 runnable cases pass; on macOS 26, 130 of 151; what remains is recorded in `docs/porting/integration-tests.md`, section 9.
- **The rest of upstream's tests**: the 21 benchmarks not ported before, the remaining XamlX parser tests, the designer support test of the Win32 mode (`tests/FerroUI.DesignerSupport.Tests`, not published) and a property-system test. `docs/porting/TESTS.md` (generated by `scripts/port-status/tests_audit.py`) tracks every upstream test: all 7245 that apply to Rust are ported, 147 of them waived with a reason.

### Changed

- **The GPU interop of a compositor is asynchronous**, as upstream's `ValueTask`: `Compositor::try_get_composition_gpu_interop`, `try_get_render_interface_feature` and `IRenderer::try_get_render_interface_feature` return a future; the first request to a compositor confined to its render thread is answered (before, it came back empty). `OpenGlCompositionInterop::try_create_compatible_gl_context` is replaced by `try_create_compatible_gl_context_async`. A breaking change of these signatures.
- **WinUI processes end as a .NET process does**: `ferroui_winui::end_process` flushes the output and lets the C runtime end the process from a thread of its own, so the values of the UI thread are not destroyed inside `ExitProcess` (the process did not always end on a hosted runner).

### Fixed

- **Tracking of the samples**: `TRACKING.md` lists every ported sample as a member of the workspace (`WinUIEmbedSample` was listed as not created because the documents had not been regenerated), and a sample that is not applicable with the state `docs/porting/data/samples.toml` records (the catalog heads for Mac Catalyst and tvOS, which have no sources).
- **A control behind a subclass or interface handle is shown as a control**: `ContentPresenter` (and `RelativePanel`, `ItemsControl`, `SelectingItemsControl`) recognised only a value boxed as exactly a control; a binding from a property typed as a subclass or an interface showed the control as text.
- **macOS: centred windows**: the frame size of a window not shown yet starts as unknown, as upstream; windows centred on the screen or their owner were placed half their size away.
- **Automation peers of a removed control**: a peer whose control is gone has no parent (an application panicked when UI Automation asked for it).
- **Markup classes without a declared constructor** get a non-public one in the run-time and the build-time type systems, as upstream; the previewer's Win32 mode starts the Win32 platform.
- **WinUI**: the default control resources (`XamlControlsResources`) are merged at the start of `OnLaunched`.

## [0.1.0-preview.2] - 2026-10-16

The second preview.

It adds the facade crate `ferroui`, which makes FerroUI one dependency, and fills the six names the first preview reserved: the Wayland and Linux framebuffer platforms, the Vulkan contracts, the WinUI integration and the VNC server of the headless platform. It completes the Windows platform (the composition modes, WGL, the input method, UI Automation, Vulkan), adds OpenGL on macOS and iOS, and builds Android with stable Rust. It is a preview: the public API may change in any part.

### Added

- **`ferroui`**: the facade crate replaces its placeholder. One dependency (`ferroui = "=0.1.0-preview.2"`) that re-exports the common crates under short names (`ferroui::base`, `ferroui::controls`, `ferroui::markup`, and per feature `desktop`, `themes_fluent`, `themes_simple`, `markup_xaml`, `dialogs`, `browser`, `vello`, `headless`, ...), with `ferroui::prelude` for the types of a minimal application. Features: `desktop` and `fluent` by default; `simple`, `xaml`, `color-picker`, `dialogs`, `browser`, `vello`, `vello-hybrid`, `vello-gpu`, `headless`, `opengl`, `metal` and `vulkan`. An application that compiles its markup still names the crates the compiled code calls. Example: `cargo run -p ferroui --example hello`.
- **`ferroui-wayland`**: the Wayland platform replaces its placeholder. Opt-in, as upstream (the platform detection still chooses X11; `use_wayland` or `use_wayland_with_fallback` on the application builder): the connection on a worker thread, outputs, toplevels over xdg-shell with server-side decorations or decorations drawn by the framework, popups over xdg_popup, the frame-callback render timer, rendering through shared memory buffers and EGL, pointer, keyboard (xkbcommon), touch and text input (text-input-unstable-v3), clipboard and drag and drop. Verified under sway on the headless backend of wlroots in CI and in a virtual machine, and under the headless Mutter of GNOME Shell. Also the dmabuf swap chain (`use_dmabuf_swapchain`: buffers allocated with GBM on the compositor's render node and handed over through `zwp_linux_dmabuf_v1`, with `libgbm` and `libdrm` opened at run time; without the protocol the backend renders through a `wl_egl_window` and says so) and a display another library opened (`WaylandWorker::start_with_foreign_display`). The swap chain itself runs only where the compositor has a render node, which the CI runners do not have.
- **VNC server of the headless platform** (`ferroui-headless-vnc`, until now a reserved name): an application runs on the headless platform and is seen and driven through a VNC viewer (`HeadlessVncPlatformExtensions::start_with_headless_vnc_platform`, optionally with a password). The RFB server is a port of the parts of the .NET library RemoteViewing that the original uses (BSD-2-Clause): raw and Tight encodings, VNC authentication, desktop resize requests. The ControlCatalog desktop sample takes `--vnc` (port 5901). See `docs/porting/headless-vnc.md`; verified with a client of the tests, not with real VNC viewers.
- **`ferroui-linuxframebuffer`**: the Linux framebuffer platform replaces its placeholder. An application runs without a window system as one full-screen view on a framebuffer device (`start_linux_fb_dev`), rendered in software, with input through `libinput` (opened at run time) or read directly from event devices (`FERROUI_USE_EVDEV=1`: touch, multi-touch, a keyboard with a US layout, a mouse). A KMS output (`start_linux_drm`, `DrmOutput`) draws the view with OpenGL through EGL on a GBM surface and presents frames with page flips; `libdrm`, `libgbm` and `libEGL` are opened at run time. Verified on virtual devices (`vkms` and its framebuffer emulation with Mesa's software renderer, `uinput`), not on a board or a GPU.
- **Windows rendering and input** (`ferroui-win32`): the Windows.UI.Composition mode (the default before DirectComposition, as upstream) with the blur, acrylic and Mica transparency levels, the DXGI swap chain mode, WGL (the OpenGL of the system), the input method of the system (IME) with the cursor rectangle and the input pane.
- **Vulkan** (`ferroui-vulkan`, `ferroui-skia`, `ferroui-x11`): `ferroui-vulkan` is the Vulkan platform contracts crate and no longer a reserved name: the instance, the device, the graphics context, the swap chain render target of a surface and the import of external objects, over the `ash` bindings with the loader of the system opened at run time. The Skia backend renders through it (Ganesh on Vulkan), and the X11 platform takes the rendering mode Vulkan.
- **`ferro_thread_local!`** (`ferroui-base`): declares thread-local variables with the grammar and the methods of `thread_local!`, which it is on every target but Android; there each variable is a slot of one table per thread. The framework declares all its thread-local variables with it.
- **`ferroui-winui`**: the WinUI integration replaces its placeholder. `FerroSwapChainPanel` shows the framework's content inside a WinUI 3 window of the Windows App SDK: a swap chain panel the content renders into through ANGLE, with pointer, keyboard and character input, text input through the input method of the system (`CoreTextEditContext`), cursors, drag and drop in both directions and the automation peers of the content in XAML's automation tree. A Rust program initialises the Windows App SDK (`WindowsAppSdk::initialize`, release 1.6 or later; the bootstrap library ships beside the executable) and runs the XAML application (`XamlApplication::start`). The sample `ControlGallery.WinUI` hosts the ControlCatalog that way, and the sample `WinUIEmbedSample` shows controls of WinUI beside the framework's content in one window (both with a smoke mode). `XamlApplication::start` merges `XamlControlsResources` into the resources of the application, so the controls of WinUI have their default styles and theme resources. Verified in a Windows 11 ARM64 virtual machine by the smoke run of the sample: the content renders in the panel, the automation tree of the panel and of the content is built, the window closes cleanly; input, the input method, drag and drop and a client of UI Automation are not verified yet. Hosting in Windows Forms (upstream's interoperability project) has no counterpart in Rust and is not ported.
- **Vulkan on Android and Windows** (`ferroui-android`, `ferroui-win32`): the rendering mode Vulkan of both platforms (`VK_KHR_android_surface` and `VK_KHR_win32_surface`, the loader of the system opened at run time), used only when an application names it, as upstream. Opt-in through the new feature `vulkan` of each crate, which gives the Skia backend its Vulkan GPU: on Android from the published Skia binary with OpenGL ES and Vulkan; on Windows by compiling Skia from source (no published binary has OpenGL and Vulkan for Windows). Without the feature the mode is passed over with a warning; the default builds and the default orders of the rendering modes are unchanged. `scripts/android/apk.sh` takes `--features`, the Android smoke application `mode=vulkan`, and the Windows example `--vulkan-probe`.
- **Vulkan swap chain** (`ferroui-vulkan`): `VK_SUBOPTIMAL_KHR` from acquiring or presenting an image is a success, and `VK_ERROR_OUT_OF_DATE_KHR` from presenting drops the frame instead of failing it; the swap chain is recreated before the next frame when the surface asks for other parameters. A rotated Android screen no longer fails every frame (an upstream defect, recorded in DEVIATIONS).
- **OpenGL on macOS and iOS** (`ferroui-native`, `ferroui-ios`, `ferroui-skia`): the rendering mode OpenGL of the macOS platform (CGL, with the IOSurface render target of the native library and the import of IOSurfaces and Metal shared events) and of the iOS platform (OpenGL ES over EAGL, with the layer class of the view following the graphics), with the default orders and fallbacks of upstream (Metal first on both). The Skia backend draws them when built with `RUSTFLAGS="--cfg ferro_skia_apple_ganesh"`, which gives Ganesh on OpenGL and on Metal on Apple targets from a published Skia binary; the default build keeps Graphite on Metal. iOS: the remaining members of the platform (the automation wrapper, the application delegate's events, the text input members of the view, the view controller's safe area padding, the bookmark members of storage items).
- **UI Automation on Windows** (`ferroui-win32`): a window answers for its accessibility tree; the automation peers of the controls are exposed as providers of UI Automation (the element and fragment interfaces and the invoke, toggle, value, range value, expand and collapse, selection, selection item, scroll and scroll item patterns) with their property, structure and focus events. Verified by a client over the client interface of the system in CI; no screen reader has been run against it.

### Changed

- **The smoke runs of the WinUI samples end on the CI runner.** The samples let go of their main view when the window closes, while the threads of the framework run; the process of `ControlGallery.WinUI` stayed after its window had closed on the hosted Windows runner. The WinUI step of the Windows job is required.
- **Skia with Vulkan on Windows is built by a workflow of its own** (`skia-vulkan-windows.yml`, by hand and weekly): the feature `vulkan` of `ferroui-win32` compiles Skia from source.

- **Android builds with stable Rust.** The native library of an application no longer needs a nightly toolchain, `-Zbuild-std` or unstable flags: the framework keeps its thread-local variables in one table per thread on Android instead of one key of the C library per variable (of which the system has 128 for a process), and a library takes 9 keys (`docs/porting/android-platform.md`, section 5.1). `scripts/android/apk.sh` builds with the stable toolchain and the target `aarch64-linux-android`.
- **One loader of the libraries opened at run time** on Linux (`ferroui-base`): `ferroui_base::platform::native_library` (`NativeLibrary`, `native_functions!`) is shared by the X11, Wayland and Linux framebuffer crates, which had loaders of their own. Their behaviour is unchanged.
- **The text view visual of an input method client is optional** (`ferroui-base`, `ferroui-controls`): `TextInputMethodClient::text_view_visual` returns an `Option`, as upstream's value is nullable.

### Fixed

- **A text box without a template** panicked when it got or lost the focus with an input method active; it takes the focus now (`ferroui-controls`).
- **A popup opened while its target is moved by a render transform** (a page that slides in) stayed where the target was at the opening; an open popup is positioned again when the transform of its target changes (`ferroui-controls`; an upstream defect, recorded in `docs/porting/DEVIATIONS.md`, Popup).
- **The children of a border with a box shadow in a render target bitmap of more than 96 DPI** were drawn at twice their size and away from their place; the transform of a drawing context after a restore is the logical one again (`ferroui-skia`; an upstream defect, recorded in DEVIATIONS).
- **The render timer of the macOS native library** stops its display link when it is destroyed; a tick at the exit of the process read the freed timer (`ferroui-native`).

### Known limitations

- **Verification.** The new platforms and modes were verified in CI, in virtual machines, in the iOS simulator and on the Android emulator, not on real devices, boards or GPUs; no screen reader was run on any platform. The per-platform documents under `docs/porting` say what was run where.
- **WinUI.** Input, the input method, drag and drop and a client of UI Automation inside a WinUI 3 application are not verified (`docs/porting/winui.md`, section 9).
- **Wayland.** Opt-in, as upstream: the platform detection chooses X11. Not verified on a desktop compositor with input, with a real input method, or with the dmabuf swap chain on a GPU.
- **OpenGL on macOS and iOS** needs Skia built with `RUSTFLAGS="--cfg ferro_skia_apple_ganesh"`; the default build draws with Graphite on Metal.
- **Vulkan on Windows** compiles Skia from source (the feature `vulkan` of `ferroui-win32`).
- **Markup.** The build-time compiler of markup does not have the manifest keys, `include_xaml!` per class, the generated `register_types` or the diagnostics and cache of the build yet (`docs/porting/xaml.md`, 9.10.1).
- **API stability, the Vello backend, documentation.** As for the first preview.

## [0.1.0-preview.1] - 2026-10-15

The first preview.

The preview makes the framework available as crates so that it can be tried outside the repository. It is not a stable release: the public API is not settled, several subsystems are incomplete, and the per-subsystem state is tracked in [docs/porting/CRITICAL-PATH.md](docs/porting/CRITICAL-PATH.md).

### Added

- **Object model and base library** (`ferroui-base`): styled and direct properties with value priorities, inheritance, coercion and change notification; logical and visual trees; routed events; layout; input, focus and gestures; styling, control themes, resources and theme variants; data binding; animation and transitions; the media object model, fonts and text layout; the compositor with its render loop and dirty-region tracking.
- **Controls** (`ferroui-controls`, `ferroui-controls-color-picker`, `ferroui-dialogs`): the application model, top levels and windows, most of the control set including items controls, text input, menus, pickers, calendar, pages and automation peers; the colour picker library; the managed file chooser and the about dialog.
- **Themes** (`ferroui-themes-fluent`, `ferroui-themes-simple`, `ferroui-fonts-inter`): the Fluent and Simple themes for every ported control, in light and dark variants, and the embedded Inter font collection.
- **Markup** (`ferroui-markup`, `ferroui-markup-xaml`, `ferroui-markup-xaml-loader`, `xamlx`, `ferroui-build`, `ferroui-build-scan`): binding paths, the XAML run-time library and markup extensions, the run-time XAML loader, and the compilation of markup documents to Rust source by the build script of a crate.
- **Rendering and text** (`ferroui-skia`, `ferroui-harfbuzz`, `ferroui-opengl`, `ferroui-metal`, `ferroui-vello`): the Skia render backend (Graphite on Metal; Ganesh on OpenGL, on ANGLE and on WebGL; raster), HarfBuzz text shaping, the OpenGL and Metal platform contracts, and the Vello render backend.
- **Platforms** (`ferroui-desktop`, `ferroui-native`, `ferroui-microcom`, `ferroui-win32`, `ferroui-x11`, `ferroui-freedesktop`, `ferroui-ios`, `ferroui-android`, `ferroui-browser`, `ferroui-headless`):
  - macOS: windows, input, clipboard, drag and drop, menus, tray icon, Metal and software surfaces.
  - Windows: windows, input including pointer and touch, clipboard and drag and drop over OLE, icons, the task bar, the tray icon, file dialogs, mounted volumes, platform settings, the native control host, the extended client area; rendering in software, through ANGLE and through DirectComposition.
  - Linux on X11: windows, input, input methods (XIM, IBus, Fcitx), clipboard, drag and drop, screens, the desktop portals (settings, file chooser), GTK file dialogs, menus and the tray icon over D-Bus, session management, the native control host and XEmbed, accessibility over AT-SPI; rendering in software, with the shared memory extension, through GLX and through EGL.
  - iOS: a view over Metal, touch, keys and text input, platform settings, clipboard, storage, the native control host, accessibility.
  - Android: a view over a surface with EGL or software rendering, touch, keys and the input method, lifecycle, clipboard, storage, the native control host, accessibility.
  - The browser platform on `wasm32-unknown-emscripten`, and the headless platform for tests.
- **Tools** (`ferroui-designer-support`, `ferroui-remote-protocol`, `microcom-codegen`): the previewer and its remote protocol, and the code generator of the interop runtime.
- **Reserved names** (`ferroui`, `ferroui-wayland`, `ferroui-vulkan`, `ferroui-linuxframebuffer`, `ferroui-winui`, `ferroui-headless-vnc`): empty placeholder crates for parts that do not exist yet. They hold no code; a later preview replaces each with the real crate.

### Known limitations

- **Platforms.** macOS and the browser are the most complete. Windows, Linux (X11), iOS and Android are new in this preview and less tested: they were verified on the runners of the continuous integration, in virtual machines, in the iOS simulator and on the Android emulator, not on a range of real machines, devices and desktops.
- **Windows.** No input method (IME) support yet: ordinary typing works, composition for languages that need it does not. No UI Automation (accessibility), no WGL, and the blur, acrylic and mica transparency levels are reported as unsupported.
- **Linux.** X11 only: Wayland, the Linux framebuffer and Vulkan are not ported. Accessibility over AT-SPI was verified against the registry of at-spi2-core, not with a screen reader.
- **iOS.** Verified in the simulator only; no signed device build is described. Accessibility was not verified with VoiceOver.
- **Android.** Verified on the emulator (API 36) only. An application for Android cannot be built with stable Rust for now: the native library needs a nightly toolchain with a rebuilt standard library, because the standard library of the stable Android targets takes one thread key per thread-local value and the system has 128 (`docs/porting/android-platform.md`, section 5.1). Accessibility was not verified with TalkBack.
- **API stability.** None. Names, signatures and crate boundaries may change between previews, and the crates of one preview work only with one another (they are pinned at the exact version).
- **Vello backend.** Experimental. Skia is the default backend and the reference; the Vello backend is measured against it and does not yet match it everywhere (see `docs/porting/vello-backend.md`).
- **Compositor.** Composition animations and custom visuals are incomplete.
- **macOS platform.** Storage dialogs and parts of accessibility are incomplete.
- **Markup.** The run-time loader passes the ported test suites with a small number of documented gaps; the compiler of markup to Rust source is used by the themes and the samples.
- **Build requirements.** The Skia backend downloads a prebuilt Skia at build time, or builds it from source when no prebuilt binary matches the target and features, which needs network access and can take a long time. HarfBuzz is built from source and needs a C++ compiler. The macOS platform compiles an Objective-C++ library and needs the Xcode command line tools. The Windows platform needs the MSVC build tools, and `libclang` for its default feature `angle`. The Linux platform opens its system libraries (Xlib, GTK, GLib, the session management libraries) at run time and links none of them. The browser platform needs Rust 1.93 or later, Emscripten and the matching `wasm-bindgen` command-line tool (see `docs/porting/browser-platform.md`).
- **Documentation.** The API documentation is the documentation comments carried over by the port; there are no guides beyond the README and the documents under `docs/`. Not every crate is expected to build on docs.rs (see `docs/release.md`).
- **Tests of the published crates.** The packages hold the sources of the tests but not the test fonts and images of the Skia backend; the test suites are run from the repository.

[Unreleased]: https://github.com/wieslawsoltes/FerroUI/compare/v0.4.0...main
[0.4.0]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.4.0
[0.4.0-preview.1]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.4.0-preview.1
[0.3.0]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.3.0
[0.3.0-preview.1]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.3.0-preview.1
[0.2.0]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.2.0
[0.1.0]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.1.0
[0.1.0-preview.4]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.1.0-preview.4
[0.1.0-preview.3]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.1.0-preview.3
[0.1.0-preview.2]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.1.0-preview.2
[0.1.0-preview.1]: https://github.com/wieslawsoltes/FerroUI/releases/tag/v0.1.0-preview.1
