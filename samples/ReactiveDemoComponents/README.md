# ReactiveDemo with components

`ReactiveDemo`, the getting-started sample of ReactiveUI (a crates.io browser with a throttled
search), written with the declarative Rust API of FerroUI (`ferroui-view`) instead of XAML, view
models and bindings. `samples/ReactiveDemo` is the port of the reference with ReactiveUI; this
sample shows the same application with components that run again and the API of phase 6
(`docs/declarative-ui.md`, section 14). It runs in the WinUI 3 theme.

## What it shows

- **Asynchronous data** (`use_future`). The search is a future of the trimmed search text: it
  waits 800 ms, the throttle of the reference, then asks the search service of `reactive-demo`
  (whose task is a future). A new text drops the future of the previous one, so only the text the
  user stopped typing at is searched, and an equal text is not searched again; an empty text
  shows no results.
- **Suspense**. `suspense` shows a progress bar while the search waits and runs, the results when
  it ends, and its error when it fails.
- **Navigation**. A result opens the page of its crate, the route `/crate/:name` of `router`; the
  search is the route `/`. The page has a `Back` button (`navigate_back`), and the search text
  outlives the page.
- **An error boundary**. The page of a crate is inside `error_boundary`: a crate that no result
  showed is an error of the page, which the boundary replaces with the error, a `Try again` button
  (`reset`) and a way back to the search.
- **The views of the reference** as functions that return elements: each result shows the badge,
  the name and version, the description and the downloads of its crate.

## Running it

```sh
cargo run -p reactive-demo-components
```

With the default feature `online` the search queries the API of crates.io. To search the
in-memory catalogue of `reactive-demo` instead:

```sh
cargo run -p reactive-demo-components --no-default-features
```

Take the picture of the window (it searches the catalogue for `http`, renders itself into
`<directory>/reactive-demo-components.png` and closes):

```sh
FERROUI_REACTIVE_DEMO_SHOTS=<directory> cargo run -p reactive-demo-components
```

Run the tests (scenarios on the test application of `ferroui-controls` with the in-memory
catalogue: the throttle, the results, a failed search, the page of a crate and back, and the error
boundary):

```sh
cargo test -p reactive-demo-components
```
