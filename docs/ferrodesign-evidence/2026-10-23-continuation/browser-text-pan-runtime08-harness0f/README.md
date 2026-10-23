# Native WebAssembly text-pan measurement

One traced benchmark attempt exercised runtime `08e29e4f530c0ae65f6e2ac072f381605e23ed0c` with the exact benchmark modules from harness `0f0732784bd6fdf1cb2477a61dd332d9a007fec2`. It finished with **exit code 1** because Software2D's strict empty-console assertion found a missing `/favicon.ico`. Both modes recorded 36 normal motion samples. WebGL2 also completed its separate six-motion trace; Software2D did not reach tracing. No assertion was relaxed or sample discarded to turn this attempt into a pass.

The actual native WASM shutdown path completed successfully. The owned connected client disconnected, BrowserServer closed gracefully, and Chromium process exit was observed with code 0. There was no forced kill, direct kill, cleanup error, or remaining owned process. This supplies real application evidence for the shutdown ownership correction.

## Workload and method

The seeded document contains 150 visible Inter text labels arranged in five columns and 30 rows, using regular, bold, italic and bold-italic text styles. The input has 4,000 code points; actual rendered glyph counts are not exposed by the bridge. Each mode receives 12 warmups followed by 36 real native pointer motions. Six separately labeled traced motions follow successful normal assertions and never enter the normal statistics.

The benchmark modules were copied from exact git objects into a temporary harness before the repository was released for further edits. All six module/package hashes matched before and after execution. The browser was Chromium `156.0.8078.4`, Playwright `1.64.1`, and Node `24.19.0`; WebGL2 used SwiftShader on the shared host. These observations include automation dispatch, native bridge reads, scheduling and rendering. The public native presentation counter is not a hardware GPU completion timestamp, and these results do not establish FPS.

## Recorded timings

All values below are milliseconds. Software2D's observations remain labeled as a failed cohort because of its final console assertion.

| Phase | Software2D median | Software2D p95 | WebGL2 median | WebGL2 p95 |
| --- | ---: | ---: | ---: | ---: |
| Dispatch to observed native frame advance | 18.20 | 214.23 | 15.24 | 26.77 |
| Input dispatch | 11.02 | 102.85 | 11.76 | 19.77 |
| Frame observation after dispatch | 6.42 | 111.38 | 2.85 | 5.71 |
| Observed native presentation | 111.45 | 356.17 | 77.61 | 118.86 |
| Settling through two subsequent animation callbacks | 141.77 | 711.04 | 775.31 | 905.03 |

The maximum dispatch-to-frame observation was 1,569.78 ms for Software2D and 117.06 ms for WebGL2. The slower settling values remain part of the evidence; the first observed frame counter alone is insufficient to describe user-visible completion. Modes ran sequentially on a shared host, so the table should not be treated as a controlled backend comparison.

Both modes observed 36 native frame advances with zero additional full prepares, incremental prepares, prepared nodes, rebuilt visuals or layer-projection builds during the measured camera motions. No render panic or page exception was recorded. The canonical document remained unchanged.

## Failure and evidence

Software2D's only console entry was a 404 resource error. The ordinary editor trace identifies the URL as `/favicon.ico`. The production fix should supply an explicit relative icon link and its asset, which also works when the benchmark serves the application under `/current/`. The strict empty-console assertion remains intact.

WebGL2 tracing completed without truncation, data loss, cleanup errors or page/console errors. The complete 2,739,494-byte trace is preserved as [current-WebGL2-pan-trace.json.gz](current-WebGL2-pan-trace.json.gz); its independent analysis is in [current-WebGL2-pan-trace-summary.json.gz](current-WebGL2-pan-trace-summary.json.gz).

The [subsequent presentation analysis](presentation-analysis/README.md) joins all six actual submitted surfaces to their Chromium display swaps. It locates the dominant recorded late wait at the browser display-swap boundary and separately retains the 42.545–54.144 ms FerroUI callback work. The underlying driver/worker/host cause remains unresolved; neither this reanalysis nor the source comparison establishes a current-runtime optimization.

[report.json.gz](report.json.gz) preserves every normal phase sample, trusted-input observation, native counter, error and shutdown record. [summary.json](summary.json) provides the readable result projection, [fixture.json.gz](fixture.json.gz) preserves the exact workload, and [provenance.json](provenance.json) plus [source-provenance.json](source-provenance.json) identify all source and artifact hashes. This measurement predates the subsequent upstream framework integration and does not validate that later binary.
