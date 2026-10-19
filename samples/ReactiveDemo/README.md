# ReactiveDemo

A NuGet package browser built with ReactiveUI on FerroUI. It is a port of `ReactiveDemo`, the
getting-started sample of the ReactiveUI documentation
(`ReactiveUI.Samples/wpf/getting-started/ReactiveDemo`), file for file.

## What it shows

- **A view model with reactive properties** (`app_view_model.rs`). `SearchTerm` is a notifying
  property written with `reactive_properties!`. The search is a pipeline over it:
  `when_any_value`, a throttle of 800 ms, the trimmed term, `distinct_until_changed`, empty terms
  filtered out, the asynchronous search, delivery on the main thread scheduler, and `to_property`
  for the output property `SearchResults`. `IsAvailable` is a second output property, derived from
  the results.
- **A command** (`nuget_details_view_model.rs`). `OpenPage` is a `ReactiveCommand` that opens the
  page of a package with the launcher of the platform.
- **Views in markup with bindings in code** (`MainWindow.xaml`, `NugetDetailsView.xaml`). The
  window is a reactive window and the package view a reactive user control. Their code-behind
  binds inside `when_activated`, as the reference does: `bind` of the search box, `one_way_bind`
  of the results and of the visibility of the list, `one_way_bind` of the icon, title and
  description of a package, and `bind_command` of its link.
- **View location**. The application registers the package view for its view model; the list has
  no item template, so the data-template hook of the integration gives it one that shows each item
  with the view located for its view model.

## Running it

```sh
cargo run -p reactive-demo
```

Type a search term: the search starts 800 ms after the last key. With the default feature
`online` the sample queries the search service of nuget.org from a worker thread and downloads the
icons of the packages. To run without the network, search an in-memory catalogue of about thirty
packages instead:

```sh
cargo run -p reactive-demo --no-default-features
```

Run the tests (the view model on the in-memory catalogue and a test scheduler, the window in the
unit-test application, and the documents of the sample):

```sh
cargo test -p reactive-demo
```

## Differences from the reference

The reference is a WPF application that uses NuGet.Protocol. The port keeps its structure and its
comments; the differences are marked in the sources with `Deviation (DEVIATIONS.md, samples)` and
listed in `docs/porting/DEVIATIONS.md`. The main ones:

- The documents name the non-generic classes of the reactive window and user control, and the
  code-behind types them with their view model.
- The bindings take a view that is the class of the document as ReactiveUI sees it
  (`code_behind_view.rs`), and property paths written with `prop!`, `ferro_prop!` and
  `settable_prop!` in place of expressions.
- The search is a small service (`nuget_search.rs`) behind a trait, so that the tests use the
  catalogue, and the throttle waits on the main thread scheduler.
- The icon of a package is downloaded and decoded by the view, as the `BitmapImage` of the
  reference does for itself.
