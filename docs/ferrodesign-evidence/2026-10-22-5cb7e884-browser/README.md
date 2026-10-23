# Browser and trace evidence at `5cb7e884`

The completed [browser job 120117259515 in run 39862047469](https://github.com/wieslawsoltes/FerroUI/actions/runs/39862047469/job/120117259515) **failed** at exact source `5cb7e8847746a2bfae45bf46cafed40889f1affb`. The [decoded job log](browser-job.log) prints this checkout at **38, 109–130**. [jobs-api.json](jobs-api.json) records successful WASM build and IndexedDB validation, followed by failures in both editor validation and text-benchmark finalization. The separate native job is successful in that API snapshot; this directory does not derive native test counts from the browser log.

The source predates gradient and padding feature work. These records do not validate later commits. Only ordinary decoded text and bounded scalar/phase summaries were retrieved; no PNG, raw trace, full artifact report or ZIP was downloaded or inspected.

## Build, persistence and actual editor scope

The optimized browser build completed in **24m 22s** at **928**, verified six JavaScript modules at **929**, and reported **FerroUI Vello (WebGL2 hybrid, CPU fallback)** at **933**. Chromium reports version **156.0.8078.4**. The five IndexedDB checks all passed at **1191–1218**: atomic migration/current-previous saves, aborted-write behavior, stale-tab conflict protection, a large-document reload, and aborted migration preservation. The large fixture contains **12,000,042 characters**, reaches durable revision **5**, and reports **40.8 ms** save time. [indexeddb-summary.json](indexeddb-summary.json) preserves the exact printed JSON after timestamp-prefix removal.

The editor requested `scope.full=true`, both modes, and all scenarios, but **did not complete that scope**. [printed-summary.json](printed-summary.json), extracted from **1249–1329**, records:

| Requested / actual surface | Completed checks | Terminal result |
| --- | ---: | --- |
| Software2D / Software2D | 24 | Failed corner export pixel comparison |
| WebGL2 / WebGL2 | 24 | Failed the same corner export pixel comparison |

The actual WebGL2 device is **ANGLE / Vulkan SwiftShader Device (Subzero)**. Both modes log named starts for `vector-network`, `native-authoring`, then `corners` (**1233–1235**, **1241–1243**). The successful check list includes smoothing commit, invalid draft rejection, Escape and exact undo/redo. Both stop at `tests/corners-browser.mjs:135:12`, after the scalar diagnostic but before the export-parity check can pass. The command exits **1** at **1330**. Later scenarios and complete-run pixel/history coverage are not established by this result.

## Corner alpha diagnostics

The two diagnostic records at **1236** and **1244** are identical. The original assertion remains **at most 48 pixels with absolute alpha difference greater than 12**; it observes **288** and fails in each mode. [summary.json](summary.json) preserves every printed scalar and the twelve bounded pixel samples.

| Measurement | Native PNG | Independently decoded SVG / comparison |
| --- | ---: | ---: |
| Image dimensions | 200 × 140 | 200 × 140 |
| Partially covered pixels, `0 < alpha < 255` | 484 | 236 |
| Alpha area, `sum(alpha / 255)` | 26,555.215686 | 26,548.501961 |
| Maximum horizontal / vertical mirror difference | 0 / 0 | 64 / 64 |
| Pixels differing by more than 12 | — | 288 |
| Such pixels involving partial coverage | — | 288 |
| Exact opaque ↔ transparent swaps | — | 0 |
| Binary `alpha >= 128` disagreements | — | 44 |
| Maximum alpha difference | — | 78 |
| Differences per quadrant | — | 72 / 72 / 72 / 72 |
| Sum of absolute alpha differences, divided by 255 | — | 35.576471 |

Representative samples are `(34, 0): 30 vs 0`, `(37, 0): 102 vs 64`, and `(145, 0): 241 vs 255` for native/SVG alpha. The reported difference bounds touch all four image edges. The sum of absolute alpha differences is not the absolute difference between the two total areas.

These measurements localize the reported disagreement to edge coverage: all 288 over-threshold pixels include partial coverage, and none swap fully opaque with fully transparent. They do **not** identify a faulty decoder, prove mathematical equivalence, or justify changing the existing threshold. The two presentation modes also do not establish two independent PNG renderers; the native offscreen export path is shared. No image-based diagnosis or later correction is claimed here.

## Text measurements completed; benchmark command failed

The benchmark records **36 ordinary measurements per mode**, followed by **six separate trace-instrumented observations per mode**. Normal measurement summaries appear at **1344–1346** and **1348–1350**. The primary timing is host input dispatch through observation of the expected native frame; it includes automation and scheduling rather than isolating renderer cost.

| Ordinary cohort | Median / p95 dispatch-to-observed-frame | Dispatch p95 | Additional frame-observation p95 | Cumulative native-presentation p95 |
| --- | ---: | ---: | ---: | ---: |
| Software2D | 14.79 / 32.57 ms | 18.34 ms | 10.52 ms | 108.98 ms |
| WebGL2 | 16.89 / 1,377.48 ms | 1,289.95 ms | 87.85 ms | 1,401.39 ms |

These table values are rounded stdout statistics; the exact printed phase values are retained in [summary.json](summary.json). Native presentation is the public FerroUI surface counter, not a physical GPU completion timestamp. The three printed slowest WebGL2 samples each show zero full/incremental preparations and rebuilt visuals. Their page-clock event-to-capture delays are **1,268.4, 1,287.9 and 1,313.3 ms**, while capture-to-bubble is about **0.3 ms** in each. These observations place the measured delay before the document capture listener; they do not alone explain its cause.

This opt-in run uses `owned-playwright-server-websocket` transport, the compact metric bridge, and trace configuration enabled. Ordinary measurements occur outside the trace cohort, but comparisons with earlier trace-off/in-process-transport runs are not transport-identical. WebGL2 uses SwiftShader, so these figures are not hardware-GPU throughput measurements.

## Bounded trace observations and shutdown

Both printed diagnostic summaries report `status=completed`, six samples, `analysisStatus=complete`, and `normalSamplesIncluded=false` (**1347**, **1351**). They record zero page/console errors, no trace data loss or truncation, no missing/duplicate/wrong-thread markers, and no dropped input events. They also report **16 / 18 unmatched begin events**; completeness is not overstated.

The Software2D/WebGL2 clock mappings each use **42 markers**, with reported uncertainty **0.181 / 0.174 ms**, plus the analysis's **0.25 ms** quantization allowance. Trace/page clocks are explicitly aligned; host timing origins are not subtracted from page timestamps. The reported raw trace sizes are **1,863,580 / 3,205,716 bytes**; those trace files were not retrieved or independently verified.

WebGL2 diagnostic sample **2** has a **1,295.799 ms** event-timestamp-to-capture window. The printed interval union attributes **1,292.888 ms** of overlapping recorded wall time to the GPU-classified group and **0.379 ms** to the page main thread. Its capture-to-bubble window is **0.324 ms**; the later presentation-to-settled window is **1,331.286 ms**, overlapping **1,317.286 ms** of GPU-classified records. Several other WebGL2 settling windows show a similar long overlap.

The classifier groups events by process/thread names and trace categories. These are **observed wall-time overlaps**, not GPU utilization, exclusive time accounting, a GPU hardware measurement, or proof that a specific renderer operation caused the stall. Groups can overlap. Only the printed bounded analysis is available here, so no unprinted trace event or call stack is inferred.

After both normal cohorts and both diagnostic cohorts completed, the final owned-browser graceful close exceeded its bounded deadline. At **1352**, shutdown reports `status=forced`, `gracefulCloseCompleted=false`, `forcedKillAttempted=true`, and an observed **SIGKILL** process exit; direct fallback kill was not needed. The command then exits **1** at **1353**. Thus measurement completion and completed trace analysis do **not** make this benchmark step pass. The full job also remains failed for the independent editor pixel assertion.

## Integrity and extraction scope

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| [browser-job.log](browser-job.log) | 161,929 | `8d8a6d0fbdac9f14427495fa59d736e203afecd7c1e40dff3ec80277e2b773b4` |
| [jobs-api.json](jobs-api.json) | 7,621 | `4a9f469eb7fd48b1c9496ffc1b209f0c58bb5154d32a6f3e26f91a0cfe64795f` |
| [printed-summary.json](printed-summary.json) | 6,848 | `0bbb12e74e68227eaa67f838455e1929efbc42306e66023140aac6a6149d8665` |
| [indexeddb-summary.json](indexeddb-summary.json) | 887 | `8129c1f9a61107a48131d8d29dca75c1e66e7df44101825edf04c01455d522d9` |
| [summary.json](summary.json) | 56,868 | `d9a718bacd506c37d9ea58948437b1dc7c9322f3183705509f3c36c340dafae1` |

The log has **1,457 lines** and retains the decoded UTF-8 BOM/ANSI content. Its byte count differs from a JavaScript string-length count. The two printed JSON blocks remove timestamp prefixes only; the derived summary records original log lines and keeps normal, trace-instrumented, failure and cleanup outcomes distinct. These hashes cover local decoded text, not a downloaded artifact or archive.
