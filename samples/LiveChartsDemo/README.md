# LiveCharts demo

This Rust sample loads one compiled XAML view with six native FerroUI chart
controls: Cartesian, pie/doughnut, polar, Sankey, treemap and geographic map.
The chart engines, drawing, layout, text measurement, brushes, property system
and data observation run in Rust. The build uses `ferroui-build` with
`TypeSystem::Model`, following the framework's sample build pipeline.

## Run

From the repository root:

```sh
# Compile the view and compare it with the runtime XAML loader.
cargo test -p livecharts-demo --lib

# Open the native desktop window (the standard desktop platform and renderer).
cargo run -p livecharts-demo --features desktop --bin livecharts-demo

# Render the actual compiled visual tree through Vello CPU and HarfBuzz.
# No display server, GPU or Skia download is required by this feature.
cargo run -p livecharts-demo --features render --bin livecharts-render -- target/livecharts-demo.png

# Render the same view after one deterministic live-data update.
cargo run -p livecharts-demo --features render --bin livecharts-render -- target/livecharts-demo-updated.png --mutated
```

The default build is a library with the compiled markup and tests. The two
executables are opt-in so compiling the gallery does not require a desktop
platform or rendering backend. The render executable verifies that all six
families contain measured data before saving the 1440 × 1040 PNG.

## What the view exercises

| Family | Data and behavior |
| --- | --- |
| Cartesian | Native gradient columns, a smooth line, category axis, legend and observable point edits |
| Pie | Four colored series with a shared doughnut radius and live value updates |
| Polar | Two closed profiles with category labels and a fixed radial range |
| Sankey | Five native nodes, four weighted links, proportional ribbons and source/sink labels |
| Treemap | Two hierarchy roots and six leaves, squarified layout, measured labels and deep child observation |
| Geographic | Embedded pinned world polygons, Mercator projection, heat values and native point markers |

The `Update live data` button changes the existing point, link, tree and map
objects. It does not rebuild the view or replace the charts. Its callback holds
a weak reference to the view. The renderer's `--mutated` argument runs the same
data action, so before/after images are reproducible. Native brush changes also
invalidate the Cartesian chart; the sample's tests change a gradient stop.

`MainView.xaml` declares chart types, series, axes, brushes and advanced model
collections. `lib.rs` supplies deterministic numeric data, category labels,
layout options and the embedded world map. `desktop.rs` and `render.rs` both
load the same compiled document. The runtime loader is a test dependency used
to compare the resulting native scenes, including compound geographic hits.

Treemap children are nullable: omitting `Children` makes a leaf. An internal
node explicitly assigns a `<TreemapNodes>` collection inside
`<TreemapNode.Children>`, so an empty internal collection stays distinct from
a leaf in both compiled and runtime markup. The numeric `Value` properties of
advanced models use `set_numeric_value` in Rust to keep the native generic
`set_value(property, value)` API available to compiled markup.

Cartesian and polar charts expose initially empty `XAxes` and `YAxes`
collections, matching the source controls. They use internal default axes until
an application declares its own. The sample adds one `<Axis>` inside each axis
property element. The doughnut radius belongs to each `<PieSeries>`;
`InnerRadius="56"` is shared by the four series in this view.

## Evidence and limits

The three sample tests verify compiled/runtime construction equivalence,
one-click invalidation and changed geometry in every family, and native gradient
mutation. CPU images exercise native layout, text shaping and the controls'
actual rendering path. They are integration smoke captures; they do not claim
pixel equivalence to the upstream LiveCharts2 snapshot suite. The full pinned
upstream inventory and remaining parity work are tracked in
[`docs/porting/livecharts2.md`](../../docs/porting/livecharts2.md).
