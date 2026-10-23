# Current c549 native text-pan benchmark

**The strict benchmark completed successfully in both Software2D and WebGL2**
using exact runtime and harness source
`c549f6bcafe614a6373c743f5efbf2a3cb0eccd2`. The process exited with code 0 at
12:47:50 UTC on 2026-10-23. Each mode completed 36 normal pan observations after
12 warm-ups and a separate six-observation Chromium trace. No trace sample is
included in the normal timing statistics.

The workload is the existing seeded document of 150 visible Inter labels,
4,000 Unicode code points and four font styles in a 1,600 × 1,100 viewport.
Actual middle-button pointer input pans the native FerroUI canvas. Each mode
reported 36 native frame advances, zero additional full or incremental prepares,
zero prepared nodes, zero rebuilt visuals and zero layer projection builds.
Authored document invariants passed. Console errors, page errors and render
panics were empty in both normal and trace cohorts.

## All normal timing phases

Milliseconds, **median / p95**, with 36 observations per cell:

| Host-observed phase | Software2D | WebGL2 / SwiftShader |
| --- | ---: | ---: |
| Dispatch start to expected native frame observation | 20.148 / 369.758 | 17.433 / 1,012.635 |
| Input dispatch | 10.645 / 51.547 | 13.849 / 954.304 |
| Frame observation after dispatch returns | 6.804 / 228.355 | 3.913 / 159.327 |
| Dispatch start to native surface presentation counter | 117.309 / 414.673 | 88.698 / 1,030.543 |
| Dispatch start through settling and two subsequent animation callbacks | 170.637 / 525.854 | 1,028.347 / 1,913.189 |

The primary frame-observation maxima were 832.707 ms in Software2D and
1,017.123 ms in WebGL2; settling reached 867.753 ms and 2,045.509 ms respectively.
These long tails and slow settling remain part of the result. A short median
frame-counter observation does not establish smooth frame delivery or FPS.
The native presentation counter is not a GPU completion timestamp, and two
subsequent animation callbacks do not fence compositor or display completion.

The browser was Chromium 156.0.8078.4, driven by Playwright 1.64.1 through an
owned Playwright server and WebSocket client. WebGL2 reported ANGLE with the
SwiftShader Vulkan software device. This shared host measurement includes input
automation, bridge serialization and browser scheduling; it is not a physical
GPU throughput or isolated renderer benchmark. Host phase times and page-clock
event/trace observations are kept separate. The trace-enabled transport and
sequential mode order also limit comparisons with other runs.

## Trace and process lifecycle

Both trace cohorts completed all six samples. Neither trace was truncated nor
reported data loss; both had empty page, console and cleanup error lists. The
trace byte streams are preserved, together with their complete analysis and
event/phase observations.

An independent [presentation join](presentation-joins.json) follows all six
WebGL2 diagnostic motions through the exact surface/frame token, display
aggregation, submission, containing browser swap and global completion events.
For motion 3, the two-callback settled marker precedes the matched swap's end
by 829.460 ms, despite a 19.442 ms native-presentation-to-settled interval.
That swap lasts 843.615 ms; the six matched swaps last 762.630–1,014.279 ms.
This directly demonstrates the observation limit above. These are six separate
trace observations, not additions to the 36 normal samples. Their wall durations
do not establish total worker CPU time, physical GPU work or a particular Rust
function as the cause.

After the real WASM workload finished, the client disconnected, graceful browser
closure completed and the owned Chromium process exited with code 0. No forced
kill or direct process kill was attempted, and shutdown reported no error.
The recorded browser PID was independently checked absent after completion.

Shared OOM counters stayed at 74 events / 6 kills throughout this run. Their
earlier increase occurred when this task had no active browser or compiler and
is not attributed to the application. The recorded pre-run anonymous memory was
about 4.47 GB; resource pressure and unrelated host activity can affect timings.

## Exact source and retained records

The verified runtime came from artifact `12502207600`; its WASM SHA-256 is
`64bd8f8b2dd9f3bc951776c9db7e3cb4bfc973ad754230219606d514bebc4115`.
[Build evidence](../browser-build-c549/README.md) records the complete archive,
toolchain, delivery and source-copy verification. All 55 frozen test/package
input hashes matched before and after the benchmark.

- [summary.json](summary.json) contains complete normal phase statistics,
  counters, environment, resource baselines and process shutdown.
- [independent-review.json](independent-review.json) recomputes all reported
  distributions and counter deltas and verifies the frozen sources, payloads,
  report fingerprints and raw trace summaries against their recorded hashes.
- [report.json.gz](report.json.gz) is the complete benchmark report, including
  every normal sample and both separate trace observations.
- [Software2D trace](current-Software2D-pan-trace.json.gz) and
  [WebGL2 trace](current-WebGL2-pan-trace.json.gz) retain the raw Chromium records.
- [Software2D trace summary](current-Software2D-pan-trace-summary.json.gz) and
  [WebGL2 trace summary](current-WebGL2-pan-trace-summary.json.gz) preserve the
  complete trace analysis.
- [fixture.json.gz](fixture.json.gz) retains the exact seeded input document.
- [benchmark.log.gz](benchmark.log.gz), [provenance.json](provenance.json) and
  [source-provenance.json](source-provenance.json) provide raw terminal output,
  byte digests and the immutable source inputs.

Every compressed record was decompressed and compared byte-for-byte with its
raw file. The earlier
[08 runtime / 0f harness run](../../2026-10-23-continuation/browser-text-pan-runtime08-harness0f/README.md)
remains a separate failed historical result: its Software2D favicon error
prevented that mode's trace cohort. The current run fixes that concrete failure
and exercises both trace cohorts; neither run establishes a causal performance
comparison across source versions. Editor interaction scenarios are reported
separately from this benchmark.
