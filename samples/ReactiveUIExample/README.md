# ReactiveUI example

The example application of the ReactiveUI integration with FerroUI. It is a port of the example of
the integration of ReactiveUI with the reference UI framework, file for file, with one page added:
a DynamicData page that the reference does not have.

## What it shows

A shell with a sidebar and a `RoutedViewHost`. The routed pages are:

- **Overview.** The features of the showcase in an `ItemsControl` that has no item template: the
  `AutoDataTemplateBindingHook` gives it one, and each row is a `FeatureView` resolved by a
  `ViewModelViewHost`.
- **Live metrics.** Measurements of this process, sampled every second while the page is active
  (`WhenActivated`): the processor use, the working set and the heap. Each card is resolved by a
  `ViewModelViewHost` with a view contract (`performance`, `memory`). A label computed with
  `ObservableAsPropertyHelper` compares the processor use with a threshold slider, and the opacity
  of a preview follows the slider through the property subjects of the two controls
  (`get_subject`, `get_binding_subject`).
- **Command lab.** A `ReactiveCommand` whose can-execute follows the input, asynchronous work that
  captures its input before it waits, a command that fails by design, and the error reported
  through an `Interaction` that only the active view handles. The view binds in code (`Bind`,
  `OneWayBind`, `BindCommand`) inside `WhenActivated`.
- **DynamicData.** An addition of the port with no counterpart in the reference. The features of the
  showcase live in a `SourceCache` keyed by their id. The list is the pipeline `connect`,
  `auto_refresh` on the favourite flag, `filter_dynamic` on the search text (observed with
  `when_any_value` and throttled), `transform` into row view models, and `sort_and_bind` into an
  `ObservableCollectionExtended` with a `SortExpressionComparer` (favourites first, then by name).
  The page adds a feature with `add_or_update` and removes one by its key; checking the favourite
  box of a row moves it to the top.

`Back` and `Reset` use the router's commands. On desktop, `AutoSuspendHelper` restores the input of
the command lab and the threshold of the metrics on launch, saves them on exit, and deletes them
after an unhandled error. The file is `ReactiveUIExample/session.json` in the local application data
folder; the sidebar reports storage errors.

## Run it

```text
cargo run -p reactiveui-example
```

The tests:

```text
cargo test -p reactiveui-example
```

## What it ports

| Reference | Port |
| --- | --- |
| The shared library of the example: `App`, `ExampleAppBuilderExtensions`, the models, the services, the view models and the views with their styles | `app.rs`, `example_app_builder_extensions.rs`, `Models/`, `Services/`, `ViewModels/`, `Views/` |
| The desktop head (`Program.cs`) | `program.rs` |
| The test project (33 tests on a headless session) | `tests/`, on the unit test application of FerroUI and the headless shell of the samples |
| The Android, iOS and browser heads | Not ported: FerroUI builds its own Android, iOS and browser applications. The shared library supports them: a single-view lifetime gets a `MainView`, and a browser gets the browser metrics service. |

The view models are built on `ferroui-reactiveui`, the views on `ferroui-reactiveui-controls`, and
the DynamicData page on `ferroui-dynamic-data`. The views are markup documents compiled by the build
of the crate, with their code next to them.

## Differences from the reference

The differences are recorded in `docs/porting/DEVIATIONS.md` (section "samples") and marked in the
sources. The main ones:

- The process measurements come from the operating system (Linux, Android, macOS and Windows)
  instead of `System.Diagnostics.Process`; where they are not available, reading fails, as it does
  in the reference. The heap is the one the allocator of the example counts.
- The services are registered in Splat's resolver; the reference uses the Microsoft dependency
  injection container.
- A view binds through a small adapter of its control (`Views/bindable_view.rs`), the view that the
  bindings of the port take.
- The live samples tick on the scheduler of the user-interface thread, and the delays of the
  asynchronous commands are timers of the dispatcher.
