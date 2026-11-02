# LiveCharts2 port

FerroUI ports [LiveCharts2](https://github.com/Live-Charts/LiveCharts2) (MIT, Copyright (c) 2021 Alberto Rodriguez Orozco) at commit `8537b90f` as its advanced chart library: Cartesian, pie, polar, Sankey, treemap and geographic charts with LiveCharts2's measurement, paint and motion. `ferroui-controls-charts` (#282) stays the simple chart library (line, area and bar series, axes, legends). The work comes from pull request #312, which stays a draft reference and is closed when the last part lands (owner decisions, 2026-10-30; the review is in the pull request record).

## Landing plan

| Part | Content |
|---|---|
| L0 | The accounting of the port against the pinned sources (this page, `scripts/livecharts2`, `docs/porting/livecharts2/`, the scan job of `livecharts.yml`) |
| L1a | `ferroui-livecharts`: the shared core (axes, scales and ticks, stacking, coordinates, easing, motion, paints and their motion, scenes and hit testing, series descriptions, visual states) with the dispositions of the core sources and tests; licence and notices |
| L1b | The Cartesian engine and series, with the upstream Cartesian series and chart tests that do not need a rendering host |
| L1c | The pie engine and series, with their upstream tests |
| L2 | `ferroui-controls-livecharts`: the native `CartesianChart` and `PieChart` controls, typed series collections, runtime XAML, the facade feature |
| L3 | `samples/LiveChartsDemo` (Cartesian and pie), the catalog pages CartesianChart and PieChart, the test, browser and render jobs of `livecharts.yml` with the scenarios of the upstream snapshot suite |
| L4 | The polar chart (#442), with the gaps of the Cartesian charts found against the upstream snapshots; then Sankey, treemap and geographic, each with its upstream tests |

## Accounting

`scripts/livecharts2/inventory.py` reads a LiveCharts2 checkout at the pin of `docs/porting/livecharts2/sources.lock.json` (the Git objects of that commit, never the working tree) and FerroUI's Roslyn syntax extractor (`scripts/api-extract`, .NET 10), and accounts for every source file, API member and test declaration of the pin: 698 source files, 4,859 API members and 1,066 test declarations.

```sh
python3 scripts/livecharts2/inventory.py --upstream ../LiveCharts2
python3 scripts/livecharts2/inventory.py --check --upstream ../LiveCharts2
```

Committed in `docs/porting/livecharts2/` (O8 of `winui-controls.md`, the pattern of the WinUI Gallery scanner): the pin, the reviewed `mapping.json`, and the generated `summary.json` and `REPORT.md`. The inventories and maps (`inventory.json`, `upstream-api.json`, `source-map.json`, `api-map.json`, `test-map.json`) are written to `target/livecharts2-scan`, an artifact of the workflow.

Every entry is `pending` until `mapping.json` records a reviewed disposition: `ported` or `native-replacement` with the Rust files that implement it (which must exist), `partial` with what remains, or `not-applicable` with the reason. Each part of the port adds the dispositions it implements. The counts are declarations, not executed tests or behavioural equivalence.

The `scan` job of `livecharts.yml` checks out the pin, runs `inventory.py --check`, puts the report in the run summary and keeps the inventories as an artifact. It runs on the paths of the port and nightly.

## Status

L1a: the core crate compiles against `main` unchanged and its 88 tests pass. `mapping.json` records the dispositions of the core: 43 source files (21 ported, 16 native replacements, 6 partial), 80 ported and 8 partial test declarations (the easing, cubic Bezier, measurement, transition, visual-state and state-behaviour suites of `CoreObjectsTests`). The gradient paint animation fixtures are partial until they run against the native brushes of L2. The colour, labeler, margin and hover-area suites follow with the types that carry them (L1b and L2).

L1b: the Cartesian engine (`cartesian_chart_engine.rs`) with its unit tests, the axis padding tests and the numerical adaptations of #312 (range bounds, maximum bar width, line scale and marker colour, value pruning, the collapsed plot). The `ShouldScale` fixtures of the column, row, scatter, step line, stacked column, stacked row, financial and box series are ported: the source's series, values, chart size and axis limits, with its assertions on the visuals read from the retained scene. The report counts 92 ported and 11 partial test declarations. The tooltip, data label and animation suites need the chart host and follow with the controls (L2).

L1c: the pie engine (`pie_chart_engine.rs`: pie, doughnut, gauge and nested rings, label placement, corner radii) with its unit tests, and `PieSeriesTest.ShouldScale` and `ShouldPlaceDataLabel` ported; the latter runs the source's thirteen-slice fixture for the five label positions and four radius configurations. The report counts 94 ported and 12 partial test declarations. The tooltip and animation tests of the pie series follow with the controls (L2), the snapshot tests with the render job (L3).

L2: `ferroui-controls-livecharts`, the native `Chart` with `CartesianChart` and `PieChart`, typed series classes for every Cartesian family and the pie series, axes, points and their observable collections, runtime and compiled XAML (`https://ferroui.dev/livecharts`), input (hover, data pointer events, zoom, pan, section zoom, keyboard), animated scene transitions, automation, and the facade feature `livecharts`. The repetitive property declarations are generated by `scripts/livecharts2/generate_controls.py` (`--check` verifies them). `ChartKind` is `non_exhaustive`: the polar, Sankey, treemap and geographic charts add their kinds, classes and properties with their engines (L4).

L3: `samples/LiveChartsDemo`, six Cartesian and pie charts in one compiled view (columns with a line, a doughnut, stacked areas, scatter, candlesticks and a gauge) with live data, and the ControlCatalog pages CartesianChart and PieChart (Collections & Data). `samples/LiveChartsDemo/snapshots.rs` restated 55 scenarios of the upstream `tests/SnapshotTests` (the column, row, line, step line, scatter, stacked, box, candlestick, range and pie suites) with the native controls; the render job of `livecharts.yml` draws each one through Vello CPU beside the upstream reference image of the same name, and a test checks that each measures its data. They are partial in `mapping.json`: the upstream suite compares SkiaSharp images pixel by pixel, which a native renderer cannot reproduce.

### What the snapshot review found

Reviewing the native renders beside the upstream references of the 55 scenarios of L3 found differences, each named in the disposition of its scenario. #442 closes these, each against the pinned source and with a test in `cartesian_chart_engine.rs`:

- Box series: the visible box series of a chart stand side by side in each category (`SeriesContext.GetBoxPosition`, `BoxMeasureHelper`, with the source's padding 5 and the theme's maximum width 60) and are drawn as `BoxGeometry`: whiskers from the box to the extremes, the median across, no caps, the theme's dark outline.
- Axis ranges: an axis without data spans ten unit widths from zero and an axis of zero width grows by half a unit width on each side (`CartesianChartEngine.Measure`) before its labels, ticks and padding are measured; a series requests its padding from the tick of its raw bounds (`Extensions.GetTick`), and the separators take their step from the draw margin (`CoreAxis.BeginMeasure`). A single remaining point now spans 0.5 to 1.5 with eleven separators, as the upstream image.
- Stacked step areas: the lower edge of an area is the upper edge of the series below walked backwards, so no cell between the series is left unfilled.
- Theme defaults: the MaterialDesign500 palette; the area at alpha 50 under line, step, range and polar lines (a transparent fill is the source's `Fill = null`); stacked areas in full colour without a stroke or geometry; line series bound their points, not the pivot; axis labels of 16 px with the theme's Cartesian padding of 12 (`LabelsPadding` is optional: `None` takes 12 for a Cartesian axis and the source's 3 for a polar axis).

One difference is the upstream's: for an empty series (`LineSeriesTests.VectorOperationsClear`) the reference image draws the X axis from 10 to 0, because the source's `Scaler` passes the sentinel bounds of the series to `AxisLimit.ValidateLimits` in swapped order. The native axis keeps the default range from 0 to 10 in the order of the axis; the disposition of the scenario says so.

Still different from the source theme (named in the disposition of every scenario): the gauge rules (no stroke, centred data labels, corner radius 8, the translucent fill series; the gauge scenarios set them), the stroke width and the light 12 px markers of line series, the corner radius and the missing stroke of bars, the scatter and candle colours, and the separators of X axes, which the theme hides.

### L4: the polar chart

`polar_chart_engine.rs` of `ferroui-livecharts` starts from the code of #312, checked against the pinned `PolarChartEngine`, `CorePolarAxis` and `CorePolarLineSeries`: independent angle and radius axes, the empty and zero-width bounds of the source, the series padding in polar ticks, the automatic margin of the angle labels, an explicit draw margin, fit-to-bounds, the source spline of an open or closed line with null splitting, stacked radii, the five data label positions with the tangent and cotangent rotation flags, and hover areas. A polar line fills the theme's area over the same path it strokes; an open line closes its area with a straight edge, as the source fills its open path. The angular inverse uses atan2 and corrects the quadrant and nonzero-minimum defects of the source.

`ferroui-controls-livecharts` adds `ChartKind::Polar` (the enum stays `non_exhaustive` for the Sankey, treemap and geographic charts), the generated `PolarChart` (its X axes are the angle axes and its Y axes the radius axes; `InitialRotation` -90 from `LiveChartsSettings`) and `PolarLineSeries`, `Chart::polar_scale`, and angle and radius conversions in `scale_data_to_pixels` and `scale_pixels_to_data`. The scene is drawn by the same native renderer as the Cartesian and pie scenes, so it takes the GPU path of every backend. The ControlCatalog has the page PolarChart, and `snapshots.rs` restates the five scenarios of the upstream `PolarChartTests` (60 scenarios in all).

`mapping.json` records `PolarLineSeriesTest.ShouldScaleProperly` and `PolarDrawMarginTests.ExplicitDrawMargin_IsHonored` as ported, the five polar snapshot scenarios and the three polar source files as partial: the report counts 96 ported and 71 partial test declarations. `PolarLineSeriesTest.ShouldPlaceToolTipsCorrectly` stays pending with the tooltip suites.

### L4b: the Sankey and treemap charts

`treemap_chart_engine.rs` and `sankey_chart_engine.rs` of `ferroui-livecharts` start from the code of #312, checked line by line against the pinned `TreemapLayout`, `TreemapChartEngine`, `CoreTreemapSeries`, `SankeyLayout`, `SankeyChartEngine`, `CoreSankeySeries` and the SkiaSharp Sankey geometries. The treemap engine has the squarified layout, the roll-up of internal nodes (a nonzero explicit value wins; an empty child list is an internal node, without a hover area or a label), the padding inset of every level, the partition of the draw margin between the visible series by their totals (a series without a slot takes the whole margin), hover areas and centred labels on the leaves only, a label culled when its text with the label padding does not fit its tile, and the paint order of the source: the fills of a series, then its strokes, then the labels of every series. The Sankey engine has the d3-sankey vertical layout (repeated nodes, self-loops, unknown ends and invalid weights skipped, Kahn depths with the cycle error, the column scale, the relaxation sweeps and collision passes, the link bands in the order of their other ends), the bipartite arc layout with its error on multi-column data, the ribbon, chord and arc segment shapes, the Auto, Outside and Inside labels with the room they reserve and the radial label margin of the arcs, the ribbon paint (the link colour, else the source node colour at half alpha, else `LinkFill`, else the theme's at alpha 90; a null `LinkFill` takes the node fill) and the paint order of the source (ribbons, node strokes, node fills, labels). The code of #312 had five more label positions and per-node treemap colours, which the source does not; they are gone.

Differences from the source, named in the dispositions: nodes and links are objects (the engine identifies them by id), and a node or link colour stands for `NodeColorMapper` and `LinkColorMapper`; the treemap model is the native `TreemapNode`, with no `ValueMapper`, `ChildrenMapper` or `LabelMapper` for other models; `DataLabelsMaxWidth` does not wrap labels; a cycle, a bipartite arc of three columns and a treemap tree that contains itself are a `ChartError` where the source throws or recurses without end; the squarified weights are normalised by the largest, so weights near `f64::MAX` keep their areas; positions are `f64` where the source uses `float`. The default theme of the source gives a treemap no fill, which draws nothing; the native treemap takes the palette colour, as the theme does for a Sankey series.

`ferroui-controls-livecharts` adds `ChartKind::Sankey` and `ChartKind::Treemap` (`ChartKind` stays `non_exhaustive` for the geographic chart), the generated `SankeyChart`, `TreemapChart`, `SankeySeries` (the properties of `ISankeySeries` with the source defaults) and `TreemapSeries` (the shared `Padding`, 2 by default, and `CornerRadius`), the collections `SankeyNodes`, `SankeyLinks` and `TreemapNodes`, and the models `SankeyNode`, `SankeyLink` and `TreemapNode` (`node_models.rs`), observed down to every child list. A link refers to its node objects, so markup declares a treemap completely while a Sankey graph is built in code; the catalog page SankeyChart builds its two graphs in its class. The ControlCatalog has the pages SankeyChart and TreemapChart, and `snapshots.rs` restates the nine image scenarios of `SankeySeriesTests` and the five of `TreemapSeriesTests` (74 scenarios in all); the hover scenarios move the pointer, so their renders show the native tooltip.

`mapping.json` records as ported `SankeySeriesTests.BipartiteArc_ThrowsOnMultiColumn`, the four Sankey cases of `SeriesHitTests`, the Sankey and treemap cases of `CollapsedDrawMarginAllChartsTests` and the four Sankey and treemap cases of `_MemoryTests`; the fourteen image scenarios as partial, each with the engine test of what it pins; and 23 source files: the two layouts and the three node models ported, the two engines and the two core series partial, and the measure contexts, the base and SkiaSharp geometries, the SkiaSharp series and the interfaces native replacements. The report counts 107 ported and 85 partial test declarations. `SharedUITests.SankeyChartTests.ShouldLoad` and `TreemapChartTests.ShouldLoad` stay pending with the UI suites.

## Remaining (#442)

- The geographic charts with their upstream tests. #312 (`feat/livecharts2-port`) holds their starting code: `geo/`, `geo_map_chart.rs`, `heat_functions.rs` and the native map models of `advanced_models.rs`.
- Sankey and treemap: the entrance and exit animations of each node, ribbon and label (the native scene transition animates the scene instead), the mappers of custom node models, and `DataLabelsMaxWidth`.
- Polar: axis names, label backgrounds and sub-separators, the marker paints (`GeometryFill`, `GeometryStroke`), tooltips.
- Not ported: the OHLC series, error bars, the labeler of a date axis, heat colour stops and point padding, and the angular gauge with its needle (the scenarios of those suites stay pending).
