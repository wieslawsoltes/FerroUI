# LiveCharts demo

This Rust sample loads one compiled XAML view with six native FerroUI LiveCharts controls, all
Cartesian or pie charts: columns with a line, a doughnut, stacked areas, scatter, candlesticks
and a gauge. The chart engines, drawing, layout, text measurement, brushes, property system and
data observation run in Rust. The build uses `ferroui-build` with `TypeSystem::Model`, following
the framework's sample build pipeline. The polar, Sankey, treemap and geographic charts join the
view with their engines (docs/porting/livecharts2.md, L4).

## Run

From the repository root:

```sh
# Compile the view and compare it with the runtime XAML loader.
cargo test -p livecharts-demo --lib

# Open the native desktop window (the standard desktop platform and renderer).
cargo run -p livecharts-demo --features desktop --bin livecharts-demo

# Render the compiled visual tree through Vello CPU and HarfBuzz (no display server or GPU).
cargo run -p livecharts-demo --features render --bin livecharts-render -- target/livecharts-demo.png

# Render the same view after one deterministic live-data update.
cargo run -p livecharts-demo --features render --bin livecharts-render -- target/livecharts-demo-updated.png --mutated

# Render the ported scenarios of the upstream snapshot suite, one PNG each.
cargo run -p livecharts-demo --features render --bin livecharts-snapshots -- target/livecharts-snapshots
```

The default build is a library with the compiled markup and tests. The executables are opt-in so
compiling the gallery does not require a desktop platform or rendering backend. The render
executable verifies that every chart contains measured data before saving the 1440 x 1040 PNG.

## What the view exercises

| Chart | Data and behavior |
| --- | --- |
| Cartesian | Native gradient columns, a smooth line, a category axis, a legend and observable point edits |
| Pie | Four series with a shared doughnut radius and live value updates |
| Stacked areas | Three stacked area series over a category axis |
| Scatter | Two series of (x, y) points with named axes |
| Financial | Eight candlesticks, coloured by direction |
| Gauge | A 270-degree gauge with a fill track and a centred data label |

The `Update live data` button changes existing point objects. It does not rebuild the view or
replace the charts, and its callback holds a weak reference to the view. The renderer's
`--mutated` argument runs the same data action, so before and after images are reproducible.
Native brush changes also invalidate the Cartesian chart; the sample's tests change a gradient
stop.

## Evidence and limits

The sample tests verify compiled and runtime construction equivalence, one-click invalidation and
changed geometry in every chart, and native gradient mutation. The CPU images exercise native
layout, text shaping and the controls' rendering path. The snapshot renders are the ported
scenarios of the upstream snapshot suite; they are visual references, not a pixel comparison with
the upstream images (which SkiaSharp draws). The pinned upstream inventory and the remaining work
are tracked in [`docs/porting/livecharts2.md`](../../docs/porting/livecharts2.md).
