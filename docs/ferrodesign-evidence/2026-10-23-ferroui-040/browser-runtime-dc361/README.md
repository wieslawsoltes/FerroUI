# Exact dc361996 browser interaction replay

**All ten selected scenario executions passed: 57 named checks in Software2D and 54 in WebGL2, 111 checks in total.** The process exited with code 0. Both the verified native WebAssembly application and the immutable 56-file browser harness came from `dc36199693472ac8297bafdaad6d4a0701acddf0`, tree `57c2625a948b9401b9517b2724b2715d8e7ca92b`.

This was a selected replay of five scenarios with the normal common gates, so `scope.full` is **false**. It does not establish complete feature or visual parity. In particular, the saved paragraph screenshots show **missing glyphs for Hebrew letters and the crab character**, even though Unicode data, rich formatting and native editing checks pass. That visual limitation is retained below and requires a separately built font correction.

The [exact browser build record](../browser-build-dc361/README.md) verifies the complete distribution. The [native feedback record](../native-dc361/README.md) separately proves the corrected native editing regressions. This local browser run used Chromium 156.0.8078.4 and Playwright 1.64.1, executing the two modes sequentially. The WebGL2 surface reported ANGLE/Vulkan **SwiftShader**, so these results do not establish physical GPU performance.

## Completed checks

| Scenario | Software2D | WebGL2 / SwiftShader |
| --- | --- | --- |
| Smoothed box corners | 2/2 passed, 7.837 s | 2/2 passed, 26.934 s |
| Polygon and star corners | 4/4 passed, 13.618 s | 4/4 passed, 74.773 s |
| Linear, radial and angular gradient handles | 17/17 passed, 32.798 s | 17/17 passed, 152.279 s |
| Native paragraph editing and composition | 7/7 passed, 21.895 s | 7/7 passed, 140.041 s |
| Guide variables, constraints and snapping | 5/5 passed, 25.030 s | 5/5 passed, 126.364 s |
| Normal common checks | 22 passed | 19 passed |

Scenario durations include real native input, reload boundaries, history checks, exports and observation waits on a shared host. They are not frame-rate measurements. Each mode finished with an empty page-error list, an empty console-error list, the `FerroUI Vello` renderer and zero reported unsupported effects.

### Rounded geometry and independent coverage

The smoothed box retained the same twelve-cubic silhouette in native PNG and SVG. The independent 16× and 32× area references converged with maximum alpha difference **1.96875**, below the unchanged limit of 4. The final native comparison had **zero pixels differing by more than 12 alpha units**. The original 1× SVG comparison, including its 288 edge-coverage differences, remains in the complete browser log.

The triangle retained the same six-cubic native, SVG and XAML geometry. Its original 1× raster comparison still records **212** pixels differing by more than 12 alpha units. The independent 16×/32× reference converged at **3.30078125**, and the native comparison against the converged reference produced **zero** pixels over 12; the allowance remains 85. Both modes retain the raw and reference measurements in their `polygon-corners-triangle-coverage.json` files. The real Inspect tab was used for XAML export, followed by a return to Design; the affine star, radius-variable detachment and exact undo/redo checks also passed.

### Native editing, gradients and guides

All 17 gradient groups passed in both modes. They cover transformed endpoint and stop drags, cancellation, capture loss, real Save/pagehide serialization, exact undo/redo, radial ellipse controls, angular sweep controls and independently sampled exported pixels. The previously failing WebGL2 linear stop insertion now receives an actual native double-click with counts **[1, 2]** at the intended point. The passive delivered-input record is preserved without changing the insertion or history assertions.

All seven paragraph groups passed in both modes, including real clipboard paste, repeated Unicode paragraphs, native UTF-16 caret placement, character and paragraph style remapping, first-clause composition visibility, cancellation, commit and native/document undo/redo. The full three-key pre-split diagnostics and clipboard observations are retained. These checks exercise the corrected native composition lifecycle; they do not assert the identity of every rendered glyph.

Guide checks use the actual native Zoom Out button to put the requested four-world-unit attraction inside the existing five-screen-pixel snap radius. They verify the view changes without changing the document or history before the gesture, then retain the original geometry, constraint, undo and redo assertions. Both final snapped-artwork captures are preserved.

### Common gates and bounded work

Normal common checks include native text/IME input, paint editing, toolbar input at the compact viewport and exact durable reload. The Software2D-only 10,000-node fixture realized **25 layer rows** before and after scrolling. Nudge, undo and redo each prepared **one node**, performed **zero full prepares**, rebuilt **zero visuals**, and reused the existing layer projection. Its focused spatial query considered one candidate with 27 bounds tests. The complete observations and timings remain in the report; these counts describe the exercised workload and do not promise a fixed frame rate.

## Visual limitation retained

The initial native canvas is saved byte-for-byte in [Software2D-initial.png](Software2D-initial.png) and [WebGL2-initial.png](WebGL2-initial.png). Native gradient adorners are visible in the retained radial and angular screenshots.

The [Software2D inline paragraph image](Software2D-native-paragraph-inline.png) and [WebGL2 image](WebGL2-native-paragraph-inline.png) visibly render missing-glyph bars where the fixture contains Hebrew letters and a crab character. The Latin text and caret are visible. The missing-glyph observation is a visual defect in this exact runtime and is not excused by the successful state/history checks or the zero unsupported-effects counter. Later font assets or fallback changes must be validated under their own source identity.

## Provenance, resources and retained artifacts

All **56 frozen harness/package files** and all **17 runtime payloads** matched their original hashes again after execution. Both trace ZIPs passed CRC validation, and every owned Node/Chromium process had closed before any subsequent benchmark was started. The report SHA-256 is `c8f355762a340ccc4feff77c79f5ebd567fd2f20930229834d1774f7074ee8ad`.

Shared disk space briefly reached zero and shared memory reached the cgroup cap while this replay was active. Verified redundant extracted runtimes and unpacked crate sources were reclaimed while their original archives, toolchains, binaries and all raw browser evidence were preserved. The complete report and traces were written successfully. OOM counters remained at **74 events / 6 kills** throughout; this record does not infer that the shared host was free from scheduling pressure.

- [summary.json](summary.json) lists every completed check and exact scenario duration, including the 10,000-node observations.
- [report.json.gz](report.json.gz) and [browser.log.gz](browser.log.gz) retain the complete report and terminal output losslessly.
- [source-provenance.json](source-provenance.json) records the exact Git source, all frozen input hashes and the artifact/WASM identity.
- [correctness-execution.json](correctness-execution.json) records the command, environment, owned process identities, exit status and post-run hash checks.
- [provenance.json](provenance.json) records every raw report-file hash, retained trace identities and cleanup-record hashes. The two original trace ZIPs remain in `/tmp/ferrodesign-e99-browser-dc361-focused-report`; they are not duplicated in Git.
- [resource-history.jsonl](resource-history.jsonl) and [memory-pressure.jsonl](memory-pressure.jsonl) retain sampled shared-host resource observations.

The selected screenshots, native exports and passive input records in this folder are exact copies of the original output files. The later strict text-pan benchmark is a separate cohort and result.
