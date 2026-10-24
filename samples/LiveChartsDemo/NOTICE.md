# LiveCharts demo

This sample is a new integration example for the FerroUI Rust port. Its view
and data are original to this repository; it is not a translation of a single
upstream sample.

The chart algorithms and embedded world map are provided by
`ferroui-livecharts`, ported from LiveCharts2 at commit
`8537b90ffb100a947cd0b1f6be49f896ae6050eb`. The upstream project is copyright
2021 Alberto Rodriguez Orozco and LiveCharts Contributors, licensed under MIT.
See [`src/FerroUI.LiveCharts/NOTICE.md`](../../src/FerroUI.LiveCharts/NOTICE.md)
for the full notice and dependency mapping.

The optional CPU rendering path uses the existing FerroUI Vello and HarfBuzz
backends. The embedded Inter font is supplied by `ferroui-fonts-inter`, under
its existing font license and notices.
