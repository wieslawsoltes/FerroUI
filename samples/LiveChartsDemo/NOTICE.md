# LiveCharts demo

This sample is a new integration example for the FerroUI Rust port. Its view
and data are original to this repository; it is not a translation of a single
upstream sample. Its snapshot scenarios (`snapshots.rs`) restate the chart
configurations of LiveCharts2's `tests/SnapshotTests`.

The chart algorithms are provided by
`ferroui-livecharts`, ported from LiveCharts2 at commit
`8537b90ffb100a947cd0b1f6be49f896ae6050eb`. The upstream project is copyright
2021 Alberto Rodriguez Orozco and LiveCharts Contributors, licensed under MIT.
See [`src/FerroUI.LiveCharts/NOTICE.md`](../../src/FerroUI.LiveCharts/NOTICE.md)
for the full notice and dependency mapping.

`mexico-states.geojson` is the map of the states of Mexico of the LiveCharts2 sample
`samples/ViewModelsSamples/Maps/CustomMap` at the same commit, copied byte for byte (376,048
bytes; Git blob 9ed7e9ca50f681372f230b92e3bc8bed8a536eff) for the scenario
`MapsTests_CustomMap_LoadsAndRendersMexicoStates`. Its regeneration script in the sample states
its origin: a derivative of the Natural Earth 1:10m Admin 1 states and provinces
(https://www.naturalearthdata.com/), through https://github.com/martynafford/natural-earth-geojson,
under CC0-1.0 (public domain).

The optional CPU rendering path uses the existing FerroUI Vello and HarfBuzz
backends. The embedded Inter font is supplied by `ferroui-fonts-inter`, under
its existing font license and notices.
