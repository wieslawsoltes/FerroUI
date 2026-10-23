# Exact dc361996 strict browser text-pan benchmark

**The strict benchmark completed successfully in both modes and shut down its owned Chromium process gracefully.** It used the verified runtime and frozen harness from `dc36199693472ac8297bafdaad6d4a0701acddf0`, tree `57c2625a948b9401b9517b2724b2715d8e7ca92b`. The process exited with code 0. There were **36 normal measurements plus six separately traced measurements per mode**, following 12 warmups per mode. Trace samples are excluded from every normal distribution below.

This is one source-specific run on a shared host with Chromium 156.0.8078.4, Playwright 1.64.1 and the owned Playwright BrowserServer/WebSocket transport. WebGL2 used SwiftShader. There was no baseline comparison (`comparisons` is empty), no speed threshold and no FPS assertion. The successful result establishes the measured workload, counter invariants, trace capture and process lifecycle; it does not establish that every interaction is fast.

**WebGL2 still shows large latency tails in this configuration.** Its p95 observation of the native frame is 762.638 ms, and the p95 observation after two subsequent animation callbacks is 1,716.649 ms. Those values are preserved rather than hidden behind the median.

## Normal measurements

Each cell is **median / p95 in milliseconds**, calculated from exactly 36 normal samples. The p95 uses the nearest-rank definition, and the median averages the two central samples. Independent arithmetic recomputation matched every recorded count, median, p95, minimum and maximum exactly.

| Host-observed interval | Software2D | WebGL2 / SwiftShader |
| --- | ---: | ---: |
| Input dispatch start → dispatch returns | 9.288 / 16.226 | 15.254 / 700.770 |
| Input dispatch start → native frame observed at the expected camera | 13.574 / 25.093 | 20.063 / 762.638 |
| Dispatch returns → native frame observed | 4.341 / 10.796 | 5.158 / 72.374 |
| Input dispatch start → public native presentation counter observed | 85.708 / 147.308 | 91.494 / 769.325 |
| Input dispatch start → observation after two subsequent animation callbacks | 112.115 / 263.382 | 1,028.619 / 1,716.649 |

The presentation and final observation rows are **cumulative from dispatch start**. They are not additional waits to add to preceding rows. The separate post-dispatch frame row measures the remaining observation wait after dispatch has returned. Distribution quantiles from different phases must not be added together.

These are host observations that include Playwright dispatch, native bridge serialization, browser scheduling and observation work. The public presentation counter is not a GPU-completion timestamp, and two animation callbacks are not a display-completion fence. The earlier [exact presentation-join record](../browser-text-pan-c549/presentation-joins.json) demonstrates that distinction on a separate c549 trace; it is not a trace result from this dc run. Node-clock durations and page-clock event timestamps are retained separately and must not be subtracted across clock origins.

## Workload and verified invariants

The seeded native fixture contains **150 labels**, 4,000 input Unicode code points, 3,250 non-space code points, Inter at 12.75 units and regular/bold/italic/bold-italic styles. Wrapping and the listed ligatures are disabled. The bridge does not expose an actual rendered-glyph count, so input characters are not presented as a glyph-throughput measurement.

Across all **72 normal samples**:

- Each pan produced exactly one observed native frame increment and one public presentation increment.
- All 150 labels remained visible and cached.
- Every document preparation, visual rebuild and layer projection delta was zero.
- Document revision and native validity assertions passed.
- Page errors, console errors and native render-panic lists remained empty.

The zero preparation deltas describe the application's recorded cache and document work. They do not imply that the browser or renderer performed no work. The workload still rendered real native frames at the requested camera positions.

## Separate trace cohort and strict shutdown

Each mode captured six additional traced pan motions after its normal cohort. Both trace records completed with no stream truncation, data loss, page/console error, fatal browser closure or cleanup error. The raw trace hashes match the report, and the standalone trace summaries match their embedded records exactly. Reprocessing both raw traces with the frozen parser reproduced the saved analyses exactly; [trace-reanalysis.json](trace-reanalysis.json) records this consistency check.

The final shutdown record reports:

- Client disconnect attempted and completed.
- Graceful BrowserServer close completed.
- The owned process exit was observed with **code 0 and no signal**.
- **No forced kill or direct kill** was attempted.
- No shutdown errors occurred, and a host-visible process scan found no remaining task-owned Node/Chromium process.

These are the outcomes of this local run. Independent CI runs have their own lifecycle results and are not superseded by this successful closure.

## Exact source and resource scope

All **56 frozen harness/package files**, all **17 runtime payloads**, and all nine runtime files in the benchmark fingerprint matched their recorded hashes after execution. The complete report has SHA-256 `1ff4ca28531b63522b0f26bfddc400d50e81b9921e0c17e3f27964f9c761db49`. Its recorded start and finish are `2026-10-23T15:43:25.287Z` and `2026-10-23T15:44:44.492Z`.

The benchmark began after the [selected interaction replay](../browser-runtime-dc361/README.md) had closed, with 944 MB free disk and about 2.04 GB total cgroup memory. No other task-owned browser or compiler ran concurrently. OOM counters remained at 74 events / 6 kills across the pre/post checks. This is a shared-host measurement; those checks do not establish constant load during every sample.

The glyph-coverage defect in the dc paragraph screenshots remains a separate visual limitation. Later embedded font or fallback changes require their own runtime validation and are not covered by this source identity.

## Preserved files

- [summary.json](summary.json) records the complete normal distributions, workload, transport, trace status and strict shutdown result.
- [report.json.gz](report.json.gz) retains the complete report; [benchmark.log.gz](benchmark.log.gz) retains all terminal output.
- Both `current-*-pan-trace.json.gz` files retain the full raw Chromium traces losslessly, with their complete `*-summary.json.gz` records.
- [verification.json](verification.json) records independent arithmetic recomputation, sample/counter checks, source/runtime hashes and process closure.
- [source-provenance.json](source-provenance.json), [execution-provenance.json](execution-provenance.json) and [provenance.json](provenance.json) preserve the exact source, commands, environment and all raw/compressed file digests.
- The seeded [fixture.json.gz](fixture.json.gz) and both native text screenshots are retained byte-for-byte.

All JSON and log compression is lossless gzip with a deterministic timestamp. No additional browser workload was run to create this evidence package.
