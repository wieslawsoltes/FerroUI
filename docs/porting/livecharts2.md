# LiveCharts2 port

FerroUI ports [LiveCharts2](https://github.com/Live-Charts/LiveCharts2) (MIT, Copyright (c) 2021 Alberto Rodriguez Orozco) at commit `8537b90f` as its advanced chart library: Cartesian, pie, polar, Sankey, treemap and geographic charts with LiveCharts2's measurement, paint and motion. `ferroui-controls-charts` (#282) stays the simple chart library (line, area and bar series, axes, legends). The work comes from pull request #312, which stays a draft reference and is closed when the last part lands (owner decisions, 2026-10-30; the review is in the pull request record).

## Landing plan

| Part | Content |
|---|---|
| L0 | The accounting of the port against the pinned sources (this page, `scripts/livecharts2`, `docs/porting/livecharts2/`, the scan job of `livecharts.yml`) |
| L1 | `ferroui-livecharts`: the shared core (paint, motion, geometry, measurement, hit testing, the chart engine base) and the Cartesian and pie engines, with their upstream tests ported; licence and notices |
| L2 | `ferroui-controls-livecharts`: the native `CartesianChart` and `PieChart` controls, typed series collections, runtime XAML, the facade feature |
| L3 | `samples/LiveChartsDemo` (Cartesian and pie), the catalog page, the test and render jobs of `livecharts.yml` |
| L4 and later | Polar, Sankey, treemap, geographic, each with its upstream tests |

## Accounting

`scripts/livecharts2/inventory.py` reads a LiveCharts2 checkout at the pin of `docs/porting/livecharts2/sources.lock.json` (the Git objects of that commit, never the working tree) and FerroUI's Roslyn syntax extractor (`scripts/api-extract`, .NET 10), and accounts for every source file, API member and test declaration of the pin: 698 source files, 4,859 API members and 1,066 test declarations.

```sh
python3 scripts/livecharts2/inventory.py --upstream ../LiveCharts2
python3 scripts/livecharts2/inventory.py --check --upstream ../LiveCharts2
```

Committed in `docs/porting/livecharts2/` (O8 of `winui-controls.md`, the pattern of the WinUI Gallery scanner): the pin, the reviewed `mapping.json`, and the generated `summary.json` and `REPORT.md`. The inventories and maps (`inventory.json`, `upstream-api.json`, `source-map.json`, `api-map.json`, `test-map.json`) are written to `target/livecharts2-scan`, an artifact of the workflow.

Every entry is `pending` until `mapping.json` records a reviewed disposition: `ported` or `native-replacement` with the Rust files that implement it (which must exist), `partial` with what remains, or `not-applicable` with the reason. Each part of the port adds the dispositions it implements. The counts are declarations, not executed tests or behavioural equivalence.

The `scan` job of `livecharts.yml` checks out the pin, runs `inventory.py --check`, puts the report in the run summary and keeps the inventories as an artifact. It runs on the paths of the port and nightly.
