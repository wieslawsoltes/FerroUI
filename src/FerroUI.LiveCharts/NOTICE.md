# LiveCharts2 attribution

The LiveCharts2 algorithms and chart contracts in this crate are derived from
https://github.com/Live-Charts/LiveCharts2 at commit
8537b90ffb100a947cd0b1f6be49f896ae6050eb.

Source-header attribution: Copyright(c) 2021 Alberto Rodriguez Orozco & LiveCharts Contributors.
The Rust/FerroUI adaptations and their current parity are recorded in
`docs/porting/livecharts2.md` and the source/API/test maps beside it.

## World map data

`geo/world.geojson` is the world map of LiveCharts2, `src/LiveChartsCore/Geo/world.geojson` at
the commit above, copied byte for byte (178,383 bytes; Git blob
c8d10e5c13f017ca6a689471438f5a5a5f0cacc0) and embedded with `include_str!` as the source embeds
it as a resource. It is distributed with LiveCharts2 under the MIT license below; the source
states no other license or origin for it. Its 177 country features (the collection is named
`world-110`) match the 1:110m Admin 0 countries of Natural Earth
(https://www.naturalearthdata.com/), which are in the public domain; the source does not name
its origin.

MIT License

Copyright (c) 2021 Alberto Rodriguez Orozco

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
